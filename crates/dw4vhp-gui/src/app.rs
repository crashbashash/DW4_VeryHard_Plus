//! The `eframe` window: one screen that picks a disc, shows what it detected,
//! lets the player choose a preset or edit the plan, and patches on a
//! background worker so a 1.4 GB copy never blocks the UI thread.
//!
//! Nothing in this module calls the engine on the UI thread. Analyze runs
//! [`inspect`] on its own worker and Patch runs [`spawn_patch`]; the only
//! engine code the UI runs directly is the pure [`transform`] used for the live
//! attack-ceiling note.

use crate::form::{
    attack_pin_note, collapse_note, default_output_path, status_for, StatusKind, StatusLine,
    UiState,
};
use crate::worker::{spawn_patch, WorkerHandle, WorkerMsg};
use dw4vhp_core::disc::{inspect, DiscReport};
use dw4vhp_core::error::Result as EngineResult;
use dw4vhp_core::layout::Layout;
use dw4vhp_core::plan::{transform, PlanSummary, Preset};
use dw4vhp_core::ratio::RatioVector;
use dw4vhp_core::serial::DEFAULT_SERIAL;
use dw4vhp_core::writer::{Phase, Progress};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;

/// The twelve authored `i16` columns, in the order the table stores them.
const STAT_NAMES: [&str; 12] = [
    "atk", "def", "wis", "spr", "spd", "fire", "ice", "thunder", "dark", "stun", "poison", "exp",
];

const CAVEAT_FORCE_VERY_HARD: &str =
    "Force Very Hard (used by the Extreme preset) has never been played. It is a single \
     instruction — the same change the original mod makes — and tests confirm it rewrites exactly \
     four bytes in the game's boot file and touches nothing else. What no test can show is the \
     result: this project was built without an emulator, so \"the game really does start on Very \
     Hard\" is an expectation, not something anyone has watched happen.";
const CAVEAT_VERY_HARD_TIER2: &str =
    "The Very Hard → tier 2 mapping is reasoned, not measured. Normal → tier 0 and Hard → tier 1 \
     were confirmed live in-game, on the valley bridge. The table is laid out as 3 tiers with 3–4 \
     variants each, and Very Hard's tier 2 slot follows from that layout — but it was never checked \
     the same way, so the Extreme preset's effect size is documented as inferred.";
const CAVEAT_MOD_OPEN_ITEMS: &str =
    "The original mod's own untested items still apply here: every boss row is unmeasured except \
     `e_mecha4` row 60; about 16 variant models have no row of their own; the 88-record \
     `beNDMWStatusInfo` table has never been explored; and the two-player graduation tripwire has \
     never been triggered.";
const CAVEAT_BRUTAL: &str =
    "The Brutal preset's enemy grouping comes from a memory snapshot taken while the game was \
     running, not from the disc — the disc does not record which rows belong to which enemy type. \
     Every enemy of a type is therefore made identical to that type's strongest row. Nobody has \
     played this preset: with no emulator here, the in-game result is unconfirmed.";

/// The window's whole state between frames.
pub struct App {
    state: UiState,
    input_text: String,
    output_text: String,
    serial_enabled: bool,
    serial_text: String,
    report: Option<EngineResult<DiscReport>>,
    summary: Option<PlanSummary>,
    analyze: Option<AnalyzeHandle>,
    worker: Option<WorkerHandle>,
    progress: Option<Progress>,
    log: Vec<String>,
}

/// A running [`inspect`] call, so Analyze never blocks the window either.
struct AnalyzeHandle {
    rx: Receiver<EngineResult<DiscReport>>,
    handle: JoinHandle<()>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: UiState::default(),
            input_text: String::new(),
            output_text: String::new(),
            serial_enabled: false,
            serial_text: DEFAULT_SERIAL.to_string(),
            report: None,
            summary: None,
            analyze: None,
            worker: None,
            progress: None,
            log: Vec::new(),
        }
    }

    fn apply_input_path(&mut self, path: PathBuf) {
        self.input_text = path.display().to_string();
        self.state.input = Some(path.clone());
        self.state.output = default_output_path(&path);
        self.output_text = self.state.output.display().to_string();
        self.report = None;
        self.summary = None;
        self.log.push(format!("chose disc {}", path.display()));
    }

    fn start_analyze(&mut self) {
        let Some(input) = self.state.input.clone() else {
            return;
        };
        self.log.push(format!("analyzing {}", input.display()));
        self.report = None;
        self.summary = None;
        self.analyze = Some(spawn_inspect(input));
    }

    fn start_patch(&mut self) {
        let Some(input) = self.state.input.clone() else {
            return;
        };
        let output = self.state.output.clone();
        self.sync_serial();
        let plan = self.state.plan.clone();
        let overwrite = self.state.overwrite;
        self.log.push(format!(
            "patching {} -> {}",
            input.display(),
            output.display()
        ));
        self.worker = Some(spawn_patch(input, output, plan, overwrite));
    }

    fn refresh_summary(&mut self) {
        self.summary = match &self.report {
            Some(Ok(report)) => {
                let ratio = RatioVector::derive(&report.authored);
                let (_, summary) = transform(&report.authored, &self.state.plan, &ratio);
                Some(summary)
            }
            _ => None,
        };
    }

    fn sync_serial(&mut self) {
        self.state.plan.serial = self.serial_enabled.then(|| self.serial_text.clone());
    }

    fn drain_workers(&mut self) {
        self.drain_patch_worker();
        self.drain_analyze_worker();
    }

    fn drain_patch_worker(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        let mut finished = false;
        loop {
            match worker.rx.try_recv() {
                Ok(WorkerMsg::Progress(progress)) => self.progress = Some(progress),
                Ok(WorkerMsg::Done(outcome)) => {
                    self.log.push(format!(
                        "patched {} ({} bytes, md5 {})",
                        self.state.output.display(),
                        outcome.bytes,
                        outcome.md5
                    ));
                    self.progress = None;
                    finished = true;
                    break;
                }
                Ok(WorkerMsg::Failed(message)) => {
                    self.log.push(format!("patch refused: {message}"));
                    self.progress = None;
                    finished = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.log
                        .push("patch worker stopped without a result".to_string());
                    self.progress = None;
                    finished = true;
                    break;
                }
            }
        }
        if finished {
            let _ = worker.handle.join();
        } else {
            self.worker = Some(worker);
        }
    }

    fn drain_analyze_worker(&mut self) {
        let Some(analyze) = self.analyze.take() else {
            return;
        };
        match analyze.rx.try_recv() {
            Ok(result) => {
                let status = status_for(&result);
                self.log.push(format!("analyze: {}", status.text));
                self.report = Some(result);
                self.refresh_summary();
                let _ = analyze.handle.join();
            }
            Err(TryRecvError::Empty) => {
                self.analyze = Some(analyze);
            }
            Err(TryRecvError::Disconnected) => {
                self.log
                    .push("analyze worker stopped without a result".to_string());
                let _ = analyze.handle.join();
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = dropped.into_iter().find_map(|file| file.path) {
            self.apply_input_path(path);
        }
    }

    fn current_status(&self) -> StatusLine {
        match &self.report {
            Some(result) => status_for(result),
            None if self.state.input.is_none() => StatusLine {
                kind: StatusKind::Unknown,
                text: "choose a disc (Browse, or drag the ISO onto this window)".to_string(),
            },
            None => StatusLine {
                kind: StatusKind::Unknown,
                text: "disc chosen — press Analyze to inspect it".to_string(),
            },
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        ui.heading("Digimon World 4 Very Hard Plus");
        ui.add_space(4.0);

        self.input_row(ui);
        self.status_line(ui);
        ui.add_space(4.0);

        self.preset_row(ui);
        self.advanced_panel(ui);
        ui.add_space(4.0);

        self.output_row(ui);
        self.actions(ui);
        self.progress_row(ui);
        self.log_panel(ui);
        about_panel(ui);
    }

    fn input_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Disc:");
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.input_text)
                    .hint_text("path to the ISO")
                    .desired_width(360.0),
            );
            if input.changed() {
                self.state.input = if self.input_text.trim().is_empty() {
                    None
                } else {
                    Some(PathBuf::from(self.input_text.trim()))
                };
                self.report = None;
                self.summary = None;
            }
            if input.lost_focus()
                && self.state.input.is_some()
                && self.output_text.trim().is_empty()
            {
                if let Some(path) = &self.state.input {
                    self.state.output = default_output_path(path);
                    self.output_text = self.state.output.display().to_string();
                }
            }
            if ui.button("Browse…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PS2 disc image", &["iso"])
                    .pick_file()
                {
                    self.apply_input_path(path);
                }
            }
        });
        ui.label("or drag and drop the ISO onto this window");
    }

    fn status_line(&mut self, ui: &mut egui::Ui) {
        let status = self.current_status();
        let color = match status.kind {
            StatusKind::Ok => egui::Color32::from_rgb(0, 160, 60),
            StatusKind::Refused => egui::Color32::from_rgb(200, 60, 60),
            StatusKind::Unknown => egui::Color32::GRAY,
        };
        ui.colored_label(color, status.text);
    }

    fn preset_row(&mut self, ui: &mut egui::Ui) {
        let mut preset = self.state.preset;
        egui::ComboBox::from_label("Preset")
            .selected_text(preset.label())
            .show_ui(ui, |ui| {
                for candidate in [
                    Preset::VeryHardPlus,
                    Preset::Extreme,
                    Preset::Brutal,
                    Preset::Custom,
                ] {
                    ui.selectable_value(&mut preset, candidate, candidate.label());
                }
            });
        if preset != self.state.preset {
            self.state.preset = preset;
            if preset != Preset::Custom {
                self.state.plan = preset.plan();
                self.serial_enabled = false;
                self.serial_text = DEFAULT_SERIAL.to_string();
                self.refresh_summary();
            }
        }
    }

    fn advanced_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Advanced")
            .default_open(false)
            .show(ui, |ui| {
                let mut plan_edited = false;
                egui::Grid::new("advanced_multipliers")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        plan_edited |= multiplier_row(ui, "hp", &mut self.state.plan.hp_mult);
                        for (name, slot) in
                            STAT_NAMES.iter().zip(self.state.plan.stat_mult.iter_mut())
                        {
                            plan_edited |= multiplier_row(ui, name, slot);
                        }
                        plan_edited |= multiplier_row(ui, "crit", &mut self.state.plan.crit_mult);
                        plan_edited |=
                            multiplier_row(ui, "paralysis", &mut self.state.plan.para_mult);
                    });

                ui.horizontal(|ui| {
                    ui.label("crown rank");
                    plan_edited |= ui
                        .add(egui::DragValue::new(&mut self.state.plan.crown_rank).range(0..=5))
                        .changed();
                });

                plan_edited |= ui
                    .checkbox(
                        &mut self.state.plan.exclude_destructibles,
                        "exclude destructibles",
                    )
                    .changed();
                plan_edited |= ui
                    .checkbox(&mut self.state.plan.exclude_tripwire, "exclude tripwire")
                    .changed();
                plan_edited |= ui
                    .checkbox(&mut self.state.plan.practice_buff, "practice buff")
                    .changed();
                plan_edited |= ui
                    .checkbox(&mut self.state.plan.force_very_hard, "force Very Hard")
                    .changed();

                plan_edited |= ui
                    .checkbox(&mut self.serial_enabled, "rename disc serial")
                    .changed();
                if self.serial_enabled {
                    plan_edited |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.serial_text)
                                .hint_text(DEFAULT_SERIAL),
                        )
                        .changed();
                }

                if plan_edited {
                    self.sync_serial();
                    self.state.on_plan_edited();
                    self.refresh_summary();
                }
            });
    }

    fn output_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Output:");
            let output = ui.add(
                egui::TextEdit::singleline(&mut self.output_text)
                    .hint_text("where to write the patched ISO")
                    .desired_width(360.0),
            );
            if output.changed() {
                self.state.output = PathBuf::from(self.output_text.trim());
            }
            ui.checkbox(&mut self.state.overwrite, "overwrite existing file");
        });
        if let Some(Ok(report)) = &self.report {
            ui.label(format!(
                "free-space note: the output copy needs {} bytes free in the destination folder",
                report.size
            ));
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let have_input = self.state.input.is_some();
            let busy = self.worker.is_some() || self.analyze.is_some();
            let have_report = matches!(self.report.as_ref(), Some(Ok(_)));

            if ui
                .add_enabled(have_input && !busy, egui::Button::new("Analyze"))
                .clicked()
            {
                self.start_analyze();
            }
            if ui
                .add_enabled(have_report && !busy, egui::Button::new("Patch ISO"))
                .clicked()
            {
                self.start_patch();
            }
        });
        if let Some(summary) = &self.summary {
            if let Some(note) = attack_pin_note(summary) {
                ui.colored_label(egui::Color32::from_rgb(200, 140, 0), note);
            }
            if let Some(note) = collapse_note(summary) {
                ui.colored_label(egui::Color32::from_rgb(200, 140, 0), note);
            }
        }
    }

    fn progress_row(&mut self, ui: &mut egui::Ui) {
        if self.analyze.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("analyzing…");
            });
            return;
        }
        if self.worker.is_some() && self.progress.is_none() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("patching…");
            });
            return;
        }
        let Some(progress) = self.progress else {
            return;
        };
        let fraction = if progress.total == 0 {
            0.0
        } else {
            (progress.done as f32 / progress.total as f32).clamp(0.0, 1.0)
        };
        let unit = match progress.phase {
            Phase::Writing => "regions",
            _ => "bytes",
        };
        ui.add(
            egui::ProgressBar::new(fraction)
                .desired_width(420.0)
                .text(format!(
                    "{} {}/{} {unit}",
                    phase_label(progress.phase),
                    progress.done,
                    progress.total
                )),
        );
    }

    fn log_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label("Log");
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(140.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.log {
                    ui.monospace(line);
                }
            });
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ctx);
        self.drain_workers();

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw(ui);
        });

        if self.worker.is_some() || self.analyze.is_some() {
            ctx.request_repaint();
        }
    }
}

/// The [`inspect`] call, on its own thread so the window never reads a
/// 1.4 GB disc on the UI thread.
fn spawn_inspect(input: PathBuf) -> AnalyzeHandle {
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let result = inspect(&input, &Layout::retail());
        let _ = tx.send(result);
    });
    AnalyzeHandle { rx, handle }
}

/// One labelled multiplier row inside the Advanced grid.
fn multiplier_row(ui: &mut egui::Ui, name: &str, value: &mut f64) -> bool {
    ui.label(name);
    let changed = ui.add(egui::DragValue::new(value).speed(0.01)).changed();
    ui.end_row();
    changed
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Inspecting => "inspecting",
        Phase::Copying => "copying",
        Phase::Writing => "writing",
        Phase::Verifying => "verifying",
    }
}

fn about_panel(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    egui::CollapsingHeader::new("About")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(CAVEAT_FORCE_VERY_HARD);
            ui.label(CAVEAT_VERY_HARD_TIER2);
            ui.label(CAVEAT_MOD_OPEN_ITEMS);
            ui.label(CAVEAT_BRUTAL);
        });
}
