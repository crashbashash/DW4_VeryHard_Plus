# DW4 Very Hard Plus

[![CI Linux](https://img.shields.io/github/actions/workflow/status/crashbashash/DW4_VeryHard_Plus/ci-linux.yml?label=CI%20Linux&branch=main)](https://github.com/crashbashash/DW4_VeryHard_Plus/actions/workflows/ci-linux.yml)
[![CI Windows](https://img.shields.io/github/actions/workflow/status/crashbashash/DW4_VeryHard_Plus/ci-windows.yml?label=CI%20Windows&branch=main)](https://github.com/crashbashash/DW4_VeryHard_Plus/actions/workflows/ci-windows.yml)
[![Release Linux](https://img.shields.io/github/actions/workflow/status/crashbashash/DW4_VeryHard_Plus/release-linux.yml?label=Release%20Linux)](https://github.com/crashbashash/DW4_VeryHard_Plus/actions/workflows/release-linux.yml)
[![Release Windows](https://img.shields.io/github/actions/workflow/status/crashbashash/DW4_VeryHard_Plus/release-windows.yml?label=Release%20Windows)](https://github.com/crashbashash/DW4_VeryHard_Plus/actions/workflows/release-windows.yml)

This is a little desktop app for **Digimon World 4** players who found the game too easy.
It takes your own copy of the game and turns it into a much harder one, in a few clicks —
no modding knowledge required.

Think of it like this: the people who made the original "yellow crown" mod edited their own
disc by hand. This app does that same edit for you, automatically, on your machine, using
your own disc.

## How it works (the short version)

1. You give the app a copy of your game disc (an `.iso` file).
2. You pick a difficulty mode (see below).
3. The app writes out a **new, separate file** — your original is never touched.

You end up with something like `Digimon World 4 [VeryHardPlus].iso`, which you can play
in an emulator or burn/load like any other copy.

## What you need

- A **US release** of Digimon World 4 (`SLUS_208.36`). If you give it a disc from another
  region or a different version, the app tells you it won't work — it doesn't guess.
- Windows or Linux. Nothing else to install.

## The modes

| Mode | In plain words |
| --- | --- |
| **Very Hard Plus** *(default)* | Every enemy gets big boosts to health, attack, and other stats, and wears the "yellow crown" — the badge the game gives the strongest enemy variants. This is the mode that's been checked byte-for-byte against real discs. |
| **Very Hard Plus — Extreme** | Same as above, **plus** the game starts you on Very Hard difficulty right away, instead of making you unlock or pick it. |
| **Very Hard Plus — Brutal** | Same as Extreme, but taken further: every enemy of a given type becomes *the strongest version of that type*. No more weaklings — every Digimon you meet is the elite of its kind (extra experience included). |
| **Custom** | The do-it-yourself mode. It appears automatically the moment you change any setting. You get every dial: one slider per stat, how strong the "crown" enemies are, whether the difficulty-boost and start-on-Very-Hard changes are on, and more. |

A few honest notes about the modes:

- The **default** mode is the only one that's been verified against real discs, byte for byte.
- **Extreme's** "start on Very Hard" change is the same single-line change the original mod
  author made, but nobody has actually sat down and played it — so it should work, but it's
  not confirmed in play.
- **Brutal's** grouping (which enemies belong to which type) was recovered from a memory
  snapshot of the running game, because the disc doesn't record it. It's never been played
  either.
- **Attack values are already near the game's maximum**, so on Very Hard most enemies hit
  just as hard as each other no matter what — the extra difficulty really comes from the
  health and stat boosts. The app shows you when this happens, and the Custom mode lets you
  turn the attack boost down if you'd rather.

## Is my disc safe?

Yes. The app opens your file **read-only**, does its work on a temporary copy, checks the
result, and only then names it as the finished file. If anything goes wrong or you cancel
halfway, all that exists is an incomplete extra file — your original is never modified.
It also checks up front that your disc is the right one, isn't already modded, and that
you have enough disk space, and writes nothing at all unless everything checks out.

## Where do I get it?

Download a ready-made version from the **Releases** page of this project — there are
installers for Windows (`.msi` or a plain `.zip`) and Linux (`.AppImage`, `.deb`, or a
plain download). No building required.

## For developers and the curious

The rest of the details live here:

- **Building from source** needs a Rust toolchain, Node.js, and — on Linux — the
  Tauri webview dependencies:

  ```sh
  sudo apt-get install -y \
    libxkbcommon-dev \
    libwayland-dev \
    libx11-dev \
    libxi-dev \
    libgl1-mesa-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
  ```

  Then:

  ```sh
  npm ci
  npm run dev        # frontend alone (mock backend, in a browser tab)
  npx tauri dev      # the desktop app, live-reloading
  npx tauri build    # distributables (exe/AppImage/deb/MSI)
  ```

- The code has two parts: `dw4vhp-core` (the engine that actually edits the disc, fully
  testable on its own) and `dw4vhp-tauri` (the window you see: a Tauri shell plus the React app in `src/`).
- **Testing against real discs:** the "golden test" proves the app reproduces known,
  shipped discs byte-for-byte. It needs real disc images supplied through environment
  variables (`DW4_GOLDEN_SHIPPED_ISO`, `DW4_GOLDEN_BLOCKS_BIN`, `DW4_GOLDEN_BLOCKS_JSON`)
  and takes ~7 minutes; it skips cleanly (rather than failing) when those aren't set.
  There's also a row-group test (`tests/rowgroup.rs`, plus `tests/rowgroup_snapshot.rs`
  with `DW4_ROWGROUP_SNAPSHOT`) that checks the Brutal mode's enemy grouping.
- **One rule of this repository:** no game data is ever committed here — no disc images,
  no extracted files, no memory dumps. Everything the app needs as a constant is an
  offset or instruction; the actual "before" values come from the player's own disc.
- Release builds (Windows zip/MSI, Linux AppImage/deb/tarball) are produced automatically
  by CI for tagged releases. The Linux ones are launch-tested; the Windows ones are built
  but never run here, and like all release assets, they're unsigned.

## Licence

GPL-3.0-or-later — see `LICENSE`.
