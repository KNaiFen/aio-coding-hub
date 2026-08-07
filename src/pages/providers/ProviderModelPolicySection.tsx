import { ChevronDown, Plus, RefreshCw, Search, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
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
  const sourceRefs = useRef<Record<number, HTMLInputElement | null>>({});
  const addButtonRef = useRef<HTMLButtonElement | null>(null);
  const focusRuleIndexRef = useRef<number | null>(null);
  const previousStatusRef = useRef(status);

  useEffect(() => {
    if (status === "ready" && policy) {
      setLocalDraft(cloneProviderModelPolicy(policy));
    }
    if (focusRuleIndexRef.current != null) {
      sourceRefs.current[focusRuleIndexRef.current]?.focus();
      focusRuleIndexRef.current = null;
    }
  }, [policy, status]);

  useEffect(() => {
    if (previousStatusRef.current === "legacy" && status === "ready") {
      setShowCutoverWarning(true);
    }
    previousStatusRef.current = status;
  }, [status]);

  const currentPolicy = status === "ready" && policy ? policy : localDraft;
  const visibleRules = currentPolicy.rules
    .map((rule, index) => ({ rule, index }))
    .filter(({ rule }) => {
      const query = search.trim().toLowerCase();
      if (!query) return true;
      return `${rule.source} ${rule.target ?? ""}`.toLowerCase().includes(query);
    });
  const policyError = validateProviderModelPolicy(currentPolicy);
  const canEdit = status === "ready" || editingLegacy;

  const emit = (next: ProviderModelPolicyV1) => {
    const normalized = normalizeProviderModelPolicyDraft(next);
    setLocalDraft(normalized);
    onChange(normalized);
  };

  const updateRule = (index: number, field: "source" | "target", value: string) => {
    emit({
      ...currentPolicy,
      rules: currentPolicy.rules.map((rule, ruleIndex) =>
        ruleIndex === index
          ? { ...rule, [field]: field === "target" ? value || null : value }
          : rule
      ),
    });
  };

  const addRule = () => {
    const nextIndex = currentPolicy.rules.length;
    emit({
      ...currentPolicy,
      rules: [...currentPolicy.rules, { source: "", target: null }],
    });
    focusRuleIndexRef.current = nextIndex;
  };

  const deleteRule = (index: number) => {
    const nextRules = currentPolicy.rules.filter((_, ruleIndex) => ruleIndex !== index);
    emit({ ...currentPolicy, rules: nextRules });
    if (nextRules.length === 0) {
      addButtonRef.current?.focus();
      return;
    }
    focusRuleIndexRef.current = Math.min(index, nextRules.length - 1);
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

  const discoveryPanel = (legacy: boolean) => (
    <div className="space-y-1.5 rounded-md border border-border bg-secondary/50 px-3 py-2 text-xs text-muted-foreground">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div role="status" aria-live="polite" className="min-w-0 space-y-1">
          {discoveryOrigin(modelDiscoveryState) ? (
            <p>
              <span className="font-medium text-foreground">仅代表当前端点</span>
              <code className="ml-2 break-all text-foreground">
                {discoveryOrigin(modelDiscoveryState)}
              </code>
              {discoveryBaseUrlIndex(modelDiscoveryState) != null ? (
                <span className="ml-2">地址 {discoveryBaseUrlIndex(modelDiscoveryState)}</span>
              ) : null}
            </p>
          ) : null}
          <p>{discoveryMessage(modelDiscoveryState)}</p>
          {hasMultipleBaseUrls ? <p>多个地址的模型能力若不同，请拆分为独立 Provider。</p> : null}
        </div>
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
      {legacy ? (
        <p>获取成功后会进入通用模型策略草稿，保存前可取消。</p>
      ) : (
        <>
          <p>
            {currentPolicy.mode === "all"
              ? "全部模型：新增规则不会限制未列出的模型。"
              : "仅选定模型：规则决定模型资格。"}
          </p>
          <p>规则不改变供应商排序。</p>
        </>
      )}
    </div>
  );

  return (
    <details
      data-cli-key={cliKey}
      className="group rounded-lg border border-border bg-surface-panel shadow-sm open:ring-2 open:ring-ring/10"
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 select-none">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-semibold text-foreground">模型路由策略</span>
            <span className="text-xs text-muted-foreground">
              {status === "legacy"
                ? "旧版"
                : status === "invalid"
                  ? "无效"
                  : `${currentPolicy.rules.length} 条规则`}
            </span>
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            规则只决定模型资格和重定向，不改变供应商排序
          </p>
        </div>
        <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-4 border-t border-border px-4 py-3">
        {status === "legacy" && !editingLegacy ? (
          <div className="space-y-3 text-sm text-muted-foreground">
            <p>当前 Claude 仍使用旧版模型映射</p>
            <div aria-label="旧版模型映射摘要" className="space-y-1 text-xs">
              <p className="font-medium text-foreground">当前旧版映射</p>
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
            <p>现有路由行为保持不变；Thinking 槽位不会自动转换为通用规则。</p>
            {discoveryPanel(true)}
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
            <p>模型策略无效，当前请求不会使用该供应商。</p>
            {discoveryPanel(false)}
            <Button
              type="button"
              variant="secondary"
              onClick={resetInvalidPolicy}
              disabled={saving}
            >
              重置为全部模型（保存后恢复全量路由）
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

            {discoveryPanel(false)}

            <FormField
              label="模型匹配模式"
              hint="all 未命中时仍可调用；selected 只允许命中规则的模型"
            >
              <Select
                aria-label="模型匹配模式"
                value={currentPolicy.mode}
                onChange={(event) =>
                  emit({
                    ...currentPolicy,
                    mode: event.currentTarget.value as ProviderModelPolicyV1["mode"],
                  })
                }
                disabled={saving || !canEdit}
              >
                <option value="all">全部模型</option>
                <option value="selected">仅选定模型</option>
              </Select>
            </FormField>

            <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
              <FormField label="搜索规则">
                <div className="relative">
                  <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    aria-label="搜索规则"
                    value={search}
                    onChange={(event) => setSearch(event.currentTarget.value)}
                    placeholder="搜索源模型或目标模型"
                    className="pl-8"
                    disabled={saving}
                  />
                </div>
              </FormField>
              <Button
                ref={addButtonRef}
                type="button"
                variant="secondary"
                onClick={addRule}
                disabled={saving || !canEdit || currentPolicy.rules.length >= 500}
              >
                <Plus className="h-4 w-4" aria-hidden="true" />
                添加规则
              </Button>
            </div>

            {policyError ? (
              <p role="alert" className="text-xs text-destructive">
                {policyError}
              </p>
            ) : null}

            <div className="space-y-2">
              {visibleRules.length === 0 ? (
                <p className="text-sm text-muted-foreground">暂无规则</p>
              ) : null}
              {visibleRules.map(({ rule, index }) => (
                <div
                  key={`${index}-${rule.source}`}
                  className="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_2.25rem] md:items-end"
                >
                  <FormField label="源模型">
                    <Input
                      ref={(element) => {
                        sourceRefs.current[index] = element;
                      }}
                      aria-label={`源模型 ${index + 1}`}
                      value={rule.source}
                      onChange={(event) => updateRule(index, "source", event.currentTarget.value)}
                      placeholder="例如 gpt-5.4 或 gpt-*"
                      disabled={saving || !canEdit}
                      mono
                    />
                  </FormField>
                  <FormField label="目标模型" hint="留空表示原样转发">
                    <Input
                      aria-label={`目标模型 ${index + 1}`}
                      value={rule.target ?? ""}
                      onChange={(event) => updateRule(index, "target", event.currentTarget.value)}
                      placeholder="可选，例如 upstream-*"
                      disabled={saving || !canEdit}
                      mono
                    />
                  </FormField>
                  <Button
                    type="button"
                    variant="secondary"
                    size="icon"
                    className="h-10 w-10 justify-self-end"
                    aria-label={`删除规则 ${index + 1}`}
                    title="删除规则"
                    onClick={() => deleteRule(index)}
                    disabled={saving || !canEdit}
                  >
                    <Trash2 className="h-4 w-4" aria-hidden="true" />
                  </Button>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </details>
  );
}

function discoveryOrigin(state: ProviderModelDiscoveryUiState) {
  if (state.status !== "ready" && state.status !== "empty" && state.status !== "capacity") {
    return null;
  }
  return state.origin;
}

function discoveryBaseUrlIndex(state: ProviderModelDiscoveryUiState) {
  if (state.status !== "ready" && state.status !== "empty" && state.status !== "capacity") {
    return null;
  }
  return state.baseUrlIndex;
}

function discoveryMessage(state: ProviderModelDiscoveryUiState) {
  switch (state.status) {
    case "idle":
      return "手动获取模型目录后，会把未覆盖模型追加为空目标规则。";
    case "loading":
      return "正在读取当前端点的模型目录…";
    case "changed":
      return "连接已变化，请重新获取。";
    case "ready":
      return state.addedCount > 0
        ? `已获取 ${state.discoveredCount} 个模型，新增 ${state.addedCount} 条规则。`
        : `已获取 ${state.discoveredCount} 个模型，没有新增规则。`;
    case "empty":
      return "当前端点返回了合法的空模型目录，草稿未修改。";
    case "capacity":
      return `发现 ${state.discoveredCount} 个模型，但规则将超过 500 条，草稿未修改。`;
    case "unsupported":
      return state.reason === "cx_2cc"
        ? "CX2CC 请到对应 Codex Provider 获取，或手工维护规则。"
        : "当前 OAuth 连接暂不支持获取，请手工维护规则。";
    case "error":
      return {
        invalid_config: "连接配置不完整，请检查 Base URL、认证方式和 API Key。",
        redirect: "端点发生重定向，请直接配置最终 endpoint。",
        unauthorized: "认证失败，请检查 API Key 或 OAuth 登录状态。",
        timeout: "获取超时，请重试。",
        network: "无法连接上游，请检查 endpoint、代理和网络。",
        invalid_response: "上游模型目录格式无法安全使用。",
        too_large: "上游模型目录过大，无法安全合并。",
      }[state.code];
    case "unexpected_error":
      return "获取失败，请查看应用日志后重试。";
  }
}
