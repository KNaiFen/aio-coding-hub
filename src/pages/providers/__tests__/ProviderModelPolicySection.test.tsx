import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  CliKey,
  ProviderModelPolicyV1,
  ProviderModelPolicyStatus,
} from "../../../services/providers/providers";
import { ProviderModelPolicySection } from "../ProviderModelPolicySection";

const allPolicy: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  rules: [],
};

function renderSection(
  cliKey: CliKey,
  status: ProviderModelPolicyStatus = "ready",
  policy: ProviderModelPolicyV1 | null = allPolicy
) {
  const onChange = vi.fn();
  render(
    <ProviderModelPolicySection
      cliKey={cliKey}
      status={status}
      policy={policy}
      saving={false}
      onChange={onChange}
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
      />
    );

    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-main");
    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-thinking");
    expect(screen.queryByLabelText("源模型 1")).not.toBeInTheDocument();
  });
});
