import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderAccountUsageInline } from "../ProviderAccountUsageInline";
import {
  providerAccountUsageFetch,
  type ProviderAccountUsageResult,
  type ProviderSummary,
} from "../../../services/providers/providers";
import { createQueryWrapper, createTestQueryClient } from "../../../test/utils/reactQuery";

vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return { ...actual, providerAccountUsageFetch: vi.fn() };
});

function makeProvider(id: number): ProviderSummary {
  return {
    id,
    provider_uuid: `11111111-1111-4111-8111-${String(id).padStart(12, "0")}`,
    cli_key: "claude",
    name: `Provider ${id}`,
    base_urls: ["https://example.test/v1"],
    base_url_mode: "order",
    claude_models: {},
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
    api_key_configured: true,
    extension_values: [
      {
        pluginId: "core.provider-account-usage",
        namespace: "accountUsage",
        values: { adapterKind: "newapi" },
        updatedAt: 1,
      },
    ],
    newapi_account_user_id: null,
    newapi_account_access_token_configured: false,
    model_mapping: { default_model: null, exact: {} },
    stream_idle_timeout_seconds: null,
    upstream_retry_policy_override: null,
    model_routing_policy_override: null,
  };
}

function usage(balance: number, overrides: Partial<ProviderAccountUsageResult> = {}) {
  return {
    adapter_kind: "newapi" as const,
    status: "available" as const,
    freshness: "fresh" as const,
    plan_name: null,
    balance,
    plan_remaining: null,
    used: 2,
    total: 3,
    unit: "USD",
    unit_note: null,
    daily_used: null,
    daily_total: null,
    weekly_used: null,
    weekly_total: null,
    monthly_used: null,
    monthly_total: null,
    expires_at: null,
    last_fetched_at: 1_700_000_000,
    message: null,
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

async function flushReactQuery() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);
  });
}

function renderUsage(...providers: ProviderSummary[]) {
  const queryClient = createTestQueryClient();
  const rendered = render(
    <>
      {providers.map((provider) => (
        <ProviderAccountUsageInline key={provider.id} provider={provider} />
      ))}
    </>,
    { wrapper: createQueryWrapper(queryClient) }
  );
  return { ...rendered, queryClient };
}

describe("ProviderAccountUsageInline", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(providerAccountUsageFetch).mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps two account rows visually stable through three heartbeat cycles", async () => {
    vi.mocked(providerAccountUsageFetch).mockImplementation((providerId) =>
      Promise.resolve(usage(providerId === 1 ? 1 : 2))
    );

    renderUsage(makeProvider(1), makeProvider(2));
    await flushReactQuery();

    const firstButton = screen.getByRole("button", {
      name: "刷新账户用量，账户: 可用 · 余额 1.00 USD，已用 2.00/3.00 USD",
    });
    const secondButton = screen.getByRole("button", {
      name: "刷新账户用量，账户: 可用 · 余额 2.00 USD，已用 2.00/3.00 USD",
    });
    const initialMarkup = [firstButton.innerHTML, secondButton.innerHTML];

    for (let cycle = 0; cycle < 3; cycle += 1) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(firstButton).toBeEnabled();
      expect(secondButton).toBeEnabled();
      expect(firstButton.innerHTML).toBe(initialMarkup[0]);
      expect(secondButton.innerHTML).toBe(initialMarkup[1]);
      expect(firstButton.querySelector("svg")).not.toHaveClass("animate-spin");
      expect(secondButton.querySelector("svg")).not.toHaveClass("animate-spin");
    }

    expect(providerAccountUsageFetch).toHaveBeenCalledTimes(8);
    expect(providerAccountUsageFetch).toHaveBeenNthCalledWith(1, 1, false);
    expect(providerAccountUsageFetch).toHaveBeenNthCalledWith(2, 2, false);
  });

  it("renders an initial query failure without usage metrics", async () => {
    vi.mocked(providerAccountUsageFetch).mockRejectedValueOnce(
      new Error("initial account usage failed")
    );

    renderUsage(makeProvider(1));
    await flushReactQuery();

    const button = screen.getByRole("button", {
      name: "刷新账户用量，initial account usage failed",
    });
    expect(button).toHaveTextContent("initial account usage failed");
    expect(button).not.toHaveTextContent("已用 2.00/3.00 USD");
    expect(button).toHaveClass("text-amber-700");
  });

  it("updates after a background success and hides stale metrics after a background failure", async () => {
    const backgroundSuccess = deferred<ProviderAccountUsageResult>();
    const backgroundFailure = deferred<ProviderAccountUsageResult>();
    vi.mocked(providerAccountUsageFetch)
      .mockResolvedValueOnce(usage(1))
      .mockImplementationOnce(() => backgroundSuccess.promise)
      .mockImplementationOnce(() => backgroundFailure.promise);

    renderUsage(makeProvider(1));
    await flushReactQuery();

    const button = screen.getByRole("button", {
      name: "刷新账户用量，账户: 可用 · 余额 1.00 USD，已用 2.00/3.00 USD",
    });
    const stableMarkup = button.innerHTML;

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    expect(button).toBeEnabled();
    expect(button.innerHTML).toBe(stableMarkup);
    expect(button.querySelector("svg")).not.toHaveClass("animate-spin");

    backgroundSuccess.resolve(usage(9));
    await flushReactQuery();
    expect(button).toHaveAccessibleName(
      "刷新账户用量，账户: 可用 · 余额 9.00 USD，已用 2.00/3.00 USD"
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    expect(button).toHaveTextContent("账户: 可用 · 余额 9.00 USD");
    expect(button).toHaveTextContent("已用 2.00/3.00 USD");
    expect(button).toBeEnabled();

    backgroundFailure.reject(new Error("background heartbeat failed"));
    await flushReactQuery();
    expect(button).toHaveTextContent("background heartbeat failed");
    expect(button).not.toHaveTextContent("余额 9.00 USD");
    expect(button).not.toHaveTextContent("已用 2.00/3.00 USD");
  });

  it("shows only the initial loading text, then keeps content during manual refresh and rejects repeat clicks", async () => {
    const initial = deferred<ProviderAccountUsageResult>();
    const manualSuccess = deferred<ProviderAccountUsageResult>();
    vi.mocked(providerAccountUsageFetch)
      .mockImplementationOnce(() => initial.promise)
      .mockImplementationOnce(() => manualSuccess.promise);

    renderUsage(makeProvider(1));
    await flushReactQuery();

    const button = screen.getByRole("button", { name: "刷新账户用量，账户: 刷新中" });
    expect(button).toHaveTextContent("账户: 刷新中");
    expect(button).not.toBeDisabled();
    expect(button.querySelector("svg")).toHaveClass("animate-spin");

    initial.resolve(usage(1));
    await flushReactQuery();
    expect(button).toHaveTextContent("账户: 可用 · 余额 1.00 USD");
    expect(button).toHaveTextContent("已用 2.00/3.00 USD");

    fireEvent.click(button);
    fireEvent.click(button);
    await flushReactQuery();
    expect(providerAccountUsageFetch).toHaveBeenCalledTimes(2);
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent("账户: 可用 · 余额 1.00 USD");
    expect(button).toHaveTextContent("已用 2.00/3.00 USD");
    expect(button).toHaveAccessibleName(
      "刷新账户用量，正在刷新，账户: 可用 · 余额 1.00 USD，已用 2.00/3.00 USD"
    );
    expect(button.querySelector("svg")).toHaveClass("animate-spin");

    manualSuccess.resolve(usage(8));
    await flushReactQuery();
    expect(button).toBeEnabled();
    expect(button).toHaveTextContent("账户: 可用 · 余额 8.00 USD");

    vi.mocked(providerAccountUsageFetch).mockRejectedValueOnce(new Error("manual refresh failed"));
    fireEvent.click(button);
    await flushReactQuery();
    expect(button).toBeEnabled();
    expect(button).toHaveTextContent("manual refresh failed");
    expect(button).not.toHaveTextContent("余额 8.00 USD");
    expect(button).not.toHaveTextContent("已用 2.00/3.00 USD");
  });
});
