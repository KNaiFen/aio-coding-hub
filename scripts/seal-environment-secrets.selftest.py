#!/usr/bin/env python3
"""Standard-library regression checks for the temporary migration bridge."""

from __future__ import annotations

import base64
from contextlib import redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import re
import stat
import tempfile
import unittest


SCRIPT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = SCRIPT_DIR.parent
HELPER_PATH = SCRIPT_DIR / "seal-environment-secrets.py"
WORKFLOW_PATH = REPOSITORY_ROOT / ".github/workflows/migrate-signing-secrets.yml"

SPEC = importlib.util.spec_from_file_location("seal_environment_secrets", HELPER_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("Unable to load migration helper.")
HELPER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HELPER)


class MigrationHelperTests(unittest.TestCase):
    def setUp(self) -> None:
        self.context = {
            "source_sha": "a" * 40,
            "workflow_run_id": 123,
            "workflow_run_attempt": 2,
        }

    def test_context_requires_canonical_manual_main(self) -> None:
        environment = {
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REPOSITORY": "KNaiFen/aio-coding-hub",
            "GITHUB_REF": "refs/heads/main",
            "GITHUB_ACTOR": "KNaiFen",
            "GITHUB_TRIGGERING_ACTOR": "KNaiFen",
            "GITHUB_SHA": "a" * 40,
            "GITHUB_RUN_ID": "123",
            "GITHUB_RUN_ATTEMPT": "2",
        }
        self.assertEqual(HELPER._load_context(environment), self.context)

        for name, bad_value in (
            ("GITHUB_EVENT_NAME", "push"),
            ("GITHUB_REPOSITORY", "other/repository"),
            ("GITHUB_REF", "refs/heads/feature"),
            ("GITHUB_ACTOR", "other-user"),
            ("GITHUB_TRIGGERING_ACTOR", "other-user"),
            ("GITHUB_SHA", "main"),
            ("GITHUB_RUN_ID", "0"),
        ):
            with self.subTest(name=name):
                invalid_environment = dict(environment)
                invalid_environment[name] = bad_value
                with self.assertRaises(HELPER.MigrationFailure):
                    HELPER._load_context(invalid_environment)

    def test_public_key_is_strict_base64_and_exactly_32_bytes(self) -> None:
        encoded_key = base64.b64encode(b"k" * 32).decode("ascii")
        self.assertEqual(HELPER._decode_public_key(encoded_key), b"k" * 32)

        for invalid_key in ("not base64", base64.b64encode(b"short").decode("ascii")):
            with self.subTest(invalid_key=invalid_key):
                with self.assertRaises(HELPER.MigrationFailure):
                    HELPER._decode_public_key(invalid_key)

    def test_document_has_exact_schema_and_both_ciphertexts(self) -> None:
        encrypted_values = {
            "TAURI_SIGNING_PRIVATE_KEY": b"a" * 49,
            "TAURI_SIGNING_PRIVATE_KEY_PASSWORD": b"b" * 49,
        }
        document = HELPER._build_document(self.context, "123456789", encrypted_values)

        self.assertEqual(
            list(document),
            [
                "schema_version",
                "repository",
                "environment",
                "source_sha",
                "workflow_run_id",
                "workflow_run_attempt",
                "key_id",
                "encrypted_secrets",
            ],
        )
        self.assertEqual(
            [entry["name"] for entry in document["encrypted_secrets"]],
            [
                "TAURI_SIGNING_PRIVATE_KEY",
                "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
            ],
        )
        self.assertEqual(
            {key for entry in document["encrypted_secrets"] for key in entry},
            {"name", "encrypted_value"},
        )
        serialized = json.dumps(document)
        for forbidden_field in ("plaintext", "digest", "sha256", "length"):
            self.assertNotIn(forbidden_field, serialized.lower())

        for missing_name in encrypted_values:
            with self.subTest(missing_name=missing_name):
                incomplete = dict(encrypted_values)
                del incomplete[missing_name]
                with self.assertRaises(HELPER.MigrationFailure):
                    HELPER._build_document(self.context, "123456789", incomplete)

    def test_output_is_new_atomic_private_file(self) -> None:
        document = HELPER._build_document(
            self.context,
            "123456789",
            {
                "TAURI_SIGNING_PRIVATE_KEY": b"a" * 49,
                "TAURI_SIGNING_PRIVATE_KEY_PASSWORD": b"b" * 49,
            },
        )
        with tempfile.TemporaryDirectory() as temporary_directory:
            output_path = Path(temporary_directory) / "nested" / "migration.json"
            HELPER._write_new_document(output_path, document)
            self.assertEqual(
                stat.S_IMODE(output_path.stat().st_mode),
                stat.S_IRUSR | stat.S_IWUSR,
            )
            self.assertEqual(json.loads(output_path.read_text(encoding="ascii")), document)
            with self.assertRaises(HELPER.MigrationFailure):
                HELPER._write_new_document(output_path, document)

    def test_explicit_masks_escape_multiline_workflow_commands(self) -> None:
        output = io.StringIO()
        with redirect_stdout(output):
            HELPER._mask_secret_values({"test": "line%one\r\nline-two"})
        self.assertEqual(output.getvalue(), "::add-mask::line%25one%0D%0Aline-two\n")

    def test_pynacl_sealed_box_round_trip_when_dependency_is_available(self) -> None:
        try:
            import nacl
            from nacl.public import PrivateKey, SealedBox
        except ModuleNotFoundError:
            self.skipTest("PyNaCl is intentionally not installed for local static checks.")

        self.assertEqual(nacl.__version__, "1.6.2")
        private_key = PrivateKey.generate()
        secret_values = {
            "TAURI_SIGNING_PRIVATE_KEY": "private-key\nwith-newline",
            "TAURI_SIGNING_PRIVATE_KEY_PASSWORD": "password%value",
        }
        encrypted_values = HELPER._seal_values(bytes(private_key.public_key), secret_values)
        decryptor = SealedBox(private_key)
        for secret_name, secret_value in secret_values.items():
            self.assertEqual(
                decryptor.decrypt(encrypted_values[secret_name]),
                secret_value.encode("utf-8"),
            )


class MigrationWorkflowTests(unittest.TestCase):
    def test_workflow_is_temporary_manual_main_only_and_read_only(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        self.assertIn("TEMPORARY: remove this workflow, its helper", workflow)
        self.assertIn("on:\n  workflow_dispatch:", workflow)
        trigger_block = workflow.split("\non:\n", maxsplit=1)[1].split(
            "\npermissions:", maxsplit=1
        )[0]
        self.assertEqual(
            re.findall(r"^  ([A-Za-z_][A-Za-z0-9_-]*):\s*$", trigger_block, re.MULTILINE),
            ["workflow_dispatch"],
        )
        self.assertIn('context.ref !== "refs/heads/main"', workflow)
        self.assertIn('mainBranch.commit.sha !== context.sha', workflow)
        self.assertIn("MIGRATION_ACTOR", workflow)
        self.assertIn("MIGRATION_TRIGGERING_ACTOR", workflow)
        self.assertIn("permissions: {}", workflow)
        self.assertNotIn("contents: write", workflow)
        self.assertNotIn("actions: write", workflow)
        self.assertNotIn("secrets: write", workflow)
        self.assertNotIn("--method DELETE", workflow)
        self.assertNotIn("gh secret", workflow)

    def test_sealing_and_probe_have_separate_secret_scopes(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        _, jobs = workflow.split("\njobs:\n", maxsplit=1)
        _, after_authorize = jobs.split("\n  seal-repository-secrets:\n", maxsplit=1)
        seal_job, probe_job = after_authorize.split(
            "\n  probe-environment-secrets:\n", maxsplit=1
        )

        self.assertNotIn("\n    environment:\n", seal_job)
        self.assertIn("\n    environment:\n      name: release-signing\n", probe_job)
        self.assertNotIn("resolve_environment_key", workflow)
        self.assertNotIn("vars.RELEASE_SIGNING_MIGRATION_PUBLIC_KEY", workflow)
        self.assertNotIn("vars.RELEASE_SIGNING_MIGRATION_KEY_ID", workflow)
        self.assertIn(
            'REVIEWED_ENVIRONMENT_PUBLIC_KEY: "8mwZgQXTC//MSfF29jGwXD+2TsXKffk4ORopLRgfgAs="',
            workflow,
        )
        self.assertIn('REVIEWED_ENVIRONMENT_KEY_ID: "3380204578043523366"', workflow)
        self.assertIn(
            "TARGET_ENVIRONMENT_PUBLIC_KEY: ${{ env.REVIEWED_ENVIRONMENT_PUBLIC_KEY }}",
            seal_job,
        )
        self.assertIn(
            "TARGET_ENVIRONMENT_KEY_ID: ${{ env.REVIEWED_ENVIRONMENT_KEY_ID }}",
            seal_job,
        )
        self.assertIn("vars.RELEASE_SIGNING_MIGRATION_READY_ENVELOPE", probe_job)
        self.assertIn("GET /repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt_number}", probe_job)
        self.assertIn("sealingRun.path !== '.github/workflows/migrate-signing-secrets.yml'", probe_job)
        self.assertIn(
            "release-signing-secret-migration-${runId}-${runAttempt}", probe_job
        )
        self.assertIn("actions: read", probe_job)
        self.assertNotIn("inputs.environment_", workflow)
        self.assertEqual(seal_job.count("secrets.TAURI_SIGNING"), 2)
        self.assertEqual(probe_job.count("secrets.TAURI_SIGNING"), 2)
        self.assertLess(
            seal_job.index("Install pinned PyNaCl sealed-box implementation"),
            seal_job.index("Run migration helper self-test"),
        )
        self.assertIn("pynacl-1.6.2", seal_job)
        self.assertIn("retention-days: 1", seal_job)
        self.assertIn("if-no-files-found: error", seal_job)
        self.assertIn(
            "path: ${{ runner.temp }}/signing-secret-migration/release-signing-secrets.json",
            seal_job,
        )
        self.assertEqual(workflow.count("retention-days: 1"), 1)
        self.assertNotIn("signer.log", probe_job)
        self.assertNotIn("verifier.log", probe_job)
        self.assertIn('-p "$raw_password"', probe_job)
        self.assertLess(
            probe_job.index("unset raw_key raw_password normalized_key"),
            probe_job.index("base64.b64decode(encoded_signature, validate=True)"),
        )
        self.assertGreaterEqual(probe_job.count(">/dev/null 2>&1"), 2)
        self.assertIn("base64.b64decode(encoded_signature, validate=True)", probe_job)
        self.assertIn("signature from tauri secret key", probe_job)
        self.assertIn('-x "$minisign_signature_path"', probe_job)

    def test_every_action_is_pinned_to_a_full_commit(self) -> None:
        workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
        uses_lines = [line.strip() for line in workflow.splitlines() if "uses:" in line]
        self.assertGreater(len(uses_lines), 0)
        for uses_line in uses_lines:
            with self.subTest(uses_line=uses_line):
                action_reference = uses_line.split("uses:", maxsplit=1)[1].split(
                    "#", maxsplit=1
                )[0]
                revision = action_reference.rsplit("@", maxsplit=1)[1].strip()
                self.assertRegex(revision, r"^[0-9a-f]{40}$")


if __name__ == "__main__":
    unittest.main()
