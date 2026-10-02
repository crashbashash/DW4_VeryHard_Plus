import { describe, expect, test } from "vitest";
import { CROWN_COLOURS, DEFAULT_SERIAL, STAT_NAMES, detectPreset, presetPlan } from "./presetData";

describe("preset plans match the Rust Preset::plan values", () => {
  test("VeryHardPlus is the default plan", () => {
    expect(presetPlan("VeryHardPlus")).toEqual({
      hpMult: 1.0,
      statMult: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
      critMult: 1.0,
      paraMult: 1.0,
      crownRank: 5,
      excludeDestructibles: true,
      excludeTripwire: true,
      excludePractice: false,
      practiceBuff: true,
      collapseToTop: false,
      forceVeryHard: false,
      serial: null,
    });
  });

  test("Extreme adds excludePractice and forceVeryHard", () => {
    expect(presetPlan("Extreme")).toMatchObject({
      excludePractice: true,
      forceVeryHard: true,
      collapseToTop: false,
      excludeDestructibles: true,
      excludeTripwire: true,
      practiceBuff: true,
      crownRank: 5,
    });
  });

  test("Brutal adds collapseToTop on top of Extreme", () => {
    expect(presetPlan("Brutal")).toMatchObject({
      excludePractice: true,
      forceVeryHard: true,
      collapseToTop: true,
    });
  });
});

describe("detect matches Rust Preset::detect", () => {
  test.each([
    ["VeryHardPlus" as const],
    ["Extreme" as const],
    ["Brutal" as const],
  ])("%s round-trips", (id) => {
    expect(detectPreset(presetPlan(id))).toBe(id);
  });

  test("a one-field edit is Custom", () => {
    const plan = presetPlan("VeryHardPlus");
    expect(detectPreset({ ...plan, statMult: [2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1] })).toBe("Custom");
  });

  test("serial is part of the plan's equality: a serial edit makes it Custom", () => {
    expect(detectPreset({ ...presetPlan("VeryHardPlus"), serial: "SLUS_208.36" })).toBe("Custom");
  });

  test("Custom has no canonical plan", () => {
    expect(() => presetPlan("Custom")).toThrow();
  });
});

test("crown colours rank 0-5 per SETRAREICON", () => {
  expect(CROWN_COLOURS.map((c) => c.rank)).toEqual([0, 1, 2, 3, 4, 5]);
  expect(CROWN_COLOURS[5]!.name).toContain("yellow");
});

test("stat names are the twelve authored columns in table order", () => {
  expect(STAT_NAMES).toEqual([
    "atk", "def", "wis", "spr", "spd", "fire", "ice", "thunder", "dark", "stun", "poison", "exp",
  ]);
});

test("default serial is the retail one", () => {
  expect(DEFAULT_SERIAL).toBe("SLUS_208.36");
});
