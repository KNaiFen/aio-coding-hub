import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ModelPriceRulesDialog } from "../ModelPriceRulesDialog";
import type { ModelPriceReference, ModelPriceRule } from "../../../services/usage/modelPrices";

const state = vi.hoisted(() => ({
  query: { data: { version: 1, rules: [] as ModelPriceRule[] }, isError: false, isFetching: false, error: null as Error | null, refetch: vi.fn() },
  mutation: { isPending: false, mutateAsync: vi.fn() },
  reference: { data: null as ModelPriceReference | null, isError: false, isFetching: false, isSuccess: true },
}));
vi.mock("../../../query/modelPrices", () => ({
  useModelPriceRulesQuery: () => state.query,
  useModelPriceRulesSetMutation: () => state.mutation,
  useModelPricesListQuery: () => ({ data: [], isError: false }),
  useModelPriceReferenceQuery: () => state.reference,
}));

beforeEach(() => {
  state.query = { data: { version: 1, rules: [] }, isError: false, isFetching: false, error: null, refetch: vi.fn() };
  state.mutation.isPending = false;
  state.mutation.mutateAsync.mockReset().mockImplementation(async (value) => value);
  state.reference = { data: null, isError: false, isFetching: false, isSuccess: true };
});

describe("ModelPriceRulesDialog", () => {
  it.each(["0", "1"])("preserves conflicting %s inputs until one side is cleared", async (value) => {
    render(<ModelPriceRulesDialog open onOpenChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "新增规则" }));
    fireEvent.change(screen.getByLabelText("完整模型名称"), { target: { value: "new-model" } });
    fireEvent.change(screen.getByLabelText("整体倍率"), { target: { value } });
    fireEvent.change(screen.getAllByLabelText("分项倍率")[1], { target: { value } });
    expect(screen.getByRole("alert")).toHaveTextContent("整体倍率与分项倍率不能同时设置");
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    expect(screen.getByLabelText("整体倍率")).toHaveValue(value);
    expect(screen.getAllByLabelText("分项倍率")[1]).toHaveValue(value);
    fireEvent.change(screen.getByLabelText("整体倍率"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(state.mutation.mutateAsync).toHaveBeenCalled());
    expect(state.mutation.mutateAsync.mock.calls[0][0].rules[0].output.multiplier).toBe(Number(value));
  });

  it("keeps the draft through refetch and save failures and resets it after cancel", async () => {
    const onOpenChange = vi.fn();
    const { rerender } = render(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.click(screen.getByRole("button", { name: "新增规则" }));
    fireEvent.change(screen.getByLabelText("完整模型名称"), { target: { value: "draft-model" } });
    fireEvent.change(screen.getAllByLabelText("自定义单价")[0], { target: { value: "7" } });
    state.query.data = { version: 1, rules: [] };
    state.reference.isFetching = true;
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    expect(screen.getByLabelText("完整模型名称")).toHaveValue("draft-model");
    state.reference.isFetching = false;
    state.reference.data = {
      reference_model: "new-target",
      input: { standard: 2, priority: null, above_200k: null, priority_above_200k: null },
      output: { standard: 10, priority: null, above_200k: null, priority_above_200k: null },
      cache_read: { standard: 0.2, priority: null, above_200k: null, priority_above_200k: null },
      cache_write_5m: { standard: 2.5, priority: null, above_200k: null, priority_above_200k: null },
      cache_write_1h: { standard: 4, priority: null, above_200k: null, priority_above_200k: null },
    };
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    expect(screen.getByText("参考模型：new-target")).toBeInTheDocument();
    expect(screen.getByLabelText("完整模型名称")).toHaveValue("draft-model");
    expect(screen.getAllByLabelText("自定义单价")[0]).toHaveValue("7");
    expect(screen.getByText("生效：7")).toBeInTheDocument();
    state.mutation.mutateAsync.mockRejectedValueOnce(new Error("disk full"));
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await screen.findByText("disk full");
    expect(screen.getByLabelText("完整模型名称")).toHaveValue("draft-model");
    expect(onOpenChange).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
    rerender(<ModelPriceRulesDialog open={false} onOpenChange={onOpenChange} />);
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    expect(screen.queryByDisplayValue("draft-model")).not.toBeInTheDocument();
  });

  it("blocks saving after a rule read error even when stale data is available", () => {
    state.query.isError = true;
    state.query.error = new Error("read failed");
    render(<ModelPriceRulesDialog open onOpenChange={vi.fn()} />);
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "新增规则" })).toBeDisabled();
    expect(screen.getByText("自定义定价加载失败")).toBeInTheDocument();
  });

  it("keeps the dialog open and disables commands while saving", () => {
    state.mutation.isPending = true;
    const onOpenChange = vi.fn();
    render(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "保存中..." })).toBeDisabled();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
  });

  it("previews the enabled alias target only when its reference is available", async () => {
    const empty = { price: null, multiplier: null };
    state.query.data = { version: 1, rules: [
      { cli_key: "claude", model: "source", enabled: true, multiplier: 2,
        input: empty, output: empty, cache_read: empty, cache_write_5m: empty, cache_write_1h: empty },
      { cli_key: "claude", model: "target", enabled: true, multiplier: 3,
        input: empty, output: empty, cache_read: empty, cache_write_5m: empty, cache_write_1h: empty },
    ] };
    state.reference.data = {
      reference_model: "target",
      input: { standard: 2, priority: 4, above_200k: 6, priority_above_200k: 8 },
      output: { standard: 10, priority: null, above_200k: null, priority_above_200k: null },
      cache_read: { standard: 0.2, priority: null, above_200k: null, priority_above_200k: null },
      cache_write_5m: { standard: 2.5, priority: null, above_200k: null, priority_above_200k: null },
      cache_write_1h: { standard: 4, priority: null, above_200k: null, priority_above_200k: null },
    };
    const targetReference = state.reference.data;
    const onOpenChange = vi.fn();
    const { rerender } = render(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.click(screen.getByRole("button", { name: "编辑 source" }));
    expect(screen.getByText("生效：标准 4 / 优先 8 / 超过 200k 12 / 优先超过 200k 16")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch", { name: "启用当前规则" }));
    expect(screen.getByText("生效：标准 6 / 优先 12 / 超过 200k 18 / 优先超过 200k 24")).toBeInTheDocument();
    expect(screen.getByLabelText("整体倍率")).toHaveValue("2");
    state.reference.data = null;
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    expect(screen.getAllByText("生效：未定价")).toHaveLength(5);
    state.reference.data = { ...targetReference, reference_model: "source" };
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    expect(screen.getByText("生效：标准 2 / 优先 4 / 超过 200k 6 / 优先超过 200k 8")).toBeInTheDocument();
    state.reference.data = targetReference;
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(state.mutation.mutateAsync).toHaveBeenCalled());
    expect(state.mutation.mutateAsync.mock.calls[0][0].rules[0]).toMatchObject({ enabled: false, multiplier: 2 });
    expect(state.mutation.mutateAsync.mock.calls[0][0].rules[1]).toMatchObject({ enabled: true, multiplier: 3 });
  });

  it("uses unsaved target changes in the candidate preview", () => {
    const empty = { price: null, multiplier: null };
    state.query.data = { version: 1, rules: [
      { cli_key: "claude", model: "source", enabled: true, multiplier: null,
        input: empty, output: empty, cache_read: empty, cache_write_5m: empty, cache_write_1h: empty },
      { cli_key: "claude", model: "target", enabled: true, multiplier: 3,
        input: empty, output: empty, cache_read: empty, cache_write_5m: empty, cache_write_1h: empty },
    ] };
    const price = { standard: 2, priority: null, above_200k: null, priority_above_200k: null };
    state.reference.data = {
      reference_model: "target", input: price, output: price, cache_read: price, cache_write_5m: price, cache_write_1h: price,
    };
    render(<ModelPriceRulesDialog open onOpenChange={vi.fn()} />);
    fireEvent.click(screen.getByRole("switch", { name: "启用 target" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑 source" }));
    fireEvent.click(screen.getByRole("switch", { name: "启用当前规则" }));
    expect(screen.getAllByText("生效：标准 2")).toHaveLength(5);
  });

  it("reopens saved rules and supports editing, disabling, and deleting", async () => {
    const empty = { price: null, multiplier: null };
    state.query.data = { version: 1, rules: [{ cli_key: "claude", model: "saved", enabled: true, multiplier: 0,
      input: { price: 2, multiplier: null }, output: { price: 10, multiplier: null }, cache_read: empty, cache_write_5m: empty, cache_write_1h: empty }] };
    const onOpenChange = vi.fn();
    const { rerender } = render(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.click(screen.getByRole("button", { name: "编辑 saved" }));
    expect(screen.getByLabelText("整体倍率")).toHaveValue("0");
    expect(screen.getAllByLabelText("自定义单价")[0]).toHaveValue("2");
    fireEvent.change(screen.getAllByLabelText("自定义单价")[1], { target: { value: "8" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
    state.query.data = state.mutation.mutateAsync.mock.calls[0][0];
    rerender(<ModelPriceRulesDialog open={false} onOpenChange={onOpenChange} />);
    rerender(<ModelPriceRulesDialog open onOpenChange={onOpenChange} />);
    fireEvent.click(screen.getByRole("switch", { name: "启用 saved" }));
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(state.mutation.mutateAsync).toHaveBeenCalledTimes(2));
    expect(state.mutation.mutateAsync.mock.calls[1][0].rules[0].enabled).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "删除 saved" }));
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(state.mutation.mutateAsync).toHaveBeenCalledTimes(3));
    expect(state.mutation.mutateAsync.mock.calls[2][0].rules).toEqual([]);
  });
});
