// A release build is a windowed app; the debug build keeps its console for `eprintln!`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// The `coverage(off)` attribute of the documented exclusions (docs/development.md, "Coverage"); the feature
// gate exists only under cargo-llvm-cov's nightly cfg, so the stable build never sees it.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// coverage: off — the Tauri event loop needs a WebView2 window and a desktop session
#[cfg_attr(coverage_nightly, coverage(off))]
fn main() {
    mrgl_app_lib::run()
}
