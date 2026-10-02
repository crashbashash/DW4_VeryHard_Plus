/** One `patch-progress` event: writer phase plus byte counters. */
export interface ProgressEvent {
  phase: "inspecting" | "copying" | "writing" | "verifying";
  done: number;
  total: number;
}
