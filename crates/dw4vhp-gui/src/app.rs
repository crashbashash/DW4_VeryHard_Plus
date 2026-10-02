//! The `eframe` window: one screen that picks a disc, shows what it detected,
//! lets the player choose a preset or edit the plan, and patches on a
//! background worker so a 1.4 GB copy never blocks the UI thread. Analysing
//! happens automatically when a disc is chosen; the one button is Patch ISO.
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
/// The crown colours the game's `SETRAREICON` draws for `RARITY` 0–5, as
/// verified live in the decomp (0 and 6 both draw nothing).
const CROWN_COLOURS: [(u8, &str); 6] = [
    (0, "none (no crown)"),
    (1, "green"),
    (2, "blue"),
    (3, "pink"),
    (4, "white"),
    (5, "yellow"),
];

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
        // Analysing is automatic: picking a disc is all it takes.
        self.start_analyze();
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
            // A path typed by hand does not re-analyse on every keystroke;
            // once it is committed (focus leaves the field or Enter is
            // pressed), analysis starts on its own.
            if (input.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)))
                && input.changed()
                && self.state.input.is_some()
            {
                self.start_analyze();
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
            self.state
                .apply_preset(preset, &mut self.serial_enabled, &mut self.serial_text);
            if preset != Preset::Custom {
                self.log.push(format!(
                    "preset set to {} — Advanced now shows its settings",
                    preset.label()
                ));
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
                    ui.label("crown colour");
                    let mut rank = self.state.plan.crown_rank;
                    egui::ComboBox::from_id_salt("crown_colour")
                        .selected_text(crown_colour_name(rank))
                        .show_ui(ui, |ui| {
                            for &(value, name) in &CROWN_COLOURS {
                                ui.selectable_value(&mut rank, value, name);
                            }
                        });
                    if rank != self.state.plan.crown_rank {
                        self.state.plan.crown_rank = rank;
                        plan_edited = true;
                    }
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
                    .checkbox(
                        &mut self.state.plan.exclude_practice,
                        "leave training-stage enemies vanilla",
                    )
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
                ui.label(
                    "these settings follow the selected preset until you edit one — then the \
                     preset becomes Custom",
                );
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
            let busy = self.worker.is_some() || self.analyze.is_some();
            let have_report = matches!(self.report.as_ref(), Some(Ok(_)));

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
fn crown_colour_name(rank: u8) -> &'static str {
    CROWN_COLOURS
        .iter()
        .find(|&&(value, _)| value == rank)
        .map(|&(_, name)| name)
        .unwrap_or("unknown")
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
