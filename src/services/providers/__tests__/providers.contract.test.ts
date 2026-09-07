import { describe, expect, it } from "vitest";
import providersSource from "../providers.ts?raw";
import type { ProviderAvailabilityResult } from "../providers";

describe("services/providers/providers contract", () => {
  it("keeps generated probe model evidence optional for older servers", () => {
    const oldResult: ProviderAvailabilityResult = {
      ok: true, provider_id: 1, provider_name: "test", base_url: "http://example.test",
      status: 200, latency_ms: 1, error: null, response_preview: null,
    };
    const mapped: ProviderAvailabilityResult = { ...oldResult, requested_model: "input", tested_model: "actual" };
    expect(oldResult.tested_model).toBeUndefined();
    expect(mapped.requested_model).toBe("input");
    expect(mapped.tested_model).toBe("actual");
  });
  it("derives provider ipc types from generated bindings instead of handwritten mirrors", () => {
    expect(providersSource).toContain("type ClaudeModels as GeneratedClaudeModels");
    expect(providersSource).toContain("type DailyResetMode as GeneratedDailyResetMode");
    expect(providersSource).toContain("type ProviderAuthMode as GeneratedProviderAuthMode");
    expect(providersSource).toContain("type ProviderBaseUrlMode as GeneratedProviderBaseUrlMode");
    expect(providersSource).toContain(
      "type ProviderOAuthDeviceCodeStartResult as GeneratedProviderOAuthDeviceCodeStartResult"
    );
    expect(providersSource).toContain(
      "type ProviderOAuthDeviceCodePollResult as GeneratedProviderOAuthDeviceCodePollResult"
    );
    expect(providersSource).toContain("type ProviderSummary as GeneratedProviderSummary");
    expect(providersSource).toContain("type ProviderUpsertInput as GeneratedProviderUpsertInput");
    expect(providersSource).toContain("type RemapGeneratedKeys");
    expect(providersSource).toContain("type ProviderUpsertFieldMap = {");
    expect(providersSource).not.toContain("export type ProviderSummary = {");
    expect(providersSource).not.toContain("export type ProviderUpsertInput = {");
  });
});
