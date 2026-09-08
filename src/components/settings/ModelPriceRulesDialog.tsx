import { useId, useState } from "react";
import { Pencil, Plus, Save, Trash2 } from "lucide-react";
import { cliShortItemsWith } from "../../constants/clis";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { Tooltip } from "../../ui/Tooltip";
import { QueryErrorCard } from "../shared/QueryErrorCard";
import { formatUnknownError } from "../../utils/errors";
import type { CliKey } from "../../services/providers/providers";
import {
  MODEL_PRICE_ITEMS,
  normalizeModelPriceRules,
  type ModelPriceItemKey,
  type ModelPriceRule,
  type ModelPriceRules,
} from "../../services/usage/modelPrices";
import {
  useModelPriceReferenceQuery,
  useModelPriceRulesQuery,
  useModelPriceRulesSetMutation,
  useModelPricesListQuery,
} from "../../query/modelPrices";

const LABELS: Record<ModelPriceItemKey, string> = {
  input: "普通输入", output: "输出", cache_read: "缓存读取",
  cache_write_5m: "缓存写入 5 分钟", cache_write_1h: "缓存写入 1 小时",
};
type Draft = {
  cli_key: CliKey; model: string; enabled: boolean; multiplier: string;
  items: Record<ModelPriceItemKey, { price: string; multiplier: string }>;
};

function draftFromRule(rule?: ModelPriceRule): Draft {
  return {
    cli_key: rule?.cli_key ?? "claude", model: rule?.model ?? "", enabled: rule?.enabled ?? true,
    multiplier: rule?.multiplier?.toLocaleString("en-US", { useGrouping: false, maximumFractionDigits: 6 }) ?? "",
    items: Object.fromEntries(MODEL_PRICE_ITEMS.map((key) => [key, {
      price: rule?.[key].price?.toLocaleString("en-US", { useGrouping: false, maximumFractionDigits: 9 }) ?? "",
      multiplier: rule?.[key].multiplier?.toLocaleString("en-US", { useGrouping: false, maximumFractionDigits: 6 }) ?? "",
    }])) as Draft["items"],
  };
}

function parseNumber(value: string, decimalPlaces: number): number | null {
  if (value.trim() === "") return null;
  if (!/^(?:\d+(?:\.\d*)?|\.\d+)$/.test(value.trim())) {
    throw new Error("单价和倍率必须为非负有限数字");
  }
  if ((value.trim().split(".")[1]?.replace(/0+$/, "").length ?? 0) > decimalPlaces) {
    throw new Error(`最多支持 ${decimalPlaces} 位小数`);
  }
  return Number(value);
}

function ruleFromDraft(draft: Draft): ModelPriceRule {
  return {
    cli_key: draft.cli_key, model: draft.model, enabled: draft.enabled,
    multiplier: parseNumber(draft.multiplier, 6),
    ...Object.fromEntries(MODEL_PRICE_ITEMS.map((key) => [key, {
      price: parseNumber(draft.items[key].price, 9), multiplier: parseNumber(draft.items[key].multiplier, 6),
    }])) as Pick<ModelPriceRule, ModelPriceItemKey>,
  };
}

function PriceRulesContent({ onClose }: { onClose: () => void }) {
  const id = useId();
  const query = useModelPriceRulesQuery();
  const mutation = useModelPriceRulesSetMutation();
  const [rules, setRules] = useState<ModelPriceRules | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const cliKey = draft?.cli_key ?? "claude";
  const models = useModelPricesListQuery(cliKey, { enabled: draft !== null });
  const reference = useModelPriceReferenceQuery(cliKey, draft?.model.trim() ?? "");
  const source = rules ?? query.data;
  const saving = mutation.isPending;
  const blocked = query.isError || !source || (rules === null && query.isFetching);
  const conflict = draft !== null && draft.multiplier.trim() !== "" &&
    MODEL_PRICE_ITEMS.some((key) => draft.items[key].multiplier.trim() !== "");
  let validationError: string | null = null;
  let candidate = source;
  if (draft && source) {
    try {
      const rule = ruleFromDraft(draft);
      const next = [...source.rules];
      if (editingIndex === null) next.push(rule);
      else next[editingIndex] = rule;
      candidate = normalizeModelPriceRules({ version: 1, rules: next });
    } catch (error) {
      validationError = formatUnknownError(error);
    }
  }

  function edit(index: number | null) {
    if (!source) return;
    setRules(source);
    setEditingIndex(index);
    setDraft(draftFromRule(index === null ? undefined : source.rules[index]));
    setSaveError(null);
  }

  async function save() {
    if (blocked || saving || validationError || !candidate) return;
    setSaveError(null);
    try {
      await mutation.mutateAsync(candidate);
      onClose();
    } catch (error) {
      setSaveError(formatUnknownError(error));
    }
  }

  return (
    <Dialog open onOpenChange={(next) => { if (!next && !saving) onClose(); }} title="自定义定价" className="max-w-3xl">
    <div className="space-y-4">
      {query.isError ? <QueryErrorCard errorText={formatUnknownError(query.error)} loading={query.isFetching}
        onRetry={() => void query.refetch()} message="自定义定价加载失败" /> : null}
      {!source || (rules === null && query.isFetching) ? <p role="status">加载中...</p> : (
        <>
          <div className="flex items-center justify-between gap-3">
            <span className="text-sm text-muted-foreground">{source.rules.length} 条规则</span>
            <Button variant="secondary" size="sm" disabled={blocked || saving || draft !== null} onClick={() => edit(null)}>
              <Plus size={16} />新增规则
            </Button>
          </div>
          <div className="divide-y divide-line-subtle">
            {source.rules.length === 0 && !draft ? <p className="py-4 text-sm text-muted-foreground">暂无自定义定价</p> : null}
            {source.rules.map((rule, index) => (
              <div key={JSON.stringify([rule.cli_key, rule.model])} className="flex min-w-0 items-center gap-2 py-2">
                <Switch checked={rule.enabled} disabled={blocked || saving || draft !== null}
                  aria-label={`启用 ${rule.model}`} onCheckedChange={(enabled) => {
                    setRules({ ...source, rules: source.rules.map((item, i) => i === index ? { ...item, enabled } : item) });
                  }} />
                <span className="text-xs text-muted-foreground">{rule.cli_key}</span>
                <span className="min-w-0 flex-1 break-all text-sm">{rule.model}</span>
                <Tooltip content="编辑"><Button variant="secondary" size="sm" aria-label={`编辑 ${rule.model}`}
                  disabled={blocked || saving || draft !== null} onClick={() => edit(index)}><Pencil size={16} /></Button></Tooltip>
                <Tooltip content="删除"><Button variant="danger" size="sm" aria-label={`删除 ${rule.model}`}
                  disabled={blocked || saving || draft !== null} onClick={() => setRules({ ...source, rules: source.rules.filter((_, i) => i !== index) })}>
                  <Trash2 size={16} /></Button></Tooltip>
              </div>
            ))}
          </div>
          {draft ? (
            <fieldset disabled={saving || blocked} className="min-w-0 space-y-4 border-t border-line-subtle pt-4">
              <div className="grid min-w-0 gap-3 sm:grid-cols-[8rem_minmax(0,1fr)]">
                <label className="min-w-0 text-sm" htmlFor={`${id}-cli`}>CLI
                  <Select id={`${id}-cli`} value={draft.cli_key} onChange={(event) => setDraft({ ...draft, cli_key: event.currentTarget.value as CliKey })}>
                    {cliShortItemsWith("pricing").map((item) => <option key={item.key} value={item.key}>{item.label}</option>)}
                  </Select>
                </label>
                <label className="min-w-0 text-sm" htmlFor={`${id}-model`}>完整模型名称
                  <Input id={`${id}-model`} list={`${id}-models`} value={draft.model} onChange={(event) => setDraft({ ...draft, model: event.currentTarget.value })} />
                </label>
                <datalist id={`${id}-models`}>{models.data?.filter((row) => row.cli_key === cliKey).map((row) => <option key={row.model} value={row.model} />)}</datalist>
              </div>
              {models.isError ? <QueryErrorCard errorText={formatUnknownError(models.error)} loading={models.isFetching}
                onRetry={() => void models.refetch()} message="模型建议加载失败" /> : null}
              <div className="flex flex-wrap items-center gap-4">
                <Switch checked={draft.enabled} aria-label="启用当前规则" onCheckedChange={(enabled) => setDraft({ ...draft, enabled })} />
                <label className="text-sm" htmlFor={`${id}-multiplier`}>整体倍率
                  <Input id={`${id}-multiplier`} className="max-w-40" inputMode="decimal" placeholder="1"
                    value={draft.multiplier} onChange={(event) => setDraft({ ...draft, multiplier: event.currentTarget.value })} />
                </label>
                <span className="text-xs text-muted-foreground">USD / 百万 Token</span>
              </div>
              {reference.isError ? <QueryErrorCard errorText={formatUnknownError(reference.error)} loading={reference.isFetching}
                onRetry={() => void reference.refetch()} message="参考价格加载失败" /> : null}
              {reference.data ? <p className="break-all text-xs text-muted-foreground">参考模型：{reference.data.reference_model}</p> : null}
              <div className="divide-y divide-line-subtle">
                {MODEL_PRICE_ITEMS.map((key) => {
                  const item = draft.items[key];
                  const price = reference.data?.[key];
                  const references = price ? [
                    ["标准", price.standard], ["优先", price.priority], ["超过 200k", price.above_200k],
                    ["优先超过 200k", price.priority_above_200k],
                  ].filter(([, value]) => value !== null) : [];
                  const fixed = item.price.trim() === "" ? null : Number(item.price);
                  const factor = Number(draft.multiplier.trim() || item.multiplier.trim() || "1");
                  let cacheFallback = reference.isSuccess && reference.data === null &&
                    draft.items.input.price.trim() !== "" && draft.items.output.price.trim() !== "" &&
                    key.startsWith("cache_") ? `继承${key === "cache_read" && Number(draft.items.input.price) === 0 ? "输出" : "输入"}单价 × ${key === "cache_read" ? "0.1" : key === "cache_write_5m" ? "1.25" : "2"}${factor === 1 ? "" : ` × ${factor}`}` : "未定价";
                  if (cacheFallback !== "未定价" && key !== "cache_read" && cliKey === "claude" && draft.model.toLowerCase().includes("1m")) {
                    cacheFallback += "；超过 200k 部分 × 2";
                  }
                  const effective = validationError || reference.isError || reference.isFetching ? "--" :
                    !draft.enabled ? references.map(([label, value]) => `${label} ${value}`).join(" / ") || "未定价" :
                    fixed !== null ? String(fixed * factor) : references.map(([label, value]) => `${label} ${Number(value) * factor}`).join(" / ") || cacheFallback;
                  return (
                    <div key={key} className="grid min-w-0 gap-2 py-3 sm:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,1fr)]">
                      <div className="min-w-0 text-sm">{LABELS[key]}
                        <p className="break-words text-xs text-muted-foreground">参考：{reference.isFetching ? "加载中..." : reference.isError ? "读取失败" : references.map(([label, value]) => `${label} ${value}`).join(" / ") || "未定价"}</p>
                        <p className="break-words text-xs text-muted-foreground">生效：{effective}</p>
                      </div>
                      <label className="min-w-0 text-xs" htmlFor={`${id}-${key}-price`}>自定义单价
                        <Input id={`${id}-${key}-price`} inputMode="decimal" placeholder="继承" value={item.price}
                          onChange={(event) => setDraft({ ...draft, items: { ...draft.items, [key]: { ...item, price: event.currentTarget.value } } })} />
                      </label>
                      <label className="min-w-0 text-xs" htmlFor={`${id}-${key}-factor`}>分项倍率
                        <Input id={`${id}-${key}-factor`} inputMode="decimal" placeholder="1" value={item.multiplier}
                          onChange={(event) => setDraft({ ...draft, items: { ...draft.items, [key]: { ...item, multiplier: event.currentTarget.value } } })} />
                      </label>
                    </div>
                  );
                })}
              </div>
              {conflict ? <p role="alert" className="text-sm text-rose-600">整体倍率与分项倍率不能同时设置</p> : validationError ? <p role="alert" className="text-sm text-rose-600">{validationError}</p> : null}
              {reference.isSuccess && reference.data === null && (draft.items.input.price.trim() === "" || draft.items.output.price.trim() === "") ?
                <p className="text-sm text-amber-600">未定价：缺少普通输入或输出单价</p> : null}
              <Button variant="secondary" size="sm" onClick={() => setDraft(null)}>取消编辑</Button>
            </fieldset>
          ) : null}
        </>
      )}
      {saveError ? <p role="alert" className="break-words text-sm text-rose-600">{saveError}</p> : null}
      <div className="flex justify-end gap-2 border-t border-line-subtle pt-3">
        <Button variant="secondary" disabled={saving} onClick={onClose}>取消</Button>
        <Button disabled={blocked || saving || validationError !== null} onClick={() => void save()}>
          <Save size={16} />{saving ? "保存中..." : "保存"}
        </Button>
      </div>
    </div>
    </Dialog>
  );
}

export function ModelPriceRulesDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  return open ? <PriceRulesContent onClose={() => onOpenChange(false)} /> : null;
}
