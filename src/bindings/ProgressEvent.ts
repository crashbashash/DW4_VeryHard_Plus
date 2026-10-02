/** One `patch-progress` event: writer phase plus byte counters. */
export interface ProgressEvent {
  phase: "copying" | "writing" | "verifying";
  done: number;
  total: number;
}
