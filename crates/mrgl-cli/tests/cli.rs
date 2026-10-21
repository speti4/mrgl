//! End-to-end tests of the `mrgl` exe: the voice's two-line rule, `--serious`, `MRGL_SERIOUS`, `--json`, the
//! exit codes, and `scan`, `list` and `root` over the synthetic flavors tree of `mrgl-core`. The tests assert
//! the plain line (docs/nerglish.md, rule 3); the murloc line is checked only for being there. `MRGL_HOME`
//! points every run at a temporary folder, so no test reads or writes the real config. The running-client
//! tests (task 023) copy this test binary into a temporary flavor folder and start it as a stand-in
//! client, never a game file.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

/// The synthetic game folder of `mrgl-core`.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/flavors")
}

/// A fresh, nonexistent home folder for one test.
fn temp_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mrgl-cli-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn mrgl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mrgl"));
    command.env_remove("MRGL_SERIOUS");
    command.env_remove("MRGL_ROOT");
    command.env("MRGL_HOME", temp_home("nohome"));
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

fn stdout_last_line(output: &Output) -> String {
    stdout(output)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn version_is_a_murloc_line_and_a_plain_line() {
    let output = mrgl().arg("--version").output().expect("run mrgl");
    assert!(output.status.success());
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "two lines, got: {stdout:?}");
    assert_eq!(lines[1], format!("mrgl {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn serious_flag_drops_the_murloc_line() {
    let output = mrgl()
        .args(["--serious", "--version"])
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout(&output).trim_end(),
        format!("mrgl {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn serious_environment_drops_the_murloc_line() {
    let output = mrgl()
        .env("MRGL_SERIOUS", "1")
        .arg("--version")
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout(&output).trim_end(),
        format!("mrgl {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_is_plain_with_and_without_the_flag() {
    for args in [vec!["--help"], vec!["--serious", "--help"]] {
        let output = mrgl().args(&args).output().expect("run mrgl");
        assert!(output.status.success());
        let stdout = stdout(&output);
        // Every murloc line starts with a capital: "Mrgl", "Rwl" or "Aaaa"; the help has none of them.
        assert!(
            !stdout.contains("Mrgl") && !stdout.contains("Rwl") && !stdout.contains("Aaaa"),
            "a murloc line in --help ({args:?}): {stdout}"
        );
        assert!(stdout.contains("Usage:"));
    }
}

#[test]
fn json_is_plain_json() {
    let output = mrgl()
        .args(["--json", "--version"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["name"], "mrgl");
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn a_planned_command_fails_with_its_task_and_exit_code_one() {
    let output = mrgl()
        .args(["add", "github:someone/something"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "add is not implemented yet (task 003)"
    );
}

#[test]
fn a_usage_error_is_exit_code_two() {
    let output = mrgl().arg("--no-such-flag").output().expect("run mrgl");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn scan_prints_one_line_per_addon_then_the_summary() {
    let output = mrgl()
        .arg("scan")
        .arg(fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        14 + 2,
        "14 addon lines and the two-line summary: {stdout}"
    );
    assert_eq!(
        lines[15],
        "14 addons in _retail_ (wow 12.1.5.70448, managed by the CurseForge app), 1 folder without a TOC skipped"
    );
    let hinted = lines
        .iter()
        .find(|line| line.starts_with("Hinted"))
        .expect("the Hinted line");
    assert!(
        hinted.contains("2.0.0") && hinted.contains("curse,wago,wowi"),
        "{hinted}"
    );
    let camelot = lines
        .iter()
        .find(|line| line.starts_with("Camelot"))
        .expect("the Camelot line");
    assert!(camelot.contains("no toc for this flavor"), "{camelot}");
    let old = lines
        .iter()
        .find(|line| line.starts_with("OldSuffixes"))
        .expect("the OldSuffixes line");
    assert!(old.contains("legacy"), "{old}");
}

#[test]
fn scan_json_is_one_document_with_the_fixed_kind() {
    let output = mrgl()
        .args(["--json", "scan"])
        .arg(fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "scan");
    assert_eq!(value["flavor"]["id"], "wow");
    assert_eq!(value["flavor"]["version"], "12.1.5.70448");
    assert_eq!(value["flavor"]["interface"], 120105);
    assert_eq!(value["addons"].as_array().map(Vec::len), Some(14));
    assert_eq!(value["skipped"], serde_json::json!(["NoToc"]));
    assert_eq!(value["warnings"], serde_json::json!([]));
    assert!(value["client_running"].is_null(), "{value}");
    let hinted = value["addons"]
        .as_array()
        .expect("addons")
        .iter()
        .find(|addon| addon["folder"] == "Hinted")
        .expect("Hinted");
    assert_eq!(hinted["chosen"], "Hinted.toc");
    assert_eq!(hinted["tocs"][0]["toc"]["curse_project_id"], "123456");
    assert_eq!(value["flavor"]["curse_client"], true);
}

#[test]
fn scan_serious_prints_only_plain_lines() {
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    let stdout = stdout(&output);
    assert_eq!(stdout.lines().count(), 14 + 1, "{stdout}");
    assert!(stdout_last_line(&output).starts_with("14 addons in _retail_"));
}

#[test]
fn list_resolves_a_friendly_name_against_the_root_flag() {
    let output = mrgl()
        .args(["list", "--flavor", "forever", "--root"])
        .arg(fixtures())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _forever_ (wowf 1.60.1.70245)"
    );
}

#[test]
fn list_resolves_an_id_against_mrgl_root() {
    let output = mrgl()
        .env("MRGL_ROOT", fixtures())
        .args(["list", "--flavor", "wow_classic_era"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "1 addon in _classic_era_ (wow_classic_era 1.15.9.70000)"
    );
}

#[test]
fn list_takes_a_flavor_folder_path() {
    let output = mrgl()
        .args(["list", "--flavor"])
        .arg(fixtures().join("_classic_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "1 addon in _classic_ (wow_classic 5.5.4.70531)"
    );
    let line = stdout(&output);
    assert!(line.starts_with("ClassicPick  mists  50504"), "{line}");
}

#[test]
fn the_root_flag_wins_over_the_environment_and_the_config() {
    let home = temp_home("flag-wins");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::write(
        home.join("config.json"),
        "{\"root\":\"C:\\\\nowhere\\\\config\"}",
    )
    .expect("config");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_ROOT", "C:\\nowhere\\env")
        .args(["list", "--flavor", "era", "--root"])
        .arg(fixtures())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout_last_line(&output).starts_with("1 addon in _classic_era_"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_environment_beats_the_config() {
    let home = temp_home("env-beats");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::write(
        home.join("config.json"),
        "{\"root\":\"C:\\\\nowhere\\\\config\"}",
    )
    .expect("config");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_ROOT", fixtures())
        .args(["list", "--flavor", "retail"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout_last_line(&output).starts_with("14 addons in _retail_"));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_remembered_root_is_the_last_resort() {
    let home = temp_home("remembered");
    let saved = mrgl()
        .env("MRGL_HOME", &home)
        .arg("root")
        .arg(fixtures())
        .output()
        .expect("run mrgl");
    assert!(saved.status.success(), "{}", stderr_last_line(&saved));
    let line = stdout_last_line(&saved);
    assert!(line.starts_with("game root saved: "), "{line}");
    assert!(
        line.ends_with("(flavors: wow_classic, wow_classic_era, wowf, wow, wow_mrgl)"),
        "{line}"
    );
    let config = std::fs::read_to_string(home.join("config.json")).expect("config.json written");
    assert!(config.contains("\"root\""), "{config}");

    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["list", "--flavor", "era"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout_last_line(&output).starts_with("1 addon in _classic_era_"));

    let shown = mrgl()
        .env("MRGL_HOME", &home)
        .arg("root")
        .output()
        .expect("run mrgl");
    assert!(stdout_last_line(&shown).starts_with("game root: "));

    let json = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--json", "root"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "root");
    assert_eq!(value["saved"], false);
    assert!(value["root"].is_string());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn no_root_at_all_names_the_three_ways() {
    let output = mrgl()
        .args(["list", "--flavor", "era"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(
        plain.starts_with("no game root for flavor era: "),
        "{plain}"
    );
    assert!(
        plain.contains("--root") && plain.contains("MRGL_ROOT") && plain.contains("mrgl root"),
        "{plain}"
    );
}

#[test]
fn no_flavor_and_an_unknown_flavor_are_errors() {
    let output = mrgl().arg("list").output().expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_last_line(&output).starts_with("no flavor given: "));

    let output = mrgl()
        .args(["list", "--flavor", "plunderstorm"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_last_line(&output).starts_with("unknown flavor: plunderstorm "));
}

#[test]
fn root_rejects_a_folder_without_flavors() {
    let home = temp_home("rejects");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .arg("root")
        .arg(fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(plain.starts_with("not a game folder: "), "{plain}");
    assert!(
        plain.ends_with("(no flavor folder with a .flavor.info in it)"),
        "{plain}"
    );
    assert!(!home.join("config.json").exists());
}

#[test]
fn root_without_a_folder_prints_nothing_when_none_is_remembered() {
    let output = mrgl().arg("root").output().expect("run mrgl");
    assert!(output.status.success());
    assert_eq!(
        stdout_last_line(&output),
        "no game root remembered: run `mrgl root <folder>`"
    );
}

#[test]
fn a_corrupt_config_is_a_warning() {
    let home = temp_home("corrupt");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::write(home.join("config.json"), "{ not json").expect("config");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "root"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(stdout.contains("unreadable config: "), "{stdout}");
    assert!(
        stdout.ends_with("no game root remembered: run `mrgl root <folder>`\n"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_unknown_flavor_id_is_a_warning_and_exit_code_zero() {
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(fixtures().join("_unknown_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(
        stdout.contains("unknown flavor id wow_mrgl in "),
        "{stdout}"
    );
    assert!(
        stdout.contains("build unknown: no row for wow_mrgl in .build.info"),
        "{stdout}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "1 addon in _unknown_ (wow_mrgl, build unknown)"
    );
}

/// A synthetic game folder under the temp dir with a `_forever_` flavor and no addons.
fn temp_game_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mrgl-cli-client-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("_forever_")).expect("the flavor folder");
    std::fs::write(
        root.join(".build.info"),
        "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\n",
    )
    .expect(".build.info");
    std::fs::write(
        root.join("_forever_").join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    )
    .expect(".flavor.info");
    root
}

/// A copy of this test binary in the flavor folder, started as the stand-in client: it runs the
/// `stand_in_client` test below, which waits for its stdin to close.
fn spawn_stand_in(flavor_folder: &Path) -> Child {
    let exe = std::env::current_exe().expect("this test binary");
    let name = format!(
        "client-stand-in{}",
        exe.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default()
    );
    let copy = flavor_folder.join(name);
    std::fs::copy(&exe, &copy).expect("the copy");
    Command::new(&copy)
        .args(["--exact", "stand_in_client", "--include-ignored"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the stand-in starts")
}

/// The stand-in client's body: it blocks until the parent closes its stdin or kills it. Ignored, so a
/// plain `cargo test` never runs it; the copy runs it by name.
#[test]
#[ignore]
fn stand_in_client() {
    let mut sink = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut sink);
}

#[cfg(windows)]
#[test]
fn scan_and_list_name_the_running_client_in_the_summary_and_the_json() {
    let root = temp_game_root("scan");
    let folder = root.join("_forever_");
    let mut child = spawn_stand_in(&folder);

    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(&folder)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _forever_ (wowf 1.60.1.70245, client running)"
    );

    let output = mrgl()
        .args(["--serious", "list", "--flavor", "forever", "--root"])
        .arg(&root)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _forever_ (wowf 1.60.1.70245, client running)"
    );

    let json = mrgl()
        .args(["--json", "list", "--flavor", "forever", "--root"])
        .arg(&root)
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "list");
    assert_eq!(value["client_running"]["pid"], child.id());
    assert!(
        value["client_running"]["exe"]
            .as_str()
            .is_some_and(|exe| exe.ends_with("client-stand-in.exe")),
        "{value}"
    );
    assert_eq!(value["warnings"], serde_json::json!([]));

    child.kill().expect("killed");
    child.wait().expect("waited");
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(&folder)
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _forever_ (wowf 1.60.1.70245)"
    );
    let json = mrgl()
        .args(["--json", "scan"])
        .arg(&folder)
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert!(value["client_running"].is_null(), "{value}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_folder_without_flavor_info_fails_with_exit_code_one() {
    let output = mrgl()
        .arg("scan")
        .arg(fixtures().join("_no_info_"))
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(plain.starts_with("not a game flavor folder: "), "{plain}");
    assert!(plain.ends_with("_no_info_ (no .flavor.info)"), "{plain}");

    let json = mrgl()
        .args(["--json", "scan"])
        .arg(fixtures().join("_no_info_"))
        .output()
        .expect("run mrgl");
    assert_eq!(json.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&json.stderr).expect("a JSON error");
    assert_eq!(value["kind"], "error");
}

// --- task 004: `add <file.zip>` and `remove` over the fixture zips, in a temporary game folder ---

/// The fixture zips of `mrgl-core`.
fn zips() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/zips")
}

/// A temporary game folder with one `_forever_` flavor; `MRGL_ROOT` points the runs at it.
struct Game {
    root: PathBuf,
    home: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let base =
            std::env::temp_dir().join(format!("mrgl-cli-install-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("game");
        std::fs::create_dir_all(root.join("_forever_")).expect("the flavor folder");
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
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.root.parent().expect("the base folder"));
    }
}

#[test]
fn add_installs_a_zip_in_hand_updates_it_and_remove_takes_it_out() {
    let game = Game::new("round");
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("one-folder-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "installing one-folder-1.0.0.zip into _forever_");
    assert_eq!(
        lines.last().copied(),
        Some("installed OneFolder 1.0.0 (1 folder: OneFolder) into _forever_")
    );
    assert!(game.addon("OneFolder").join("OneFolder.toc").is_file());
    assert!(game.state_path().is_file());

    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("one-folder-1.1.0.zip"))
        .args(["--flavor", "wowf"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "updated OneFolder from 1.0.0 to 1.1.0 (1 folder: OneFolder) in _forever_"
    );
    assert!(game.addon("OneFolder").join("new.lua").is_file());
    assert!(!game.addon("OneFolder").join("old.lua").exists());
    assert_eq!(
        stdout(&output).lines().count(),
        4,
        "two two-line messages: {}",
        stdout(&output)
    );

    let output = game
        .mrgl()
        .args(["remove", "onefolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "removed OneFolder 1.1.0 (1 folder: OneFolder) from _forever_"
    );
    assert!(!game.addon("OneFolder").exists());
    let state = std::fs::read_to_string(game.state_path()).expect("the sidecar");
    assert!(!state.contains("OneFolder"), "{state}");
}

#[test]
fn add_and_remove_json_are_one_document_each() {
    let game = Game::new("json");
    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("several-folders-2.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "add");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["flavor"]["id"], "wowf");
    assert_eq!(value["addon"]["name"], "SeveralFolders");
    assert_eq!(value["addon"]["version"], "2.0.0");
    assert_eq!(value["addon"]["previous_version"], serde_json::Value::Null);
    assert_eq!(
        value["addon"]["folders"],
        serde_json::json!([
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ])
    );
    assert!(
        value["addon"]["source"]
            .as_str()
            .is_some_and(|source| source.ends_with("several-folders-2.0.0.zip"))
    );
    assert_eq!(value["warnings"], serde_json::json!([]));

    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("several-folders-2.1.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["addon"]["previous_version"], "2.0.0");
    assert_eq!(
        value["addon"]["removed_folders"],
        serde_json::json!(["SeveralFolders_Data"])
    );

    let output = game
        .mrgl()
        .args(["--json", "remove", "SeveralFolders", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "remove");
    assert_eq!(value["addon"]["name"], "SeveralFolders");
    assert_eq!(value["addon"]["version"], "2.1.0");
    assert_eq!(
        value["addon"]["folders"],
        serde_json::json!(["SeveralFolders", "SeveralFolders_Options"])
    );
}

#[test]
fn add_dry_run_says_what_it_would_do_and_writes_nothing() {
    let game = Game::new("dry-run");
    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("several-folders-2.0.0.zip"))
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "dry run: would install SeveralFolders 2.0.0 (3 folders: SeveralFolders, SeveralFolders_Data, SeveralFolders_Options) in _forever_"
    );
    assert!(!game.addon("SeveralFolders").exists());
    assert!(!game.state_path().exists());
    assert!(
        std::fs::read_dir(game.home.join("wowf").join("staging"))
            .map(|entries| entries.count() == 0)
            .unwrap_or(true),
        "the staging folder is cleaned"
    );

    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("several-folders-2.0.0.zip"))
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "add");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["addon"]["name"], "SeveralFolders");
}

#[test]
fn add_refuses_an_escaping_zip_with_exit_code_one() {
    let game = Game::new("escaping");
    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("escaping-path.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(plain.starts_with("refused "), "{plain}");
    assert!(
        plain.ends_with(
            "the entry `Escape/../../Outside.lua` would land outside the staging folder"
        ),
        "{plain}"
    );
    assert!(!game.addon("Escape").exists());
}

#[test]
fn remove_of_an_unknown_or_unmanaged_addon_is_exit_code_one() {
    let game = Game::new("unknown");
    let output = game
        .mrgl()
        .args(["remove", "Nope", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "addon not found: Nope (flavor forever)"
    );

    std::fs::create_dir_all(game.addon("ByHand")).expect("a folder by hand");
    let output = game
        .mrgl()
        .args(["remove", "ByHand", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_last_line(&output)
            .starts_with("addon not managed by mrgl: ByHand (flavor forever); "),
        "{}",
        stderr_last_line(&output)
    );
    assert!(game.addon("ByHand").is_dir());
}

#[test]
fn add_takes_a_flavor_folder_path_and_needs_a_flavor() {
    let game = Game::new("folder-path");
    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("root-toc-3.0.0.zip"))
        .arg("--flavor")
        .arg(game.root.join("_forever_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "installed RootToc 3.0.0 (1 folder: RootToc) into _forever_"
    );

    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("root-toc-3.0.0.zip"))
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_last_line(&output).starts_with("no flavor given: "));
}

#[test]
fn a_source_spec_and_update_still_name_their_tasks() {
    let output = mrgl()
        .args(["add", "wago:something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "add is not implemented yet (task 003)"
    );
    let output = mrgl()
        .args(["update", "Something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "update is not implemented yet (task 006)"
    );
}
