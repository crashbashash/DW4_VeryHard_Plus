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

/// ISO9660 sector size, so a file's extent starts at `lba * SECTOR`.
const SECTOR: u32 = 2048;

/// Sector holding the primary volume descriptor.
const PVD_SECTOR: u32 = 16;

/// Sector holding the root directory. One sector is one more than the volume
/// descriptor's, so a fixture stays inside the metadata area.
const ROOT_DIR_SECTOR: u32 = 17;

/// Bytes of a directory record before its name (see `iso9660.rs`).
const RECORD_FIXED_LEN: usize = 33;

/// Offset of the root directory's record inside the primary volume descriptor.
const PVD_ROOT_RECORD_OFFSET: usize = 156;

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

/// An ISO9660 image with a primary volume descriptor and one root-directory
/// record per entry, each entry `(record name including the `;1` version suffix,
/// extent LBA, size in bytes)`.
///
/// The files' extents are reserved — sized and described by the records — but
/// left zeroed, so a caller that needs a payload writes it afterwards. All
/// records live in one root-directory sector, padded to even lengths, so none of
/// them straddles a sector boundary. The image extends past the highest extent.
///
/// The fixture writes nothing but the volume descriptor and the directory below
/// `0x20000`; callers place extents above it so that [`Layout::mini`]'s blocks
/// stay intact (the documented fixtures use LBA 0x1000 and 0x2000).
pub fn iso_with_files(files: &[(&str, u32, u32)]) -> Vec<u8> {
    let directory_offset = u64::from(ROOT_DIR_SECTOR) * u64::from(SECTOR);
    let mut directory = Vec::new();
    for &(name, lba, size) in files {
        directory.extend_from_slice(&directory_record(name.as_bytes(), lba, size));
    }
    assert!(
        directory.len() <= SECTOR as usize,
        "the fixture's root directory must fit in one sector"
    );

    let extent_end = files
        .iter()
        .map(|&(_, lba, size)| u64::from(lba) * u64::from(SECTOR) + u64::from(size))
        .max()
        .unwrap_or(0);
    let len = (directory_offset + u64::from(SECTOR)).max(extent_end);
    let mut iso = vec![0u8; len as usize];

    let pvd_offset = (PVD_SECTOR * SECTOR) as usize;
    iso[pvd_offset] = 1; // primary volume descriptor
    iso[pvd_offset + 1..pvd_offset + 6].copy_from_slice(b"CD001");
    iso[pvd_offset + 6] = 1; // version
    let root = directory_record(&[0], ROOT_DIR_SECTOR, SECTOR);
    iso[pvd_offset + PVD_ROOT_RECORD_OFFSET..][..root.len()].copy_from_slice(&root);

    let directory_offset = directory_offset as usize;
    iso[directory_offset..directory_offset + directory.len()].copy_from_slice(&directory);
    iso
}

/// One directory record: length, extended-attribute length 0, extent LBA and
/// data length as both-endian 8-byte fields, a zeroed timestamp, flags 0, unit
/// size 0, interleave gap 0, volume sequence 1 as a both-endian 4-byte field,
/// the name length and the name, padded to an even length.
fn directory_record(name: &[u8], lba: u32, size: u32) -> Vec<u8> {
    let unpadded = RECORD_FIXED_LEN + name.len();
    let len = unpadded + unpadded % 2;
    assert!(len <= u8::MAX as usize, "a record length is one byte");

    let mut record = vec![0u8; len];
    record[0] = len as u8;
    both_ended_u32(&mut record[2..10], lba);
    both_ended_u32(&mut record[10..18], size);
    both_ended_u16(&mut record[28..32], 1);
    record[32] = name.len() as u8;
    record[33..unpadded].copy_from_slice(name);
    record
}

/// Writes `value` into an 8-byte both-endian field: little-endian half first.
fn both_ended_u32(field: &mut [u8], value: u32) {
    field[..4].copy_from_slice(&value.to_le_bytes());
    field[4..].copy_from_slice(&value.to_be_bytes());
}

/// Writes `value` into a 4-byte both-endian field: little-endian half first.
fn both_ended_u16(field: &mut [u8], value: u16) {
    field[..2].copy_from_slice(&value.to_le_bytes());
    field[2..].copy_from_slice(&value.to_be_bytes());
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
