//! Every decision the window makes, as plain functions: which preset a plan
//! stands for, what the output file should be called, how a refused disc is
//! explained, and whether to warn about the attack ceiling.
//!
//! Nothing here imports `egui`, `eframe` or `rfd`. That is what keeps the
//! module testable with no display and no event loop; the drawing code lives in
//! `main.rs` (and, from Task 13 on, `app.rs`).

use dw4vhp_core::disc::DiscReport;
use dw4vhp_core::error::{Error, Result};
use dw4vhp_core::plan::{PatchPlan, PlanSummary, Preset};
use dw4vhp_core::serial::DEFAULT_SERIAL;
use std::path::{Path, PathBuf};

/// The disc this patcher supports, named as the refusals name it.
const SUPPORTED_RELEASE: &str = "Digimon World 4 (USA) SLUS_208.36";

/// The marker a patched image's name carries, so it can never be mistaken for
/// the original.
const OUTPUT_SUFFIX: &str = " [VeryHardPlus].iso";

/// The `i16` ceiling every overflowing attack value clamps to.
const ATTACK_CEILING: i16 = i16::MAX;

/// What the window holds between frames. `plan` is the source of truth for what
/// will be written; `preset` is only the label the plan currently matches.
#[derive(Debug)]
pub struct UiState {
    pub input: Option<PathBuf>,
    pub output: PathBuf,
    pub preset: Preset,
    pub plan: PatchPlan,
    pub overwrite: bool,
}

impl Default for UiState {
    /// Nothing chosen yet: the `Very Hard Plus` plan, which is what a first run
    /// offers, and an empty output path that fills in once an input is picked.
    fn default() -> Self {
        Self {
            input: None,
            output: PathBuf::new(),
            preset: Preset::VeryHardPlus,
            plan: PatchPlan::default(),
            overwrite: false,
        }
    }
}

impl UiState {
    /// Re-detects the preset after any edit to the plan, so touching a single
    /// knob reports `Custom` instead of a named build the plan no longer is.
    pub fn on_plan_edited(&mut self) {
        self.preset = Preset::detect(&self.plan);
    }

    /// Selecting a preset loads that preset's whole plan, so the Advanced
    /// panel always shows the values the selected build will write. Selecting
    /// `Custom` keeps the current plan — it has no canonical values to load.
    /// A preset never carries a serial edit, so the serial controls reset too.
    pub fn apply_preset(
        &mut self,
        preset: Preset,
        serial_enabled: &mut bool,
        serial_text: &mut String,
    ) {
        self.preset = preset;
        if preset != Preset::Custom {
            self.plan = preset.plan();
            *serial_enabled = false;
            *serial_text = DEFAULT_SERIAL.to_string();
        }
    }
}

/// How to read a [`StatusLine`]: a disc that can be patched, a disc the app
/// refuses to patch, or anything else worth saying.
#[derive(PartialEq, Eq, Debug)]
pub enum StatusKind {
    Ok,
    Refused,
    Unknown,
}

/// One line of feedback, shown under the input row.
#[derive(PartialEq, Eq, Debug)]
pub struct StatusLine {
    pub kind: StatusKind,
    pub text: String,
}

/// The output path an input suggests: the same directory, the input's own stem
/// plus the `[VeryHardPlus]` marker, so the default never targets the original
/// image.
pub fn default_output_path(input: &Path) -> PathBuf {
    let mut name = input.file_stem().unwrap_or_default().to_os_string();
    name.push(OUTPUT_SUFFIX);
    input.with_file_name(name)
}

/// The line to show for an inspection result: a clean disc and its authored
/// copy counts, or a refusal that says what to do instead — every refusal
/// reuses the engine's own message, which names the fix.
pub fn status_for(result: &Result<DiscReport>) -> StatusLine {
    match result {
        Ok(report) => StatusLine {
            kind: StatusKind::Ok,
            text: format!(
                "clean {SUPPORTED_RELEASE}: {} HPMAX, {} stats and {} chargen copies",
                report.copies[0], report.copies[1], report.copies[2]
            ),
        },
        Err(error) => StatusLine {
            kind: match error {
                Error::AlreadyModded { .. } | Error::WrongRevision { .. } | Error::NotThisDisc => {
                    StatusKind::Refused
                }
                // Everything else here is a read or layout problem, not a
                // verdict on the disc; the message still says what to do.
                _ => StatusKind::Unknown,
            },
            text: error.to_string(),
        },
    }
}

/// The note about the attack column's ceiling, or `None` when the plan pins
/// nothing and there is nothing to warn about.
///
/// `summary.live_rows` is the authored table's live-row base, so the note
/// states how much of the table the clamp costs rather than a bare count.
pub fn attack_pin_note(summary: &PlanSummary) -> Option<String> {
    (summary.attack_pinned > 0).then(|| {
        format!(
            "attack: {} of {} live rows pinned at {}",
            summary.attack_pinned, summary.live_rows, ATTACK_CEILING
        )
    })
}

/// The note about the row collapse, or `None` when the plan does not collapse.
///
/// The attack-pin note stays a scaling-time figure (spec §12.2), so this line
/// is what tells a player that the delivered table is nearly uniform.
pub fn collapse_note(summary: &PlanSummary) -> Option<String> {
    (summary.collapsed_rows > 0).then(|| {
        format!(
            "collapse: {} rows now carry their type's strongest row — every enemy of a type is \
             identical, EXP included",
            summary.collapsed_rows
        )
    })
}
