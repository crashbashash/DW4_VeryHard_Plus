use crate::error::{Error, Result};
use crate::layout::Layout;
use crate::table::EnemyTable;
use std::path::Path;

/// Counts the non-overlapping occurrences of `needle` in `iso`.
pub fn count_copies(iso: &[u8], needle: &[u8]) -> usize {
    memchr::memmem::find_iter(iso, needle).count()
}

/// The three blocks, in the order of the [`Layout`] offset fields: HP first,
/// then the twelve stats, then chargen.
const BLOCK_NAMES: [&str; 3] = ["HPMAX", "stats", "chargen"];

/// Authored discs crown 60 rows and patched discs crown 638, so more than this
/// many crowned rows can only mean the disc was patched already.
const ALREADY_MODDED_ROWS: usize = 100;

/// What [`inspect`] found on a disc: its size, how many copies of each authored
/// block it carries, how many rows already carry a crown, and the table itself.
///
/// A report only exists for a disc that passed every check, so `already_modded`
/// is always `false` here; the field is kept because the GUI shows it.
#[derive(PartialEq, Debug, Clone)]
pub struct DiscReport {
    pub size: u64,
    pub copies: [usize; 3],
    pub rarity_nonzero: usize,
    pub already_modded: bool,
    pub authored: EnemyTable,
}

/// Reads the disc at `path` and refuses it unless it is an unpatched copy of
/// the release `layout` describes. Nothing is written.
pub fn inspect(path: &Path, layout: &Layout) -> Result<DiscReport> {
    inspect_bytes(&std::fs::read(path)?, layout)
}

/// The checks [`inspect`] applies, in order, against an image already in memory.
///
/// 1. the three authored blocks parse, else [`Error::NotThisDisc`];
/// 2. each block occurs exactly [`Layout::expected_copies`] times, else
///    [`Error::WrongRevision`] naming the block;
/// 3. more than 100 rows carry a crown, then the report is returned, otherwise
///    [`Error::AlreadyModded`].
pub fn inspect_bytes(iso: &[u8], layout: &Layout) -> Result<DiscReport> {
    let authored = EnemyTable::read(iso, layout)?;

    let blocks = authored.blocks();
    let mut copies = [0usize; 3];
    for (found, block) in copies.iter_mut().zip(blocks.iter()) {
        *found = count_copies(iso, block);
    }
    for (&found, block) in copies.iter().zip(BLOCK_NAMES) {
        if found != layout.expected_copies {
            return Err(Error::WrongRevision {
                block,
                found,
                expected: layout.expected_copies,
            });
        }
    }

    let rarity_nonzero = authored
        .rarity
        .iter()
        .filter(|&&rarity| rarity != 0)
        .count();
    let already_modded = rarity_nonzero > ALREADY_MODDED_ROWS;
    if already_modded {
        return Err(Error::AlreadyModded { rarity_nonzero });
    }

    Ok(DiscReport {
        size: iso.len() as u64,
        copies,
        rarity_nonzero,
        already_modded,
        authored,
    })
}
