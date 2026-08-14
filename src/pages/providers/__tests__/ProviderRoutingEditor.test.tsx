import type { ReactElement } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  ProviderEditorDialog,
  type ProviderEditorDialogProps,
  type ProviderEditorRouteMode,
} from "../ProviderEditorDialog";
import {
  providerModelRoutingPolicyGet,
  providerModelRoutingPolicySave,
  routingProviderCandidatesList,
  type ProviderModelRoutingPolicyView,
  type RoutingProviderCandidate,
} from "../../../services/providers/sortModes";
import { providerModelsGet } from "../../../services/providers/providerModels";
import {
  providerUpsert,
  type ProviderSummary,
} from "../../../services/providers/providers";
import { createTestQueryClient } from "../../../test/utils/reactQuery";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../services/providers/sortModes", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/sortModes")>(
    "../../../services/providers/sortModes"
  );
  return {
    ...actual,
    providerModelRoutingPolicyGet: vi.fn(),
    providerModelRoutingPolicySave: vi.fn(),
    routingProviderCandidatesList: vi.fn(),
  };
});
vi.mock("../../../services/providers/providerModels", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providerModels")>(
    "../../../services/providers/providerModels"
  );
  return { ...actual, providerModelsGet: vi.fn() };
});
vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return { ...actual, providerUpsert: vi.fn() };
});

const SOURCE_UUID = "11111111-1111-4111-8111-111111111111";
const TARGET_UUID = "22222222-2222-4222-8222-222222222222";
const MISSING_UUID = "33333333-3333-4333-8333-333333333333";
const MODE_ONE: ProviderEditorRouteMode = {
  modeId: 11,
  modeUuid: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  name: "工作日",
};
const MODE_TWO: ProviderEditorRouteMode = {
  modeId: 12,
  modeUuid: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
  name: "备用",
};
const REVISION = "a".repeat(64);

function provider(partial: Partial<ProviderSummary> = {}): ProviderSummary {
  return {
    id: 1,
    provider_uuid: SOURCE_UUID,
    cli_key: "grok",
    name: "Source",
    base_urls: ["https://example.com/v1"],
    base_url_mode: "order",
    claude_models: {},
    model_mapping: { default_model: null, exact: {} },
    enabled: true,
    priority: 0,
    cost_multiplier: 1,
    limit_5h_usd: null,
    limit_daily_usd: null,
    daily_reset_mode: "fixed",
    daily_reset_time: "00:00:00",
    limit_weekly_usd: null,
    limit_monthly_usd: null,
    limit_total_usd: null,
    tags: [],
    note: "",
    created_at: 0,
    updated_at: 0,
    auth_mode: "api_key",
    oauth_provider_type: null,
    oauth_email: null,
    oauth_expires_at: null,
    oauth_last_error: null,
    source_provider_id: null,
    bridge_type: null,
    availability_test_model: null,
    availability_probe_enabled: false,
    availability_probe_interval_minutes: 10,
    api_key_configured: true,
    newapi_account_user_id: null,
    newapi_account_access_token_configured: false,
    stream_idle_timeout_seconds: null,
    extension_values: [],
    upstream_retry_policy_override: null,
    model_routing_policy_override: null,
    ...partial,
  };
}

function policyView(
  input: {
    mode?: ProviderEditorRouteMode | null;
    memberPresent?: boolean;
    memberEnabled?: boolean;
    sourceModel?: string;
    ordinaryRevision?: string;
    crossTarget?: string | null;
  } = {}
): ProviderModelRoutingPolicyView {
  const mode = input.mode === undefined ? MODE_ONE : input.mode;
  const memberPresent = mode != null && (input.memberPresent ?? true);
  return {
    provider_id: 1,
    provider_uuid: SOURCE_UUID,
    cli_key: "grok",
    provider_override_enabled: true,
    ordinary_policy: {
      enabled: true,
      rules: [
        {
          source_model: input.sourceModel ?? "grok-source",
          source_reasoning_effort: null,
          target_model: "grok-target",
          reasoning_effort: null,
        },
      ],
    },
    ordinary_policy_revision: input.ordinaryRevision ?? REVISION,
    selected_mode:
      mode == null
        ? null
        : { mode_id: mode.modeId, mode_uuid: mode.modeUuid, name: mode.name },
    cross_policy:
      memberPresent && input.crossTarget
        ? {
            enabled: true,
            rules: [
              {
                source_model: "cross-source",
                source_reasoning_effort: "high",
                target_provider_uuid: input.crossTarget,
                target_model: null,
                target_reasoning_effort: null,
              },
            ],
          }
        : null,
    cross_policy_revision: memberPresent ? "b".repeat(64) : null,
    source_member_enabled: memberPresent && (input.memberEnabled ?? true),
    source_member_present: memberPresent,
  };
}

function candidates(): RoutingProviderCandidate[] {
  return [
    {
      provider_id: 1,
      provider_uuid: SOURCE_UUID,
      cli_key: "grok",
      name: "Source",
      enabled: true,
      source_provider_id: null,
      bridge_type: null,
      model_catalog_supported: false,
    },
    {
      provider_id: 2,
      provider_uuid: TARGET_UUID,
      cli_key: "grok",
      name: "Target",
      enabled: true,
      source_provider_id: null,
      bridge_type: null,
      model_catalog_supported: true,
    },
  ];
}

function renderDialog(ui: ReactElement) {
  const client = createTestQueryClient();
  const view = render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
  return {
    ...view,
    rerender: (nextUi: ReactElement) =>
      view.rerender(<QueryClientProvider client={client}>{nextUi}</QueryClientProvider>),
  };
}

function editor(
  overrides: Partial<Extract<ProviderEditorDialogProps, { mode: "edit" }>> = {}
) {
  return (
    <ProviderEditorDialog
      mode="edit"
      open={true}
      provider={provider()}
      routeMode={MODE_ONE}
      routeModes={[MODE_ONE, MODE_TWO]}
      onRouteModeChange={vi.fn()}
      onSaved={vi.fn()}
      onOpenChange={vi.fn()}
      {...overrides}
    />
  );
}

describe("pages/providers/ProviderRoutingEditor", () => {
  beforeEach(() => {
    vi.mocked(providerModelRoutingPolicyGet).mockReset();
    vi.mocked(providerModelRoutingPolicyGet).mockImplementation(async (input) =>
      policyView({
        mode:
          input.mode_id == null
            ? null
            : input.mode_id === MODE_TWO.modeId
              ? MODE_TWO
              : MODE_ONE,
      })
    );
    vi.mocked(providerModelRoutingPolicySave).mockReset();
    vi.mocked(providerModelRoutingPolicySave).mockImplementation(async (input) => ({
      ...policyView({
        mode:
          input.mode_id == null
            ? null
            : input.mode_id === MODE_TWO.modeId
              ? MODE_TWO
              : MODE_ONE,
      }),
      provider_override_enabled: input.provider_override_enabled,
      ordinary_policy: input.ordinary_policy,
      cross_policy: input.cross_policy,
    }));
    vi.mocked(routingProviderCandidatesList).mockReset();
    vi.mocked(routingProviderCandidatesList).mockResolvedValue(candidates());
    vi.mocked(providerModelsGet).mockReset();
    vi.mocked(providerModelsGet).mockResolvedValue({
      providerId: 2,
      providerUuid: TARGET_UUID,
      protocol: "openai_compatible",
      stale: false,
      lastAttemptAt: 1,
      lastSuccessAt: 1,
      lastErrorCode: null,
      models: [
        {
          modelUuid: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
          providerId: 2,
          remoteModelId: "target-catalog-model",
          source: "discovered",
          stale: false,
          lastSeenAt: 1,
          createdAt: 1,
          updatedAt: 1,
          capabilitiesConfigured: false,
          supportedReasoningEfforts: [],
          defaultReasoningEffort: null,
          contextWindow: null,
        },
      ],
    });
    vi.mocked(providerUpsert).mockReset();
    vi.mocked(providerUpsert).mockResolvedValue(provider());
  });

  it("uses strict source and target effort selects for ordinary rules", async () => {
    renderDialog(editor());

    const sourceEffort = await screen.findByLabelText("模型路由规则 1 来源思考强度");
    const targetEffort = screen.getByLabelText("模型路由规则 1 目标思考强度");
    const expected = ["", "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"];

    expect(sourceEffort.tagName).toBe("SELECT");
    expect(targetEffort.tagName).toBe("SELECT");
    expect(screen.getByText("目标供应商：本供应商（Source）")).toBeInTheDocument();
    expect(
      within(sourceEffort)
        .getAllByRole("option")
        .map((option) => option.getAttribute("value"))
    ).toEqual(expected);
    expect(
      within(targetEffort)
        .getAllByRole("option")
        .map((option) => option.getAttribute("value"))
    ).toEqual(expected);
  });

  it("syncs a clean ordinary draft when another mode exposes a newer revision", async () => {
    vi.mocked(providerModelRoutingPolicyGet).mockImplementation(async (input) =>
      policyView({
        mode: input.mode_id === MODE_TWO.modeId ? MODE_TWO : MODE_ONE,
        sourceModel: input.mode_id === MODE_TWO.modeId ? "latest-source" : "grok-source",
        ordinaryRevision: input.mode_id === MODE_TWO.modeId ? "c".repeat(64) : REVISION,
      })
    );
    const view = renderDialog(editor());
    expect(await screen.findByDisplayValue("grok-source")).toBeInTheDocument();

    view.rerender(editor({ routeMode: MODE_TWO }));
    expect(await screen.findByDisplayValue("latest-source")).toBeInTheDocument();

    view.rerender(editor({ open: false, routeMode: MODE_TWO }));
    await waitFor(() =>
      expect(screen.queryByDisplayValue("latest-source")).not.toBeInTheDocument()
    );
    view.rerender(editor({ open: true, routeMode: MODE_TWO }));
    expect(await screen.findByDisplayValue("latest-source")).toBeInTheDocument();
  });

  it("disables cross rules for Default and a missing source member", async () => {
    const { rerender } = renderDialog(editor({ routeMode: null }));
    expect(await screen.findByText(/Default 不支持跨供应商目标/)).toBeInTheDocument();

    vi.mocked(providerModelRoutingPolicyGet).mockResolvedValueOnce(
      policyView({ mode: MODE_ONE, memberPresent: false })
    );
    rerender(editor({ routeMode: MODE_ONE }));
    expect(await screen.findByText(/当前供应商不在该方案中/)).toBeInTheDocument();
    expect(screen.getByLabelText("模型路由规则 1 来源思考强度")).toBeEnabled();
  });

  it("creates a cross rule with the first eligible target and offers catalog suggestions", async () => {
    renderDialog(editor());
    const addRule = await screen.findByRole("button", { name: "新增跨规则" });
    await waitFor(() => expect(addRule).toBeEnabled());
    fireEvent.click(addRule);

    const targetSelect = screen.getByLabelText("跨供应商模型路由规则 1 目标供应商");
    expect(targetSelect).toHaveValue(TARGET_UUID);
    await waitFor(() => expect(providerModelsGet).toHaveBeenCalledWith(2, TARGET_UUID));
    await waitFor(() =>
      expect(
        document.querySelector('datalist option[value="target-catalog-model"]')
      ).not.toBeNull()
    );
    fireEvent.click(
      screen.getByRole("button", { name: "删除跨供应商模型路由规则 1" })
    );
    expect(
      screen.queryByRole("group", { name: "跨供应商模型路由规则 1" })
    ).not.toBeInTheDocument();
  });

  it("keeps an invalid stored target visible without rewriting it", async () => {
    vi.mocked(providerModelRoutingPolicyGet).mockResolvedValue(
      policyView({ crossTarget: MISSING_UUID })
    );
    renderDialog(editor());

    const targetSelect = await screen.findByLabelText(
      "跨供应商模型路由规则 1 目标供应商"
    );
    expect(targetSelect).toHaveValue(MISSING_UUID);
    expect(screen.getByText(/该目标已失效/)).toBeInTheDocument();
  });

  it("saves provider fields before the revision-guarded combined routing policy", async () => {
    renderDialog(editor());
    const sourceInput = await screen.findByDisplayValue("grok-source");
    fireEvent.change(sourceInput, { target: { value: "changed-source" } });
    const save = screen.getByRole("button", { name: "保存" });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);

    await waitFor(() => expect(providerModelRoutingPolicySave).toHaveBeenCalledOnce());
    expect(vi.mocked(providerUpsert).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(providerModelRoutingPolicySave).mock.invocationCallOrder[0]
    );
    expect(providerModelRoutingPolicySave).toHaveBeenCalledWith(
      expect.objectContaining({
        provider_id: 1,
        provider_uuid: SOURCE_UUID,
        mode_id: MODE_ONE.modeId,
        mode_uuid: MODE_ONE.modeUuid,
        expected_ordinary_policy_revision: REVISION,
        expected_cross_policy_revision: "b".repeat(64),
        ordinary_policy: expect.objectContaining({
          rules: [expect.objectContaining({ source_model: "changed-source" })],
        }),
      })
    );
    expect(providerUpsert).toHaveBeenCalledWith(
      expect.not.objectContaining({ modelRoutingPolicyOverride: expect.anything() })
    );
  });

  it("keeps ordinary edits across a mode change and confirms dirty cross drafts", async () => {
    vi.mocked(providerModelRoutingPolicyGet).mockImplementation(async (input) =>
      policyView({
        mode: input.mode_id === MODE_TWO.modeId ? MODE_TWO : MODE_ONE,
        sourceModel: input.mode_id === MODE_TWO.modeId ? "new-server-source" : "grok-source",
        ordinaryRevision: input.mode_id === MODE_TWO.modeId ? "c".repeat(64) : REVISION,
      })
    );
    const onRouteModeChange = vi.fn();
    const view = renderDialog(editor({ onRouteModeChange }));
    const ordinaryInput = await screen.findByDisplayValue("grok-source");
    fireEvent.change(ordinaryInput, { target: { value: "ordinary-draft" } });
    const addRule = screen.getByRole("button", { name: "新增跨规则" });
    await waitFor(() => expect(addRule).toBeEnabled());
    fireEvent.click(addRule);

    fireEvent.change(screen.getByLabelText("跨供应商模型路由方案"), {
      target: { value: String(MODE_TWO.modeId) },
    });
    const firstPrompt = screen.getByRole("dialog", { name: "保存跨供应商规则草稿？" });
    expect(firstPrompt).toBeInTheDocument();
    fireEvent.click(within(firstPrompt).getByRole("button", { name: "取消" }));
    expect(onRouteModeChange).not.toHaveBeenCalled();
    expect(screen.getByDisplayValue("ordinary-draft")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("跨供应商模型路由方案"), {
      target: { value: String(MODE_TWO.modeId) },
    });
    const discardPrompt = screen.getByRole("dialog", { name: "保存跨供应商规则草稿？" });
    fireEvent.click(within(discardPrompt).getByRole("button", { name: "放弃草稿" }));
    expect(onRouteModeChange).toHaveBeenCalledWith(MODE_TWO.modeId);

    view.rerender(editor({ routeMode: MODE_TWO, onRouteModeChange }));
    expect(await screen.findByDisplayValue("ordinary-draft")).toBeInTheDocument();
  });

  it("saves a dirty cross draft before switching modes", async () => {
    const onRouteModeChange = vi.fn();
    renderDialog(editor({ onRouteModeChange }));
    const addRule = await screen.findByRole("button", { name: "新增跨规则" });
    await waitFor(() => expect(addRule).toBeEnabled());
    fireEvent.click(addRule);

    fireEvent.change(screen.getByLabelText("跨供应商模型路由方案"), {
      target: { value: String(MODE_TWO.modeId) },
    });
    const prompt = screen.getByRole("dialog", { name: "保存跨供应商规则草稿？" });
    fireEvent.click(within(prompt).getByRole("button", { name: "保存并切换" }));

    await waitFor(() => expect(providerModelRoutingPolicySave).toHaveBeenCalledOnce());
    expect(providerModelRoutingPolicySave).toHaveBeenCalledWith(
      expect.objectContaining({
        mode_id: MODE_ONE.modeId,
        mode_uuid: MODE_ONE.modeUuid,
        cross_policy: expect.objectContaining({
          rules: [expect.objectContaining({ target_provider_uuid: TARGET_UUID })],
        }),
      })
    );
    expect(onRouteModeChange).toHaveBeenCalledWith(MODE_TWO.modeId);
  });
});
