/** The `patch-done` payload: verified result of the write. */
export interface PatchOutcome {
  md5: string;
  bytes: number;
  summary: import("./PlanSummary").PlanSummary;
}
