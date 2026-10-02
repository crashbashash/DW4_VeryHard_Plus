import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import { PresetCards } from "./PresetCards";

describe("PresetCards", () => {
  test("shows the three named presets, no Custom while a named one is active", () => {
    render(<PresetCards preset="VeryHardPlus" onPick={() => {}} />);
    expect(screen.getByText("Very Hard Plus")).toBeTruthy();
    expect(screen.getByText("Very Hard Plus — Extreme")).toBeTruthy();
    expect(screen.getByText("Very Hard Plus — Brutal")).toBeTruthy();
    expect(screen.queryByText("Custom")).toBeNull();
  });

  test("Custom appears as a card only when active", () => {
    render(<PresetCards preset="Custom" onPick={() => {}} />);
    expect(screen.getByText("Custom")).toBeTruthy();
  });

  test("clicking a card picks that preset", async () => {
    const onPick = vi.fn();
    render(<PresetCards preset="VeryHardPlus" onPick={onPick} />);
    await userEvent.click(screen.getByText("Very Hard Plus — Brutal"));
    expect(onPick).toHaveBeenCalledWith("Brutal");
  });

  test("the active card is marked", () => {
    render(<PresetCards preset="Extreme" onPick={() => {}} />);
    const card = screen.getByText("Very Hard Plus — Extreme").closest("button")!;
    expect(card.className).toContain("preset-active");
  });
});
