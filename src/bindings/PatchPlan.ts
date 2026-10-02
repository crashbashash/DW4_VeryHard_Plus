/** Mirrors `dw4vhp_core::plan::PatchPlan`, camelCase for the wire. */
export interface PatchPlan {
  hpMult: number;
  /** The twelve authored stat columns, in table order (see STAT_NAMES). */
  statMult: number[];
  critMult: number;
  paraMult: number;
  /** The RARITY value the game's SETRAREICON draws as a crown, 0-5. */
  crownRank: number;
  excludeDestructibles: boolean;
  excludeTripwire: boolean;
  excludePractice: boolean;
  practiceBuff: boolean;
  collapseToTop: boolean;
  forceVeryHard: boolean;
  /** 11-character serial like SLUS_208.36, or null for no rename. */
  serial: string | null;
}
