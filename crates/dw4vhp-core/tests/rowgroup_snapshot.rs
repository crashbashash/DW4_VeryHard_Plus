//! Proves the committed row groups are the ones the runtime table carries.
//!
//! Skips cleanly unless `DW4_ROWGROUP_SNAPSHOT` points at a RAM snapshot of the
//! authored table (spec §12.1). The snapshot is outside the repository, like
//! the discs: no game data is committed.
use dw4vhp_core::rowgroup::ROW_GROUPS;
use std::path::PathBuf;

const DB: usize = 0x174_8840;
const ROWS: usize = 649;

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// The authored HP of `row`, through the table's i32 container.
fn hp_of(bytes: &[u8], row: usize) -> i32 {
    i32_at(
        bytes,
        u32_at(bytes, u32_at(bytes, DB + 0x1C) + 0x10) + 4 * row,
    )
}

/// The `MODEL` string of `row`, or `None` for an unfilled slot.
fn model_of(bytes: &[u8], row: usize) -> Option<String> {
    let table = u32_at(bytes, u32_at(bytes, DB + 0x34) + 0x10);
    let entry = u32_at(bytes, table + 4 * row);
    if entry == 0 {
        return None;
    }
    let name = u32_at(bytes, entry + 8);
    if !(0x10_0000..0x200_0000).contains(&name) || name + 64 > bytes.len() {
        return None;
    }
    let end = bytes[name..name + 64].iter().position(|&b| b == 0)? + name;
    let text = std::str::from_utf8(&bytes[name..end]).ok()?;
    (!text.is_empty()).then(|| text.to_string())
}

#[test]
fn the_committed_groups_are_the_runtime_tables_groups() {
    let Some(path) = std::env::var_os("DW4_ROWGROUP_SNAPSHOT").map(PathBuf::from) else {
        eprintln!("SKIP: DW4_ROWGROUP_SNAPSHOT is not set");
        return;
    };
    let bytes = std::fs::read(&path).unwrap();

    let mut derived: Vec<(String, Vec<usize>)> = Vec::new();
    for row in 0..ROWS {
        let Some(model) = model_of(&bytes, row) else {
            continue;
        };
        // A type's rows are not adjacent (`e_goburi` is 0-11, 379-393, 638-639),
        // so group by model in first-appearance order, which is first-row order.
        match derived.iter_mut().find(|(name, _)| *name == model) {
            Some((_, rows)) => rows.push(row),
            None => derived.push((model, vec![row])),
        }
    }

    assert_eq!(derived.len(), ROW_GROUPS.len(), "group count");
    for (expected, (model, rows)) in ROW_GROUPS.iter().zip(derived.iter()) {
        assert_eq!(expected.model, model, "group order and names");
        assert_eq!(expected.rows, rows.as_slice(), "{model} membership");
        let top = rows
            .iter()
            .copied()
            .max_by_key(|&r| hp_of(&bytes, r))
            .unwrap();
        assert_eq!(expected.top, top, "{model} top row");
    }
    assert_eq!(ROW_GROUPS.iter().map(|g| g.rows.len()).sum::<usize>(), 580);
}
