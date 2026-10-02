import { act, renderHook } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { usePlan } from "./usePlan";

describe("usePlan — the preset/advanced reactivity contract", () => {
  test("defaults to the VeryHardPlus preset and plan", () => {
    const { result } = renderHook(() => usePlan());
    expect(result.current.state.preset).toBe("VeryHardPlus");
    expect(result.current.state.plan.crownRank).toBe(5);
  });

  test("applyPreset loads the whole plan and resets serial controls", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.applyPreset("Extreme"));
    expect(result.current.state.plan.excludePractice).toBe(true);
    expect(result.current.state.plan.forceVeryHard).toBe(true);
  });

  test("editField flips the preset to Custom and keeps other fields", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.applyPreset("Extreme"));
    act(() => result.current.editField("hpMult", 1.5));
    expect(result.current.state.preset).toBe("Custom");
    expect(result.current.state.plan.excludePractice).toBe(true);
  });

  test("applyPreset after edits resets serial to defaults", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.setSerialEnabled(true));
    act(() => result.current.setSerialText("SLUS_208.37"));
    act(() => result.current.applyPreset("Brutal"));
    expect(result.current.state.serialEnabled).toBe(false);
    expect(result.current.state.serialText).toBe("SLUS_208.36");
    expect(result.current.state.plan.serial).toBeNull();
    expect(result.current.state.preset).toBe("Brutal");
  });

  test("editing to a plan that exactly matches another preset reports that preset", () => {
    // Rust Preset::detect semantics (spec §3.3): Brutal minus collapse is
    // exactly the Extreme plan, so the label becomes Extreme, not Custom.
    const { result } = renderHook(() => usePlan());
    act(() => result.current.applyPreset("Brutal"));
    act(() => result.current.editField("collapseToTop", false));
    expect(result.current.state.preset).toBe("Extreme");
  });

  test("serial toggle writes plan.serial from serialText", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.setSerialEnabled(true));
    expect(result.current.state.plan.serial).toBe("SLUS_208.36");
    act(() => result.current.setSerialEnabled(false));
    expect(result.current.state.plan.serial).toBeNull();
  });

  test("serial text only reaches the plan while enabled", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.setSerialText("SLUS_208.37"));
    expect(result.current.state.plan.serial).toBeNull();
    act(() => result.current.setSerialEnabled(true));
    expect(result.current.state.plan.serial).toBe("SLUS_208.37");
  });

  test("applyPreset Custom keeps the current plan", () => {
    const { result } = renderHook(() => usePlan());
    act(() => result.current.applyPreset("Extreme"));
    act(() => result.current.editField("hpMult", 1.5));
    act(() => result.current.applyPreset("Custom"));
    expect(result.current.state.plan.hpMult).toBe(1.5);
    expect(result.current.state.preset).toBe("Custom");
  });
});
