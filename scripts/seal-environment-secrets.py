#!/usr/bin/env python3
"""Seal the legacy updater secrets for one-time Environment migration."""

from __future__ import annotations

import argparse
import base64
import binascii
import json
import os
from pathlib import Path
import re
import tempfile
from typing import Mapping, Protocol


CANONICAL_REPOSITORY = "KNaiFen/aio-coding-hub"
TARGET_ENVIRONMENT = "release-signing"
EXPECTED_REF = "refs/heads/main"
EXPECTED_EVENT = "workflow_dispatch"
EXPECTED_ACTOR = "KNaiFen"
SCHEMA_VERSION = 1
PUBLIC_KEY_BYTES = 32
SECRET_ENVIRONMENT_NAMES = {
    "TAURI_SIGNING_PRIVATE_KEY": "LEGACY_TAURI_SIGNING_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY_PASSWORD": (
        "LEGACY_TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
    ),
}
SHA_PATTERN = re.compile(r"[0-9a-f]{40}")
KEY_ID_PATTERN = re.compile(r"[0-9]{1,64}")


class MigrationFailure(Exception):
    """Expected validation failure with intentionally redacted diagnostics."""


class SealedBoxLike(Protocol):
    def encrypt(self, message: bytes) -> bytes: ...


def _workflow_command_value(value: str) -> str:
    return value.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def _mask_secret_values(secret_values: Mapping[str, str]) -> None:
    for value in secret_values.values():
        if value:
            print(f"::add-mask::{_workflow_command_value(value)}", flush=True)


def _positive_integer(raw_value: str) -> int:
    if not raw_value.isascii() or not raw_value.isdecimal():
        raise MigrationFailure
    value = int(raw_value)
    if value < 1:
        raise MigrationFailure
    return value


def _load_context(environment: Mapping[str, str]) -> dict[str, object]:
    if environment.get("GITHUB_EVENT_NAME") != EXPECTED_EVENT:
        raise MigrationFailure
    if environment.get("GITHUB_REPOSITORY") != CANONICAL_REPOSITORY:
        raise MigrationFailure
    if environment.get("GITHUB_REF") != EXPECTED_REF:
        raise MigrationFailure
    if environment.get("GITHUB_ACTOR") != EXPECTED_ACTOR:
        raise MigrationFailure
    if environment.get("GITHUB_TRIGGERING_ACTOR") != EXPECTED_ACTOR:
        raise MigrationFailure

    source_sha = environment.get("GITHUB_SHA", "")
    if SHA_PATTERN.fullmatch(source_sha) is None:
        raise MigrationFailure

    return {
        "source_sha": source_sha,
        "workflow_run_id": _positive_integer(environment.get("GITHUB_RUN_ID", "")),
        "workflow_run_attempt": _positive_integer(
            environment.get("GITHUB_RUN_ATTEMPT", "")
        ),
    }


def _decode_public_key(encoded_key: str) -> bytes:
    try:
        public_key = base64.b64decode(encoded_key, validate=True)
    except (binascii.Error, ValueError) as error:
        raise MigrationFailure from error
    if len(public_key) != PUBLIC_KEY_BYTES:
        raise MigrationFailure
    return public_key


def _build_document(
    context: Mapping[str, object],
    key_id: str,
    encrypted_values: Mapping[str, bytes],
) -> dict[str, object]:
    if KEY_ID_PATTERN.fullmatch(key_id) is None:
        raise MigrationFailure
    if set(encrypted_values) != set(SECRET_ENVIRONMENT_NAMES):
        raise MigrationFailure

    encrypted_secrets = []
    for secret_name in SECRET_ENVIRONMENT_NAMES:
        encrypted_value = encrypted_values[secret_name]
        if len(encrypted_value) <= 48:
            raise MigrationFailure
        encrypted_secrets.append(
            {
                "name": secret_name,
                "encrypted_value": base64.b64encode(encrypted_value).decode("ascii"),
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "repository": CANONICAL_REPOSITORY,
        "environment": TARGET_ENVIRONMENT,
        "source_sha": context["source_sha"],
        "workflow_run_id": context["workflow_run_id"],
        "workflow_run_attempt": context["workflow_run_attempt"],
        "key_id": key_id,
        "encrypted_secrets": encrypted_secrets,
    }


def _write_new_document(output_path: Path, document: Mapping[str, object]) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if output_path.exists() or output_path.is_symlink():
        raise MigrationFailure

    temporary_path: Path | None = None
    try:
        descriptor, raw_temporary_path = tempfile.mkstemp(
            dir=output_path.parent,
            prefix=f".{output_path.name}.",
        )
        temporary_path = Path(raw_temporary_path)
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "w", encoding="ascii", newline="\n") as output_file:
            json.dump(document, output_file, ensure_ascii=True, indent=2)
            output_file.write("\n")
            output_file.flush()
            os.fsync(output_file.fileno())
        os.replace(temporary_path, output_path)
        temporary_path = None
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def _seal_values(public_key: bytes, secret_values: Mapping[str, str]) -> dict[str, bytes]:
    from nacl.public import PublicKey, SealedBox

    sealed_box: SealedBoxLike = SealedBox(PublicKey(public_key))
    return {
        secret_name: sealed_box.encrypt(secret_values[secret_name].encode("utf-8"))
        for secret_name in SECRET_ENVIRONMENT_NAMES
    }


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Seal both updater secrets into a fixed-schema migration envelope."
    )
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        context = _load_context(os.environ)
        secret_values = {
            secret_name: os.environ.get(environment_name, "")
            for secret_name, environment_name in SECRET_ENVIRONMENT_NAMES.items()
        }
        _mask_secret_values(secret_values)
        if (
            not secret_values["TAURI_SIGNING_PRIVATE_KEY"].strip()
            or not secret_values["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"]
        ):
            raise MigrationFailure

        key_id = os.environ.get("TARGET_ENVIRONMENT_KEY_ID", "")
        public_key = _decode_public_key(
            os.environ.get("TARGET_ENVIRONMENT_PUBLIC_KEY", "")
        )
        encrypted_values = _seal_values(public_key, secret_values)
        document = _build_document(context, key_id, encrypted_values)
        _write_new_document(args.output, document)
    # Never surface crypto/decoder exceptions because they can describe secret material.
    except Exception:
        print("Signing-secret migration sealing failed.", file=os.sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
