import { useState } from "react";
import { ChevronDown } from "lucide-react";
import type {
  CliKey,
  ModelRoutingPolicy,
  ProviderSummary,
  UpstreamRetryPolicy,
} from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { TabList } from "../../ui/TabList";
import type { ProviderEditorInitialValues } from "./providerDuplicate";
import { useProviderEditorForm } from "./useProviderEditorForm";
import { OAuthSection } from "./OAuthSection";
import { Cx2ccSection } from "./Cx2ccSection";
import { CodexBridgeSection } from "./CodexBridgeSection";
import { ApiKeySection } from "./ApiKeySection";
import { ProviderAccountUsageSection } from "./ProviderAccountUsageSection";
import { LimitsSection } from "./LimitsSection";
import { ClaudeModelSection } from "./ClaudeModelSection";
import { RetryPolicyFields } from "../../components/gateway/RetryPolicyFields";
import {
  CrossProviderModelRoutingPolicyFields,
  ModelRoutingPolicyFields,
} from "../../components/gateway/ModelRoutingPolicyFields";
import { cn } from "../../utils/cn";
import { ContributionSlot } from "../../plugins/contributions/ContributionSlot";

type ProviderEditorDialogBaseProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
  onModelFetchFailedAfterSave?: (provider: ProviderSummary) => void;
  codexProviders?: ProviderSummary[];
  bridgeSourceProviders?: ProviderSummary[];
};

export type ProviderEditorRouteMode = {
  modeId: number;
  modeUuid: string;
  name: string;
};

export type ProviderEditorDialogProps =
  | (ProviderEditorDialogBaseProps & {
      mode: "create";
      cliKey: CliKey;
      initialValues?: ProviderEditorInitialValues | null;
    })
  | (ProviderEditorDialogBaseProps & {
      mode: "edit";
      provider: ProviderSummary;
      routeMode?: ProviderEditorRouteMode | null;
      routeModes?: ProviderEditorRouteMode[];
      onRouteModeChange?: (modeId: number | null) => void;
    });

export function ProviderEditorDialog(props: ProviderEditorDialogProps) {
  const f = useProviderEditorForm(props);
  const saveBlocked =
    f.saving ||
    (f.routingEditorEnabled && (f.routingPolicyLoading || f.routingPolicyError != null)) ||
    f.accountUsageCustomTestInFlight ||
    (f.accountUsageAdapterKind === "custom" && Boolean(f.accountUsageCustomAllowedOriginsError));

  return (
    <Dialog
      open={f.open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && (f.saving || f.accountUsageCustomTestInFlight)) return;
        f.onOpenChange(nextOpen);
      }}
      title={f.title}
      description={f.description}
      className="max-w-4xl"
    >
      <div className="space-y-4">
        {/* ── Auth mode selector ── */}
        {f.supportsOAuth && !f.supportsCx2cc ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化" group>
            <TabList<"api_key" | "oauth">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                { key: "oauth", label: "OAuth 登录" },
              ]}
              value={f.authMode as "api_key" | "oauth"}
              onChange={(next) => {
                f.setAuthMode(next);
                f.setValue("auth_mode", next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : f.supportsCx2cc ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化" group>
            <TabList<"api_key" | "oauth" | "cx2cc">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                ...(f.supportsOAuth ? [{ key: "oauth" as const, label: "OAuth 登录" }] : []),
                { key: "cx2cc", label: f.cliKey === "codex" ? "转译" : "CX2CC 转译" },
              ]}
              value={f.authMode as "api_key" | "oauth" | "cx2cc"}
              onChange={(next) => {
                f.setAuthMode(next);
                f.setValue("auth_mode", next === "cx2cc" ? "api_key" : next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : null}

        {f.authMode === "oauth" ? (
          <OAuthSection form={f} />
        ) : f.authMode === "cx2cc" && f.cliKey === "claude" ? (
          <Cx2ccSection form={f} />
        ) : f.authMode === "cx2cc" && f.cliKey === "codex" ? (
          <CodexBridgeSection form={f} />
        ) : (
          <ApiKeySection form={f} />
        )}

        <FormField label="定时可用性测试" group>
          <div className="flex flex-wrap items-center gap-3">
            <Switch
              checked={f.availabilityProbeEnabled}
              onCheckedChange={f.setAvailabilityProbeEnabled}
              disabled={f.saving}
              aria-label="启用定时可用性测试"
            />
            <div className="flex min-w-0 items-center gap-2">
              <Input
                aria-label="定时可用性测试间隔"
                type="number"
                min="1"
                max="1440"
                step="1"
                value={f.availabilityProbeIntervalMinutes}
                onChange={(event) =>
                  f.setAvailabilityProbeIntervalMinutes(event.currentTarget.value)
                }
                disabled={f.saving || !f.availabilityProbeEnabled}
                className="w-28"
              />
              <span className="text-sm text-muted-foreground">分钟</span>
            </div>
          </div>
        </FormField>

        <FormField
          label="流式空闲超时覆盖（秒）"
          hint="留空或 0 表示沿用全局设置；仅对当前 Provider 的流式请求生效。"
        >
          {(id, hintId) => (
            <Input
              id={id}
              aria-describedby={hintId}
              type="number"
              min="0"
              max="3600"
              step="1"
              placeholder="0"
              value={f.streamIdleTimeoutSeconds}
              onChange={(e) => f.setStreamIdleTimeoutSeconds(e.currentTarget.value)}
              disabled={f.saving}
            />
          )}
        </FormField>

        <ProviderAccountUsageSection form={f} />

        <ContributionSlot
          slotId="providers.editor.sections"
          valuesByContributionKey={f.extensionValuesByContributionKey}
          onChange={(contribution, key, value) => f.setExtensionValue(contribution, key, value)}
          disabled={f.saving}
        />

        <ProviderRetryPolicySection form={f} />
        <ProviderModelRoutingPolicySection form={f} />

        <LimitsSection form={f} />
        <ClaudeModelSection form={f} />

        <div className="flex items-center justify-between border-t border-border pt-3 dark:border-border">
          <div className="flex items-center gap-2">
            <span className="text-sm text-secondary-foreground">启用</span>
            <Switch
              checked={f.enabled}
              onCheckedChange={(checked) => f.setValue("enabled", checked, { shouldDirty: true })}
              disabled={f.saving}
            />
          </div>
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button
              onClick={() => f.onOpenChange(false)}
              variant="secondary"
              disabled={f.saving || f.accountUsageCustomTestInFlight}
            >
              取消
            </Button>
            {f.canFetchProviderModels ? (
              <Button
                onClick={() => void f.saveAndFetchModels()}
                variant="secondary"
                disabled={saveBlocked}
              >
                {f.savingWithModelFetch ? "保存并获取中…" : "保存并获取模型"}
              </Button>
            ) : null}
            <Button onClick={f.save} variant="primary" disabled={saveBlocked}>
              {f.saving && !f.savingWithModelFetch ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </div>
    </Dialog>
  );
}

function ProviderModelRoutingPolicySection({
  form,
}: {
  form: ReturnType<typeof useProviderEditorForm>;
}) {
  const enabled = form.modelRoutingPolicyOverrideEnabled;
  const policy = form.modelRoutingPolicyDraft;

  function updatePolicy(next: ModelRoutingPolicy) {
    form.setModelRoutingPolicyDraft(next);
  }

  return (
    <div className="overflow-hidden rounded-lg border border-border bg-white dark:bg-secondary">
      <div className="flex w-full items-center justify-between gap-3 px-4 py-3 transition-colors hover:bg-secondary/50 dark:hover:bg-secondary/40">
        <button
          type="button"
          className="min-w-0 flex-1 text-left"
          onClick={() => form.setModelRoutingPolicyOverrideEnabled(!enabled)}
          aria-expanded={enabled}
        >
          <div className="text-sm font-semibold text-foreground">覆盖全局模型路由</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            关闭时继承全局；开启后当前供应商使用独立规则。
          </div>
        </button>
        <div className="flex items-center gap-2">
          <Switch
            checked={enabled}
            aria-label="覆盖全局模型路由"
            onCheckedChange={(checked) => form.setModelRoutingPolicyOverrideEnabled(checked)}
            disabled={form.saving}
          />
          <ChevronDown
            className={cn(
              "h-4 w-4 text-muted-foreground transition-transform",
              enabled && "rotate-180"
            )}
          />
        </div>
      </div>
      {enabled ? (
        <div className="space-y-4 border-t border-border px-4 py-4">
          <div className="space-y-5">
            <div>
              <div className="text-sm font-medium text-foreground">普通规则</div>
              <div className="mt-0.5 text-xs text-muted-foreground">
                仅改写当前供应商的模型或思考强度，不绑定调用方案。
              </div>
              {form.mode === "edit" ? (
                <div className="mt-2 text-xs text-muted-foreground">
                  目标供应商：本供应商（{form.editProviderName}）
                </div>
              ) : null}
            </div>
            <ModelRoutingPolicyFields
              policy={policy}
              disabled={form.saving || form.routingPolicyLoading}
              onChange={updatePolicy}
            />
            {form.routingEditorEnabled ? (
              <ProviderCrossRoutingPolicySection form={form} />
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ProviderCrossRoutingPolicySection({
  form,
}: {
  form: ReturnType<typeof useProviderEditorForm>;
}) {
  const [pendingModeId, setPendingModeId] = useState<number | null | undefined>(undefined);
  const routeMode = form.routeMode;
  const disabled =
    form.saving ||
    form.routingPolicyLoading ||
    form.routingCandidatesLoading ||
    form.routingCandidatesError != null ||
    routeMode == null ||
    !form.routingPolicyView?.source_member_present ||
    !form.routingPolicyView?.source_member_enabled;
  const disabledMessage =
    routeMode == null
      ? "Default 不支持跨供应商目标，请选择命名调用方案。"
      : form.routingPolicyLoading
        ? "正在加载当前方案的跨供应商规则…"
        : form.routingPolicyError
          ? "无法读取当前方案的跨供应商规则，请稍后重试。"
          : !form.routingPolicyView?.source_member_present
            ? "当前供应商不在该方案中；普通规则仍可编辑，请先加入方案。"
            : !form.routingPolicyView?.source_member_enabled
              ? "当前供应商在该方案中已禁用；普通规则仍可编辑，请先启用成员。"
              : null;

  const requestRouteModeChange = (modeId: number | null) => {
    if (modeId === routeMode?.modeId || (modeId == null && routeMode == null)) return;
    if (!form.crossRoutingDirty) {
      form.onRouteModeChange(modeId);
      return;
    }
    setPendingModeId(modeId);
  };

  const saveThenSwitch = async () => {
    if (pendingModeId === undefined) return;
    const saved = await form.saveRoutingPolicies();
    if (!saved) return;
    form.onRouteModeChange(pendingModeId);
    setPendingModeId(undefined);
  };

  const discardThenSwitch = () => {
    if (pendingModeId === undefined) return;
    form.discardCrossRoutingDraft();
    form.onRouteModeChange(pendingModeId);
    setPendingModeId(undefined);
  };

  return (
    <>
      <div className="border-t border-border pt-5">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div>
            <div className="text-sm font-medium text-foreground">方案跨供应商规则</div>
            <div className="mt-0.5 text-xs text-muted-foreground">
              仅绑定命名调用方案；目标候选限于该方案中同 CLI 的启用成员。
            </div>
          </div>
          <label className="block space-y-1.5 text-xs font-medium text-muted-foreground">
            当前方案
            <Select
              value={routeMode ? String(routeMode.modeId) : ""}
              aria-label="跨供应商模型路由方案"
              disabled={form.saving}
              onChange={(event) =>
                requestRouteModeChange(
                  event.currentTarget.value ? Number(event.currentTarget.value) : null
                )
              }
            >
              <option value="">Default</option>
              {form.routeModes.map((mode) => (
                <option key={mode.modeUuid} value={mode.modeId}>
                  {mode.name}
                </option>
              ))}
            </Select>
          </label>
        </div>

        {disabledMessage ? (
          <div className="border border-dashed border-border px-3 py-4 text-sm text-muted-foreground">
            {disabledMessage}
          </div>
        ) : form.crossRoutingPolicy ? (
          <div className="space-y-3">
            {form.routingCandidatesLoading ? (
              <div className="border border-dashed border-border px-3 py-3 text-sm text-muted-foreground">
                正在加载目标供应商候选；已保存规则暂时只读。
              </div>
            ) : null}
            {form.routingCandidatesError ? (
              <div className="border border-dashed border-border px-3 py-3 text-sm text-amber-700 dark:text-amber-300">
                无法读取目标供应商候选；已保存规则仍可见，但当前目标暂不可验证。
              </div>
            ) : null}
            <CrossProviderModelRoutingPolicyFields
              policy={form.crossRoutingPolicy}
              candidates={form.routingCandidates}
              candidateValidationAvailable={
                !form.routingCandidatesLoading && form.routingCandidatesError == null
              }
              sourceProviderUuid={form.editProviderUuid}
              disabled={disabled}
              onChange={form.setCrossRoutingPolicy}
            />
          </div>
        ) : null}
      </div>

      <Dialog
        open={pendingModeId !== undefined}
        onOpenChange={(open) => {
          if (!open) setPendingModeId(undefined);
        }}
        title="保存跨供应商规则草稿？"
        description="切换调用方案前，请保存当前草稿、放弃当前草稿，或取消本次切换。普通规则不会被放弃。"
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            onClick={() => setPendingModeId(undefined)}
            variant="secondary"
            disabled={form.saving}
          >
            取消
          </Button>
          <Button onClick={discardThenSwitch} variant="secondary" disabled={form.saving}>
            放弃草稿
          </Button>
          <Button onClick={() => void saveThenSwitch()} variant="primary" disabled={form.saving}>
            保存并切换
          </Button>
        </div>
      </Dialog>
    </>
  );
}

function ProviderRetryPolicySection({ form }: { form: ReturnType<typeof useProviderEditorForm> }) {
  const enabled = form.upstreamRetryPolicyOverrideEnabled;
  const policy = form.upstreamRetryPolicyDraft;

  function updatePolicy(next: UpstreamRetryPolicy) {
    form.setUpstreamRetryPolicyDraft(next);
  }

  return (
    <div className="overflow-hidden rounded-lg border border-border bg-white dark:bg-secondary">
      <div className="flex w-full items-center justify-between gap-3 px-4 py-3 transition-colors hover:bg-secondary/50 dark:hover:bg-secondary/40">
        <button
          type="button"
          className="min-w-0 flex-1 text-left"
          onClick={() => form.setUpstreamRetryPolicyOverrideEnabled(!enabled)}
          aria-expanded={enabled}
        >
          <div className="text-sm font-semibold text-foreground">覆盖全局重试策略</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            关闭时继承全局；开启后当前供应商使用自己的瞬时错误重试规则。
          </div>
        </button>
        <div className="flex items-center gap-2">
          <Switch
            checked={enabled}
            aria-label="覆盖全局重试策略"
            onCheckedChange={(checked) => form.setUpstreamRetryPolicyOverrideEnabled(checked)}
            disabled={form.saving}
          />
          <ChevronDown
            className={cn(
              "h-4 w-4 text-muted-foreground transition-transform",
              enabled && "rotate-180"
            )}
          />
        </div>
      </div>
      {enabled ? (
        <div className="space-y-4 border-t border-border px-4 py-4">
          <RetryPolicyFields policy={policy} disabled={form.saving} onChange={updatePolicy} />
        </div>
      ) : null}
    </div>
  );
}
