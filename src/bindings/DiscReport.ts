/** The GUI-facing slice of `dw4vhp_core::disc::DiscReport` (no enemy table). */
export interface DiscReport {
  size: number;
  /** Authored copy counts of the HPMAX, stats and chargen blocks. */
  copies: [number, number, number];
  bootElfPresent: boolean;
}
