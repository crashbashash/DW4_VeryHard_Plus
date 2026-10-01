//! The composition entry point the GUI calls: inspect the disc, derive the
//! factors from it, compute the new table, turn that — plus the optional ELF
//! and serial steps — into same-length write regions, write them onto a copy
//! and verify the result before it is renamed into place.
//!
//! The input image is opened read-only and memory-mapped so a 1.4 GB retail
//! ISO is never loaded into memory, and it is never written: every edit lands
//! on the output copy, in place, at exactly the length it replaces.

use crate::disc::{count_copies, inspect_bytes, DiscReport};
use crate::elf::{difficulty_region, ElfInfo};
use crate::error::{Error, Result};
use crate::layout::Layout;
use crate::plan::{transform, PatchPlan, PlanSummary};
use crate::ratio::RatioVector;
use crate::region::WriteRegion;
use crate::serial::serial_regions;
use crate::table::EnemyTable;
use crate::writer::{write_output, Progress, WriteOptions};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// The three blocks, in the order of the [`Layout`] offset fields: HP first,
/// then the twelve stats, then chargen. Mirrors [`crate::disc`]'s names so the
/// refusals name the same block the inspection did.
const BLOCK_NAMES: [&str; 3] = ["HPMAX", "stats", "chargen"];

/// Controls whether an existing output may be replaced.
#[derive(PartialEq, Debug, Clone)]
pub struct PatchOptions {
    pub overwrite: bool,
}

/// The result of a successful patch: the final file's MD5, its length and the
/// work the transform reported.
#[derive(PartialEq, Debug, Clone)]
pub struct PatchOutcome {
    pub md5: String,
    pub bytes: u64,
    pub summary: PlanSummary,
}

/// Applies `plan` to the disc's authored table and returns the transformed
/// table with the transform's summary.
fn derive_patch(report: &DiscReport, plan: &PatchPlan) -> (EnemyTable, PlanSummary) {
    let ratio = RatioVector::derive(&report.authored);
    transform(&report.authored, plan, &ratio)
}

/// Turns an inspected disc into the write regions a patch applies: one region
/// per authored block copy (carrying the computed replacement bytes), plus the
/// ELF difficulty word when `plan.force_very_hard` is set and the two serial
/// rewrites when `plan.serial` is set.
///
/// The authored block copies are located by searching `iso` for the authored
/// blocks, so the retail disc's scattered copies are all found without
/// hardcoding their offsets. A block that does not occur exactly
/// [`Layout::expected_copies`] times is [`Error::WrongRevision`].
pub fn plan_regions(
    iso: &[u8],
    report: &DiscReport,
    plan: &PatchPlan,
    layout: &Layout,
) -> Result<(Vec<WriteRegion>, PlanSummary)> {
    let (new_table, summary) = derive_patch(report, plan);
    let new_blocks = new_table.blocks();
    let old_blocks = report.authored.blocks();

    let mut regions = Vec::new();
    for (index, (new_block, old_block)) in new_blocks.iter().zip(old_blocks.iter()).enumerate() {
        let occurrences: Vec<u64> = memchr::memmem::find_iter(iso, old_block)
            .map(|offset| offset as u64)
            .collect();
        if occurrences.len() != layout.expected_copies {
            return Err(Error::WrongRevision {
                block: BLOCK_NAMES[index],
                found: occurrences.len(),
                expected: layout.expected_copies,
            });
        }
        for offset in occurrences {
            regions.push(WriteRegion {
                offset,
                bytes: new_block.clone(),
            });
        }
    }

    if plan.force_very_hard {
        let boot_elf = report.boot_elf.as_ref().ok_or(Error::ElfNotFound)?;
        let elf = ElfInfo::read(iso, boot_elf)?;
        regions.push(difficulty_region(iso, &elf)?);
    }

    if let Some(serial) = &plan.serial {
        let boot_elf = report.boot_elf.as_ref().ok_or(Error::FileNotFound {
            name: "SLUS_208.36".to_string(),
        })?;
        regions.extend(serial_regions(iso, boot_elf, serial)?);
    }

    Ok((regions, summary))
}

/// The app's entry point: patches `input` into `output` against the retail
/// [`Layout::retail`] layout.
pub fn patch_file(
    input: &Path,
    output: &Path,
    plan: &PatchPlan,
    opts: &PatchOptions,
    progress: &mut dyn FnMut(Progress),
) -> Result<PatchOutcome> {
    patch_file_with_layout(input, output, plan, opts, &Layout::retail(), progress)
}

/// The layout-parameterised seam the GUI worker and the tests use. `patch_file`
/// delegates here so every caller takes the same inspect → plan → write →
/// verify path.
pub fn patch_file_with_layout(
    input: &Path,
    output: &Path,
    plan: &PatchPlan,
    opts: &PatchOptions,
    layout: &Layout,
    progress: &mut dyn FnMut(Progress),
) -> Result<PatchOutcome> {
    let file = File::open(input)?;
    // Safety: the mapping is read-only and `input` is never written.
    let mmap = unsafe { Mmap::map(&file)? };

    let report = inspect_bytes(&mmap[..], layout)?;
    let (new_table, _) = derive_patch(&report, plan);
    let blocks = new_table.blocks();
    let (regions, summary) = plan_regions(&mmap[..], &report, plan, layout)?;

    let write_opts = WriteOptions {
        overwrite: opts.overwrite,
    };
    let verify = |path: &Path| verify_output(path, &blocks, layout);

    let outcome = write_output(input, output, &regions, &write_opts, progress, &verify)?;
    Ok(PatchOutcome {
        md5: outcome.md5,
        bytes: outcome.bytes,
        summary,
    })
}

/// Re-reads the written file and confirms the computed table blocks are exactly
/// what landed: the anchors hold them byte-for-byte and each occurs
/// [`Layout::expected_copies`] times.
fn verify_output(path: &Path, blocks: &[Vec<u8>; 3], layout: &Layout) -> Result<()> {
    let file = File::open(path)?;
    // Safety: the mapping is read-only and `path` is not modified while mapped.
    let mmap = unsafe { Mmap::map(&file)? };
    verify_blocks(&mmap[..], blocks, layout)
}

/// The block-level checks [`verify_output`] applies against an image in memory.
fn verify_blocks(iso: &[u8], blocks: &[Vec<u8>; 3], layout: &Layout) -> Result<()> {
    let anchors = [layout.hp_off, layout.stat_off, layout.byte_off];
    for (index, (block, &anchor)) in blocks.iter().zip(anchors.iter()).enumerate() {
        let name = BLOCK_NAMES[index];
        let start = usize::try_from(anchor).map_err(|_| {
            Error::VerificationFailed(format!("the {name} block offset {anchor:#X} is too large"))
        })?;
        let end = start.checked_add(block.len()).ok_or_else(|| {
            Error::VerificationFailed(format!(
                "the {name} block at {anchor:#X} overflows the file offset range"
            ))
        })?;
        let actual = iso.get(start..end).ok_or_else(|| {
            Error::VerificationFailed(format!(
                "the {name} block at {anchor:#X} runs past the end of the written file"
            ))
        })?;
        if actual != block.as_slice() {
            return Err(Error::VerificationFailed(format!(
                "the {name} block at {anchor:#X} does not match the computed patch"
            )));
        }
    }

    for (index, block) in blocks.iter().enumerate() {
        let name = BLOCK_NAMES[index];
        let found = count_copies(iso, block);
        if found != layout.expected_copies {
            return Err(Error::VerificationFailed(format!(
                "found {found} copies of the {name} block in the written file, expected {}",
                layout.expected_copies
            )));
        }
    }

    Ok(())
}
