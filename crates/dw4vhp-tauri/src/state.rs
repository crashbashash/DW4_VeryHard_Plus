use dw4vhp_core::disc::DiscReport;
use std::path::PathBuf;
use std::sync::Mutex;

/// What the shell holds between commands: the analysed disc (from the last
/// successful [`analyze`]), and the input path it was read from. The patch
/// worker reads both; the frontend never sees the enemy table.
#[derive(Default)]
pub struct AppState {
    pub input: Mutex<Option<PathBuf>>,
    pub report: Mutex<Option<DiscReport>>,
}
