//! mrgl-app: the Tauri 2 shell over `mrgl-core` (M2 Window MVP, task 009).
//!
//! The webview loads the static files under `ui/` (embedded at compile time) and talks to Rust through the
//! commands in [`commands`]; `ui/data.js` calls them behind `window.Data`. Everything that does not need a
//! webview lives in plain modules with tests ([`list`]: the flavors and their addon rows); the window only
//! shows what the core says, in the voice's two lines (D-005).

pub mod commands;
pub mod list;

/// Builds the window and runs the event loop until it closes.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::flavors,
            commands::choose_root,
            commands::diagnostics,
            commands::resolve,
            commands::update_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running the mrgl window");
}
