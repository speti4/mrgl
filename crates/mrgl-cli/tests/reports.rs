//! End-to-end tests of what the `mrgl` exe reports about a run's surroundings: the staging folder the doctor
//! names (task 088). Every run points `MRGL_HOME` and `MRGL_ROOT` at a temporary folder, answers the
//! sources from the fixture folder stub and reads no registry, as `tests/cli.rs` does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The fixture answers of the GitHub source, served by the folder stub.
fn releases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/releases")
}

fn mrgl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mrgl"));
    command.env_remove("MRGL_SERIOUS");
    command.env_remove("MRGL_ROOT");
    command.env_remove("MRGL_GITHUB_TOKEN");
    // Every source answers from the fixture folder, never from the network.
    command.env("MRGL_HTTP_STUB", releases());
    // No test reads the real registry or looks under Program Files.
    command.env("MRGL_DISCOVERY", "off");
    command
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_last_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

/// A temporary game folder with one `_forever_` flavor; `MRGL_ROOT` points the runs at it.
struct Game {
    root: PathBuf,
    home: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let base =
            std::env::temp_dir().join(format!("mrgl-cli-reports-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("game");
        std::fs::create_dir_all(root.join("_forever_").join("Interface").join("AddOns"))
            .expect("the AddOns folder");
        std::fs::write(
            root.join(".build.info"),
            "Version!STRING:0|Product!STRING:0\n1.60.1.70245|wowf\n",
        )
        .expect(".build.info");
        std::fs::write(
            root.join("_forever_").join(".flavor.info"),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect(".flavor.info");
        Game {
            root,
            home: base.join("home"),
        }
    }

    fn mrgl(&self) -> Command {
        let mut command = mrgl();
        command.env("MRGL_HOME", &self.home);
        command.env("MRGL_ROOT", &self.root);
        command
    }

    fn staging(&self) -> PathBuf {
        self.home.join("wowf").join("staging")
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.root.parent().expect("the base folder"));
    }
}

// --- task 088: the doctor names a staging folder kept for its `previous\` folders ---

/// A folder under `staging\` as a run three hours ago would have named it, with one file in it.
fn planted_run(game: &Game, stem: &str) -> PathBuf {
    let nanos = (std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 60 * 60))
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_nanos();
    let dir = game.staging().join(format!("{stem}-{nanos}-0"));
    std::fs::create_dir_all(&dir).expect("the run folder");
    std::fs::write(dir.join("left.txt"), "left over\n").expect("a file");
    dir
}

#[test]
fn doctor_names_a_staging_folder_kept_for_its_previous_folders_in_text_and_json() {
    let game = Game::new("doctor-kept");
    let kept = planted_run(&game, "download-Kept.zip");
    let previous = kept.join("previous").join("OneFolder");
    std::fs::create_dir_all(&previous).expect("the previous folder");
    std::fs::write(previous.join("OneFolder.toc"), "## Interface: 16001\n").expect("a TOC");
    let plain = planted_run(&game, "download-Plain.zip");
    let name = |dir: &Path| {
        dir.file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned()
    };

    let output = game
        .mrgl()
        .args(["--serious", "doctor", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("  kept staging: "))
        .unwrap_or_else(|| panic!("no kept staging line in:\n{text}"));
    assert!(
        lines[at].ends_with(&format!(
            "{}{}{} (3 hours old, 30 B in 2 files; previous: OneFolder)",
            Path::new("wowf").join("staging").display(),
            std::path::MAIN_SEPARATOR,
            name(&kept)
        )),
        "{text}"
    );
    assert!(
        lines[at + 1].starts_with(
            "  kept staging advice: a failed swap moved the addon folders under previous aside;"
        ),
        "{text}"
    );
    assert!(!text.contains(&name(&plain)), "{text}");
    assert_eq!(
        lines.last().copied(),
        Some(
            "diagnostics of 1 flavor, 1 thing to look at (a kept staging folder of _forever_); paste the lines above into a bug report"
        ),
        "{text}"
    );

    let output = game
        .mrgl()
        .args(["--json", "doctor", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    let kept_json = &value["flavors"][0]["kept_staging"];
    assert_eq!(kept_json.as_array().map(Vec::len), Some(1), "{value}");
    assert!(
        kept_json[0]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with(&name(&kept))),
        "{kept_json}"
    );
    assert_eq!(kept_json[0]["previous"], serde_json::json!(["OneFolder"]));
    assert_eq!(kept_json[0]["bytes"], 30);
    assert!(kept_json[0]["advice"].is_string(), "{kept_json}");

    // Without its previous folder the run folder is the sweep's to remove, and the doctor names nothing.
    std::fs::remove_dir_all(kept.join("previous")).expect("removes");
    let output = game
        .mrgl()
        .args(["--json", "doctor", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["flavors"][0]["kept_staging"], serde_json::json!([]));
    let output = game
        .mrgl()
        .args(["--serious", "doctor", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(
        !stdout(&output).contains("kept staging"),
        "{}",
        stdout(&output)
    );
}
