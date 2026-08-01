import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Tooltip } from "../Tooltip";

describe("ui/Tooltip", () => {
  it("renders children without tooltip content initially", () => {
    render(
      <Tooltip content="Tip text">
        <span>Hover me</span>
      </Tooltip>
    );
    expect(screen.getByText("Hover me")).toBeInTheDocument();
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("shows tooltip on mouseEnter and hides on mouseLeave", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Hello tooltip">
        <span>Anchor</span>
      </Tooltip>
    );

    const anchor = screen.getByText("Anchor");
    await user.hover(anchor);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Hello tooltip");

    await user.unhover(anchor);
    await waitFor(() => expect(screen.queryByRole("tooltip")).not.toBeInTheDocument());
  });

  it("renders with placement=top by default", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Top tip">
        <span>Anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("Anchor"));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("Top tip");
    const container = tooltip.closest("[data-side]");
    expect(container).not.toBeNull();
    expect(container).toHaveAttribute("data-side", "top");
  });

  it("renders with placement=bottom", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Bottom tip" placement="bottom">
        <span>Anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("Anchor"));
    const tooltip = await screen.findByRole("tooltip");
    expect(tooltip).toHaveTextContent("Bottom tip");
    const container = tooltip.closest("[data-side]");
    expect(container).not.toBeNull();
    expect(container).toHaveAttribute("data-side", "bottom");
  });

  it("merges custom className on the anchor wrapper", () => {
    const { container } = render(
      <Tooltip content="Tip" className="anchor-class">
        <span>Anchor</span>
      </Tooltip>
    );
    expect(container.querySelector(".anchor-class")).toBeInTheDocument();
  });

  it("merges contentClassName on the tooltip content", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Styled tip" contentClassName="tip-style">
        <span>Anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("Anchor"));
    await waitFor(() =>
      expect(screen.getByRole("tooltip").closest(".tip-style")).toBeInTheDocument()
    );
  });

  it("keeps the inverse surface by default", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Inverse tip">
        <span>Inverse anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("Inverse anchor"));
    const content = (await screen.findByRole("tooltip")).closest("[data-side]");
    expect(content).toHaveClass("bg-foreground", "text-background");
    expect(content).not.toHaveClass("bg-popover");
  });

  it("supports a semantic panel surface for rich content", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Panel tip" surface="panel" interactive collisionPadding={12}>
        <span>Panel anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("Panel anchor"));
    const content = (await screen.findByRole("tooltip")).closest("[data-side]");
    expect(content).toHaveClass("border-border", "bg-popover", "text-popover-foreground");
    expect(content).not.toHaveClass("bg-foreground");
  });

  it("wraps plain text children so the tooltip can still open", async () => {
    const user = userEvent.setup();
    render(<Tooltip content="Plain tip">Plain anchor</Tooltip>);

    const anchor = screen.getByText("Plain anchor");
    expect(anchor).toHaveClass("inline-flex");

    await user.hover(anchor);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Plain tip");
  });
});
