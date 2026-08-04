// Usage:
// - Shared parser for `request_logs.attempts_json` (serialized gateway FailoverAttempt list).
// - Single contract for the provider chain view and the error-card failure summary.
// - `timeout_secs` is the structured first-byte timeout; never parse it out of `outcome`.

import type { StreamInternalErrorEvidence } from "../../generated/bindings";

export type { StreamInternalErrorEvidence } from "../../generated/bindings";

export type AttemptJsonEntry = {
  provider_id: number;
  provider_name: string;
  base_url: string;
  requested_upstream_model?: string | null;
  outcome: string;
  status: number | null;
  provider_index?: number | null;
  retry_index?: number | null;
  session_reuse?: boolean | null;
  error_category?: string | null;
  error_code?: string | null;
  decision?: string | null;
  reason?: string | null;
  selection_method?: string | null;
  reason_code?: string | null;
  attempt_started_ms?: number | null;
  attempt_duration_ms?: number | null;
  circuit_state_before?: string | null;
  circuit_state_after?: string | null;
  circuit_failure_count?: number | null;
  circuit_failure_threshold?: number | null;
  // Circuit attribution for gate-skip attempts; the backend omits both keys
  // entirely on success and non-circuit paths (space constraint).
  circuit_recover_at_unix?: number | null;
  circuit_trigger_error_code?: string | null;
  timeout_secs?: number | null;
  stream_internal_error?: StreamInternalErrorEvidence | null;
};

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value != null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function asRequiredString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function asOptionalString(value: unknown): string | null {
  if (value == null) return null;
  return typeof value === "string" ? value : null;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isOptionalString(value: unknown): boolean {
  return value == null || typeof value === "string";
}

function isOptionalFiniteNumber(value: unknown): boolean {
  return value == null || isFiniteNumber(value);
}

function isOptionalBoolean(value: unknown): boolean {
  return value == null || typeof value === "boolean";
}

function isAttemptJsonEntry(value: unknown): value is AttemptJsonEntry {
  const record = asRecord(value);
  if (
    !record ||
    !isFiniteNumber(record.provider_id) ||
    typeof record.provider_name !== "string" ||
    typeof record.base_url !== "string" ||
    typeof record.outcome !== "string" ||
    !(record.status === null || isFiniteNumber(record.status))
  ) {
    return false;
  }

  return (
    [
      "requested_upstream_model",
      "error_category",
      "error_code",
      "decision",
      "reason",
      "selection_method",
      "reason_code",
      "circuit_state_before",
      "circuit_state_after",
      "circuit_trigger_error_code",
    ].every((key) => isOptionalString(record[key])) &&
    [
      "provider_index",
      "retry_index",
      "attempt_started_ms",
      "attempt_duration_ms",
      "circuit_failure_count",
      "circuit_failure_threshold",
      "circuit_recover_at_unix",
      "timeout_secs",
    ].every((key) => isOptionalFiniteNumber(record[key])) &&
    isOptionalBoolean(record.session_reuse)
  );
}

export function parseStreamInternalErrorEvidence(
  value: unknown
): StreamInternalErrorEvidence | null {
  const record = asRecord(value);
  if (!record) return null;

  const eventType = asRequiredString(record.event_type);
  const classification = asRequiredString(record.classification);
  const disposition = asRequiredString(record.disposition);
  if (!eventType || !classification || !disposition || typeof record.truncated !== "boolean") {
    return null;
  }

  return {
    event_type: eventType,
    error_type: asOptionalString(record.error_type),
    error_code: asOptionalString(record.error_code),
    message: asOptionalString(record.message),
    classification,
    matched_keyword: asOptionalString(record.matched_keyword),
    disposition,
    truncated: record.truncated,
  };
}

export function parseAttemptsJson(raw: string | null | undefined): AttemptJsonEntry[] | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return null;
    if (!parsed.every(isAttemptJsonEntry)) return null;
    return parsed.map((entry) => {
      return {
        ...entry,
        stream_internal_error: parseStreamInternalErrorEvidence(entry.stream_internal_error),
      };
    });
  } catch {
    return null;
  }
}
