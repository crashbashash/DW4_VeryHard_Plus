import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { MockBackend } from "../../ipc/mock";
import { useDisc } from "./useDisc";

describe("useDisc — pick, default output, auto-analyze", () => {
  test("choosing a disc sets the path then auto-analyzes", async () => {
    const backend = new MockBackend();
    let analyzed: string | null = null;
    const original = backend.analyze.bind(backend);
    backend.analyze = async (path) => {
      analyzed = path;
      return original(path);
    };
    const { result } = renderHook(() => useDisc(backend));
    await result.current.pickDisc();
    await waitFor(() => expect(result.current.status?.kind).toBe("ok"));
    expect(result.current.inputPath).toBe("/mock/game.iso");
    expect(analyzed).toBe("/mock/game.iso");
  });

  test("setInputPath computes the default output and analyzes", async () => {
    const backend = new MockBackend();
    const { result } = renderHook(() => useDisc(backend));
    await result.current.setInputPath("/mock/mygame.iso");
    await waitFor(() => expect(result.current.status?.kind).toBe("ok"));
    expect(result.current.outputPath).toBe("/mock/mygame [VeryHardPlus].iso");
  });

  test("an analyzed refusal is surfaced as status", async () => {
    const backend = new MockBackend();
    backend.nextAnalyzeError = "this disc is already modded (600 rows carry a crown) — use your original ISO";
    const { result } = renderHook(() => useDisc(backend));
    await result.current.setInputPath("/mock/modded.iso");
    await waitFor(() => expect(result.current.status?.kind).toBe("refused"));
  });
});
