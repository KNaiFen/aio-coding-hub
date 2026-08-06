import type { ProviderModelPolicyV1, ProviderModelRule } from "../../services/providers/providers";

export const DEFAULT_PROVIDER_MODEL_POLICY: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  rules: [],
};

export function cloneProviderModelPolicy(policy: ProviderModelPolicyV1): ProviderModelPolicyV1 {
  return {
    version: policy.version,
    mode: policy.mode,
    rules: policy.rules.map((rule) => ({ ...rule })),
  };
}

export function validateProviderModelPolicy(policy: ProviderModelPolicyV1): string | null {
  if (policy.version !== 1) return "模型策略版本不受支持";
  if (policy.rules.length > 500) return "模型策略最多支持 500 条规则";
  if (policy.mode === "selected" && policy.rules.length === 0) {
    return "选定模型模式至少需要一条规则";
  }

  const seen = new Set<string>();
  for (const rule of policy.rules) {
    const source = rule.source.trim();
    const target = rule.target?.trim() ?? "";
    if (!source) return "源模型不能为空";
    if (Array.from(source).length > 200) return "源模型最多 200 个字符";
    if (Array.from(target).length > 200) return "目标模型最多 200 个字符";
    if ((source.match(/\*/g) ?? []).length > 1) return "源模型最多包含一个 *";
    if ((target.match(/\*/g) ?? []).length > 1) return "目标模型最多包含一个 *";
    if (!source.includes("*") && target.includes("*")) {
      return "目标模型使用 * 时，源模型也必须使用 *";
    }
    if (!seen.has(source)) {
      seen.add(source);
    } else {
      return "源模型不能重复";
    }
  }
  return null;
}

export function normalizeProviderModelPolicyDraft(policy: ProviderModelPolicyV1) {
  return {
    ...policy,
    rules: policy.rules.map<ProviderModelRule>((rule) => ({
      source: rule.source.trim(),
      target: rule.target?.trim() || null,
    })),
  };
}
