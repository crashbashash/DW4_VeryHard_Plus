import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import { DiscPicker } from "./DiscPicker";

function setup(status: { kind: string; text: string } | null) {
  const pickDisc = vi.fn();
  render(
    <DiscPicker
      inputPath={null}
      outputPath={null}
      status={status as never}
      analyzing={false}
      onPick={pickDisc}
      onSetOutput={() => {}}
    />,
  );
  return pickDisc;
}

describe("DiscPicker", () => {
  test("idle prompt invites the dialog or a drag", () => {
    setup(null);
    expect(screen.getByText(/choose a disc/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: /browse/i })).toBeTruthy();
  });

  test("Browse triggers the pick callback", async () => {
    const pickDisc = setup(null);
    await userEvent.click(screen.getByRole("button", { name: /browse/i }));
    expect(pickDisc).toHaveBeenCalled();
  });

  test.each([
    ["ok", "status-ok"],
    ["refused", "status-refused"],
    ["unknown", "status-unknown"],
  ])("status kind %s gets class %s", (kind, cls) => {
    setup({ kind, text: "verdict text" });
    const line = screen.getByText("verdict text");
    expect(line.className).toContain(cls);
  });
});
