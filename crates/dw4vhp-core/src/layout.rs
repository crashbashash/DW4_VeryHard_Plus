/// Byte layout of the three authored blocks inside an ISO.
///
/// Every block's copy is contiguous: HP is `rows × i32` column-major, the twelve
/// stats are `rows × 12 × i16` row-major, and crit/paralysis/rarity are
/// `rows × 3 × u8` row-major (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub hp_off: u64,
    pub stat_off: u64,
    pub byte_off: u64,
    pub rows: usize,
    pub expected_copies: usize,
}

/// ISO offset where `mini` places the first block: above the ISO9660 metadata
/// area, so one fixture can carry both a table and a volume descriptor.
const MINI_BASE: u64 = 0x20000;

impl Layout {
    /// The retail NTSC-U disc's block offsets, row count and copy count.
    pub fn retail() -> Self {
        Self {
            hp_off: 0x78D64C5,
            stat_off: 0x78DACB9,
            byte_off: 0x78DA51D,
            rows: 649,
            expected_copies: 665,
        }
    }

    /// A compact layout with the same encodings, used by generated test
    /// fixtures: the three blocks start contiguously at 0x20000 and the row/copy
    /// counts are caller-chosen.
    pub fn mini(rows: usize, copies: usize) -> Self {
        let hp_off = MINI_BASE;
        let stat_off = hp_off + (rows * 4) as u64;
        let byte_off = stat_off + (rows * 24) as u64;
        Self {
            hp_off,
            stat_off,
            byte_off,
            rows,
            expected_copies: copies,
        }
    }
}
