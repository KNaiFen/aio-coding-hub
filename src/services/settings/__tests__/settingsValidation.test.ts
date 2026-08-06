import { describe, expect, it } from "vitest";
import {
  cloneUpstreamRetryPolicy,
  DEFAULT_UPSTREAM_RETRY_POLICY,
} from "../../gateway/upstreamRetryPolicy";
import { validateSettingsSetInput } from "../settingsValidation";

describe("services/settings/settingsValidation", () => {
  it("accepts backend-aligned numeric boundary values", () => {
    expect(
      validateSettingsSetInput({
        preferredPort: 1024,
        logRetentionDays: 3650,
        requestLogRetentionDays: 1,
        providerCooldownSeconds: 0,
        providerAvailabilityHours: 6,
        providerBaseUrlPingCacheTtlSeconds: 1,
        upstreamFirstByteTimeoutSeconds: 3600,
        upstreamStreamIdleTimeoutSeconds: 0,
        upstreamRequestTimeoutNonStreamingSeconds: 86400,
        failoverMaxAttemptsPerProvider: 20,
        failoverMaxProvidersToTry: 5,
        circuitBreakerFailureThreshold: 50,
        circuitBreakerOpenDurationMinutes: 1440,
      })
    ).toBeNull();

    expect(validateSettingsSetInput({ upstreamStreamIdleTimeoutSeconds: 60 })).toBeNull();
  });

  it("accepts only the supported provider availability ranges", () => {
    for (const hours of [3, 6, 12]) {
      expect(validateSettingsSetInput({ providerAvailabilityHours: hours })).toBeNull();
    }
    expect(validateSettingsSetInput({ providerAvailabilityHours: 4 })).toContain("3、6 或 12");
  });

  it("rejects numeric settings outside backend bounds before IPC", () => {
    expect(validateSettingsSetInput({ preferredPort: 1023 })).toContain("首选端口必须 >= 1024");
    expect(validateSettingsSetInput({ logRetentionDays: 3651 })).toContain(
      "日志保留天数必须 <= 3650"
    );
    expect(validateSettingsSetInput({ requestLogRetentionDays: 0 })).toContain(
      "请求记录保留天数必须 >= 1"
    );
    expect(validateSettingsSetInput({ providerCooldownSeconds: 3601 })).toContain(
      "Provider 冷却时间必须 <= 3600"
    );
    expect(validateSettingsSetInput({ providerBaseUrlPingCacheTtlSeconds: 0 })).toContain(
      "Provider Base URL 探测缓存 TTL必须 >= 1"
    );
    expect(validateSettingsSetInput({ upstreamFirstByteTimeoutSeconds: 3601 })).toContain(
      "首字节超时必须 <= 3600"
    );
    expect(
      validateSettingsSetInput({ upstreamRequestTimeoutNonStreamingSeconds: 86401 })
    ).toContain("非流式请求超时必须 <= 86400");
    expect(validateSettingsSetInput({ circuitBreakerFailureThreshold: 0 })).toContain(
      "熔断失败阈值必须 >= 1"
    );
    expect(validateSettingsSetInput({ circuitBreakerOpenDurationMinutes: 1441 })).toContain(
      "熔断打开时长必须 <= 1440"
    );
  });

  it("rejects fractional values and stream idle timeout values in the forbidden gap", () => {
    expect(validateSettingsSetInput({ preferredPort: 37123.5 })).toContain("首选端口必须是整数");
    expect(validateSettingsSetInput({ upstreamStreamIdleTimeoutSeconds: 30 })).toContain(
      "流式空闲超时必须为 0"
    );
    expect(validateSettingsSetInput({ upstreamStreamIdleTimeoutSeconds: 3601 })).toContain(
      "流式空闲超时必须 <= 3600"
    );
  });

  it("rejects failover product overflow when both dimensions are present", () => {
    expect(
      validateSettingsSetInput({
        failoverMaxAttemptsPerProvider: 20,
        failoverMaxProvidersToTry: 6,
      })
    ).toContain("Failover 总尝试次数必须 <= 100");
  });

  it("validates nested retry rules before settings IPC", () => {
    const policy = cloneUpstreamRetryPolicy(DEFAULT_UPSTREAM_RETRY_POLICY);
    policy.http_rules[0] = {
      enabled: true,
      status_code: 599,
      body_contains: ["Quota", "literal *.json"],
      description: "Temporary quota",
    };
    expect(validateSettingsSetInput({ upstreamRetryPolicy: policy })).toBeNull();

    policy.http_rules[0].status_code = 600;
    expect(validateSettingsSetInput({ upstreamRetryPolicy: policy })).toContain("400-599");
    policy.http_rules[0].status_code = 503;
    policy.http_rules[0].description = "unsafe\nline";
    expect(validateSettingsSetInput({ upstreamRetryPolicy: policy })).toContain("控制字符");
  });

  it("validates upstream error response rules before settings IPC", () => {
    const rule = {
      id: "8ca12e7b-4f19-45f7-9185-cc6fbd951c51",
      name: "限额响应",
      description: "",
      enabled: true,
      priority: 10,
      status_codes: [429],
      keywords: ["quota"],
      match_mode: "all" as const,
      cli_keys: ["codex"],
      provider_ids: [7],
      status_behavior: { mode: "override" as const, status_code: 503 },
      message_behavior: { mode: "passthrough" as const },
    };
    expect(validateSettingsSetInput({ upstreamErrorResponseRules: [rule] })).toBeNull();
    expect(
      validateSettingsSetInput({
        upstreamErrorResponseRules: [{ ...rule, keywords: [], status_codes: [] }],
      })
    ).toContain("至少需要");
  });
});
