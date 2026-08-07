import type {
  ProviderModelDiscoveryErrorCode,
  ProviderModelDiscoveryUnsupportedReason,
  ProviderModelMapping,
  ProviderModelPolicyV1,
} from "../../services/providers/providers";

export const DEFAULT_PROVIDER_MODEL_POLICY: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  modelPatterns: [],
  mappings: [],
};

export function cloneProviderModelPolicy(policy: ProviderModelPolicyV1): ProviderModelPolicyV1 {
  return {
    version: policy.version,
    mode: policy.mode,
    modelPatterns: [...policy.modelPatterns],
    mappings: policy.mappings.map((mapping) => ({ ...mapping })),
  };
}

export function validateProviderModelPolicy(policy: ProviderModelPolicyV1): string | null {
  if (policy.version !== 1) return "模型策略版本不受支持";
  if (policy.modelPatterns.length > 500 || policy.mappings.length > 500) {
    return "模型策略最多支持 500 个模型";
  }

  const patternSources = new Set<string>();
  for (const rawPattern of policy.modelPatterns) {
    const pattern = rawPattern.trim();
    const error = validateModelPattern(pattern, "模型");
    if (error) return error;
    if (patternSources.has(pattern)) return "模型不能重复";
    patternSources.add(pattern);
  }

  const mappingSources = new Set<string>();
  for (const mapping of policy.mappings) {
    const source = mapping.source.trim();
    const target = mapping.target.trim();
    const sourceError = validateModelPattern(source, "请求模型");
    if (sourceError) return sourceError;
    if (!target) return "上游模型不能为空";
    if (Array.from(target).length > 200) return "上游模型最多 200 个字符";
    if ((target.match(/\*/g) ?? []).length > 1) return "上游模型最多包含一个 *";
    if (!source.includes("*") && target.includes("*")) {
      return "上游模型使用 * 时，请求模型也必须使用 *";
    }
    if (mappingSources.has(source)) return "请求模型不能重复";
    mappingSources.add(source);
  }

  const uniqueSources = new Set([...patternSources, ...mappingSources]);
  if (uniqueSources.size > 500) return "模型策略最多支持 500 个模型";
  if (policy.mode === "selected" && uniqueSources.size === 0) {
    return "仅这些可用模式至少需要一个模型或映射";
  }
  return null;
}

function validateModelPattern(value: string, label: string) {
  if (!value) return `${label}不能为空`;
  if (Array.from(value).length > 200) return `${label}最多 200 个字符`;
  if ((value.match(/\*/g) ?? []).length > 1) return `${label}最多包含一个 *`;
  return null;
}

export function normalizeProviderModelPolicyDraft(policy: ProviderModelPolicyV1) {
  return {
    ...policy,
    modelPatterns: policy.modelPatterns.map((pattern) => pattern.trim()),
    mappings: policy.mappings.map<ProviderModelMapping>((mapping) => ({
      source: mapping.source.trim(),
      target: mapping.target.trim(),
    })),
  };
}

export type MergeDiscoveredModelIdsResult = {
  capacityExceeded: boolean;
  addedCount: number;
  policy: ProviderModelPolicyV1;
};

export type ProviderModelDiscoveryUiState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "changed" }
  | {
      status: "ready";
      discoveredCount: number;
      addedCount: number;
      origin: string;
      baseUrlIndex: number | null;
    }
  | { status: "empty"; origin: string; baseUrlIndex: number | null }
  | { status: "capacity"; discoveredCount: number; origin: string; baseUrlIndex: number | null }
  | { status: "unsupported"; reason: ProviderModelDiscoveryUnsupportedReason }
  | { status: "error"; code: ProviderModelDiscoveryErrorCode }
  | { status: "unexpected_error" };

function sourceMatchesModel(source: string, modelId: string) {
  const wildcardIndex = source.indexOf("*");
  if (wildcardIndex < 0) return source === modelId;
  return (
    modelId.startsWith(source.slice(0, wildcardIndex)) &&
    modelId.endsWith(source.slice(wildcardIndex + 1))
  );
}

export function mergeDiscoveredModelIds(
  policy: ProviderModelPolicyV1,
  discoveredIds: string[]
): MergeDiscoveredModelIdsResult {
  if (policy.mode === "excluded") {
    return { capacityExceeded: false, addedCount: 0, policy };
  }

  const normalizedIds = [...new Set(discoveredIds.map((id) => id.trim()).filter(Boolean))].sort();
  const configuredSources = [
    ...policy.modelPatterns,
    ...policy.mappings.map((mapping) => mapping.source),
  ];
  const additions = normalizedIds.filter(
    (modelId) => !configuredSources.some((source) => sourceMatchesModel(source.trim(), modelId))
  );

  if (new Set([...configuredSources.map((source) => source.trim()), ...additions]).size > 500) {
    return { capacityExceeded: true, addedCount: 0, policy };
  }

  return {
    capacityExceeded: false,
    addedCount: additions.length,
    policy: additions.length
      ? { ...policy, modelPatterns: [...policy.modelPatterns, ...additions] }
      : policy,
  };
}
