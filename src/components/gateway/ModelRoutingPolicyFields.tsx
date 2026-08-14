import { Plus, Trash2 } from "lucide-react";
import { useId } from "react";
import type {
  CrossProviderModelRoutingPolicy,
  RoutingProviderCandidate,
} from "../../services/providers/sortModes";
import type { ModelRoutingPolicy, ModelRoutingRule } from "../../services/settings/settings";
import {
  emptyModelRoutingRule,
  MAX_MODEL_ROUTING_RULES,
  MODEL_ROUTING_REASONING_EFFORTS,
} from "../../services/gateway/modelRoutingPolicy";
import { useProviderModelCatalogQuery } from "../../query/providerModels";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { Tooltip } from "../../ui/Tooltip";

function optionalInput(value: string): string | null {
  return value.length > 0 ? value : null;
}

function EffortSelect({
  value,
  disabled,
  ariaLabel,
  onChange,
}: {
  value: string | null;
  disabled: boolean;
  ariaLabel: string;
  onChange: (value: string | null) => void;
}) {
  return (
    <Select
      value={value ?? ""}
      disabled={disabled}
      aria-label={ariaLabel}
      mono
      onChange={(event) => onChange(optionalInput(event.currentTarget.value))}
    >
      <option value="">留空</option>
      {MODEL_ROUTING_REASONING_EFFORTS.map((effort) => (
        <option key={effort} value={effort}>
          {effort}
        </option>
      ))}
    </Select>
  );
}

function ModelRoutingRuleEditor({
  rule,
  index,
  disabled,
  onChange,
  onDelete,
}: {
  rule: ModelRoutingRule;
  index: number;
  disabled: boolean;
  onChange: (rule: ModelRoutingRule) => void;
  onDelete: () => void;
}) {
  const id = useId();
  return (
    <div
      role="group"
      aria-label={`模型路由规则 ${index + 1}`}
      className="grid gap-3 py-3 first:pt-2 md:grid-cols-[minmax(9rem,1fr)_minmax(8rem,0.72fr)_minmax(9rem,1fr)_minmax(8rem,0.72fr)_2rem]"
    >
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        来源模型
        <Input
          id={`${id}-source`}
          value={rule.source_model}
          disabled={disabled}
          placeholder="fable5"
          onChange={(event) => onChange({ ...rule, source_model: event.currentTarget.value })}
        />
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        来源强度
        <EffortSelect
          value={rule.source_reasoning_effort}
          disabled={disabled}
          ariaLabel={`模型路由规则 ${index + 1} 来源思考强度`}
          onChange={(source_reasoning_effort) => onChange({ ...rule, source_reasoning_effort })}
        />
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        目标模型
        <Input
          id={`${id}-target`}
          value={rule.target_model ?? ""}
          disabled={disabled}
          placeholder="留空不改模型"
          onChange={(event) =>
            onChange({ ...rule, target_model: optionalInput(event.currentTarget.value) })
          }
        />
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        目标强度
        <EffortSelect
          value={rule.reasoning_effort}
          disabled={disabled}
          ariaLabel={`模型路由规则 ${index + 1} 目标思考强度`}
          onChange={(reasoning_effort) => onChange({ ...rule, reasoning_effort })}
        />
      </label>
      <div className="flex h-8 items-center justify-end md:mt-5">
        <Tooltip content={`删除模型路由规则 ${index + 1}`}>
          <Button
            variant="ghost"
            size="icon"
            aria-label={`删除模型路由规则 ${index + 1}`}
            disabled={disabled}
            onClick={onDelete}
          >
            <Trash2 className="h-4 w-4" aria-hidden="true" />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}

function emptyCrossProviderModelRoutingRule(targetProviderUuid: string) {
  return {
    source_model: "",
    source_reasoning_effort: null,
    target_provider_uuid: targetProviderUuid,
    target_model: null,
    target_reasoning_effort: null,
  };
}

function CrossProviderModelRoutingRuleEditor({
  rule,
  index,
  sourceProviderUuid,
  candidates,
  candidateValidationAvailable,
  disabled,
  onChange,
  onDelete,
}: {
  rule: CrossProviderModelRoutingPolicy["rules"][number];
  index: number;
  sourceProviderUuid: string;
  candidates: RoutingProviderCandidate[];
  candidateValidationAvailable: boolean;
  disabled: boolean;
  onChange: (rule: CrossProviderModelRoutingPolicy["rules"][number]) => void;
  onDelete: () => void;
}) {
  const id = useId();
  const target = candidates.find(
    (candidate) =>
      candidate.provider_uuid === rule.target_provider_uuid &&
      candidate.provider_uuid !== sourceProviderUuid
  );
  const catalogQuery = useProviderModelCatalogQuery(
    target?.provider_id ?? null,
    target?.provider_uuid ?? null,
    { enabled: target?.model_catalog_supported === true }
  );
  const modelSuggestions = catalogQuery.data?.models ?? [];
  const targetMissing = target == null;
  const selectableCandidates = candidates.filter(
    (candidate) => candidate.provider_uuid !== sourceProviderUuid
  );

  return (
    <div
      role="group"
      aria-label={`跨供应商模型路由规则 ${index + 1}`}
      className="grid gap-3 py-3 first:pt-2 md:grid-cols-[minmax(8rem,1fr)_minmax(7rem,0.7fr)_minmax(10rem,1fr)_minmax(9rem,1fr)_minmax(7rem,0.7fr)_2rem]"
    >
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        来源模型
        <Input
          id={`${id}-source`}
          value={rule.source_model}
          disabled={disabled}
          placeholder="fable5"
          onChange={(event) => onChange({ ...rule, source_model: event.currentTarget.value })}
        />
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        来源强度
        <EffortSelect
          value={rule.source_reasoning_effort}
          disabled={disabled}
          ariaLabel={`跨供应商模型路由规则 ${index + 1} 来源思考强度`}
          onChange={(source_reasoning_effort) => onChange({ ...rule, source_reasoning_effort })}
        />
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        目标供应商
        <Select
          value={rule.target_provider_uuid}
          disabled={disabled}
          aria-label={`跨供应商模型路由规则 ${index + 1} 目标供应商`}
          onChange={(event) =>
            onChange({ ...rule, target_provider_uuid: event.currentTarget.value })
          }
        >
          {targetMissing ? (
            <option value={rule.target_provider_uuid}>失效目标（{rule.target_provider_uuid}）</option>
          ) : null}
          {selectableCandidates.map((candidate) => (
            <option key={candidate.provider_uuid} value={candidate.provider_uuid}>
              {candidate.name}
            </option>
          ))}
        </Select>
        {targetMissing ? (
          <span className="block text-amber-700 dark:text-amber-300">
            {candidateValidationAvailable
              ? "该目标已失效，保存其他规则不会改写它。"
              : "候选列表暂不可用，当前目标无法验证且不会被改写。"}
          </span>
        ) : null}
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        目标模型
        <Input
          id={`${id}-target-model`}
          list={target?.model_catalog_supported ? `${id}-target-model-options` : undefined}
          value={rule.target_model ?? ""}
          disabled={disabled}
          placeholder="留空只切供应商"
          onChange={(event) =>
            onChange({ ...rule, target_model: optionalInput(event.currentTarget.value) })
          }
        />
        {target?.model_catalog_supported ? (
          <datalist id={`${id}-target-model-options`}>
            {modelSuggestions.map((model) => (
              <option key={model.modelUuid} value={model.remoteModelId} />
            ))}
          </datalist>
        ) : null}
        {target?.model_catalog_supported && catalogQuery.isFetching ? (
          <span className="block">正在加载目录建议…</span>
        ) : null}
        {target?.model_catalog_supported && catalogQuery.isError ? (
          <span className="block text-amber-700 dark:text-amber-300">
            目录建议暂不可用，仍可直接输入模型 ID。
          </span>
        ) : null}
      </label>
      <label className="space-y-1.5 text-xs font-medium text-muted-foreground">
        目标强度
        <EffortSelect
          value={rule.target_reasoning_effort}
          disabled={disabled}
          ariaLabel={`跨供应商模型路由规则 ${index + 1} 目标思考强度`}
          onChange={(target_reasoning_effort) => onChange({ ...rule, target_reasoning_effort })}
        />
      </label>
      <div className="flex h-8 items-center justify-end md:mt-5">
        <Tooltip content={`删除跨供应商模型路由规则 ${index + 1}`}>
          <Button
            variant="ghost"
            size="icon"
            aria-label={`删除跨供应商模型路由规则 ${index + 1}`}
            disabled={disabled}
            onClick={onDelete}
          >
            <Trash2 className="h-4 w-4" aria-hidden="true" />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}

export function ModelRoutingPolicyFields({
  policy,
  disabled,
  onChange,
}: {
  policy: ModelRoutingPolicy;
  disabled: boolean;
  onChange: (policy: ModelRoutingPolicy) => void;
}) {
  function updateRule(index: number, rule: ModelRoutingRule) {
    const rules = [...policy.rules];
    rules[index] = rule;
    onChange({ ...policy, rules });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="text-sm font-medium text-foreground">启用模型路由</div>
        <Switch
          checked={policy.enabled}
          aria-label="启用模型路由"
          disabled={disabled}
          onCheckedChange={(enabled) => onChange({ ...policy, enabled })}
        />
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between gap-3">
          <div className="text-xs font-medium text-muted-foreground">精确匹配规则</div>
          <Button
            variant="secondary"
            size="sm"
            disabled={disabled || policy.rules.length >= MAX_MODEL_ROUTING_RULES}
            onClick={() =>
              onChange({ ...policy, rules: [...policy.rules, emptyModelRoutingRule()] })
            }
          >
            <Plus className="h-3.5 w-3.5" aria-hidden="true" />
            新增规则
          </Button>
        </div>
        <div className="divide-y divide-border border-y border-border">
          {policy.rules.map((rule, index) => (
            <ModelRoutingRuleEditor
              key={index}
              rule={rule}
              index={index}
              disabled={disabled}
              onChange={(next) => updateRule(index, next)}
              onDelete={() =>
                onChange({
                  ...policy,
                  rules: policy.rules.filter((_, ruleIndex) => ruleIndex !== index),
                })
              }
            />
          ))}
          {policy.rules.length === 0 ? (
            <div className="py-4 text-xs text-muted-foreground">暂无模型路由规则</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function CrossProviderModelRoutingPolicyFields({
  policy,
  candidates,
  sourceProviderUuid,
  candidateValidationAvailable = true,
  disabled,
  onChange,
}: {
  policy: CrossProviderModelRoutingPolicy;
  candidates: RoutingProviderCandidate[];
  sourceProviderUuid: string;
  candidateValidationAvailable?: boolean;
  disabled: boolean;
  onChange: (policy: CrossProviderModelRoutingPolicy) => void;
}) {
  const crossTargets = candidates.filter(
    (candidate) => candidate.provider_uuid !== sourceProviderUuid
  );

  function updateRule(index: number, rule: CrossProviderModelRoutingPolicy["rules"][number]) {
    const rules = [...policy.rules];
    rules[index] = rule;
    onChange({ ...policy, rules });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-sm font-medium text-foreground">启用跨供应商模型路由</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            规则只在当前命名调用方案中生效；目标供应商会临时替代当前供应商一次。
          </div>
        </div>
        <Switch
          checked={policy.enabled}
          aria-label="启用跨供应商模型路由"
          disabled={disabled}
          onCheckedChange={(enabled) => onChange({ ...policy, enabled })}
        />
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between gap-3">
          <div className="text-xs font-medium text-muted-foreground">跨供应商规则</div>
          <Button
            variant="secondary"
            size="sm"
            disabled={
              disabled ||
              crossTargets.length === 0 ||
              policy.rules.length >= MAX_MODEL_ROUTING_RULES
            }
            onClick={() =>
              onChange({
                ...policy,
                rules: [
                  ...policy.rules,
                  emptyCrossProviderModelRoutingRule(crossTargets[0].provider_uuid),
                ],
              })
            }
          >
            <Plus className="h-3.5 w-3.5" aria-hidden="true" />
            新增跨规则
          </Button>
        </div>
        {crossTargets.length === 0 ? (
          <div className="border-y border-border py-4 text-xs text-muted-foreground">
            当前方案没有其他可用供应商，普通规则仍可将目标保留在本供应商。
          </div>
        ) : null}
        {policy.rules.length > 0 || crossTargets.length > 0 ? (
          <div className="divide-y divide-border border-y border-border">
            {policy.rules.map((rule, index) => (
              <CrossProviderModelRoutingRuleEditor
                key={`${rule.target_provider_uuid}:${index}`}
                rule={rule}
                index={index}
                sourceProviderUuid={sourceProviderUuid}
                candidates={candidates}
                candidateValidationAvailable={candidateValidationAvailable}
                disabled={disabled}
                onChange={(next) => updateRule(index, next)}
                onDelete={() =>
                  onChange({
                    ...policy,
                    rules: policy.rules.filter((_, ruleIndex) => ruleIndex !== index),
                  })
                }
              />
            ))}
            {policy.rules.length === 0 ? (
              <div className="py-4 text-xs text-muted-foreground">暂无跨供应商模型路由规则</div>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}
