import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { FormField } from "../FormField";

describe("ui/FormField", () => {
  it("associates a control with its visible label and hint", async () => {
    const user = userEvent.setup();
    render(
      <FormField label="Email" hint="We won't share it">
        {(id, hintId) => <input id={id} aria-describedby={hintId} />}
      </FormField>
    );

    const input = screen.getByRole("textbox", { name: "Email" });
    const label = screen.getByText("Email");
    const hint = screen.getByText("We won't share it");
    expect(input).toHaveAttribute("id");
    expect(label.closest("label")).toHaveAttribute("for", input.id);
    expect(hint).toHaveAttribute("id");
    expect(input).toHaveAttribute("aria-describedby", hint.id);
    expect(input).toHaveAccessibleDescription("We won't share it");

    await user.click(label);
    expect(input).toHaveFocus();
  });

  it("uses an explicit htmlFor for render-prop controls", () => {
    render(
      <FormField label="Explicit" htmlFor="my-id">
        {(id) => <input id={id} data-testid="explicit-input" />}
      </FormField>
    );

    const input = screen.getByTestId("explicit-input");
    expect(input.id).toBe("my-id");
    expect(screen.getByText("Explicit").closest("label")).toHaveAttribute("for", "my-id");
  });

  it("omits hint when not provided", () => {
    const { container } = render(<FormField label="Name">{(id) => <input id={id} />}</FormField>);
    // Only the label text and the input should be present
    const hintCandidates = container.querySelectorAll(".text-xs");
    expect(hintCandidates).toHaveLength(0);
  });

  it("renders ReactNode as hint", () => {
    render(
      <FormField label="Field" hint={<span data-testid="custom-hint">Custom</span>}>
        {(id, hintId) => <input id={id} aria-describedby={hintId} />}
      </FormField>
    );
    expect(screen.getByTestId("custom-hint")).toBeInTheDocument();
    expect(screen.getByText("Custom")).toBeInTheDocument();
  });

  it("merges custom className", () => {
    const { container } = render(
      <FormField label="Styled" className="my-field">
        {(id) => <input id={id} />}
      </FormField>
    );
    expect(container.firstElementChild).toHaveClass("my-field");
  });

  it("labels composite children as a group", () => {
    render(
      <FormField label="Multi" hint="Choose both values" group>
        <input aria-label="first" />
        <input aria-label="second" />
      </FormField>
    );
    const group = screen.getByRole("group", { name: "Multi" });
    expect(group).toHaveAccessibleDescription("Choose both values");
    expect(screen.getByText("Multi").closest("label")).toBeNull();
    expect(screen.getByLabelText("first")).toBeInTheDocument();
    expect(screen.getByLabelText("second")).toBeInTheDocument();
  });
});
