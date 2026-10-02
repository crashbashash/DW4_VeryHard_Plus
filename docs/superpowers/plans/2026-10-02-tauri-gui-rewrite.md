# Tauri GUI rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the egui `dw4vhp-gui` crate with a Tauri 2 + React 19 + TypeScript GUI in the DW4orge style, fixing resize responsiveness and preset↔Advanced reactivity.

**Architecture:** `dw4vhp-core` is untouched. A new `crates/dw4vhp-tauri` shell exposes six commands and two patch events over Tauri IPC; a new Vite+React frontend in `src/` owns all plan state in one `useReducer` store, so preset selection and advanced edits can never desync. All shell access goes through one `src/ipc/backend.ts` seam with a mock implementation for tests.

**Tech Stack:** Tauri 2 (wry/tauri + plugin-dialog), React 19, TypeScript strict, Vite, Vitest + Testing Library, eslint 9, react-aria-components. Rust workspace stays edition 2021 for members.

**Spec:** `docs/superpowers/specs/2026-10-02-tauri-gui-rewrite-design.md`

## Global Constraints

- `crates/dw4vhp-core` must not be modified in any task; `cargo test --workspace` must stay green throughout.
- Root `Cargo.toml` members become `["crates/dw4vhp-core"]`; `crates/dw4vhp-tauri` is NOT a workspace member (webkit2gtk system deps, same exclusion as DW4orge's `src-tauri`).
- Preset values pinned from `crates/dw4vhp-core/src/plan.rs`: default plan is `hp_mult 1.0, stat_mult [1.0;12], crit_mult 1.0, para_mult 1.0, crown_rank 5, exclude_destructibles true, exclude_tripwire true, exclude_practice false, practice_buff true, collapse_to_top false, force_very_hard false, serial None`. Extreme = default + `exclude_practice:true, force_very_hard:true`. Brutal = Extreme + `collapse_to_top:true`. Labels: "Very Hard Plus", "Very Hard Plus — Extreme", "Very Hard Plus — Brutal", "Custom".
- Crown colours rank 0–5: `none (no crown)`, `green`, `blue`, `pink`, `white`, `yellow`.
- Serial default `SLUS_208.36` (`serial::DEFAULT_SERIAL`); output suffix ` [VeryHardPlus].iso`.
- MSI UpgradeCode stays `bd9d3915-b839-48c7-ae79-afbf60a6cd78`.
- App identifier `io.github.crashbashash.dw4-veryhard-plus`; window title "DW4 Very Hard Plus".
- No network access in the app (Tauri CSP restricts; no fetch calls).
- This container cannot compile the Tauri crate (no webkit2gtk system deps). Tauri-crate compile/test steps are verified by CI; every task must still leave the workspace and frontend gates green locally: `cargo test --workspace`, `npm run typecheck`, `npm run lint`, `npm test`.

## Review Focus

- **Preset↔plan desync after a failed preset application:** selecting Extreme then editing `exclude_practice` must show Custom, and re-selecting Extreme must reset ALL fields including serial rename to off — test: `usePlan.test.ts` "applyPreset resets serial and every flag", `preset.test.ts` "detect matches Rust Preset::detect".
- **Plan with a serial set must still detect the right preset** (Rust's `detect` ignores nothing — `PatchPlan` includes `serial`, and `Option` is part of `PartialEq`; a plan differing only by serial is Custom): test in `preset.test.ts` pinning that a VeryHardPlus plan + serial ≠ VHP preset.
- **Progress events outliving the patch:** user closes/reloads the window mid-patch; events must not panic the Rust worker and the frontend must drop stale listeners — test: `tauri.ts` unlisten cleanup in `backend.test.ts` (mock), Rust side relies on `tauri::Emitter`'s already-safe send; manual CI verify.
- **Overwrite refusal UX:** patching to an existing output with overwrite off must surface `Error::OutputExists`'s message verbatim in the log, not a generic failure — test: `disc.test.tsx` / integration via mock `backend.test.ts` "patch failure surfaces engine message".
- **Window resize extremes (500px and 1600px wide):** no horizontal scrollbar, no clipped controls; the multiplier grid reflows — test: `app.test.tsx` asserting the two-pane container has class `layout-two-col` at wide via matchMedia mock and `layout-one-col` at narrow.

---

### Task 1: Remove the egui crate, scaffold the frontend

**Files:**
- Delete: `crates/dw4vhp-gui/` (whole directory)
- Modify: `Cargo.toml` (root: members)
- Create: `package.json`, `tsconfig.json`, `vite.config.ts`, `eslint.config.js`, `index.html`, `src/main.tsx`, `src/app/App.tsx`, `src/app/theme.css`, `src/styles/` (imported by theme.css)
- Copy: `crates/dw4vhp-gui/icons/icon.ico` and `icon.png` → `crates/dw4vhp-tauri/icons/` (do this BEFORE deleting the gui crate)

**Interfaces:**
- Produces: a runnable `npm run dev`/`vite build`/`npm test` frontend with `App.tsx` rendering the app shell (header, one main region) and `theme.css` defining light/dark via `prefers-color-scheme` CSS variables (`--bg`, `--fg`, `--accent`, `--danger`); `App.test.tsx` renders the header "DW4 Very Hard Plus".

- [ ] **Step 1: Copy icons and delete the gui crate, update root Cargo.toml**

```bash
mkdir -p crates/dw4vhp-tauri/icons
cp crates/dw4vhp-gui/icons/icon.ico crates/dw4vhp-tauri/icons/
cp crates/dw4vhp-gui/icons/icon.png crates/dw4vhp-tauri/icons/
rm -rf crates/dw4vhp-gui
```

Root `Cargo.toml`: `members = ["crates/dw4vhp-core"]`.

- [ ] **Step 2: Scaffold Vite+React frontend**

`package.json` scripts mirror DW4orge's (`dev`, `build` = `tsc --noEmit && vite build`, `test` = `vitest run`, `typecheck`, `lint`); deps: `react@^19.3.0`, `react-dom@^19.3.0`, `react-aria-components@^1.21.1`, `@tauri-apps/api@^2`, `@tauri-apps/plugin-dialog@^2`; devDeps as DW4orge (vite, @vitejs/plugin-react, typescript ~5.9.3, vitest, @testing-library/react + user-event, jsdom, eslint 9 + typescript-eslint + react-hooks plugin, @tauri-apps/cli). `tsconfig.json` strict, `jsx: "react-jsx"`, types `vite/client`. `vite.config.ts` with `@vitejs/plugin-react` and `test` config (environment jsdom, globals). `index.html` mounts `#root` → `main.tsx` → `App`. `theme.css` defines the CSS variables for light and dark and a base reset; `App.tsx` renders `<h1>DW4 Very Hard Plus</h1>` plus `<main>` placeholder.

- [ ] **Step 3: Write the failing test**

`src/app/App.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { App } from "./App";

test("renders the app header", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "DW4 Very Hard Plus" })).toBeTruthy();
});
```

- [ ] **Step 4: Run test to verify it passes** (it may pass immediately since App is written with the scaffold; if so, verify it FAILS first by running before creating App.tsx)

Run: `npm test`
Expected: PASS (1 test)

- [ ] **Step 5: Run all gates**

Run: `npm run typecheck && npm run lint && npm test && cargo test --workspace`
Expected: all PASS, and `cargo test --workspace` covers only dw4vhp-core now.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(gui)!: remove egui app, scaffold Tauri+React frontend shell"
```

### Task 2: Bindings and the preset logic (the reactivity core)

**Files:**
- Create: `src/bindings/PatchPlan.ts`, `src/bindings/Preset.ts`, `src/bindings/PlanSummary.ts`, `src/bindings/DiscReport.ts`, `src/bindings/DiscStatus.ts`, `src/bindings/ProgressEvent.ts`, `src/bindings/PatchOutcome.ts`, `src/bindings/index.ts`
- Create: `src/features/presets/presetData.ts`
- Test: `src/features/presets/preset.test.ts`

**Interfaces:**
- Consumes: the engine values in Global Constraints (verbatim).
- Produces:
  - `PatchPlan` type: `{ hpMult: number; statMult: number[] /* length 12 */; critMult: number; paraMult: number; crownRank: number; excludeDestructibles: boolean; excludeTripwire: boolean; excludePractice: boolean; practiceBuff: boolean; collapseToTop: boolean; forceVeryHard: boolean; serial: string | null }`
  - `PRESETS: PresetId[]` = `["VeryHardPlus", "Extreme", "Brutal", "Custom"]`; `presetPlan(id): PatchPlan`; `detectPreset(plan): PresetId`; `PRESET_LABELS: Record<PresetId, string>`; `CROWN_COLOURS: { rank: number; name: string }[]` (6 entries per Global Constraints); `STAT_NAMES: string[12]` = `["atk","def","wis","spr","spd","fire","ice","thunder","dark","stun","poison","exp"]`; `DEFAULT_SERIAL = "SLUS_208.36"`.

- [ ] **Step 1: Write the failing test**

`src/features/presets/preset.test.ts` — assertions pinning Global Constraints:

```ts
import { detectPreset, presetPlan } from "./presetData";

test("preset plans match the Rust Preset::plan values", () => {
  expect(presetPlan("VeryHardPlus")).toEqual({
    hpMult: 1.0, statMult: [1,1,1,1,1,1,1,1,1,1,1,1], critMult: 1.0, paraMult: 1.0,
    crownRank: 5, excludeDestructibles: true, excludeTripwire: true,
    excludePractice: false, practiceBuff: true, collapseToTop: false,
    forceVeryHard: false, serial: null,
  });
  expect(presetPlan("Extreme")).toMatchObject({ excludePractice: true, forceVeryHard: true, collapseToTop: false });
  expect(presetPlan("Brutal")).toMatchObject({ excludePractice: true, forceVeryHard: true, collapseToTop: true });
});

test("detect matches Rust Preset::detect", () => {
  expect(detectPreset(presetPlan("VeryHardPlus"))).toBe("VeryHardPlus");
  expect(detectPreset(presetPlan("Extreme"))).toBe("Extreme");
  expect(detectPreset(presetPlan("Brutal"))).toBe("Brutal");
  expect(detectPreset({ ...presetPlan("VeryHardPlus"), statMult: [2,1,1,1,1,1,1,1,1,1,1,1] })).toBe("Custom");
  // serial is part of the plan's equality: a serial edit makes it Custom
  expect(detectPreset({ ...presetPlan("VeryHardPlus"), serial: "SLUS_208.36" })).toBe("Custom");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- preset`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `presetData.ts` and the bindings types**

`presetData.ts` holds the three concrete preset plans as literals (transcribed from `Preset::plan`) and implements `detectPreset` by deep equality against each of the three, returning `"Custom"` otherwise. Bindings files are plain `export type` declarations matching the Rust serialisations defined in Task 4 (`camelCase` keys; `DiscStatus` = `{ kind: "ok" | "refused" | "unknown"; text: string }`; `ProgressEvent` = `{ phase: "copying" | "writing" | "verifying"; done: number; total: number }`; `PatchOutcome` = `{ md5: string; bytes: number; summary: PlanSummary }`; `DiscReport` = `{ size: number; copies: [number, number, number]; bootElfPresent: boolean }` — the shell sends only the GUI-facing fields, not the enemy table).

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- preset`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/bindings src/features/presets
git commit -m "feat(frontend): bindings types and preset/detect logic"
```

### Task 3: The plan store (usePlan) — the reactivity fix

**Files:**
- Create: `src/features/presets/usePlan.ts`
- Test: `src/features/presets/usePlan.test.ts`

**Interfaces:**
- Consumes: `presetPlan`, `detectPreset` (Task 2).
- Produces: `usePlan(): { state: PlanUiState; applyPreset(id: PresetId): void; editField<K extends PlanField>(field: K, value: PatchPlan[K]): void; setSerialEnabled(on: boolean): void; setSerialText(text: string): void }` where `PlanUiState = { preset: PresetId; plan: PatchPlan; serialEnabled: boolean; serialText: string }`; type `PlanField` = string keys of `PatchPlan` except `serial` (serial goes through `setSerial*`).

- [ ] **Step 1: Write the failing test**

`src/features/presets/usePlan.test.ts` — render a probe component with `renderHook`; assertions:

```ts
const { result } = renderHook(() => usePlan());
// default: VHP preset, default plan
expect(result.current.state.preset).toBe("VeryHardPlus");
// applyPreset loads the whole plan AND resets serial controls
act(() => result.current.applyPreset("Extreme"));
expect(result.current.state.plan.excludePractice).toBe(true);
expect(result.current.state.plan.forceVeryHard).toBe(true);
// editField flips preset to Custom and keeps the other fields
act(() => result.current.editField("hpMult", 1.5));
expect(result.current.state.preset).toBe("Custom");
expect(result.current.state.plan.excludePractice).toBe(true);
// applyPreset after an edit resets serialEnabled/serialText to defaults
act(() => result.current.setSerialEnabled(true));
act(() => result.current.setSerialText("SLUS_208.37"));
act(() => result.current.applyPreset("Brutal"));
expect(result.current.state.serialEnabled).toBe(false);
expect(result.current.state.serialText).toBe("SLUS_208.36");
expect(result.current.state.plan.serial).toBeNull();
expect(result.current.state.preset).toBe("Brutal");
// editing back to the exact preset plan still reports Custom until a preset is applied
act(() => result.current.editField("collapseToTop", false));
expect(result.current.state.preset).toBe("Custom");
// serial toggle writes plan.serial from serialEnabled/serialText
act(() => result.current.setSerialEnabled(true));
expect(result.current.state.plan.serial).toBe("SLUS_208.36");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- usePlan`
Expected: FAIL

- [ ] **Step 3: Implement `usePlan.ts`**

One `useReducer` with action types `PresetApplied(PresetId)`, `FieldEdited(field, value)`, `SerialEnabled(bool)`, `SerialText(string)`. Each action's reducer: build the next `PlanUiState`, then recompute `preset = detectPreset(nextPlan)` **except** for `PresetApplied`, which sets the preset directly (Custom applies no plan). `SerialEnabled(true)` sets `plan.serial = serialText` when non-empty; `SerialText` updates `plan.serial` only when `serialEnabled`.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- usePlan`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/presets/usePlan.ts src/features/presets/usePlan.test.ts
git commit -m "feat(frontend): single plan store fixing preset/advanced reactivity"
```

### Task 4: Tauri shell — crate scaffold and state

**Files:**
- Create: `crates/dw4vhp-tauri/Cargo.toml`, `crates/dw4vhp-tauri/build.rs`, `crates/dw4vhp-tauri/tauri.conf.json`, `crates/dw4vhp-tauri/capabilities/default.json`, `crates/dw4vhp-tauri/src/main.rs`, `crates/dw4vhp-tauri/src/lib.rs`, `crates/dw4vhp-tauri/src/state.rs`

**Interfaces:**
- Consumes: `dw4vhp-core` (path dep), icons copied in Task 1.
- Produces: `state::AppState` (`Mutex<Option<disc::DiscReport>>` for the analysed report + plan paths); crate name `dw4vhp-tauri`, bin name `dw4-veryhard-plus`, lib name `dw4vhp_tauri`.

- [ ] **Step 1: Write the crate files**

`Cargo.toml` (not a workspace member — root already excludes it):

```toml
[package]
name = "dw4vhp-tauri"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0-or-later"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
dw4vhp-core = { path = "../dw4vhp-core" }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`build.rs` = `tauri_build::build()`. `tauri.conf.json` mirrors DW4orge's shape: `productName "DW4 Very Hard Plus"`, `identifier "io.github.crashbashash.dw4-veryhard-plus"`, `build.frontendDist "../dist"`, `devUrl "http://localhost:5173"`, `beforeDevCommand "npm run dev"`, `beforeBuildCommand "npm run build"`, window `{ title, width 1000, height 720, dragDropEnabled: true }`, `bundle.icon` pointing at the copied icons, and `bundle.windows.wix.upgradeCode "bd9d3915-b839-48c7-ae79-afbf60a6cd78"`. Capabilities: `core:default`, `dialog:default` — no network permissions. `main.rs` calls `dw4vhp_tauri::run()`. `lib.rs` builds the tauri app, registers `.manage(AppState::default())` and the commands from Task 5. `state.rs`:

```rust
#[derive(Default)]
pub struct AppState {
    pub report: std::sync::Mutex<Option<dw4vhp_core::disc::DiscReport>>,
}
```

- [ ] **Step 2: Verify what is verifiable locally**

Run: `cargo test --workspace`
Expected: PASS (dw4vhp-core only; the new crate is excluded so no webkit deps needed). Record in the task notes that `cargo check` of `crates/dw4vhp-tauri` is CI-only in this container.

- [ ] **Step 3: Commit**

```bash
git add crates/dw4vhp-tauri Cargo.toml
git commit -m "feat(shell): Tauri crate scaffold, config, capabilities, state"
```

### Task 5: Tauri commands and patch events

**Files:**
- Create: `crates/dw4vhp-tauri/src/commands.rs`
- Modify: `crates/dw4vhp-tauri/src/lib.rs` (register commands), `src/state.rs` if needed

**Interfaces:**
- Consumes: `disc::inspect`, `patch::{patch_file_with_layout, PatchOptions}`, `plan::{transform, PatchPlan}`, `ratio::RatioVector`, `writer::{Phase, Progress}`; `state::AppState`.
- Produces (all `#[tauri::command]`, all payload types `#[derive(Serialize, Clone)]` with camelCase `#[serde(rename_all = "camelCase")]`):
  - `choose_iso() -> Option<String>` — `tauri_plugin_dialog` file dialog, filter `*.iso`
  - `choose_output(default_name: String) -> Option<String>`
  - `analyze(path: String, state) -> DiscStatusPayload` — spawns a thread running `inspect(&path, &Layout::retail())`; on success stores the `DiscReport` in `AppState` and returns `{ kind: "ok"|"refused"|"unknown", text: String, size: u64, copies: [usize;3] }`; on error, kind is `refused` for `AlreadyModded|WrongRevision|NotThisDisc` (same mapping as the old `form::status_for`) else `unknown`, text = `error.to_string()`
  - `plan_summary(plan: PlanPayload) -> PlanSummaryPayload` — pure: reads the stored report, `RatioVector::derive`, `transform`, returns the summary with camelCase field names matching `PlanSummary` bindings
  - `start_patch(plan, output: String, overwrite: bool, app_handle) -> Result<(), String>` — spawns a thread: `patch_file_with_layout(input from stored paths… )`; progress callback emits `"patch-progress"` with `{phase, done, total}`; terminal emits `"patch-done"` with `{md5, bytes, summary}` or `"patch-failed"` with the engine's error string. Returns `Err(msg)` synchronously only for argument problems (no analysed disc).
  - `default_output(input: String) -> String` — ports `form::default_output_path` (stem + ` [VeryHardPlus].iso`, same directory).
- The stored input path: `analyze` also stores the input path in `AppState` (`Mutex<Option<PathBuf>>`).

- [ ] **Step 1: Implement `commands.rs` and register commands**

No `#[cfg(test)]` tests here — this crate cannot compile in the container; correctness is covered by the TS-side tests against the mock (Task 6) and by CI compiling this crate. Keep the functions thin: every engine call is one of the signatures above; the status-kind mapping and the output-suffix naming are the only logic, transcribed exactly from `form.rs` (StatusKind mapping and `default_output_path`).

- [ ] **Step 2: Verify gates that still work**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/dw4vhp-tauri/src
git commit -m "feat(shell): IPC commands for choose/analyze/summary/patch with progress events"
```

### Task 6: The backend seam (Backend, tauri.ts, mock.ts, context)

**Files:**
- Create: `src/ipc/backend.ts`, `src/ipc/tauri.ts`, `src/ipc/mock.ts`, `src/ipc/context.tsx`
- Test: `src/ipc/backend.test.ts`

**Interfaces:**
- Consumes: bindings (Task 2); `@tauri-apps/api/core` `invoke` + `event.listen`.
- Produces:

```ts
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
}
```

`context.tsx` exports `BackendProvider` (uses the tauri impl when `window.__TAURI_INTERNALS__` exists, else the mock) and `useBackend(): Backend` that throws outside a provider.

- [ ] **Step 1: Write the failing test**

`src/ipc/backend.test.ts` — drive `mock.ts` (which keeps an internal state: `mockBackend.setStatus`, and `startPatch` emits progress→done/failed through the registered handlers):

```ts
test("patch lifecycle delivers progress then terminal event", async () => {
  const backend = mockBackend();
  const progress: ProgressEvent[] = [];
  await backend.onPatchProgress((p) => progress.push(p));
  const failed = vi.fn();
  await backend.onPatchFailed(failed);
  let done: PatchOutcome | null = null;
  await backend.onPatchDone((o) => { done = o; });
  await backend.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
  expect(progress.length).toBeGreaterThan(0);
  expect(done).not.toBeNull();
  expect(failed).not.toHaveBeenCalled();
});

test("failure surfaces the engine message", async () => {
  const backend = mockBackend();
  backend.nextFailure = "the output file already exists — choose a new name, or allow overwriting it";
  const failed = vi.fn();
  await backend.onPatchFailed(failed);
  await backend.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
  expect(failed).toHaveBeenCalledWith("the output file already exists — choose a new name, or allow overwriting it");
});

test("unlisten stops handler delivery", async () => {
  const backend = mockBackend();
  const handler = vi.fn();
  const unlisten = await backend.onPatchProgress(handler);
  await unlisten();
  await backend.startPatch(presetPlan("VeryHardPlus"), "out.iso", false);
  expect(handler).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- backend`
Expected: FAIL

- [ ] **Step 3: Implement the seam**

`mock.ts`: same interface; `startPatch` synthesises 3 progress events then done (or the queued failure). `tauri.ts`: `invoke("choose_iso")` etc., `listen("patch-progress", …)` returning the unlisten fn; event payload cast to the binding types. `context.tsx` with lazy impl selection.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- backend`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ipc
git commit -m "feat(frontend): backend seam with Tauri impl and test mock"
```

### Task 7: Features — disc, presets, advanced, output (UI components)

**Files:**
- Create: `src/features/disc/DiscPicker.tsx`, `src/features/disc/useDisc.ts`, `src/features/presets/PresetCards.tsx`, `src/features/advanced/AdvancedPanel.tsx`, `src/features/output/OutputPanel.tsx`
- Modify: `src/app/App.tsx` (compose the four features)
- Test: `src/features/disc/useDisc.test.ts`, `src/features/disc/DiscPicker.test.tsx`, `src/features/presets/PresetCards.test.tsx`, `src/features/advanced/AdvancedPanel.test.tsx`, `src/features/output/OutputPanel.test.tsx`

**Interfaces:**
- Consumes: `usePlan` (Task 3), `Backend` via `useBackend` (Task 6), bindings.
- Produces: `useDisc(): { inputPath: string | null; status: DiscStatus | null; analyzing: boolean; pickDisc(): Promise<void>; setInputPath(path: string): Promise<void> }` — `pickDisc` calls `backend.chooseIso()`, then sets output via `backend.defaultOutput`, then auto-`analyze`s (automatic, as today). `PresetCards` renders one card per `PRESETS` entry except Custom (Custom appears as a card only when `state.preset === "Custom"`), highlighting the active one, `onClick = applyPreset(id)`. `AdvancedPanel` gets `{ state, editField, setSerialEnabled, setSerialText, summary }` and renders the multiplier grid (`STAT_NAMES` + hp/crit/para), crown colour dropdown with colour swatches, the five checkboxes (labels verbatim from the old egui app: "exclude destructibles", "exclude tripwire", "leave training-stage enemies vanilla", "practice buff", "force Very Hard"), and the serial checkbox + text field. `OutputPanel` gets `{ output, setOutput, overwrite, setOverwrite, busy, canPatch, onPatch, progress, log }`; shows the free-space note when a report exists, the attack-pin and collapse notes (ported from `form::attack_pin_note`/`collapse_note`: strings "attack: {pinned} of {live} live rows pinned at 32767" and "collapse: {rows} rows now carry their type's strongest row — every enemy of a type is identical, EXP included"), progress bar from `progress` events, and the log list.

- [ ] **Step 1: Write the failing tests**

Key assertions (one test file per component; behaviour, not markup):

- `useDisc.test.ts`: choosing a disc sets the path, then auto-analyzes; analyze result sets status.
- `PresetCards.test.tsx`: clicking "Very Hard Plus — Extreme" calls `applyPreset("Extreme")`; the active card has class `preset-active`; no Custom card when preset is a named one; Custom card present when Custom.
- `AdvancedPanel.test.tsx`: changing the hp field calls `editField("hpMult", value)`; crown dropdown offers the six colours in order; toggling "leave training-stage enemies vanilla" calls `editField("excludePractice", true)`.
- `OutputPanel.test.tsx`: shows the engine failure text from a `patch-failed` in the log; Patch button disabled unless `canPatch`; free-space note text contains the report size.
- `DiscPicker.test.tsx`: renders the status line with kind→colour class mapping (`status-ok`, `status-refused`, `status-unknown`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- --run src/features`
Expected: FAIL

- [ ] **Step 3: Implement the components**

Plain React + react-aria-components (`TextField`, `NumberField`, `Checkbox`, `Select`); styles via the theme.css variables; no fixed pixel widths anywhere (`max-width`, `flex`, `grid` with `minmax`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- --run src/features`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features src/app
git commit -m "feat(frontend): disc, preset cards, advanced panel, output/patch UI"
```

### Task 8: Responsive layout, theming, drag-drop, App composition

**Files:**
- Modify: `src/app/App.tsx`, `src/app/theme.css`, `src/main.tsx`
- Create: `src/app/layout.css`
- Test: `src/app/App.test.tsx` (extend)

**Interfaces:**
- Consumes: all four feature components (Task 7).
- Produces: App recomputes the live summary on every plan change: a `useEffect` over `state.plan` calling `backend.planSummary(state.plan)` and storing the result, passed to `AdvancedPanel`/`OutputPanel` as `summary`. This is the "live plan summary recalculated reactively" polish from spec §2.
- Produces: final window layout; `<body class="app">`; App wires `usePlan` once and passes it down; `patch-progress/done/failed` subscriptions live in App via `useBackend` and feed OutputPanel's progress/log; Tauri drag-drop of an ISO onto the window calls `useDisc.setInputPath` (Tauri 2: `getCurrentWindow().onDragDropEvent` in `src/ipc/tauri.ts`, exposed as `backend.onFileDropped(handler): Promise<() => void>` — add to the `Backend` interface and mock).

- [ ] **Step 1: Write the failing test**

Extend `App.test.tsx`:

```tsx
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
```

`mockMatchMedia` is a small helper stubbing `window.matchMedia` (jsdom lacks it).

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --run src/app`
Expected: FAIL

- [ ] **Step 3: Implement layout + theme + drag-drop**

`layout.css`: `main { display:grid; grid-template-columns: 1fr; }` and `@media (min-width: 900px) { main.layout-two-col { grid-template-columns: minmax(0,1fr) minmax(0,1fr); } }`; App picks the class from a `matchMedia` listener hook (`useMediaQuery`). Theme follows OS via `prefers-color-scheme` variables (no manual toggle — spec table's "light/dark following the OS"). Drag-drop: subscribe `onFileDropped`, filter to `.iso` paths.

- [ ] **Step 4: Run test to verify it passes; then all gates**

Run: `npm run typecheck && npm run lint && npm test && cargo test --workspace`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "feat(frontend): responsive two-pane layout, OS theme, ISO drag-drop"
```

### Task 9: Packaging and CI

**Files:**
- Delete: `crates/dw4vhp-gui/packaging/` (already gone with the crate in Task 1)
- Modify: `.github/workflows/ci-linux.yml`, `.github/workflows/ci-windows.yml`, `.github/workflows/release-linux.yml`, `.github/workflows/release-windows.yml`

**Interfaces:**
- Consumes: Tauri bundler (`tauri build`), the conf's `bundle.windows.wix.upgradeCode` from Task 4.

- [ ] **Step 1: Update CI workflows**

- `ci-linux.yml`: keep the apt install (add `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` if not already there), add a job/step running `npm ci && npm run typecheck && npm run lint && npm test`, then `cargo test --workspace`, then `cargo build -p dw4vhp-tauri` (system deps now present) and `cargo test -p dw4vhp-tauri --manifest-path crates/dw4vhp-tauri/Cargo.toml` if any Rust-side tests exist.
- `ci-windows.yml`: same frontend gates, then `cargo build -p dw4vhp-tauri`.
- `release-linux.yml`: replace the egui exe build with `npm ci && npm run build && cargo tauri build --bundles appimage` (publish the AppImage as today).
- `release-windows.yml`: replace `cargo build --release -p dw4vhp-gui` with `npm ci && npm run build && cargo tauri build --bundles msi`; publish the produced `dw4-veryhard-plus-<version>-windows-x86_64.msi` from `target/release/bundle/msi/` **and** the bare exe from `target/release/dw4-veryhard-plus.exe` (zip asset unchanged). Drop the WiX `dotnet tool` steps and the `packaging/msi.wxs` reference — the bundler's own WiX pass replaces them, carrying the pinned UpgradeCode.
- All four: rename every remaining `dw4vhp-gui`/`dw4-veryhard-plus.exe` path reference to the new crate; the binary name from Task 4 is `dw4-veryhard-plus` so the zip/asset names keep matching.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows
git commit -m "ci: build the Tauri app; MSI via the bundler, AppImage unchanged"
```

### Task 10: Docs and final verification

**Files:**
- Modify: `README.md` (only if it names the GUI framework or build steps), `docs/superpowers/specs/2026-10-02-tauri-gui-rewrite-design.md` (status → implemented)

- [ ] **Step 1: Update README build/dev instructions**

If `README.md` mentions building the GUI (check with `grep -n "gui\|cargo run" README.md`), update to: `npm ci`, `npm run dev` for the frontend, `cargo tauri dev` for the desktop app; `cargo tauri build` for distributables.

- [ ] **Step 2: Run every gate**

Run: `npm run typecheck && npm run lint && npm test && cargo test --workspace`
Expected: all PASS.

- [ ] **Step 3: Manual checklist (record results in the task notes)**

- `npm run dev` in a browser: mock backend drives the whole flow (choose disc via mock, presets flip Advanced, edit flips Custom, patch runs against the mock).
- Resize across 500 px → 1600 px: one column becomes two, nothing clips.
- Note honestly what could NOT be verified locally (Tauri crate compile, real ISO patch, MSI install) — those are CI/Windows follow-ups.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "docs: Tauri GUI build instructions, spec status"
```
