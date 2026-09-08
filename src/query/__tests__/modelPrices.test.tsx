import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ModelPriceAliases,
  ModelPriceReference,
  ModelPriceSummary,
  ModelPricesSyncReport,
} from "../../services/usage/modelPrices";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPriceRulesSet,
  modelPriceReferenceGet,
  modelPricesList,
  modelPricesSyncBasellm,
} from "../../services/usage/modelPrices";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { modelPricesKeys } from "../keys";
import {
  isModelPricesSyncNotModified,
  useModelPriceAliasesQuery,
  useModelPriceAliasesSetMutation,
  useModelPriceRulesSetMutation,
  useModelPriceReferenceQuery,
  useModelPricesListQuery,
  useModelPricesSyncBasellmMutation,
  useModelPricesTotalCountQuery,
} from "../modelPrices";

vi.mock("../../services/usage/modelPrices", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/modelPrices")>(
    "../../services/usage/modelPrices"
  );
  return {
    ...actual,
    modelPricesList: vi.fn(),
    modelPricesSyncBasellm: vi.fn(),
    modelPriceAliasesGet: vi.fn(),
    modelPriceAliasesSet: vi.fn(),
    modelPriceRulesSet: vi.fn(),
    modelPriceReferenceGet: vi.fn(),
  };
});

function makeModelPriceSummary(overrides: Partial<ModelPriceSummary> = {}): ModelPriceSummary {
  return {
    id: 1,
    cli_key: "claude",
    model: "claude-3-7-sonnet",
    currency: "USD",
    created_at: 1,
    updated_at: 2,
    ...overrides,
  };
}

function makeModelPricesSyncReport(
  overrides: Partial<ModelPricesSyncReport> = {}
): ModelPricesSyncReport {
  return {
    status: "updated",
    inserted: 1,
    updated: 0,
    skipped: 0,
    total: 1,
    ...overrides,
  };
}

describe("query/modelPrices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("updates only the saved rule cache without invalidating historical queries", async () => {
    const client = createTestQueryClient();
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const rules = { version: 1, rules: [] };
    vi.mocked(modelPriceRulesSet).mockResolvedValue(rules);
    const { result } = renderHook(() => useModelPriceRulesSetMutation(), { wrapper: createQueryWrapper(client) });
    await act(async () => { await result.current.mutateAsync(rules); });
    expect(client.getQueryData(modelPricesKeys.rules())).toEqual(rules);
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("calls modelPricesList with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPricesListQuery(" claude " as never), { wrapper });

    await waitFor(() => {
      expect(modelPricesList).toHaveBeenCalledWith("claude");
    });
  });

  it("useModelPricesListQuery enters error state when modelPricesList rejects", async () => {
    setTauriRuntime();
    vi.mocked(modelPricesList).mockRejectedValue(new Error("model prices query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesListQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useModelPricesTotalCountQuery sums list lengths across all CLIs", async () => {
    setTauriRuntime();

    vi.mocked(modelPricesList)
      .mockResolvedValueOnce([makeModelPriceSummary({ id: 1 })])
      .mockResolvedValueOnce([makeModelPriceSummary({ id: 2 }), makeModelPriceSummary({ id: 3 })])
      .mockResolvedValueOnce([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesTotalCountQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.data).toBe(3);
    });
  });

  it("useModelPriceAliasesQuery calls modelPriceAliasesGet", async () => {
    setTauriRuntime();

    const aliases: ModelPriceAliases = { version: 1, rules: [] };
    vi.mocked(modelPriceAliasesGet).mockResolvedValue(aliases);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useModelPriceAliasesQuery(), { wrapper });

    await waitFor(() => {
      expect(modelPriceAliasesGet).toHaveBeenCalled();
    });
  });

  it("useModelPriceAliasesSetMutation updates cache and invalidates aliases", async () => {
    setTauriRuntime();

    const legacyInput: ModelPriceAliases = {
      version: 1,
      rules: [
        {
          cli_key: " codex " as never,
          match_type: "prefix",
          pattern: " gpt- ",
          target_model: " gpt-5 ",
          enabled: true,
        },
      ],
    };
    const updated: ModelPriceAliases = {
      version: 2,
      rules: [
        {
          cli_key: "codex",
          match_type: "prefix",
          pattern: "gpt-",
          target_model: "gpt-5",
          enabled: true,
        },
      ],
    };
    vi.mocked(modelPriceAliasesSet).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(modelPricesKeys.aliases(), { version: 1, rules: [] } as ModelPriceAliases);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPriceAliasesSetMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(legacyInput);
    });

    expect(client.getQueryData(modelPricesKeys.aliases())).toEqual(updated);
    expect(modelPriceAliasesSet).toHaveBeenCalledWith({
      version: 2,
      rules: [
        {
          cli_key: "codex",
          match_type: "prefix",
          pattern: "gpt-",
          target_model: "gpt-5",
          enabled: true,
        },
      ],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.aliases() });
  });

  it("reloads a fresh reference cache after saving a new alias target", async () => {
    const price = { standard: 2, priority: null, above_200k: null, priority_above_200k: null };
    const original: ModelPriceReference = {
      reference_model: "target-a", input: price, output: price, cache_read: price, cache_write_5m: price, cache_write_1h: price,
    };
    const updated: ModelPriceReference = {
      ...original, reference_model: "target-b", input: { ...price, standard: 5 },
    };
    vi.mocked(modelPriceReferenceGet).mockResolvedValueOnce(original).mockResolvedValueOnce(updated);
    const aliases: ModelPriceAliases = { version: 2, rules: [{
      cli_key: "claude", match_type: "exact", pattern: "model-a", target_model: "target-b", enabled: true,
    }] };
    vi.mocked(modelPriceAliasesSet).mockResolvedValue(aliases);
    const client = createTestQueryClient();
    client.setDefaultOptions({ queries: { retry: false, staleTime: 5 * 60 * 1000 } });
    const invalidate = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);
    const first = renderHook(() => useModelPriceReferenceQuery("claude", "model-a"), { wrapper });
    await waitFor(() => expect(first.result.current.data).toEqual(original));
    expect(first.result.current.isStale).toBe(false);
    first.unmount();

    const mutation = renderHook(() => useModelPriceAliasesSetMutation(), { wrapper });
    await act(async () => { await mutation.result.current.mutateAsync(aliases); });
    expect(invalidate.mock.calls).toEqual([
      [{ queryKey: modelPricesKeys.references() }],
      [{ queryKey: modelPricesKeys.aliases() }],
    ]);
    expect(client.getQueryState(modelPricesKeys.reference("claude", "model-a"))?.isInvalidated).toBe(true);
    const reopened = renderHook(() => useModelPriceReferenceQuery("claude", "model-a"), { wrapper });
    await waitFor(() => expect(reopened.result.current.data).toEqual(updated));
    expect(modelPriceReferenceGet).toHaveBeenCalledTimes(2);
    expect(modelPriceReferenceGet).toHaveBeenLastCalledWith("claude", "model-a");
  });

  it("useModelPricesSyncBasellmMutation invalidates modelPricesKeys.all", async () => {
    setTauriRuntime();

    const report = makeModelPricesSyncReport();
    vi.mocked(modelPricesSyncBasellm).mockResolvedValue(report);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useModelPricesSyncBasellmMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ force: true });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: modelPricesKeys.all });
  });

  it("isModelPricesSyncNotModified detects not_modified reports", () => {
    expect(isModelPricesSyncNotModified(null)).toBe(false);
    expect(isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "updated" }))).toBe(
      false
    );
    expect(
      isModelPricesSyncNotModified(makeModelPricesSyncReport({ status: "not_modified" }))
    ).toBe(true);
  });
});
