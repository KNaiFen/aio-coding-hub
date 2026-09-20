import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";
import type { ResponseCommitContext, ResponseCommitResult } from "../../../packages/plugin-sdk/src";

const require = createRequire(import.meta.url);
const extension = require("../codex-model-consistency/extension.cjs");
let check: (context: ResponseCommitContext) => ResponseCommitResult;
extension.activate({
  gateway: {
    registerHook(name: string, callback: typeof check) {
      expect(name).toBe("gateway.response.beforeCommit");
      check = callback;
    },
  },
});
const model = "gpt-test";
function run(body: string, stream = false, expected: string | null = model) {
  return check({
    hook: "gateway.response.beforeCommit",
    traceId: "trace-model-check",
    config: {},
    context: {
      hookName: "gateway.response.beforeCommit",
      traceId: "trace-model-check",
      request: { cliKey: "codex", method: "POST", path: "/responses", requestedModel: model },
      outboundRequest: { model: expected },
      attempt: { providerId: 1, providerIndex: 0, retryIndex: 0 },
      response: {
        status: 200,
        headers: {},
        body,
        contentType: stream ? "text/event-stream; charset=utf-8" : "application/json",
        complete: true,
        decodedBytes: Buffer.byteLength(body),
      },
    },
  });
}
function event(type: string, response: object = {}) {
  return `event: ${type}\ndata: ${JSON.stringify({ type, response })}\n\n`;
}
const done = (response: object = { model, status: "completed" }) =>
  event("response.completed", response);
const reason = (name: string) => ({
  action: "switchProvider",
  reasonCode: `codex_model_consistency.${name}`,
});

describe("installed model consistency plugin entry", () => {
  it("accepts completed JSON and repeated matching stream declarations without DONE", () => {
    expect(run(JSON.stringify({ model, status: "completed" }))).toEqual({ action: "pass" });
    expect(
      run(
        event("response.created", { model }) + event("response.in_progress", { model }) + done(),
        true
      )
    ).toEqual({ action: "pass" });
    expect(run(event("response.created") + done(), true)).toEqual({ action: "pass" });
  });
  it.each(["before", "after", "after-completion"])("rejects a conflicting model %s", (position) => {
    const wrong = event("response.in_progress", { model: "wrong" });
    const body =
      position === "before"
        ? wrong + done()
        : position === "after"
          ? event("response.created", { model }) + done({ model: "wrong", status: "completed" })
          : done() + wrong;
    expect(run(body, true)).toMatchObject(reason("model_mismatch"));
  });
  it.each([null, 1, "", " ", {}, []])("rejects invalid model %j", (invalid) => {
    expect(run(JSON.stringify({ model: invalid, status: "completed" }))).toMatchObject(
      reason("invalid_model")
    );
  });
  it("requires a model and exact name including snapshots and surrounding whitespace", () => {
    expect(run('{"status":"completed"}')).toMatchObject(reason("model_missing"));
    for (const actual of ["gpt-test-2026-09-19", " gpt-test", "GPT-test"]) {
      expect(run(JSON.stringify({ model: actual, status: "completed" }))).toMatchObject(
        reason("model_mismatch")
      );
    }
  });
  it.each([
    '{"model":"wrong","model":"gpt-test","status":"completed"}',
    '{"model":"gpt-test","mo\\u0064el":"gpt-test","status":"completed"}',
    '{"model":"gpt-test","status":"failed","status":"completed"}',
  ])("rejects duplicate protocol keys without JSON.parse hiding them", (json) => {
    expect(run(json)).toMatchObject(reason("invalid_response"));
  });
  it("rejects duplicate nested response model/response keys", () => {
    expect(
      run(
        'data: {"type":"response.completed","response":{"model":"wrong","model":"gpt-test","status":"completed"}}\n\n',
        true
      )
    ).toMatchObject(reason("invalid_response"));
    expect(
      run(
        'data: {"type":"response.completed","response":{"model":"wrong"},"response":{"model":"gpt-test","status":"completed"}}\n\n',
        true
      )
    ).toMatchObject(reason("invalid_response"));
  });
  it("does not interpret model in output, text, or tool arguments", () => {
    expect(
      run(
        JSON.stringify({
          model,
          status: "completed",
          output: [
            {
              model: "business",
              arguments: '{"model":"wrong","model":"business"}',
              text: '"model":"wrong"',
            },
          ],
        })
      )
    ).toEqual({ action: "pass" });
    expect(
      run(
        'data: {"type":"response.output_text.delta","delta":"model: wrong","item":{"model":"business"}}\n\n' +
          done(),
        true
      )
    ).toEqual({ action: "pass" });
  });
  it("supports CRLF, comments, multi-line data, and optional DONE", () => {
    const body =
      ': heartbeat\r\nevent: response.completed\r\ndata: {"type":"response.completed",\r\ndata: "response":{"model":"gpt-test","status":"completed"}}\r\n\r\ndata: [DONE]\r\n\r\n';
    expect(run(body, true)).toEqual({ action: "pass" });
    expect(run(done() + ": final heartbeat", true)).toEqual({ action: "pass" });
  });
  it.each(["failed", "incomplete"])("rejects %s before or after a completed event", (status) => {
    expect(run(JSON.stringify({ model, status }))).toMatchObject(reason("response_unsuccessful"));
    expect(run(event(`response.${status}`, { model, status }) + done(), true)).toMatchObject(
      reason("response_unsuccessful")
    );
    expect(run(done() + event(`response.${status}`, { model, status }), true)).toMatchObject(
      reason("response_unsuccessful")
    );
  });
  it.each([
    "",
    "data: [DONE]\n\n",
    event("response.created", { model }),
    done().trimEnd(),
    done().slice(0, -1),
  ])("rejects missing completion or truncated event", (body) => {
    expect(run(body, true)).toMatchObject(reason("incomplete_response"));
  });
  it("rejects malformed or contradictory protocol events", () => {
    expect(run("{bad}")).toMatchObject(reason("invalid_response"));
    expect(
      run(
        'event: response.completed\ndata: {"type":"response.failed","response":{"model":"gpt-test","status":"completed"}}\n\n',
        true
      )
    ).toMatchObject(reason("response_unsuccessful"));
  });
  it.each([
    [false, '"\\\n\t'],
    [false, "漢😀"],
    [true, '"\\\n\t'],
    [true, "漢😀"],
  ] as const)("checks tail models at the 2 MiB boundary (stream=%s, unit=%j)", (stream, unit) => {
    const encoded = JSON.stringify(unit).slice(1, -1);
    const prefix = stream
      ? 'event: response.output_text.delta\ndata: {"type":"response.output_text.delta","delta":"'
      : '{"output":[{"content":[{"text":"';
    const suffix = stream ? '"}\n\n' + done() : '"}]}],"status":"completed","model":"gpt-test"}';
    for (const size of [2 * 1024 * 1024 - 1, 2 * 1024 * 1024]) {
      const available = size - Buffer.byteLength(prefix + suffix);
      const unitBytes = Buffer.byteLength(encoded);
      const body =
        prefix +
        encoded.repeat(Math.floor(available / unitBytes)) +
        "x".repeat(available % unitBytes) +
        suffix;
      expect(Buffer.byteLength(body)).toBe(size);
      expect(run(body, stream)).toEqual({ action: "pass" });
      expect(run(body, stream, "different")).toMatchObject(reason("model_mismatch"));
    }
  });

  it("keeps expected model and failures local across sessions", () => {
    expect(run(done({ model: "session-b", status: "completed" }), true, "session-a")).toMatchObject(
      reason("model_mismatch")
    );
    expect(run(done({ model: "session-b", status: "completed" }), true, "session-b")).toEqual({
      action: "pass",
    });
    expect(run(done(), true)).toEqual({ action: "pass" });
    expect(run(done(), true, null)).toMatchObject({ action: "block" });
  });
});
