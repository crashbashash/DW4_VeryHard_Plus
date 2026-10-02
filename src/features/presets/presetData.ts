import type { PresetId } from "../../bindings";
import type { PatchPlan } from "../../bindings/PatchPlan";

/**
 * The named presets' plans, transcribed from `dw4vhp_core::plan::Preset::plan`.
 * A unit test in `preset.test.ts` pins these values; changing a preset in the
 * engine means changing this table in the same commit.
 *
 * `Custom` has no canonical plan — it is only a label for an edited plan — so
 * `presetPlan("Custom")` throws, mirroring the Rust baseline comment.
 */
export function presetPlan(id: PresetId): PatchPlan {
  switch (id) {
    case "VeryHardPlus":
      return {
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
      };
    case "Extreme":
      return {
        ...presetPlan("VeryHardPlus"),
        excludePractice: true,
        forceVeryHard: true,
      };
    case "Brutal":
      return {
        ...presetPlan("Extreme"),
        collapseToTop: true,
      };
    case "Custom":
      throw new Error("Custom has no canonical plan; it labels an edited plan");
  }
}

/** The preset a plan is exactly equal to, or Custom for any other plan. */
export function detectPreset(plan: PatchPlan): PresetId {
  for (const id of ["VeryHardPlus", "Extreme", "Brutal"] as const) {
    if (plansEqual(plan, presetPlan(id))) return id;
  }
  return "Custom";
}

/** Deep equality over PatchPlan (statMult is an array, so !== is not enough). */
function plansEqual(a: PatchPlan, b: PatchPlan): boolean {
  return (
    a.hpMult === b.hpMult &&
    a.critMult === b.critMult &&
    a.paraMult === b.paraMult &&
    a.crownRank === b.crownRank &&
    a.excludeDestructibles === b.excludeDestructibles &&
    a.excludeTripwire === b.excludeTripwire &&
    a.excludePractice === b.excludePractice &&
    a.practiceBuff === b.practiceBuff &&
    a.collapseToTop === b.collapseToTop &&
    a.forceVeryHard === b.forceVeryHard &&
    a.serial === b.serial &&
    a.statMult.length === b.statMult.length &&
    a.statMult.every((v, i) => v === b.statMult[i])
  );
}

/** GUI-facing names, pinned to the engine's `Preset::label`. */
export const PRESET_LABELS: Record<PresetId, string> = {
  VeryHardPlus: "Very Hard Plus",
  Extreme: "Very Hard Plus — Extreme",
  Brutal: "Very Hard Plus — Brutal",
  Custom: "Custom",
};

/** Short descriptions for the preset cards. */
export const PRESET_NOTES: Record<PresetId, string> = {
  VeryHardPlus: "Big enemy boosts and the yellow crown, as shipped and verified.",
  Extreme: "Also starts you on Very Hard difficulty right away.",
  Brutal: "Extreme, and every enemy of a type is its strongest version.",
  Custom: "Your own edited settings — shown once you change a dial.",
};

/**
 * The twelve authored `i16` columns, in the order the table stores them.
 * Mirrors the engine's `STAT_NAMES`.
 */
export const STAT_NAMES = [
  "atk", "def", "wis", "spr", "spd", "fire", "ice", "thunder", "dark", "stun", "poison", "exp",
] as const;

/**
 * The crown colours the game's SETRAREICON draws for RARITY 0-5, as verified
 * live in the decomp (rank 0 draws nothing).
 */
export const CROWN_COLOURS: { rank: number; name: string }[] = [
  { rank: 0, name: "none (no crown)" },
  { rank: 1, name: "green" },
  { rank: 2, name: "blue" },
  { rank: 3, name: "pink" },
  { rank: 4, name: "white" },
  { rank: 5, name: "yellow" },
];

/** Swatch colour per crown rank, for the dropdown's colour chips. */
export const CROWN_SWATCH: Record<number, string> = {
  0: "transparent",
  1: "#3fae5a",
  2: "#3f6fae",
  3: "#d06fb0",
  4: "#f0f0f0",
  5: "#d4c020",
};

/** The disc's shipped serial; the serial field's placeholder and default. */
export const DEFAULT_SERIAL = "SLUS_208.36";
