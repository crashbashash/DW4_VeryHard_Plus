use crate::error::{Error, Result};
use crate::layout::Layout;

/// The enemy stat table read out of the disc's three authored blocks.
///
/// The fields mirror the authored encodings: `hp` is one `i32` per row, `stat`
/// is twelve `i16` values per row, and `crit`/`para`/`rarity` are the three
/// bytes per row that chargen reads.
#[derive(Debug, Clone, PartialEq)]
pub struct EnemyTable {
    pub hp: Vec<i32>,
    pub stat: Vec<[i16; 12]>,
    pub crit: Vec<u8>,
    pub para: Vec<u8>,
    pub rarity: Vec<u8>,
}

impl EnemyTable {
    /// Reads every row of the three blocks out of `iso`.
    ///
    /// All three regions are bounds-checked before any byte is read, so a
    /// buffer that cannot hold the whole table is reported as
    /// [`Error::NotThisDisc`] instead of panicking part-way through.
    pub fn read(iso: &[u8], layout: &Layout) -> Result<Self> {
        let rows = layout.rows;
        let region_end = |start: u64, per_row: u64| {
            (rows as u64)
                .checked_mul(per_row)
                .and_then(|len| start.checked_add(len))
        };
        let hp_end = region_end(layout.hp_off, 4);
        let stat_end = region_end(layout.stat_off, 24);
        let byte_end = region_end(layout.byte_off, 3);
        let len = iso.len() as u64;
        let fits = |end: Option<u64>| end.is_some_and(|e| e <= len);
        if !fits(hp_end) || !fits(stat_end) || !fits(byte_end) {
            return Err(Error::NotThisDisc);
        }

        let hp_off = layout.hp_off as usize;
        let stat_off = layout.stat_off as usize;
        let byte_off = layout.byte_off as usize;

        let mut hp = Vec::with_capacity(rows);
        let mut stat = Vec::with_capacity(rows);
        let mut crit = Vec::with_capacity(rows);
        let mut para = Vec::with_capacity(rows);
        let mut rarity = Vec::with_capacity(rows);

        for r in 0..rows {
            let hp_at = hp_off + 4 * r;
            hp.push(i32::from_le_bytes(
                iso[hp_at..hp_at + 4].try_into().unwrap(),
            ));

            let mut row = [0i16; 12];
            for (c, slot) in row.iter_mut().enumerate() {
                let at = stat_off + 24 * r + 2 * c;
                *slot = i16::from_le_bytes(iso[at..at + 2].try_into().unwrap());
            }
            stat.push(row);

            let bytes_at = byte_off + 3 * r;
            crit.push(iso[bytes_at]);
            para.push(iso[bytes_at + 1]);
            rarity.push(iso[bytes_at + 2]);
        }

        Ok(Self {
            hp,
            stat,
            crit,
            para,
            rarity,
        })
    }

    pub fn len(&self) -> usize {
        self.hp.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hp.is_empty()
    }

    /// Encodes the three blocks back into ISO bytes, in the same order as the
    /// [`Layout`] fields: HP, then the twelve stats, then chargen.
    ///
    /// This is the exact inverse of [`EnemyTable::read`].
    pub fn blocks(&self) -> [Vec<u8>; 3] {
        let mut hp = Vec::with_capacity(self.hp.len() * 4);
        for value in &self.hp {
            hp.extend_from_slice(&value.to_le_bytes());
        }

        let mut stat = Vec::with_capacity(self.stat.len() * 24);
        for row in &self.stat {
            for value in row {
                stat.extend_from_slice(&value.to_le_bytes());
            }
        }

        let mut chargen = Vec::with_capacity(self.len() * 3);
        for r in 0..self.len() {
            chargen.extend_from_slice(&[self.crit[r], self.para[r], self.rarity[r]]);
        }

        [hp, stat, chargen]
    }
}
