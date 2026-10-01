use crate::table::EnemyTable;

/// The four weakest authored elite rows the factors are measured from.
pub const REF_ORDINARY_ROWS: [usize; 4] = [0, 1, 2, 3];

/// The authored top-rank row every factor is measured against.
pub const REF_TOP_ROW: usize = 391;

/// The per-field multipliers that scale the authored elite table up to the
/// player's table, derived from the disc's own reference rows.
///
/// The factors stay unrounded in `f64`: rounding happens where they are
/// applied.
#[derive(PartialEq, Debug, Clone)]
pub struct RatioVector {
    pub hp: f64,
    pub stat: [f64; 12],
    pub crit_delta: f64,
    pub para_delta: f64,
}

/// Median of four values: the mean of the middle two after sorting.
///
/// This is deliberately neither the lower middle value nor an interpolation of
/// any other pair.
pub fn median4(v: [i32; 4]) -> f64 {
    let mut sorted = v;
    sorted.sort_unstable();
    (sorted[1] as f64 + sorted[2] as f64) / 2.0
}

impl RatioVector {
    /// Derives the factors by comparing the [`REF_ORDINARY_ROWS`] against
    /// [`REF_TOP_ROW`].
    ///
    /// A zero HP denominator is floored to 1 so the division cannot fail, and
    /// a zero per-column denominator yields a factor of 1.0 — the field is
    /// left as the top-rank row authored it.
    pub fn derive(t: &EnemyTable) -> Self {
        let hp_floor = REF_ORDINARY_ROWS
            .iter()
            .map(|&r| t.hp[r])
            .min()
            .unwrap_or(0);
        let hp = t.hp[REF_TOP_ROW] as f64 / hp_floor.max(1) as f64;

        let mut stat = [1.0f64; 12];
        for (c, factor) in stat.iter_mut().enumerate() {
            let floor = REF_ORDINARY_ROWS
                .iter()
                .map(|&r| t.stat[r][c])
                .min()
                .unwrap_or(0);
            if floor != 0 {
                *factor = t.stat[REF_TOP_ROW][c] as f64 / floor as f64;
            }
        }

        Self {
            hp,
            stat,
            crit_delta: t.crit[REF_TOP_ROW] as f64
                - median4(REF_ORDINARY_ROWS.map(|r| t.crit[r] as i32)),
            para_delta: t.para[REF_TOP_ROW] as f64
                - median4(REF_ORDINARY_ROWS.map(|r| t.para[r] as i32)),
        }
    }
}
