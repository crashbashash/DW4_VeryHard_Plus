# DW4 Very Hard Plus — Tauri GUI rewrite design

**Date:** 2026-10-02
**Status:** draft — awaiting approval
**Repo:** `github.com/crashbashash/DW4_VeryHard_Plus`

## 1. Why rewrite

The current `dw4vhp-gui` crate is an `eframe`/egui desktop window. Two complaints:

1. **Poor responsiveness to size changes.** egui's immediate-mode layout with
   fixed `desired_width(360.0)` text fields, a 420 px progress bar and a grid
   of drag values does not adapt to window resizing; content stays small in a
   large window and clips in a small one.
2. **Broken preset ↔ Advanced reactivity.** The `UiState::apply_preset` /
   `on_plan_edited` round-trip (selecting a preset should load its plan into the
   Advanced panel; editing any knob should flip the preset to Custom) works in
   isolation but the wiring through combobox change-detection in `app.rs` does
   not keep them in sync reliably.

The fix is a rewrite of the GUI layer only, in the style of **DW4orge**
(`github.com/crashbashash/DW4orge`): a Rust core, a Tauri shell, and a
React + TypeScript + Vite web frontend that is fully responsive by nature and
where reactivity is plain React state, not manual change-detection.

**The engine stays.** `dw4vhp-core` (plan, transform, inspect, patch, writer)
is untouched; every byte-level decision remains exactly as verified today.

## 2. What this is

A new GUI app with the same player-facing behaviour as today, plus polish:

| Existing behaviour (ported as-is) | Polish added |
| --- | --- |
| Pick an ISO (dialog **and** drag-drop onto the window) | Presets presented as selectable cards with short descriptions instead of a combobox |
| Automatic analysis on disc pick, with the verified/refused status line | Live plan summary recalculated reactively on every plan change (attack-pin note, collapse note, free-space note) |
| Preset → loads full plan into Advanced; any Advanced edit → preset flips to Custom | Live diff-style plan summary panel showing what will change (rows affected, crown colour preview) |
| Crown rank as a colour dropdown (0–5: none/green/blue/pink/white/yellow) | Crown colour swatches in the dropdown |
| Advanced panel: 12 stat multipliers + hp/crit/para, exclusions, practice buff, force Very Hard, serial rename | Responsive layout: single column that becomes two columns in a wide window; Advanced section collapses/expands |
| Output path defaulting to `<stem> [VeryHardPlus].iso`, overwrite checkbox | Patch progress as a proper progress bar driven by IPC events, not polling |
| Patch on a background worker with Phase/Copying/Writing/Verifying progress | Log panel scrollable, styled by severity |
| Analyze/Patch never block the UI thread | Light/dark theme following the OS, as DW4orge does |

## 3. Architecture

Follows DW4orge's three-layer split:

```text
crates/
  dw4vhp-core/          unchanged — the patch engine
  dw4vhp-tauri/         NEW — Tauri 2 shell (Rust): commands + IPC state
src/                    NEW — React + TypeScript + Vite frontend
```

### 3.1 Tauri shell (`crates/dw4vhp-tauri`)

A small crate, mirroring DW4orge's `src-tauri` (which is only 142 lines there):

- `state.rs` — holds the current `PatchPlan`, paths, and the in-flight worker
  handles; no engine call ever runs on the UI thread.
- `commands.rs` — Tauri commands:
  - `choose_iso()` — native open dialog (plugin-dialog), returns path or null
  - `choose_output(default_name)` — native save dialog
  - `analyze(path) -> AnalyzeResult` — runs `disc::inspect` on a worker thread,
    returns the `DiscReport`/error (serialised: copies, size, status text)
  - `start_patch(plan, input, output, overwrite)` — spawns the patch worker;
    emits `patch-progress` events (`Phase`, `done`, `total`); emits
    `patch-done` / `patch-failed` with the outcome (bytes, md5) or message
  - `plan_summary(report, plan) -> PlanSummary` — pure `transform` for the
    live summary; may run inline (it is cheap and pure)
- `bindings` are generated as hand-written TypeScript types in
  `src/bindings/` (same approach as DW4orge: small per-type files, one
  `index.ts`), because the surface is ~6 commands.

Workspace membership: `dw4vhp-tauri` is **excluded from the workspace members**
in the root `Cargo.toml` like DW4orge does (Tauri pulls in webkit2gtk/wry
system deps that would break the whole workspace gate); it is built by its own
CI job after installing system dependencies. `dw4vhp-gui` is deleted.

### 3.2 Frontend (`src/`, React 19 + TypeScript + Vite)

Structure copied from DW4orge's conventions:

```text
src/
  main.tsx
  app/              App shell, layout, theming
  features/
    disc/           ISO pick, drag-drop, status line, analysis state
    presets/        preset cards + the plan state machine
    advanced/       multiplier grid, toggles, crown colour, serial
    output/         output path, overwrite, patch button, progress, log
  ipc/
    backend.ts      the seam: one Backend interface
    tauri.ts        Tauri implementation (invoke + event listen)
    mock.ts         browser mock (for vitest and `vite dev` in a plain tab)
    context.tsx     React context providing the Backend
  bindings/         hand-written TS types (PatchPlan, Preset, PlanSummary, …)
```

**The seam.** Everything the UI asks of the shell goes through
`src/ipc/backend.ts` — `Backend { chooseIso, chooseOutput, analyze,
startPatch, planSummary, onProgress, onDone, onFailed }` with a mock
implementation for tests. No component imports `@tauri-apps/api` directly.

### 3.3 The reactivity fix (the actual point)

All plan state lives in **one React state object** (`useReducer` in
`features/presets/usePlan.ts`), not in Rust and not in a mutable egui struct:

- `applyPreset(preset)` — dispatch: replaces the whole plan with
  `preset.plan()` (from a bindings constant table mirroring `Preset::plan`),
  resets the serial controls, marks Custom unchanged.
- `editField(field, value)` — dispatch: sets one field, then
  `Preset::detect(plan)` (ported as a pure TS function over the same preset
  constant table) recomputes the preset label on the same render pass.

Because preset selection and plan edits are two actions on one store, the
Advanced panel and the preset selector can never disagree — that was the
egui bug's root cause. Rust stays the source of truth for what a preset
*is* (the constant table is generated from `dw4vhp-core` values and kept in
sync by a unit test comparing against `Preset::plan()` output over IPC in the
Tauri tests).

### 3.4 Responsive layout

- CSS with flex/grid and `clamp()`-based sizing; no fixed pixel widths.
- Single column ≤ 900 px; two columns (left: disc + presets; right:
  advanced + output) ≥ 900 px.
- The window gets `min-width`/`min-height` so nothing clips.

## 4. Tooling and quality gates

Same stack as DW4orge, pinned to the same major versions where possible:
Tauri 2, Vite, TypeScript strict, ESLint, Vitest + Testing Library
(jsdom), react-aria-components for accessible controls.

- `npm run typecheck`, `npm run lint`, `npm test` gate the frontend.
- `cargo test -p dw4vhp-tauri` covers command-level tests (mock-based
  analysis on the core's `testkit` discs where feasible) — but only in the
  Tauri CI job, since workspace exclusion applies.
- CI: the existing `ci-windows.yml`/`ci-linux.yml` and release workflows are
  updated to install system deps (webkit2gtk-4.1 on Linux, etc.), build the
  Tauri app, and the Windows release keeps shipping the **MSI** built from an
  updated `packaging/msi.wxs` (UpgradeCode kept, product name/binary path
  updated to the new executable). Linux release builds the AppImage as today.

## 5. What stays out of scope

- Any change to `dw4vhp-core` or to the bytes the patches write.
- A CLI over the Tauri commands (the repo has none today).
- Renaming the app or changing difficulty semantics/preset definitions.
- The old `dw4vhp-gui` crate's egui code — deleted, not maintained.

## 6. Risks

- **Linux build env**: wry/webkit2gtk system deps must exist for dev builds;
  mitigated by workspace exclusion + CI installing deps (same approach as
  DW4orge, which is proven here).
- **Preset constant duplication** between Rust and TS is a drift risk;
  mitigated by the sync test in §3.3.
- **Drag-drop**: Tauri 2 handles file drops; needs `dragDropEnabled` config
  and an event listener — verified during implementation.

## 7. Verification

- `cargo test --workspace` (core unchanged, must stay green).
- Frontend: typecheck, lint, vitest.
- Manual: resize the window across widths — no clipping, layout reflows.
  Select each preset — Advanced shows its plan; edit one knob — preset card
  flips to Custom; edit back — Custom persists until a preset is clicked.
- Packaging: build the MSI and install it; launch the app from the shortcut.
