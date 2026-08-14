import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Dialog } from "../Dialog";
import { Popover } from "../Popover";

describe("ui components", () => {
  it("Popover opens and closes (click outside + toggle)", async () => {
    const user = userEvent.setup();
    render(
      <Popover trigger={<span>trigger</span>} placement="bottom" align="center">
        <div>content</div>
      </Popover>
    );

    await user.click(screen.getByRole("button"));
    expect(await screen.findByText("content")).toBeInTheDocument();

    fireEvent.pointerDown(document.body);
    await waitFor(() => expect(screen.queryByText("content")).not.toBeInTheDocument());

    // toggle close
    await user.click(screen.getByRole("button"));
    expect(await screen.findByText("content")).toBeInTheDocument();
    await user.click(screen.getByRole("button"));
    await waitFor(() => expect(screen.queryByText("content")).not.toBeInTheDocument());
  });

  it("Dialog calls onOpenChange from overlay and Escape", async () => {
    const onOpenChange = vi.fn();
    render(
      <Dialog open title="T" description="D" onOpenChange={onOpenChange}>
        <div>content</div>
      </Dialog>
    );

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    expect(onOpenChange).toHaveBeenCalledWith(false);

    onOpenChange.mockClear();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
