import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { PatchOutcome, ProgressEvent } from "../bindings";
import { presetPlan } from "../features/presets/presetData";
import { mockBackend, type MockBackend } from "./mock";

describe("mock backend — patch lifecycle", () => {
  let backend: MockBackend;

  beforeEach(() => {
    backend = mockBackend();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  test("delivers progress then done, no failure", async () => {
    const progress: ProgressEvent[] = [];
    await backend.onPatchProgress((p) => progress.push(p));
    const failed = vi.fn();
    await backend.onPatchFailed(failed);
    let done: PatchOutcome | null = null;
    await backend.onPatchDone((o) => {
      done = o;
    });
    await backend.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
    expect(progress.length).toBeGreaterThan(0);
    expect(done).not.toBeNull();
    expect(done!.md5).toMatch(/^[0-9a-f]{32}$/);
    expect(failed).not.toHaveBeenCalled();
  });

  test("failure surfaces the engine message verbatim", async () => {
    const message =
      "the output file already exists — choose a new name, or allow overwriting it";
    backend.nextFailure = message;
    const failed = vi.fn();
    await backend.onPatchFailed(failed);
    await backend.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
    expect(failed).toHaveBeenCalledWith(message);
  });

  test("unlisten stops handler delivery", async () => {
    const backend2 = mockBackend();
    const handler = vi.fn();
    const unlisten = await backend2.onPatchProgress(handler);
    await unlisten();
    await backend2.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
    expect(handler).not.toHaveBeenCalled();
  });

  test("analyze returns the seeded status and clears on new failure", async () => {
    backend.seedStatus = { kind: "ok", text: "clean disc", size: 1440, copies: [1, 1, 1] };
    expect(await backend.analyze("game.iso")).toEqual({
      kind: "ok",
      text: "clean disc",
      size: 1440,
      copies: [1, 1, 1],
    });
    backend.nextAnalyzeError = "this is not a Digimon World 4 (USA) disc — use the NTSC-U release, SLUS_208.36";
    const status = await backend.analyze("bad.iso");
    expect(status.kind).toBe("refused");
    expect(status.text).toContain("NTSC-U");
  });
});
