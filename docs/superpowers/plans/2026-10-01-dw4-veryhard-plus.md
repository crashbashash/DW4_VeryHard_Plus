# DW4 Very Hard Plus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform desktop app that turns a player's own clean Digimon World 4 (USA) ISO into a much harder modded ISO.

**Architecture:** A Cargo workspace with a headless engine crate (`dw4vhp-core`) that inspects a disc, derives the stat ratio from the disc's own bytes, computes a new 649-row enemy table, and writes the result to a *copy* — plus a thin egui app crate (`dw4vhp-gui`) that only edits a `PatchPlan`. All byte-level logic lives in the engine and is testable without a GUI or a real disc.

**Tech Stack:** Rust (stable), `egui`/`eframe` for the GUI, `memchr` for the 665-copy search, `md-5` for output digests, `rfd` for native file dialogs, `thiserror` for errors. No Node, no Python, no webview.

**Spec:** `docs/superpowers/specs/2026-10-01-dw4-veryhard-plus-design.md`

## Global Constraints

Copy these verbatim into every task's working set; they override convenience.

- **No game data in the repo, ever** — no ISO, ELF, memory card, savestate or extract, however small. `.gitignore` covers `*.iso`, `*.elf`, `*.p2s`, `*.ps2`.
- **The input ISO is opened read-only and never written.** The output is always a separate file.
- **Rounding is `f64::round_ties_even()`, never `f64::round()`** — the reference implementation used Python's ties-to-even `round()`, and the golden md5s depend on it.
- **Every write is same-length and in place** on the output copy. Nothing grows, moves or is repacked.
- **Every write is gated on a precondition**; when a check fails the app writes nothing and says which check and what to do.
- **Player-facing messages name the fix** ("this disc is already modded — use your original ISO"), never just the fault.
- **All tests must skip cleanly, not fail, when the real discs are absent.** CI has no discs.
- **Retail constants that must not drift:** rows `649`; copies `665`; HP block `0x78D64C5`; stats block `0x78DACB9`; chargen block `0x78DA51D`; block sizes `2596 / 15576 / 1947`; destructible rows `370..=378`; tripwire rows `644, 645`; crown rank `5`; HP cap `400_000`; i16 cap `±32767`; crit/para cap `60`; difficulty vaddr `0x00371DDC`; boot ELF `SLUS_208.36`, length `4_541_744`.
- **Verification gate for every task:** `cargo fmt --all --check` && `cargo clippy --workspace --all-targets -- -D warnings` && `cargo test --workspace` all clean.
- **Lint hygiene, because CI runs `-D warnings`:** assert booleans as `assert!(x)` / `assert!(!x)`; build variants with struct-update syntax (`PatchPlan { force_very_hard: true, ..Default::default() }`) rather than `Default::default()` followed by field assignment; derive `PartialEq` and `Debug` on every type a test compares.
- Crate names and the public API in each task's **Interfaces** block are fixed; dependency versions are the implementer's choice (`cargo add <crate>`).

## Review Focus

The five non-obvious failure modes most likely to bite a real player. Each is pinned by a test in the task that owns the code.

1. **The output path is the input path** (`patch_file(same, same)`, or a player browsing to their only ISO for both). Must refuse before copying a single byte. → Task 9.
2. **The input file is read-only** — normal for disc dumps copied off Windows. Patching must still succeed, because the input is never opened for writing. → Task 9.
3. **An interrupted run left `<output>.part` behind.** A re-run must not treat it as an output, and a failed run must never leave a half-written file at the final path. → Task 9.
4. **A disc that is nearly right** — a PAL release, a re-dump, an already-modded disc. Must refuse with the specific reason and write nothing, rather than patch the wrong bytes. → Tasks 5 and 10.
5. **Paths with spaces and non-ASCII characters** in the directory or filename, which is the normal case on a Windows desktop. Must work end to end. → Task 10.

---

### Task 1: Workspace, error type, layout constants, write regions

**Files:**
- Create: `Cargo.toml` (workspace), `crates/dw4vhp-core/Cargo.toml`, `crates/dw4vhp-core/src/lib.rs`, `crates/dw4vhp-core/src/error.rs`, `crates/dw4vhp-core/src/layout.rs`, `crates/dw4vhp-core/src/region.rs`
- Create: `.gitignore`
- Test: `crates/dw4vhp-core/tests/layout.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Layout { pub hp_off: u64, pub stat_off: u64, pub byte_off: u64, pub rows: usize, pub expected_copies: usize }` with `Layout::retail()` and `Layout::mini(rows: usize, copies: usize)`
  - `pub enum Error { NotThisDisc, WrongRevision { block: &'static str, found: usize, expected: usize }, AlreadyModded { rarity_nonzero: usize }, ElfNotFound, UnexpectedInstruction { offset: u64, word: u32 }, SerialInvalid(String), OutputIsInput, OutputExists, NoSpace { needed: u64, free: u64 }, VerificationFailed(String), Io(std::io::Error) }` and `pub type Result<T>`
  - `pub struct WriteRegion { pub offset: u64, pub bytes: Vec<u8> }`

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/layout.rs
use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;

#[test]
fn retail_layout_matches_the_documented_constants() {
    let l = Layout::retail();
    assert_eq!((l.hp_off, l.stat_off, l.byte_off), (0x78D64C5, 0x78DACB9, 0x78DA51D));
    assert_eq!(l.rows, 649);
    assert_eq!(l.expected_copies, 665);
    assert_eq!(l.rows * 4, 2596);
    assert_eq!(l.rows * 24, 15576);
    assert_eq!(l.rows * 3, 1947);
}

#[test]
fn already_modded_message_tells_the_user_to_use_their_original() {
    let msg = Error::AlreadyModded { rarity_nonzero: 638 }.to_string();
    assert!(msg.contains("already"), "{msg}");
    assert!(msg.contains("original"), "{msg}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test layout`
Expected: FAIL — no such crate/workspace.

- [ ] **Step 3: Create the workspace and the three modules**

Workspace `Cargo.toml` with `members = ["crates/dw4vhp-core", "crates/dw4vhp-gui"]` and `resolver = "2"`; `dw4vhp-core` with `[lib] name = "dw4vhp_core"`. `.gitignore`: `/target`, `*.iso`, `*.elf`, `*.p2s`, `*.ps2` — no binary fixtures exist, because every test fixture is generated in code. `Layout::mini(rows, copies)` places the three blocks contiguously from `0x20000`, above the ISO9660 metadata area, so one fixture can carry both a table and a volume descriptor. `Error` derives `thiserror::Error` with the player-facing `#[error(...)]` strings from the spec §6.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test layout`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml .gitignore crates/dw4vhp-core
git commit -m "feat(core): workspace, error type, retail layout constants"
```

---

### Task 2: EnemyTable — read and encode the three blocks

**Files:**
- Create: `crates/dw4vhp-core/src/table.rs` (add `pub mod table;` to `lib.rs`)
- Test: `crates/dw4vhp-core/tests/table.rs`

**Interfaces:**
- Consumes: `Layout`, `Error`, `Result` from Task 1.
- Produces:
  - `pub struct EnemyTable { pub hp: Vec<i32>, pub stat: Vec<[i16; 12]>, pub crit: Vec<u8>, pub para: Vec<u8>, pub rarity: Vec<u8> }`
  - `impl EnemyTable { pub fn read(iso: &[u8], layout: &Layout) -> Result<Self>; pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool; pub fn blocks(&self) -> [Vec<u8>; 3] }`
  - `EnemyTable` derives `PartialEq, Debug, Clone`.

The encodings are the trap: HP is a flat i32 column (`hp[r]` at `hp_off + 4*r`); the twelve stats are **row-major i16** (`stat[r][c]` at `stat_off + 24*r + 2*c`); chargen is three u8 per row at `byte_off + 3*r`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/table.rs
use dw4vhp_core::layout::Layout;
use dw4vhp_core::table::EnemyTable;

fn mini_iso(rows: usize) -> Vec<u8> {
    let l = Layout::mini(rows, 3);
    let mut iso = vec![0u8; (l.byte_off as usize) + rows * 3];
    for r in 0..rows {
        iso[(l.hp_off as usize) + 4 * r..][..4].copy_from_slice(&((r as i32) * 100).to_le_bytes());
        for c in 0..12 {
            let v = ((r * 12 + c) as i16) - 5;
            iso[(l.stat_off as usize) + 24 * r + 2 * c..][..2].copy_from_slice(&v.to_le_bytes());
        }
        iso[(l.byte_off as usize) + 3 * r] = (r % 20) as u8;       // crit
        iso[(l.byte_off as usize) + 3 * r + 1] = (r % 7) as u8;    // para
        iso[(l.byte_off as usize) + 3 * r + 2] = if r == 1 { 5 } else { 0 };
    }
    iso
}

#[test]
fn reads_hp_stats_and_chargen_in_their_documented_encodings() {
    let l = Layout::mini(8, 3);
    let t = EnemyTable::read(&mini_iso(8), &l).unwrap();
    assert_eq!(t.len(), 8);
    assert_eq!(t.hp[3], 300);
    assert_eq!(t.stat[3][2], (3 * 12 + 2) as i16 - 5);
    assert_eq!(t.crit[3], 3);
    assert_eq!(t.para[3], 3);
    assert_eq!(t.rarity[1], 5);
    assert_eq!(t.rarity[0], 0);
}

#[test]
fn blocks_round_trip_and_have_the_documented_sizes() {
    let l = Layout::mini(649, 3);
    let t = EnemyTable::read(&mini_iso(649), &l).unwrap();
    let b = t.blocks();
    assert_eq!([b[0].len(), b[1].len(), b[2].len()], [2596, 15576, 1947]);
    let mut iso = mini_iso(649);
    iso[(l.hp_off as usize)..][..2596].copy_from_slice(&b[0]);
    iso[(l.stat_off as usize)..][..15576].copy_from_slice(&b[1]);
    iso[(l.byte_off as usize)..][..1947].copy_from_slice(&b[2]);
    let again = EnemyTable::read(&iso, &l).unwrap();
    assert_eq!(again.hp, t.hp);
    assert_eq!(again.stat, t.stat);
    assert_eq!(again.rarity, t.rarity);
}

#[test]
fn read_rejects_a_buffer_that_is_too_short() {
    let l = Layout::retail();
    assert!(EnemyTable::read(&vec![0u8; 64], &l).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test table`
Expected: FAIL — `table` module does not exist.

- [ ] **Step 3: Implement `EnemyTable::read` and `EnemyTable::blocks`**

Bounds-check the three regions against `iso.len()` first and return `Error::NotThisDisc` when short. `blocks()` is the exact inverse of `read`. The `mini_iso` test helper *is* the encoding contract, so keep it explicit — `hp` at `hp_off + 4*r`, `stat` at `stat_off + 24*r + 2*c`, chargen at `byte_off + 3*r`. Note the test uses `Layout::mini(649, 3)`, not `Layout::retail()`, so a round trip covers the real block sizes without allocating a 126 MB buffer.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test table`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/table.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/tests/table.rs
git commit -m "feat(core): enemy stat table read/encode with the documented block encodings"
```

---

### Task 3: RatioVector — derive the factors from the disc

**Files:**
- Create: `crates/dw4vhp-core/src/ratio.rs`
- Test: `crates/dw4vhp-core/tests/ratio.rs`

**Interfaces:**
- Consumes: `EnemyTable`.
- Produces:
  - `pub struct RatioVector { pub hp: f64, pub stat: [f64; 12], pub crit_delta: f64, pub para_delta: f64 }`
  - `impl RatioVector { pub fn derive(t: &EnemyTable) -> Self }`
  - `pub fn median4(v: [i32; 4]) -> f64`
  - `pub const REF_ORDINARY_ROWS: [usize; 4] = [0, 1, 2, 3];` and `pub const REF_TOP_ROW: usize = 391;`

Per spec §3.2: `hp = hp[391] / max(1, min(hp[0..3]))`; `stat[c] = stat[391][c] / min(stat[0..3][c])` or `1.0` when the denominator is 0; the crit/para deltas are `chargen[391] - median(chargen[0..3])` where the median of four values is the mean of the middle two.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/ratio.rs
use dw4vhp_core::ratio::{median4, RatioVector};
use dw4vhp_core::table::EnemyTable;

fn table_with(hp: &[(usize, i32)], stat: &[(usize, usize, i16)], chg: &[(usize, u8, u8)]) -> EnemyTable {
    let mut t = EnemyTable { hp: vec![0; 649], stat: vec![[0; 12]; 649], crit: vec![0; 649], para: vec![0; 649], rarity: vec![0; 649] };
    for &(r, v) in hp { t.hp[r] = v; }
    for &(r, c, v) in stat { t.stat[r][c] = v; }
    for &(r, a, b) in chg { t.crit[r] = a; t.para[r] = b; }
    t
}

#[test]
fn derives_the_documented_factors_from_the_reference_rows() {
    // authored reference values (e_goburi rows 0-3 vs the rank-5 row 391)
    let hp = [(0, 50), (1, 58), (2, 74), (3, 77), (391, 650)];
    let mut stat = Vec::new();
    for (r, atk, def) in [(0, 16, 90), (1, 18, 92), (2, 22, 95), (3, 24, 98), (391, 190, 194)] {
        stat.push((r, 0, atk));
        stat.push((r, 1, def));
    }
    let chg = [(0, 3, 3), (1, 4, 4), (2, 5, 5), (3, 6, 6), (391, 24, 24)];
    let v = RatioVector::derive(&table_with(&hp, &stat, &chg));
    assert_eq!(v.hp, 13.0);                     // 650 / 50
    assert!((v.stat[0] - 11.875).abs() < 1e-12); // 190 / 16
    assert!((v.stat[1] - 194.0 / 90.0).abs() < 1e-12);
    assert_eq!(v.crit_delta, 19.5);
    assert_eq!(v.para_delta, 19.5);
}

#[test]
fn median_of_four_is_the_mean_of_the_middle_two() {
    assert_eq!(median4([3, 4, 5, 6]), 4.5);
    assert_eq!(median4([10, 1, 4, 4]), 4.0);
}

#[test]
fn a_zero_denominator_yields_a_factor_of_one() {
    let v = RatioVector::derive(&table_with(&[(391, 650)], &[(391, 0, 190)], &[]));
    assert_eq!(v.hp, 650.0);        // max(1, 0) guards the HP denominator
    assert_eq!(v.stat[0], 1.0);     // column denominator 0 -> 1.0
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test ratio`
Expected: FAIL — `ratio` module does not exist.

- [ ] **Step 3: Implement `RatioVector::derive` and `median4`**

Work in `f64` throughout; do not round the factors.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test ratio`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/ratio.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/tests/ratio.rs
git commit -m "feat(core): derive the elite ratio vector from the disc's own reference rows"
```

---

### Task 4: PatchPlan, presets, the transform, and the summary

**Files:**
- Create: `crates/dw4vhp-core/src/plan.rs`
- Test: `crates/dw4vhp-core/tests/plan.rs`

**Interfaces:**
- Consumes: `EnemyTable`, `RatioVector`.
- Produces:
  - `pub struct PatchPlan { pub hp_mult: f64, pub stat_mult: [f64; 12], pub crit_mult: f64, pub para_mult: f64, pub crown_rank: u8, pub exclude_destructibles: bool, pub exclude_tripwire: bool, pub practice_buff: bool, pub force_very_hard: bool, pub serial: Option<String> }` with `impl Default` (all mults `1.0`, rank `5`, both exclusions and `practice_buff` `true`, `force_very_hard` `false`, `serial: None`).
  - `pub enum Preset { VeryHardPlus, Extreme, Custom }` with `pub fn plan(self) -> PatchPlan`, `pub fn detect(p: &PatchPlan) -> Preset`, `pub fn label(self) -> &'static str`.
  - `PatchPlan` derives `PartialEq, Debug, Clone`; `Preset` derives `PartialEq, Eq, Debug, Clone, Copy`; `PlanSummary` derives `PartialEq, Eq, Debug, Default, Clone`.
  - `pub const DESTRUCTIBLE_ROWS: std::ops::RangeInclusive<usize> = 370..=378;`
  - `pub const TRIPWIRE_ROWS: [usize; 2] = [644, 645];`
  - `pub const PRACTICE_MAP: [(usize, usize); 6] = [(638, 0), (639, 1), (640, 33), (641, 34), (642, 12), (643, 13)];` — `(destination, source)`, source read **after** scaling.
  - `pub struct PlanSummary { pub rows_changed: usize, pub attack_pinned: usize, pub hp_capped: usize, pub crit_pinned: usize, pub practice_rows: usize, pub elf_step: bool, pub serial_step: bool }`
  - `pub fn transform(authored: &EnemyTable, plan: &PatchPlan, ratio: &RatioVector) -> (EnemyTable, PlanSummary)`

The transform per spec §3.3: skip excluded rows; `round_ties_even` everywhere; clamp HP to `400_000`; clamp the twelve stats to `-32768..=32767`; clamp crit/para to `0..=60`; set `rarity = plan.crown_rank` on **every** handled row including the 69 empty ones; then apply `PRACTICE_MAP` by copying the whole row (hp, twelve stats, all three chargen bytes) from the already-computed table.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/plan.rs
use dw4vhp_core::plan::{transform, PatchPlan, Preset, DESTRUCTIBLE_ROWS, PRACTICE_MAP, TRIPWIRE_ROWS};
use dw4vhp_core::ratio::RatioVector;
use dw4vhp_core::table::EnemyTable;

fn blank() -> EnemyTable {
    EnemyTable { hp: vec![100; 649], stat: vec![[10; 12]; 649], crit: vec![2; 649], para: vec![2; 649], rarity: vec![0; 649] }
}

#[test]
fn default_plan_is_the_very_hard_plus_preset() {
    let p = PatchPlan::default();
    assert_eq!(Preset::detect(&p), Preset::VeryHardPlus);
    assert!(p.practice_buff && p.exclude_destructibles && p.exclude_tripwire);
    assert!(!p.force_very_hard && p.serial.is_none() && p.crown_rank == 5);
    assert!(Preset::Extreme.plan().force_very_hard);
    assert_eq!(Preset::detect(&Preset::Extreme.plan()), Preset::Extreme);
}

#[test]
fn exclusions_are_left_byte_identical_and_every_other_row_gets_the_crown() {
    let t = blank();
    let (out, s) = transform(&t, &PatchPlan::default(), &RatioVector { hp: 2.0, stat: [2.0; 12], crit_delta: 0.0, para_delta: 0.0 });
    for r in DESTRUCTIBLE_ROWS.chain(TRIPWIRE_ROWS) {
        assert_eq!((out.hp[r], out.rarity[r]), (100, 0), "row {r} must be untouched");
    }
    assert_eq!(out.rarity[0], 5);
    assert_eq!(out.rarity[300], 5);          // an empty row still gets the crown
    assert_eq!(s.rows_changed, 649 - 11);
}

#[test]
fn the_i16_clamp_pins_high_values_and_does_not_keep_the_authored_value() {
    let mut t = blank();
    t.stat[0][0] = 30_000;                   // 30000 * 2 overflows
    t.stat[1][0] = -30_000;                  // and the negative side clamps too
    let (out, s) = transform(&t, &PatchPlan::default(), &RatioVector { hp: 1.0, stat: [2.0; 12], crit_delta: 0.0, para_delta: 0.0 });
    assert_eq!(out.stat[0][0], 32767);
    assert_eq!(out.stat[1][0], -32768);
    assert_eq!(s.attack_pinned, 1);
}

#[test]
fn caps_apply_to_hp_and_to_crit_and_paralysis() {
    let mut t = blank();
    t.hp[0] = 300_000;
    t.crit[0] = 55;
    t.para[0] = 55;
    let (out, s) = transform(&t, &PatchPlan::default(), &RatioVector { hp: 13.0, stat: [1.0; 12], crit_delta: 19.5, para_delta: 19.5 });
    assert_eq!(out.hp[0], 400_000);
    assert_eq!((out.crit[0], out.para[0]), (60, 60));
    assert_eq!((s.hp_capped, s.crit_pinned), (1, 1));
}

#[test]
fn rounding_is_ties_to_even() {
    let mut t = blank();
    t.crit[0] = 1;                            // 1 + 19.5 = 20.5 -> 20 ties-to-even, not 21
    t.crit[1] = 2;                            // 2 + 19.5 = 21.5 -> 22
    let (out, _) = transform(&t, &PatchPlan::default(), &RatioVector { hp: 1.0, stat: [1.0; 12], crit_delta: 19.5, para_delta: 19.5 });
    assert_eq!(out.crit[0], 20);
    assert_eq!(out.crit[1], 22);
}

#[test]
fn the_practice_rows_become_an_ordinary_row_of_their_type() {
    let mut t = blank();
    t.hp[0] = 650; t.stat[0][0] = 190; t.crit[0] = 22;
    let (out, s) = transform(&t, &PatchPlan::default(), &RatioVector { hp: 1.0, stat: [1.0; 12], crit_delta: 0.0, para_delta: 0.0 });
    assert_eq!(out.hp[638], out.hp[0]);
    assert_eq!(out.stat[638], out.stat[0]);
    assert_eq!(out.crit[638], out.crit[0]);
    assert_eq!(s.practice_rows, PRACTICE_MAP.len());
    assert_eq!((out.hp[644], out.rarity[644]), (100, 0));   // the tripwire survives the buff
}

#[test]
fn multipliers_scale_the_derived_factors() {
    let t = blank();
    let ratio = RatioVector { hp: 13.0, stat: [2.0; 12], crit_delta: 10.0, para_delta: 10.0 };
    let mut p = PatchPlan { hp_mult: 0.5, crit_mult: 0.0, ..Default::default() };
    p.stat_mult[0] = 3.0;
    let (out, _) = transform(&t, &p, &ratio);
    assert_eq!(out.hp[0], 650);              // 100 * 6.5
    assert_eq!(out.stat[0][0], 60);         // 10 * 6.0
    assert_eq!(out.crit[0], 2);              // 2 + 0
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test plan`
Expected: FAIL — `plan` module does not exist.

- [ ] **Step 3: Implement `PatchPlan`, `Preset`, `transform`**

`Preset::detect` compares against the two built-in plans field by field and returns `Custom` when neither matches. Count `attack_pinned` as rows whose **unclamped** attack would have exceeded the i16 range, and `hp_capped`/`crit_pinned` likewise.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test plan`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/plan.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/tests/plan.rs
git commit -m "feat(core): patch plan, presets, transform and plan summary"
```

---

### Task 5: DiscReport — inspect a disc and refuse the wrong ones

**Files:**
- Create: `crates/dw4vhp-core/src/disc.rs`, `crates/dw4vhp-core/src/testkit.rs`
- Test: `crates/dw4vhp-core/tests/disc.rs`

**Interfaces:**
- Consumes: `Layout`, `EnemyTable`, `IsoFile` (Task 6 — declare `boot_elf`/`cnf` as `Option<IsoFile>` now and leave them `None` until Task 6 fills them in).
- Produces:
  - `pub fn count_copies(iso: &[u8], needle: &[u8]) -> usize` (use `memchr::memmem::find_iter`)
  - `pub struct DiscReport { pub size: u64, pub copies: [usize; 3], pub rarity_nonzero: usize, pub already_modded: bool, pub authored: EnemyTable, pub boot_elf: Option<IsoFile>, pub cnf: Option<IsoFile> }`
  - `pub fn inspect(path: &Path, layout: &Layout) -> Result<DiscReport>`
  - `pub mod testkit` — the fixture builders, public and documented as *not* part of the stable API, so the GUI crate's tests reuse them instead of duplicating them
Checks, in order, returning the specific error: the three blocks parse (else `NotThisDisc`); each block occurs exactly `expected_copies` times (else `WrongRevision`); more than 100 rows have `rarity != 0` ⇒ `AlreadyModded` (authored is 60, patched is 638), earlier than any write.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/disc.rs
use dw4vhp_core::disc::{count_copies, inspect};
use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::testkit;

#[test]
fn counts_every_occurrence() {
    let hay = b"abcabcabc";
    assert_eq!(count_copies(hay, b"abc"), 3);
    assert_eq!(count_copies(hay, b"zzz"), 0);
}

#[test]
fn accepts_a_clean_fixture() {
    let (dir, path, layout) = testkit::clean_disc_tempfile();
    let r = inspect(&path, &layout).unwrap();
    assert_eq!(r.copies, [3, 3, 3]);
    assert!(!r.already_modded);
    assert_eq!(r.authored.len(), 649);
    drop(dir);
}

#[test]
fn a_wrong_copy_count_is_a_revision_mismatch() {
    let (dir, path, _) = testkit::clean_disc_tempfile(); // the fixture writes 3 copies
    let err = inspect(&path, &Layout::mini(649, 665)).unwrap_err();
    assert!(matches!(err, Error::WrongRevision { .. }), "{err:?}");
    drop(dir);
}

#[test]
fn a_disc_with_crowns_already_written_is_already_modded() {
    let (dir, path, layout) = testkit::modded_disc_tempfile();
    let err = inspect(&path, &layout).unwrap_err();
    assert!(matches!(err, Error::AlreadyModded { .. }), "{err:?}");
    drop(dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test disc`
Expected: FAIL — `disc` module and `testkit` fixture do not exist.

- [ ] **Step 3: Implement `inspect` and create `testkit`**

Create `src/testkit.rs` with the fixture builders: `clean_disc_tempfile() -> (TempDir, PathBuf, Layout)`, `modded_disc_tempfile()`, and — added by later tasks — `iso_with_files`, `iso_with_elf`, `boot_elf`, `iso_with_elf_and_cnf`, `full_disc_tempfile`, `changed_byte_count`, `changed_offsets`, `elf_region_offset` and `serial_offsets`. **Every fixture uses 649 rows** at `Layout::mini(649, 3)` offsets, because the transform's row indices (391, 644) are fixed: only the offsets and the copy count are small. `clean_disc_tempfile` writes each of the three blocks three times at different offsets with `rarity = 0` on every row; `modded_disc_tempfile` sets `rarity = 5` on all rows. `inspect` may use `std::fs::read` for these small fixtures; the real disc is mapped with `memmap2` in Task 10.

Add `tempfile` and `memchr` as dependencies and re-export `testkit` from `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test disc`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/disc.rs crates/dw4vhp-core/src/testkit.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/tests/disc.rs
git commit -m "feat(core): disc inspection, copy counting and refusal detection"
```

---

### Task 6: iso9660 — find a file and rewrite a directory-record name

**Files:**
- Create: `crates/dw4vhp-core/src/iso9660.rs`
- Modify: `crates/dw4vhp-core/src/disc.rs` — populate the `boot_elf` and `cnf` fields that Task 5 left as `None`
- Test: `crates/dw4vhp-core/tests/iso9660.rs`

**Interfaces:**
- Consumes: `Error`, `Result`.
- Produces:
  - `pub struct IsoFile { pub record_offset: u64, pub name_offset: u64, pub name_len: usize, pub lba: u32, pub size: u32 }`
  - `pub fn find_file(iso: &[u8], name: &str) -> Result<IsoFile>`
  - `pub fn rewrite_name(iso: &mut [u8], file: &IsoFile, new_name: &str) -> Result<()>`

The primary volume descriptor is at `0x8000` (identifier `CD001`); the root directory record is at offset 156 of it and holds LBA + length as both-endian pairs (read the little-endian half). Walk one level of directory records; a record is `len, ext_attr_len, extent_lba(8), size(8), datetime(7), flags(1), unit, gap, vol_seq(4), name_len(1), name`. Names carry a `;1` version suffix — strip it when matching. `rewrite_name` refuses when `new_name.len() != file.name_len`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/iso9660.rs
use dw4vhp_core::testkit;
use dw4vhp_core::iso9660::{find_file, rewrite_name};

#[test]
fn finds_a_root_directory_file_and_rewrites_its_name_in_place() {
    let mut iso = testkit::iso_with_files(&[("SLUS_208.36;1", 0x1000, 4096), ("SYSTEM.CNF;1", 0x2000, 57)]);
    let f = find_file(&iso, "SLUS_208.36").unwrap();
    assert_eq!((f.lba, f.size), (0x1000, 4096));
    assert_eq!(f.name_len, 13);
    rewrite_name(&mut iso, &f, "SLUS_000.00;1").unwrap();
    assert_eq!(&iso[f.name_offset as usize..][..13], b"SLUS_000.00;1");
    assert!(find_file(&iso, "SLUS_208.36").is_err());          // the old name is gone
    assert!(find_file(&iso, "SLUS_000.00").is_ok());
}

#[test]
fn refuses_a_name_of_a_different_length() {
    let mut iso = testkit::iso_with_files(&[("SLUS_208.36;1", 0x1000, 4096)]);
    let f = find_file(&iso, "SLUS_208.36").unwrap();
    assert!(rewrite_name(&mut iso, &f, "X;1").is_err());
}

#[test]
fn a_missing_file_is_an_error() {
    let iso = testkit::iso_with_files(&[("SYSTEM.CNF;1", 0x2000, 57)]);
    assert!(find_file(&iso, "SLUS_208.36").is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test iso9660`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement `find_file`/`rewrite_name` and `testkit::iso_with_files`**

`testkit::iso_with_files` builds a minimal image: sector 16 = PVD with a root record pointing at a sector holding one record per supplied file, plus a `SYSTEM.CNF` payload when asked for. Directory records must be padded to even lengths and must not cross a sector boundary.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test iso9660`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/iso9660.rs crates/dw4vhp-core/src/disc.rs crates/dw4vhp-core/tests/iso9660.rs
git commit -m "feat(core): minimal ISO9660 lookup and same-length record rewriting"
```

---

### Task 7: elf — program headers, vaddr mapping, and the difficulty patch

**Files:**
- Create: `crates/dw4vhp-core/src/elf.rs`
- Test: `crates/dw4vhp-core/tests/elf.rs`

**Interfaces:**
- Consumes: `IsoFile`, `WriteRegion`, `Error`.
- Produces:
  - `pub struct Phdr { pub p_type: u32, pub p_offset: u32, pub p_vaddr: u32, pub p_filesz: u32 }`
  - `pub struct ElfInfo { pub iso_offset: u64, pub phdrs: Vec<Phdr> }` with `pub fn read(iso: &[u8], boot_elf: &IsoFile) -> Result<Self>` and `pub fn vaddr_to_iso_offset(&self, vaddr: u32) -> Option<u64>`
  - `pub mod mips { pub fn daddu(rd: u32, rs: u32, rt: u32) -> u32; pub fn addiu(rt: u32, rs: u32, imm: i32) -> u32; pub fn opcode(word: u32) -> u32; pub fn rs(word: u32) -> u32; pub fn rt(word: u32) -> u32; pub fn rd(word: u32) -> u32; pub fn funct(word: u32) -> u32; }`
  - `pub const DIFFICULTY_VADDR: u32 = 0x0037_1DDC;`
  - `pub fn difficulty_region(iso: &[u8], elf: &ElfInfo) -> Result<WriteRegion>`

The patch is one instruction (spec §4): verify the word at `DIFFICULTY_VADDR` decodes to `daddu rd=s0, rs=a0, rt=zero`, then replace it with `addiu rt=s0, rs=zero, imm=1`. Both words are computed from their fields; no instruction bytes are written into the source.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/elf.rs
use dw4vhp_core::testkit;
use dw4vhp_core::elf::{difficulty_region, mips, ElfInfo, DIFFICULTY_VADDR};
use dw4vhp_core::error::Error;

#[test]
fn the_two_instructions_are_encoded_from_their_fields() {
    assert_eq!(mips::daddu(16, 4, 0), 0x0080_802D);      // daddu s0, a0, zero
    assert_eq!(mips::addiu(16, 0, 1), 0x2410_0001);      // addiu s0, zero, 1
}

#[test]
fn decodes_the_fields_it_encoded() {
    let w = mips::daddu(16, 4, 0);
    assert_eq!((mips::opcode(w), mips::funct(w), mips::rd(w), mips::rs(w), mips::rt(w)), (0, 0x2D, 16, 4, 0));
}

#[test]
fn maps_a_vaddr_through_the_program_headers() {
    let iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    assert_eq!(elf.vaddr_to_iso_offset(0x0010_0000), Some(0x80 + 0x1000));
    assert!(elf.vaddr_to_iso_offset(0x0037_1DDC).is_some());
    assert_eq!(elf.vaddr_to_iso_offset(0x9000_0000), None);
}

#[test]
fn produces_a_four_byte_replacement_at_the_mapped_offset() {
    let iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    let r = difficulty_region(&iso, &elf).unwrap();
    assert_eq!(r.bytes.len(), 4);
    assert_eq!(r.bytes, 0x2410_0001u32.to_le_bytes());
    assert_eq!(r.offset, elf.vaddr_to_iso_offset(DIFFICULTY_VADDR).unwrap());
}

#[test]
fn refuses_when_the_instruction_is_not_what_it_expects() {
    let mut iso = testkit::iso_with_elf();
    let elf = ElfInfo::read(&iso, &testkit::boot_elf(&iso)).unwrap();
    let off = elf.vaddr_to_iso_offset(DIFFICULTY_VADDR).unwrap() as usize;
    iso[off..off + 4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
    assert!(matches!(difficulty_region(&iso, &elf).unwrap_err(), Error::UnexpectedInstruction { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test elf`
Expected: FAIL — module and fixture do not exist.

- [ ] **Step 3: Implement `elf.rs` and `testkit::iso_with_elf`**

`ElfInfo::read` checks the `\x7fELF` magic, reads `e_phoff`/`e_phentsize`/`e_phnum` from the header, and keeps `PT_LOAD` entries (`p_type == 1`). `vaddr_to_iso_offset` scans those entries and returns `iso_offset + p_offset + (vaddr - p_vaddr)` when `p_vaddr <= vaddr < p_vaddr + p_filesz`. `testkit::iso_with_elf` extends the Task 6 image with a `SLUS_208.36` payload: a 64-byte ELF header, two program headers at `0x34`, `PT_LOAD vaddr 0x100000 / off 0x80 / filesz 0x2000`, and `daddu s0,a0,zero` planted at the file offset corresponding to `0x00371DDC`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test elf`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/elf.rs crates/dw4vhp-core/src/testkit.rs crates/dw4vhp-core/tests/elf.rs
git commit -m "feat(core): ELF program headers, vaddr mapping and the one-instruction difficulty patch"
```

---

### Task 8: serial — validate the name and produce the two rewrites

**Files:**
- Create: `crates/dw4vhp-core/src/serial.rs`
- Test: `crates/dw4vhp-core/tests/serial.rs`

**Interfaces:**
- Consumes: `IsoFile`, `WriteRegion`, `Error`.
- Produces:
  - `pub const DEFAULT_SERIAL: &str = "SLUS_000.00";`
  - `pub fn validate_serial(s: &str) -> Result<()>`
  - `pub fn serial_regions(iso: &[u8], boot_elf: &IsoFile, new_serial: &str) -> Result<Vec<WriteRegion>>`

Two same-length regions (spec §5): the 13-byte ISO9660 record name becomes `"{new_serial};1"`, and the 11-byte serial inside `SYSTEM.CNF`'s `BOOT2 = cdrom0:\...;1` line is replaced in place. Shape: 11 characters matching `[A-Z0-9]{4}_[0-9]{3}\.[0-9]{2}`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/serial.rs
use dw4vhp_core::testkit;
use dw4vhp_core::iso9660::find_file;
use dw4vhp_core::serial::{serial_regions, validate_serial, DEFAULT_SERIAL};

#[test]
fn accepts_the_eleven_character_shape_and_rejects_everything_else() {
    assert!(validate_serial(DEFAULT_SERIAL).is_ok());
    assert!(validate_serial("SLUS_208.36").is_ok());
    for bad in ["", "SLUS_208.3", "slus_208.36", "SLUS-208.36", "SLUS_2083.36", "SLUS_208.366"] {
        assert!(validate_serial(bad).is_err(), "{bad} should be rejected");
    }
}

#[test]
fn produces_two_same_length_regions_covering_the_record_and_the_boot_line() {
    let iso = testkit::iso_with_elf_and_cnf("SLUS_208.36");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    let rs = serial_regions(&iso, &elf, DEFAULT_SERIAL).unwrap();
    assert_eq!(rs.len(), 2);
    assert_eq!(rs[0].bytes, b"SLUS_000.00;1");
    assert_eq!(rs[1].bytes, b"SLUS_000.00");
    for r in &rs {
        let old = &iso[r.offset as usize..r.offset as usize + r.bytes.len()];
        assert_ne!(old, &r.bytes[..]);
        assert!(old.len() == r.bytes.len());          // same length: nothing moves
    }
}

#[test]
fn an_invalid_serial_never_produces_regions() {
    let iso = testkit::iso_with_elf_and_cnf("SLUS_208.36");
    let elf = find_file(&iso, "SLUS_208.36").unwrap();
    assert!(serial_regions(&iso, &elf, "nope").is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test serial`
Expected: FAIL — module and fixture do not exist.

- [ ] **Step 3: Implement `serial.rs` and `testkit::iso_with_elf_and_cnf`**

Locate the boot line by searching the `SYSTEM.CNF` payload for `BOOT2` then for the old serial; error with `Error::NotThisDisc` if either is missing. Do not hardcode the old serial — read the current 11 characters from the directory record.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test serial`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/serial.rs crates/dw4vhp-core/src/testkit.rs crates/dw4vhp-core/tests/serial.rs
git commit -m "feat(core): disc serial validation and the two same-length rewrites"
```

---

### Task 9: writer — copy, apply, verify, and the safety properties

**Files:**
- Create: `crates/dw4vhp-core/src/writer.rs`
- Test: `crates/dw4vhp-core/tests/writer.rs`

**Interfaces:**
- Consumes: `WriteRegion`, `Error`.
- Produces:
  - `pub enum Phase { Inspecting, Copying, Writing, Verifying }`
  - `pub struct Progress { pub phase: Phase, pub done: u64, pub total: u64 }`
  - `pub struct WriteOptions { pub overwrite: bool }`
  - `pub struct WriteOutcome { pub md5: String, pub bytes: u64 }`
  - `pub fn ensure_space(needed: u64, free: u64) -> Result<()>`
  - `Phase` derives `PartialEq, Eq, Debug, Clone, Copy`.
  - `pub fn write_output(input: &Path, output: &Path, regions: &[WriteRegion], opts: &WriteOptions, progress: &mut dyn FnMut(Progress), verify: &dyn Fn(&Path) -> Result<()>) -> Result<WriteOutcome>`

Behaviour: canonicalise both paths and refuse with `Error::OutputIsInput` when equal, and check free space with `ensure_space`, both **before anything is created or truncated**; refuse an existing output with `Error::OutputExists` unless `opts.overwrite`; stream the input to `"<output>.part"` in 8 MiB chunks reporting `Phase::Copying` progress (creating or truncating the part file, which replaces a stale one); reopen the part file and apply the regions (`Phase::Writing`), `sync_all`, then call `verify(&part_path)` — on failure remove the part file and return the error, leaving nothing at `output`; on success `fs::rename` the part onto `output` and return the md5 of the final file.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/writer.rs
use dw4vhp_core::error::Error;
use dw4vhp_core::region::WriteRegion;
use dw4vhp_core::writer::{write_output, Phase, WriteOptions};
use std::io::Write;

fn fixture(dir: &tempfile::TempDir, bytes: &[u8]) -> std::path::PathBuf {
    let p = dir.path().join("in.iso");
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(bytes).unwrap();
    p
}
fn noop(_: dw4vhp_core::writer::Progress) {}
fn ok(_: &std::path::Path) -> dw4vhp_core::error::Result<()> { Ok(()) }

#[test]
fn refuses_when_the_destination_has_less_space_than_the_copy_needs() {
    assert!(dw4vhp_core::writer::ensure_space(1_448_902_656, 1_000).is_err());
    assert!(dw4vhp_core::writer::ensure_space(1_448_902_656, 2_000_000_000).is_ok());
}

#[test]
fn copies_the_input_and_applies_regions_without_touching_the_input() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let out = dir.path().join("out.iso");
    let r = write_output(&input, &out, &[WriteRegion { offset: 2, bytes: b"XY".to_vec() }], &WriteOptions { overwrite: false }, &mut noop, &ok).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"01XY456789");
    assert_eq!(std::fs::read(&input).unwrap(), b"0123456789");     // input untouched
    assert_eq!(r.bytes, 10);
    assert_eq!(r.md5.len(), 32);
    assert!(!dir.path().join("out.iso.part").exists());
}

#[test]
fn refuses_when_the_output_is_the_input() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let err = write_output(&input, &input.clone(), &[], &WriteOptions { overwrite: true }, &mut noop, &ok).unwrap_err();
    assert!(matches!(err, Error::OutputIsInput), "{err:?}");
    assert_eq!(std::fs::read(&input).unwrap(), b"0123456789");     // still intact
}

#[test]
fn refuses_an_existing_output_unless_asked_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let out = dir.path().join("out.iso");
    std::fs::write(&out, b"existing").unwrap();
    assert!(matches!(write_output(&input, &out, &[], &WriteOptions { overwrite: false }, &mut noop, &ok).unwrap_err(), Error::OutputExists));
    write_output(&input, &out, &[], &WriteOptions { overwrite: true }, &mut noop, &ok).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"0123456789");
}

#[test]
fn works_when_the_input_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let mut perms = std::fs::metadata(&input).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&input, perms).unwrap();
    let out = dir.path().join("out.iso");
    write_output(&input, &out, &[], &WriteOptions { overwrite: false }, &mut noop, &ok).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"0123456789");
}

#[test]
fn a_stale_part_file_is_replaced_not_trusted() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let out = dir.path().join("out.iso");
    std::fs::write(dir.path().join("out.iso.part"), b"garbage-from-an-interrupted-run").unwrap();
    write_output(&input, &out, &[], &WriteOptions { overwrite: false }, &mut noop, &ok).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"0123456789");
    assert!(!dir.path().join("out.iso.part").exists());
}

#[test]
fn a_failed_verification_leaves_nothing_at_the_output_path() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let out = dir.path().join("out.iso");
    let bad = |_: &std::path::Path| Err(Error::VerificationFailed("nope".into()));
    let err = write_output(&input, &out, &[], &WriteOptions { overwrite: false }, &mut noop, &bad).unwrap_err();
    assert!(matches!(err, Error::VerificationFailed(_)));
    assert!(!out.exists(), "no half-written output may remain");
    assert!(!dir.path().join("out.iso.part").exists(), "the part file must be cleaned up");
}

#[test]
fn reports_the_copy_and_write_phases() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(&dir, b"0123456789");
    let out = dir.path().join("out.iso");
    let mut phases = Vec::new();
    let mut rec = |p: dw4vhp_core::writer::Progress| phases.push(p.phase);
    write_output(&input, &out, &[WriteRegion { offset: 0, bytes: b"Z".to_vec() }], &WriteOptions { overwrite: false }, &mut rec, &ok).unwrap();
    assert!(phases.contains(&Phase::Copying));
    assert!(phases.contains(&Phase::Writing));
    assert!(phases.contains(&Phase::Verifying));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test writer`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement `write_output`**

Use `std::fs::canonicalize` on the input and on the output's parent + file name for the identity check (the output may not exist yet). Query free space with `fs4::available_space` on the output's parent directory — it supports Windows and Linux — and pass it through `ensure_space`, whose comparison is unit-tested. Run the identity, space and exists checks in that order and before opening anything for writing.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test writer`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/writer.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/tests/writer.rs
git commit -m "feat(core): copy-and-patch writer with input protection, atomic rename and verification"
```

---

### Task 10: patch_file — the composition entry point

**Files:**
- Create: `crates/dw4vhp-core/src/patch.rs`
- Test: `crates/dw4vhp-core/tests/patch.rs`

**Interfaces:**
- Consumes: everything above.
- Produces:
  - `pub struct PatchOptions { pub overwrite: bool }`
  - `pub struct PatchOutcome { pub md5: String, pub bytes: u64, pub summary: PlanSummary }`
  - `pub fn plan_regions(iso: &[u8], report: &DiscReport, plan: &PatchPlan, layout: &Layout) -> Result<(Vec<WriteRegion>, PlanSummary)>`
  - `pub fn patch_file(input: &Path, output: &Path, plan: &PatchPlan, opts: &PatchOptions, progress: &mut dyn FnMut(Progress)) -> Result<PatchOutcome>` — the app's entry point, using `Layout::retail()`
  - `pub fn patch_file_with_layout(input: &Path, output: &Path, plan: &PatchPlan, opts: &PatchOptions, layout: &Layout, progress: &mut dyn FnMut(Progress)) -> Result<PatchOutcome>` — the seam the tests and the GUI worker use; `patch_file` delegates to it, and the verifier re-inspects with the same layout

`patch_file`: `inspect` → derive ratio → `transform` → `plan_regions` (three table regions per copy × 665, plus the ELF region when `force_very_hard`, plus the serial regions when `serial` is set) → `write_output` with a verifier that re-runs `inspect` on the written file and confirms the copy counts and the expected crown rank.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/patch.rs
use dw4vhp_core::error::Error;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::patch::{patch_file_with_layout, PatchOptions};
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::testkit;
use std::path::Path;

fn run(layout: &Layout, input: &Path, out: &Path, plan: &PatchPlan) {
    patch_file_with_layout(input, out, plan, &PatchOptions { overwrite: false }, layout, &mut |_| {}).unwrap();
}

#[test]
fn patches_a_clean_disc_and_reports_the_work_it_did() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let before = std::fs::read(&input).unwrap();
    let out = dir.path().join("out.iso");
    let o = patch_file_with_layout(&input, &out, &PatchPlan::default(), &PatchOptions { overwrite: false }, &layout, &mut |_| {}).unwrap();
    assert_eq!(o.summary.rows_changed, 649 - 11);
    assert!(o.summary.practice_rows > 0);
    assert!(!o.summary.elf_step && !o.summary.serial_step);
    assert_eq!(std::fs::read(&input).unwrap(), before, "the input must be untouched");
    assert!(testkit::changed_byte_count(&input, &out) > 0);
}

#[test]
fn the_elf_step_changes_exactly_four_bytes_and_only_when_enabled() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let plan = PatchPlan { force_very_hard: true, ..Default::default() };
    let out = dir.path().join("extreme.iso");
    run(&layout, &input, &out, &plan);
    let elf = testkit::elf_region_offset(&input).unwrap();
    assert_eq!(testkit::changed_offsets(&input, &out), vec![elf]);
    assert_eq!(std::fs::read(&out).unwrap()[elf as usize..elf as usize + 4], 0x2410_0001u32.to_le_bytes());
}

#[test]
fn the_serial_step_changes_only_the_two_intended_ranges() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let plan = PatchPlan { serial: Some("SLUS_000.00".into()), ..Default::default() };
    let out = dir.path().join("serial.iso");
    run(&layout, &input, &out, &plan);
    let (a, b) = testkit::serial_offsets(&input).unwrap();
    let offsets = testkit::changed_offsets(&input, &out);
    assert!(offsets.iter().all(|o| (a..a + 13).contains(o) || (b..b + 11).contains(o)), "{offsets:?}");
}

#[test]
fn paths_with_spaces_and_non_ascii_characters_work() {
    let (dir, input, layout) = testkit::full_disc_tempfile();
    let sub = dir.path().join("Dígimon Wörld 4 — saves");
    std::fs::create_dir_all(&sub).unwrap();
    let out = sub.join("Very Hard Plus!.iso");
    run(&layout, &input, &out, &PatchPlan::default());
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn refuses_an_already_modded_disc_and_writes_nothing() {
    let (dir, input, layout) = testkit::modded_disc_tempfile();
    let out = dir.path().join("out.iso");
    let err = patch_file_with_layout(&input, &out, &PatchPlan::default(), &PatchOptions { overwrite: false }, &layout, &mut |_| {}).unwrap_err();
    assert!(matches!(err, Error::AlreadyModded { .. }));
    assert!(!out.exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-core --test patch`
Expected: FAIL — `patch` module and `full_disc_tempfile` do not exist.

- [ ] **Step 3: Implement `plan_regions`/`patch_file` and `testkit::full_disc_tempfile`**

`testkit::full_disc_tempfile()` composes the Task 6/7/8 fixtures into one image: a `Layout::mini(649, 3)` table with 3 copies, a PVD/root directory holding `SLUS_208.36;1` and `SYSTEM.CNF;1`, an ELF with the authored `daddu` planted at the file offset mapping to `0x00371DDC`, and a `BOOT2 = cdrom0:\SLUS_208.36;1` line. Alongside it add the assert helpers: `changed_byte_count(&Path, &Path) -> usize`, `changed_offsets(&Path, &Path) -> Vec<u64>`, `elf_region_offset(&Path) -> Result<u64>` and `serial_offsets(&Path) -> Result<(u64, u64)>`.

For the real disc, read the file through `memmap2` (read-only) because it is 1.4 GB, and keep the copies search on the mapped bytes.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-core --test patch`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-core/src/patch.rs crates/dw4vhp-core/src/lib.rs crates/dw4vhp-core/src/testkit.rs crates/dw4vhp-core/tests/patch.rs
git commit -m "feat(core): patch_file composition with table, elf and serial steps"
```

---

### Task 11: Golden tests against the real discs

**Files:**
- Create: `crates/dw4vhp-core/tests/golden.rs`

**Interfaces:**
- Consumes: `patch_file`, `Layout`, `PatchPlan`.
- Produces: nothing (test-only).

Environment: `DW4_GOLDEN_SHIPPED_ISO`, `DW4_GOLDEN_BLOCKS_BIN`, `DW4_GOLDEN_BLOCKS_JSON`. If any is unset, the test prints a skip line and returns `Ok(())` — CI never has these.

**The chain was validated during planning**, so these md5s are known-reachable with the algorithm in Tasks 3–4:

| Step | Preset | Expected md5 |
|---|---|---|
| 1 | revert the shipped disc with the backup | `3185f04230b1dabd853db750e4b51108` |
| 2 | default (`Very Hard Plus`) | `df426b9ff17c9e8218779f8ff1ecf524` |
| 3 | default with `practice_buff = false` | `f760258575f4fe2de05a9930e0a6b396` |
| 4 | default with `serial = Some(DEFAULT_SERIAL)` | only the two serial ranges differ |
| 5 | default with `force_very_hard = true` | exactly four bytes differ |

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-core/tests/golden.rs
use dw4vhp_core::layout::Layout;
use dw4vhp_core::patch::{patch_file, PatchOptions};
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::serial::DEFAULT_SERIAL;
use std::path::{Path, PathBuf};

fn env_paths() -> Option<(PathBuf, PathBuf, PathBuf)> {
    let get = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let (a, b, c) = (get("DW4_GOLDEN_SHIPPED_ISO"), get("DW4_GOLDEN_BLOCKS_BIN"), get("DW4_GOLDEN_BLOCKS_JSON"));
    match (a, b, c) {
        (Some(a), Some(b), Some(c)) => Some((a, b, c)),
        _ => { eprintln!("SKIP: golden discs not configured"); None }
    }
}

fn md5_file(p: &Path) -> String { /* stream md5 */ }

#[test]
fn golden_chain_reproduces_the_shipped_builds() {
    let Some((shipped, blocks_bin, blocks_json)) = env_paths() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let pristine = tmp.path().join("pristine.iso");

    // step 1: revert the shipped disc with the recorded regions -> the pristine master
    revert_with_backup(&shipped, &pristine, &blocks_bin, &blocks_json);
    assert_eq!(md5_file(&pristine), "3185f04230b1dabd853db750e4b51108");

    // step 2: the default preset reproduces the practice-buff disc
    let out = tmp.path().join("default.iso");
    patch_file(&pristine, &out, &PatchPlan::default(), &PatchOptions { overwrite: false }, &mut |_| {}).unwrap();
    assert_eq!(md5_file(&out), "df426b9ff17c9e8218779f8ff1ecf524");

    // step 3: practice buff off reproduces the shipped v5b4 disc
    let no_practice = PatchPlan { practice_buff: false, ..Default::default() };
    let out3 = tmp.path().join("v5b4.iso");
    patch_file(&pristine, &out3, &no_practice, &PatchOptions { overwrite: false }, &mut |_| {}).unwrap();
    assert_eq!(md5_file(&out3), "f760258575f4fe2de05a9930e0a6b396");

    // step 4: the serial step touches two ranges and nothing else
    let serial = PatchPlan { serial: Some(DEFAULT_SERIAL.into()), ..Default::default() };
    let out4 = tmp.path().join("serial.iso");
    patch_file(&pristine, &out4, &serial, &PatchOptions { overwrite: false }, &mut |_| {}).unwrap();
    let changed = testkit::changed_offsets(&out, &out4);
    let (a, b) = testkit::serial_offsets(&out).unwrap();
    assert!(changed.iter().all(|o| (a..a + 13).contains(o) || (b..b + 11).contains(o)));

    // step 5: the elf step changes exactly four bytes
    let extreme = PatchPlan { force_very_hard: true, ..Default::default() };
    let out5 = tmp.path().join("extreme.iso");
    patch_file(&pristine, &out5, &extreme, &PatchOptions { overwrite: false }, &mut |_| {}).unwrap();
    assert_eq!(testkit::changed_offsets(&out, &out5), vec![testkit::elf_region_offset(&out).unwrap()]);
}
```

Add the small helpers `revert_with_backup`, `md5_file` and `env_paths` in the same file. `revert_with_backup(shipped, pristine, blocks_bin, blocks_json)` reads `shipped` and writes a **new** file at `pristine` with the recorded regions restored — the shipped disc is only ever read. It mirrors `patch_iso.py --revert`: parse `{"writes": [[offset, len], ...]}`, walk the backup blob in order, write each region.

These files are 1.4 GB, so everything here streams: `md5_file` reads in 4 MiB chunks, `revert_with_backup` maps the shipped disc read-only with `memmap2`, and `testkit::changed_offsets` walks the two files chunk by chunk rather than reading either into memory.

- [ ] **Step 2: Run it with the real discs (they exist in this workspace)**

Run:
```bash
DW4_GOLDEN_SHIPPED_ISO="/workspace/Decomp/DW4/dw4_yellowcrown/Digimon World 4 (USA) [yellowcrown-v5].iso" \
DW4_GOLDEN_BLOCKS_BIN="/workspace/Decomp/DW4/dw4_yellowcrown/iso_orig_blocks.bin" \
DW4_GOLDEN_BLOCKS_JSON="/workspace/Decomp/DW4/dw4_yellowcrown/iso_orig_blocks.json" \
cargo test -p dw4vhp-core --test golden -- --nocapture
```
Expected: FAIL until Tasks 3–4 are byte-exact; then PASS. This test is the acceptance gate for the whole engine — a failure means the transform diverges from the reference (check ties-to-even rounding first).

- [ ] **Step 3: Run without the env vars and confirm it skips**

Run: `cargo test -p dw4vhp-core --test golden`
Expected: PASS with a `SKIP` line, not a failure.

- [ ] **Step 4: Commit**

```bash
git add crates/dw4vhp-core/tests/golden.rs
git commit -m "test(core): golden chain reproducing both shipped discs byte-for-byte"
```

---

### Task 12: GUI form logic (pure and unit-tested)

**Files:**
- Create: `crates/dw4vhp-gui/Cargo.toml`, `crates/dw4vhp-gui/src/main.rs`, `crates/dw4vhp-gui/src/form.rs`
- Test: `crates/dw4vhp-gui/tests/form.rs`

**Interfaces:**
- Consumes: `PatchPlan`, `Preset`, `PlanSummary`, `DiscReport`, `Error`, `DEFAULT_SERIAL`.
- Produces:
  - `pub struct UiState { pub input: Option<PathBuf>, pub output: PathBuf, pub preset: Preset, pub plan: PatchPlan, pub overwrite: bool }` with `pub fn on_plan_edited(&mut self)` (re-detects the preset, so any edit selects `Custom`)
  - `UiState` derives `Debug, Default`; `StatusKind` derives `PartialEq, Eq, Debug`; `StatusLine` derives `PartialEq, Eq, Debug`
  - `pub fn default_output_path(input: &Path) -> PathBuf` → same directory, stem + ` [VeryHardPlus].iso`
  - `pub enum StatusKind { Ok, Refused, Unknown }`
  - `pub struct StatusLine { pub kind: StatusKind, pub text: String }`
  - `pub fn status_for(r: &Result<DiscReport>) -> StatusLine` — always `Err(AlreadyModded)` → `Refused` with the original-ISO advice; `WrongRevision`/`NotThisDisc` → `Refused` naming NTSC-U `SLUS_208.36`; success → `Ok` naming the serial and the copy counts.
  - `pub fn attack_pin_note(summary: &PlanSummary) -> Option<String>` → `Some("attack: 337 of 580 live rows pinned at 32767")` when `attack_pinned > 0`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-gui/tests/form.rs
use dw4vhp_gui::form::{default_output_path, status_for, StatusKind, UiState};
use dw4vhp_core::error::Error;
use dw4vhp_core::plan::Preset;
use std::path::{Path, PathBuf};

#[test]
fn the_default_output_name_keeps_the_directory_and_marks_the_suffix() {
    let p = default_output_path(Path::new("/games/Digimon World 4 (USA).iso"));
    assert_eq!(p, PathBuf::from("/games/Digimon World 4 (USA) [VeryHardPlus].iso"));
}

#[test]
fn editing_any_knob_switches_the_preset_to_custom() {
    let mut s = UiState { preset: Preset::VeryHardPlus, ..Default::default() };
    s.plan.crown_rank = 3;
    s.on_plan_edited();
    assert_eq!(s.preset, Preset::Custom);
}

#[test]
fn an_already_modded_disc_is_refused_with_the_fix() {
    let l = status_for(&Err(Error::AlreadyModded { rarity_nonzero: 638 }));
    assert_eq!(l.kind, StatusKind::Refused);
    assert!(l.text.contains("original"), "{}", l.text);
}

#[test]
fn a_wrong_revision_is_refused_and_names_the_supported_release() {
    let l = status_for(&Err(Error::WrongRevision { block: "HPMAX", found: 1, expected: 665 }));
    assert_eq!(l.kind, StatusKind::Refused);
    assert!(l.text.contains("SLUS_208.36"), "{}", l.text);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-gui --test form`
Expected: FAIL — crate does not exist.

- [ ] **Step 3: Implement the gui crate's `form.rs`**

`UiState::default()` uses `PatchPlan::default()`, `Preset::VeryHardPlus` and an empty output path. Keep this module free of any `egui` import so it stays unit-testable.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p dw4vhp-gui --test form`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/dw4vhp-gui
git commit -m "feat(gui): pure form logic for presets, output naming and status lines"
```

---

### Task 13: GUI app and the background worker

**Files:**
- Create: `crates/dw4vhp-gui/src/app.rs`, `crates/dw4vhp-gui/src/worker.rs`; modify `crates/dw4vhp-gui/src/main.rs`
- Test: `crates/dw4vhp-gui/tests/worker.rs`

**Interfaces:**
- Consumes: `UiState`, `form::*`, `patch_file`, `patch_file_with_layout`, `inspect`, `Progress`, `PatchPlan`, `Layout`.
- Produces:
  - `pub enum WorkerMsg { Progress(Progress), Done(Box<PatchOutcome>), Failed(String) }`
  - `pub struct WorkerHandle { pub rx: std::sync::mpsc::Receiver<WorkerMsg>, pub handle: std::thread::JoinHandle<()> }`
  - `pub fn spawn_patch(input: PathBuf, output: PathBuf, plan: PatchPlan, overwrite: bool) -> WorkerHandle` — the app's entry point, using `Layout::retail()`
  - `pub fn spawn_patch_with_layout(input: PathBuf, output: PathBuf, plan: PatchPlan, overwrite: bool, layout: Layout) -> WorkerHandle` — the seam the worker tests use
  - `pub struct App { /* UiState, log: Vec<String>, worker: Option<WorkerHandle>, report: Option<Result<DiscReport>>, ... */ }` implementing `eframe::App`

The app never calls the engine on the UI thread. Analyze runs `inspect` on a worker too. Layout is one screen as in spec §8: input row + Browse + drag-drop (`ctx.input(|i| i.raw.dropped_files.clone())`), status line, preset combo, collapsible Advanced grid (one `DragValue` per multiplier pre-filled from the derived vector, crown rank, toggles), output row + free-space note, Analyze and Patch ISO buttons (Patch disabled unless the report is `Ok`), progress bar, scrolling log, and an About panel with the unverified caveats from spec §9.

- [ ] **Step 1: Write the failing test**

```rust
// crates/dw4vhp-gui/tests/worker.rs
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::testkit;
use dw4vhp_gui::worker::{spawn_patch_with_layout, WorkerMsg};

#[test]
fn the_worker_reports_progress_then_a_terminal_message() {
    let (dir, input, layout) = testkit::clean_disc_tempfile();
    let out = dir.path().join("out.iso");
    let w = spawn_patch_with_layout(input, out, PatchPlan::default(), false, layout);
    let mut saw_terminal = false;
    for msg in w.rx.iter() {
        match msg {
            WorkerMsg::Progress(_) => {}
            WorkerMsg::Done(_) => saw_terminal = true,
            WorkerMsg::Failed(e) => panic!("unexpected failure: {e}"),
        }
    }
    assert!(saw_terminal);
    w.handle.join().unwrap();
}

#[test]
fn the_worker_reports_a_refusal_as_failed_not_as_a_panic() {
    let (dir, input, layout) = testkit::modded_disc_tempfile();
    let out = dir.path().join("out.iso");
    let w = spawn_patch_with_layout(input, out, PatchPlan::default(), false, layout);
    let mut failed = None;
    for msg in w.rx.iter() {
        if let WorkerMsg::Failed(e) = msg { failed = Some(e); }
    }
    assert!(failed.unwrap().contains("original"));
    w.handle.join().unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p dw4vhp-gui --test worker`
Expected: FAIL — `worker` module does not exist.

- [ ] **Step 3: Implement `worker.rs` then `app.rs`**

The worker owns the mpsc sender, sends `Progress` from the engine callback, and always sends exactly one terminal message. The app drains `try_recv` each frame and repaints while a worker is live (`ctx.request_repaint()`).

- [ ] **Step 4: Run the tests, then the app**

Run: `cargo test -p dw4vhp-gui` → Expected: PASS (6 tests).
Run: `cargo run -p dw4vhp-gui` → Expected: the window opens; with no disc loaded, Patch is disabled and the status line says what to do.

- [ ] **Step 5: Manual end-to-end check, then commit**

Drag the shipped disc onto the window, press Analyze, confirm the status line reports the disc and the copy counts, then Patch to a new file and confirm the progress bar advances and the log ends with a verified digest. Record the outcome in the commit message.

```bash
git add crates/dw4vhp-gui
git commit -m "feat(gui): egui app with a background patch worker, progress and log"
```

---

### Task 14: README, repository rules, and CI

**Files:**
- Create: `README.md`, `.github/workflows/ci.yml`
- Modify: `.gitignore`

**Interfaces:** none (repository hygiene).

- [ ] **Step 1: Write the README**

It must state: you need your own legally obtained **NTSC-U `SLUS_208.36`** disc; the app writes a **new** file and never modifies the original; the presets and what each does; the three unverified items from spec §9 (force Very Hard unplayed, Very Hard → tier 2 inferred, the mod's own open boss/variant questions); the attack-ceiling note; how to run the golden test with the three environment variables; and the GPL-3.0-or-later licence.

- [ ] **Step 2: Add the CI workflow**

`ci.yml` on push to `main` and pull requests, with `permissions: contents: read`, `concurrency` with `cancel-in-progress`, and a matrix of `ubuntu-latest` + `windows-latest` running: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Pin action SHAs, as the sibling project does. On Linux install the GUI build deps egui needs (`libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxi-dev`, `libgl1-mesa-dev`, `libgtk-3-dev`) before building.

- [ ] **Step 3: Confirm the repository carries no game data**

Run: `git status --short && git ls-files | grep -Ei '\.(iso|elf|p2s|ps2|bin)$' || echo "no game binaries tracked"`
Expected: no `.iso`/`.elf`/`.p2s`/`.ps2`/`.bin` paths in the output.

- [ ] **Step 4: Run the exact CI commands locally**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all clean, with the golden test skipping.

- [ ] **Step 5: Commit**

```bash
git add README.md .github/workflows/ci.yml .gitignore
git commit -m "docs: README with disc requirements and verification; ci: build and test matrix"
```

---

### Task 15: Release workflow, icon, and bundle verification

**Files:**
- Create: `.github/workflows/release.yml`, `tools/gen_icon.py`, `crates/dw4vhp-gui/icons/icon.png`
- Modify: `crates/dw4vhp-gui/src/main.rs` (window icon + title)

**Interfaces:** none.

- [ ] **Step 1: Generate the icon**

`tools/gen_icon.py` is a small Pillow script that renders the app icon from a committed source image at 256², 128², 64², 48² and 32² (the sizes `eframe`'s `IconData` wants), writing `crates/dw4vhp-gui/icons/icon.png`.

Run: `python3 tools/gen_icon.py && ls -l crates/dw4vhp-gui/icons/`
Expected: the PNG exists and is committed (it is artwork, not game data).

- [ ] **Step 2: Add the release workflow**

Tag-triggered (`v*`) matrix producing: Windows → `dw4-veryhard-plus-<tag>-windows-x86_64.zip` containing the release `.exe`; Linux → `.AppImage` and `.deb` plus a plain binary tarball. Upload as release assets.

- [ ] **Step 3: Verify the Linux bundle locally**

Handle `--version` and `--help` at the top of `main` before eframe starts (eframe has no CLI of its own: print `env!("CARGO_PKG_VERSION")` and exit).

Run: `cargo build --release -p dw4vhp-gui && ./target/release/dw4-veryhard-plus --version`
Expected: builds and prints a version. Then package the AppImage with the same steps the workflow uses and confirm it launches.

- [ ] **Step 4: Record what could not be verified here**

Windows: the workflow builds the `.exe` in CI, but launching it is unverified in this environment — say so in the README and in the release notes, exactly as the spec's risk table requires. Do not claim otherwise.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release.yml tools/gen_icon.py crates/dw4vhp-gui/icons crates/dw4vhp-gui/src/main.rs
git commit -m "build: release workflow, app icon and bundle verification notes"
```
