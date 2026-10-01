use crate::ratio::RatioVector;
use crate::table::EnemyTable;

/// Authored rows for the destructibles (`g_*` crates and barrels), left
/// byte-identical so they still break normally (spec §3.4).
pub const DESTRUCTIBLE_ROWS: std::ops::RangeInclusive<usize> = 370..=378;

/// The `e_cockatri` tutorial pair: a deliberate tripwire, left byte-identical so
/// a base-stat Kokatorimon proves which row a spawn took (spec §3.4).
pub const TRIPWIRE_ROWS: [usize; 2] = [644, 645];

/// Practice-stage overrides, `(destination, source)`: the tutorial rows
/// 638–643 are authored as token values, so each is replaced by an ordinary
/// tier-0 row of the same type. Sources are read from the *computed* table
/// (after scaling) and the whole row is copied. Rows 644/645 are not in this
/// map, so the tripwire survives the buff (spec §3.6).
pub const PRACTICE_MAP: [(usize, usize); 6] = [
    (638, 0),
    (639, 1),
    (640, 33),
    (641, 34),
    (642, 12),
    (643, 13),
];

/// HP is an `i32` that must never go negative, so its clamp floor is 0.
const HP_FLOOR: f64 = 0.0;
const HP_CAP: f64 = 400_000.0;
const STAT_MIN: f64 = -32768.0;
const STAT_MAX: f64 = 32767.0;
const CHARGEN_FLOOR: f64 = 0.0;
const CHARGEN_CAP: f64 = 60.0;

/// The whole surface the GUI edits: the multipliers applied on top of the
/// derived [`RatioVector`], the crown rank, and the exclusions/toggles that
/// define a build (spec §7).
#[derive(PartialEq, Debug, Clone)]
pub struct PatchPlan {
    pub hp_mult: f64,
    pub stat_mult: [f64; 12],
    pub crit_mult: f64,
    pub para_mult: f64,
    pub crown_rank: u8,
    pub exclude_destructibles: bool,
    pub exclude_tripwire: bool,
    pub practice_buff: bool,
    pub force_very_hard: bool,
    pub serial: Option<String>,
}

impl Default for PatchPlan {
    /// The Very Hard Plus preset: derived factors unchanged, crown rank 5, both
    /// exclusions and the practice buff on, no ELF patch and no serial edit.
    fn default() -> Self {
        Self {
            hp_mult: 1.0,
            stat_mult: [1.0; 12],
            crit_mult: 1.0,
            para_mult: 1.0,
            crown_rank: 5,
            exclude_destructibles: true,
            exclude_tripwire: true,
            practice_buff: true,
            force_very_hard: false,
            serial: None,
        }
    }
}

/// The named build presets. `Custom` is what the GUI shows once any field is
/// edited away from a named preset (spec §8).
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Preset {
    VeryHardPlus,
    Extreme,
    Custom,
}

impl Preset {
    /// The plan this preset stands for.
    ///
    /// `Custom` has no canonical plan — it is a label the GUI applies to an
    /// edited plan — so it yields the default plan as an editing baseline.
    pub fn plan(self) -> PatchPlan {
        match self {
            Preset::Extreme => PatchPlan {
                force_very_hard: true,
                ..PatchPlan::default()
            },
            Preset::VeryHardPlus | Preset::Custom => PatchPlan::default(),
        }
    }

    /// The preset a plan is exactly equal to, or `Custom` for any other plan.
    pub fn detect(p: &PatchPlan) -> Preset {
        if *p == Preset::VeryHardPlus.plan() {
            Preset::VeryHardPlus
        } else if *p == Preset::Extreme.plan() {
            Preset::Extreme
        } else {
            Preset::Custom
        }
    }

    /// The GUI-facing name, pinned so the window and the reports agree.
    pub fn label(self) -> &'static str {
        match self {
            Preset::VeryHardPlus => "Very Hard Plus",
            Preset::Extreme => "Very Hard Plus — Extreme",
            Preset::Custom => "Custom",
        }
    }
}

/// What a single [`transform`] changed, for the pre-flight Analyze report.
#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct PlanSummary {
    pub rows_changed: usize,
    pub attack_pinned: usize,
    pub hp_capped: usize,
    pub crit_pinned: usize,
    pub practice_rows: usize,
    pub elf_step: bool,
    pub serial_step: bool,
}

fn is_excluded(r: usize, plan: &PatchPlan) -> bool {
    (plan.exclude_destructibles && DESTRUCTIBLE_ROWS.contains(&r))
        || (plan.exclude_tripwire && TRIPWIRE_ROWS.contains(&r))
}

/// Applies `plan` and `ratio` to `authored`, returning the table that gets
/// written back and a count of what happened (spec §3.3, §3.4, §3.6).
///
/// Rounding is ties-to-even throughout, matching the Python patcher that
/// produced the shipped discs. Excluded rows are copied through untouched,
/// including their rarity.
pub fn transform(
    authored: &EnemyTable,
    plan: &PatchPlan,
    ratio: &RatioVector,
) -> (EnemyTable, PlanSummary) {
    let mut out = authored.clone();
    let mut summary = PlanSummary {
        elf_step: plan.force_very_hard,
        serial_step: plan.serial.is_some(),
        ..PlanSummary::default()
    };

    for r in 0..authored.len() {
        if is_excluded(r, plan) {
            continue;
        }
        summary.rows_changed += 1;

        let hp = (authored.hp[r] as f64 * ratio.hp * plan.hp_mult).round_ties_even();
        if hp > HP_CAP {
            summary.hp_capped += 1;
        }
        out.hp[r] = hp.clamp(HP_FLOOR, HP_CAP) as i32;

        for (c, slot) in out.stat[r].iter_mut().enumerate() {
            let value =
                (authored.stat[r][c] as f64 * ratio.stat[c] * plan.stat_mult[c]).round_ties_even();
            if c == 0 && value > STAT_MAX {
                summary.attack_pinned += 1;
            }
            *slot = value.clamp(STAT_MIN, STAT_MAX) as i16;
        }

        let crit = (authored.crit[r] as f64 + ratio.crit_delta * plan.crit_mult).round_ties_even();
        let para = (authored.para[r] as f64 + ratio.para_delta * plan.para_mult).round_ties_even();
        if crit > CHARGEN_CAP || para > CHARGEN_CAP {
            summary.crit_pinned += 1;
        }
        out.crit[r] = crit.clamp(CHARGEN_FLOOR, CHARGEN_CAP) as u8;
        out.para[r] = para.clamp(CHARGEN_FLOOR, CHARGEN_CAP) as u8;

        // Every handled row gets the crown, including the 69 empty rows.
        out.rarity[r] = plan.crown_rank;
    }

    if plan.practice_buff {
        for &(dest, src) in &PRACTICE_MAP {
            out.hp[dest] = out.hp[src];
            out.stat[dest] = out.stat[src];
            out.crit[dest] = out.crit[src];
            out.para[dest] = out.para[src];
            out.rarity[dest] = out.rarity[src];
            summary.practice_rows += 1;
        }
    }

    (out, summary)
}
