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
  rules: [],
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
      expect(screen.getByText("模型路由策略")).toBeInTheDocument();
      expect(screen.getByText("规则只决定模型资格和重定向，不改变供应商排序")).toBeInTheDocument();
    }
  );

  it("edits, searches, adds, and deletes rules with focus recovery", () => {
    const onChange = renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      rules: [{ source: "gpt-5.4", target: null }],
    });

    fireEvent.change(screen.getByLabelText("搜索规则"), { target: { value: "gpt-5" } });
    expect(screen.getByDisplayValue("gpt-5.4")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "添加规则" }));
    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "selected",
      rules: [
        { source: "gpt-5.4", target: null },
        { source: "", target: null },
      ],
    });

    fireEvent.click(screen.getByRole("button", { name: "删除规则 1" }));
    expect(screen.getByRole("button", { name: "添加规则" })).toHaveFocus();
  });

  it("keeps all mode when adding a rule", () => {
    const onChange = renderSection("codex", "ready", allPolicy);

    fireEvent.click(screen.getByRole("button", { name: "添加规则" }));

    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "all",
      rules: [{ source: "", target: null }],
    });
  });

  it("counts Unicode scalar values for the 200-character boundary", () => {
    const onChange = renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      rules: [{ source: "😀".repeat(200), target: null }],
    });

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("shows legacy opt-in and invalid reset consequences", () => {
    const legacyChange = renderSection("claude", "legacy", null);
    expect(screen.getByText("当前 Claude 仍使用旧版模型映射")).toBeInTheDocument();
    expect(screen.getByText("未配置，沿用请求模型。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "改用通用模型策略" }));
    expect(legacyChange).toHaveBeenCalledWith(allPolicy);
    expect(screen.getByText("保存后无法在界面切回旧策略")).toBeInTheDocument();

    cleanup();
    renderSection("codex", "invalid", null);
    expect(screen.getByRole("alert")).toHaveTextContent("模型策略无效");
    fireEvent.click(screen.getByRole("button", { name: "重置为全部模型（保存后恢复全量路由）" }));
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
    expect(screen.queryByLabelText("源模型 1")).not.toBeInTheDocument();
  });

  it("keeps rule editing enabled while discovery is loading", () => {
    const onDiscoverModels = vi.fn();
    render(
      <ProviderModelPolicySection
        cliKey="codex"
        status="ready"
        policy={{
          version: 1,
          mode: "all",
          rules: [{ source: "gpt-5.4", target: null }],
        }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "loading" }}
        onDiscoverModels={onDiscoverModels}
        hasMultipleBaseUrls={false}
      />
    );

    const button = screen.getByRole("button", { name: "获取上游模型" });
    expect(button).toBeDisabled();
    expect(screen.getByRole("button", { name: "添加规则" })).toBeEnabled();
    expect(screen.getByLabelText("源模型 1")).toBeEnabled();
    expect(screen.getByText("正在读取当前端点的模型目录…")).toBeInTheDocument();
    expect(onDiscoverModels).not.toHaveBeenCalled();
  });

  it("explains unsupported connections without exposing upstream details", () => {
    renderSection("claude", "ready", allPolicy, {
      status: "unsupported",
      reason: "cx_2cc",
    });

    expect(
      screen.getByText("CX2CC 请到对应 Codex Provider 获取，或手工维护规则。")
    ).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
    expect(screen.queryByText(/Location|response body|http/i)).not.toBeInTheDocument();
  });

  it("maps discovery errors to actionable Chinese copy", () => {
    renderSection("grok", "ready", allPolicy, {
      status: "error",
      code: "redirect",
    });

    expect(screen.getByText("端点发生重定向，请直接配置最终 endpoint。")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
  });

  it("uses authentication copy that covers API key and OAuth", () => {
    renderSection("grok", "ready", allPolicy, {
      status: "error",
      code: "unauthorized",
    });

    expect(screen.getByText("认证失败，请检查 API Key 或 OAuth 登录状态。")).toBeInTheDocument();
  });

  it("explains the split-provider boundary for multiple configured addresses", () => {
    renderSection("codex", "ready", allPolicy, { status: "idle" }, true);

    expect(
      screen.getByText("多个地址的模型能力若不同，请拆分为独立 Provider。")
    ).toBeInTheDocument();
  });

  it("describes all and selected policy modes precisely", () => {
    renderSection("codex");
    expect(screen.getByText("全部模型：新增规则不会限制未列出的模型。")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();

    cleanup();
    renderSection("codex", "ready", { version: 1, mode: "selected", rules: [] });
    expect(screen.getByText("仅选定模型：规则决定模型资格。")).toBeInTheDocument();
  });

  it("shows the sanitized origin and selected base URL index", () => {
    renderSection("codex", "ready", allPolicy, {
      status: "ready",
      discoveredCount: 3,
      addedCount: 2,
      origin: "https://example.com:8443",
      baseUrlIndex: 2,
    });

    expect(screen.getByText("https://example.com:8443")).toBeInTheDocument();
    expect(screen.getByText("地址 2")).toBeInTheDocument();
    expect(screen.getByText("仅代表当前端点")).toBeInTheDocument();
  });
});
