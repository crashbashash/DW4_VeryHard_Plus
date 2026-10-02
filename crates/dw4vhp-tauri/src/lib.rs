mod commands;
mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::choose_iso,
            commands::choose_output,
            commands::analyze,
            commands::plan_summary,
            commands::start_patch,
            commands::default_output,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the DW4 Very Hard Plus window");
}
