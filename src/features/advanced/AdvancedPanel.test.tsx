import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import { presetPlan } from "../presets/presetData";
import type { PlanUiState } from "../presets/usePlan";
import { AdvancedPanel } from "./AdvancedPanel";

function makeState(overrides: Partial<PlanUiState> = {}): PlanUiState {
  return {
    preset: "Custom",
    plan: presetPlan("VeryHardPlus"),
    serialEnabled: false,
    serialText: "SLUS_208.36",
    ...overrides,
  };
}

function setup(state: PlanUiState = makeState()) {
  const editField = vi.fn();
  const setSerialEnabled = vi.fn();
  const setSerialText = vi.fn();
  render(
    <AdvancedPanel
      state={state}
      onEditField={editField}
      onSetSerialEnabled={setSerialEnabled}
      onSetSerialText={setSerialText}
    />,
  );
  return { editField, setSerialEnabled, setSerialText };
}

describe("AdvancedPanel", () => {
  test("renders one numeric field per multiplier: hp, 12 stats, crit, paralysis", () => {
    setup();
    for (const name of ["hp", "atk", "def", "wis", "spr", "spd", "fire", "ice", "thunder", "dark", "stun", "poison", "exp", "crit", "paralysis"]) {
      expect(screen.getByLabelText(name, { selector: "input" })).toBeTruthy();
    }
  });

  test("editing a multiplier calls editField with the parsed value", async () => {
    const { editField } = setup();
    const input = screen.getByLabelText("hp", { selector: "input" });
    await userEvent.clear(input);
    await userEvent.type(input, "1.5");
    expect(editField).toHaveBeenCalledWith("hpMult", 1.5);
  });

  test("crown dropdown offers the six ranks in order with yellow last", async () => {
    const { editField } = setup();
    await userEvent.click(screen.getByLabelText(/crown colour/i));
    const options = screen.getAllByRole("option");
    expect(options.length).toBe(6);
    expect(options[5]!.textContent).toContain("yellow");
    await userEvent.selectOptions(screen.getByLabelText("crown colour"), "0");
    expect(editField).toHaveBeenCalledWith("crownRank", 0);
  });

  test("checkboxes edit their plan fields with the old egui labels", async () => {
    const { editField } = setup();
    await userEvent.click(screen.getByLabelText(/leave training-stage enemies vanilla/i));
    expect(editField).toHaveBeenCalledWith("excludePractice", true);
    await userEvent.click(screen.getByLabelText(/exclude destructibles/i));
    expect(editField).toHaveBeenCalledWith("excludeDestructibles", false);
  });

  test("serial checkbox and text edit the serial controls", async () => {
    const { setSerialText } = setup(makeState({ serialEnabled: true }));
    await userEvent.type(screen.getByLabelText("disc serial"), "X");
    expect(setSerialText).toHaveBeenCalled();
  });
});
