//! mrgl-app: the Tauri 2 shell over `mrgl-core` (M2 Window MVP starts here, task 009).
//!
//! The webview loads the static files under `ui/` (embedded at compile time) and talks to Rust through the
//! commands in [`commands`]. Everything that does not need a webview lives in plain modules with unit
//! tests; the window only shows what the core says, in the voice's two lines (D-005).

pub mod commands;

/// Builds the window and runs the event loop until it closes.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::banner
        ])
        .run(tauri::generate_context!())
        .expect("error while running the mrgl window");
}
