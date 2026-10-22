fn main() {
    // tauri-build embeds icons/icon.ico in the exe and the code generation reads the icons, but it only asks to
    // rerun on tauri.conf.json and the capabilities: a regenerated icon (scripts/make-icon.py) would not reach
    // the exe until a clean build.
    println!("cargo::rerun-if-changed=icons/icon.ico");
    println!("cargo::rerun-if-changed=icons/icon.png");
    tauri_build::build()
}
