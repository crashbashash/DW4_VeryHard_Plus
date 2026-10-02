import type { DiscStatus, PatchOutcome, PatchPlan, PlanSummary, ProgressEvent } from "../bindings";
import type { Backend } from "./backend";

/**
 * The mock backend: drives the whole UI flow in a plain browser tab and in
 * tests. The patch synthesises three progress events and a done payload; the
 * failure path and the analyze verdict are queued through the public fields.
 */
export class MockBackend implements Backend {
  /** The status the next `analyze` call returns, then resets. */
  seedStatus: DiscStatus | null = null;
  /** The message the next `startPatch` reports as patch-failed, then resets. */
  nextFailure: string | null = null;
  /** The error text the next `analyze` reports (kind derived from content). */
  nextAnalyzeError: string | null = null;
  /** The summary the next `planSummary` returns, then resets. */
  seedSummary: PlanSummary | null = null;
  private listeners: Record<string, Set<(...args: never[]) => void>> = {};

  async chooseIso(): Promise<string | null> {
    return "/mock/game.iso";
  }

  async chooseOutput(defaultName: string): Promise<string | null> {
    return `/mock/${defaultName}`;
  }

  async analyze(path: string): Promise<DiscStatus> {
    const error = this.nextAnalyzeError;
    this.nextAnalyzeError = null;
    if (error !== null) {
      return { kind: "refused", text: error, size: 0, copies: null };
    }
    const seeded = this.seedStatus;
    this.seedStatus = null;
    return (
      seeded ?? {
        kind: "ok",
        text: `clean mock disc from ${path}`,
        size: 4_600_000_000,
        copies: [10, 580, 5],
      }
    );
  }

  async defaultOutput(input: string): Promise<string> {
    return input.replace(/\.iso$/i, " [VeryHardPlus].iso");
  }

  async planSummary(plan: PatchPlan): Promise<PlanSummary | null> {
    const seeded = this.seedSummary;
    this.seedSummary = null;
    // A tiny deterministic stand-in so the panel has data in the browser.
    return (
      seeded ?? {
        rowsChanged: 580,
        liveRows: 580,
        attackPinned: (plan.statMult[0] ?? 1) > 1 ? 120 : 0,
        hpCapped: plan.hpMult > 1 ? 40 : 0,
        critPinned: 0,
        practiceRows: 0,
        collapsedRows: plan.collapseToTop ? 120 : 0,
        elfStep: plan.forceVeryHard,
        serialStep: plan.serial !== null,
      }
    );
  }

  async startPatch(
    _plan: PatchPlan,
    _output: string,
    _overwrite: boolean,
  ): Promise<void> {
    for (let i = 1; i <= 3; i++) {
      this.emit<ProgressEvent>("patch-progress", { phase: "copying", done: i, total: 3 });
    }
    const failure = this.nextFailure;
    this.nextFailure = null;
    if (failure !== null) {
      this.emit("patch-failed", failure);
      return;
    }
    this.emit<PatchOutcome>("patch-done", {
      md5: "d41d8cd98f00b204e9800998ecf8427e",
      bytes: 4_600_000_000,
      summary: {
        rowsChanged: 580,
        liveRows: 580,
        attackPinned: 0,
        hpCapped: 0,
        critPinned: 0,
        practiceRows: 0,
        collapsedRows: 0,
        elfStep: false,
        serialStep: false,
      },
    });
  }

  async onPatchProgress(handler: (p: ProgressEvent) => void): Promise<() => void> {
    return this.subscribe("patch-progress", handler as (...args: never[]) => void);
  }

  async onPatchDone(handler: (o: PatchOutcome) => void): Promise<() => void> {
    return this.subscribe("patch-done", handler as (...args: never[]) => void);
  }

  async onPatchFailed(handler: (message: string) => void): Promise<() => void> {
    return this.subscribe("patch-failed", handler as (...args: never[]) => void);
  }

  async onFileDropped(): Promise<() => void> {
    return () => {};
  }

  subscribe(name: string, handler: (...args: never[]) => void): () => void {
    const set = (this.listeners[name] ??= new Set());
    set.add(handler);
    return () => {
      set.delete(handler);
    };
  }

  private emit<T>(name: string, payload: T): void {
    for (const handler of this.listeners[name] ?? []) {
      (handler as (p: T) => void)(payload);
    }
  }
}

export function mockBackend(): MockBackend {
  return new MockBackend();
}
