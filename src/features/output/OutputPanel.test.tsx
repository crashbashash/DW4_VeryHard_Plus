import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";
import type { DiscStatus, PatchOutcome, PlanSummary, ProgressEvent } from "../../bindings";
import { OutputPanel } from "./OutputPanel";

function setup(overrides: Partial<Parameters<typeof OutputPanel>[0]> = {}) {
  const onPatch = vi.fn();
  const onChooseOutput = vi.fn();
  const onSetOutput = vi.fn();
  const onSetOverwrite = vi.fn();
  const props = {
    outputPath: "/mock/game [VeryHardPlus].iso",
    overwrite: false,
    busy: false,
    canPatch: true,
    progress: null as ProgressEvent | null,
    log: [] as string[],
    status: null as DiscStatus | null,
    summary: null as PlanSummary | null,
    lastOutcome: null as PatchOutcome | null,
    onPatch,
    onChooseOutput,
    onSetOutput,
    onSetOverwrite,
    ...overrides,
  };
  render(<OutputPanel {...props} />);
  return { onPatch, onSetOutput, onSetOverwrite, onChooseOutput };
}

describe("OutputPanel", () => {
  test("Patch button disabled when cannot patch", () => {
    setup({ canPatch: false });
    expect(screen.getByRole("button", { name: /patch iso/i }).hasAttribute("disabled")).toBe(true);
  });

  test("Patch button enabled and triggers the callback", async () => {
    const { onPatch } = setup();
    await userEvent.click(screen.getByRole("button", { name: /patch iso/i }));
    expect(onPatch).toHaveBeenCalled();
  });

  test("free-space note shows the analysed size", () => {
    setup({ status: { kind: "ok", text: "", size: 4600000000, copies: [1, 1, 1] } });
    expect(screen.getByText(/4,600,000,000|4600000000/)).toBeTruthy();
  });

  test("attack-pin and collapse notes appear when the summary reports them", () => {
    setup({
      summary: {
        rowsChanged: 580,
        liveRows: 580,
        attackPinned: 12,
        hpCapped: 0,
        critPinned: 0,
        practiceRows: 0,
        collapsedRows: 40,
        elfStep: false,
        serialStep: false,
      },
    });
    expect(screen.getByText(/attack: 12 of 580 live rows pinned at 32767/)).toBeTruthy();
    expect(screen.getByText(/collapse: 40 rows now carry their type's strongest row/)).toBeTruthy();
  });

  test("engine failure message lands in the log verbatim", () => {
    setup({
      log: ["patch refused: the output file already exists — choose a new name, or allow overwriting it"],
    });
    expect(screen.getByText(/the output file already exists/)).toBeTruthy();
  });

  test("overwrite checkbox edits the flag", async () => {
    const { onSetOverwrite } = setup();
    await userEvent.click(screen.getByLabelText(/overwrite existing file/i));
    expect(onSetOverwrite).toHaveBeenCalledWith(true);
  });

  test("progress bar shows phase and fraction", () => {
    setup({ progress: { phase: "copying", done: 500, total: 1000 }, busy: true });
    expect(screen.getByText(/copying/)).toBeTruthy();
  });
});
