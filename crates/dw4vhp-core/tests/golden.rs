//! Golden test: prove the engine reproduces two real, shipped discs
//! byte-for-byte.
//!
//! The chain self-validates by running in both directions. First the shipped
//! disc is reverted with its recorded original bytes, and the result must be
//! the pristine master's md5 — that proves the reconstruction is genuine rather
//! than merely plausible. Then the pristine master is patched with each preset
//! and its md5 is checked against the shipped builds.
//!
//! The real discs are never stored in the repository, so this test skips
//! cleanly when the three `DW4_GOLDEN_*` environment variables are unset; CI
//! has no discs. When they are set, the shipped disc is opened read-only and
//! every output is a new file under a `TempDir`.

use dw4vhp_core::patch::{patch_file, PatchOptions};
use dw4vhp_core::plan::PatchPlan;
use dw4vhp_core::serial::DEFAULT_SERIAL;
use dw4vhp_core::testkit;
use md5::{Digest, Md5};
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Chunk size for everything that walks a 1.4 GB image, so none of them load
/// it into memory.
const CHUNK: usize = 4 * 1024 * 1024;

fn env_paths() -> Option<(PathBuf, PathBuf, PathBuf)> {
    let get = |key: &str| std::env::var_os(key).map(PathBuf::from);
    let shipped = get("DW4_GOLDEN_SHIPPED_ISO");
    let blocks_bin = get("DW4_GOLDEN_BLOCKS_BIN");
    let blocks_json = get("DW4_GOLDEN_BLOCKS_JSON");
    match (shipped, blocks_bin, blocks_json) {
        (Some(shipped), Some(blocks_bin), Some(blocks_json)) => {
            Some((shipped, blocks_bin, blocks_json))
        }
        _ => {
            eprintln!("SKIP: golden discs not configured");
            None
        }
    }
}

/// Hex MD5 of `path`, streamed so a 1.4 GB image is not loaded into memory.
fn md5_file(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    format!("{:x}", hasher.finalize())
}

/// Restores the pristine master by copying `shipped` to a new file at
/// `pristine` and writing the recorded original regions back over it.
///
/// `blocks_json` is the `{"writes": [[offset, len], ...], ...}` manifest the
/// reference patcher produced, and `blocks_bin` is the concatenation of each
/// region's original bytes in that same order. The shipped disc is mapped
/// read-only and never written; the backup blob is 13 MB, so it is the only
/// input read whole.
fn revert_with_backup(shipped: &Path, pristine: &Path, blocks_bin: &Path, blocks_json: &Path) {
    let json = std::fs::read_to_string(blocks_json).unwrap();
    let writes = parse_writes(&json);

    let src = File::open(shipped).unwrap();
    // Safety: the mapping is read-only and `shipped` is never written.
    let mmap = unsafe { Mmap::map(&src).unwrap() };

    let mut dst = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(pristine)
        .unwrap();

    let mut written = 0usize;
    while written < mmap.len() {
        let end = (written + CHUNK).min(mmap.len());
        dst.write_all(&mmap[written..end]).unwrap();
        written = end;
    }

    let blob = std::fs::read(blocks_bin).unwrap();
    let mut cursor = 0usize;
    for (offset, len) in writes {
        let len = len as usize;
        assert!(
            offset + len as u64 <= mmap.len() as u64,
            "backup region runs past the end of the disc"
        );
        assert!(
            cursor + len <= blob.len(),
            "backup blob is shorter than the writes manifest"
        );
        dst.seek(SeekFrom::Start(offset)).unwrap();
        dst.write_all(&blob[cursor..cursor + len]).unwrap();
        cursor += len;
    }
    assert_eq!(cursor, blob.len(), "backup blob has trailing bytes");
}

/// Parses the `writes` array out of the backup JSON: `[[offset, len], ...]`.
///
/// The manifest has one flat array of two-element integer arrays, so a
/// dependency-free scan is enough; the values are never negative and the
/// surrounding keys are ignored.
fn parse_writes(json: &str) -> Vec<(u64, u64)> {
    let bytes = json.as_bytes();
    let key = "\"writes\"";
    let key_pos = json.find(key).expect("backup JSON must carry a writes key");
    let mut i = key_pos + key.len();
    while i < bytes.len() && bytes[i] != b'[' {
        i += 1;
    }
    assert!(i < bytes.len(), "backup JSON writes array not found");
    i += 1; // enter the writes array

    let mut writes = Vec::new();
    loop {
        skip_whitespace(bytes, &mut i);
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b']' => break,
            b'[' => {
                i += 1;
                skip_whitespace(bytes, &mut i);
                let offset = read_number(bytes, &mut i);
                skip_whitespace(bytes, &mut i);
                assert_eq!(bytes[i], b',', "malformed writes entry");
                i += 1;
                skip_whitespace(bytes, &mut i);
                let len = read_number(bytes, &mut i);
                skip_whitespace(bytes, &mut i);
                assert_eq!(bytes[i], b']', "malformed writes entry");
                i += 1;
                writes.push((offset, len));
            }
            b',' => i += 1,
            _ => i += 1,
        }
    }
    writes
}

fn skip_whitespace(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn read_number(bytes: &[u8], i: &mut usize) -> u64 {
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    assert!(*i > start, "expected a number");
    std::str::from_utf8(&bytes[start..*i])
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn golden_chain_reproduces_the_shipped_builds() {
    let Some((shipped, blocks_bin, blocks_json)) = env_paths() else {
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let pristine = tmp.path().join("pristine.iso");

    // Step 1: revert the shipped disc with the recorded regions and assert the
    // result is the pristine master.
    revert_with_backup(&shipped, &pristine, &blocks_bin, &blocks_json);
    let pristine_md5 = md5_file(&pristine);
    eprintln!("step 1 pristine md5: {pristine_md5}");
    assert_eq!(pristine_md5, "3185f04230b1dabd853db750e4b51108");

    // Step 2: the default Very Hard Plus preset reproduces the practice-buff
    // disc.
    let out = tmp.path().join("default.iso");
    patch_file(
        &pristine,
        &out,
        &PatchPlan::default(),
        &PatchOptions { overwrite: false },
        &mut |_| {},
    )
    .unwrap();
    let default_md5 = md5_file(&out);
    eprintln!("step 2 default md5: {default_md5}");
    assert_eq!(default_md5, "df426b9ff17c9e8218779f8ff1ecf524");

    // Step 3: practice buff off reproduces the shipped v5 disc.
    let no_practice = PatchPlan {
        practice_buff: false,
        ..PatchPlan::default()
    };
    let out3 = tmp.path().join("v5b4.iso");
    patch_file(
        &pristine,
        &out3,
        &no_practice,
        &PatchOptions { overwrite: false },
        &mut |_| {},
    )
    .unwrap();
    let no_practice_md5 = md5_file(&out3);
    eprintln!("step 3 practice_buff=false md5: {no_practice_md5}");
    assert_eq!(no_practice_md5, "f760258575f4fe2de05a9930e0a6b396");

    // Step 4: the serial step changes exactly the two serial ranges, and the
    // shared table rewrite cancels between the two patched outputs.
    let serial = PatchPlan {
        serial: Some(DEFAULT_SERIAL.into()),
        ..PatchPlan::default()
    };
    let out4 = tmp.path().join("serial.iso");
    patch_file(
        &pristine,
        &out4,
        &serial,
        &PatchOptions { overwrite: false },
        &mut |_| {},
    )
    .unwrap();
    let changed = testkit::changed_offsets(&out, &out4);
    let (a, b) = testkit::serial_offsets(&out).unwrap();
    eprintln!("step 4 serial changed runs: {changed:?} (serial ranges {a:#X}, {b:#X})");
    assert!(!changed.is_empty(), "the serial step produced no changes");
    assert!(
        changed
            .iter()
            .all(|o| (a..a + 13).contains(o) || (b..b + 11).contains(o)),
        "{changed:?}"
    );

    // Step 5: the ELF step changes exactly four bytes, at the difficulty word.
    let extreme = PatchPlan {
        force_very_hard: true,
        ..PatchPlan::default()
    };
    let out5 = tmp.path().join("extreme.iso");
    patch_file(
        &pristine,
        &out5,
        &extreme,
        &PatchOptions { overwrite: false },
        &mut |_| {},
    )
    .unwrap();
    let elf = testkit::elf_region_offset(&out).unwrap();
    let elf_changed = testkit::changed_offsets(&out, &out5);
    eprintln!("step 5 force_very_hard changed runs: {elf_changed:?} (elf at {elf:#X})");
    assert_eq!(elf_changed, vec![elf]);
    assert_eq!(
        testkit::changed_byte_count(&out, &out5),
        4,
        "the difficulty patch must change exactly four bytes"
    );
}
