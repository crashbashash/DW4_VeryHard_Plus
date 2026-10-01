//! Fixture builders for tests that need a disc image.
//!
//! **Not part of the stable API.** This module exists so that the core's tests
//! and the GUI crate's tests build their images in code — no game data is ever
//! stored in the repository — instead of each writing its own copy of the same
//! table. Its shape may change with any test; nothing outside tests should
//! depend on it.

use crate::layout::Layout;
use crate::table::EnemyTable;
use std::path::PathBuf;
use tempfile::TempDir;

/// Rows in every generated table. The transform's row indices (391, 644) are
/// fixed against the retail disc, so fixtures keep 649 rows and shrink only the
/// block offsets and the copy count.
const ROWS: usize = 649;

/// Copies of each block a generated image carries, so the copy count is a real
/// signal rather than one block found by accident.
const COPIES: usize = 3;

/// Distance between the three copies of a block: 64 KiB, sector-aligned and far
/// wider than one copy (20 119 bytes), so no two copies touch.
const COPY_STRIDE: u64 = 0x1_0000;

/// The crown the modded fixture writes into every one of its 649 rows. An
/// authored disc crowns 60 rows and a patched one 638, so a disc crowned on
/// every row is exactly what `inspect` refuses as already modded.
const MODDED_RARITY: u8 = 5;

/// Writes a clean disc image to a new temp file and returns its directory, its
/// path and the layout it was built with. Keep the [`TempDir`] alive while the
/// file is in use.
pub fn clean_disc_tempfile() -> (TempDir, PathBuf, Layout) {
    disc_tempfile("clean.iso", 0)
}

/// The same image, with the crown written into every one of the 649 rows, as a
/// patched disc has: [`crate::disc::inspect`] refuses it as already modded.
pub fn modded_disc_tempfile() -> (TempDir, PathBuf, Layout) {
    disc_tempfile("modded.iso", MODDED_RARITY)
}

/// Writes `disc_bytes` to a fresh file called `name` in a new temp directory.
fn disc_tempfile(name: &str, rarity: u8) -> (TempDir, PathBuf, Layout) {
    let layout = Layout::mini(ROWS, COPIES);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, disc_bytes(&layout, rarity)).unwrap();
    (dir, path, layout)
}

/// The image itself: the authored table at `layout`'s offsets, written `COPIES`
/// times [`COPY_STRIDE`] apart, over zeroed space so only the copies can match a
/// block search.
fn disc_bytes(layout: &Layout, rarity: u8) -> Vec<u8> {
    let blocks = authored_table(rarity).blocks();
    let starts = [
        layout.hp_off as usize,
        layout.stat_off as usize,
        layout.byte_off as usize,
    ];
    let len = starts[2] + (COPIES - 1) * COPY_STRIDE as usize + blocks[2].len();

    let mut iso = vec![0u8; len];
    for copy in 0..COPIES {
        let shift = copy * COPY_STRIDE as usize;
        for (block, &start) in blocks.iter().zip(starts.iter()) {
            iso[start + shift..start + shift + block.len()].copy_from_slice(block);
        }
    }
    iso
}

/// A full 649-row table whose every row differs from its neighbours, so each
/// block's bytes stay recognisable in the image, with `rarity` on every row.
fn authored_table(rarity: u8) -> EnemyTable {
    let hp: Vec<i32> = (0..ROWS as i32).map(|row| 100 + row * 37).collect();
    let stat: Vec<[i16; 12]> = (0..ROWS)
        .map(|row| {
            let mut values = [0i16; 12];
            for (column, value) in values.iter_mut().enumerate() {
                *value = (row * 12 + column) as i16 - 4000;
            }
            values
        })
        .collect();
    let crit: Vec<u8> = (0..ROWS)
        .map(|row| (row as u8).wrapping_mul(11).wrapping_add(7))
        .collect();
    let para: Vec<u8> = (0..ROWS)
        .map(|row| (row as u8).wrapping_mul(29).wrapping_add(3))
        .collect();

    EnemyTable {
        hp,
        stat,
        crit,
        para,
        rarity: vec![rarity; ROWS],
    }
}
