//! Runs the Node tests of the UI modules (every `tests/ui/*.test.js`, against `ui/*.js`) when `node` is on
//! PATH; without Node the test prints a skip line and passes, so a machine without it still builds and tests
//! the Rust side. In CI (`CI` set) a missing Node fails instead, so the UI tests cannot vanish unnoticed.

use std::path::PathBuf;
use std::process::Command;

fn ui_tests() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ui");
    let mut tests: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".test.js"))
        })
        .collect();
    tests.sort();
    assert!(!tests.is_empty(), "no *.test.js under {}", dir.display());
    tests
}

#[test]
fn the_ui_modules_pass_their_node_tests() {
    let tests = ui_tests();
    let output = match Command::new("node").arg("--test").args(&tests).output() {
        Ok(output) => output,
        Err(err) if std::env::var_os("CI").is_none() => {
            eprintln!("skipped: node is not available ({err})");
            return;
        }
        Err(err) => panic!("node is not available in CI ({err})"),
    };
    assert!(
        output.status.success(),
        "node --test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
