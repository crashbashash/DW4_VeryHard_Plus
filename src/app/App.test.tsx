import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { App } from "./App";
import { mockBackend } from "../ipc/mock";

let media: Record<string, boolean> = {};

function mockMatchMedia(query: string, matches: boolean) {
  media[query] = matches;
}

beforeEach(() => {
  media = {};
  vi.stubGlobal(
    "matchMedia",
    vi.fn().mockImplementation((query: string) => ({
      matches: media[query] ?? false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
    })),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("App layout", () => {
  test("renders the app header", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "DW4 Very Hard Plus" })).toBeTruthy();
  });

  test("wide layout uses two columns", () => {
    mockMatchMedia("(min-width: 900px)", true);
    render(<App />);
    expect(document.querySelector("main")?.className).toContain("layout-two-col");
  });

  test("narrow layout uses one column", () => {
    mockMatchMedia("(min-width: 900px)", false);
    render(<App />);
    expect(document.querySelector("main")?.className).toContain("layout-one-col");
  });
});

describe("App wiring", () => {
  test("preset click updates the advanced panel through the store", async () => {
    render(<App />);
    await user_clickExtreme();
    expect(screen.getByText(/leave training-stage enemies vanilla/)).toBeTruthy();
    const checkbox = screen.getByLabelText(/leave training-stage enemies vanilla/i) as HTMLInputElement;
    expect(checkbox.checked).toBe(true);
  });

  test("editing a multiplier flips the preset to Custom", async () => {
    render(<App />);
    await user_clickExtreme();
    const hp = screen.getByLabelText("hp", { selector: "input" }) as HTMLInputElement;
    // The Extreme preset's hp multiplier is 1; typing 1.5 at the end makes 11.5 —
    // clear first to replace the value.
    const user = (await import("@testing-library/user-event")).default;
    await user.clear(hp);
    await user.type(hp, "1.5");
    expect(screen.getByText("Custom")).toBeTruthy();
  });

  test("patch button starts disabled until a disc is analysed", async () => {
    render(<App />);
    const patchBtn = screen.getByRole("button", { name: /patch iso/i });
    expect(patchBtn.hasAttribute("disabled")).toBe(true); // no disc yet
    expect(screen.getByText(/choose a disc/i)).toBeTruthy();
  });

  void mockBackend;
});

async function user_clickExtreme() {
  const user = (await import("@testing-library/user-event")).default;
  await user.click(screen.getByText("Very Hard Plus — Extreme"));
}

describe("App disc wiring", () => {
  test("typing an ISO path into the disc field starts analysis", async () => {
    render(<App />);
    const user = (await import("@testing-library/user-event")).default;
    const field = screen.getByLabelText("disc path");
    await user.type(field, "/mock/game.iso");
    await user.tab(); // commit on blur, as the old egui app did
    await waitFor(() => expect(screen.getByText(/clean mock disc/)).toBeTruthy());
  });

  test("typing into the disc field suggests the output name", async () => {
    render(<App />);
    const user = (await import("@testing-library/user-event")).default;
    const field = screen.getByLabelText("disc path");
    await user.type(field, "/mock/mygame.iso");
    await user.tab(); // commit on blur
    await waitFor(() =>
      expect(
        (screen.getByLabelText("output path") as HTMLInputElement).value,
      ).toBe("/mock/mygame [VeryHardPlus].iso"),
    );
  });
});
