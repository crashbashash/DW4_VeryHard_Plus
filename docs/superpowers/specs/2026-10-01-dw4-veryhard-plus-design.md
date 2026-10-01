# DW4 Very Hard Plus — design

**Date:** 2026-10-01
**Status:** approved for planning
**Repo:** `github.com/crashbashash/DW4_VeryHard_Plus`

## 1. What this is

A distributable desktop app that turns a player's own clean **Digimon World 4 (USA,
`SLUS_208.36`)** disc into a much harder one. The player picks their ISO, picks a preset,
and the app writes a new, modded ISO. It is the "every enemy is elite / yellow crown" data
mod from the decompilation project, packaged for other players instead of for the mod
author's own machine.

The app itself is called **DW4 Very Hard Plus**; the discs it produces are the product.

### Why a new project

The existing mod is one 248-line Python script, `patch_iso.py`, that hardcodes its own
machine's paths and reads the "before" values from a PCSX2 RAM snapshot
(`~/pi-re/snapshots/pink_ram.bin`). Neither is shippable: a player has no snapshot, and the
script also writes its revert backup to a fixed absolute path. This project reimplements the
patch as a self-contained Rust engine that reads everything it needs out of the player's own
disc, plus a thin GUI over it.

### Audience and constraints

- **Players**, not the mod author. No Python, no Rust toolchain, no emulator internals.
- **Windows and Linux** at minimum, as prebuilt bundles.
- **No game data may be committed to this repo, ever.** No ISO, no ELF, no extracts, no
  savestates, no memory cards. Every byte the app writes is either computed from the user's
  own disc or encoded from a documented operation. See §10.

## 2. Scope

### v1 (this spec)

- One patcher engine that writes three kinds of same-size, in-place change to a **copy** of
  the player's ISO: the enemy stat table, optionally the boot ELF, optionally the disc serial.
- Presets **Very Hard Plus** (default) and **Very Hard Plus — Extreme**, plus a **Custom**
  panel that exposes every knob.
- egui GUI (one screen) and a headless engine that is byte-exactly testable.
- CI + release bundling for Windows and Linux.

### v2 (designed for, not built here)

- **Very Hard Plus — Brutal**: force Very Hard *and* collapse every spawn to its enemy type's
  strongest row. Blocked on a row→type mapping the disc does not carry — see §12.

### Non-goals

- No mod manager, no other mods (no warp randomiser, no spawn swaps, no practice-disc
  variants beyond the toggle).
- No ISO growth, repacking or file relocation. Every write is same-length, in place, on a
  copy. (`repack_iso.py` documents why: the boot ELF sits at LBA 289 with 17 files after it.)
- No PAL/NTSC-J support. They are **refused**, not guessed. Every constant in §3.1 is
  measured from the NTSC-U disc.
- No tier-based (non-uniform) scaling. The v5b4 design is deliberately uniform, and the
  tier-assignment heuristic needs the runtime `MODEL` column, which is not in the ISO.
- No CLI in v1. The engine is a library and the golden tests drive it directly; a CLI is a
  one-file follow-on that costs nothing to add later.
- No "fit to field" attack rescaling. Investigated and rejected — see §3.5.

## 3. The crown patch, precisely

Everything below was re-measured against the shipped discs while writing this spec; the
numbers in this section are the contract the implementation must reproduce byte-for-byte.

### 3.1 The table

The 649-row enemy stat table is authored data inside `AREA.AFS`, stored as **665
byte-identical copies**, one per stage/layer pack. Three blocks per copy:

| Block | Encoding | Size | Offset of copy 1 (ISO byte offset) |
|---|---|---|---|
| HPMAX | 649 × i32, **column-major** | 2 596 B | `0x78D64C5` |
| 12 stats | 649 × 12 × i16, **row-major** | 15 576 B | `0x78DACB9` |
| crit / paralysis / RARITY | 649 × 3 × u8, row-major | 1 947 B | `0x78DA51D` |

The i16 column order, in full: `c0` attack (`TSUYOSA`) · `c1` defence (`MAMORI`) · `c2`
wisdom (`KASHIKOSA`) · `c3` spirit (`SEISHIN`) · `c4` speed (`SUBAYASA`) · `c5`–`c10` the six
resists (fire, ice, thunder, dark, stun, poison) · `c11` EXP.

All three offsets are verified: reading them out of the shipped disc yields row 0 = 650 HP /
RARITY 5, row 60 (boss) = 41 522 HP, row 370 (`g_*`) = 100 HP / RARITY 0, row 644 = 35 HP /
RARITY 0, and exactly 665 copies of the HP block, the first at `0x78D64C5`.

### 3.2 The ratio is derived from the disc, not shipped

`patch_iso.py` reads the "before" values from a RAM snapshot. We read them from the player's
own ISO at the offsets above — which is what makes the tool shippable. The ratio is:

- **reference ordinary rows:** `obs = rows 0..=3` (the four weakest `e_goburi` variants)
- **reference top-rank row:** `r5 = row 391` (`e_goburi` rank-5, tier-0)
- `factor[HP] = hp[391] / max(1, min(hp[0..3]))`
- `factor[c] = stat[r5][c] / min(stat[0..3][c])`, or `1.0` if the denominator is 0
- `delta[crit] = crit[391] - median(crit[0..3])`, and the same for paralysis

Measured from the authored table, this reproduces the documented vector exactly:

```text
HP 13.0 | atk 11.875 | def 2.1556 | wis 12.0909 | spr 1.7857 | spd 3.1
resists 1.5 / 2.0 / 2.1667 / 2.5 / 3.7 / 2.3333 | EXP 14.6 | crit +19.5 | para +19.5
```

(The authored crit values for rows 0–3 are 3, 4, 5, 6 — median 4.5 — and row 391 is 24, hence
the +19.5.)

### 3.3 The transform

For every row `r`, unless excluded:

```text
new_hp[r]     = clamp_i32(round_ties_even(hp[r] * factor_hp * hp_mult), cap = 400_000)
new_stat[r][c]= clamp_i16(round_ties_even(stat[r][c] * factor[c] * stat_mult[c]))
new_crit[r]   = clamp_u8(round_ties_even(crit[r] + delta_crit * crit_mult), 0, 60)
new_para[r]   = clamp_u8(round_ties_even(para[r] + delta_para * para_mult), 0, 60)
new_rarity[r] = rank                      // default 5 (yellow)
```

Semantics that must be preserved exactly, because they are load-bearing:

- **Rounding is ties-to-even**, matching Python's `round()` used by the original patcher.
  Rust's `f64::round()` rounds ties away from zero and would silently differ (e.g. authored
  `2 + 19.5 = 21.5` → 22 either way, but `1 + 19.5 = 20.5` → 20 ties-to-even vs 21 away).
  Use `f64::round_ties_even()`.
- **The i16 clamp is the whole point of v5b**: an overflowing value clamps to 32 767 rather
  than keeping the authored value. See §3.5 for what this costs.
- **The crown rank is written to every handled row, including the 69 empty rows** (HP 0, no
  model). The shipped disc has 638 rows at RARITY 5 and 11 rows that are not.
- Authored crown rows are **not** remapped; they are scaled like everything else.

### 3.4 Exclusions

| Rows | What | Why |
|---|---|---|
| 370–378 (9 rows) | destructibles (`g_*`) | crates and barrels must still break normally; left byte-identical (100 HP, RARITY 0) |
| 644, 645 | the `e_cockatri` tutorial pair | deliberate tripwire: the practice stage's 2-player graduation spawn uses these rows, so a base-stat Kokatorimon proves which row a spawn took |

Verified on the shipped disc: exactly 11 rows keep RARITY 0, and they are exactly
`370..=378, 644, 645`.

### 3.5 Attack saturation — accepted, and why the alternatives were rejected

The authored attack column is already near its ceiling: the designers hand-capped rows 609
and 637 at **32 000** of 32 767, and tier-2 `e_goburi` is authored at 21 084–21 943.
Multiplying by ×11.875 therefore pins **337 of 580 live rows (58 %)** at 32 767, collapsing
505 distinct authored attack values to 199:

| attack factor | rows pinned | distinct values | `e_goburi` tier-2 attack |
|---|---|---|---|
| ×1.0 | 0 | 505 | 21084 / 21423 / 21855 / 21943 |
| ×1.25 | 102 (18 %) | 412 | 26355 / 26779 / 27319 / 27429 |
| ×11.875 (shipped) | 337 (58 %) | 199 | 32767 / 32767 / 32767 / 32767 |

The docs' "fit to field" idea does not fix this: a whole-column fit is **×1.024** (the
headroom was already spent by the designers), and per-row `min(factor, 32767/authored)` still
bunches tier-2 rows at ~32 767. So the real choice was "keep the shipped behaviour" or "stop
boosting attack", and **the shipped behaviour was kept** so the default preset stays
byte-identical to the disc that already exists and already has a verification oracle.

Cost, accepted and surfaced in the UI: on Very Hard every tier-1/tier-2 enemy and every boss
hits for the same capped number. The Advanced panel exposes the attack multiplier so a player
can lower it in one click, and the GUI shows the live cost ("attack: 337 of 580 rows pinned at
32767").

HP is not affected the same way — ×13 pins only 6 rows — so the mod's difficulty does not
depend on attack scaling.

### 3.6 Practice-stage buff (on by default)

The tutorial rows 638–643 are authored as token values (HP 20–35, EXP 3), so after scaling
they are weaker than the weakest ordinary enemy of their own type. The buff copies an
ordinary tier-0 row of the same type over each of them — whole rows, all three columns, in all
665 copies:

| practice row | ← source row | type |
|---|---|---|
| 638, 639 | 0, 1 | `e_goburi` |
| 640, 641 | 33, 34 | `e_ogre` |
| 642, 643 | 12, 13 | `e_nume` |

Rows 644/645 are **not** in this map — the tripwire survives the buff. The source rows are read
from the *computed* table (i.e. after scaling), which is how the shipped practice-buff disc was
built and is equivalent for these rows because none of them clamps. Applying this is the
default; turning it off must reproduce the shipped non-buff disc byte-for-byte.

This settles the open item the mod's own docs left ("fold it in or ship two discs?"): the buff
is folded in as the default.

## 4. The optional ELF patch (force Very Hard)

`SYSsetDifficulty` lives at vaddr `0x00371DD0`. Its body, read out of the retail ELF:

```text
0x371DD0  27BDFFE0  addiu sp,sp,-32
0x371DD4  FFBF0010  sd ra,16(sp)
0x371DD8  7FB00000  sq s0,0(sp)
0x371DDC  0080802D  daddu s0,a0,zero     <- arg0 is saved into s0
0x371DE0  0C0DC704  jal  ...             (bePlayer_GetStruct; delay slot sets a0=0)
0x371DF0  0C0EA488  jal  ...             (igProperty_GetRecord)
0x371DF8  AC50000C  sw s0,12(v0)         <- and stored into the player record
```

The documented mod (`mods/code-mod-difficulty`) rewrites the function so the argument is
ignored and `1` (Very Hard) is stored — a 15-byte change to the *linked* build. We do not need
fifteen bytes: replacing the single word at `0x371DDC` with `addiu s0, zero, 1` stores the
constant 1 and is semantically identical, because `a0` is reloaded by the delay slot and again
before the call, and `s0` is read only by the store at `0x371DF8`. One instruction, four bytes.

The expected and replacement words are **encoded from their fields**, not shipped as game
bytes:

```text
daddu s0, a0, zero  == SPECIAL(0) | rs=4<<21 | rt=0<<16 | rd=16<<11 | funct=0x2D  == 0x0080802D
addiu s0, zero, 1   == OP(0x09)<<26 | rs=0<<21 | rt=16<<16 | imm=1              == 0x24100001
```

Location: `SLUS_208.36` is the boot ELF, ISO9660 entry at LBA 289 (ISO offset `0x90800`),
length 4 541 744. Its first program header is `PT_LOAD vaddr 0x100000 / file offset 0x80`, so
the vaddr→file mapping is read from the ELF's own program headers rather than assumed. For the
retail disc, `0x371DDC` lands at ISO offset `0x302650`.

This is the one part of v1 that **cannot be verified in-game here** — see §9.

## 5. The optional serial patch

PCSX2 titles a disc from its GameDB by serial, so a modded disc is indistinguishable from the
original in the game list. Rewriting the serial gives it its own row. Two same-length writes:

1. the ISO9660 directory record name of the boot ELF (`SLUS_208.36;1`)
2. the `SYSTEM.CNF` line `BOOT2 = cdrom0:\SLUS_208.36;1`

The replacement must be exactly 11 characters in the `XXXX_NNN.NN` shape — any other length
would change the directory record and need a full rebuild, so it is refused. The value
pre-filled in the UI is `SLUS_000.00`, the one verified on PCSX2 2.8.2; it is an editable text
field, not a free-form one. **Default off**,
because the documented cost is real: the GameDB entry stops applying, so the disc loses its
region flag, compatibility rating and `gsHWFixes` (`halfPixelOffset: 4`, which fixes misaligned
blur). The trade-off is stated inline in the UI.

## 6. Preconditions and refusal behaviour

The tool refuses rather than guesses. Nothing is ever written unless every check passes; when
a check fails, the app explains which one and writes nothing.

| Check | Failure means |
|---|---|
| The three anchors parse as a 649-row table with plausible values | not this disc, or not this revision |
| Each of the three authored blocks occurs **exactly 665** times | a different revision; refuse |
| `SLUS_208.36` present, length 4 541 744, a `PT_LOAD` covers `0x371DDC` | wrong disc/region |
| At most ~100 rows have `RARITY != 0` | the disc is **already modded** (authored = 60 rows, patched = 638) → tell the player to use their original |
| For the ELF step: the word at `0x371DDC` decodes to the expected `daddu` | unexpected code; refuse that step |
| Free space ≥ input size at the destination | refuse before copying |
| Destination does not already exist (unless confirmed) | never silently clobber |
| The input file is opened **read-only**, and never written to | — |

The "already modded" detector is what makes the always-write-a-new-file model safe: because we
never patch in place and never keep a backup, a player who wants different settings just
re-runs from their original ISO.

## 7. Architecture

A Cargo workspace, mirroring the sibling save editor's core/UI split:

```text
Cargo.toml              workspace
crates/dw4vhp-core/     the engine: no GUI, no CLI, fully testable
crates/dw4vhp-gui/      the egui app (bin: dw4-veryhard-plus)
docs/                   this spec, plus a README explaining discs and verification
.github/workflows/      ci.yml, release.yml
```

### Engine pipeline

1. **Inspect** — open the input read-only, apply every check in §6, and read the authored
   table from the anchors. Produce a `DiscReport` (serial, size, row count, discovered copy
   count, already-modded flag, ELF status) and never write.
2. **Derive** — the ratio vector (§3.2).
3. **Plan** — turn a `PatchPlan` plus the authored table into the new table, a list of write
   regions, and a summary (`rows changed`, `rows pinned at the attack ceiling`, `rows at the
   HP cap`, `practice rows rewritten`, `ELF step`, `serial step`). This is what the Analyze
   button shows and what the GUI displays live, and it is pure — no I/O.
4. **Write** — stream a copy of the input to `<output>.part` with progress, then reopen the
   part file read-write and apply the write regions, fsync, and rename into place. The input
   handle stays read-only for the whole run.
5. **Verify** — re-read the output's anchors, recount the 665 copies, confirm the ELF word and
   the serial ranges if they were touched, and report the md5. Success is *proved*, not
   assumed.

Progress is reported through a callback (`phase`, `bytes_done`, `bytes_total`) so the GUI can
show a bar and the tests can pass a no-op. The engine never loads the ISO into memory: the
writes total ~13.4 MB across 1 995 regions; the blocks are located with a content search
(`memchr`).

`PatchPlan` is the whole surface the GUI edits:

```rust
struct PatchPlan {
    hp_mult: f64,                     // scales the derived HP factor       (default 1.0)
    stat_mult: [f64; 12],             // scales each derived i16 factor     (default 1.0)
    crit_mult: f64, para_mult: f64,   // scale the derived additive deltas  (default 1.0)
    crown_rank: u8,                   // RARITY written to every handled row (default 5)
    exclude_destructibles: bool,      // rows 370-378                       (default true)
    exclude_tripwire: bool,           // rows 644/645                       (default true)
    practice_buff: bool,              // rows 638-643                       (default true)
    force_very_hard: bool,            // the ELF instruction patch          (default false)
    serial: Option<String>,           // None = leave the serial alone      (default None)
}
```

## 8. Presets and the GUI

| Preset | ratios | crown | practice buff | force Very Hard | serial |
|---|---|---|---|---|---|
| **Very Hard Plus** (default) | derived ×1.0 | 5 | on | off | off |
| **Very Hard Plus — Extreme** | derived ×1.0 | 5 | on | **on** | off |
| **Custom** (auto-selected on any edit) | editable | 0–5 | toggle | toggle | toggle |

Only the first is a shipped, byte-verified build (and it is the shipped practice-buff disc,
`df426b9f…`). Extreme is new in exactly one respect: the one-instruction difficulty patch.

The window is one screen, no wizard:

- **Input row** — path field, browse, drag-and-drop, and a detected-disc status line that is
  green on a clean disc and specific-red otherwise ("already modded — use your original ISO",
  "not the NTSC-U release", "unrecognised revision").
- **Preset dropdown** with a one-line description of the effective multipliers.
- **Collapsible Advanced panel** — one editable multiplier per stat pre-filled with the
  derived values, the crown rank, the exclusion toggles, the three toggles of §7. Any edit
  switches the preset to Custom.
- **Output row** — pre-filled as `<input dir>/<name> [VeryHardPlus].iso`, with a free-space
  check.
- **Analyze** (pre-flight report, writes nothing) and **Patch ISO** (disabled until a clean
  disc is detected).
- **Progress + log** — phase text, progress bar, scrolling log of what was written and
  verified.
- **About panel** carrying the mod's own verified/not-verified caveats (§9), so a player sees
  them without reading the repo.

## 9. Verification

### Golden tests (env-gated, discs never committed)

The oracle chain needs no pristine master from anywhere else. Given the shipped disc and its
`iso_orig_blocks.bin/json` backup (both already exist in the decomp workspace), a test can:

1. apply the 1 995 recorded regions back over the shipped disc → assert md5
   `3185f04230b1dabd853db750e4b51108`, i.e. **prove** the reconstruction is the pristine
   master;
2. run the engine with the **default preset** → assert md5 `df426b9ff17c9e8218779f8ff1ecf524`
   (the shipped practice-buff disc);
3. run with **practice buff off** → assert md5 `f760258575f4fe2de05a9930e0a6b396` (the shipped
   v5b4 disc);
4. run with the **serial** option → assert *only* the two expected ranges differ from the
   input;
5. run with **force Very Hard** → assert *exactly four bytes* differ, at the computed offset,
   and that the input is untouched.

Steps 1–3 are byte-exact end-to-end proofs that the Rust port reproduces the Python
implementation on real data. The test skips cleanly (not fails) when the discs are absent, and
the paths come from environment variables. Discs are never added to git.

**Validated during planning (2026-10-01).** The chain was exercised end-to-end against the real
discs before this plan was written, using a throwaway Python reference implementation of §3:
step 1 reproduced `3185f04230b1dabd853db750e4b51108`; step 2 produced `df426b9f…` with the
default preset; step 3 produced `f7602585…` with the practice toggle off. The ratio derived from
the pristine ISO's own bytes matched the §3.2 vector on every one of the fifteen factors, and
each block was found and rewritten in exactly 665 copies. The md5 targets in steps 1–3 are
therefore known-reachable, not merely expected.

### Unit and property tests

- The ratio derivation, against the authored table (pinned to the vector in §3.2).
- The transform: caps, clamps, ties-to-even rounding, empty rows still getting the crown rank,
  the exclusion sets, the practice mapping.
- The MIPS encode/decode preconditions, including a wrong-word rejection.
- Serial name validation and the ISO9660 directory-record / `SYSTEM.CNF` rewriting.
- Refusal paths: already-modded disc, wrong copy count, bad anchors, insufficient space.
- `PatchPlan::default()` + the default preset produce the shipped byte pattern for the table.

### Not verified, and stated as such in the UI

- **Force Very Hard has never been played.** It is a new implementation (one instruction) that
  is semantically equivalent to the documented mod, but "the game really does run Very Hard
  after this patch" is unconfirmed here — there is no emulator in this environment. This goes
  in the About panel and the README.
- **Very Hard → tier 2 is inferred, not measured.** Normal → tier 0 and Hard → tier 1 were
  verified live on the valley bridge; the Very Hard leg follows from the 3-tier × 3–4-variant
  block layout. So Extreme's effect size is documented as inferred.
- The mod's own open items carry over verbatim: every boss row except `e_mecha4` row 60 is
  unmeasured, ~16 variant models have no row of their own, the 88-record
  `beNDMWStatusInfo` table is unexplored, and the 2-player graduation tripwire has not been
  triggered.

## 10. Licensing and repository rules

- **GPL-3.0-or-later** — the LICENSE already in the repo, matching the decompilation project
  the patch is derived from.
- **No game data, ever.** No ISO, ELF, memory card, savestate, or extract — including
  "small" ones. Every constant the engine needs is either an offset, a row index, a field
  name, or an instruction encoded from its fields (§4); "before" values come from the
  player's own disc at runtime. `.gitignore` covers `*.iso`, `*.elf`, `*.p2s`, `*.ps2`.
- The repo README must say plainly that the user supplies their own legally obtained disc, and
  that the app writes a new file and never modifies the original.

## 11. Packaging and CI

- **Windows**: a release `.exe` in a zip. egui needs no installer, and an `.msi` would add
  WiX for no gain.
- **Linux**: `.AppImage` (any distro) and `.deb`. No webview, no GTK runtime beyond the
  usual — one binary.
- `ci.yml`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace`, on Linux and Windows.
- `release.yml`: tag-triggered matrix producing both bundles, following the sibling project's
  convention (pinned action SHAs, `persist-credentials: false`).
- App icon generated from a committed source image, as the sibling project does.

## 12. Deferred: Brutal (v2)

**Very Hard Plus — Brutal** = force Very Hard *and* make every spawn use its enemy type's
strongest row, so the variant a stage happens to pick no longer matters.

It cannot be built from the disc alone. The three patchable columns carry no type identity —
they are numbers. A spawn's row is chosen by the stage's generator entry, so "every enemy is
its top variant" means either collapsing each type's rows onto its strongest row (whole-row
copy, the same proven mechanism as the practice buff, in all 665 copies) or rewriting every
stage's generator entries (`patch_spawn.py`, a per-stage job). The first needs a **row→type
mapping**, which lives only in the runtime `MODEL` column.

The mapping is documented — `mods/crown-elite-stat-boosts.md` §6 lists all 41 models with a
Base row and a Top row (`e_goburi` 0 → 393, `e_ogre` 33 → 41, `e_nume` 12 → 637, …) — but it
has to be materialised as a committed **row-group table** (row indices only, no game bytes)
and verified: each type's top row must be its block's maximum HP, and the known anchors
(370–378, 379–393, 394–408, 409–423, 610–624, 638–645, the 69 empty rows) must fall out
correctly. The layout is not a clean per-type sort — rows 609 and 637 are special high-stat
rows for `e_ldknight`/`e_nume` sitting among other types — so this is a verification exercise,
not a guess.

Prerequisites for v2, in order: build the row-group table; verify it against the documented
anchors and top rows; decide whether Brutal's collapse should also override the exclusions
(destructibles and the tripwire); then implement it as one more `PatchPlan` field.

Accepted consequence, to state in the UI when it lands: collapsing to a type's top row
collapses everything that row carries — EXP payouts (mostly at the 32 767 ceiling, so every
kill pays the same), resists and crit/paralysis. Combined with the flat attack column, Brutal
is close to uniform enemies at the ceiling, which is the intent of the preset but the opposite
of the differentiation the other presets retain.

## 13. Risks

| Risk | Handling |
|---|---|
| A write lands at a wrong offset and corrupts a 1.4 GB copy | Every write is gated on a precondition (§6) and the input is only ever read; the worst case is a bad copy, never a damaged original |
| Byte-exactness drift from the Python patcher | Golden tests assert md5s of the shipped discs; ties-to-even rounding is pinned in §3.3 |
| Wrong disc revision slips through | Anchors + exact 665-copy count + ELF shape; refuse otherwise |
| Force Very Hard misbehaves in game | One instruction, semantically argued, precondition-checked, and clearly labelled unverified; it is only in the Extreme preset, never the default |
| 1.4 GB copy is slow or the disk fills | Free-space check up front, `.part` file + rename, progress and cancel |
| Players point it at an already-modded disc | Explicit detection and message; the always-copy model means they can always start over from their original |
