fn main() {
    // tauri-build embeds icons/icon.ico in the exe and the code generation reads the icons, but it only asks to
    // rerun on tauri.conf.json and the capabilities: a regenerated icon (scripts/make-icon.py) would not reach
    // the exe until a clean build.
    println!("cargo::rerun-if-changed=icons/icon.ico");
    println!("cargo::rerun-if-changed=icons/icon.png");
    tauri_build::build();
    // The test binaries link the resource library tauri-build links into the exe (tauri-winres writes it as
    // `resource.lib` under OUT_DIR): its application manifest asks for Common Controls v6, which the menu
    // crate's `TaskDialogIndirect` import needs, so a test binary that builds a Tauri app on the mock runtime
    // (`tests/commands.rs`, task 117) starts at all; without the manifest the loader refuses it before `main`.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set for a build script");
        println!("cargo::rustc-link-arg-tests={out_dir}/resource.lib");
    }
}
