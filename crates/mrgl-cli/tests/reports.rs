//! End-to-end tests of what the `mrgl` exe reports: the staging folder the doctor names (task 088), an
//! update a second entry of the same source has installed already (task 089) and the dependency check
//! of `import` (task 090). Every run points `MRGL_HOME` and `MRGL_ROOT` at a temporary folder, answers the
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

// --- task 089: `update` reports a second entry of the same source that has the pick's version ---

impl Game {
    fn addon(&self, folder: &str) -> PathBuf {
        self.root
            .join("_forever_")
            .join("Interface")
            .join("AddOns")
            .join(folder)
    }

    fn state_path(&self) -> PathBuf {
        self.home.join("wowf").join("state.json")
    }

    /// An addon folder with a TOC, made by hand.
    fn by_hand(&self, folder: &str, version: &str) {
        std::fs::create_dir_all(self.addon(folder)).expect("the folder");
        std::fs::write(
            self.addon(folder).join(format!("{folder}.toc")),
            format!("## Interface: 16001\n## Title: {folder}\n## Version: {version}\n"),
        )
        .expect("the TOC");
    }

    /// A sidecar that manages the entries (folder, source, version), each with its folder made by hand.
    fn managed(&self, entries: &[(&str, &str, &str)]) {
        let mut json = Vec::new();
        for (folder, source, version) in entries {
            self.by_hand(folder, version);
            json.push(format!(
                r#""{folder}": {{ "source": "{source}", "version": "{version}", "folders": ["{folder}"], "files": {{}}, "installed_at": "2026-10-21T14:03:05Z" }}"#
            ));
        }
        std::fs::create_dir_all(self.home.join("wowf")).expect("the flavor home");
        std::fs::write(
            self.state_path(),
            format!(
                r#"{{ "schema": 1, "flavor": "wowf", "addons": {{ {} }} }}"#,
                json.join(", ")
            ),
        )
        .expect("the sidecar");
    }

    fn sidecar(&self) -> serde_json::Value {
        let text = std::fs::read_to_string(self.state_path()).expect("the sidecar");
        serde_json::from_str(&text).expect("the sidecar is JSON")
    }
}

/// A game whose sidecar has two entries of `github:Questie/Questie`: `Questie` at the current release
/// 1.2.3 of the fixture answers, and `QuestieClassic`, an older name of it, at 1.2.2. The one install path
/// looks an addon up by its source and finds `Questie` first.
fn two_entries_of_one_source(name: &str) -> Game {
    let game = Game::new(name);
    game.managed(&[
        ("Questie", "github:Questie/Questie", "1.2.3"),
        ("QuestieClassic", "github:Questie/Questie", "1.2.2"),
    ]);
    game
}

const SECOND_ENTRY_LINE: &str = "QuestieClassic not updated: Questie, a second entry of github:Questie/Questie, has 1.2.3 installed already in _forever_; QuestieClassic stays at 1.2.2";

#[test]
fn update_reports_a_second_entry_of_the_same_source_and_counts_it_apart() {
    for (args, checked, summary) in [
        (
            ["update", "QuestieClassic"],
            vec!["checking QuestieClassic of _forever_ against their sources"],
            "nothing updated: 1 installed already",
        ),
        (
            ["update", "--all"],
            vec![
                "checking the managed addons of _forever_ against their sources",
                "checking Questie against github:Questie/Questie (1 of 2)",
                "Questie  1.2.3  up to date",
            ],
            "nothing updated: 1 installed already, 1 up to date",
        ),
    ] {
        let game = two_entries_of_one_source(&format!("update-second-{}", args[1]));
        let before = game.sidecar();
        let output = game
            .mrgl()
            .arg("--serious")
            .args(args)
            .args(["--yes", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let text = stdout(&output);
        let mut expected = checked;
        let at = if args[1] == "--all" { 2 } else { 1 };
        let checking =
            format!("checking QuestieClassic against github:Questie/Questie ({at} of {at})");
        expected.extend([
            checking.as_str(),
            "QuestieClassic  1.2.2 -> 1.2.3  update available",
            "updating QuestieClassic from 1.2.2 to 1.2.3",
            SECOND_ENTRY_LINE,
            summary,
        ]);
        assert_eq!(text.lines().collect::<Vec<_>>(), expected, "{text}");
        assert_eq!(game.sidecar(), before, "nothing written");

        let output = game
            .mrgl()
            .arg("--json")
            .args(args)
            .args(["--yes", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("one document");
        assert_eq!(value["updated"], 0, "{value}");
        assert_eq!(value["installed_already"], 1, "{value}");
        let addon = value["addons"]
            .as_array()
            .expect("the addons")
            .iter()
            .find(|addon| addon["name"] == "QuestieClassic")
            .expect("QuestieClassic");
        assert_eq!(addon["updated"], false, "{addon}");
        assert_eq!(addon["addon"], serde_json::Value::Null, "{addon}");
        assert_eq!(
            addon["installed_already"],
            serde_json::json!({
                "name": "Questie",
                "version": "1.2.3",
                "outcome": null,
                "previous_version": null,
                "source": "github:Questie/Questie",
                "folders": ["Questie"],
                "removed_folders": [],
                "same_entry": false,
                "message": SECOND_ENTRY_LINE,
            }),
            "{addon}"
        );
        assert_eq!(addon["error"], serde_json::Value::Null, "{addon}");
        assert_eq!(game.sidecar(), before, "nothing written");
    }
}

// --- task 090: `import` runs the dependency check after an install ---

/// The fixture zips of `mrgl-core`.
fn zips() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/zips")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the folder");
    for entry in std::fs::read_dir(from).expect("read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// A copy of the fixture answers with a zip behind the Forever asset of `Questie/Questie` v1.2.3, so an
/// import downloads and installs it offline.
fn stub_with_forever_zip(game: &Game, zip: &str) -> PathBuf {
    let stub = game.root.parent().expect("the base").join("stub");
    let _ = std::fs::remove_dir_all(&stub);
    copy_tree(&releases(), &stub);
    std::fs::copy(
        zips().join(zip),
        stub.join("github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip"),
    )
    .expect("the asset");
    stub
}

/// A list file of one entry, `NeedsLibrary` from `github:Questie/Questie` at 1.2.3.
fn needs_library_list(game: &Game) -> PathBuf {
    let path = game.root.parent().expect("the base").join("list.json");
    std::fs::write(
        &path,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [{
                "name": "NeedsLibrary",
                "source": "github:Questie/Questie",
                "version": "1.2.3",
                "folders": ["NeedsLibrary"],
            }],
        })
        .to_string(),
    )
    .expect("the list");
    path
}

const MISSING_LINE: &str = "NeedsLibrary needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside NeedsLibrary, and the client does not count a library inside an addon's own folder)";

#[test]
fn import_warns_about_a_missing_dependency_of_an_installed_addon_and_not_of_a_complete_one() {
    for complete in [false, true] {
        for json in [false, true] {
            let game = Game::new(&format!("import-dependencies-{complete}-{json}"));
            if complete {
                // The libraries are addons of their own already: the check finds them.
                game.by_hand("LibEmbedded", "1.0");
                game.by_hand("LibMissing", "1.0");
            }
            let stub = stub_with_forever_zip(&game, "needs-library-1.0.0.zip");
            let list = needs_library_list(&game);
            let output = game
                .mrgl()
                .env("MRGL_HTTP_STUB", &stub)
                .arg(if json { "--json" } else { "--serious" })
                .arg("import")
                .arg(&list)
                .args(["--flavor", "forever"])
                .output()
                .expect("run mrgl");
            assert!(output.status.success(), "{}", stderr_last_line(&output));
            if json {
                let value: serde_json::Value =
                    serde_json::from_slice(&output.stdout).expect("one JSON document");
                let item = &value["addons"][0];
                assert_eq!(item["outcome"], "installed", "{value}");
                let dependencies = &item["dependencies"];
                assert_eq!(dependencies.as_array().map(Vec::len), Some(1), "{value}");
                assert_eq!(dependencies[0]["folder"], "NeedsLibrary");
                let line = serde_json::Value::from(MISSING_LINE);
                if complete {
                    assert_eq!(dependencies[0]["missing"], serde_json::json!([]), "{value}");
                    assert_eq!(
                        dependencies[0]["met"],
                        serde_json::json!(["LibEmbedded", "LibMissing", "Blizzard_Something"]),
                        "{value}"
                    );
                    assert_eq!(item["warnings"], serde_json::json!([]), "{value}");
                    assert_eq!(value["warnings"], serde_json::json!([]), "{value}");
                } else {
                    assert_eq!(
                        dependencies[0]["missing"],
                        serde_json::json!(["LibEmbedded", "LibMissing"]),
                        "{value}"
                    );
                    assert_eq!(item["warnings"], serde_json::json!([line]), "{value}");
                    assert_eq!(value["warnings"], serde_json::json!([line]), "{value}");
                }
            } else {
                let text = stdout(&output);
                let mut expected = vec![
                    "importing 1 addon from list.json into _forever_",
                    "NeedsLibrary  installed 1.2.3 from github:Questie/Questie",
                ];
                if !complete {
                    expected.push(MISSING_LINE);
                }
                expected.push("imported 1 addon of list.json into _forever_: 1 installed");
                assert_eq!(text.lines().collect::<Vec<_>>(), expected, "{text}");
            }
        }
    }
}
