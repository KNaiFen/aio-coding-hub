import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  CliKey,
  ProviderModelPolicyV1,
  ProviderModelPolicyStatus,
} from "../../../services/providers/providers";
import { ProviderModelPolicySection } from "../ProviderModelPolicySection";
import type { ProviderModelDiscoveryUiState } from "../providerModelPolicy";

const allPolicy: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  modelPatterns: [],
  mappings: [],
};

function renderSection(
  cliKey: CliKey,
  status: ProviderModelPolicyStatus = "ready",
  policy: ProviderModelPolicyV1 | null = allPolicy,
  modelDiscoveryState: ProviderModelDiscoveryUiState = { status: "idle" },
  hasMultipleBaseUrls = false
) {
  const onChange = vi.fn();
  render(
    <ProviderModelPolicySection
      cliKey={cliKey}
      status={status}
      policy={policy}
      saving={false}
      onChange={onChange}
      modelDiscoveryState={modelDiscoveryState}
      onDiscoverModels={vi.fn()}
      hasMultipleBaseUrls={hasMultipleBaseUrls}
    />
  );
  return onChange;
}

describe("pages/providers/ProviderModelPolicySection", () => {
  it.each<CliKey>(["claude", "codex", "gemini", "grok"])(
    "renders the shared model section for %s",
    (cliKey) => {
      renderSection(cliKey);
      expect(screen.getByText("模型路由")).toBeInTheDocument();
      expect(screen.getByText("模型范围")).toBeInTheDocument();
      expect(screen.getByText("模型映射（可选）")).toBeInTheDocument();
      expect(
        screen.queryByText("规则只决定模型资格和重定向，不改变供应商排序")
      ).not.toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "全部可用" })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "仅这些可用" })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "排除这些" })).toBeInTheDocument();
    }
  );

  it("edits range models, searches, adds, and deletes with focus recovery", () => {
    const onChange = renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: ["gpt-5.4"],
      mappings: [],
    });

    fireEvent.change(screen.getByLabelText("搜索模型"), { target: { value: "gpt-5" } });
    expect(screen.getByDisplayValue("gpt-5.4")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "添加可用模型" }));
    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "selected",
      modelPatterns: ["gpt-5.4", ""],
      mappings: [],
    });

    fireEvent.click(screen.getByRole("button", { name: "删除可用模型 1" }));
    expect(screen.getByRole("button", { name: "添加可用模型" })).toHaveFocus();
  });

  it("keeps mapping source and target in a separate required-target section", () => {
    const onChange = renderSection("codex");
    fireEvent.click(screen.getByRole("button", { name: "添加映射" }));
    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "all",
      modelPatterns: [],
      mappings: [{ source: "", target: "" }],
    });
  });

  it("counts Unicode scalar values for the 200-character boundary", () => {
    renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: ["😀".repeat(200)],
      mappings: [],
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows legacy opt-in and invalid reset consequences", () => {
    const legacyChange = renderSection("claude", "legacy", null);
    expect(screen.getByText("当前 Claude 使用旧版模型映射")).toBeInTheDocument();
    expect(screen.getByText("未配置，沿用请求模型。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "改用通用模型策略" }));
    expect(legacyChange).toHaveBeenCalledWith(allPolicy);
    expect(screen.getByText("保存后无法在界面切回旧策略")).toBeInTheDocument();

    cleanup();
    renderSection("codex", "invalid", null);
    expect(screen.getByRole("alert")).toHaveTextContent("模型策略无效");
    fireEvent.click(screen.getByRole("button", { name: "重置为全部可用" }));
  });

  it("shows configured legacy Claude mappings without editing controls", () => {
    render(
      <ProviderModelPolicySection
        cliKey="claude"
        status="legacy"
        policy={null}
        legacyClaudeModels={{ main_model: "legacy-main", reasoning_model: "legacy-thinking" }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "idle" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
      />
    );

    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-main");
    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-thinking");
    expect(screen.queryByLabelText("请求模型 1")).not.toBeInTheDocument();
  });

  it("keeps range editing enabled while discovery is loading", () => {
    render(
      <ProviderModelPolicySection
        cliKey="codex"
        status="ready"
        policy={{
          version: 1,
          mode: "all",
          modelPatterns: ["gpt-5.4"],
          mappings: [],
        }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "loading" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
      />
    );

    expect(screen.getByRole("button", { name: "获取上游模型" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "添加优先模型" })).toBeEnabled();
    expect(screen.getByLabelText("优先模型 1")).toBeEnabled();
    expect(screen.getByText("正在获取上游模型…")).toBeInTheDocument();
  });

  it("keeps unsupported and error states actionable without endpoint claims", () => {
    renderSection("claude", "ready", allPolicy, {
      status: "unsupported",
      reason: "cx_2cc",
    });
    expect(screen.getByText("CX2CC 请在对应 Codex Provider 获取")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();

    cleanup();
    renderSection("grok", "ready", allPolicy, { status: "error", code: "redirect" });
    expect(screen.getByText("端点发生重定向，请配置最终 endpoint")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
  });

  it("describes all, selected, and excluded modes plainly", () => {
    renderSection("codex");
    expect(
      screen.getByText("未列出的模型也可用；列出的模型会优先路由到此 Provider。")
    ).toBeInTheDocument();

    cleanup();
    renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: [],
      mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
    });
    expect(screen.getByText("只接收下列模型和映射中的请求模型。")).toBeInTheDocument();

    cleanup();
    renderSection("codex", "ready", {
      version: 1,
      mode: "excluded",
      modelPatterns: ["legacy-model"],
      mappings: [],
    });
    expect(screen.getByText("下列模型不可用；其余模型保持可用。")).toBeInTheDocument();
  });

  it("does not claim a route boundary twice in discovery status", () => {
    renderSection("codex", "ready", allPolicy, {
      status: "ready",
      discoveredCount: 3,
      addedCount: 2,
      origin: "https://example.com:8443",
      baseUrlIndex: 2,
    });

    expect(screen.getByText(/已获取 3 个 · 新增 2/)).toBeInTheDocument();
    expect(screen.getByText(/https:\/\/example\.com:8443 · 地址 2/)).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
  });
});
