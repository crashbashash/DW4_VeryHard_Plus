# DW4 Very Hard Plus

A distributable desktop app that turns a player's own clean **Digimon World 4
(USA, `SLUS_208.36`)** disc into a much harder one. You pick your ISO, pick a
preset, and the app writes a new, modded ISO. It is the "every enemy is elite /
yellow crown" data mod from the decompilation project, packaged for other
players instead of for the mod author's own machine.

## What you need

- A legally obtained **NTSC-U `SLUS_208.36`** disc image (the US release of
  Digimon World 4). Other regions and revisions are **refused**, not guessed.
- Windows or Linux. No Python, no Rust toolchain, and no emulator internals are
  required to *use* the app.

The app opens your ISO **read-only** and always writes a **separate, new file**
(`<name> [VeryHardPlus].iso`). It never patches in place and never keeps a
hidden backup of your disc, so the original is never modified and you can always
re-run from it.

## Presets

| Preset | What it does |
| --- | --- |
| **Very Hard Plus** (default) | Derived ×1.0 stat multipliers, yellow crown (RARITY 5), practice-stage buff on, force Very Hard off, serial unchanged. This is the shipped, byte-verified build. |
| **Very Hard Plus — Extreme** | Identical to the default, plus **force Very Hard**: a one-instruction ELF patch that makes the game start on Very Hard difficulty. |
| **Custom** (auto-selected on any edit) | Exposes every knob: one multiplier per stat, the crown rank (0–5), the exclusion toggles, the practice buff, force Very Hard, and the serial rewrite. |

The default and Extreme presets are named, exact builds; editing any field
switches the preset to **Custom**.

## How it works

The engine streams a copy of your ISO to `<output>.part`, then applies the
same-length, in-place edits to that copy, verifies the result, and renames it
into place. Every edit is computed from your own disc's bytes plus a documented
operation — no game data is shipped with the app. The input handle stays
read-only for the whole run, and a failed or interrupted run can only ever leave
a bad copy, never a damaged original.

The app refuses rather than guesses. It checks that the disc is the clean NTSC-U
release (the table anchors, the exact 665-copy layout, and the boot ELF shape),
that it is not already modded, and that there is free space for the output —
and writes nothing unless every check passes.

## Verification status

The default build is byte-verified against real, shipped discs (see
[Golden test](#golden-test) below). Three things are **not** verified and are
stated as such in the app's About panel:

- **Force Very Hard has never been played.** It is a single instruction — the
  same change the original mod makes — and tests confirm it rewrites exactly
  four bytes in the game's boot file and touches nothing else. What no test can
  show is the result: this project was built without an emulator, so "the game
  really does start on Very Hard" is an expectation, not something anyone has
  watched happen.
- **The Very Hard → tier 2 mapping is reasoned, not measured.** Normal → tier 0
  and Hard → tier 1 were confirmed live in-game, on the valley bridge. The table
  is laid out as 3 tiers with 3–4 variants each, and Very Hard's tier 2 slot
  follows from that layout — but it was never checked the same way, so the
  Extreme preset's effect size is documented as inferred.
- **The original mod's own untested items still apply here:** every boss row is
  unmeasured except `e_mecha4` row 60; about 16 variant models have no row of
  their own; the 88-record `beNDMWStatusInfo` table has never been explored;
  and the two-player graduation tripwire has never been triggered.

## The attack ceiling

The authored attack column is already near its `i16` ceiling, so the derived
×11.875 attack factor pins **337 of 580 live rows (58%)** at 32 767 — on Very
Hard every tier-1/tier-2 enemy and every boss hits for the same capped number.
HP is not affected the same way (×13 pins only 6 rows), so the difficulty does
not depend on attack scaling. This cost is surfaced live in the app ("attack:
337 of 580 rows pinned at 32767"), and the Advanced panel exposes the attack
multiplier so it can be lowered in one click.

## Building from source

Prerequisites: a stable Rust toolchain. Windows needs no extra system packages.
On Linux, install egui's build dependencies first:

```sh
sudo apt-get install -y \
  libxkbcommon-dev \
  libwayland-dev \
  libx11-dev \
  libxi-dev \
  libgl1-mesa-dev \
  libgtk-3-dev
```

Then build and run:

```sh
cargo build --release
./target/release/dw4-veryhard-plus
```

The workspace has two crates: `dw4vhp-core` (the headless, byte-exactly
testable engine) and `dw4vhp-gui` (the egui app, binary
`dw4-veryhard-plus`).

## Golden test

The golden test proves the engine reproduces real, shipped discs byte-for-byte.
It needs the real discs, which are **never committed** to the repository, and
reads their paths from three environment variables:

| Variable | Contents |
| --- | --- |
| `DW4_GOLDEN_SHIPPED_ISO` | the shipped, already-patched disc image |
| `DW4_GOLDEN_BLOCKS_BIN` | the recorded original-region backup blob |
| `DW4_GOLDEN_BLOCKS_JSON` | the `{"writes": [[offset, len], ...], ...}` manifest for that backup |

Run it with:

```sh
DW4_GOLDEN_SHIPPED_ISO=/path/to/shipped.iso \
DW4_GOLDEN_BLOCKS_BIN=/path/to/iso_orig_blocks.bin \
DW4_GOLDEN_BLOCKS_JSON=/path/to/iso_orig_blocks.json \
cargo test -p dw4vhp-core --test golden -- --nocapture
```

It takes ~6 minutes against the real discs and asserts three known md5s: the
reconstructed pristine master `3185f04230b1dabd853db750e4b51108`, the default
Very Hard Plus build `df426b9ff17c9e8218779f8ff1ecf524`, and the practice-buff-off
build `f760258575f4fe2de05a9930e0a6b396` — plus that the serial and force-Very-Hard
options touch only their expected regions and leave the input untouched.

When the three variables are unset, the test **skips cleanly** rather than
failing, so a normal `cargo test --workspace` (and CI) is unaffected.

## Repository rules

**No game data is committed to this repository, ever** — no ISO, no ELF, no
memory card, no savestate, no extract, and no `.bin` backup, however small.
Every constant the engine needs is an offset, a row index, a field name, or an
instruction encoded from its fields; the "before" values come from the player's
own disc at runtime.

## Release bundles

Tagged releases (`v*`) are built and published by
`.github/workflows/release.yml`:

| Asset | Contents |
| --- | --- |
| `dw4-veryhard-plus-<tag>-windows-x86_64.zip` | the release `.exe` |
| `dw4-veryhard-plus-<tag>-linux-x86_64.AppImage` | the app in one file |
| `dw4-veryhard-plus-<tag>-linux-x86_64.deb` | the app installed to `/usr/bin` with a desktop entry and icon |
| `dw4-veryhard-plus-<tag>-linux-x86_64.tar.gz` | the plain binary |

The Linux bundles are built and launch-verified here (the AppImage is run
headlessly under Xvfb). The Windows zip is built in CI, but launching the `.exe`
is unverified in this environment — it has not been run here.

The app icon is generated from committed artwork at
`crates/dw4vhp-gui/icons/source.png` by `tools/gen_icon.py`, which writes
`crates/dw4vhp-gui/icons/icon.png` (256²) — the single image the window embeds.

## Licence

GPL-3.0-or-later (see `LICENSE`).
