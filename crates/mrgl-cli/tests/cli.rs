//! End-to-end tests of the `mrgl` exe: the voice's two-line rule, `--serious`, `MRGL_SERIOUS`, `--json`, the
//! exit codes, and `scan`, `list` and `root` over the synthetic flavors tree of `mrgl-core`. The tests assert
//! the plain line (docs/nerglish.md, rule 3); the murloc line is checked only for being there. `MRGL_HOME`
//! points every run at a temporary folder, so no test reads or writes the real config, and
//! `MRGL_DISCOVERY` is `off` or a stub (task 020), so no test reads the real registry or looks under
//! `Program Files`. The running-client tests (task 023) copy this test binary into a temporary flavor
//! folder and start it as a stand-in client, never a game file. The `backup` tests (task 013) copy the
//! synthetic fixture `WTF` tree into a temporary flavor folder and never touch a real one. The managed and
//! unmanaged `list` tests (task 037) install a fixture zip through the engine into a temporary game folder.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

/// The synthetic game folder of `mrgl-core`.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/flavors")
}

/// A `MRGL_DISCOVERY` stub: the launcher's value and the usual folders, `;`-separated.
fn discovery(registry: Option<&Path>, folders: &[&Path]) -> String {
    let mut items: Vec<String> = registry
        .map(|path| format!("registry={}", path.display()))
        .into_iter()
        .collect();
    items.extend(
        folders
            .iter()
            .map(|path| format!("folder={}", path.display())),
    );
    items.join(";")
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
    command.env_remove("MRGL_GITHUB_TOKEN");
    command.env("MRGL_HOME", temp_home("nohome"));
    // Every source answers from the fixture folder, never from the network.
    command.env("MRGL_HTTP_STUB", releases());
    // No test reads the real registry or looks under Program Files.
    command.env("MRGL_DISCOVERY", "off");
    command
}

/// The fixture answers of the GitHub source, served by the folder stub.
fn releases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/releases")
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
fn the_config_files_serious_preference_is_the_third_source() {
    // Task 034: the window's switch writes `serious` into config.json; the flag and MRGL_SERIOUS come first.
    let home = temp_home("serious-preference");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::write(home.join("config.json"), r#"{ "serious": true }"#).expect("config");
    let plain = format!("mrgl {}", env!("CARGO_PKG_VERSION"));
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .arg("--version")
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout(&output).trim_end(),
        plain,
        "the file makes it serious"
    );
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_SERIOUS", "0")
        .arg("--version")
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout(&output).lines().count(),
        2,
        "MRGL_SERIOUS=0 wins over the file: {}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(&home);
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
        "1 addon in _classic_era_ (wow_classic_era 1.15.9.70000): 0 managed by mrgl, 1 unmanaged"
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
        "1 addon in _classic_ (wow_classic 5.5.4.70531): 0 managed by mrgl, 1 unmanaged"
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
fn the_remembered_root_beats_the_discovery() {
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

    // The discovery would find a game root with `_forever_` only; the remembered one has `era`.
    let other = temp_game_root("remembered-beats");
    let stub = discovery(Some(&other), &[]);
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_DISCOVERY", &stub)
        .args(["--serious", "list", "--flavor", "era"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(stdout_last_line(&output).starts_with("1 addon in _classic_era_"));
    assert!(!stdout.contains("game root found"), "{stdout}");

    let shown = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_DISCOVERY", &stub)
        .arg("root")
        .output()
        .expect("run mrgl");
    let line = stdout_last_line(&shown);
    assert!(line.starts_with("game root: "), "{line}");
    assert!(line.ends_with(" (remembered)"), "{line}");

    let json = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_DISCOVERY", &stub)
        .args(["--json", "root"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "root");
    assert_eq!(value["saved"], false);
    assert_eq!(value["root_origin"], "remembered");
    assert!(value["root"].is_string());
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn list_finds_the_root_in_the_launchers_registry_key() {
    let output = mrgl()
        .env("MRGL_DISCOVERY", discovery(Some(&fixtures()), &[]))
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "the found line and the summary: {stdout}");
    assert!(
        lines[0].starts_with("game root found in the launcher's registry key: "),
        "{stdout}"
    );
    assert!(
        lines[0].ends_with("; run `mrgl root <folder>` to remember it"),
        "{stdout}"
    );
    assert_eq!(lines[1], "no addons in _forever_ (wowf 1.60.1.70245)");

    let json = mrgl()
        .env("MRGL_DISCOVERY", discovery(Some(&fixtures()), &[]))
        .args(["--json", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "list");
    assert_eq!(value["flavor"]["id"], "wowf");
    assert_eq!(value["root_origin"], "registry");
    let warnings = value["warnings"].as_array().expect("warnings");
    assert_eq!(warnings.len(), 1, "{value}");
    assert!(
        warnings[0].as_str().is_some_and(
            |line| line.starts_with("game root found in the launcher's registry key: ")
        ),
        "{value}"
    );
}

#[test]
fn a_registry_value_that_names_a_flavor_folder_gives_its_parent() {
    let output = mrgl()
        .env(
            "MRGL_DISCOVERY",
            discovery(Some(&fixtures().join("_retail_")), &[]),
        )
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("game root found in the launcher's registry key: "),
        "{stdout}"
    );
    assert!(
        stdout.contains("flavors; run `mrgl root <folder>` to remember it"),
        "the parent of _retail_: {stdout}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _forever_ (wowf 1.60.1.70245)"
    );
}

#[test]
fn the_usual_folders_are_tried_in_order_after_the_registry() {
    let nowhere = Path::new("Q:\\nowhere\\registry");
    let elsewhere = Path::new("Q:\\nowhere\\folder");
    let output = mrgl()
        .env(
            "MRGL_DISCOVERY",
            discovery(Some(nowhere), &[elsewhere, &fixtures()]),
        )
        .args(["--serious", "list", "--flavor", "classic"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("game root found in a usual install folder: "),
        "{stdout}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "1 addon in _classic_ (wow_classic 5.5.4.70531): 0 managed by mrgl, 1 unmanaged"
    );
}

#[test]
fn a_hit_that_is_not_a_game_root_is_skipped() {
    // Neither the folder nor its parent is a game root: no .build.info next to a flavor folder.
    let inside = fixtures().join("_retail_").join("Interface");
    let output = mrgl()
        .env("MRGL_DISCOVERY", discovery(Some(&inside), &[&inside]))
        .args(["list", "--flavor", "era"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_last_line(&output).starts_with("no game root for flavor era: "),
        "{}",
        stderr_last_line(&output)
    );
}

#[test]
fn root_reports_a_discovered_root_and_never_saves_it() {
    let home = temp_home("discovered");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_DISCOVERY", discovery(Some(&fixtures()), &[]))
        .arg("root")
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let line = stdout_last_line(&output);
    assert!(
        line.starts_with("game root found in the launcher's registry key: "),
        "{line}"
    );
    assert!(
        line.ends_with("; run `mrgl root <folder>` to remember it"),
        "{line}"
    );
    assert!(!home.join("config.json").exists(), "nothing saved");

    let json = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_DISCOVERY", discovery(None, &[&fixtures()]))
        .args(["--json", "root"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "root");
    assert_eq!(value["saved"], false);
    assert_eq!(value["root_origin"], "common_folder");
    assert!(value["root"].is_string(), "{value}");
    assert_eq!(value["warnings"], serde_json::json!([]));
    assert!(!home.join("config.json").exists(), "nothing saved");
}

#[test]
fn root_names_the_environment_as_the_origin() {
    let output = mrgl()
        .env("MRGL_ROOT", fixtures())
        .args(["--serious", "root"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let line = stdout_last_line(&output);
    assert!(line.starts_with("game root: "), "{line}");
    assert!(line.ends_with(" (MRGL_ROOT)"), "{line}");
    let json = mrgl()
        .env("MRGL_ROOT", fixtures())
        .args(["--json", "root"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["root_origin"], "environment");
}

#[test]
fn no_root_at_all_names_the_three_ways_and_the_discovery() {
    let output = mrgl()
        .args(["list", "--flavor", "era"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(
        plain.starts_with("no game root for flavor era: none in the registry"),
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
fn root_without_a_folder_prints_nothing_when_none_is_remembered_or_found() {
    let output = mrgl().arg("root").output().expect("run mrgl");
    assert!(output.status.success());
    assert_eq!(
        stdout_last_line(&output),
        "no game root: none remembered, none in the registry or the usual install folders; run `mrgl root <folder>`"
    );
    let json = mrgl().args(["--json", "root"]).output().expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert!(value["root"].is_null(), "{value}");
    assert!(value["root_origin"].is_null(), "{value}");
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
        stdout.ends_with("the usual install folders; run `mrgl root <folder>`\n"),
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

/// The stand-in client (task 063): a copy of the signed `ping.exe` of the system folder, named
/// `client-stand-in.exe` in the flavor folder and pinging the loopback address for an hour, so it runs
/// until the test stops it. Never a copy of this test binary: a binary that copies itself and runs the
/// copy is what an antivirus heuristic reads as a dropper. `None` when the system folder has no
/// `ping.exe`; the test is then skipped with a message.
#[cfg(windows)]
fn spawn_stand_in(flavor_folder: &Path) -> Option<StandIn> {
    let ping = PathBuf::from(std::env::var_os("SystemRoot")?)
        .join("System32")
        .join("ping.exe");
    if !ping.is_file() {
        return None;
    }
    let copy = flavor_folder.join("client-stand-in.exe");
    std::fs::copy(&ping, &copy).expect("the copy");
    let child = Command::new(&copy)
        .args(["-n", "3600", "127.0.0.1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the stand-in starts");
    Some(StandIn {
        child,
        stopped: false,
    })
}

/// The running stand-in client: `stop` kills and waits for it, and so does a drop, so a failing
/// assertion leaves no process behind.
#[cfg(windows)]
struct StandIn {
    child: Child,
    stopped: bool,
}

#[cfg(windows)]
impl StandIn {
    fn id(&self) -> u32 {
        self.child.id()
    }

    fn stop(&mut self) {
        self.child.kill().expect("killed");
        self.child.wait().expect("waited");
        self.stopped = true;
    }
}

#[cfg(windows)]
impl Drop for StandIn {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(windows)]
#[test]
fn scan_and_list_name_the_running_client_in_the_summary_and_the_json() {
    let root = temp_game_root("scan");
    let folder = root.join("_forever_");
    let Some(mut child) = spawn_stand_in(&folder) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };

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

    child.stop();
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

/// The synthetic game folder of `mrgl-core` whose `.build.info` has a row for retail only, with client
/// exe stubs (task 019).
fn client_exe_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/client-exe")
}

#[test]
fn scan_and_list_say_when_the_version_comes_from_the_client_exe() {
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(client_exe_fixtures().join("_classic_era_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output).trim_end(),
        "no addons in _classic_era_ (wow_classic_era 1.15.7.70100, version read from WowClassic.exe)",
        "the summary alone: no warning"
    );

    let output = mrgl()
        .args(["--serious", "list", "--flavor", "era", "--root"])
        .arg(client_exe_fixtures())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _classic_era_ (wow_classic_era 1.15.7.70100, version read from WowClassic.exe)"
    );

    let json = mrgl()
        .args(["--json", "list", "--flavor", "era", "--root"])
        .arg(client_exe_fixtures())
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "list");
    assert_eq!(value["flavor"]["version"], "1.15.7.70100");
    assert_eq!(value["flavor"]["interface"], 11507);
    assert_eq!(value["flavor"]["version_source"], "client-exe");
    assert_eq!(value["flavor"]["version_file"], "WowClassic.exe");
    assert_eq!(value["warnings"], serde_json::json!([]));

    // The row wins over the exe, and the summary says nothing about where the version came from.
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(client_exe_fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _retail_ (wow 12.1.5.70448)"
    );
    let json = mrgl()
        .args(["--json", "scan"])
        .arg(client_exe_fixtures().join("_retail_"))
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["flavor"]["version_source"], "build-info");
    assert_eq!(value["flavor"]["version_file"], ".build.info");

    // No row and an exe without a version resource: the build stays unknown, the warning names both.
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(client_exe_fixtures().join("_classic_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(
        stdout.contains(
            "build unknown: no row for wow_classic in .build.info; WowClassic.exe has no readable version"
        ),
        "{stdout}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "no addons in _classic_ (wow_classic, build unknown)"
    );
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

// --- task 037: `list` reads the sidecar, managed and unmanaged addons ---

impl Game {
    /// An addon folder put there by hand, with a CurseForge hint in its TOC: unmanaged.
    fn by_hand(&self, folder: &str) {
        let dir = self.addon(folder);
        std::fs::create_dir_all(&dir).expect("the addon folder");
        std::fs::write(
            dir.join(format!("{folder}.toc")),
            "## Interface: 16001\r\n## Title: By Hand\r\n## Version: 0.1\r\n## X-Curse-Project-ID: 4242\r\n\r\ncore.lua\r\n",
        )
        .expect("the TOC");
    }

    /// Installs a fixture zip through the engine (`mrgl add <zip>`).
    fn install(&self, zip: &str) {
        let output = self
            .mrgl()
            .arg("add")
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
    }

    fn list_json(&self) -> serde_json::Value {
        let output = self
            .mrgl()
            .args(["--json", "list", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        serde_json::from_slice(&output.stdout).expect("one JSON document")
    }
}

/// The addon object of a folder in a `list` document.
fn listed<'a>(value: &'a serde_json::Value, folder: &str) -> &'a serde_json::Value {
    value["addons"]
        .as_array()
        .expect("addons")
        .iter()
        .find(|addon| addon["folder"] == folder)
        .unwrap_or_else(|| panic!("{folder} in {value}"))
}

#[test]
fn list_shows_the_installed_addon_as_managed_and_the_rest_unmanaged() {
    let game = Game::new("list-managed");
    game.by_hand("ByHand");

    // No sidecar yet: every addon is unmanaged, without a word about the missing file, and list writes none.
    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output),
        "ByHand  0.1  16001  unmanaged  curse\n1 addon in _forever_ (wowf 1.60.1.70245): 0 managed by mrgl, 1 unmanaged\n"
    );
    assert!(!game.state_path().exists(), "list writes no sidecar");

    game.install("one-folder-1.0.0.zip");
    let zip = std::path::absolute(zips().join("one-folder-1.0.0.zip")).expect("the zip path");
    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "two addon lines and the summary: {text}");
    assert_eq!(lines[0], "ByHand  0.1  16001  unmanaged  curse");
    assert_eq!(
        lines[1],
        format!(
            "OneFolder  1.0.0  16001  managed  {}  forever:unknown",
            zip.display()
        )
    );
    assert_eq!(
        lines[2],
        "2 addons in _forever_ (wowf 1.60.1.70245): 1 managed by mrgl, 1 unmanaged"
    );

    let value = game.list_json();
    assert_eq!(value["kind"], "list");
    assert_eq!(
        value["counts"],
        serde_json::json!({ "addons": 2, "managed": 1, "unmanaged": 1 })
    );
    assert_eq!(value["root_origin"], "environment");
    assert_eq!(value["root"], game.root.display().to_string());
    assert_eq!(value["warnings"], serde_json::json!([]));
    let one = listed(&value, "OneFolder");
    assert_eq!(one["managed"], true);
    assert_eq!(one["name"], "OneFolder");
    assert_eq!(one["source"], zip.display().to_string());
    assert_eq!(one["version"], "1.0.0");
    assert_eq!(one["chosen"], "OneFolder.toc", "the scan's fields stay");
    let by_hand = listed(&value, "ByHand");
    assert_eq!(by_hand["managed"], false);
    assert!(by_hand["name"].is_null() && by_hand["source"].is_null());
    assert_eq!(by_hand["version"], "0.1");
    assert_eq!(by_hand["tocs"][0]["toc"]["curse_project_id"], "4242");

    // The version of a managed addon is the installed one, whatever its TOC says now.
    let toc = game.addon("OneFolder").join("OneFolder.toc");
    let edited = std::fs::read_to_string(&toc)
        .expect("the TOC")
        .replace("## Version: 1.0.0", "## Version: 9.9.9");
    std::fs::write(&toc, edited).expect("the TOC");
    assert_eq!(listed(&game.list_json(), "OneFolder")["version"], "1.0.0");
}

#[test]
fn list_names_the_owner_of_a_secondary_folder() {
    let game = Game::new("list-owner");
    game.install("several-folders-2.0.0.zip");
    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    assert!(
        lines[0].starts_with("SeveralFolders  2.0.0  16001  managed  "),
        "{text}"
    );
    assert!(
        lines[1].starts_with("SeveralFolders_Data  2.0.0  16001  managed by SeveralFolders  "),
        "{text}"
    );
    assert!(
        lines[2].starts_with("SeveralFolders_Options  2.0.0  16001  managed by SeveralFolders  "),
        "{text}"
    );
    assert_eq!(
        lines[3],
        "3 addons in _forever_ (wowf 1.60.1.70245): 3 managed by mrgl, 0 unmanaged"
    );
    let value = game.list_json();
    let options = listed(&value, "SeveralFolders_Options");
    assert_eq!(options["managed"], true);
    assert_eq!(options["name"], "SeveralFolders");
}

#[test]
fn list_rebuilds_a_corrupt_sidecar_without_a_word() {
    let game = Game::new("list-corrupt");
    game.install("one-folder-1.0.0.zip");
    std::fs::write(game.state_path(), "{ not json").expect("the corrupt sidecar");
    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output),
        "OneFolder  1.0.0  16001  unmanaged\n1 addon in _forever_ (wowf 1.60.1.70245): 0 managed by mrgl, 1 unmanaged\n",
        "no warning line"
    );
    assert!(output.stderr.is_empty(), "{}", stderr_last_line(&output));
    let value = game.list_json();
    assert_eq!(value["warnings"], serde_json::json!([]));
    assert_eq!(listed(&value, "OneFolder")["managed"], false);
    assert_eq!(
        std::fs::read_to_string(game.state_path()).expect("the sidecar"),
        "{ not json",
        "list writes nothing; the next install rewrites the sidecar"
    );
}

#[test]
fn list_json_names_the_root_origin_or_null_for_a_flavor_folder() {
    let output = mrgl()
        .args(["--json", "list", "--flavor", "era", "--root"])
        .arg(fixtures())
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["root_origin"], "flag");
    assert_eq!(value["root"], fixtures().display().to_string());

    let output = mrgl()
        .args(["--json", "list", "--flavor"])
        .arg(fixtures().join("_classic_era_"))
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert!(value["root_origin"].is_null(), "{value}");
    assert!(value["root"].is_null(), "{value}");
    assert_eq!(
        value["counts"],
        serde_json::json!({ "addons": 1, "managed": 0, "unmanaged": 1 })
    );

    // `scan` stays the raw folder view: no sidecar fields, no counts, no root.
    let output = mrgl()
        .args(["--json", "scan"])
        .arg(fixtures().join("_classic_era_"))
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    for field in ["counts", "root", "root_origin"] {
        assert!(value.get(field).is_none(), "{field} in {value}");
    }
    for field in ["managed", "name", "source", "version"] {
        assert!(
            value["addons"][0].get(field).is_none(),
            "{field} in {value}"
        );
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

/// Task 038: with the stand-in client of task 023 running from the flavor folder, `add <file.zip>` and
/// `remove` warn through the voice and in `--json` and still write; once it ends, nothing is said.
#[cfg(windows)]
#[test]
fn add_and_remove_warn_while_the_client_runs_and_still_write() {
    let game = Game::new("client-running");
    let Some(mut child) = spawn_stand_in(&game.root.join("_forever_")) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let install_line = format!(
        "the client is running in _forever_ (client-stand-in.exe, pid {}): the installed addons load at /reload or the next login",
        child.id()
    );

    let output = game
        .mrgl()
        .arg("add")
        .arg(zips().join("one-folder-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        6,
        "progress, the warning, the summary, two lines each: {text}"
    );
    assert_eq!(lines[3], install_line, "{text}");
    assert_eq!(
        lines[5],
        "installed OneFolder 1.0.0 (1 folder: OneFolder) into _forever_"
    );
    assert!(game.addon("OneFolder").join("OneFolder.toc").is_file());

    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("one-folder-1.1.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["client_warning"], install_line.as_str());
    assert_eq!(value["warnings"], serde_json::json!([]));
    assert!(game.addon("OneFolder").join("new.lua").is_file());

    let output = game
        .mrgl()
        .args(["--serious", "remove", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            format!(
                "the client is running in _forever_ (client-stand-in.exe, pid {}): the removed addons stay loaded until /reload or the next login",
                child.id()
            )
            .as_str(),
            "removed OneFolder 1.1.0 (1 folder: OneFolder) from _forever_",
        ]
    );
    assert!(!game.addon("OneFolder").exists());

    child.stop();
    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("one-folder-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert!(value["client_warning"].is_null(), "{value}");
    let output = game
        .mrgl()
        .args(["--json", "remove", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert!(value["client_warning"].is_null(), "{value}");
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
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "installed RootToc 3.0.0 (1 folder: RootToc) into _forever_"),
        "{text}"
    );
    // RootToc.toc names the retail interface 120105, which a Forever build does not load (task 015).
    assert_eq!(
        stdout_last_line(&output),
        "RootToc: interface 120105 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor"
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
fn an_unknown_source_spec_is_a_plain_error() {
    let output = mrgl()
        .args(["add", "wago:something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "unknown source: wago:something (expected github:<owner>/<repo>)"
    );
}

#[test]
fn add_dry_run_names_the_forever_asset_against_the_stub() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "resolving github:Questie/Questie for forever",
            "pick for forever: github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)",
            "forever build: yes (release.json names forever)",
            "dry run: nothing installed",
        ],
        "{stdout}"
    );
}

#[test]
fn add_speaks_murloc_on_every_message() {
    let output = mrgl()
        .args([
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert_eq!(
        stdout.lines().count(),
        8,
        "four two-line messages: {stdout}"
    );
    assert_eq!(stdout_last_line(&output), "dry run: nothing installed");
}

#[test]
fn add_reads_the_installed_build_from_the_game_folder() {
    // Without a game folder the progressing classic has no game type: the pick is a guess.
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "classic",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let guessed = stdout(&output);
    assert!(
        guessed.contains("pick for classic, a guess: github:Questie/Questie v1.2.3 (Questie-v1.2.3-mists.zip, flavor mists, interface 50504): release.json names mists (interface 50504); the installed build is unknown, so the game type is a guess"),
        "{guessed}"
    );
    // With the game folder (here through MRGL_ROOT) the build 5.5.4 names Mists: no guess.
    let output = mrgl()
        .env("MRGL_ROOT", fixtures())
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "classic",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let known = stdout(&output);
    assert!(
        known.contains("pick for classic: github:Questie/Questie v1.2.3 (Questie-v1.2.3-mists.zip, flavor mists, interface 50504): release.json names mists (interface 50504)"),
        "{known}"
    );
    // A flavor folder path works the same way.
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--dry-run",
            "--flavor",
        ])
        .arg(fixtures().join("_classic_era_"))
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output).contains("(Questie-v1.2.3-classic.zip, flavor classic, interface 11509)"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn add_json_is_one_document_with_the_pick() {
    let output = mrgl()
        .env("MRGL_ROOT", fixtures())
        .args([
            "--json",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "add");
    assert_eq!(value["source"], "github:Questie/Questie");
    assert_eq!(value["flavor"], "wowf");
    assert_eq!(value["build"], "1.60.1.70245");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["installed"], false);
    assert_eq!(value["pick"]["release"], "v1.2.3");
    assert_eq!(value["pick"]["version"], "1.2.3");
    assert_eq!(value["pick"]["asset"], "Questie-v1.2.3-forever.zip");
    assert_eq!(value["pick"]["flavor"], "forever");
    assert_eq!(value["pick"]["interface"], 16001);
    assert_eq!(value["pick"]["guess"], false);
    assert_eq!(value["pick"]["prerelease"], false);
    assert!(
        value["pick"]["url"]
            .as_str()
            .unwrap()
            .ends_with("/Questie-v1.2.3-forever.zip")
    );
    assert_eq!(value["warnings"], serde_json::json!([]));
}

// --- task 040: `add github:` installs the pick through the engine, against the folder stub ---

/// The sidecar of a game as JSON.
fn sidecar(game: &Game) -> serde_json::Value {
    let text = std::fs::read_to_string(game.state_path()).expect("the sidecar");
    serde_json::from_str(&text).expect("the sidecar is JSON")
}

/// The names under the flavor's staging folder (none once every install call has returned).
fn staging_entries(game: &Game) -> Vec<String> {
    match std::fs::read_dir(game.home.join("wowf").join("staging")) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn add_github_installs_the_pick_and_a_second_run_finds_it_installed() {
    let game = Game::new("github");
    let output = game
        .mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "resolving github:Questie/Questie for forever",
            "pick for forever: github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)",
            "downloading Questie-v1.2.3-forever.zip",
            "installing Questie-v1.2.3-forever.zip into _forever_",
            // The Forever signal of task 011 comes with the install too.
            "forever build: yes (release.json names forever)",
            "installed Questie 1.2.3 (1 folder: Questie) into _forever_",
        ],
        "{text}"
    );
    assert!(game.addon("Questie").join("Questie.toc").is_file());
    // The sidecar records the pick's source and version; the TOC says `v1.2.3`, the fallback only.
    let state = sidecar(&game);
    assert_eq!(
        state["addons"]["Questie"]["source"],
        "github:Questie/Questie"
    );
    assert_eq!(state["addons"]["Questie"]["version"], "1.2.3");
    assert!(
        std::fs::read_to_string(game.addon("Questie").join("Questie.toc"))
            .unwrap()
            .contains("## Version: v1.2.3")
    );
    assert_eq!(
        staging_entries(&game),
        Vec::<String>::new(),
        "the download is gone"
    );
    assert!(
        game.home
            .join("wowf")
            .join("cache")
            .join("Questie")
            .join("1.2.3.zip")
            .is_file(),
        "the engine keeps the zip for a rollback"
    );

    // The second run finds it in the sidecar: no download, no swap, and every message has its murloc line.
    let before = std::fs::read(game.state_path()).unwrap();
    let output = game
        .mrgl()
        .args(["add", "github:Questie/Questie", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    // Four two-line messages: the resolve, the pick, the Forever line of task 011, the outcome.
    assert_eq!(text.lines().count(), 8, "four two-line messages: {text}");
    assert!(!text.contains("downloading"), "{text}");
    assert_eq!(
        stdout_last_line(&output),
        "Questie 1.2.3 is installed already in _forever_ from github:Questie/Questie, nothing to do"
    );
    assert_eq!(
        std::fs::read(game.state_path()).unwrap(),
        before,
        "the sidecar is untouched"
    );
}

#[test]
fn add_github_updates_another_version_and_json_is_one_document() {
    let game = Game::new("github-json");
    let run = |args: &[&str]| {
        game.mrgl()
            .args(args)
            .args(["add", "github:Questie/Questie", "--flavor", "forever"])
            .output()
            .expect("run mrgl")
    };
    let output = run(&["--json"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "add");
    assert_eq!(value["source"], "github:Questie/Questie");
    assert_eq!(value["flavor"], "wowf");
    assert_eq!(value["build"], "1.60.1.70245");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["installed"], true);
    assert_eq!(value["already_installed"], false);
    assert_eq!(value["pick"]["version"], "1.2.3");
    assert_eq!(value["pick"]["asset"], "Questie-v1.2.3-forever.zip");
    assert_eq!(
        value["addon"],
        serde_json::json!({
            "name": "Questie",
            "version": "1.2.3",
            "previous_version": null,
            "source": "github:Questie/Questie",
            "folders": ["Questie"],
            "removed_folders": [],
            // The dependency check of task 029 runs on the installed folders.
            "dependencies": [{
                "folder": "Questie", "required": [], "met": [], "embedded": [], "missing": [], "nolib": false
            }],
        })
    );
    assert_eq!(value["warnings"], serde_json::json!([]));

    // Another version in the sidecar: the same call updates it.
    let mut state = sidecar(&game);
    state["addons"]["Questie"]["version"] = "1.2.2".into();
    std::fs::write(game.state_path(), state.to_string()).unwrap();
    let output = run(&["--serious"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout(&output).contains("downloading Questie-v1.2.3-forever.zip"));
    assert_eq!(
        stdout_last_line(&output),
        "updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in _forever_"
    );

    let output = run(&["--json"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["installed"], false);
    assert_eq!(value["already_installed"], true);
    assert_eq!(value["addon"]["version"], "1.2.3");
    assert_eq!(value["addon"]["previous_version"], "1.2.2");
    assert_eq!(value["addon"]["source"], "github:Questie/Questie");
}

#[test]
fn add_github_without_dry_run_needs_the_game_folder_and_the_asset() {
    // No game folder: the dry run works on the flavor id alone, the install needs the AddOns folder.
    let output = mrgl()
        .args(["add", "github:Questie/Questie", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_last_line(&output).starts_with("no game root for flavor forever: "),
        "{}",
        stderr_last_line(&output)
    );

    // An asset the stub does not serve: the download fails, nothing is installed, nothing stays behind.
    let game = Game::new("github-missing");
    let output = game
        .mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/NoReleaseJson",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        // The asset's 404 names the asset and the release (task 046), not the release list's hint.
        "the asset Thing-3.0.0-forever.zip of github:Example/NoReleaseJson 3.0.0 is missing: GitHub answered 404 for https://github.com/Example/NoReleaseJson/releases/download/3.0.0/Thing-3.0.0-forever.zip (removed or renamed after the release was listed)"
    );
    let text = stdout(&output);
    assert!(
        text.contains("downloading Thing-3.0.0-forever.zip"),
        "{text}"
    );
    assert!(!text.contains("installing"), "{text}");
    assert_eq!(staging_entries(&game), Vec::<String>::new());
    assert!(!game.state_path().exists());
}

#[test]
fn add_a_pick_by_name_is_a_warning() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/NoReleaseJson",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("pick for forever, a guess: github:Example/NoReleaseJson 3.0.0 (Thing-3.0.0-forever.zip, flavor forever): no release.json names forever; the asset name ends in -forever, a guess"),
        "{stdout}"
    );
    let json = mrgl()
        .args([
            "--json",
            "add",
            "github:Example/NoReleaseJson",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["pick"]["guess"], true);
    assert_eq!(value["pick"]["interface"], serde_json::Value::Null);
}

#[test]
fn add_failures_are_plain_lines_with_exit_code_one() {
    let cases = [
        (
            vec!["add", "wago:abc", "--flavor", "forever", "--dry-run"],
            "unknown source: wago:abc (expected github:<owner>/<repo>)",
        ),
        (
            vec!["add", "github:Questie/Questie", "--dry-run"],
            "no flavor given: --flavor forever, retail, classic, era, a .flavor.info id or a flavor folder",
        ),
        (
            vec![
                "add",
                "github:Questie/Questie",
                "--flavor",
                "plunderstorm",
                "--dry-run",
            ],
            "unknown flavor: plunderstorm (forever, retail, classic, era, a .flavor.info id or a flavor folder)",
        ),
        (
            vec![
                "add",
                "github:Example/Empty",
                "--flavor",
                "forever",
                "--dry-run",
            ],
            "no release in github:Example/Empty",
        ),
        (
            vec![
                "add",
                "github:Example/Missing",
                "--flavor",
                "forever",
                "--dry-run",
            ],
            "GitHub answered 404 for https://api.github.com/repos/Example/Missing/releases?per_page=30: no such repository, or it is private",
        ),
        (
            vec![
                "add",
                "github:Example/MainlineOnly",
                "--flavor",
                "forever",
                "--dry-run",
            ],
            "no asset for flavor forever in github:Example/MainlineOnly v4.0.0 (release.json names mainline, standard; assets: Only-v4.0.0.zip, release.json)",
        ),
    ];
    for (args, plain) in cases {
        let output = mrgl().args(&args).output().expect("run mrgl");
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(stderr_last_line(&output), plain, "{args:?}");
    }
}

#[test]
fn add_caches_the_release_list_under_the_home_folder() {
    let home = temp_home("release-cache");
    let run = || {
        mrgl()
            .env("MRGL_HOME", &home)
            .args([
                "--serious",
                "add",
                "github:Questie/Questie",
                "--flavor",
                "forever",
                "--dry-run",
            ])
            .output()
            .expect("run mrgl")
    };
    let first = run();
    assert!(first.status.success(), "{}", stderr_last_line(&first));
    let cache = home
        .join("cache")
        .join("github")
        .join("questie")
        .join("questie");
    let text =
        std::fs::read_to_string(cache.join("releases.json")).expect("the release list is cached");
    assert!(text.contains(r#""etag""#), "with its tag: {text}");
    let second = run();
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "the cached list gives the same pick"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// The synthetic `WTF` tree of `mrgl-core`.
fn fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/wtf/WTF")
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

/// A tree as relative paths to the bytes of every file, folders as names with a trailing `/`.
fn snapshot(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, into: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("read") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() {
                into.insert(format!("{prefix}{name}/"), Vec::new());
                walk(&entry.path(), &format!("{prefix}{name}/"), into);
            } else {
                into.insert(
                    format!("{prefix}{name}"),
                    std::fs::read(entry.path()).expect("bytes"),
                );
            }
        }
    }
    let mut into = std::collections::BTreeMap::new();
    walk(dir, "", &mut into);
    into
}

/// A synthetic game folder with a `_forever_` flavor and the fixture `WTF` tree copied in.
fn temp_game_root_with_wtf(name: &str) -> PathBuf {
    let root = temp_game_root(name);
    copy_tree(&fixture_wtf(), &root.join("_forever_").join("WTF"));
    root
}

#[test]
fn backup_writes_a_dated_zip_lists_it_and_restores_it() {
    let root = temp_game_root_with_wtf("backup");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    let backups = root.join("backups");
    let before = snapshot(&wtf);

    let output = mrgl()
        .args(["--serious", "backup", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.starts_with("backing up WTF of _forever_ into "),
        "{text}"
    );
    let line = stdout_last_line(&output);
    assert!(
        line.starts_with("backed up WTF of _forever_: 8 files, "),
        "{line}"
    );
    let zip = PathBuf::from(line.rsplit(", in ").next().expect("the zip path"));
    assert!(zip.is_file(), "{}", zip.display());
    assert_eq!(zip.parent(), Some(backups.as_path()));
    let name = zip
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    assert!(name.starts_with("WTF-") && name.ends_with(".zip"), "{name}");

    let output = mrgl()
        .args(["--serious", "backup", "--list", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(
        lines[0].starts_with(&name) && lines[0].contains("  8 files  "),
        "{}",
        lines[0]
    );
    assert_eq!(
        lines[1],
        format!("1 backup of _forever_ in {}", backups.display())
    );

    let json = mrgl()
        .args(["--json", "backup", "--list", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "backup-list");
    assert_eq!(value["backups"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["backups"][0]["name"], name);
    assert_eq!(value["backups"][0]["files"], 8);
    assert_eq!(value["aside"], serde_json::json!([]));
    assert_eq!(value["warnings"], serde_json::json!([]));

    // Changed since the backup; the restore puts the backup back and the change aside.
    std::fs::write(wtf.join("Config.wtf"), b"SET portal \"changed\"\r\n").expect("changed");
    let output = mrgl()
        .args([
            "--serious",
            "backup",
            "--yes",
            "--restore",
            &name,
            "--flavor",
        ])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let line = stdout_last_line(&output);
    assert!(
        line.starts_with(&format!(
            "restored WTF of _forever_ from {name}: 8 files; the previous WTF is at "
        )),
        "{line}"
    );
    assert_eq!(snapshot(&wtf), before, "byte for byte");
    let aside = PathBuf::from(line.rsplit(" is at ").next().expect("the aside path"));
    assert!(aside.is_dir(), "{}", aside.display());
    assert_eq!(
        std::fs::read(aside.join("Config.wtf")).expect("the aside copy"),
        b"SET portal \"changed\"\r\n"
    );

    let output = mrgl()
        .args(["--serious", "backup", "--list", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "1 backup of _forever_ in {}, 1 WTF folder moved aside by a restore",
            backups.display()
        )
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn backup_json_is_one_document_and_the_default_folder_is_under_the_home() {
    let root = temp_game_root_with_wtf("backup-json");
    let home = temp_home("backup-json");
    let flavor = root.join("_forever_");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--json", "backup", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "backup");
    assert_eq!(value["flavor"]["id"], "wowf");
    assert_eq!(value["files"], 8);
    assert_eq!(value["folders"], 6);
    assert!(value["bytes"].as_u64().is_some_and(|bytes| bytes > 0));
    assert_eq!(value["warnings"], serde_json::json!([]));
    let zip = PathBuf::from(value["zip"].as_str().expect("the zip path"));
    assert!(zip.is_file());
    assert_eq!(
        zip.parent(),
        Some(home.join("backups").join("wowf").as_path())
    );

    let json = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--json", "backup", "--yes", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(json.status.success(), "{}", stderr_last_line(&json));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "restore");
    assert_eq!(value["files"], 8);
    assert_eq!(value["folders"], 6);
    assert!(
        value["aside"]
            .as_str()
            .is_some_and(|aside| aside.contains("WTF-before-restore-")),
        "{value}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn restore_asks_first_unless_yes_and_json_needs_yes() {
    let root = temp_game_root_with_wtf("restore-ask");
    let flavor = root.join("_forever_");
    let backups = root.join("backups");
    let output = mrgl()
        .args(["--json", "backup", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let zip = PathBuf::from(value["zip"].as_str().expect("the zip path"));
    let before = snapshot(&flavor.join("WTF"));

    let mut child = mrgl()
        .args(["--serious", "backup", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run mrgl");
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(b"n\n")
            .expect("the answer");
    }
    let output = child.wait_with_output().expect("finished");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.starts_with(&format!(
            "restore {} as the WTF folder of _forever_? ",
            zip.file_name().expect("a name").to_string_lossy()
        )),
        "{stdout}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "restore cancelled, nothing changed"
    );
    assert_eq!(snapshot(&flavor.join("WTF")), before);

    let json = mrgl()
        .args(["--json", "backup", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert_eq!(json.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&json.stderr).expect("a JSON error");
    assert_eq!(value["kind"], "error");
    assert!(
        value["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("restore needs --yes with --json")),
        "{value}"
    );
    assert_eq!(snapshot(&flavor.join("WTF")), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn backup_without_a_wtf_folder_and_a_missing_backup_are_errors() {
    let root = temp_game_root("no-wtf");
    let flavor = root.join("_forever_");
    let output = mrgl()
        .args(["backup", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(root.join("backups"))
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!("nothing to back up: no WTF folder in {}", flavor.display())
    );
    assert!(!root.join("backups").exists());

    let output = mrgl()
        .args([
            "backup",
            "--yes",
            "--restore",
            "WTF-2026-01-01-000000.zip",
            "--flavor",
        ])
        .arg(&flavor)
        .arg("--to")
        .arg(root.join("backups"))
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "backup not found: {}",
            root.join("backups")
                .join("WTF-2026-01-01-000000.zip")
                .display()
        )
    );

    let output = mrgl()
        .args(["backup", "--list", "--restore", "x.zip"])
        .output()
        .expect("run mrgl");
    assert_eq!(
        output.status.code(),
        Some(2),
        "--list and --restore conflict"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn restore_is_refused_while_the_client_runs() {
    let root = temp_game_root_with_wtf("restore-client");
    let flavor = root.join("_forever_");
    let backups = root.join("backups");
    let output = mrgl()
        .args(["--json", "backup", "--flavor"])
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let zip = PathBuf::from(value["zip"].as_str().expect("the zip path"));
    let before = snapshot(&flavor.join("WTF"));

    let Some(mut child) = spawn_stand_in(&flavor) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let output = mrgl()
        .args(["backup", "--yes", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&flavor.join("WTF")), before, "nothing was touched");

    let json = mrgl()
        .args(["--json", "backup", "--yes", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert_eq!(json.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&json.stderr).expect("a JSON error");
    assert_eq!(value["kind"], "error");

    child.stop();
    let output = mrgl()
        .args(["--serious", "backup", "--yes", "--restore"])
        .arg(&zip)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout_last_line(&output).starts_with("restored WTF of _forever_ from "));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn add_dry_run_shows_the_release_notes_line_and_the_page() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/Notes",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let serious = stdout(&output);
    let lines: Vec<&str> = serious.lines().collect();
    assert_eq!(
        lines,
        [
            "resolving github:Example/Notes for forever",
            "pick for forever: github:Example/Notes v2.1.0 (Notes-v2.1.0-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)",
            "release notes: Quest arrows point the right way again (https://github.com/Example/Notes/releases/tag/v2.1.0)",
            "forever build: yes (release.json names forever)",
            "dry run: nothing installed",
        ],
        "{serious}"
    );
    // The murloc speaks the notes line too: five two-line messages (the Forever line is task 011's).
    let output = mrgl()
        .args([
            "add",
            "github:Example/Notes",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert_eq!(stdout(&output).lines().count(), 10, "{}", stdout(&output));
    let json = mrgl()
        .args([
            "--json",
            "add",
            "github:Example/Notes",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(
        value["pick"]["notes"],
        "Quest arrows point the right way again"
    );
    assert_eq!(
        value["pick"]["page"],
        "https://github.com/Example/Notes/releases/tag/v2.1.0"
    );
}

#[test]
fn add_dry_run_of_a_release_with_an_empty_body_shows_no_notes() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/NoNotes",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 4, "no notes line and no empty line: {stdout}");
    assert!(
        lines[1].starts_with("pick for forever, a guess: github:Example/NoNotes v1.0.0"),
        "{stdout}"
    );
    assert!(!stdout.contains("release notes"), "{stdout}");
    // The Forever line (task 011) sits where the notes line would be.
    assert_eq!(lines[2], "forever build: unknown (no release.json says)");
    assert_eq!(lines[3], "dry run: nothing installed");
    let json = mrgl()
        .args([
            "--json",
            "add",
            "github:Example/NoNotes",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert!(value["pick"].get("notes").is_none(), "{value}");
    assert_eq!(
        value["pick"]["page"],
        "https://github.com/Example/NoNotes/releases/tag/v1.0.0"
    );
}

// --- task 029: the dependency check in `list` and after `add` ---

/// The synthetic game folder of the dependency check: `Needy` with a met, an embedded and a missing
/// required dependency, `LibMet`, and `NolibBuild` with a library folder inside.
fn dependency_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/dependencies/_forever_")
}

#[test]
fn list_warns_about_a_missing_dependency_and_marks_the_line() {
    let output = mrgl()
        .args(["--serious", "list", "--flavor"])
        .arg(dependency_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "LibMet  1.0.0  16001  unmanaged",
            "Needy  1.0.0  16001  unmanaged  missing: LibMissing",
            "NolibBuild  1.0.0  16001  unmanaged",
            "Needy needs the missing dependency LibMissing: install it or the addon does not load",
            "3 addons in _forever_ (wowf 1.60.1.70245): 0 managed by mrgl, 3 unmanaged",
        ],
        "{text}"
    );
}

#[test]
fn list_json_carries_the_dependencies_of_every_addon() {
    let output = mrgl()
        .args(["--json", "list", "--flavor"])
        .arg(dependency_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let addon = |folder: &str| {
        value["addons"]
            .as_array()
            .expect("addons")
            .iter()
            .find(|addon| addon["folder"] == folder)
            .cloned()
            .unwrap_or_else(|| panic!("no {folder} in {value}"))
    };
    assert_eq!(
        addon("Needy")["dependencies"],
        serde_json::json!({
            "folder": "Needy",
            "required": ["LibMet", "LibEmbedded", "LibMissing", "Blizzard_Something"],
            "met": ["LibMet", "Blizzard_Something"],
            "embedded": ["LibEmbedded"],
            "missing": ["LibMissing"],
            "nolib": null,
        })
    );
    assert_eq!(
        addon("LibMet")["dependencies"]["required"],
        serde_json::json!([])
    );
    assert_eq!(
        value["warnings"],
        serde_json::json!([
            "Needy needs the missing dependency LibMissing: install it or the addon does not load"
        ])
    );
}

#[test]
fn list_reads_the_nolib_flag_from_the_sidecar() {
    let home = temp_home("nolib-flag");
    std::fs::create_dir_all(home.join("wowf")).expect("the flavor's home");
    std::fs::write(
        home.join("wowf").join("state.json"),
        r#"{"schema": 1, "flavor": "wowf", "addons": {"NolibBuild": {"source": "github:someone/nolib-build",
            "version": "1.0.0", "folders": ["NolibBuild"], "files": {}, "installed_at": "2026-10-21T14:03:05Z",
            "nolib": true}}}"#,
    )
    .expect("the sidecar");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "list", "--flavor"])
        .arg(dependency_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "NolibBuild  1.0.0  16001  managed  github:someone/nolib-build  missing: LibEmbedded  forever:unknown"),
        "{text}"
    );
    assert!(
        text.lines().any(|line| line
            == "NolibBuild needs the missing dependency LibEmbedded: install it or the addon does not load (a nolib build: its libraries are addons of their own)"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn scan_reads_the_folder_only_and_checks_no_dependency() {
    let output = mrgl()
        .args(["--json", "scan"])
        .arg(dependency_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert!(value["addons"][0].get("dependencies").is_none(), "{value}");
    assert_eq!(value["warnings"], serde_json::json!([]));
}

#[test]
fn add_warns_after_the_install_when_a_dependency_is_missing() {
    let game = Game::new("dependencies");
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("needs-library-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(
        output.status.success(),
        "a warning, never a refusal: {}",
        stderr_last_line(&output)
    );
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "installing needs-library-1.0.0.zip into _forever_",
            "NeedsLibrary needs the missing dependency LibMissing: install it or the addon does not load",
            "installed NeedsLibrary 1.0.0 (1 folder: NeedsLibrary) into _forever_",
        ],
        "{text}"
    );
    assert!(
        game.addon("NeedsLibrary")
            .join("NeedsLibrary.toc")
            .is_file()
    );

    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("needs-library-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let dependencies = &value["addon"]["dependencies"][0];
    assert_eq!(dependencies["folder"], "NeedsLibrary");
    assert_eq!(dependencies["embedded"], serde_json::json!(["LibEmbedded"]));
    assert_eq!(
        dependencies["met"],
        serde_json::json!(["Blizzard_Something"]),
        "the client's own addon (task 064)"
    );
    assert_eq!(dependencies["missing"], serde_json::json!(["LibMissing"]));
    assert_eq!(
        dependencies["nolib"],
        serde_json::Value::Null,
        "a zip in hand does not say"
    );
}

// --- task 011: the Forever notes on `add --dry-run` and `list` ---

#[test]
fn add_dry_run_carries_the_forever_signal() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/NoReleaseJson",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output).contains("\nforever build: unknown (no release.json says)\n"),
        "{}",
        stdout(&output)
    );
    let json = |source: &str, flavor: &str| {
        let output = mrgl()
            .args(["--json", "add", source, "--flavor", flavor, "--dry-run"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("one JSON document")
    };
    let questie = json("github:Questie/Questie", "forever");
    assert_eq!(questie["pick"]["forever_build"], "yes");
    assert_eq!(questie["forever"]["build"], "yes");
    assert_eq!(questie["forever"]["note"], serde_json::Value::Null);
    assert_eq!(questie["forever"]["stale"], false);
    // Another flavor: the pick still says what the source publishes, the Forever note is not asked for.
    let retail = json("github:Example/MainlineOnly", "retail");
    assert_eq!(retail["pick"]["forever_build"], "no");
    assert_eq!(retail["forever"], serde_json::Value::Null);
    // A retail dry run prints no Forever line.
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Example/MainlineOnly",
            "--flavor",
            "retail",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(
        !stdout(&output).contains("forever build"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn list_marks_a_managed_forever_addon_from_the_cache_alone() {
    let game = Game::new("forever-list");
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("one-folder-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    // The install from a GitHub pick is task 040: the sidecar entry is pointed at the repository by hand.
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(game.state_path()).expect("the sidecar")).unwrap();
    state["addons"]["OneFolder"]["source"] = "github:Questie/Questie".into();
    std::fs::write(game.state_path(), state.to_string()).unwrap();
    let list = |json: bool| {
        let mut command = game.mrgl();
        command.arg(if json { "--json" } else { "--serious" });
        let output = command
            .args(["list", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        output
    };

    // Nothing cached yet: unknown.
    let before = stdout(&list(false));
    let line = before
        .lines()
        .find(|line| line.starts_with("OneFolder  "))
        .expect("the addon line");
    assert!(line.ends_with("  forever:unknown"), "{before}");

    // A dry run fetches the release list and its release.json; list reads them back offline.
    let output = game
        .mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let after = list(false);
    let text = stdout(&after);
    let line = text
        .lines()
        .find(|line| line.starts_with("OneFolder  "))
        .expect("the addon line");
    assert!(line.ends_with("  forever:yes"), "{text}");
    let value: serde_json::Value =
        serde_json::from_slice(&list(true).stdout).expect("one JSON document");
    let addon = value["addons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|addon| addon["folder"] == "OneFolder")
        .expect("the addon");
    assert_eq!(
        addon["forever"],
        serde_json::json!({
            "build": "yes",
            "note": null,
            "kind": null,
            "verified": null,
            "source": null,
            "stale": false,
        })
    );

    // Another flavor's listing carries no Forever field.
    let output = mrgl()
        .args(["--json", "list", "--flavor", "retail", "--root"])
        .arg(fixtures())
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let addons = value["addons"].as_array().expect("the addons");
    assert!(!addons.is_empty());
    assert!(addons.iter().all(|addon| addon.get("forever").is_none()));
}

/// A made-up token: the doctor names the key and never prints its value (task 024).
const DOCTOR_SECRET: &str = "ghp_madeUpTokenValueForTheDoctorEndToEndTest";

/// The doctor's synthetic game folder: the client tests' `_forever_` with an empty AddOns folder.
fn doctor_game_root(name: &str) -> PathBuf {
    let root = temp_game_root(&format!("doctor-{name}"));
    std::fs::create_dir_all(root.join("_forever_").join("Interface").join("AddOns"))
        .expect("the AddOns folder");
    root
}

/// Asserts that an output carries no home folder path: neither the profile folder of this run nor any
/// `\Users\` folder (the 8.3 short form a runner's temporary folder carries included).
fn assert_no_home_path(output: &str) {
    let lower = output.to_lowercase();
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        let profile = profile.to_string_lossy().to_lowercase();
        assert!(!lower.contains(&profile), "the profile folder in: {output}");
    }
    assert!(
        !lower.contains(":\\users\\") && !lower.contains(":\\\\users\\\\"),
        "a home folder path in: {output}"
    );
}

#[test]
fn doctor_prints_every_line_offline_without_a_key_value_or_a_home_path() {
    let root = doctor_game_root("lines");
    let home = temp_home("doctor-lines");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env("MRGL_GITHUB_TOKEN", DOCTOR_SECRET)
        .env("MRGL_DISCOVERY", "webview2=131.0.2903.70")
        .args(["--serious", "doctor", "--root"])
        .arg(&root)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    let root_name = root
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    assert_eq!(lines[0], format!("mrgl: {}", env!("CARGO_PKG_VERSION")));
    assert_eq!(
        lines[1], "windows: unknown",
        "the stub names no Windows version"
    );
    assert!(
        lines[2].starts_with("root: ") && lines[2].ends_with(&format!("{root_name} (from --root)")),
        "{stdout}"
    );
    assert!(
        lines[3].starts_with("home: ") && lines[3].ends_with("doctor-lines"),
        "{stdout}"
    );
    let rest = [
        "webview2: 131.0.2903.70 (per-machine)",
        "github rate limit: 58 of 60 left, resets 2026-10-21T16:00:00Z",
        "api keys: MRGL_GITHUB_TOKEN set",
        "flavor: _forever_ (wowf)",
    ];
    assert_eq!(lines[4..8], rest, "{stdout}");
    assert!(
        lines[8].starts_with("  folder: ") && lines[8].ends_with("_forever_"),
        "{stdout}"
    );
    let flavor = [
        "  build: 1.60.1.70245, interface 16001, from .build.info",
        "  curseforge app: no",
        "  addons folder: writable",
        "  sidecar: missing",
        "  cache: empty",
        "  client: not running",
        "diagnostics of 1 flavor, nothing to look at; paste the lines above into a bug report",
    ];
    assert_eq!(lines[9..], flavor, "{stdout}");
    assert!(!stdout.contains(DOCTOR_SECRET), "{stdout}");
    assert_no_home_path(&stdout);
    // The probe left nothing behind, and the home folder got nothing either.
    let left = std::fs::read_dir(root.join("_forever_").join("Interface").join("AddOns"))
        .expect("the AddOns folder")
        .count();
    assert_eq!(left, 0);
    assert!(
        !home.exists(),
        "the doctor writes nothing under the home folder"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_json_is_one_document_without_a_key_value_or_a_home_path() {
    let root = doctor_game_root("json");
    let output = mrgl()
        .env("MRGL_GITHUB_TOKEN", DOCTOR_SECRET)
        .env("MRGL_DISCOVERY", "webview2=131.0.2903.70")
        .args(["--json", "doctor", "--flavor", "forever", "--root"])
        .arg(&root)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(!text.contains(DOCTOR_SECRET), "{text}");
    assert_no_home_path(&text);
    let value: serde_json::Value = serde_json::from_str(&text).expect("one JSON document");
    assert_eq!(value["kind"], "doctor");
    assert_eq!(value["mrgl_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["root_origin"], "flag");
    assert_eq!(value["webview2"]["version"], "131.0.2903.70");
    assert_eq!(value["github_rate_limit"]["remaining"], 58);
    assert_eq!(value["github_rate_limit"]["limit"], 60);
    assert_eq!(
        value["api_keys"],
        serde_json::json!([{ "name": "MRGL_GITHUB_TOKEN", "present": true }])
    );
    let flavor = &value["flavors"][0];
    assert_eq!(flavor["id"], "wowf");
    assert_eq!(flavor["version"], "1.60.1.70245");
    assert_eq!(flavor["version_source"], "build-info");
    assert_eq!(flavor["addons"]["status"], "writable");
    assert_eq!(flavor["sidecar"]["status"], "missing");
    assert_eq!(flavor["cache"]["bytes"], 0);
    assert_eq!(flavor["client"]["status"], "not_running");
    assert_eq!(value["warnings"], serde_json::json!([]));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn doctor_offline_without_a_root_says_unknown_and_succeeds() {
    let empty_stub = temp_home("doctor-offline-stub");
    std::fs::create_dir_all(&empty_stub).expect("an empty stub folder");
    let output = mrgl()
        .env("MRGL_HTTP_STUB", &empty_stub)
        .arg("doctor")
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[1], "windows: unknown");
    assert_eq!(lines[2], "root: none found");
    assert_eq!(lines[4], "webview2: unknown");
    assert_eq!(lines[5], "github rate limit: unknown (HTTP 404)");
    assert_eq!(lines[6], "api keys: MRGL_GITHUB_TOKEN not set");
    // The summary speaks through the voice: a murloc line, then the plain one.
    assert_eq!(lines.len(), 9, "{stdout}");
    assert!(lines[7].starts_with("Mrgl"), "{stdout}");
    assert_eq!(
        lines[8],
        "diagnostics of no flavor, 1 thing to look at (no game root); paste the lines above into a bug report"
    );
    assert_no_home_path(&stdout);
    let _ = std::fs::remove_dir_all(&empty_stub);
}

#[test]
fn doctor_names_a_missing_flavor_and_refuses_an_unknown_one() {
    let root = doctor_game_root("flavor");
    let output = mrgl()
        .args(["--serious", "doctor", "--flavor", "era", "--root"])
        .arg(&root)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout
            .lines()
            .any(|line| line == "flavor: _classic_era_ (wow_classic_era), folder missing"),
        "{stdout}"
    );
    assert!(!root.join("_classic_era_").exists(), "nothing created");
    assert_eq!(
        stdout_last_line(&output),
        "diagnostics of 1 flavor, 1 thing to look at (_classic_era_ is missing); paste the lines above into a bug report"
    );

    let output = mrgl()
        .args(["doctor", "--flavor", "bogus"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "unknown flavor: bogus (forever, retail, classic, era or a .flavor.info id)"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// --- task 005: `link` and `unlink` over the synthetic development folder, in a temporary game folder ---

/// A copy of the synthetic development folder of `mrgl-core` (`OneFolder` and `DevAddon` with a TOC,
/// `Media` without one) next to the game folder, so no run can touch the fixture itself.
#[cfg(windows)]
fn dev_copy(game: &Game) -> PathBuf {
    let dev = game.root.parent().expect("the base folder").join("dev");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/dev"),
        &dev,
    );
    dev
}

#[cfg(windows)]
fn addons_listing(game: &Game) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(game.addon(""))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[cfg(windows)]
#[test]
fn link_lists_refuses_and_unlink_leaves_the_development_folder_untouched() {
    let game = Game::new("link");
    let dev = dev_copy(&game);
    let before = snapshot(&dev);

    let output = game
        .mrgl()
        .args(["--serious", "link"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(
        lines[0],
        format!("linking the addons of {} into _forever_", dev.display())
    );
    assert!(lines[1].starts_with("linked 2 addons from "), "{text}");
    assert!(
        lines[1].ends_with(" into _forever_: DevAddon, OneFolder, 1 folder without a TOC left out"),
        "{text}"
    );
    assert_eq!(addons_listing(&game), ["DevAddon", "OneFolder"]);

    // `list` reads through the junctions and flags them.
    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let one = text
        .lines()
        .find(|line| line.starts_with("OneFolder"))
        .expect("the OneFolder line");
    // The Forever mark of task 011 follows the flags on a Forever listing.
    assert!(
        one.contains("9.9.9-dev") && one.contains("  linked"),
        "{one}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("DevAddon") && line.contains("linked")),
        "{text}"
    );

    // An install, a dry run or a removal over a linked addon is refused, and the junction stays.
    for args in [vec!["add", "--dry-run"], vec!["add"]] {
        let output = game
            .mrgl()
            .args(&args)
            .arg(zips().join("one-folder-1.0.0.zip"))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let plain = stderr_last_line(&output);
        assert!(
            plain.starts_with(
                "refused to install over OneFolder (flavor forever): it is linked from "
            ),
            "{plain}"
        );
        assert!(
            plain.ends_with("; `mrgl unlink` with that folder removes the link"),
            "{plain}"
        );
    }
    let output = game
        .mrgl()
        .args(["remove", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_last_line(&output).starts_with("refused to remove OneFolder (flavor forever): "),
        "{}",
        stderr_last_line(&output)
    );
    assert_eq!(addons_listing(&game), ["DevAddon", "OneFolder"]);
    assert!(game.addon("OneFolder").join("OneFolder.lua").is_file());

    // A second link says there is nothing new, with the murloc line on top of the plain one.
    let output = game
        .mrgl()
        .arg("link")
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(stdout(&output).lines().count(), 4, "{}", stdout(&output));
    assert!(
        stdout_last_line(&output).contains(", 2 of them linked already"),
        "{}",
        stdout(&output)
    );

    let output = game
        .mrgl()
        .args(["--serious", "unlink"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let last = stdout_last_line(&output);
    assert!(last.starts_with("unlinked 2 addons of "), "{last}");
    assert!(
        last.ends_with(" from _forever_: DevAddon, OneFolder"),
        "{last}"
    );
    assert!(addons_listing(&game).is_empty());
    assert_eq!(snapshot(&dev), before, "the development folder");
    let state = std::fs::read_to_string(game.state_path()).expect("the sidecar");
    assert!(!state.contains("link:"), "{state}");

    let output = game
        .mrgl()
        .args(["--serious", "unlink"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout_last_line(&output).starts_with("nothing linked from "),
        "{}",
        stdout(&output)
    );
}

#[cfg(windows)]
#[test]
fn link_and_unlink_json_are_one_document_each() {
    let game = Game::new("link-json");
    let dev = dev_copy(&game);
    let output = game
        .mrgl()
        .args(["--json", "link"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "link");
    assert_eq!(value["flavor"]["id"], "wowf");
    let source = value["source"].as_str().expect("a source");
    assert_eq!(
        source,
        format!("link:{}", value["folder"].as_str().expect("a folder"))
    );
    let linked = value["linked"].as_array().expect("linked");
    assert_eq!(linked.len(), 2);
    assert_eq!(linked[0]["name"], "DevAddon");
    assert_eq!(linked[0]["version"], "unknown");
    assert_eq!(linked[1]["name"], "OneFolder");
    assert_eq!(linked[1]["version"], "9.9.9-dev");
    assert_eq!(linked[1]["already_linked"], false);
    assert!(
        linked[1]["target"]
            .as_str()
            .expect("a target")
            .ends_with("OneFolder")
    );
    assert_eq!(value["skipped"], serde_json::json!([]));
    assert_eq!(value["without_toc"], serde_json::json!(["Media"]));

    let state = std::fs::read_to_string(game.state_path()).expect("the sidecar");
    let state: serde_json::Value = serde_json::from_str(&state).expect("JSON");
    assert_eq!(state["addons"]["OneFolder"]["source"], source);

    let output = game
        .mrgl()
        .args(["--json", "unlink"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "unlink");
    assert_eq!(value["source"], source);
    assert_eq!(
        value["unlinked"],
        serde_json::json!(["DevAddon", "OneFolder"])
    );
    assert_eq!(value["kept"], serde_json::json!([]));
}

#[cfg(windows)]
#[test]
fn link_skips_a_real_folder_with_a_warning_and_never_replaces_it() {
    let game = Game::new("link-skip");
    let dev = dev_copy(&game);
    std::fs::create_dir_all(game.addon("DevAddon")).expect("a real folder");
    std::fs::write(game.addon("DevAddon").join("mine.lua"), "-- mine\n").expect("a file");
    let output = game
        .mrgl()
        .args(["--serious", "link"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "skipped DevAddon: a folder of that name is in AddOns already and stays (not a junction)"),
        "{text}"
    );
    assert!(
        stdout_last_line(&output)
            .ends_with(" into _forever_: OneFolder, 1 skipped, 1 folder without a TOC left out"),
        "{text}"
    );
    assert_eq!(
        std::fs::read(game.addon("DevAddon").join("mine.lua")).expect("kept"),
        b"-- mine\n"
    );
}

#[cfg(not(windows))]
#[test]
fn link_and_unlink_say_junctions_are_windows_only() {
    for command in ["link", "unlink"] {
        let output = mrgl()
            .args([command, "SomeFolder", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            stderr_last_line(&output),
            "junctions are Windows-only: link and unlink work on Windows"
        );
    }
}

// --- task 015: `export` and `import` of the addon list, and the interface warning ---

/// A copy of the fixture answers with a zip behind the Forever asset of `Questie/Questie` v1.2.3 (the
/// fixture tree has none: its asset names are made up), so an import downloads and installs offline.
fn stub_with_forever_zip(base: &Path, zip: &str) -> PathBuf {
    let stub = base.join("stub");
    let _ = std::fs::remove_dir_all(&stub);
    copy_tree(&releases(), &stub);
    std::fs::copy(
        zips().join(zip),
        stub.join("github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip"),
    )
    .expect("the asset");
    stub
}

/// The sidecar entries without their install time, which differs by design between two installs.
fn entries_without_time(state: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(state).expect("the sidecar");
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("the sidecar is JSON");
    for entry in value["addons"]
        .as_object_mut()
        .expect("the addons")
        .values_mut()
    {
        let entry = entry.as_object_mut().expect("an entry");
        entry.remove("installed_at");
        // The `nolib` flag (task 029) comes with a pick: an import from GitHub records it, a zip in hand
        // does not, so it is not part of "the same entry".
        entry.remove("nolib");
    }
    value["addons"].clone()
}

#[test]
fn export_then_import_gives_an_empty_flavor_the_same_sidecar_entries() {
    // The fixture sidecar: a real install of two zips, then one entry turned into a `github:` one (the
    // source and version the stub's Questie/Questie release carries) and a linked entry added.
    let from = Game::new("export");
    for zip in ["several-folders-2.0.0.zip", "one-folder-1.0.0.zip"] {
        let output = from
            .mrgl()
            .arg("add")
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
    }
    let text = std::fs::read_to_string(from.state_path()).expect("the sidecar");
    let mut state: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    state["addons"]["SeveralFolders"]["source"] = "github:Questie/Questie".into();
    state["addons"]["SeveralFolders"]["version"] = "1.2.3".into();
    state["addons"]["DevAddon"] = serde_json::json!({
        "source": "link:D:\\dev\\DevAddon",
        "version": "dev",
        "folders": ["DevAddon"],
        "files": {},
        "installed_at": "2026-10-21T14:03:05Z",
    });
    std::fs::write(from.state_path(), state.to_string()).expect("the fixture sidecar");
    std::fs::create_dir_all(from.addon("DevAddon")).expect("the linked folder");
    std::fs::write(
        from.addon("DevAddon").join("DevAddon.toc"),
        "## Interface: 16001\n",
    )
    .expect("its TOC");

    let base = from.root.parent().expect("the base").to_path_buf();
    let list = base.join("lists").join("forever.json");
    let output = from
        .mrgl()
        .args(["--serious", "export", "--flavor", "forever", "--to"])
        .arg(&list)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    // The zip in hand and the linked folder are left out of the shareable list (task 028).
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "DevAddon is left out of the list: its source link:D:\\dev\\DevAddon is a linked development folder"),
        "{text}"
    );
    assert!(
        text.lines().any(
            |line| line.starts_with("OneFolder is left out of the list: its source ")
                && line.ends_with("one-folder-1.0.0.zip is a zip file")
        ),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "exported 1 managed addon of _forever_ to {}",
            list.display()
        )
    );
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&list).expect("the list")).expect("JSON");
    assert_eq!(written["format"], "mrgl-list");
    assert_eq!(written["schema"], 1);
    assert_eq!(written["flavor"], "wowf");
    let names: Vec<&str> = written["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| addon["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["SeveralFolders"]);
    assert_eq!(
        written["addons"][0],
        serde_json::json!({
            "name": "SeveralFolders",
            "source": "github:Questie/Questie",
            "version": "1.2.3",
            "folders": ["SeveralFolders", "SeveralFolders_Data", "SeveralFolders_Options"],
        })
    );

    // Without --to the list is the standard output and the message goes to the standard error.
    let output = from
        .mrgl()
        .args(["--serious", "export", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let printed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the list alone on stdout");
    assert_eq!(printed["addons"], written["addons"]);
    assert_eq!(
        stderr_last_line(&output),
        "exported 1 managed addon of _forever_"
    );

    let to = Game::new("import");
    let stub = stub_with_forever_zip(&base, "several-folders-2.0.0.zip");
    let output = to
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "dry run: reading 1 addon from forever.json into _forever_",
            "SeveralFolders  would install 1.2.3 from github:Questie/Questie",
            "dry run: 1 addon of forever.json for _forever_: 1 to install; nothing written",
        ],
        "{text}"
    );
    assert!(!to.state_path().exists(), "a dry run writes no sidecar");
    assert!(!to.addon("SeveralFolders").exists(), "nor an addon");

    let output = to
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "SeveralFolders  installed 1.2.3 from github:Questie/Questie"),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "imported 1 addon of forever.json into _forever_: 1 installed"
    );
    assert!(
        to.addon("SeveralFolders_Options")
            .join("core.lua")
            .is_file()
    );
    let expected = &entries_without_time(&from.state_path())["SeveralFolders"];
    let imported = entries_without_time(&to.state_path());
    assert_eq!(&imported["SeveralFolders"], expected, "the same entry");
    assert_eq!(
        imported.as_object().map(|addons| addons.len()),
        Some(1),
        "the zip in hand and the linked folder stay behind: {imported}"
    );

    // A second import leaves the installed addon alone, at any version.
    let output = to
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--json", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "import");
    assert_eq!(value["list_flavor"], "wowf");
    assert_eq!(value["dry_run"], false);
    let outcomes: Vec<&str> = value["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| addon["outcome"].as_str().expect("an outcome"))
        .collect();
    assert_eq!(outcomes, ["already-installed"]);
    assert_eq!(value["addons"][0]["version"], "1.2.3");
}

#[test]
fn import_failures_and_unreadable_lists_are_exit_code_one() {
    let game = Game::new("import-fail");
    let base = game.root.parent().expect("the base").to_path_buf();
    let list = base.join("list.json");
    std::fs::write(
        &list,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [
                { "name": "Empty", "source": "github:Example/Empty", "version": "1.0.0", "folders": ["Empty"] },
            ],
        })
        .to_string(),
    )
    .expect("the list");
    let output = game
        .mrgl()
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "Empty  failed: no release in github:Example/Empty"),
        "{text}"
    );
    assert_eq!(
        stderr_last_line(&output),
        "1 addon of list.json failed: Empty; the other entries are done"
    );

    std::fs::write(
        &list,
        "{\"format\": \"something-else\", \"schema\": 1, \"flavor\": \"wowf\"}",
    )
    .expect("another format");
    let output = game
        .mrgl()
        .arg("import")
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "not a readable addon list: {} (format something-else instead of mrgl-list)",
            list.display()
        )
    );
}

#[test]
fn the_interface_warning_fires_after_an_install_and_in_list() {
    let game = Game::new("interface");
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("stale-interface-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "StaleInterface: interface 110207 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor"
    );
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("one-folder-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "installed OneFolder 1.0.0 (1 folder: OneFolder) into _forever_",
        "16001 matches the Forever build: no warning"
    );
    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("stale-interface-1.0.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["interface_mismatch"]["folder"], "StaleInterface");
    assert_eq!(
        value["interface_mismatch"]["interfaces"],
        serde_json::json!([110207])
    );

    let output = game
        .mrgl()
        .args(["--serious", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            // The managed part (task 037) and the Forever mark (task 011) sit before the interface note.
            format!(
                "OneFolder  1.0.0  16001  managed  {}  forever:unknown",
                std::path::absolute(zips().join("one-folder-1.0.0.zip"))
                    .expect("the zip path")
                    .display()
            ),
            format!(
                "StaleInterface  1.0.0  110207  managed  {}  forever:unknown  interface mismatch",
                std::path::absolute(zips().join("stale-interface-1.0.0.zip"))
                    .expect("the zip path")
                    .display()
            ),
            "1 addon names no interface of the build 1.60.1.70245: StaleInterface (110207); made for another flavor or an older version of the game?".to_string(),
            "2 addons in _forever_ (wowf 1.60.1.70245): 2 managed by mrgl, 0 unmanaged".to_string(),
        ],
        "{text}"
    );
    let output = game
        .mrgl()
        .args(["--json", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        value["interface_mismatches"],
        serde_json::json!([{
            "folder": "StaleInterface",
            "toc": "StaleInterface.toc",
            "interfaces": [110207],
            "build": "1.60.1.70245",
            "major": 1,
        }])
    );
}

// --- task 021: `pin`, `unpin`, `ignore`, `unignore`, `channel` and their marks in `list` ---

/// Runs `mrgl --serious <args> --flavor forever` in a game and returns the output.
fn run_in(game: &Game, args: &[&str]) -> Output {
    game.mrgl()
        .arg("--serious")
        .args(args)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl")
}

/// An addon folder with a TOC, made by hand in a game.
fn addon_by_hand(game: &Game, folder: &str, version: &str) {
    std::fs::create_dir_all(game.addon(folder)).expect("the folder");
    std::fs::write(
        game.addon(folder).join(format!("{folder}.toc")),
        format!("## Interface: 16001\n## Title: {folder}\n## Version: {version}\n"),
    )
    .expect("the TOC");
}

#[test]
fn list_marks_a_pinned_an_ignored_and_a_prerelease_addon_of_a_fixture_sidecar() {
    let game = Game::new("marks-list");
    for (folder, version) in [
        ("Beta", "0.9.0-beta"),
        ("ByHand", "0.1"),
        ("Ignored", "2.0.0"),
        ("Pinned", "1.3.0"),
        ("Plain", "1.0.0"),
    ] {
        addon_by_hand(&game, folder, version);
    }
    // A sidecar as a user's could look: three marked entries and one written before task 021, without
    // the fields; ByHand has no entry.
    let entry = |version: &str, folder: &str, marks: &str| {
        format!(
            r#""{folder}": {{ "source": "github:example/{folder}", "version": "{version}", "folders": ["{folder}"], "files": {{}}, "installed_at": "2026-10-21T14:03:05Z"{marks} }}"#
        )
    };
    let sidecar = format!(
        r#"{{ "schema": 1, "flavor": "wowf", "addons": {{ {}, {}, {}, {} }} }}"#,
        entry("1.3.0", "Pinned", r#", "pinned": "1.2.0""#),
        entry("2.0.0", "Ignored", r#", "ignored": true"#),
        entry("0.9.0-beta", "Beta", r#", "channel": "prerelease""#),
        entry("1.0.0", "Plain", ""),
    );
    std::fs::create_dir_all(game.home.join("wowf")).expect("the flavor home");
    std::fs::write(game.state_path(), &sidecar).expect("the sidecar");

    let output = run_in(&game, &["list"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            // A managed addon of a Forever listing also carries the Forever mark of task 011.
            "Beta  0.9.0-beta  16001  managed  github:example/Beta  prerelease  forever:unknown",
            "ByHand  0.1  16001  unmanaged",
            "Ignored  2.0.0  16001  managed  github:example/Ignored  ignored  forever:unknown",
            "Pinned  1.3.0  16001  managed  github:example/Pinned  pinned at 1.2.0  forever:unknown",
            "Plain  1.0.0  16001  managed  github:example/Plain  forever:unknown",
            "5 addons in _forever_ (wowf 1.60.1.70245): 4 managed by mrgl, 1 unmanaged, 1 pinned, 1 ignored, 1 on the prerelease channel",
        ]
    );

    let output = game
        .mrgl()
        .args(["--json", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let marks: Vec<serde_json::Value> = value["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| {
            serde_json::json!([
                addon["folder"],
                addon["pinned"],
                addon["ignored"],
                addon["channel"]
            ])
        })
        .collect();
    assert_eq!(
        marks,
        [
            serde_json::json!(["Beta", null, false, "prerelease"]),
            serde_json::json!(["ByHand", null, false, null]),
            serde_json::json!(["Ignored", null, true, "stable"]),
            serde_json::json!(["Pinned", "1.2.0", false, "stable"]),
            serde_json::json!(["Plain", null, false, "stable"]),
        ]
    );
    assert_eq!(
        std::fs::read_to_string(game.state_path()).expect("the sidecar"),
        sidecar,
        "list writes nothing"
    );
}

#[test]
fn pin_unpin_ignore_unignore_and_channel_round_trip_through_the_sidecar() {
    let game = Game::new("marks-commands");
    for zip in ["one-folder-1.0.0.zip", "several-folders-2.0.0.zip"] {
        let output = game
            .mrgl()
            .arg("add")
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
    }
    let before = std::fs::read(game.state_path()).expect("the sidecar");

    let steps: [(&[&str], &str); 11] = [
        (
            &["pin", "OneFolder"],
            "pinned OneFolder at 1.0.0 in _forever_",
        ),
        (
            &["pin", "onefolder"],
            "OneFolder is already pinned at 1.0.0 in _forever_",
        ),
        (
            &["pin", "OneFolder", "0.9.0"],
            "pinned OneFolder at 0.9.0 in _forever_ (1.0.0 is installed)",
        ),
        (
            &["ignore", "SeveralFolders_Options"],
            "ignoring SeveralFolders in _forever_",
        ),
        (
            &["ignore", "SeveralFolders"],
            "SeveralFolders is already ignored in _forever_",
        ),
        (
            &["channel", "SeveralFolders", "prerelease"],
            "SeveralFolders follows the prerelease channel in _forever_",
        ),
        (&["unpin", "OneFolder"], "unpinned OneFolder in _forever_"),
        (
            &["unpin", "OneFolder"],
            "OneFolder is not pinned in _forever_",
        ),
        (
            &["unignore", "SeveralFolders"],
            "SeveralFolders is no longer ignored in _forever_",
        ),
        (
            &["unignore", "SeveralFolders"],
            "SeveralFolders is not ignored in _forever_",
        ),
        (
            &["channel", "SeveralFolders", "stable"],
            "SeveralFolders follows the stable channel in _forever_",
        ),
    ];
    for (args, plain) in steps {
        let output = run_in(&game, args);
        assert!(
            output.status.success(),
            "{args:?}: {}",
            stderr_last_line(&output)
        );
        assert_eq!(stdout(&output), format!("{plain}\n"), "{args:?}");
    }
    assert_eq!(
        std::fs::read(game.state_path()).expect("the sidecar"),
        before,
        "every mark undone leaves the sidecar as it was"
    );

    // The murloc speaks on a change; JSON is one plain document.
    let output = game
        .mrgl()
        .args(["ignore", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(stdout(&output).lines().count(), 2, "{}", stdout(&output));
    assert_eq!(stdout_last_line(&output), "ignoring OneFolder in _forever_");
    let output = game
        .mrgl()
        .args(["--json", "pin", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "pin");
    assert_eq!(value["flavor"]["id"], "wowf");
    assert_eq!(
        value["addon"],
        serde_json::json!({
            "name": "OneFolder",
            "version": "1.0.0",
            "pinned": "1.0.0",
            "ignored": true,
            "channel": "stable",
        })
    );
    assert_eq!(value["changed"], true);
    assert_eq!(value["warnings"], serde_json::json!([]));

    // A reinstall keeps the marks and says the pin differs from the new version.
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("one-folder-1.1.0.zip"))
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output).contains(
            "OneFolder is pinned at 1.0.0 and 1.1.0 is installed now; the pin stays until `mrgl unpin OneFolder`"
        ),
        "{}",
        stdout(&output)
    );
    let output = run_in(&game, &["list"]);
    assert!(
        stdout(&output).lines().any(
            |line| line.starts_with("OneFolder  1.1.0  16001  managed  ")
                && line.ends_with("  pinned at 1.0.0  ignored  forever:unknown")
        ),
        "{}",
        stdout(&output)
    );
    assert!(
        stdout_last_line(&output).ends_with(", 1 pinned, 1 ignored"),
        "{}",
        stdout_last_line(&output)
    );
}

#[test]
fn a_mark_on_an_unknown_or_unmanaged_addon_is_exit_code_one_and_writes_nothing() {
    let game = Game::new("marks-unmanaged");
    addon_by_hand(&game, "ByHand", "0.1");
    let unmanaged = "addon not managed by mrgl: ByHand (flavor forever); it was installed by hand or by another manager, and only an addon mrgl installed can be";
    let cases: [(&[&str], String); 6] = [
        (
            &["pin", "Nope"],
            "addon not found: Nope (flavor forever)".to_string(),
        ),
        (&["pin", "ByHand", "1.0"], format!("{unmanaged} pinned")),
        (&["unpin", "ByHand"], format!("{unmanaged} unpinned")),
        (&["ignore", "ByHand"], format!("{unmanaged} ignored")),
        (&["unignore", "ByHand"], format!("{unmanaged} unignored")),
        (
            &["channel", "ByHand", "prerelease"],
            format!("{unmanaged} put on a channel"),
        ),
    ];
    for (args, plain) in cases {
        let output = run_in(&game, args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(stderr_last_line(&output), plain, "{args:?}");
    }
    assert!(!game.state_path().exists(), "nothing was written");

    let output = run_in(&game, &["channel", "ByHand", "nightly"]);
    assert_eq!(output.status.code(), Some(2), "a usage error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown channel nightly: stable or prerelease"),
        "{stderr}"
    );
}

// --- task 022: `rollback` from the kept zip ---

/// Installs fixture zips in a game, in order, through the exe.
fn add_in(game: &Game, zips_in_order: &[&str]) {
    for zip in zips_in_order {
        let output = game
            .mrgl()
            .arg("add")
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
    }
}

#[test]
fn rollback_puts_the_previous_version_back_pins_it_and_goes_forward_again() {
    let game = Game::new("rollback");
    add_in(&game, &["one-folder-1.0.0.zip", "one-folder-1.1.0.zip"]);
    let updated_core = std::fs::read(game.addon("OneFolder").join("core.lua")).expect("core.lua");
    assert_eq!(updated_core, b"-- OneFolder 1.1.0\n");

    let output = run_in(&game, &["rollback", "onefolder"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output),
        "rolling back onefolder in _forever_ from the kept zip\n\
         rolled back OneFolder from 1.1.0 to 1.0.0 (1 folder: OneFolder) in _forever_; pinned at 1.0.0, `mrgl unpin OneFolder` lets updates reach it again\n"
    );
    assert_eq!(
        std::fs::read(game.addon("OneFolder").join("core.lua")).expect("core.lua"),
        b"-- OneFolder 1.0.0\n"
    );
    assert!(game.addon("OneFolder").join("old.lua").is_file());
    assert!(!game.addon("OneFolder").join("new.lua").exists());
    let output = run_in(&game, &["list"]);
    // The managed part (task 037) sits before the pin, the Forever mark (task 011) after it.
    assert!(
        stdout(&output).lines().any(
            |line| line.starts_with("OneFolder  1.0.0  16001  managed  ")
                && line.ends_with("  pinned at 1.0.0  forever:unknown")
        ),
        "{}",
        stdout(&output)
    );
    let cache = game.home.join("wowf").join("cache").join("OneFolder");
    assert!(cache.join("1.0.0.zip").is_file() && cache.join("1.1.0.zip").is_file());

    // The murloc speaks on the progress and the success; JSON is one plain document.
    let output = game
        .mrgl()
        .args(["--json", "rollback", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "rollback");
    assert_eq!(value["flavor"]["id"], "wowf");
    assert_eq!(value["addon"]["name"], "OneFolder");
    assert_eq!(value["addon"]["version"], "1.1.0", "forward again");
    assert_eq!(value["addon"]["previous_version"], "1.0.0");
    assert_eq!(value["addon"]["pinned"], "1.1.0");
    assert_eq!(value["addon"]["folders"], serde_json::json!(["OneFolder"]));
    assert_eq!(value["addon"]["removed_folders"], serde_json::json!([]));
    assert!(
        value["addon"]["source"]
            .as_str()
            .is_some_and(|source| source.ends_with("one-folder-1.1.0.zip"))
    );
    assert_eq!(value["warnings"], serde_json::json!([]));

    let output = game
        .mrgl()
        .args(["rollback", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(stdout(&output).lines().count(), 4, "{}", stdout(&output));
    assert!(stdout_last_line(&output).starts_with("rolled back OneFolder from 1.1.0 to 1.0.0"));
}

#[test]
fn rollback_errors_are_exit_code_one_and_change_nothing() {
    let game = Game::new("rollback-errors");
    addon_by_hand(&game, "ByHand", "0.1");
    let output = run_in(&game, &["rollback", "Nope"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "addon not found: Nope (flavor forever)"
    );
    let output = run_in(&game, &["rollback", "ByHand"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "addon not managed by mrgl: ByHand (flavor forever); it was installed by hand or by another manager, and only an addon mrgl installed can be rolled back"
    );
    assert!(!game.state_path().exists(), "nothing was written");

    add_in(&game, &["one-folder-1.0.0.zip"]);
    let before = std::fs::read(game.state_path()).expect("the sidecar");
    let output = run_in(&game, &["rollback", "OneFolder"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "nothing to roll back to: OneFolder 1.0.0 has no previous version (flavor forever); mrgl keeps the version an update replaces"
    );

    add_in(&game, &["one-folder-1.1.0.zip"]);
    let kept = game
        .home
        .join("wowf")
        .join("cache")
        .join("OneFolder")
        .join("1.0.0.zip");
    std::fs::remove_file(&kept).expect("the kept zip deleted by hand");
    let before_missing = std::fs::read(game.state_path()).expect("the sidecar");
    assert_ne!(before_missing, before);
    let output = game
        .mrgl()
        .args(["--json", "rollback", "OneFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("one JSON error document");
    assert_eq!(value["kind"], "error");
    assert_eq!(
        value["message"],
        format!(
            "cannot roll back OneFolder to 1.0.0: its zip is not in the cache ({}); install that version again with `mrgl add <zip>`",
            kept.display()
        )
    );
    assert!(output.stdout.is_empty(), "no progress line in JSON");
    assert_eq!(
        std::fs::read(game.state_path()).expect("the sidecar"),
        before_missing,
        "the sidecar is untouched"
    );
    assert_eq!(
        std::fs::read(game.addon("OneFolder").join("core.lua")).expect("core.lua"),
        b"-- OneFolder 1.1.0\n"
    );
}

// --- task 025: `savedvars` lists the SavedVariables by size and moves the orphans into a backup zip ---

/// The SavedVariables files the `savedvars` tests add to the fixture `WTF` tree, relative to `WTF`, with
/// their sizes: `GoneAddon` (a `.lua` and its `.bak`), `NoTocFolder` (its folder has no TOC) and a lone
/// `GoneToo.lua.bak` are the orphans; `Blizzard_Console` is the client's; `DisabledAddon` is disabled in
/// `AddOns.txt` and keeps its folder.
const SAVEDVARS_ADDED: [(&str, usize); 6] = [
    ("Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua", 300),
    ("Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua.bak", 200),
    ("Account/MRGLFIXTURE/SavedVariables/NoTocFolder.lua", 120),
    (
        "Account/MRGLFIXTURE/SavedVariables/Blizzard_Console.lua",
        40,
    ),
    (
        "Account/MRGLFIXTURE/Fixture Realm/Mrgltest/SavedVariables/DisabledAddon.lua",
        60,
    ),
    (
        "Account/MRGLFIXTURE/Fixture Realm/Mrgltest/SavedVariables/GoneToo.lua.bak",
        90,
    ),
];

/// The files `savedvars --clean` moves, relative to `WTF`.
const SAVEDVARS_ORPHANS: [&str; 4] = [
    "Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua",
    "Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua.bak",
    "Account/MRGLFIXTURE/SavedVariables/NoTocFolder.lua",
    "Account/MRGLFIXTURE/Fixture Realm/Mrgltest/SavedVariables/GoneToo.lua.bak",
];

/// A synthetic game folder with the fixture `WTF` tree, the files of [`SAVEDVARS_ADDED`] and an AddOns
/// folder with `MrglFixture` and `DisabledAddon` (with TOCs) and `NoTocFolder` (without one).
fn savedvars_game(name: &str) -> PathBuf {
    let root = temp_game_root_with_wtf(&format!("savedvars-{name}"));
    let wtf = root.join("_forever_").join("WTF");
    for (file, size) in SAVEDVARS_ADDED {
        std::fs::write(wtf.join(file), vec![b'x'; size]).expect("a SavedVariables file");
    }
    let addons = root.join("_forever_").join("Interface").join("AddOns");
    for folder in ["MrglFixture", "DisabledAddon"] {
        std::fs::create_dir_all(addons.join(folder)).expect("an addon folder");
        std::fs::write(
            addons.join(folder).join(format!("{folder}.toc")),
            format!("## Interface: 11508\n## Title: {folder}\n"),
        )
        .expect("a TOC");
    }
    std::fs::create_dir_all(addons.join("NoTocFolder")).expect("a folder without a TOC");
    root
}

/// The byte count as `savedvars` prints it.
fn savedvars_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    }
}

#[test]
fn savedvars_lists_the_sizes_and_marks_the_orphans() {
    let root = savedvars_game("list");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    let before = snapshot(&wtf);
    let total: u64 = before
        .iter()
        .filter(|(path, _)| {
            path.contains("/SavedVariables/") && (path.ends_with(".lua") || path.ends_with(".bak"))
        })
        .map(|(_, bytes)| bytes.len() as u64)
        .sum();

    let output = mrgl()
        .args(["--serious", "savedvars", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        7 + 1,
        "a line per entry and the summary: {text}"
    );
    assert_eq!(lines[0], "    500 B  GoneAddon  account-wide  orphan");
    assert!(
        lines.contains(&"    120 B  NoTocFolder  account-wide  orphan"),
        "{text}"
    );
    assert!(
        lines.contains(&"     90 B  GoneToo  Fixture Realm / Mrgltest  orphan"),
        "{text}"
    );
    assert!(
        lines.contains(&"     60 B  DisabledAddon  Fixture Realm / Mrgltest"),
        "a disabled addon is not an orphan: {text}"
    );
    assert!(
        lines.contains(&"     40 B  Blizzard_Console  account-wide  client"),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "7 SavedVariables files of _forever_, {} in all; 3 orphans (710 B) have no addon folder: `mrgl savedvars --clean` moves them into a backup zip",
            savedvars_bytes(total)
        )
    );

    // The murloc line comes with the summary when not serious.
    let output = mrgl()
        .args(["savedvars", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 7 + 2, "{text}");
    assert!(!lines[7].is_empty());

    let output = mrgl()
        .args(["--json", "savedvars", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "savedvars");
    assert_eq!(value["files"].as_array().map(Vec::len), Some(7));
    assert_eq!(value["files"][0]["addon"], "GoneAddon");
    assert_eq!(value["files"][0]["status"], "orphan");
    assert_eq!(value["files"][0]["lua_bytes"], 300);
    assert_eq!(value["files"][0]["bak_bytes"], 200);
    assert_eq!(value["orphans"], 3);
    assert_eq!(value["orphan_bytes"], 710);
    assert_eq!(value["bytes"], total);
    assert_eq!(snapshot(&wtf), before, "a listing writes nothing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn savedvars_clean_moves_the_orphans_into_a_zip_under_the_home() {
    let root = savedvars_game("clean");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    let home = root.join("home");
    let before = snapshot(&wtf);

    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "savedvars", "--clean", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{output:?}");
    let backups = home.join("backups").join("wowf");
    let zips: Vec<PathBuf> = std::fs::read_dir(&backups)
        .expect("the backups folder")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(zips.len(), 1, "{zips:?}");
    let zip = &zips[0];
    let name = zip
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    assert!(
        name.starts_with("SavedVariables-") && name.ends_with(".zip"),
        "{name}"
    );
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "moved 3 orphans of _forever_ (4 files, 710 B) into {}; unzip it into the WTF folder to put them back",
            zip.display()
        )
    );
    let text = stdout(&output);
    assert_eq!(
        text.lines().count(),
        3 + 1,
        "the moved and the summary: {text}"
    );
    let mut expected = before.clone();
    for orphan in SAVEDVARS_ORPHANS {
        assert!(expected.remove(orphan).is_some(), "{orphan}");
    }
    assert_eq!(snapshot(&wtf), expected, "moved, and nothing else touched");

    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--json", "savedvars", "--clean", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "savedvars-clean");
    assert_eq!(value["zip"], serde_json::Value::Null);
    assert_eq!(value["files"], 0);
    assert_eq!(value["moved"].as_array().map(Vec::len), Some(0));

    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "savedvars", "--clean", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success());
    assert_eq!(
        stdout(&output).trim_end(),
        "no orphans in the SavedVariables of _forever_, nothing moved"
    );

    // `backup --restore` is not taught to read the cleanup zip: it refuses it before anything moves.
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args([
            "--serious",
            "backup",
            "--yes",
            "--restore",
            &name,
            "--flavor",
        ])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "not a WTF backup: {} holds the SavedVariables a cleanup moved; unzip it into the WTF folder by hand to put them back",
            zip.display()
        )
    );
    assert_eq!(snapshot(&wtf), expected);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn savedvars_clean_is_refused_while_the_client_runs() {
    let root = savedvars_game("client");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    let home = root.join("home");
    let before = snapshot(&wtf);

    let Some(mut child) = spawn_stand_in(&flavor) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "savedvars", "--clean", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    let pid = child.id();
    child.stop();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {pid}): close the game first, nothing under WTF is written while it runs"
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing was touched");
    assert!(!home.join("backups").exists(), "no zip either");
    let _ = std::fs::remove_dir_all(&root);
}

// --- task 039: the bound on the unpacked size of a zip, and `--max-unpacked` ---

/// A fixture zip by its absolute path without `..`, as the messages name it.
fn fixture_zip(name: &str) -> PathBuf {
    std::path::absolute(zips().join(name)).expect("an absolute path")
}

#[test]
fn add_refuses_a_zip_past_the_bound_and_max_unpacked_raises_it() {
    let game = Game::new("max-unpacked");
    let zip = fixture_zip("declares-too-much-1.0.0.zip");
    let output = game
        .mrgl()
        .arg("add")
        .arg(&zip)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "refused {}: it declares 3.0 GB unpacked, more than the bound of 2.0 GB; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you trust",
            zip.display()
        )
    );
    assert!(!game.addon("DeclaresTooMuch").exists());
    assert!(staging_entries(&game).is_empty());
    assert!(!game.state_path().exists());

    // The dry run stages the zip, so the bound refuses it there too.
    let output = game
        .mrgl()
        .arg("add")
        .arg(&zip)
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_last_line(&output).contains("more than the bound of 2.0 GB"),
        "{}",
        stderr_last_line(&output)
    );

    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(&zip)
        .args(["--flavor", "forever", "--max-unpacked", "4G"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output)
            .lines()
            .any(|line| line.starts_with("installed DeclaresTooMuch 1.0.0 (1 folder")),
        "{}",
        stdout(&output)
    );
    assert!(game.addon("DeclaresTooMuch").join("core.lua").is_file());
}

#[test]
fn add_refuses_a_zip_that_lies_about_its_size_and_leaves_staging_empty() {
    let game = Game::new("lies-about-its-size");
    let zip = fixture_zip("lies-about-its-size-1.0.0.zip");
    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(&zip)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("one JSON error document");
    assert_eq!(value["kind"], "error");
    assert_eq!(
        value["message"],
        format!(
            "refused {}: the entry `LiesAboutItsSize/core.lua` unpacks to more than the 16 B it declares, so the bound of 2.0 GB cannot hold (109 B written, then removed)",
            zip.display()
        )
    );
    assert!(!game.addon("LiesAboutItsSize").exists());
    assert!(
        staging_entries(&game).is_empty(),
        "the staging folder is clean"
    );
    assert!(!game.state_path().exists());
}

#[test]
fn rollback_takes_max_unpacked_and_refuses_past_it_changing_nothing() {
    let game = Game::new("rollback-max-unpacked");
    add_in(&game, &["one-folder-1.0.0.zip", "one-folder-1.1.0.zip"]);
    let before = std::fs::read(game.state_path()).expect("the sidecar");
    let output = run_in(&game, &["rollback", "OneFolder", "--max-unpacked", "100"]);
    assert_eq!(output.status.code(), Some(1));
    let plain = stderr_last_line(&output);
    assert!(
        plain.starts_with("refused ") && plain.contains("more than the bound of 100 B"),
        "{plain}"
    );
    assert_eq!(
        std::fs::read(game.state_path()).expect("the sidecar"),
        before
    );
    assert_eq!(
        std::fs::read(game.addon("OneFolder").join("core.lua")).expect("core.lua"),
        b"-- OneFolder 1.1.0\n"
    );
    assert!(staging_entries(&game).is_empty());

    let output = run_in(&game, &["rollback", "OneFolder", "--max-unpacked", "1M"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout_last_line(&output).starts_with("rolled back OneFolder from 1.1.0 to 1.0.0"));
}

#[test]
fn restore_refuses_a_backup_past_the_bound_and_max_unpacked_raises_it() {
    let root = temp_game_root_with_wtf("restore-max-unpacked");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    let backups = root.join("backups");
    let before = snapshot(&wtf);
    let zip = fixture_zip("wtf-declares-too-much.zip");
    let restore = |extra: &[&str]| {
        mrgl()
            .args(["--serious", "backup", "--yes", "--restore"])
            .arg(&zip)
            .arg("--flavor")
            .arg(&flavor)
            .arg("--to")
            .arg(&backups)
            .args(extra)
            .output()
            .expect("run mrgl")
    };
    let output = restore(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "refused {}: it declares 3.0 GB unpacked, more than the bound of 2.0 GB; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you trust",
            zip.display()
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing moved");
    assert!(!backups.exists(), "nothing moved aside");

    let output = restore(&["--max-unpacked", "4G"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        std::fs::read(wtf.join("Config.wtf")).expect("Config.wtf"),
        b"SET portal \"EU\"\r\n"
    );

    // The lie fails the restore while writing, and the WTF folder the last restore wrote comes back.
    let restored = snapshot(&wtf);
    let liar = fixture_zip("wtf-lies-about-its-size.zip");
    let output = mrgl()
        .args(["--serious", "backup", "--yes", "--restore"])
        .arg(&liar)
        .arg("--flavor")
        .arg(&flavor)
        .arg("--to")
        .arg(&backups)
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "restoring {} failed and WTF is as it was: the entry `Config.wtf` unpacks to more than the 16 B it declares, so the bound of 2.0 GB cannot hold (0 B written, then removed)",
            liar.display()
        )
    );
    assert_eq!(snapshot(&wtf), restored);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn max_unpacked_is_a_usage_error_when_it_is_not_a_size_or_has_nothing_to_bound() {
    for args in [
        vec!["add", "x.zip", "--max-unpacked", "lots"],
        vec!["add", "x.zip", "--max-unpacked", "0"],
        vec!["rollback", "OneFolder", "--max-unpacked", "1.5G"],
        vec!["backup", "--max-unpacked", "3G"],
        vec!["backup", "--list", "--max-unpacked", "3G"],
    ] {
        let output = mrgl().args(&args).output().expect("run mrgl");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}

// --- task 051: the Windows version line of the doctor report, from the `windows=` stub item ---

#[test]
fn doctor_names_the_windows_version_in_the_block_and_the_json() {
    let output = mrgl()
        .env(
            "MRGL_DISCOVERY",
            "webview2=131.0.2903.70;windows=24H2,26100.1234",
        )
        .args(["--serious", "doctor"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines[1], "windows: Windows 11 24H2, build 26100.1234",
        "{stdout}"
    );

    let output = mrgl()
        .env("MRGL_DISCOVERY", "windows=,19045")
        .args(["--json", "doctor"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        value["windows"],
        serde_json::json!({
            "name": "Windows 10",
            "display_version": null,
            "build": 19045,
            "ubr": null,
            "product_name": null,
        }),
        "a part the registry lacks is null"
    );
}

// --- task 043: the game's Uninstall entry as a second registry source, and expanded values ---

#[test]
fn root_finds_the_game_root_in_the_uninstall_entry_alone() {
    let home = temp_home("uninstall-entry");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .env(
            "MRGL_DISCOVERY",
            format!("uninstall={}", fixtures().display()),
        )
        .args(["--serious", "root"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let line = stdout_last_line(&output);
    assert!(
        line.starts_with("game root found in the game's Uninstall registry entry: "),
        "{line}"
    );
    assert!(
        line.ends_with("; run `mrgl root <folder>` to remember it"),
        "{line}"
    );
    assert!(!home.join("config.json").exists(), "nothing saved");

    let json = mrgl()
        .env(
            "MRGL_DISCOVERY",
            format!("uninstall={}", fixtures().display()),
        )
        .args(["--json", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(json.status.success(), "{}", stderr_last_line(&json));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["root_origin"], "uninstall_entry");
    assert_eq!(value["flavor"]["id"], "wowf");
}

#[test]
fn the_launchers_value_wins_over_the_uninstall_entry() {
    let other = doctor_game_root("uninstall-other");
    let output = mrgl()
        .env(
            "MRGL_DISCOVERY",
            format!(
                "registry={};uninstall={}",
                fixtures().display(),
                other.display()
            ),
        )
        .args(["--json", "root"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["root_origin"], "registry");

    // A launcher's value that is no game root falls through to the Uninstall entry, which the doctor
    // names too.
    let output = mrgl()
        .env(
            "MRGL_DISCOVERY",
            format!("registry=Q:\\nowhere;uninstall={}", other.display()),
        )
        .args(["--serious", "doctor"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.lines().any(|line| line.starts_with("root: ")
            && line.ends_with("(found in the game's Uninstall registry entry)")),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn an_expandable_registry_value_resolves_to_the_root_of_its_literal() {
    let parent = fixtures()
        .parent()
        .expect("the fixtures' parent")
        .to_path_buf();
    let root_of = |stub: String| {
        let output = mrgl()
            .env("MRGL_TEST_GAMES_043", &parent)
            .env("MRGL_DISCOVERY", stub)
            .args(["--json", "root"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("one JSON document");
        (value["root"].clone(), value["root_origin"].clone())
    };
    let (literal, origin) = root_of(format!("registry={}", fixtures().display()));
    assert_eq!(origin, "registry");
    assert!(literal.is_string(), "{literal}");
    // The stub's expandable items go through the same environment call as a `REG_EXPAND_SZ` value.
    assert_eq!(
        root_of("registry-expand=%MRGL_TEST_GAMES_043%\\flavors".to_string()),
        (literal.clone(), "registry".into())
    );
    assert_eq!(
        root_of("uninstall-expand=%MRGL_TEST_GAMES_043%\\flavors".to_string()),
        (literal, "uninstall_entry".into())
    );
}

// --- task 046: the asset download's 404 names the asset, a short download is refused ---

#[test]
fn add_refuses_an_asset_shorter_than_its_listed_size() {
    let game = Game::new("short-asset");
    let stub = game.root.parent().expect("the base folder").join("stub");
    let list = stub.join("api.github.com/repos/Example/Short/releases.json");
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(
        &list,
        r#"[{"tag_name":"v1.0.0","assets":[{"name":"Short-v1.0.0-forever.zip","size":2048,"browser_download_url":"https://github.com/Example/Short/releases/download/v1.0.0/Short-v1.0.0-forever.zip"}]}]"#,
    )
    .unwrap();
    let asset =
        stub.join("github.com/Example/Short/releases/download/v1.0.0/Short-v1.0.0-forever.zip");
    std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
    std::fs::write(&asset, b"PK cut short").unwrap();
    let run = |json: bool| {
        let mut command = game.mrgl();
        command.env("MRGL_HTTP_STUB", &stub);
        if json {
            command.arg("--json");
        } else {
            command.arg("--serious");
        }
        command
            .args(["add", "github:Example/Short", "--flavor", "forever"])
            .output()
            .expect("run mrgl")
    };
    let plain = "refused Short-v1.0.0-forever.zip of github:Example/Short v1.0.0: 12 bytes downloaded, the release list names 2048; the download was cut short or the asset changed, try again";
    let output = run(false);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr_last_line(&output), plain);
    let text = stdout(&output);
    assert!(
        text.contains("downloading Short-v1.0.0-forever.zip"),
        "{text}"
    );
    assert!(!text.contains("installing"), "{text}");
    assert_eq!(staging_entries(&game), Vec::<String>::new());
    assert!(!game.state_path().exists(), "nothing installed");
    assert!(!game.addon("Short").exists());

    let output = run(true);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("one JSON error document");
    assert_eq!(value["kind"], "error");
    assert_eq!(value["message"], plain);
}

// --- task 041: the offline resolve from the cached release list, with a warning ---

/// The plain line of a network error on the Questie release list, the stub folder missing.
const QUESTIE_OFFLINE: &str = "cannot reach https://api.github.com/repos/Questie/Questie/releases?per_page=30: offline, the stub folder is missing";

/// Whether a line is the stale warning of the Questie pick.
fn is_stale_warning(line: &str) -> bool {
    line.starts_with("offline: github:Questie/Questie v1.2.3 is from the release list cached ")
        && line.ends_with(&format!(", the pick may be stale ({QUESTIE_OFFLINE})"))
}

#[test]
fn add_dry_run_offline_answers_from_a_warm_cache_with_a_warning() {
    let home = temp_home("offline-dry-run");
    let gone = home.join("no-such-stub");
    let run = |stub: &Path, mode: &str| {
        mrgl()
            .env("MRGL_HOME", &home)
            .env("MRGL_HTTP_STUB", stub)
            .args([
                mode,
                "add",
                "github:Questie/Questie",
                "--flavor",
                "forever",
                "--dry-run",
            ])
            .output()
            .expect("run mrgl")
    };
    // A cold cache: the network error stands.
    let cold = run(&gone, "--serious");
    assert_eq!(cold.status.code(), Some(1));
    assert_eq!(stderr_last_line(&cold), QUESTIE_OFFLINE);

    // A working network warms the cache and warns about nothing.
    let online = run(&releases(), "--serious");
    assert!(online.status.success(), "{}", stderr_last_line(&online));
    assert!(!stdout(&online).contains("offline"), "{}", stdout(&online));

    let offline = run(&gone, "--serious");
    assert!(offline.status.success(), "{}", stderr_last_line(&offline));
    let text = stdout(&offline);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "resolving github:Questie/Questie for forever");
    assert!(is_stale_warning(lines[1]), "{text}");
    assert_eq!(
        lines[2],
        "pick for forever: github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)"
    );
    assert_eq!(lines.last(), Some(&"dry run: nothing installed"));

    let json = run(&gone, "--json");
    assert!(json.status.success(), "{}", stderr_last_line(&json));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["pick"]["release"], "v1.2.3");
    assert_eq!(value["pick"]["stale"]["reason"], QUESTIE_OFFLINE);
    assert!(value["pick"]["stale"]["checked_at"].is_string(), "{value}");
    let warnings = value["warnings"].as_array().expect("the warnings");
    assert_eq!(warnings.len(), 1, "{value}");
    assert!(
        is_stale_warning(warnings[0].as_str().unwrap_or_default()),
        "{value}"
    );

    let fresh = run(&releases(), "--json");
    let value: serde_json::Value =
        serde_json::from_slice(&fresh.stdout).expect("one JSON document");
    assert_eq!(value["pick"]["stale"], serde_json::Value::Null);
    assert_eq!(value["warnings"], serde_json::json!([]));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn add_offline_finds_the_installed_release_from_the_cache_with_a_warning() {
    let game = Game::new("offline-add");
    let gone = game
        .root
        .parent()
        .expect("the base folder")
        .join("no-such-stub");
    let online = game
        .mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(online.status.success(), "{}", stderr_last_line(&online));

    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &gone)
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(text.lines().any(is_stale_warning), "{text}");
    assert_eq!(
        stdout_last_line(&output),
        "Questie 1.2.3 is installed already in _forever_ from github:Questie/Questie, nothing to do"
    );

    let json = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &gone)
        .args([
            "--json",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(json.status.success(), "{}", stderr_last_line(&json));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["already_installed"], true);
    assert_eq!(value["pick"]["stale"]["reason"], QUESTIE_OFFLINE);
    assert!(
        value["warnings"]
            .as_array()
            .expect("the warnings")
            .iter()
            .any(|warning| is_stale_warning(warning.as_str().unwrap_or_default())),
        "{value}"
    );
}

// --- task 006: the update check and `update --all` / `update <name>`, against the folder stub ---

/// A game with a fixture sidecar of ten managed addons, one for every state of the update check, each
/// with its folder made by hand, and an unmanaged folder. The fixture answers serve the Forever zip of
/// `Questie/Questie` v1.2.3, the one update.
fn update_game(name: &str) -> Game {
    let game = Game::new(name);
    let entries = [
        (
            "Beta",
            "github:Example/Prerelease",
            "1.1.0-beta",
            r#", "channel": "prerelease""#,
        ),
        ("Dev", r"link:D:\\dev\\Dev", "dev", ""),
        ("Empty", "github:Example/Empty", "1.0.0", ""),
        ("Gone", "github:Example/Missing", "1.0.0", ""),
        ("Guess", "github:Example/NoReleaseJson", "2.0.0", ""),
        (
            "Ignored",
            "github:Example/NoMetadata",
            "1.0.0",
            r#", "ignored": true"#,
        ),
        (
            "Pinned",
            "github:Example/Notes",
            "2.0.0",
            r#", "pinned": "2.0.0""#,
        ),
        ("Plain", "github:Example/Prerelease", "1.0.0", ""),
        ("Questie", "github:Questie/Questie", "1.2.2", ""),
        ("Zipped", r"C:\\Downloads\\Zipped-1.0.0.zip", "1.0.0", ""),
    ];
    let mut json = Vec::new();
    for (folder, source, version, marks) in entries {
        addon_by_hand(&game, folder, version);
        json.push(format!(
            r#""{folder}": {{ "source": "{source}", "version": "{version}", "folders": ["{folder}"], "files": {{}}, "installed_at": "2026-10-21T14:03:05Z"{marks} }}"#
        ));
    }
    addon_by_hand(&game, "ByHand", "0.1");
    std::fs::create_dir_all(game.home.join("wowf")).expect("the flavor home");
    std::fs::write(
        game.state_path(),
        format!(
            r#"{{ "schema": 1, "flavor": "wowf", "addons": {{ {} }} }}"#,
            json.join(", ")
        ),
    )
    .expect("the sidecar");
    game
}

/// Every file under the flavor folder with its bytes, and the sidecar's bytes.
fn game_snapshot(game: &Game) -> (std::collections::BTreeMap<String, Vec<u8>>, Vec<u8>) {
    (
        snapshot(&game.root.join("_forever_")),
        std::fs::read(game.state_path()).expect("the sidecar"),
    )
}

#[test]
fn update_dry_run_reports_every_state_of_a_fixture_sidecar_and_writes_nothing() {
    let game = update_game("update-dry");
    let before = game_snapshot(&game);
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let mut lines: Vec<&str> = text.lines().collect();
    // The 404 line names the release list URL, whose query belongs to the GitHub source.
    let gone = lines.remove(4);
    assert!(
        gone.starts_with(
            "Gone  1.0.0  source gone: GitHub answered 404 for https://api.github.com/repos/Example/Missing/releases"
        ) && gone.ends_with(": no such repository, or it is private"),
        "{gone}"
    );
    assert_eq!(
        lines,
        [
            "checking the managed addons of _forever_ against their sources",
            "Beta  1.1.0-beta  up to date",
            "Dev  dev  not checked: a linked development folder",
            "Empty  1.0.0  source gone: no release in github:Example/Empty",
            "Guess  2.0.0 -> 3.0.0  guess only: no release.json names forever; the asset name ends in -forever, a guess; `mrgl add github:Example/NoReleaseJson` installs it anyway",
            "Ignored  1.0.0  ignored, not checked",
            "Pinned  2.0.0  pinned at 2.0.0, 2.1.0 is out, left alone",
            "Plain  1.0.0  up to date",
            "Questie  1.2.2 -> 1.2.3  update available",
            "Zipped  1.0.0  not checked: a zip in hand; a newer zip goes in with mrgl add <zip>",
            "dry run: 1 update available, 2 up to date, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked; nothing written",
        ],
        "{text}"
    );
    assert_eq!(game_snapshot(&game), before, "a dry run writes nothing");
    assert!(!game.home.join("wowf").join("staging").exists());
}

#[test]
fn update_all_with_yes_updates_through_the_one_path_and_counts_the_rest() {
    let game = update_game("update-all");
    let before = sidecar(&game);
    let output = run_in(&game, &["update", "--all", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().skip(11).collect();
    assert_eq!(
        lines,
        [
            "updating Questie from 1.2.2 to 1.2.3",
            "downloading Questie-v1.2.3-forever.zip",
            "installing Questie-v1.2.3-forever.zip into _forever_",
            "updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in _forever_",
            "1 addon updated, 2 up to date, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked",
        ],
        "{text}"
    );
    assert!(
        std::fs::read_to_string(game.addon("Questie").join("Questie.toc"))
            .expect("the TOC")
            .contains("## Version: v1.2.3"),
        "the release's folder replaced the one made by hand"
    );
    let after = sidecar(&game);
    assert_eq!(after["addons"]["Questie"]["version"], "1.2.3");
    assert_eq!(after["addons"]["Questie"]["previous_version"], "1.2.2");
    for name in ["Beta", "Pinned", "Ignored", "Guess", "Empty", "Zipped"] {
        assert_eq!(after["addons"][name], before["addons"][name], "{name}");
    }
    assert_eq!(staging_entries(&game), Vec::<String>::new());

    // A second run finds nothing to update.
    let output = run_in(&game, &["update", "--all", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "nothing updated: 3 up to date, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked"
    );
}

#[test]
fn update_without_yes_and_without_a_terminal_says_what_yes_does() {
    let game = update_game("update-no-yes");
    let before = game_snapshot(&game);
    let output = run_in(&game, &["update", "--all"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "1 addon to update, nothing updated: no terminal to answer the confirmation; --yes updates without asking"
    );
    let output = game
        .mrgl()
        .args(["--json", "update", "--all", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value =
        serde_json::from_str(&stderr_last_line(&output)).expect("a JSON error");
    assert_eq!(
        error["message"],
        "1 addon to update, nothing updated: --json needs --yes, nobody is there to answer the confirmation"
    );
    assert_eq!(game_snapshot(&game), before, "nothing written");
}

#[test]
fn update_json_is_one_document_with_every_state_and_count() {
    let game = update_game("update-json");
    let output = game
        .mrgl()
        .args(["--json", "update", "--all", "--yes", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "update");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["updated"], 1);
    assert_eq!(
        value["counts"],
        serde_json::json!({
            "update-available": 1, "up-to-date": 2, "pinned": 1, "ignored": 1, "guess-only": 1,
            "source-gone": 2, "not-checked": 2, "failed": 0,
        })
    );
    let states: Vec<(&str, &str)> = value["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| {
            (
                addon["name"].as_str().expect("a name"),
                addon["state"].as_str().expect("a state"),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            ("Beta", "up-to-date"),
            ("Dev", "not-checked"),
            ("Empty", "source-gone"),
            ("Gone", "source-gone"),
            ("Guess", "guess-only"),
            ("Ignored", "ignored"),
            ("Pinned", "pinned"),
            ("Plain", "up-to-date"),
            ("Questie", "update-available"),
            ("Zipped", "not-checked"),
        ]
    );
    let questie = &value["addons"][8];
    assert_eq!(questie["latest"], "1.2.3");
    assert_eq!(questie["updated"], true);
    assert_eq!(questie["addon"]["version"], "1.2.3");
    assert_eq!(questie["addon"]["previous_version"], "1.2.2");
    assert_eq!(questie["pick"]["asset"], "Questie-v1.2.3-forever.zip");
    assert_eq!(value["addons"][1]["updated"], false);
    assert_eq!(value["addons"][6]["pinned"], "2.0.0");
    assert_eq!(value["addons"][6]["latest"], "2.1.0");
    assert_eq!(value["addons"][0]["channel"], "prerelease");
}

#[test]
fn update_one_addon_takes_the_same_path_and_names_an_unknown_one() {
    let game = update_game("update-one");
    let output = run_in(&game, &["update", "pinned", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "checking pinned of _forever_ against their sources",
            "Pinned  2.0.0  pinned at 2.0.0, 2.1.0 is out, left alone",
            "nothing updated: 1 pinned",
        ]
    );
    let output = run_in(&game, &["update", "Questie", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(stdout_last_line(&output), "1 addon updated");
    assert_eq!(sidecar(&game)["addons"]["Questie"]["version"], "1.2.3");

    for (name, plain) in [
        ("Nope", "addon not found: Nope (flavor forever)"),
        (
            "ByHand",
            "addon not managed by mrgl: ByHand (flavor forever); it was installed by hand or by another manager, and only an addon mrgl installed can be updated",
        ),
    ] {
        let output = run_in(&game, &["update", name]);
        assert_eq!(output.status.code(), Some(1), "{name}");
        assert_eq!(stderr_last_line(&output), plain);
    }
    // Neither a name nor --all, or both: a usage error.
    for args in [
        vec!["update", "--flavor", "forever"],
        vec!["update", "Questie", "--all", "--flavor", "forever"],
    ] {
        let output = game.mrgl().args(&args).output().expect("run mrgl");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}

// --- task 057: `import --exact` installs the version the list names ---

/// A copy of the fixture answers with the fixture `Questie` zip behind the Forever asset of the older
/// `Questie/Questie` release, v1.2.2 (no `release.json`: a pick by its name).
fn stub_with_older_questie(base: &Path) -> PathBuf {
    let stub = base.join("stub-older");
    let _ = std::fs::remove_dir_all(&stub);
    copy_tree(&releases(), &stub);
    let older = stub.join("github.com/Questie/Questie/releases/download/v1.2.2");
    std::fs::create_dir_all(&older).expect("the release folder");
    std::fs::copy(
        releases()
            .join("github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip"),
        older.join("Questie-v1.2.2-forever.zip"),
    )
    .expect("the asset");
    // The release list names the asset's size, which the download is checked against (task 046): the
    // copy names the size of the zip now behind it.
    let list_path = stub.join("api.github.com/repos/Questie/Questie/releases.json");
    let mut list: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&list_path).expect("the release list"))
            .expect("JSON");
    let size = std::fs::metadata(older.join("Questie-v1.2.2-forever.zip"))
        .expect("the asset")
        .len();
    for release in list.as_array_mut().expect("the releases") {
        for asset in release["assets"].as_array_mut().expect("the assets") {
            if asset["name"] == "Questie-v1.2.2-forever.zip" {
                asset["size"] = size.into();
            }
        }
    }
    std::fs::write(&list_path, list.to_string()).expect("the release list");
    stub
}

/// A list file of entries `(name, source, version)`, written under the game's base folder.
fn write_list(game: &Game, file: &str, entries: &[(&str, &str, &str)]) -> PathBuf {
    let path = game.root.parent().expect("the base").join(file);
    let addons: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, source, version)| {
            serde_json::json!({ "name": name, "source": source, "version": version, "folders": [name] })
        })
        .collect();
    std::fs::write(
        &path,
        serde_json::json!({ "format": "mrgl-list", "schema": 1, "flavor": "wowf", "addons": addons })
            .to_string(),
    )
    .expect("the list");
    path
}

#[test]
fn import_exact_installs_the_listed_version_and_reports_one_the_source_lacks() {
    let game = Game::new("import-exact");
    let stub = stub_with_older_questie(game.root.parent().expect("the base"));
    let list = write_list(
        &game,
        "exact.json",
        &[
            ("Questie", "github:Questie/Questie", "1.2.2"),
            ("Notes", "github:Example/Notes", "0.5.0"),
        ],
    );
    let run = |args: &[&str]| {
        game.mrgl()
            .env("MRGL_HTTP_STUB", &stub)
            .args(["--serious", "import"])
            .arg(&list)
            .args(["--flavor", "forever", "--exact"])
            .args(args)
            .output()
            .expect("run mrgl")
    };

    let output = run(&["--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "dry run: reading 2 addons from exact.json into _forever_ at the versions it names",
            "Questie  would install 1.2.2 from github:Questie/Questie",
            "Notes  skipped, version 0.5.0 not offered: no release 0.5.0 in github:Example/Notes",
            "dry run: 2 addons of exact.json for _forever_: 1 to install, 1 version not offered; nothing written",
        ]
    );
    assert!(!game.state_path().exists(), "a dry run writes nothing");

    let output = run(&[]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "Questie  installed 1.2.2 from github:Questie/Questie"),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "imported 2 addons of exact.json into _forever_: 1 installed, 1 version not offered"
    );
    let state = sidecar(&game);
    assert_eq!(state["addons"]["Questie"]["version"], "1.2.2");
    assert_eq!(
        state["addons"]["Questie"]["source"],
        "github:Questie/Questie"
    );
    assert!(state["addons"].get("Notes").is_none(), "not installed");
    assert!(game.addon("Questie").join("Questie.toc").is_file());
    assert!(!game.addon("Notes").exists());

    // The JSON names the option and the outcome.
    std::fs::remove_dir_all(&game.home).expect("a fresh home");
    std::fs::remove_dir_all(game.addon("Questie")).expect("a fresh folder");
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--json", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--exact"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["exact"], true);
    assert_eq!(value["addons"][0]["outcome"], "installed");
    assert_eq!(value["addons"][0]["version"], "1.2.2");
    assert_eq!(value["addons"][1]["outcome"], "version-not-offered");
    assert_eq!(value["addons"][1]["listed_version"], "0.5.0");
}

#[test]
fn import_without_exact_still_takes_the_current_release() {
    let game = Game::new("import-current");
    let stub = stub_with_older_questie(game.root.parent().expect("the base"));
    let list = write_list(
        &game,
        "current.json",
        &[("Questie", "github:Questie/Questie", "1.2.2")],
    );
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output).lines().any(|line| line
            == "Questie  installed 1.2.3 from github:Questie/Questie, the list had 1.2.2"),
        "{}",
        stdout(&output)
    );
    assert_eq!(sidecar(&game)["addons"]["Questie"]["version"], "1.2.3");
}

// --- tasks 006 and 057 over task 041: a pick from the cached release list is a warning on update and import ---

#[test]
fn update_and_import_warn_about_a_pick_from_the_cached_release_list() {
    let game = update_game("update-stale");
    let offline = game.root.parent().expect("the base").join("no-such-stub");
    let stale = |text: &str| {
        text.lines().any(|line| {
            line.starts_with(
                "offline: github:Questie/Questie v1.2.3 is from the release list cached ",
            )
        })
    };
    // Online first, so the release lists are cached; then every request is a network error.
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(!stale(&stdout(&output)), "{}", stdout(&output));
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &offline)
        .args([
            "--serious",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    let text = stdout(&output);
    assert!(stale(&text), "{text}");
    assert!(
        text.lines()
            .any(|line| line == "Questie  1.2.2 -> 1.2.3  update available"),
        "{text}"
    );

    let list = write_list(
        &game,
        "stale.json",
        &[("Fresh", "github:Questie/Questie", "1.2.3")],
    );
    // Another flavor folder and home with the same warm release-list cache.
    let other = Game::new("import-stale");
    copy_tree(&game.home.join("cache"), &other.home.join("cache"));
    let output = other
        .mrgl()
        .env("MRGL_HTTP_STUB", &offline)
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    let text = stdout(&output);
    assert!(stale(&text), "{text}");
    assert!(
        text.lines()
            .any(|line| line == "Fresh  would install 1.2.3 from github:Questie/Questie"),
        "{text}"
    );
}

// --- task 028: the shareable list's format check and the Forever starter list, against the folder stub ---

/// The Forever starter list in the repo.
fn starter_list() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lists/forever-starter.json")
}

/// The stub answers of the starter list's repositories: one release each, in the shape the repository's
/// latest release had when the list was checked (synthetic sizes, no zips: a dry run downloads none).
fn starter_stub() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/starter-list")
}

/// The starter list's entries as JSON.
fn starter_entries() -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(starter_list()).expect("lists/forever-starter.json");
    let list: serde_json::Value = serde_json::from_str(&text).expect("the list is JSON");
    list["addons"].as_array().expect("the addons").clone()
}

#[test]
fn the_starter_list_and_its_stub_name_the_same_repositories() {
    let mut listed: Vec<String> = starter_entries()
        .iter()
        .map(|entry| {
            let source = entry["source"].as_str().expect("a source");
            source
                .strip_prefix("github:")
                .unwrap_or_else(|| panic!("a GitHub entry only: {source}"))
                .to_string()
        })
        .collect();
    listed.sort();
    let mut stubbed = Vec::new();
    for owner in std::fs::read_dir(starter_stub().join("api.github.com/repos")).expect("the owners")
    {
        let owner = owner.expect("an owner");
        for repo in std::fs::read_dir(owner.path()).expect("the repositories") {
            let repo = repo.expect("a repository");
            stubbed.push(format!(
                "{}/{}",
                owner.file_name().to_string_lossy(),
                repo.file_name().to_string_lossy()
            ));
        }
    }
    stubbed.sort();
    assert_eq!(listed, stubbed, "a new entry brings its stub repository");
}

#[test]
fn the_forever_starter_list_installs_into_an_empty_flavor_with_dry_run() {
    let game = Game::new("starter");
    let entries = starter_entries();
    let count = entries.len();
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", starter_stub())
        .args(["--serious", "import"])
        .arg(starter_list())
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let mut expected = vec![format!(
        "dry run: reading {count} addons from forever-starter.json into _forever_"
    )];
    for entry in &entries {
        expected.push(format!(
            "{}  would install {} from {}",
            entry["name"].as_str().expect("a name"),
            entry["version"].as_str().expect("a version"),
            entry["source"].as_str().expect("a source"),
        ));
    }
    expected.push(format!(
        "dry run: {count} addons of forever-starter.json for _forever_: {count} to install; nothing written"
    ));
    assert_eq!(text.lines().collect::<Vec<_>>(), expected, "{text}");
    assert!(!game.state_path().exists(), "a dry run writes no sidecar");
    assert!(
        !game.root.join("_forever_").join("Interface").exists(),
        "nor an addon"
    );

    // Every pick is the zip the release's release.json names for Forever.
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", starter_stub())
        .args(["--json", "import"])
        .arg(starter_list())
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let addons = value["addons"].as_array().expect("the addons");
    assert_eq!(addons.len(), count);
    for addon in addons {
        assert_eq!(addon["outcome"], "would-install", "{addon}");
        let detail = addon["detail"].as_str().expect("the pick");
        assert!(
            detail.ends_with(
                ", flavor forever, interface 16001): release.json names forever (interface 16001)"
            ),
            "{detail}"
        );
    }
    assert_eq!(value["warnings"], serde_json::json!([]));
}

#[test]
fn import_refuses_a_list_with_a_local_path_and_names_the_entry() {
    let game = Game::new("not-shareable");
    let base = game.root.parent().expect("the base").to_path_buf();
    let list = base.join("shared.json");
    std::fs::write(
        &list,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [
                { "name": "Questie", "source": "github:Questie/Questie", "version": "1.2.3", "folders": ["Questie"] },
                { "name": "Zipped", "source": "C:\\Downloads\\Zipped-1.0.0.zip", "version": "1.0.0", "folders": ["Zipped"] },
            ],
        })
        .to_string(),
    )
    .expect("the list");
    let expected = format!(
        "not a shareable addon list: {} (Zipped: its source C:\\Downloads\\Zipped-1.0.0.zip is a zip file); a list names sources and versions, never a file or a folder of one machine",
        list.display()
    );
    for dry_run in [true, false] {
        let mut command = game.mrgl();
        command
            .arg("import")
            .arg(&list)
            .args(["--flavor", "forever"]);
        if dry_run {
            command.arg("--dry-run");
        }
        let output = command.output().expect("run mrgl");
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stderr_last_line(&output), expected);
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).lines().count(),
            2,
            "a murloc line and the plain line"
        );
        assert!(output.stdout.is_empty(), "refused before anything is read");
        assert!(!game.state_path().exists(), "no sidecar");
        assert!(!game.addon("Questie").exists(), "no addon");
    }
}

// Task 014: the addon sets (`mrgl set`) over the synthetic `addon-sets` WTF tree of `mrgl-core`, two
// characters on two realms, copied into a temporary flavor folder; the sets file lives under a temporary
// mrgl home.

/// The synthetic `WTF` tree with two characters of `mrgl-core`.
fn sets_fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/addon-sets/WTF")
}

/// A game folder with the `addon-sets` tree as its `_forever_` WTF, and a fresh home next to it.
fn temp_game_root_with_sets(name: &str) -> (PathBuf, PathBuf) {
    let root = temp_game_root(&format!("sets-{name}"));
    copy_tree(&sets_fixture_wtf(), &root.join("_forever_").join("WTF"));
    (root.clone(), root.join("home"))
}

/// `mrgl <args> --flavor <the flavor folder>` with the test's home.
fn set_run(root: &Path, home: &Path, args: &[&str]) -> Output {
    mrgl()
        .env("MRGL_HOME", home)
        .args(args)
        .arg("--flavor")
        .arg(root.join("_forever_"))
        .output()
        .expect("run mrgl")
}

const SETS_FIRST: &str = "Account/MRGLSETS/Fixture Realm/Mrgltest/AddOns.txt";
const SETS_SECOND: &str = "Account/MRGLSETS/Second Realm/Grrlwave/AddOns.txt";

#[test]
fn set_save_list_show_apply_and_remove_speak_plain_lines() {
    let (root, home) = temp_game_root_with_sets("cycle");
    let wtf = root.join("_forever_").join("WTF");
    let before = snapshot(&wtf);

    let output = set_run(&root, &home, &["--serious", "set", "list"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_last_line(&output),
        "no sets of _forever_ yet: `mrgl set save <name>` saves one"
    );

    let output = set_run(&root, &home, &["--serious", "set", "save", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "left out of set everyday, the characters disagree on them: BossTimers, QuestArrows",
            "saved set everyday of _forever_ from 2 characters: 2 addons (2 enabled, 0 disabled)",
        ]
    );
    let sets_path = home.join("wowf").join("sets.json");
    assert!(
        sets_path.is_file(),
        "sets.json under the flavor's home folder"
    );
    assert_eq!(snapshot(&wtf), before, "a save only reads WTF");

    // A set with a disabled addon, as a user would keep one, added to the file `save` wrote.
    let mut file: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&sets_path).expect("sets.json")).expect("JSON");
    file["sets"]["raid"] = serde_json::json!({
        "saved_at": "2026-10-22T00:00:00Z",
        "addons": { "BagSorter": "disabled", "BossTimers": "enabled", "RaidFrames": "enabled" },
    });
    std::fs::write(&sets_path, file.to_string()).expect("written");

    let output = set_run(&root, &home, &["--serious", "set", "list"]);
    let lines: Vec<String> = stdout(&output).lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].starts_with("everyday  2 addons (2 enabled, 0 disabled)  saved 20"));
    assert_eq!(
        lines[1],
        "raid  3 addons (2 enabled, 1 disabled)  saved 2026-10-22T00:00:00Z"
    );
    assert_eq!(
        lines[2],
        format!("2 sets of _forever_ in {}", sets_path.display())
    );

    let output = set_run(&root, &home, &["--serious", "set", "show", "raid"]);
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "BagSorter: disabled",
            "BossTimers: enabled",
            "RaidFrames: enabled",
            "set raid of _forever_: 3 addons (2 enabled, 1 disabled), saved 2026-10-22T00:00:00Z",
        ]
    );

    let output = set_run(&root, &home, &["--serious", "set", "apply", "raid"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "applying set raid to every character of _forever_",
            "Fixture Realm/Mrgltest: BagSorter disabled",
            "Second Realm/Grrlwave: BagSorter disabled, BossTimers enabled, RaidFrames enabled",
            "set raid names addons that are not installed in _forever_, their lines are written all the same: BagSorter, BossTimers, RaidFrames",
            "applied set raid to 2 characters of _forever_: 4 addon lines changed",
        ]
    );
    let after = snapshot(&wtf);
    assert_eq!(
        after[SETS_FIRST],
        b"RaidFrames: enabled\r\nBossTimers: enabled\r\nQuestArrows: disabled\r\nBagSorter: disabled\r\n"
    );
    assert_eq!(
        after[SETS_SECOND],
        b"QuestArrows: enabled\n# a line the client never wrote, kept as it is\nBagSorter: disabled\nBossTimers: enabled\nRaidFrames: enabled\n"
    );

    let output = set_run(&root, &home, &["--serious", "set", "apply", "raid"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_last_line(&output),
        "set raid already matches every character of _forever_ (2 characters), nothing written"
    );
    assert_eq!(snapshot(&wtf), after);

    let output = set_run(&root, &home, &["--serious", "set", "remove", "raid"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_last_line(&output),
        "removed set raid of _forever_ (3 addons (2 enabled, 1 disabled)); no AddOns.txt was touched"
    );
    assert_eq!(snapshot(&wtf), after);

    let output = set_run(&root, &home, &["--serious", "set", "apply", "raid"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "set not found: raid (flavor forever); the saved sets: everyday"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn set_json_is_one_document_per_command() {
    let (root, home) = temp_game_root_with_sets("json");
    let output = set_run(&root, &home, &["--json", "set", "save", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "set-save");
    assert_eq!(value["name"], "everyday");
    assert_eq!(value["set"]["addons"]["BagSorter"], "enabled");
    assert_eq!(value["characters"].as_array().map(Vec::len), Some(2));
    assert_eq!(value["characters"][1]["realm"], "Second Realm");
    assert_eq!(
        value["conflicts"],
        serde_json::json!(["BossTimers", "QuestArrows"])
    );
    assert_eq!(value["replaced"], false);

    let output = set_run(&root, &home, &["--json", "set", "apply", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "set-apply");
    assert_eq!(value["changed"], 1);
    assert_eq!(value["characters"][0]["realm"], "Fixture Realm");
    assert_eq!(value["characters"][0]["changed"], serde_json::json!([]));
    assert_eq!(value["characters"][1]["name"], "Grrlwave");
    assert_eq!(
        value["characters"][1]["changed"],
        serde_json::json!([{ "name": "RaidFrames", "state": "enabled" }])
    );
    assert_eq!(value["characters"][1]["created"], false);

    let output = set_run(&root, &home, &["--json", "set", "list"]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "set-list");
    assert_eq!(value["sets"][0]["name"], "everyday");
    assert_eq!(value["sets"][0]["enabled"], 2);

    let output = set_run(&root, &home, &["--json", "set", "remove", "nope"]);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).expect("one error");
    assert_eq!(value["kind"], "error");
    assert_eq!(
        value["message"],
        "set not found: nope (flavor forever); the saved sets: everyday"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn set_apply_is_refused_while_the_client_runs() {
    let (root, home) = temp_game_root_with_sets("client");
    let wtf = root.join("_forever_").join("WTF");
    let output = set_run(&root, &home, &["--serious", "set", "save", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let before = snapshot(&wtf);

    let Some(mut child) = spawn_stand_in(&root.join("_forever_")) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let output = set_run(&root, &home, &["--serious", "set", "apply", "everyday"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing was written");
    child.stop();

    let output = set_run(&root, &home, &["--serious", "set", "apply", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    assert_ne!(snapshot(&wtf), before);
    let _ = std::fs::remove_dir_all(&root);
}

// Task 027: `set apply <name> --character <name> [--realm <realm>]` and `set save --character`, over the
// same `addon-sets` tree.

#[test]
fn set_apply_to_one_character_leaves_the_other_and_refuses_an_ambiguous_name() {
    let (root, home) = temp_game_root_with_sets("character");
    let wtf = root.join("_forever_").join("WTF");
    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "save",
            "raid",
            "--character",
            "Mrgltest",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_last_line(&output),
        "saved set raid of _forever_ from Fixture Realm/Mrgltest: 4 addons (3 enabled, 1 disabled)"
    );
    let before = snapshot(&wtf);

    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "apply",
            "raid",
            "--character",
            "grrlwave",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "applying set raid to the character grrlwave of _forever_",
            "Second Realm/Grrlwave: BossTimers enabled, QuestArrows disabled, RaidFrames enabled",
            "set raid names addons that are not installed in _forever_, their lines are written all the same: BagSorter, BossTimers, QuestArrows, RaidFrames",
            "applied set raid to Second Realm/Grrlwave of _forever_: 3 addon lines changed",
        ]
    );
    let after = snapshot(&wtf);
    assert_eq!(
        after[SETS_FIRST], before[SETS_FIRST],
        "the other character, byte for byte"
    );
    assert_eq!(
        after[SETS_SECOND],
        b"QuestArrows: disabled\n# a line the client never wrote, kept as it is\nBagSorter: enabled\nBossTimers: enabled\nRaidFrames: enabled\n"
    );

    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "apply",
            "raid",
            "--character",
            "Grrlwave",
        ],
    );
    assert_eq!(
        stdout_last_line(&output),
        "set raid already matches Second Realm/Grrlwave of _forever_, nothing written"
    );

    // A second Mrgltest on the other realm: the bare name is refused with both realms, `--realm` picks.
    let twin = wtf.join("Account/MRGLSETS/Second Realm/Mrgltest");
    std::fs::create_dir_all(&twin).expect("a second Mrgltest");
    std::fs::write(twin.join("AddOns.txt"), b"QuestArrows: enabled\r\n").expect("its file");
    let before = snapshot(&wtf);
    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "apply",
            "raid",
            "--character",
            "Mrgltest",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "character Mrgltest matches 2 characters (flavor forever): Fixture Realm, Second Realm; name one with --realm"
    );
    assert_eq!(snapshot(&wtf), before, "nothing was written");

    let output = set_run(
        &root,
        &home,
        &[
            "--json",
            "set",
            "apply",
            "raid",
            "--character",
            "Mrgltest",
            "--realm",
            "Second Realm",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["characters"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["characters"][0]["realm"], "Second Realm");
    let after = snapshot(&wtf);
    assert_eq!(after[SETS_FIRST], before[SETS_FIRST]);
    assert_eq!(after[SETS_SECOND], before[SETS_SECOND]);
    assert!(
        after["Account/MRGLSETS/Second Realm/Mrgltest/AddOns.txt"]
            .starts_with(b"QuestArrows: disabled\r\n")
    );

    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "apply",
            "raid",
            "--realm",
            "Second Realm",
        ],
    );
    assert_eq!(output.status.code(), Some(2), "--realm needs --character");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn set_apply_to_one_character_is_refused_while_the_client_runs() {
    let (root, home) = temp_game_root_with_sets("character-client");
    let wtf = root.join("_forever_").join("WTF");
    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "save",
            "raid",
            "--character",
            "Mrgltest",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let before = snapshot(&wtf);
    let Some(mut child) = spawn_stand_in(&root.join("_forever_")) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let output = set_run(
        &root,
        &home,
        &[
            "--serious",
            "set",
            "apply",
            "raid",
            "--character",
            "Grrlwave",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing was written");
    child.stop();
    let _ = std::fs::remove_dir_all(&root);
}

// --- task 058: the curated Forever note of a partly restricted addon in `list` ---

#[test]
fn list_shows_the_curated_note_of_a_partly_restricted_addon() {
    let game = Game::new("forever-note");
    addon_by_hand(&game, "Grid2", "4.1.9");
    // A sidecar as `add github:michaelnpsp/Grid2` would leave it, with the canonical spec as its source.
    std::fs::create_dir_all(game.home.join("wowf")).expect("the flavor home");
    std::fs::write(
        game.state_path(),
        r#"{ "schema": 1, "flavor": "wowf", "addons": { "Grid2": { "source": "github:michaelnpsp/Grid2", "version": "4.1.9", "folders": ["Grid2"], "files": {}, "installed_at": "2026-10-22T09:00:00Z" } } }"#,
    )
    .expect("the sidecar");
    // The expectation comes from the compiled-in file, so a later check of the note (a new date) or its
    // age (stale after 90 days) does not break the test; the note itself has to be there.
    let status = mrgl_core::forever::ForeverStatus::for_source(
        "github:michaelnpsp/Grid2",
        mrgl_core::forever::ForeverBuild::Unknown,
    );
    let reason = status
        .note
        .as_deref()
        .expect("data/forever-notes.json has a note on Grid2");
    assert_eq!(status.kind.as_deref(), Some("secret-combat-values"));
    assert_eq!(
        status.source.as_deref(),
        Some("https://github.com/michaelnpsp/Grid2/issues/471")
    );
    let (mark, stale) = if status.stale {
        (",note,stale", ", stale: older than 90 days")
    } else {
        (",note", "")
    };

    let output = run_in(&game, &["list"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            format!(
                "Grid2  4.1.9  16001  managed  github:michaelnpsp/Grid2  forever:unknown{mark}"
            ),
            format!(
                "forever note on Grid2: {reason} (verified {}{stale}, https://github.com/michaelnpsp/Grid2/issues/471)",
                status.verified.as_deref().unwrap_or_default()
            ),
            "1 addon in _forever_ (wowf 1.60.1.70245): 1 managed by mrgl, 0 unmanaged".to_string(),
        ]
    );

    let value = game.list_json();
    let forever = &listed(&value, "Grid2")["forever"];
    assert_eq!(forever["build"], "unknown");
    assert_eq!(forever["note"], reason);
    assert_eq!(forever["kind"], "secret-combat-values");
    assert_eq!(
        forever["source"],
        "https://github.com/michaelnpsp/Grid2/issues/471"
    );
    assert_eq!(forever["stale"], status.stale);
}
