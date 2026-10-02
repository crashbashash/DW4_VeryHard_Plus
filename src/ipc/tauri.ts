import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DiscStatus, PatchOutcome, PatchPlan, PlanSummary, ProgressEvent } from "../bindings";
import type { Backend } from "./backend";

/** The real backend: Tauri invoke + event listen. */
export class TauriBackend implements Backend {
  async chooseIso(): Promise<string | null> {
    return invoke<string | null>("choose_iso");
  }

  async chooseOutput(defaultName: string): Promise<string | null> {
    return invoke<string | null>("choose_output", { defaultName });
  }

  async analyze(path: string): Promise<DiscStatus> {
    return invoke<DiscStatus>("analyze", { path });
  }

  async defaultOutput(input: string): Promise<string> {
    return invoke<string>("default_output", { input });
  }

  async planSummary(plan: PatchPlan): Promise<PlanSummary | null> {
    return invoke<PlanSummary | null>("plan_summary", { plan });
  }

  async startPatch(plan: PatchPlan, output: string, overwrite: boolean): Promise<void> {
    await invoke("start_patch", { plan, output, overwrite });
  }

  async onPatchProgress(handler: (p: ProgressEvent) => void): Promise<() => void> {
    return listen<ProgressEvent>("patch-progress", (event) => handler(event.payload));
  }

  async onPatchDone(handler: (o: PatchOutcome) => void): Promise<() => void> {
    return listen<PatchOutcome>("patch-done", (event) => handler(event.payload));
  }

  async onPatchFailed(handler: (message: string) => void): Promise<() => void> {
    return listen<string>("patch-failed", (event) => handler(event.payload));
  }

  async onFileDropped(handler: (path: string) => void): Promise<() => void> {
    const unlisten = await getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === "drop" && event.payload.paths.length > 0) {
        const iso = event.payload.paths.find((p) => p.toLowerCase().endsWith(".iso"));
        if (iso !== undefined) handler(iso);
      }
    });
    return unlisten as UnlistenFn;
  }
}
