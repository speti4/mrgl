//! mrgl-app: the Tauri 2 shell over `mrgl-core` (M2 Window MVP, tasks 009 and 010).
//!
//! The webview loads the static files under `ui/` (embedded at compile time) and talks to Rust through the
//! commands in [`commands`]; `ui/data.js` calls them behind `window.Data`. Everything that does not need a
//! webview lives in plain modules with tests ([`list`]: the flavors and their addon rows; [`actions`]: the
//! update check, the updates and the marks; [`toolbox`]: a flavor's addon sets with their undo and edit and
//! its SavedVariables, tasks 074 and 085, and the adopt report with its apply, task 109); the window only
//! shows what the core says, in the voice's two lines (D-005).

// The `coverage(off)` attribute of the documented exclusions (docs/development.md, "Coverage"); the feature
// gate exists only under cargo-llvm-cov's nightly cfg, so the stable build never sees it.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod actions;
pub mod commands;
pub mod list;
pub mod toolbox;

use tauri::Manager;

/// Builds the window and runs the event loop until it closes.
// coverage: off — the Tauri event loop needs a WebView2 window and a desktop session
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(actions::UpdateRun::default())
        .manage(toolbox::ToolboxRun::default())
        .setup(|app| {
            // Task 079: the title bar follows the theme preference from the start, not the system's.
            if let Some(window) = app.get_webview_window("main") {
                commands::apply_native_theme(&window, &commands::preferences().theme);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::flavors,
            commands::choose_root,
            commands::diagnostics,
            commands::resolve,
            commands::check_updates,
            commands::update_all,
            commands::update_one,
            commands::cancel_update,
            commands::mark,
            commands::preferences,
            commands::set_preferences,
            commands::toolbox,
            commands::apply_set,
            commands::undo_set,
            commands::edit_set,
            commands::clean_saved_variables,
            commands::adopt,
            commands::adopt_apply
        ])
        .run(tauri::generate_context!())
        .expect("error while running the mrgl window");
}
