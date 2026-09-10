import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { providerLimitReset, providerLimitUsageV1 } from "../../services/providers/providerLimitUsage";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { ProviderLimitRefreshError, useProviderLimitResetMutation, useProviderLimitUsageV1Query } from "../providerLimitUsage";

vi.mock("../../services/providers/providerLimitUsage", async () => {
  const actual = await vi.importActual<
    typeof import("../../services/providers/providerLimitUsage")
  >("../../services/providers/providerLimitUsage");
  return {
    ...actual,
    providerLimitUsageV1: vi.fn(),
    providerLimitReset: vi.fn(),
  };
});

describe("query/providerLimitUsage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("waits for all mounted usage queries after committing the reset", async () => {
    vi.mocked(providerLimitReset).mockResolvedValue(undefined);
    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const { result } = renderHook(() => ({
      all: useProviderLimitUsageV1Query(null),
      cli: useProviderLimitUsageV1Query("codex"),
      reset: useProviderLimitResetMutation(),
    }), { wrapper });
    await waitFor(() => expect(result.current.all.isSuccess && result.current.cli.isSuccess).toBe(true));
    let finish!: (rows: []) => void;
    const refresh = new Promise<[]>((resolve) => { finish = resolve; });
    vi.mocked(providerLimitUsageV1).mockReturnValue(refresh);
    let pending!: Promise<void>;
    act(() => { pending = result.current.reset.mutateAsync({ providerId: 7, period: "weekly" }); });
    await waitFor(() => expect(providerLimitUsageV1).toHaveBeenCalledTimes(4));
    expect(result.current.reset.isPending).toBe(true);
    expect(providerLimitReset).toHaveBeenCalledWith(7, "weekly");
    await act(async () => { finish([]); await pending; });
    await waitFor(() => expect(result.current.reset.isSuccess).toBe(true));
  });

  it("distinguishes a committed reset from failed refresh and never retries the write", async () => {
    vi.mocked(providerLimitReset).mockResolvedValue(undefined);
    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);
    const wrapper = createQueryWrapper(createTestQueryClient());
    const { result } = renderHook(() => ({
      query: useProviderLimitUsageV1Query("codex"), reset: useProviderLimitResetMutation(),
    }), { wrapper });
    await waitFor(() => expect(result.current.query.isSuccess).toBe(true));
    vi.mocked(providerLimitUsageV1).mockRejectedValue(new Error("read failed"));
    await act(async () => {
      await expect(result.current.reset.mutateAsync({ providerId: 7, period: "monthly" })).rejects.toBeInstanceOf(ProviderLimitRefreshError);
    });
    expect(providerLimitReset).toHaveBeenCalledTimes(1);
    vi.mocked(providerLimitReset).mockRejectedValueOnce(new Error("write failed"));
    await act(async () => {
      await expect(result.current.reset.mutateAsync({ providerId: 7, period: "monthly" })).rejects.toThrow("write failed");
    });
    expect(providerLimitUsageV1).toHaveBeenCalledTimes(2);
  });

  it("replaces an initial read started before the reset instead of accepting its stale result", async () => {
    vi.mocked(providerLimitReset).mockResolvedValue(undefined);
    let finishInitial!: (rows: []) => void;
    vi.mocked(providerLimitUsageV1)
      .mockImplementationOnce(() => new Promise<[]>((resolve) => { finishInitial = resolve; }))
      .mockResolvedValue([]);
    const wrapper = createQueryWrapper(createTestQueryClient());
    const { result } = renderHook(() => ({
      query: useProviderLimitUsageV1Query("codex"), reset: useProviderLimitResetMutation(),
    }), { wrapper });
    await waitFor(() => expect(providerLimitUsageV1).toHaveBeenCalledTimes(1));
    await act(async () => { await result.current.reset.mutateAsync({ providerId: 7, period: "daily" }); });
    expect(providerLimitUsageV1).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(result.current.query.isSuccess).toBe(true));
    await act(async () => { finishInitial([]); });
    expect(result.current.query.isSuccess).toBe(true);
  });

  it("calls providerLimitUsageV1 with tauri runtime", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query(" claude " as never), { wrapper });

    await waitFor(() => {
      expect(providerLimitUsageV1).toHaveBeenCalledWith("claude");
    });
  });

  it("useProviderLimitUsageV1Query enters error state when providerLimitUsageV1 rejects", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockRejectedValue(new Error("provider limit usage query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useProviderLimitUsageV1Query("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query("claude", { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(providerLimitUsageV1).not.toHaveBeenCalled();
  });

  it("supports refetchInterval option without altering query function args", async () => {
    setTauriRuntime();

    vi.mocked(providerLimitUsageV1).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useProviderLimitUsageV1Query(null, { refetchIntervalMs: 5000 }), { wrapper });

    await waitFor(() => {
      expect(providerLimitUsageV1).toHaveBeenCalledWith(null);
    });
  });
});
