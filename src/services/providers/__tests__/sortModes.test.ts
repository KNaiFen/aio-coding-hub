import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import {
  MAX_SORT_MODE_NAME_CHARS,
  MAX_SORT_MODE_PROVIDER_IDS,
  type ProviderModelRoutingPolicyView,
  type SortModeActiveRow,
  type SortModeProviderRow,
  type SortModeSummary,
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProvidersList,
  sortModeProviderSetEnabled,
  sortModeProviderSetSessionReusePriority,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
  providerModelRoutingPolicyGet,
  providerModelRoutingPolicySave,
  routingProviderCandidatesList,
  validateRoutingPolicyRevision,
  validateSortModeId,
  validateSortModeUuid,
} from "../sortModes";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      sortModesList: vi.fn(),
      sortModeCreate: vi.fn(),
      sortModeRename: vi.fn(),
      sortModeDelete: vi.fn(),
      sortModeActiveList: vi.fn(),
      sortModeActiveSet: vi.fn(),
      sortModeProvidersList: vi.fn(),
      sortModeProvidersSetOrder: vi.fn(),
      sortModeProviderSetEnabled: vi.fn(),
      sortModeProviderSetSessionReusePriority: vi.fn(),
      providerModelRoutingPolicyGet: vi.fn(),
      providerModelRoutingPolicySave: vi.fn(),
      routingProviderCandidatesList: vi.fn(),
    },
  };
});

const MODE_UUID = "11111111-1111-4111-8111-111111111111";
const PROVIDER_UUID = "22222222-2222-4222-8222-222222222222";
const TARGET_UUID = "33333333-3333-4333-8333-333333333333";
const REVISION = "a".repeat(64);

function makeSortModeSummary(overrides: Partial<SortModeSummary> = {}): SortModeSummary {
  return {
    id: 1,
    mode_uuid: MODE_UUID,
    name: "Work",
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

function makeSortModeActiveRow(overrides: Partial<SortModeActiveRow> = {}): SortModeActiveRow {
  return {
    cli_key: "claude",
    mode_id: 1,
    updated_at: 0,
    ...overrides,
  };
}

function makeSortModeProviderRow(
  overrides: Partial<SortModeProviderRow> = {}
): SortModeProviderRow {
  return {
    provider_id: 101,
    provider_uuid: "22222222-2222-4222-8222-222222222222",
    enabled: true,
    session_reuse_priority: 0,
    cross_policy: null,
    ...overrides,
  };
}

function makeRoutingPolicyView(
  overrides: Partial<ProviderModelRoutingPolicyView> = {}
): ProviderModelRoutingPolicyView {
  return {
    provider_id: 101,
    provider_uuid: PROVIDER_UUID,
    cli_key: "claude",
    provider_override_enabled: true,
    ordinary_policy: { enabled: true, rules: [] },
    ordinary_policy_revision: REVISION,
    selected_mode: { mode_id: 1, mode_uuid: MODE_UUID, name: "Work" },
    cross_policy: { enabled: true, rules: [] },
    cross_policy_revision: REVISION,
    source_member_enabled: true,
    source_member_present: true,
    ...overrides,
  };
}

describe("services/providers/sortModes", () => {
  it("invokes sort mode commands with expected parameters", async () => {
    vi.mocked(commands.sortModesList).mockResolvedValue({ status: "ok", data: [] });
    vi.mocked(commands.sortModeCreate).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 2, name: "M1" }),
    });
    vi.mocked(commands.sortModeRename).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 1, name: "M2" }),
    });
    vi.mocked(commands.sortModeDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.sortModeActiveList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeActiveRow()],
    });
    vi.mocked(commands.sortModeActiveSet).mockResolvedValue({
      status: "ok",
      data: makeSortModeActiveRow({ mode_id: null }),
    });
    vi.mocked(commands.sortModeProvidersList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProvidersSetOrder).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow({ provider_id: 9 })],
    });
    vi.mocked(commands.sortModeProviderSetEnabled).mockResolvedValue({
      status: "ok",
      data: makeSortModeProviderRow({ provider_id: 9, enabled: false }),
    });
    vi.mocked(commands.sortModeProviderSetSessionReusePriority).mockResolvedValue({
      status: "ok",
      data: makeSortModeProviderRow({ provider_id: 9, session_reuse_priority: 75 }),
    });

    await sortModesList();
    expect(commands.sortModesList).toHaveBeenCalledWith();

    await sortModeCreate({ name: "M1" });
    expect(commands.sortModeCreate).toHaveBeenCalledWith("M1");

    await sortModeRename({ mode_id: 1, name: "M2" });
    expect(commands.sortModeRename).toHaveBeenCalledWith(1, "M2");

    await sortModeDelete({ mode_id: 2 });
    expect(commands.sortModeDelete).toHaveBeenCalledWith(2);

    await sortModeActiveList();
    expect(commands.sortModeActiveList).toHaveBeenCalledWith();

    await sortModeActiveSet({ cli_key: "claude", mode_id: null });
    expect(commands.sortModeActiveSet).toHaveBeenCalledWith("claude", null);

    await sortModeProvidersList({ mode_id: 3, cli_key: "codex" });
    expect(commands.sortModeProvidersList).toHaveBeenCalledWith(3, "codex");

    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: "gemini",
      ordered_provider_ids: [9, 8, 7],
    });
    expect(commands.sortModeProvidersSetOrder).toHaveBeenCalledWith(4, "gemini", [9, 8, 7]);

    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: "claude",
      provider_id: 9,
      enabled: false,
    });
    expect(commands.sortModeProviderSetEnabled).toHaveBeenCalledWith(5, "claude", 9, false);

    await sortModeProviderSetSessionReusePriority({
      mode_id: 5,
      cli_key: "claude",
      provider_id: 9,
      session_reuse_priority: 75,
    });
    expect(commands.sortModeProviderSetSessionReusePriority).toHaveBeenCalledWith(
      5,
      "claude",
      9,
      75
    );
  });

  it("normalizes and validates sort mode command inputs before IPC", async () => {
    vi.mocked(commands.sortModeCreate).mockClear();
    vi.mocked(commands.sortModeRename).mockClear();
    vi.mocked(commands.sortModeActiveSet).mockClear();
    vi.mocked(commands.sortModeProvidersList).mockClear();
    vi.mocked(commands.sortModeProvidersSetOrder).mockClear();
    vi.mocked(commands.sortModeProviderSetEnabled).mockClear();
    vi.mocked(commands.sortModeProviderSetSessionReusePriority).mockClear();

    vi.mocked(commands.sortModeCreate).mockResolvedValue({
      status: "ok",
      data: makeSortModeSummary({ id: 10, name: "Trimmed" }),
    });
    vi.mocked(commands.sortModeActiveSet).mockResolvedValue({
      status: "ok",
      data: makeSortModeActiveRow({ cli_key: "claude", mode_id: 2 }),
    });
    vi.mocked(commands.sortModeProvidersList).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProvidersSetOrder).mockResolvedValue({
      status: "ok",
      data: [makeSortModeProviderRow()],
    });
    vi.mocked(commands.sortModeProviderSetEnabled).mockResolvedValue({
      status: "ok",
      data: makeSortModeProviderRow({ enabled: false }),
    });

    await sortModeCreate({ name: "  Trimmed  " });
    expect(commands.sortModeCreate).toHaveBeenCalledWith("Trimmed");
    await sortModeActiveSet({ cli_key: " claude " as never, mode_id: 2 });
    await sortModeProvidersList({ mode_id: 3, cli_key: " codex " as never });
    await sortModeProvidersSetOrder({
      mode_id: 4,
      cli_key: " gemini " as never,
      ordered_provider_ids: [9, 8, 7],
    });
    await sortModeProviderSetEnabled({
      mode_id: 5,
      cli_key: " claude " as never,
      provider_id: 9,
      enabled: false,
    });

    expect(commands.sortModeActiveSet).toHaveBeenCalledWith("claude", 2);
    expect(commands.sortModeProvidersList).toHaveBeenCalledWith(3, "codex");
    expect(commands.sortModeProvidersSetOrder).toHaveBeenCalledWith(4, "gemini", [9, 8, 7]);
    expect(commands.sortModeProviderSetEnabled).toHaveBeenCalledWith(5, "claude", 9, false);

    vi.mocked(commands.sortModeRename).mockClear();
    vi.mocked(commands.sortModeProvidersSetOrder).mockClear();
    vi.mocked(commands.sortModeProviderSetEnabled).mockClear();

    await expect(sortModeCreate({ name: "" })).rejects.toThrow("mode name is required");
    await expect(sortModeCreate({ name: "default" })).rejects.toThrow("mode name is reserved");
    await expect(
      sortModeCreate({ name: "x".repeat(MAX_SORT_MODE_NAME_CHARS + 1) })
    ).rejects.toThrow("mode name is too long");

    await expect(sortModeRename({ mode_id: 0, name: "Next" })).rejects.toThrow("invalid modeId=0");
    expect(() => validateSortModeId(0)).toThrow("SEC_INVALID_INPUT");
    await expect(
      sortModeProvidersSetOrder({ mode_id: 1, cli_key: "claude", ordered_provider_ids: [1, 0] })
    ).rejects.toThrow("invalid providerId=0");
    await expect(
      sortModeProvidersSetOrder({
        mode_id: 1,
        cli_key: "opencode" as never,
        ordered_provider_ids: [1],
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      sortModeProvidersSetOrder({ mode_id: 1, cli_key: "claude", ordered_provider_ids: [1, 1] })
    ).rejects.toThrow("duplicate providerId=1");
    await expect(
      sortModeProvidersSetOrder({
        mode_id: 1,
        cli_key: "claude",
        ordered_provider_ids: Array.from(
          { length: MAX_SORT_MODE_PROVIDER_IDS + 1 },
          (_, index) => index + 1
        ),
      })
    ).rejects.toThrow("orderedProviderIds must contain at most");
    await expect(
      sortModeProviderSetEnabled({ mode_id: 1, cli_key: "claude", provider_id: -1, enabled: true })
    ).rejects.toThrow("invalid providerId=-1");
    await expect(
      sortModeProviderSetSessionReusePriority({
        mode_id: 1,
        cli_key: "claude",
        provider_id: 1,
        session_reuse_priority: 1001,
      })
    ).rejects.toThrow("sessionReusePriority must be between 0 and 1000");

    expect(commands.sortModeRename).not.toHaveBeenCalled();
    expect(commands.sortModeProvidersSetOrder).not.toHaveBeenCalled();
    expect(commands.sortModeProviderSetEnabled).not.toHaveBeenCalled();
    expect(commands.sortModeProviderSetSessionReusePriority).not.toHaveBeenCalled();
  });

  it("gets and saves combined routing policy under exact identities", async () => {
    const view = makeRoutingPolicyView();
    vi.mocked(commands.providerModelRoutingPolicyGet).mockResolvedValue({
      status: "ok",
      data: view,
    });
    vi.mocked(commands.providerModelRoutingPolicySave).mockResolvedValue({
      status: "ok",
      data: view,
    });

    await expect(
      providerModelRoutingPolicyGet({
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: MODE_UUID,
      })
    ).resolves.toEqual(view);
    expect(commands.providerModelRoutingPolicyGet).toHaveBeenCalledWith(
      101,
      PROVIDER_UUID,
      1,
      MODE_UUID
    );

    await providerModelRoutingPolicySave({
      provider_id: 101,
      provider_uuid: PROVIDER_UUID,
      mode_id: 1,
      mode_uuid: MODE_UUID,
      provider_override_enabled: true,
      ordinary_policy: {
        enabled: true,
        rules: [
          {
            source_model: " source ",
            source_reasoning_effort: " HIGH ",
            target_model: " target ",
            reasoning_effort: " LOW ",
          },
        ],
      },
      expected_ordinary_policy_revision: REVISION,
      cross_policy: {
        enabled: true,
        rules: [
          {
            source_model: " source ",
            source_reasoning_effort: " HIGH ",
            target_provider_uuid: TARGET_UUID,
            target_model: " target ",
            target_reasoning_effort: " MEDIUM ",
          },
        ],
      },
      expected_cross_policy_revision: REVISION,
    });

    expect(commands.providerModelRoutingPolicySave).toHaveBeenCalledWith({
      providerId: 101,
      providerUuid: PROVIDER_UUID,
      modeId: 1,
      modeUuid: MODE_UUID,
      providerOverrideEnabled: true,
      ordinaryPolicy: {
        enabled: true,
        rules: [
          {
            source_model: "source",
            source_reasoning_effort: "high",
            target_model: "target",
            reasoning_effort: "low",
          },
        ],
      },
      expectedOrdinaryPolicyRevision: REVISION,
      crossPolicy: {
        enabled: true,
        rules: [
          {
            source_model: "source",
            source_reasoning_effort: "high",
            target_provider_uuid: TARGET_UUID,
            target_model: "target",
            target_reasoning_effort: "medium",
          },
        ],
      },
      expectedCrossPolicyRevision: REVISION,
    });
  });

  it("rejects malformed identities, revisions, and Default cross policy before IPC", async () => {
    vi.mocked(commands.providerModelRoutingPolicyGet).mockClear();
    vi.mocked(commands.providerModelRoutingPolicySave).mockClear();

    expect(() => validateSortModeUuid("not-a-uuid")).toThrow("SEC_INVALID_INPUT");
    expect(() => validateRoutingPolicyRevision("A".repeat(64))).toThrow("SEC_INVALID_INPUT");
    await expect(
      providerModelRoutingPolicyGet({
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: null,
      })
    ).rejects.toThrow("modeId and modeUuid must be provided together");
    await expect(
      providerModelRoutingPolicySave({
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: null,
        mode_uuid: null,
        provider_override_enabled: true,
        ordinary_policy: { enabled: true, rules: [] },
        expected_ordinary_policy_revision: REVISION,
        cross_policy: { enabled: true, rules: [] },
        expected_cross_policy_revision: null,
      })
    ).rejects.toThrow("Default cannot save cross-provider policy");
    expect(commands.providerModelRoutingPolicyGet).not.toHaveBeenCalled();
    expect(commands.providerModelRoutingPolicySave).not.toHaveBeenCalled();
  });

  it("rejects an identity-mismatched policy response", async () => {
    vi.mocked(commands.providerModelRoutingPolicyGet).mockResolvedValue({
      status: "ok",
      data: makeRoutingPolicyView({ provider_uuid: TARGET_UUID }),
    });

    await expect(
      providerModelRoutingPolicyGet({
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: MODE_UUID,
      })
    ).rejects.toThrow("IPC_INVALID_SCOPE");
  });

  it("projects candidate responses to the bounded non-secret DTO", async () => {
    vi.mocked(commands.routingProviderCandidatesList).mockResolvedValue({
      status: "ok",
      data: [
        {
          provider_id: 102,
          provider_uuid: TARGET_UUID,
          cli_key: "claude",
          name: "Target",
          enabled: true,
          source_provider_id: null,
          bridge_type: null,
          model_catalog_supported: true,
          api_key: "must-not-escape",
          base_urls: ["https://sensitive.invalid"],
        } as never,
      ],
    });

    await expect(
      routingProviderCandidatesList({
        mode_id: 1,
        mode_uuid: MODE_UUID,
        cli_key: "claude",
      })
    ).resolves.toEqual([
      {
        provider_id: 102,
        provider_uuid: TARGET_UUID,
        cli_key: "claude",
        name: "Target",
        enabled: true,
        source_provider_id: null,
        bridge_type: null,
        model_catalog_supported: true,
      },
    ]);
  });
});
