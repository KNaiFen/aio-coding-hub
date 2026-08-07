import { ArrowRight, ChevronDown, Plus, RefreshCw, Search, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "../../ui/Button";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { RadioGroup } from "../../ui/RadioGroup";
import type {
  ClaudeModels,
  CliKey,
  ProviderModelPolicyStatus,
  ProviderModelPolicyV1,
} from "../../services/providers/providers";
import {
  cloneProviderModelPolicy,
  DEFAULT_PROVIDER_MODEL_POLICY,
  type ProviderModelDiscoveryUiState,
  normalizeProviderModelPolicyDraft,
  validateProviderModelPolicy,
} from "./providerModelPolicy";

export type ProviderModelPolicySectionProps = {
  cliKey: CliKey;
  status: ProviderModelPolicyStatus;
  policy: ProviderModelPolicyV1 | null;
  legacyClaudeModels?: ClaudeModels | null;
  saving: boolean;
  onChange: (policy: ProviderModelPolicyV1) => void;
  modelDiscoveryState: ProviderModelDiscoveryUiState;
  onDiscoverModels: () => void | Promise<void>;
  hasMultipleBaseUrls: boolean;
};

const MODE_OPTIONS = [
  { value: "all", label: "全部可用", description: "默认可用，显式模型优先路由到此 Provider" },
  { value: "selected", label: "仅这些可用", description: "只接收列出或映射的模型" },
  { value: "excluded", label: "排除这些", description: "列出的模型不可用，其余模型默认可用" },
];

export function ProviderModelPolicySection({
  cliKey,
  status,
  policy,
  legacyClaudeModels,
  saving,
  onChange,
  modelDiscoveryState,
  onDiscoverModels,
  hasMultipleBaseUrls,
}: ProviderModelPolicySectionProps) {
  const [search, setSearch] = useState("");
  const [localDraft, setLocalDraft] = useState<ProviderModelPolicyV1>(() =>
    cloneProviderModelPolicy(policy ?? DEFAULT_PROVIDER_MODEL_POLICY)
  );
  const [editingLegacy, setEditingLegacy] = useState(false);
  const [showCutoverWarning, setShowCutoverWarning] = useState(false);
  const patternRefs = useRef<Record<number, HTMLInputElement | null>>({});
  const mappingRefs = useRef<Record<number, HTMLInputElement | null>>({});
  const patternAddButtonRef = useRef<HTMLButtonElement | null>(null);
  const mappingAddButtonRef = useRef<HTMLButtonElement | null>(null);
  const focusRef = useRef<{ kind: "pattern" | "mapping"; index: number } | null>(null);
  const previousStatusRef = useRef(status);

  useEffect(() => {
    if (status === "ready" && policy) setLocalDraft(cloneProviderModelPolicy(policy));
    const focus = focusRef.current;
    if (!focus) return;
    (focus.kind === "pattern" ? patternRefs : mappingRefs).current[focus.index]?.focus();
    focusRef.current = null;
  }, [policy, status]);

  useEffect(() => {
    if (previousStatusRef.current === "legacy" && status === "ready") {
      setShowCutoverWarning(true);
    }
    previousStatusRef.current = status;
  }, [status]);

  const currentPolicy = status === "ready" && policy ? policy : localDraft;
  const query = search.trim().toLowerCase();
  const visiblePatterns = currentPolicy.modelPatterns
    .map((pattern, index) => ({ pattern, index }))
    .filter(({ pattern }) => !query || pattern.toLowerCase().includes(query));
  const visibleMappings = currentPolicy.mappings
    .map((mapping, index) => ({ mapping, index }))
    .filter(
      ({ mapping }) => !query || `${mapping.source} ${mapping.target}`.toLowerCase().includes(query)
    );
  const policyError = validateProviderModelPolicy(currentPolicy);
  const canEdit = status === "ready" || editingLegacy;
  const entryCount = new Set([
    ...currentPolicy.modelPatterns,
    ...currentPolicy.mappings.map((mapping) => mapping.source),
  ]).size;

  const emit = (next: ProviderModelPolicyV1) => {
    const normalized = normalizeProviderModelPolicyDraft(next);
    setLocalDraft(normalized);
    onChange(normalized);
  };

  const updatePattern = (index: number, value: string) => {
    emit({
      ...currentPolicy,
      modelPatterns: currentPolicy.modelPatterns.map((pattern, patternIndex) =>
        patternIndex === index ? value : pattern
      ),
    });
  };

  const addPattern = () => {
    const index = currentPolicy.modelPatterns.length;
    emit({ ...currentPolicy, modelPatterns: [...currentPolicy.modelPatterns, ""] });
    focusRef.current = { kind: "pattern", index };
  };

  const deletePattern = (index: number) => {
    const modelPatterns = currentPolicy.modelPatterns.filter(
      (_, patternIndex) => patternIndex !== index
    );
    emit({ ...currentPolicy, modelPatterns });
    if (modelPatterns.length === 0) patternAddButtonRef.current?.focus();
    else focusRef.current = { kind: "pattern", index: Math.min(index, modelPatterns.length - 1) };
  };

  const updateMapping = (index: number, field: "source" | "target", value: string) => {
    emit({
      ...currentPolicy,
      mappings: currentPolicy.mappings.map((mapping, mappingIndex) =>
        mappingIndex === index ? { ...mapping, [field]: value } : mapping
      ),
    });
  };

  const addMapping = () => {
    const index = currentPolicy.mappings.length;
    emit({
      ...currentPolicy,
      mappings: [...currentPolicy.mappings, { source: "", target: "" }],
    });
    focusRef.current = { kind: "mapping", index };
  };

  const deleteMapping = (index: number) => {
    const mappings = currentPolicy.mappings.filter((_, mappingIndex) => mappingIndex !== index);
    emit({ ...currentPolicy, mappings });
    if (mappings.length === 0) mappingAddButtonRef.current?.focus();
    else focusRef.current = { kind: "mapping", index: Math.min(index, mappings.length - 1) };
  };

  const legacyMappings = [
    ["主模型", legacyClaudeModels?.main_model],
    ["推理模型 (Thinking)", legacyClaudeModels?.reasoning_model],
    ["Haiku", legacyClaudeModels?.haiku_model],
    ["Sonnet", legacyClaudeModels?.sonnet_model],
    ["Opus", legacyClaudeModels?.opus_model],
  ].filter(([, value]) => typeof value === "string" && value.trim());

  const enterGenericPolicy = () => {
    setEditingLegacy(true);
    setShowCutoverWarning(true);
    emit(cloneProviderModelPolicy(DEFAULT_PROVIDER_MODEL_POLICY));
  };

  const resetInvalidPolicy = () => {
    setEditingLegacy(true);
    emit(cloneProviderModelPolicy(DEFAULT_PROVIDER_MODEL_POLICY));
  };

  const discoveryRow = (legacy: boolean) => (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <p role="status" aria-live="polite" className="min-w-0 text-xs text-muted-foreground">
        {discoveryMessage(modelDiscoveryState, currentPolicy.mode)}
        {discoveryEndpoint(modelDiscoveryState)}
        {hasMultipleBaseUrls ? " · 多地址建议拆分 Provider" : ""}
        {legacy ? " · 获取后生成通用策略草稿" : ""}
      </p>
      <Button
        type="button"
        variant="secondary"
        onClick={() => void onDiscoverModels()}
        disabled={saving || modelDiscoveryState.status === "loading"}
        aria-busy={modelDiscoveryState.status === "loading"}
      >
        <RefreshCw
          className={`h-4 w-4 ${modelDiscoveryState.status === "loading" ? "animate-spin" : ""}`}
          aria-hidden="true"
        />
        获取上游模型
      </Button>
    </div>
  );

  return (
    <details
      data-cli-key={cliKey}
      className="group rounded-lg border border-border bg-surface-panel shadow-sm open:ring-2 open:ring-ring/10"
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 select-none">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <span className="text-sm font-semibold text-foreground">模型路由</span>
          <span className="text-xs text-muted-foreground">
            {policySummary(status, currentPolicy)}
          </span>
        </div>
        <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-4 border-t border-border px-4 py-3">
        {status === "legacy" && !editingLegacy ? (
          <div className="space-y-3 text-sm text-muted-foreground">
            <p className="font-medium text-foreground">当前 Claude 使用旧版模型映射</p>
            <div aria-label="旧版模型映射摘要" className="space-y-1 text-xs">
              {legacyMappings.length > 0 ? (
                <ul className="space-y-1">
                  {legacyMappings.map(([label, value]) => (
                    <li key={label} className="flex flex-wrap gap-x-2">
                      <span>{label}：</span>
                      <code className="break-all text-foreground">{value}</code>
                    </li>
                  ))}
                </ul>
              ) : (
                <p>未配置，沿用请求模型。</p>
              )}
            </div>
            {discoveryRow(true)}
            <Button
              type="button"
              variant="secondary"
              onClick={enterGenericPolicy}
              disabled={saving}
            >
              改用通用模型策略
            </Button>
          </div>
        ) : status === "invalid" && !editingLegacy ? (
          <div
            role="alert"
            className="space-y-3 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm text-foreground"
          >
            <p>模型策略无效，当前请求不会使用该 Provider。</p>
            {discoveryRow(false)}
            <Button
              type="button"
              variant="secondary"
              onClick={resetInvalidPolicy}
              disabled={saving}
            >
              重置为全部可用
            </Button>
          </div>
        ) : (
          <>
            {showCutoverWarning ? (
              <p
                role="alert"
                className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm text-foreground"
              >
                保存后无法在界面切回旧策略
              </p>
            ) : null}

            <section className="space-y-3" aria-labelledby={`${cliKey}-model-range-title`}>
              <div className="space-y-1">
                <h3
                  id={`${cliKey}-model-range-title`}
                  className="text-sm font-semibold text-foreground"
                >
                  模型范围
                </h3>
                <p className="text-xs text-muted-foreground">{modeHint(currentPolicy.mode)}</p>
              </div>

              <RadioGroup
                name={`${cliKey}-provider-model-mode`}
                ariaLabel="模型范围"
                value={currentPolicy.mode}
                onChange={(mode) =>
                  emit({
                    ...currentPolicy,
                    mode: mode as ProviderModelPolicyV1["mode"],
                  })
                }
                options={MODE_OPTIONS}
                disabled={saving || !canEdit}
              />

              {discoveryRow(false)}

              <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
                <FormField label="搜索模型">
                  <div className="relative">
                    <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      aria-label="搜索模型"
                      value={search}
                      onChange={(event) => setSearch(event.currentTarget.value)}
                      placeholder="搜索范围或映射"
                      className="pl-8"
                      disabled={saving}
                    />
                  </div>
                </FormField>
                <Button
                  ref={patternAddButtonRef}
                  type="button"
                  variant="secondary"
                  onClick={addPattern}
                  disabled={saving || !canEdit || entryCount >= 500}
                >
                  <Plus className="h-4 w-4" aria-hidden="true" />
                  {rangeAddLabel(currentPolicy.mode)}
                </Button>
              </div>

              <div className="space-y-2">
                <p className="text-xs font-semibold text-muted-foreground">
                  {rangeListLabel(currentPolicy.mode)}
                </p>
                {visiblePatterns.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    暂无{rangeListLabel(currentPolicy.mode)}
                  </p>
                ) : null}
                {visiblePatterns.map(({ pattern, index }) => (
                  <div key={index} className="flex items-end gap-2">
                    <FormField label={`${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}>
                      <Input
                        ref={(element) => {
                          patternRefs.current[index] = element;
                        }}
                        aria-label={`${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}
                        value={pattern}
                        onChange={(event) => updatePattern(index, event.currentTarget.value)}
                        placeholder="例如 gpt-5.6-luna 或 gpt-*"
                        disabled={saving || !canEdit}
                        mono
                      />
                    </FormField>
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon"
                      className="h-10 w-10 shrink-0"
                      aria-label={`删除${rangeItemLabel(currentPolicy.mode)} ${index + 1}`}
                      title="删除"
                      onClick={() => deletePattern(index)}
                      disabled={saving || !canEdit}
                    >
                      <Trash2 className="h-4 w-4" aria-hidden="true" />
                    </Button>
                  </div>
                ))}
              </div>
            </section>

            <section
              className="space-y-3 border-t border-border pt-4"
              aria-labelledby={`${cliKey}-model-mapping-title`}
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h3
                  id={`${cliKey}-model-mapping-title`}
                  className="text-sm font-semibold text-foreground"
                >
                  模型映射（可选）
                </h3>
                <Button
                  ref={mappingAddButtonRef}
                  type="button"
                  variant="secondary"
                  onClick={addMapping}
                  disabled={saving || !canEdit || entryCount >= 500}
                >
                  <Plus className="h-4 w-4" aria-hidden="true" />
                  添加映射
                </Button>
              </div>

              {visibleMappings.length === 0 ? (
                <p className="text-sm text-muted-foreground">暂无模型映射</p>
              ) : null}
              <div className="space-y-2">
                {visibleMappings.map(({ mapping, index }) => (
                  <div
                    key={index}
                    className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_1rem_minmax(0,1fr)_2.5rem] md:items-end"
                  >
                    <FormField label="请求模型">
                      <Input
                        ref={(element) => {
                          mappingRefs.current[index] = element;
                        }}
                        aria-label={`请求模型 ${index + 1}`}
                        value={mapping.source}
                        onChange={(event) =>
                          updateMapping(index, "source", event.currentTarget.value)
                        }
                        placeholder="例如 gpt-5.6-luna"
                        disabled={saving || !canEdit}
                        mono
                      />
                    </FormField>
                    <ArrowRight
                      className="mb-3 hidden h-4 w-4 text-muted-foreground md:block"
                      aria-hidden="true"
                    />
                    <FormField label="上游模型">
                      <Input
                        aria-label={`上游模型 ${index + 1}`}
                        value={mapping.target}
                        onChange={(event) =>
                          updateMapping(index, "target", event.currentTarget.value)
                        }
                        placeholder="例如 deepseek-v4-flash"
                        disabled={saving || !canEdit}
                        mono
                      />
                    </FormField>
                    <Button
                      type="button"
                      variant="secondary"
                      size="icon"
                      className="h-10 w-10 justify-self-end"
                      aria-label={`删除模型映射 ${index + 1}`}
                      title="删除映射"
                      onClick={() => deleteMapping(index)}
                      disabled={saving || !canEdit}
                    >
                      <Trash2 className="h-4 w-4" aria-hidden="true" />
                    </Button>
                  </div>
                ))}
              </div>
            </section>

            {policyError ? (
              <p role="alert" className="text-xs text-destructive">
                {policyError}
              </p>
            ) : null}
          </>
        )}
      </div>
    </details>
  );
}

function policySummary(status: ProviderModelPolicyStatus, policy: ProviderModelPolicyV1) {
  if (status === "legacy") return "旧版";
  if (status === "invalid") return "无效";
  const mapping = policy.mappings.length ? ` · 映射 ${policy.mappings.length}` : "";
  if (policy.mode === "all") {
    const explicit = policy.modelPatterns.length ? ` · 优先 ${policy.modelPatterns.length}` : "";
    return `全部可用${explicit}${mapping}`;
  }
  if (policy.mode === "selected") {
    return `仅 ${new Set([...policy.modelPatterns, ...policy.mappings.map((item) => item.source)]).size} 个模型${mapping}`;
  }
  return `排除 ${policy.modelPatterns.length} 个${mapping}`;
}

function modeHint(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "未列出的模型也可用；列出的模型会优先路由到此 Provider。";
  if (mode === "selected") return "只接收下列模型和映射中的请求模型。";
  return "下列模型不可用；其余模型保持可用。";
}

function rangeListLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "优先模型";
  if (mode === "selected") return "可用模型";
  return "排除模型";
}

function rangeItemLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "优先模型";
  if (mode === "selected") return "可用模型";
  return "排除模型";
}

function rangeAddLabel(mode: ProviderModelPolicyV1["mode"]) {
  if (mode === "all") return "添加优先模型";
  if (mode === "selected") return "添加可用模型";
  return "添加排除模型";
}

function discoveryEndpoint(state: ProviderModelDiscoveryUiState) {
  if (state.status !== "ready" && state.status !== "empty" && state.status !== "capacity") {
    return "";
  }
  const index = state.baseUrlIndex == null ? "" : ` · 地址 ${state.baseUrlIndex}`;
  return ` · ${state.origin}${index}`;
}

function discoveryMessage(
  state: ProviderModelDiscoveryUiState,
  mode: ProviderModelPolicyV1["mode"]
) {
  switch (state.status) {
    case "idle":
      return "尚未获取上游模型";
    case "loading":
      return "正在获取上游模型…";
    case "changed":
      return "连接已变化，请重新获取";
    case "ready":
      if (mode === "excluded") return `已获取 ${state.discoveredCount} 个 · 排除列表未修改`;
      return state.addedCount > 0
        ? `已获取 ${state.discoveredCount} 个 · 新增 ${state.addedCount}`
        : `已获取 ${state.discoveredCount} 个 · 无新增`;
    case "empty":
      return "上游未返回模型";
    case "capacity":
      return `发现 ${state.discoveredCount} 个，超过 500 个上限`;
    case "unsupported":
      return state.reason === "cx_2cc"
        ? "CX2CC 请在对应 Codex Provider 获取"
        : "当前 OAuth 连接不支持获取";
    case "error":
      return {
        invalid_config: "连接配置不完整，请检查 Base URL、认证方式和 API Key",
        redirect: "端点发生重定向，请配置最终 endpoint",
        unauthorized: "认证失败，请检查 API Key 或 OAuth 登录状态",
        timeout: "获取超时，请重试",
        network: "无法连接上游，请检查 endpoint、代理和网络",
        invalid_response: "上游模型目录格式无法使用",
        too_large: "上游模型目录过大，无法合并",
      }[state.code];
    case "unexpected_error":
      return "获取失败，请查看应用日志后重试";
  }
}
