import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  SortModeActiveRow,
  SortModeProviderRow,
  SortModeSummary,
  ProviderModelRoutingPolicyView,
} from "../../services/providers/sortModes";
import {
  providerModelRoutingPolicyGet,
  providerModelRoutingPolicySave,
  routingProviderCandidatesList,
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProviderSetEnabled,
  sortModeProviderSetSessionReusePriority,
  sortModeProvidersList,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
} from "../../services/providers/sortModes";
import { createDeferred } from "../../test/utils/deferred";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { sortModesKeys } from "../keys";
import {
  sortModeProvidersQueryKey,
  sortModeProvidersQueryPrefix,
  providerRoutingPolicyQueryKey,
  routingProviderCandidatesQueryKey,
  useProviderRoutingPolicyQuery,
  useProviderRoutingPolicySaveMutation,
  useRoutingProviderCandidatesQuery,
  useSortModeActiveListQuery,
  useSortModeActiveSetMutation,
  useSortModeCreateMutation,
  useSortModeDeleteMutation,
  useSortModeProviderSetEnabledMutation,
  useSortModeProviderSetSessionReusePriorityMutation,
  useSortModeProvidersListQuery,
  useSortModeProvidersSetOrderMutation,
  useSortModeRenameMutation,
  useSortModesListQuery,
} from "../sortModes";

vi.mock("../../services/providers/sortModes", async () => {
  const actual = await vi.importActual<typeof import("../../services/providers/sortModes")>(
    "../../services/providers/sortModes"
  );
  return {
    ...actual,
    sortModesList: vi.fn(),
    sortModeActiveList: vi.fn(),
    sortModeActiveSet: vi.fn(),
    sortModeCreate: vi.fn(),
    sortModeRename: vi.fn(),
    sortModeDelete: vi.fn(),
    sortModeProvidersList: vi.fn(),
    sortModeProvidersSetOrder: vi.fn(),
    sortModeProviderSetEnabled: vi.fn(),
    sortModeProviderSetSessionReusePriority: vi.fn(),
    providerModelRoutingPolicyGet: vi.fn(),
    providerModelRoutingPolicySave: vi.fn(),
    routingProviderCandidatesList: vi.fn(),
  };
});

const MODE_UUID = "11111111-1111-4111-8111-111111111111";
const OTHER_MODE_UUID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const PROVIDER_UUID = "22222222-2222-4222-8222-222222222222";
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

describe("query/sortModes", () => {
  it("builds normalized CLI-wide prefixes and UUID-qualified mode keys", () => {
    const prefix = sortModeProvidersQueryPrefix(" claude " as never);

    expect(prefix).toEqual([...sortModesKeys.all, "providers", "claude"]);
    expect(sortModeProvidersQueryKey(7, MODE_UUID, " claude " as never)).toEqual([
      ...prefix,
      7,
      MODE_UUID,
    ]);
    expect(sortModeProvidersQueryKey(7, OTHER_MODE_UUID, "claude")).not.toEqual(
      sortModeProvidersQueryKey(7, MODE_UUID, "claude")
    );
    expect(() => sortModeProvidersQueryPrefix("opencode" as never)).toThrow("SEC_INVALID_INPUT");
  });

  it("calls sortModesList and sortModeActiveList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(sortModesList).mockResolvedValue([]);
    vi.mocked(sortModeActiveList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSortModesListQuery(), { wrapper });
    renderHook(() => useSortModeActiveListQuery(), { wrapper });

    await waitFor(() => {
      expect(sortModesList).toHaveBeenCalled();
      expect(sortModeActiveList).toHaveBeenCalled();
    });
  });

  it("calls sortModeProvidersList with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProvidersList).mockResolvedValue([makeSortModeProviderRow()]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useSortModeProvidersListQuery({
          modeId: 1,
          modeUuid: MODE_UUID,
          cliKey: " claude " as never,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(sortModeProvidersList).toHaveBeenCalledWith({ mode_id: 1, cli_key: "claude" });
    });

    expect(client.getQueryState(sortModeProvidersQueryKey(1, MODE_UUID, "claude"))).toBeTruthy();
    expect(
      client.getQueryState([...sortModesKeys.all, "providers", " claude ", 1] as const)
    ).toBeUndefined();
  });

  it("isolates provider policy and candidate caches by stable UUID scope", () => {
    expect(
      providerRoutingPolicyQueryKey({
        cliKey: "claude",
        providerId: 101,
        providerUuid: PROVIDER_UUID,
        modeId: 1,
        modeUuid: MODE_UUID,
      })
    ).not.toEqual(
      providerRoutingPolicyQueryKey({
        cliKey: "claude",
        providerId: 101,
        providerUuid: PROVIDER_UUID,
        modeId: 1,
        modeUuid: OTHER_MODE_UUID,
      })
    );
    expect(
      routingProviderCandidatesQueryKey({ cliKey: "claude", modeId: 1, modeUuid: MODE_UUID })
    ).not.toEqual(
      routingProviderCandidatesQueryKey({
        cliKey: "claude",
        modeId: 1,
        modeUuid: OTHER_MODE_UUID,
      })
    );
  });

  it("rejects incomplete provider and mode identities before creating routing keys", () => {
    expect(() =>
      providerRoutingPolicyQueryKey({
        cliKey: "claude",
        providerId: 101,
        providerUuid: null,
        modeId: null,
        modeUuid: null,
      })
    ).toThrow("SEC_INVALID_INPUT");
    expect(() =>
      providerRoutingPolicyQueryKey({
        cliKey: "claude",
        providerId: 101,
        providerUuid: PROVIDER_UUID,
        modeId: 1,
        modeUuid: null,
      })
    ).toThrow("SEC_INVALID_INPUT");
  });

  it("queries named provider policy and narrow candidates with the full identity scope", async () => {
    setTauriRuntime();
    vi.mocked(providerModelRoutingPolicyGet).mockResolvedValue(makeRoutingPolicyView());
    vi.mocked(routingProviderCandidatesList).mockResolvedValue([
      {
        provider_id: 102,
        provider_uuid: "33333333-3333-4333-8333-333333333333",
        cli_key: "claude",
        name: "Target",
        enabled: true,
        source_provider_id: null,
        bridge_type: null,
        model_catalog_supported: true,
      },
    ]);
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useProviderRoutingPolicyQuery({
          cliKey: "claude",
          providerId: 101,
          providerUuid: PROVIDER_UUID,
          modeId: 1,
          modeUuid: MODE_UUID,
        }),
      { wrapper }
    );
    renderHook(
      () =>
        useRoutingProviderCandidatesQuery({
          cliKey: "claude",
          modeId: 1,
          modeUuid: MODE_UUID,
        }),
      { wrapper }
    );

    await waitFor(() => {
      expect(providerModelRoutingPolicyGet).toHaveBeenCalledWith({
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: MODE_UUID,
      });
      expect(routingProviderCandidatesList).toHaveBeenCalledWith({
        mode_id: 1,
        mode_uuid: MODE_UUID,
        cli_key: "claude",
      });
    });
  });

  it("does not query cross-provider candidates for Default", () => {
    setTauriRuntime();
    vi.mocked(routingProviderCandidatesList).mockClear();
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(
      () =>
        useRoutingProviderCandidatesQuery({
          cliKey: "claude",
          modeId: null,
          modeUuid: null,
        }),
      { wrapper }
    );

    expect(result.current.fetchStatus).toBe("idle");
    expect(routingProviderCandidatesList).not.toHaveBeenCalled();
  });

  it("stores a saved policy only under its exact identity and invalidates its candidates", async () => {
    setTauriRuntime();
    const saved = makeRoutingPolicyView();
    vi.mocked(providerModelRoutingPolicySave).mockResolvedValue(saved);
    const client = createTestQueryClient();
    const otherKey = providerRoutingPolicyQueryKey({
      cliKey: "claude",
      providerId: 101,
      providerUuid: PROVIDER_UUID,
      modeId: 1,
      modeUuid: OTHER_MODE_UUID,
    });
    client.setQueryData(otherKey, { marker: "other identity" });
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useProviderRoutingPolicySaveMutation(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        cliKey: "claude",
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: MODE_UUID,
        provider_override_enabled: true,
        ordinary_policy: { enabled: true, rules: [] },
        expected_ordinary_policy_revision: REVISION,
        cross_policy: { enabled: true, rules: [] },
        expected_cross_policy_revision: REVISION,
      });
    });

    const exactKey = providerRoutingPolicyQueryKey({
      cliKey: "claude",
      providerId: 101,
      providerUuid: PROVIDER_UUID,
      modeId: 1,
      modeUuid: MODE_UUID,
    });
    expect(client.getQueryData(exactKey)).toEqual(saved);
    expect(client.getQueryData(otherKey)).toEqual({ marker: "other identity" });
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: routingProviderCandidatesQueryKey({
        cliKey: "claude",
        modeId: 1,
        modeUuid: MODE_UUID,
      }),
      exact: true,
    });
  });

  it("rejects a policy response that belongs to another CLI", async () => {
    setTauriRuntime();
    vi.mocked(providerModelRoutingPolicyGet).mockResolvedValue(
      makeRoutingPolicyView({ cli_key: "codex" })
    );
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(
      () =>
        useProviderRoutingPolicyQuery({
          cliKey: "claude",
          providerId: 101,
          providerUuid: PROVIDER_UUID,
          modeId: 1,
          modeUuid: MODE_UUID,
        }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error).toEqual(
      expect.objectContaining({ message: "IPC_INVALID_SCOPE: provider routing policy CLI" })
    );
  });

  it("rejects a saved policy response that belongs to another CLI", async () => {
    setTauriRuntime();
    vi.mocked(providerModelRoutingPolicySave).mockResolvedValue(
      makeRoutingPolicyView({ cli_key: "codex" })
    );
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => useProviderRoutingPolicySaveMutation(), { wrapper });

    await expect(
      result.current.mutateAsync({
        cliKey: "claude",
        provider_id: 101,
        provider_uuid: PROVIDER_UUID,
        mode_id: 1,
        mode_uuid: MODE_UUID,
        provider_override_enabled: true,
        ordinary_policy: { enabled: true, rules: [] },
        expected_ordinary_policy_revision: REVISION,
        cross_policy: { enabled: true, rules: [] },
        expected_cross_policy_revision: REVISION,
      })
    ).rejects.toThrow("IPC_INVALID_SCOPE: saved provider routing policy CLI");
  });

  it("rejects invalid sort mode provider cliKey before creating query adapters", () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    expect(() =>
      renderHook(
        () =>
          useSortModeProvidersListQuery({
            modeId: 1,
            modeUuid: MODE_UUID,
            cliKey: "opencode" as never,
          }),
        { wrapper }
      )
    ).toThrow("SEC_INVALID_INPUT");
    expect(sortModeProvidersList).not.toHaveBeenCalled();
  });

  it("rejects invalid sort mode ids before query keys or optimistic updates", async () => {
    setTauriRuntime();
    vi.mocked(sortModeActiveSet).mockClear();
    vi.mocked(sortModeProvidersList).mockClear();
    vi.mocked(sortModeProvidersSetOrder).mockClear();

    const previous: SortModeActiveRow[] = [{ cli_key: "claude", mode_id: 1, updated_at: 0 }];
    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    expect(() => sortModeProvidersQueryKey(0, MODE_UUID, "claude")).toThrow(
      "SEC_INVALID_INPUT"
    );
    expect(() =>
      renderHook(
        () =>
          useSortModeProvidersListQuery({
            modeId: 0,
            modeUuid: MODE_UUID,
            cliKey: "claude",
          }),
        { wrapper }
      )
    ).toThrow("SEC_INVALID_INPUT");
    expect(sortModeProvidersList).not.toHaveBeenCalled();

    const activeResult = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await expect(
      activeResult.result.current.mutateAsync({ cliKey: "claude", modeId: 0 })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(sortModeActiveSet).not.toHaveBeenCalled();
    expect(client.getQueryData(sortModesKeys.activeList())).toEqual(previous);

    const orderResult = renderHook(() => useSortModeProvidersSetOrderMutation(), { wrapper });
    await expect(
      orderResult.result.current.mutateAsync({
        modeId: 0,
        modeUuid: MODE_UUID,
        cliKey: "claude",
        orderedProviderIds: [101],
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    expect(sortModeProvidersSetOrder).not.toHaveBeenCalled();
    expect(invalidateSpy).not.toHaveBeenCalledWith({
      queryKey: [...sortModesKeys.all, "providers", "claude", 0, MODE_UUID] as const,
    });
  });

  it("useSortModesListQuery enters error state when sortModesList rejects", async () => {
    setTauriRuntime();

    vi.mocked(sortModesList).mockRejectedValue(new Error("sort modes query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModesListQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useSortModeActiveSetMutation optimistically updates activeList and invalidates on settle", async () => {
    setTauriRuntime();

    const previous: SortModeActiveRow[] = [
      { cli_key: "claude", mode_id: 1, updated_at: 0 },
      { cli_key: "gemini", mode_id: null, updated_at: 0 },
    ];
    const updated: SortModeActiveRow = { cli_key: "claude", mode_id: 2, updated_at: 123 };

    const deferred = createDeferred<SortModeActiveRow>();
    vi.mocked(sortModeActiveSet).mockImplementation(() => deferred.promise);

    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });

    act(() => {
      result.current.mutate({ cliKey: " claude " as never, modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual([
      { ...previous[0], mode_id: 2 },
      previous[1],
    ]);
    await waitFor(() => {
      expect(sortModeActiveSet).toHaveBeenCalledWith({ cli_key: "claude", mode_id: 2 });
    });

    deferred.resolve(updated);

    await act(async () => {
      await deferred.promise;
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual([updated, previous[1]]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("rolls back when sortModeActiveSet throws", async () => {
    setTauriRuntime();

    const previous: SortModeActiveRow[] = [{ cli_key: "claude", mode_id: 1, updated_at: 0 }];

    vi.mocked(sortModeActiveSet).mockRejectedValue(new Error("boom"));

    const client = createTestQueryClient();
    client.setQueryData(sortModesKeys.activeList(), previous);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      try {
        await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
      } catch {
        // expected
      }
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toEqual(previous);
  });

  it("invalidates without updating cache when activeList is missing", async () => {
    setTauriRuntime();

    const updated: SortModeActiveRow = { cli_key: "claude", mode_id: 2, updated_at: 123 };
    vi.mocked(sortModeActiveSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeActiveSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ cliKey: "claude", modeId: 2 });
    });

    expect(client.getQueryData(sortModesKeys.activeList())).toBeUndefined();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeCreateMutation invalidates list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeCreate).mockResolvedValue(makeSortModeSummary());

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeCreateMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ name: "Work" });
    });

    expect(sortModeCreate).toHaveBeenCalledWith({ name: "Work" });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeRenameMutation invalidates list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeRename).mockResolvedValue(makeSortModeSummary({ id: 2, name: "Life" }));

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeRenameMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ modeId: 2, name: "Life" });
    });

    expect(sortModeRename).toHaveBeenCalledWith({ mode_id: 2, name: "Life" });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).not.toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeDeleteMutation invalidates list and activeList on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeDelete).mockResolvedValue(true);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ modeId: 3, modeUuid: MODE_UUID });
    });

    expect(sortModeDelete).toHaveBeenCalledWith({ mode_id: 3 });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.list() });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: sortModesKeys.activeList() });
  });

  it("useSortModeProvidersSetOrderMutation invalidates the provider list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProvidersSetOrder).mockResolvedValue([makeSortModeProviderRow()]);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeProvidersSetOrderMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        modeId: 3,
        modeUuid: MODE_UUID,
        cliKey: " codex " as never,
        orderedProviderIds: [101],
      });
    });

    expect(sortModeProvidersSetOrder).toHaveBeenCalledWith({
      mode_id: 3,
      cli_key: "codex",
      ordered_provider_ids: [101],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: sortModeProvidersQueryKey(3, MODE_UUID, "codex"),
    });
  });

  it("useSortModeProviderSetEnabledMutation invalidates the provider list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProviderSetEnabled).mockResolvedValue(
      makeSortModeProviderRow({ enabled: false })
    );

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeProviderSetEnabledMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        modeId: 4,
        modeUuid: MODE_UUID,
        cliKey: " gemini " as never,
        providerId: 101,
        enabled: false,
      });
    });

    expect(sortModeProviderSetEnabled).toHaveBeenCalledWith({
      mode_id: 4,
      cli_key: "gemini",
      provider_id: 101,
      enabled: false,
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: sortModeProvidersQueryKey(4, MODE_UUID, "gemini"),
    });
  });

  it("useSortModeProviderSetSessionReusePriorityMutation invalidates the provider list on settle", async () => {
    setTauriRuntime();

    vi.mocked(sortModeProviderSetSessionReusePriority).mockResolvedValue(
      makeSortModeProviderRow({ session_reuse_priority: 75 })
    );

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSortModeProviderSetSessionReusePriorityMutation(), {
      wrapper,
    });
    await act(async () => {
      await result.current.mutateAsync({
        modeId: 4,
        modeUuid: MODE_UUID,
        cliKey: " gemini " as never,
        providerId: 101,
        sessionReusePriority: 75,
      });
    });

    expect(sortModeProviderSetSessionReusePriority).toHaveBeenCalledWith({
      mode_id: 4,
      cli_key: "gemini",
      provider_id: 101,
      session_reuse_priority: 75,
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: sortModeProvidersQueryKey(4, MODE_UUID, "gemini"),
    });
  });
});
