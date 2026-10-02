/** Mirrors `dw4vhp_core::plan::PlanSummary`. */
export interface PlanSummary {
  rowsChanged: number;
  liveRows: number;
  attackPinned: number;
  hpCapped: number;
  critPinned: number;
  practiceRows: number;
  collapsedRows: number;
  elfStep: boolean;
  serialStep: boolean;
}
