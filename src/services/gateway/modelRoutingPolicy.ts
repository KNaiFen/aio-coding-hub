import type { ModelRoutingPolicy, ModelRoutingRule } from "../../generated/bindings";

export const MAX_MODEL_ROUTING_RULES = 128;
export const MAX_MODEL_ROUTING_MODEL_BYTES = 256;
export const MODEL_ROUTING_REASONING_EFFORTS = [
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
  "ultra",
] as const;
export type ModelRoutingReasoningEffort = (typeof MODEL_ROUTING_REASONING_EFFORTS)[number];
const MODEL_ROUTING_REASONING_EFFORT_SET = new Set<string>(MODEL_ROUTING_REASONING_EFFORTS);

export const DEFAULT_MODEL_ROUTING_POLICY: ModelRoutingPolicy = {
  enabled: false,
  rules: [],
};

export function cloneModelRoutingPolicy(
  policy: ModelRoutingPolicy | null | undefined
): ModelRoutingPolicy {
  const source = policy ?? DEFAULT_MODEL_ROUTING_POLICY;
  return {
    enabled: source.enabled,
    rules: source.rules.map((rule) => ({ ...rule })),
  };
}

export function emptyModelRoutingRule(): ModelRoutingRule {
  return {
    source_model: "",
    source_reasoning_effort: null,
    target_model: null,
    reasoning_effort: null,
  };
}

function normalizedOptional(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized.length > 0 ? normalized : null;
}

function normalizedEffort(value: string | null | undefined): string | null {
  return normalizedOptional(value)?.toLowerCase() ?? null;
}

function containsControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

export function normalizeModelRoutingPolicy(policy: ModelRoutingPolicy): ModelRoutingPolicy {
  return {
    enabled: policy.enabled,
    rules: policy.rules.map((rule) => ({
      source_model: rule.source_model.trim(),
      source_reasoning_effort: normalizedEffort(rule.source_reasoning_effort),
      target_model: normalizedOptional(rule.target_model),
      reasoning_effort: normalizedEffort(rule.reasoning_effort),
    })),
  };
}

export function validateModelRoutingPolicy(policy: ModelRoutingPolicy): string | null {
  if (policy.rules.length > MAX_MODEL_ROUTING_RULES) {
    return `模型路由规则最多 ${MAX_MODEL_ROUTING_RULES} 条`;
  }

  const seen = new Set<string>();
  for (const [index, rawRule] of policy.rules.entries()) {
    const rule = {
      source_model: rawRule.source_model.trim(),
      source_reasoning_effort: normalizedEffort(rawRule.source_reasoning_effort),
      target_model: normalizedOptional(rawRule.target_model),
      reasoning_effort: normalizedEffort(rawRule.reasoning_effort),
    };
    const label = `第 ${index + 1} 条模型路由`;
    if (!rule.source_model) return `${label}必须填写来源模型`;
    if (new TextEncoder().encode(rule.source_model).length > MAX_MODEL_ROUTING_MODEL_BYTES) {
      return `${label}的来源模型不能超过 ${MAX_MODEL_ROUTING_MODEL_BYTES} 字节`;
    }
    if (containsControlCharacter(rule.source_model)) return `${label}的来源模型包含控制字符`;
    if (
      rule.source_reasoning_effort != null &&
      !MODEL_ROUTING_REASONING_EFFORT_SET.has(rule.source_reasoning_effort)
    ) {
      return `${label}的来源思考强度不是受支持的标准值`;
    }
    const sourceKey = `${rule.source_model}\u0000${rule.source_reasoning_effort ?? ""}`;
    if (seen.has(sourceKey)) return `${label}与已有来源模型及思考强度重复`;
    seen.add(sourceKey);

    if (rule.target_model) {
      if (new TextEncoder().encode(rule.target_model).length > MAX_MODEL_ROUTING_MODEL_BYTES) {
        return `${label}的目标模型不能超过 ${MAX_MODEL_ROUTING_MODEL_BYTES} 字节`;
      }
      if (containsControlCharacter(rule.target_model)) return `${label}的目标模型包含控制字符`;
    }
    if (rule.reasoning_effort) {
      if (!MODEL_ROUTING_REASONING_EFFORT_SET.has(rule.reasoning_effort)) {
        return `${label}的目标思考强度不是受支持的标准值`;
      }
    }
    if (!rule.target_model && !rule.reasoning_effort) {
      return `${label}至少填写目标模型或思考强度`;
    }
  }

  return null;
}
