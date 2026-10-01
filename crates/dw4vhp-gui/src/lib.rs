//! The window's app for the Digimon World 4 Very Hard Plus patcher.
//!
//! [`form`] is the pure half: every decision the window makes, with no `egui`,
//! `eframe` or `rfd` dependency, so `cargo test` covers it headlessly.

pub mod app;
pub mod form;
pub mod worker;
