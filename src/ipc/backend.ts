import type { DiscStatus, PatchOutcome, PatchPlan, PlanSummary, ProgressEvent } from "../bindings";

/**
 * Everything the UI can ask of the shell, through one interface. Components
 * never import `@tauri-apps/api` directly; they get a `Backend` from
 * `useBackend()`, which is the Tauri implementation in the desktop app and
 * the mock in a plain browser tab and in tests.
 */
export interface Backend {
  chooseIso(): Promise<string | null>;
  chooseOutput(defaultName: string): Promise<string | null>;
  analyze(path: string): Promise<DiscStatus>;
  defaultOutput(input: string): Promise<string>;
  planSummary(plan: PatchPlan): Promise<PlanSummary | null>;
  startPatch(plan: PatchPlan, output: string, overwrite: boolean): Promise<void>;
  onPatchProgress(handler: (p: ProgressEvent) => void): Promise<() => void>;
  onPatchDone(handler: (o: PatchOutcome) => void): Promise<() => void>;
  onPatchFailed(handler: (message: string) => void): Promise<() => void>;
  /** An ISO dropped onto the window (desktop only; mock never fires it). */
  onFileDropped(handler: (path: string) => void): Promise<() => void>;
}
