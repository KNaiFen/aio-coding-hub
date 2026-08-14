import {
  commands,
  type CrossProviderModelRoutingPolicy as GeneratedCrossProviderModelRoutingPolicy,
  type CrossProviderModelRoutingRule as GeneratedCrossProviderModelRoutingRule,
  type ModelRoutingPolicy,
  type ProviderModelRoutingPolicySaveInput as GeneratedProviderModelRoutingPolicySaveInput,
  type ProviderModelRoutingPolicyView as GeneratedProviderModelRoutingPolicyView,
  type RoutingProviderCandidate as GeneratedRoutingProviderCandidate,
} from "../../generated/bindings";
import { FeValidationError } from "../../utils/errors";
import {
  normalizeModelRoutingPolicy,
  validateModelRoutingPolicy,
  MODEL_ROUTING_REASONING_EFFORTS,
} from "../gateway/modelRoutingPolicy";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import {
  validateProviderCliKey,
  validateProviderId,
  validateSessionReusePriority,
  type CliKey,
} from "./providers";
import { isCanonicalUuidV4 } from "./uuid";

export const MAX_SORT_MODE_NAME_CHARS = 32;
export const MAX_SORT_MODE_PROVIDER_IDS = 512;

export type CrossProviderModelRoutingRule = GeneratedCrossProviderModelRoutingRule;
export type CrossProviderModelRoutingPolicy = GeneratedCrossProviderModelRoutingPolicy;
export type ProviderModelRoutingPolicyView = GeneratedProviderModelRoutingPolicyView;
export type RoutingProviderCandidate = GeneratedRoutingProviderCandidate;

export type ProviderModelRoutingPolicySaveInput = {
  provider_id: number;
  provider_uuid: string;
  mode_id: number | null;
  mode_uuid: string | null;
  provider_override_enabled: boolean;
  ordinary_policy: ModelRoutingPolicy;
  expected_ordinary_policy_revision: string;
  cross_policy: CrossProviderModelRoutingPolicy | null;
  expected_cross_policy_revision: string | null;
};

export type SortModeSummary = {
  id: number;
  mode_uuid: string;
  name: string;
  created_at: number;
  updated_at: number;
};

export type SortModeActiveRow = {
  cli_key: CliKey;
  mode_id: number | null;
  updated_at: number;
};

export type SortModeProviderRow = {
  provider_id: number;
  provider_uuid: string;
  enabled: boolean;
  session_reuse_priority: number;
  cross_policy: CrossProviderModelRoutingPolicy | null;
};

const ROUTING_POLICY_REVISION_RE = /^[0-9a-f]{64}$/;
const MODEL_ROUTING_EFFORT_SET = new Set<string>(MODEL_ROUTING_REASONING_EFFORTS);
const MAX_MODEL_ROUTING_RULES = 128;
const MAX_MODEL_ROUTING_MODEL_BYTES = 256;

function containsControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

function normalizedOptional(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized.length > 0 ? normalized : null;
}

function normalizedEffort(value: string | null | undefined): string | null {
  return normalizedOptional(value)?.toLowerCase() ?? null;
}

function validateCanonicalUuid(value: string, label: string): string {
  if (!isCanonicalUuidV4(value)) {
    throw new FeValidationError(`SEC_INVALID_INPUT: invalid ${label}`);
  }
  return value;
}

export function validateSortModeUuid(modeUuid: string): string {
  return validateCanonicalUuid(modeUuid, "modeUuid");
}

export function validateRoutingPolicyRevision(revision: string, label = "revision"): string {
  if (!ROUTING_POLICY_REVISION_RE.test(revision)) {
    throw new FeValidationError(`SEC_INVALID_INPUT: invalid ${label}`);
  }
  return revision;
}

function validateModeIdentity(modeId: number | null, modeUuid: string | null) {
  if ((modeId == null) !== (modeUuid == null)) {
    throw new FeValidationError("SEC_INVALID_INPUT: modeId and modeUuid must be provided together");
  }
  return {
    modeId: modeId == null ? null : validateSortModeId(modeId),
    modeUuid: modeUuid == null ? null : validateSortModeUuid(modeUuid),
  };
}

export function normalizeCrossProviderModelRoutingPolicy(
  policy: CrossProviderModelRoutingPolicy
): CrossProviderModelRoutingPolicy {
  return {
    enabled: policy.enabled,
    rules: policy.rules.map((rule) => ({
      source_model: rule.source_model.trim(),
      source_reasoning_effort: normalizedEffort(rule.source_reasoning_effort),
      target_provider_uuid: rule.target_provider_uuid.trim().toLowerCase(),
      target_model: normalizedOptional(rule.target_model),
      target_reasoning_effort: normalizedEffort(rule.target_reasoning_effort),
    })),
  };
}

export function validateCrossProviderModelRoutingPolicy(
  policy: CrossProviderModelRoutingPolicy
): string | null {
  if (typeof policy.enabled !== "boolean" || !Array.isArray(policy.rules)) {
    return "跨供应商模型路由策略格式无效";
  }
  if (policy.rules.length > MAX_MODEL_ROUTING_RULES) {
    return `跨供应商模型路由规则最多 ${MAX_MODEL_ROUTING_RULES} 条`;
  }

  const seen = new Set<string>();
  for (const [index, rawRule] of policy.rules.entries()) {
    const rule = {
      source_model: rawRule.source_model?.trim() ?? "",
      source_reasoning_effort: normalizedEffort(rawRule.source_reasoning_effort),
      target_provider_uuid: rawRule.target_provider_uuid?.trim().toLowerCase() ?? "",
      target_model: normalizedOptional(rawRule.target_model),
      target_reasoning_effort: normalizedEffort(rawRule.target_reasoning_effort),
    };
    const label = `第 ${index + 1} 条跨供应商模型路由`;
    if (!rule.source_model) return `${label}必须填写来源模型`;
    if (new TextEncoder().encode(rule.source_model).length > MAX_MODEL_ROUTING_MODEL_BYTES) {
      return `${label}的来源模型不能超过 ${MAX_MODEL_ROUTING_MODEL_BYTES} 字节`;
    }
    if (containsControlCharacter(rule.source_model)) return `${label}的来源模型包含控制字符`;
    if (
      rule.source_reasoning_effort != null &&
      !MODEL_ROUTING_EFFORT_SET.has(rule.source_reasoning_effort)
    ) {
      return `${label}的来源思考强度不是受支持的标准值`;
    }
    const sourceKey = `${rule.source_model}\u0000${rule.source_reasoning_effort ?? ""}`;
    if (seen.has(sourceKey)) return `${label}与已有来源模型及思考强度重复`;
    seen.add(sourceKey);
    if (!isCanonicalUuidV4(rule.target_provider_uuid)) return `${label}的目标供应商无效`;
    if (rule.target_model != null) {
      if (new TextEncoder().encode(rule.target_model).length > MAX_MODEL_ROUTING_MODEL_BYTES) {
        return `${label}的目标模型不能超过 ${MAX_MODEL_ROUTING_MODEL_BYTES} 字节`;
      }
      if (containsControlCharacter(rule.target_model)) return `${label}的目标模型包含控制字符`;
    }
    if (
      rule.target_reasoning_effort != null &&
      !MODEL_ROUTING_EFFORT_SET.has(rule.target_reasoning_effort)
    ) {
      return `${label}的目标思考强度不是受支持的标准值`;
    }
  }
  return null;
}

function requireRoutingPolicyView(
  value: GeneratedProviderModelRoutingPolicyView,
  expected: {
    providerId: number;
    providerUuid: string;
    modeId: number | null;
    modeUuid: string | null;
  }
): ProviderModelRoutingPolicyView {
  if (
    value.provider_id !== expected.providerId ||
    value.provider_uuid !== expected.providerUuid ||
    validateProviderCliKey(value.cli_key) !== value.cli_key ||
    typeof value.provider_override_enabled !== "boolean" ||
    typeof value.source_member_enabled !== "boolean" ||
    typeof value.source_member_present !== "boolean"
  ) {
    throw new Error("IPC_INVALID_SCOPE: provider routing policy view");
  }
  validateCanonicalUuid(value.provider_uuid, "view.provider_uuid");
  validateRoutingPolicyRevision(value.ordinary_policy_revision, "ordinary_policy_revision");
  const ordinaryError = validateModelRoutingPolicy(value.ordinary_policy);
  if (ordinaryError != null) throw new Error(`IPC_INVALID_POLICY: ${ordinaryError}`);

  if (expected.modeId == null) {
    if (
      value.selected_mode != null ||
      value.cross_policy != null ||
      value.cross_policy_revision != null ||
      value.source_member_enabled ||
      value.source_member_present
    ) {
      throw new Error("IPC_INVALID_SCOPE: Default routing policy view");
    }
  } else {
    if (
      value.selected_mode?.mode_id !== expected.modeId ||
      value.selected_mode.mode_uuid !== expected.modeUuid ||
      validateSortModeUuid(value.selected_mode.mode_uuid) !== value.selected_mode.mode_uuid
    ) {
      throw new Error("IPC_INVALID_SCOPE: selected sort mode identity");
    }
    if (value.source_member_present !== (value.cross_policy_revision != null)) {
      throw new Error("IPC_INVALID_SCOPE: member routing revision");
    }
    if (value.cross_policy_revision != null) {
      validateRoutingPolicyRevision(value.cross_policy_revision, "cross_policy_revision");
    }
    if (value.cross_policy != null) {
      const crossError = validateCrossProviderModelRoutingPolicy(value.cross_policy);
      if (crossError != null) throw new Error(`IPC_INVALID_POLICY: ${crossError}`);
    }
  }
  return value;
}

function requireRoutingProviderCandidate(
  value: GeneratedRoutingProviderCandidate,
  expectedCliKey: CliKey
): RoutingProviderCandidate {
  const providerId = validateProviderId(value.provider_id, "candidate.provider_id");
  const providerUuid = validateCanonicalUuid(value.provider_uuid, "candidate.provider_uuid");
  const cliKey = validateProviderCliKey(value.cli_key);
  if (cliKey !== expectedCliKey || typeof value.name !== "string" || !value.name.trim()) {
    throw new Error("IPC_INVALID_SCOPE: routing provider candidate");
  }
  if (typeof value.enabled !== "boolean" || !value.enabled) {
    throw new Error("IPC_INVALID_SCOPE: disabled routing provider candidate");
  }
  const sourceProviderId =
    value.source_provider_id == null
      ? null
      : validateProviderId(value.source_provider_id, "candidate.source_provider_id");
  if (value.bridge_type != null && typeof value.bridge_type !== "string") {
    throw new Error("IPC_INVALID_STRING: candidate.bridge_type");
  }
  if (typeof value.model_catalog_supported !== "boolean") {
    throw new Error("IPC_INVALID_BOOLEAN: candidate.model_catalog_supported");
  }
  return {
    provider_id: providerId,
    provider_uuid: providerUuid,
    cli_key: cliKey,
    name: value.name,
    enabled: value.enabled,
    source_provider_id: sourceProviderId,
    bridge_type: value.bridge_type,
    model_catalog_supported: value.model_catalog_supported,
  };
}

function normalizeSortModeName(name: string) {
  const trimmed = name.trim();
  if (!trimmed) {
    throw new Error("SEC_INVALID_INPUT: mode name is required");
  }
  if ([...trimmed].length > MAX_SORT_MODE_NAME_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: mode name is too long (max ${MAX_SORT_MODE_NAME_CHARS} chars)`
    );
  }
  if (trimmed.toLowerCase() === "default" || trimmed === "默认") {
    throw new Error("SEC_INVALID_INPUT: mode name is reserved");
  }
  return trimmed;
}

function validatePositiveId(field: string, value: number) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new FeValidationError(`SEC_INVALID_INPUT: invalid ${field}=${value}`);
  }
}

export function validateSortModeId(modeId: number): number {
  validatePositiveId("modeId", modeId);
  return modeId;
}

function validateOrderedProviderIds(orderedProviderIds: number[]) {
  if (orderedProviderIds.length > MAX_SORT_MODE_PROVIDER_IDS) {
    throw new Error(
      `SEC_INVALID_INPUT: orderedProviderIds must contain at most ${MAX_SORT_MODE_PROVIDER_IDS} entries`
    );
  }

  const seen = new Set<number>();
  for (const providerId of orderedProviderIds) {
    validatePositiveId("providerId", providerId);
    if (seen.has(providerId)) {
      throw new Error(`SEC_INVALID_INPUT: duplicate providerId=${providerId}`);
    }
    seen.add(providerId);
  }
}

function requireSortModeSummary(value: SortModeSummary): SortModeSummary {
  validateSortModeId(value.id);
  validateSortModeUuid(value.mode_uuid);
  if (
    typeof value.name !== "string" ||
    !value.name.trim() ||
    !Number.isSafeInteger(value.created_at) ||
    !Number.isSafeInteger(value.updated_at)
  ) {
    throw new Error("IPC_INVALID_SCOPE: sort mode summary");
  }
  return value;
}

function requireSortModeProviderRow(value: SortModeProviderRow): SortModeProviderRow {
  validateProviderId(value.provider_id, "sortModeProvider.provider_id");
  validateCanonicalUuid(value.provider_uuid, "sortModeProvider.provider_uuid");
  if (typeof value.enabled !== "boolean") {
    throw new Error("IPC_INVALID_BOOLEAN: sortModeProvider.enabled");
  }
  validateSessionReusePriority(value.session_reuse_priority);
  if (value.cross_policy != null) {
    const error = validateCrossProviderModelRoutingPolicy(value.cross_policy);
    if (error != null) throw new Error(`IPC_INVALID_POLICY: ${error}`);
  }
  return value;
}

export async function sortModesList() {
  return invokeGeneratedIpc<SortModeSummary[]>({
    title: "读取排序模板失败",
    cmd: "sort_modes_list",
    invoke: () =>
      commands.sortModesList().then((response) =>
        mapGeneratedCommandResponse(response, (rows) => rows.map(requireSortModeSummary))
      ) as Promise<GeneratedCommandResult<SortModeSummary[]>>,
  });
}

export async function sortModeCreate(input: { name: string }) {
  const name = normalizeSortModeName(input.name);

  return invokeGeneratedIpc<SortModeSummary>({
    title: "创建排序模板失败",
    cmd: "sort_mode_create",
    args: { name },
    invoke: () =>
      commands.sortModeCreate(name).then((response) =>
        mapGeneratedCommandResponse(response, requireSortModeSummary)
      ) as Promise<GeneratedCommandResult<SortModeSummary>>,
  });
}

export async function sortModeRename(input: { mode_id: number; name: string }) {
  const modeId = validateSortModeId(input.mode_id);
  const name = normalizeSortModeName(input.name);

  return invokeGeneratedIpc<SortModeSummary>({
    title: "重命名排序模板失败",
    cmd: "sort_mode_rename",
    args: { modeId, name },
    invoke: () =>
      commands.sortModeRename(modeId, name).then((response) =>
        mapGeneratedCommandResponse(response, requireSortModeSummary)
      ) as Promise<GeneratedCommandResult<SortModeSummary>>,
  });
}

export async function sortModeDelete(input: { mode_id: number }) {
  const modeId = validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<boolean>({
    title: "删除排序模板失败",
    cmd: "sort_mode_delete",
    args: { modeId },
    invoke: () => commands.sortModeDelete(modeId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function sortModeActiveList() {
  return invokeGeneratedIpc<SortModeActiveRow[]>({
    title: "读取激活排序模板失败",
    cmd: "sort_mode_active_list",
    invoke: () =>
      commands.sortModeActiveList() as Promise<GeneratedCommandResult<SortModeActiveRow[]>>,
  });
}

export async function sortModeActiveSet(input: { cli_key: CliKey; mode_id: number | null }) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = input.mode_id == null ? null : validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<SortModeActiveRow>({
    title: "设置激活排序模板失败",
    cmd: "sort_mode_active_set",
    args: { cliKey, modeId },
    invoke: () =>
      commands.sortModeActiveSet(cliKey, modeId) as Promise<
        GeneratedCommandResult<SortModeActiveRow>
      >,
  });
}

export async function sortModeProvidersList(input: { mode_id: number; cli_key: CliKey }) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "读取排序模板供应商失败",
    cmd: "sort_mode_providers_list",
    args: { modeId, cliKey },
    invoke: () =>
      commands.sortModeProvidersList(modeId, cliKey).then((response) =>
        mapGeneratedCommandResponse(response, (rows) => rows.map(requireSortModeProviderRow))
      ) as Promise<GeneratedCommandResult<SortModeProviderRow[]>>,
  });
}

export async function sortModeProvidersSetOrder(input: {
  mode_id: number;
  cli_key: CliKey;
  ordered_provider_ids: number[];
}) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);
  validateOrderedProviderIds(input.ordered_provider_ids);

  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "更新排序模板供应商顺序失败",
    cmd: "sort_mode_providers_set_order",
    args: {
      modeId,
      cliKey,
      orderedProviderIds: input.ordered_provider_ids,
    },
    invoke: () =>
      commands
        .sortModeProvidersSetOrder(modeId, cliKey, input.ordered_provider_ids)
        .then((response) =>
          mapGeneratedCommandResponse(response, (rows) => rows.map(requireSortModeProviderRow))
        ) as Promise<GeneratedCommandResult<SortModeProviderRow[]>>,
  });
}

export async function sortModeProviderSetEnabled(input: {
  mode_id: number;
  cli_key: CliKey;
  provider_id: number;
  enabled: boolean;
}) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);
  validatePositiveId("providerId", input.provider_id);

  return invokeGeneratedIpc<SortModeProviderRow>({
    title: "更新排序模板供应商启用状态失败",
    cmd: "sort_mode_provider_set_enabled",
    args: {
      modeId,
      cliKey,
      providerId: input.provider_id,
      enabled: input.enabled,
    },
    invoke: () =>
      commands
        .sortModeProviderSetEnabled(modeId, cliKey, input.provider_id, input.enabled)
        .then((response) =>
          mapGeneratedCommandResponse(response, requireSortModeProviderRow)
        ) as Promise<GeneratedCommandResult<SortModeProviderRow>>,
  });
}

export async function sortModeProviderSetSessionReusePriority(input: {
  mode_id: number;
  cli_key: CliKey;
  provider_id: number;
  session_reuse_priority: number;
}) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);
  validatePositiveId("providerId", input.provider_id);
  const sessionReusePriority = validateSessionReusePriority(input.session_reuse_priority);

  return invokeGeneratedIpc<SortModeProviderRow>({
    title: "更新排序模板会话复用优先级失败",
    cmd: "sort_mode_provider_set_session_reuse_priority",
    args: {
      modeId,
      cliKey,
      providerId: input.provider_id,
      sessionReusePriority,
    },
    invoke: () =>
      commands
        .sortModeProviderSetSessionReusePriority(
          modeId,
          cliKey,
          input.provider_id,
          sessionReusePriority
        )
        .then((response) =>
          mapGeneratedCommandResponse(response, requireSortModeProviderRow)
        ) as Promise<GeneratedCommandResult<SortModeProviderRow>>,
  });
}

export async function providerModelRoutingPolicyGet(input: {
  provider_id: number;
  provider_uuid: string;
  mode_id: number | null;
  mode_uuid: string | null;
}) {
  const providerId = validateProviderId(input.provider_id);
  const providerUuid = validateCanonicalUuid(input.provider_uuid, "providerUuid");
  const { modeId, modeUuid } = validateModeIdentity(input.mode_id, input.mode_uuid);
  const expected = { providerId, providerUuid, modeId, modeUuid };

  return invokeGeneratedIpc<ProviderModelRoutingPolicyView>({
    title: "读取供应商模型路由失败",
    cmd: "provider_model_routing_policy_get",
    args: { providerId, providerUuid, modeId, modeUuid },
    invoke: () =>
      commands
        .providerModelRoutingPolicyGet(providerId, providerUuid, modeId, modeUuid)
        .then((response) =>
          mapGeneratedCommandResponse(response, (value) => requireRoutingPolicyView(value, expected))
        ) as Promise<GeneratedCommandResult<ProviderModelRoutingPolicyView>>,
  });
}

export async function providerModelRoutingPolicySave(input: ProviderModelRoutingPolicySaveInput) {
  const providerId = validateProviderId(input.provider_id);
  const providerUuid = validateCanonicalUuid(input.provider_uuid, "providerUuid");
  const { modeId, modeUuid } = validateModeIdentity(input.mode_id, input.mode_uuid);
  const ordinaryPolicy = normalizeModelRoutingPolicy(input.ordinary_policy);
  const ordinaryError = validateModelRoutingPolicy(ordinaryPolicy);
  if (ordinaryError != null) throw new FeValidationError(`SEC_INVALID_INPUT: ${ordinaryError}`);
  const expectedOrdinaryPolicyRevision = validateRoutingPolicyRevision(
    input.expected_ordinary_policy_revision,
    "expectedOrdinaryPolicyRevision"
  );

  if (
    modeId == null &&
    (input.cross_policy != null || input.expected_cross_policy_revision != null)
  ) {
    throw new FeValidationError("SEC_INVALID_INPUT: Default cannot save cross-provider policy");
  }
  const crossPolicy =
    input.cross_policy == null
      ? null
      : normalizeCrossProviderModelRoutingPolicy(input.cross_policy);
  if (crossPolicy != null) {
    const crossError = validateCrossProviderModelRoutingPolicy(crossPolicy);
    if (crossError != null) throw new FeValidationError(`SEC_INVALID_INPUT: ${crossError}`);
  }
  const expectedCrossPolicyRevision =
    input.expected_cross_policy_revision == null
      ? null
      : validateRoutingPolicyRevision(
          input.expected_cross_policy_revision,
          "expectedCrossPolicyRevision"
        );
  const payload: GeneratedProviderModelRoutingPolicySaveInput = {
    providerId,
    providerUuid,
    modeId,
    modeUuid,
    providerOverrideEnabled: input.provider_override_enabled,
    ordinaryPolicy,
    expectedOrdinaryPolicyRevision,
    crossPolicy,
    expectedCrossPolicyRevision,
  };
  const expected = { providerId, providerUuid, modeId, modeUuid };

  return invokeGeneratedIpc<ProviderModelRoutingPolicyView>({
    title: "保存供应商模型路由失败",
    cmd: "provider_model_routing_policy_save",
    args: {
      providerId,
      providerUuid,
      modeId,
      modeUuid,
      expectedOrdinaryPolicyRevision,
      expectedCrossPolicyRevision,
    },
    invoke: () =>
      commands.providerModelRoutingPolicySave(payload).then((response) =>
        mapGeneratedCommandResponse(response, (value) => requireRoutingPolicyView(value, expected))
      ) as Promise<GeneratedCommandResult<ProviderModelRoutingPolicyView>>,
  });
}

export async function routingProviderCandidatesList(input: {
  mode_id: number;
  mode_uuid: string;
  cli_key: CliKey;
}) {
  const modeId = validateSortModeId(input.mode_id);
  const modeUuid = validateSortModeUuid(input.mode_uuid);
  const cliKey = validateProviderCliKey(input.cli_key);

  return invokeGeneratedIpc<RoutingProviderCandidate[]>({
    title: "读取跨供应商路由候选失败",
    cmd: "routing_provider_candidates_list",
    args: { modeId, modeUuid, cliKey },
    invoke: () =>
      commands.routingProviderCandidatesList(modeId, modeUuid, cliKey).then((response) =>
        mapGeneratedCommandResponse(response, (rows) =>
          rows.map((row) => requireRoutingProviderCandidate(row, cliKey))
        )
      ) as Promise<GeneratedCommandResult<RoutingProviderCandidate[]>>,
  });
}
