// Ordinary Extension Host plugin: all state belongs to this one hook invocation.
const PREFIX = "codex_model_consistency.";

function reject(reason, message) {
  return { action: "switchProvider", reasonCode: PREFIX + reason, message };
}

// JSON.parse validates the grammar. Scan direct member boundaries without a
// regex over output strings: long escaped text must stay within the worker budget.
// Nested output/tool members are opaque; only the protocol response is inspected.
function protocolObject(text, value = JSON.parse(text)) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Expected a protocol object");
  }
  const keys = new Set();
  let responseText;
  let depth = 0;
  let key = null;
  let valueStart = -1;
  let expectKey = false;
  for (let offset = 0; offset < text.length; offset += 1) {
    const part = text[offset];
    if (part === '"') {
      const start = offset;
      offset += 1;
      while (offset < text.length && text[offset] !== '"') {
        // JSON grammar is already valid, so an escape always has a next unit.
        offset += text[offset] === "\\" ? 2 : 1;
      }
      if (depth === 1 && expectKey) {
        key = JSON.parse(text.slice(start, offset + 1));
        if (keys.has(key)) throw new Error("Duplicate protocol member");
        keys.add(key);
        expectKey = false;
      }
      continue;
    }
    if (depth === 1 && part === ":") {
      valueStart = offset + 1;
    } else if (depth === 1 && (part === "," || part === "}")) {
      if (key === "response") responseText = text.slice(valueStart, offset);
      key = null;
      expectKey = part === ",";
    }
    if (part === "{" || part === "[") {
      depth += 1;
      if (depth === 1) expectKey = true;
    } else if (part === "}" || part === "]") {
      depth -= 1;
    }
  }
  return { value, responseText };
}

function inspect(context) {
  const expected = context.outboundRequest.model;
  if (typeof expected !== "string" || expected.trim() === "") {
    return {
      action: "block",
      reasonCode: PREFIX + "expected_model_missing",
      message: "最终出站请求没有有效的模型名称，无法校验响应。",
    };
  }

  let declarations = 0;
  let completed = false;
  let failure = null;
  function inspectObject(object) {
    if (Object.prototype.hasOwnProperty.call(object, "model")) {
      declarations += 1;
      if (typeof object.model !== "string" || object.model.trim() === "") {
        failure = reject("invalid_model", "上游响应包含无效的模型声明。");
      } else if (object.model !== expected) {
        failure = reject("model_mismatch", "上游声明的模型与最终出站模型不一致。");
      }
    }
    if (object.status === "failed" || object.status === "incomplete") {
      failure = reject("response_unsuccessful", "上游响应未成功完成。");
    }
  }

  function inspectPayload(text, eventName, streaming) {
    const root = protocolObject(text);
    inspectObject(root.value);
    let response = root.value;
    if (root.responseText !== undefined) {
      response = protocolObject(root.responseText, root.value.response).value;
      inspectObject(response);
    }
    const type = root.value.type ?? eventName;
    if (eventName && root.value.type && eventName !== root.value.type) {
      failure = reject("invalid_response", "上游事件类型相互矛盾。");
    }
    if (type === "response.failed" || type === "response.incomplete" || type === "error") {
      failure = reject("response_unsuccessful", "上游响应未成功完成。");
    }
    if ((!streaming || type === "response.completed") && response.status === "completed") {
      completed = true;
    }
  }

  const contentType = (context.response.contentType ?? "").split(";", 1)[0].trim().toLowerCase();
  try {
    if (contentType === "application/json") {
      inspectPayload(context.response.body, null, false);
    } else if (contentType === "text/event-stream") {
      let data = [];
      let eventName = null;
      // An SSE event may have several data lines; join them before JSON parsing.
      const lines = context.response.body.replace(/^\uFEFF/, "").split(/\r\n|\r|\n/);
      // A trailing newline ends a line; it does not itself dispatch an SSE event.
      if (lines[lines.length - 1] === "") lines.pop();
      for (const line of lines) {
        if (line === "") {
          if (data.length > 0) {
            const text = data.join("\n");
            if (text !== "[DONE]") inspectPayload(text, eventName, true);
          }
          data = [];
          eventName = null;
          continue;
        }
        const colon = line.indexOf(":");
        const field = colon === -1 ? line : line.slice(0, colon);
        let value = colon === -1 ? "" : line.slice(colon + 1);
        if (value.startsWith(" ")) value = value.slice(1);
        if (field === "data") data.push(value);
        if (field === "event") eventName = value;
      }
      if (data.length > 0) return reject("incomplete_response", "上游流式响应事件未完整结束。");
    } else {
      return reject("unsupported_content_type", "上游未返回受支持的 JSON 或 SSE 响应。");
    }
  } catch {
    return reject("invalid_response", "上游响应格式无效或协议字段存在重复键。");
  }
  if (failure) return failure;
  if (!completed) return reject("incomplete_response", "上游响应缺少成功完成状态。");
  if (declarations === 0) return reject("model_missing", "上游响应未声明模型。");
  return { action: "pass" };
}

module.exports.activate = function (api) {
  api.gateway.registerHook("gateway.response.beforeCommit", (ctx) => inspect(ctx.context));
};
