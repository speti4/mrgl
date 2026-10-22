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
#[cfg(windows)]
use std::process::Child;
use std::process::{Command, Output, Stdio};

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
    command.env_remove("MRGL_WAGO_KEY");
    command.env_remove("MRGL_CURSEFORGE_KEY");
    // No test reads or writes the real Windows Credential Manager (task 093).
    command.env("MRGL_KEY_STORE", "off");
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
        .args(["add", "bitbucket:something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "unknown source: bitbucket:something (expected github:<owner>/<repo>, wago:<id>, wowi:<id> or curse:<id>)"
    );
}

// --- task 092: a source this build does not have names what is missing ---

#[test]
fn add_of_a_source_this_build_lacks_names_what_is_missing() {
    for (spec, line) in [
        (
            "wago:something",
            "source not configured: wago:something (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)",
        ),
        (
            "https://www.curseforge.com/projects/77",
            "source not configured: curse:77 (this build has no CurseForge source)",
        ),
        (
            "curse:42",
            "source not configured: curse:42 (this build has no CurseForge source)",
        ),
    ] {
        for dry_run in [false, true] {
            let mut command = mrgl();
            command.args(["--serious", "add", spec, "--flavor", "forever"]);
            if dry_run {
                command.arg("--dry-run");
            }
            let output = command.output().expect("run mrgl");
            assert_eq!(output.status.code(), Some(1), "{spec}");
            assert_eq!(stderr_last_line(&output), line, "{spec}");
        }
    }
    // Through the voice: the murloc line comes before the plain one.
    let output = mrgl()
        .args(["add", "wago:something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.lines().count() >= 2, "{stderr}");
    assert_eq!(
        stderr_last_line(&output),
        "source not configured: wago:something (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)"
    );
}

#[test]
fn import_reads_a_list_entry_of_a_source_this_build_lacks_as_not_configured() {
    let game = Game::new("import-not-configured");
    let base = game.root.parent().expect("the base").to_path_buf();
    let list = base.join("not-configured.json");
    std::fs::write(
        &list,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [
                { "name": "Questie", "source": "github:Questie/Questie", "version": "1.2.3",
                  "folders": ["Questie"] },
                { "name": "Clique", "source": "curse:123", "version": "1.0.0", "folders": ["Clique"] },
            ],
        })
        .to_string(),
    )
    .expect("the list");
    let output = game
        .mrgl()
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "Clique  source not configured (curse:123): this build has no CurseForge source"),
        "{text}"
    );
    assert!(
        stdout_last_line(&output)
            .ends_with("1 to install, 1 source not configured; nothing written"),
        "{text}"
    );
    let output = game
        .mrgl()
        .args(["--json", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["addons"][1]["outcome"], "source-not-configured");
    assert_eq!(
        value["addons"][1]["detail"],
        "this build has no CurseForge source"
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
            "outcome": "installed",
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
            "source not configured: wago:abc (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)",
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
            "no asset for flavor forever in github:Example/MainlineOnly v4.0.0 (release.json names mainline, standard; assets: Only-v4.0.0.zip, release.json; looked at 1 release)",
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

/// The synthetic game folder of the dependency check: `Needy` with a met, an embedded-only and a missing
/// required dependency, `LibMet`, and `NolibBuild` with a library folder inside. An embedded-only
/// dependency is missing (task 076): the client does not count a library inside an addon's own folder.
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
            "Needy  1.0.0  16001  unmanaged  missing: LibEmbedded, LibMissing",
            "NolibBuild  1.0.0  16001  unmanaged  missing: LibEmbedded",
            "Needy needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside Needy, and the client does not count a library inside an addon's own folder)",
            "NolibBuild needs the missing dependency LibEmbedded: install it or the addon does not load (LibEmbedded is only embedded inside NolibBuild, and the client does not count a library inside an addon's own folder)",
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
            "missing": ["LibEmbedded", "LibMissing"],
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
            "Needy needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside Needy, and the client does not count a library inside an addon's own folder)",
            "NolibBuild needs the missing dependency LibEmbedded: install it or the addon does not load (LibEmbedded is only embedded inside NolibBuild, and the client does not count a library inside an addon's own folder)"
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
            "NeedsLibrary needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside NeedsLibrary, and the client does not count a library inside an addon's own folder)",
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
    assert_eq!(
        dependencies["missing"],
        serde_json::json!(["LibEmbedded", "LibMissing"])
    );
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
/// `\Users\` folder (the 8.3 short form a runner's temporary folder carries included), with `\` or `/` as
/// the separator.
fn assert_no_home_path(output: &str) {
    let lower = output.to_lowercase().replace('/', "\\");
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        let profile = profile.to_string_lossy().to_lowercase().replace('/', "\\");
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
        "api keys: MRGL_GITHUB_TOKEN set (environment), MRGL_WAGO_KEY not set, MRGL_CURSEFORGE_KEY not set",
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
        serde_json::json!([
            { "name": "MRGL_GITHUB_TOKEN", "present": true, "origin": "environment" },
            { "name": "MRGL_WAGO_KEY", "present": false, "origin": "not_set" },
            { "name": "MRGL_CURSEFORGE_KEY", "present": false, "origin": "not_set" },
        ])
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
fn doctor_hides_a_home_path_written_with_forward_slashes() {
    let root = doctor_game_root("forward-slashes");
    let home = temp_home("doctor-forward-slashes");
    let forward = |path: &Path| path.display().to_string().replace('\\', "/");
    let output = mrgl()
        .env("MRGL_HOME", forward(&home))
        .args(["--serious", "doctor", "--root", &forward(&root)])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let stdout = stdout(&output);
    assert_no_home_path(&stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines[2].starts_with("root: "), "{stdout}");
    assert!(lines[3].starts_with("home: "), "{stdout}");
    // The temporary folder lies under the profile folder (in its long or short form) on a usual machine
    // and on the runner; elsewhere there is nothing to hide.
    let temp = std::env::temp_dir().display().to_string().to_lowercase();
    if temp.contains(":\\users\\") {
        assert!(lines[2].starts_with("root: ~/"), "{stdout}");
        assert!(lines[3].starts_with("home: ~/"), "{stdout}");
    }
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
    assert_eq!(
        lines[6],
        "api keys: MRGL_GITHUB_TOKEN not set, MRGL_WAGO_KEY not set, MRGL_CURSEFORGE_KEY not set"
    );
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

// --- task 093: `mrgl key` and the doctor name the keys and their origin, never a value ---

const KEY_SECRET: &str = "wago_madeUpKeyValueForTheKeyCommandTest";

/// `mrgl` with a standard input to read.
fn mrgl_with_input(args: &[&str], input: &str, env: &[(&str, &str)]) -> Output {
    use std::io::Write as _;
    let mut command = mrgl();
    command
        .args(args)
        .envs(env.iter().copied())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("run mrgl");
    child
        .stdin
        .take()
        .expect("a stdin")
        .write_all(input.as_bytes())
        .expect("the input");
    child.wait_with_output().expect("mrgl ends")
}

/// Every byte a run wrote, the standard output and the error output.
fn all_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn key_list_names_every_key_and_its_origin_never_a_value() {
    let env = [("MRGL_WAGO_KEY", KEY_SECRET), ("MRGL_CURSEFORGE_KEY", "  ")];
    let output = mrgl_with_input(&["--serious", "key", "list"], "", &env);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "MRGL_GITHUB_TOKEN not set",
            "MRGL_WAGO_KEY set (environment)",
            "MRGL_CURSEFORGE_KEY not set",
            "the key store is not available: MRGL_KEY_STORE=off",
            "1 of 3 keys set, none shown",
        ],
        "{text}"
    );
    let json = mrgl_with_input(&["--json", "key", "list"], "", &env);
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["kind"], "keys");
    assert_eq!(
        value["api_keys"],
        serde_json::json!([
            { "source": "github", "name": "MRGL_GITHUB_TOKEN", "origin": "not_set" },
            { "source": "wago", "name": "MRGL_WAGO_KEY", "origin": "environment" },
            { "source": "curse", "name": "MRGL_CURSEFORGE_KEY", "origin": "not_set" },
        ])
    );
    assert_eq!(value["store"], "the key store");
    assert_eq!(
        value["store_unavailable"],
        "the key store is not available: MRGL_KEY_STORE=off"
    );
    for output in [&output, &json] {
        assert!(
            !all_output(output).contains(KEY_SECRET),
            "{}",
            all_output(output)
        );
    }
}

#[test]
fn key_set_and_clear_read_stdin_and_never_print_the_key() {
    // The store is off in every test: the round trip itself is the core's, against a store in memory.
    for args in [
        vec!["--serious", "key", "set", "wago"],
        vec!["--json", "key", "set", "curse"],
        vec!["--serious", "key", "clear", "github"],
    ] {
        let output = mrgl_with_input(&args, &format!("{KEY_SECRET}\n"), &[]);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        // The JSON error document carries the same plain line.
        assert!(
            stderr_last_line(&output)
                .contains("the key store is not available: MRGL_KEY_STORE=off"),
            "{args:?}: {}",
            stderr_last_line(&output)
        );
        assert!(
            !all_output(&output).contains(KEY_SECRET),
            "{}",
            all_output(&output)
        );
    }
    for (source, line) in [
        ("wowi", "no key for WoWInterface: its API needs none"),
        (
            "bitbucket",
            "no key source: bitbucket (github, wago or curse)",
        ),
    ] {
        let output = mrgl_with_input(&["--serious", "key", "set", source], KEY_SECRET, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stderr_last_line(&output), line);
        assert!(!all_output(&output).contains(KEY_SECRET));
    }
    // The key is never an argument: one after the source is refused, and the refusal does not echo it.
    let output = mrgl()
        .args(["--serious", "key", "set", "wago", KEY_SECRET])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "a key is never an argument, the shell's history would keep it: pipe it in (Get-Clipboard | mrgl key set wago); nothing stored"
    );
    assert!(
        !all_output(&output).contains(KEY_SECRET),
        "{}",
        all_output(&output)
    );
}

#[test]
fn no_key_value_reaches_any_output_of_the_doctor_or_a_source() {
    let env = [
        ("MRGL_GITHUB_TOKEN", DOCTOR_SECRET),
        ("MRGL_WAGO_KEY", KEY_SECRET),
        ("MRGL_CURSEFORGE_KEY", KEY_SECRET),
    ];
    let runs = [
        vec!["doctor"],
        vec!["--json", "doctor"],
        vec!["--serious", "add", "wago:abc", "--flavor", "forever"],
        vec![
            "--json",
            "add",
            "curse:42",
            "--flavor",
            "forever",
            "--dry-run",
        ],
        vec![
            "--serious",
            "add",
            "github:Example/Missing",
            "--flavor",
            "forever",
            "--dry-run",
        ],
    ];
    for args in runs {
        let output = mrgl_with_input(&args, "", &env);
        let all = all_output(&output);
        assert!(
            !all.contains(DOCTOR_SECRET) && !all.contains(KEY_SECRET),
            "{args:?}: {all}"
        );
    }
    let output = mrgl_with_input(&["--serious", "doctor"], "", &env);
    assert!(
        stdout(&output).lines().any(|line| line
            == "api keys: MRGL_GITHUB_TOKEN set (environment), MRGL_WAGO_KEY set (environment), MRGL_CURSEFORGE_KEY set (environment)"),
        "{}",
        stdout(&output)
    );
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

/// A copy of the synthetic addon folder of `mrgl-core` (`single-addon/SoloAddon`: a TOC with the Forever
/// suffix only and an embedded `LibSolo` with a TOC of its own) next to the game folder (task 062).
#[cfg(windows)]
fn solo_copy(game: &Game) -> PathBuf {
    let solo = game
        .root
        .parent()
        .expect("the base folder")
        .join("repos")
        .join("SoloAddon");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../mrgl-core/tests/fixtures/single-addon/SoloAddon"),
        &solo,
    );
    solo
}

#[cfg(windows)]
#[test]
fn link_and_unlink_take_a_single_addon_folder() {
    let game = Game::new("link-single");
    let solo = solo_copy(&game);
    let dev = dev_copy(&game);
    let before = snapshot(&solo);

    let output = game
        .mrgl()
        .args(["--serious", "link"])
        .arg(&solo)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(
        lines[0],
        format!("linking the addon folder {} into _forever_", solo.display())
    );
    assert!(
        lines[1].starts_with("linked the addon SoloAddon from "),
        "{text}"
    );
    assert!(lines[1].ends_with("SoloAddon into _forever_"), "{text}");
    assert_eq!(addons_listing(&game), ["SoloAddon"]);

    // Again: nothing new, said as such.
    let output = game
        .mrgl()
        .args(["--serious", "link"])
        .arg(&solo)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout_last_line(&output).ends_with(" into _forever_ already"),
        "{}",
        stdout(&output)
    );

    // The JSON names the shape; a development folder still links its subfolders.
    let output = game
        .mrgl()
        .args(["--json", "link"])
        .arg(&dev)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["shape"], "development-folder");
    assert_eq!(value["linked"].as_array().expect("linked").len(), 2);
    assert_eq!(
        addons_listing(&game),
        ["DevAddon", "OneFolder", "SoloAddon"]
    );
    let output = game
        .mrgl()
        .args(["--json", "link"])
        .arg(&solo)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["shape"], "addon-folder");
    assert_eq!(value["linked"][0]["name"], "SoloAddon");
    assert_eq!(value["linked"][0]["already_linked"], true);
    assert_eq!(value["without_toc"], serde_json::json!([]));

    // Unlinking the addon folder takes its one junction; the development folder's stay.
    let output = game
        .mrgl()
        .args(["--serious", "unlink"])
        .arg(&solo)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let last = stdout_last_line(&output);
    assert!(last.starts_with("unlinked 1 addon of "), "{last}");
    assert!(last.ends_with(" from _forever_: SoloAddon"), "{last}");
    assert_eq!(addons_listing(&game), ["DevAddon", "OneFolder"]);
    assert_eq!(snapshot(&solo), before, "the addon folder");
    let state = std::fs::read_to_string(game.state_path()).expect("the sidecar");
    assert!(!state.contains("SoloAddon"), "{state}");
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

/// The synthetic game folder of the interface states (task 076): `_retail_` at `12.1.5.70448` (interface
/// 120105) with `Behind` (120100), `Current` (120105, 50504) and `Elsewhere` (110207).
fn interface_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/interface/_retail_")
}

#[test]
fn scan_and_list_mark_an_out_of_date_toc_apart_from_an_interface_mismatch() {
    let summary = "1 addon is out of date for the build 12.1.5.70448 (interface 120105): Behind (120100); the client lists it as out of date";
    let mismatch = "1 addon names no interface of the build 12.1.5.70448: Elsewhere (110207); made for another flavor or an older version of the game?";
    let output = mrgl()
        .args(["--serious", "scan"])
        .arg(interface_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "Behind  1.0.0  120100  out of date",
            "Current  1.0.0  120105,50504",
            "Elsewhere  1.0.0  110207  interface mismatch",
            mismatch,
            summary,
            "3 addons in _retail_ (wow 12.1.5.70448)",
        ],
        "{text}"
    );
    let output = mrgl()
        .args(["--serious", "list", "--flavor"])
        .arg(interface_fixture())
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "Behind  1.0.0  120100  unmanaged  out of date",
            "Current  1.0.0  120105,50504  unmanaged",
            "Elsewhere  1.0.0  110207  unmanaged  interface mismatch",
            mismatch,
            summary,
            "3 addons in _retail_ (wow 12.1.5.70448): 0 managed by mrgl, 3 unmanaged",
        ],
        "{text}"
    );
    for command in ["scan", "list"] {
        let mut run = mrgl();
        run.args(["--json", command]);
        if command == "list" {
            run.arg("--flavor");
        }
        let output = run.arg(interface_fixture()).output().expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("one JSON document");
        assert_eq!(
            value["interface_out_of_date"],
            serde_json::json!([{
                "folder": "Behind",
                "toc": "Behind.toc",
                "interfaces": [120100],
                "build": "12.1.5.70448",
                "interface": 120105,
                "highest": 120100,
            }]),
            "{command}"
        );
        assert_eq!(
            value["interface_mismatches"],
            serde_json::json!([{
                "folder": "Elsewhere",
                "toc": "Elsewhere.toc",
                "interfaces": [110207],
                "build": "12.1.5.70448",
                "major": 12,
            }]),
            "{command}"
        );
    }
}

// --- task 082: `add <file.zip>`, `import` and `update` report the whole interface check ---

/// The after-install line of the out-of-date fixture zip (16000 under the Forever fixture build 1.60.1).
const BEHIND_LINE: &str = "BehindInterface: interface 16000 is out of date for the build 1.60.1.70245 (16001) of _forever_; the client lists it as out of date";
/// The after-install line of the mismatch fixture zip (110207, another major).
const STALE_LINE: &str = "StaleInterface: interface 110207 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor";

/// The `interface_out_of_date` object of the out-of-date fixture zip.
fn behind_json() -> serde_json::Value {
    serde_json::json!({
        "folder": "BehindInterface",
        "toc": "BehindInterface.toc",
        "interfaces": [16000],
        "build": "1.60.1.70245",
        "interface": 16001,
        "highest": 16000,
    })
}

/// The `interface_mismatch` object of the mismatch fixture zip.
fn stale_json() -> serde_json::Value {
    serde_json::json!({
        "folder": "StaleInterface",
        "toc": "StaleInterface.toc",
        "interfaces": [110207],
        "build": "1.60.1.70245",
        "major": 1,
    })
}

#[test]
fn add_reports_an_out_of_date_install_next_to_the_mismatch() {
    let game = Game::new("add-out-of-date");
    for (zip, summary, warning) in [
        (
            "behind-interface-1.0.0.zip",
            "installed BehindInterface 1.0.0 (1 folder: BehindInterface) into _forever_",
            BEHIND_LINE,
        ),
        (
            "stale-interface-1.0.0.zip",
            "installed StaleInterface 1.0.0 (1 folder: StaleInterface) into _forever_",
            STALE_LINE,
        ),
    ] {
        let output = game
            .mrgl()
            .args(["--serious", "add"])
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let text = stdout(&output);
        let lines: Vec<&str> = text.lines().skip(1).collect();
        assert_eq!(lines, [summary, warning], "{zip}: {text}");
    }

    for (zip, mismatch, out_of_date) in [
        (
            "behind-interface-1.0.0.zip",
            serde_json::Value::Null,
            behind_json(),
        ),
        (
            "stale-interface-1.0.0.zip",
            stale_json(),
            serde_json::Value::Null,
        ),
        (
            "one-folder-1.0.0.zip",
            serde_json::Value::Null,
            serde_json::Value::Null,
        ),
    ] {
        let output = game
            .mrgl()
            .args(["--json", "add"])
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("one JSON document");
        assert_eq!(value["interface_mismatch"], mismatch, "{zip}");
        assert_eq!(value["interface_out_of_date"], out_of_date, "{zip}");
    }
}

#[test]
fn import_reports_an_out_of_date_install_next_to_the_mismatch() {
    for (zip, name, warning, mismatch, out_of_date) in [
        (
            "behind-interface-1.0.0.zip",
            "BehindInterface",
            BEHIND_LINE,
            serde_json::Value::Null,
            behind_json(),
        ),
        (
            "stale-interface-1.0.0.zip",
            "StaleInterface",
            STALE_LINE,
            stale_json(),
            serde_json::Value::Null,
        ),
    ] {
        for json in [false, true] {
            let game = Game::new(&format!("import-interface-{name}-{json}"));
            let base = game.root.parent().expect("the base").to_path_buf();
            let stub = stub_with_forever_zip(&base, zip);
            let list = write_list(
                &game,
                "list.json",
                &[(name, "github:Questie/Questie", "1.2.3")],
            );
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
                assert_eq!(item["interface_mismatch"], mismatch, "{zip}");
                assert_eq!(item["interface_out_of_date"], out_of_date, "{zip}");
            } else {
                let text = stdout(&output);
                let lines: Vec<&str> = text.lines().collect();
                let installed = format!("{name}  installed 1.2.3 from github:Questie/Questie");
                assert_eq!(
                    lines,
                    [
                        "importing 1 addon from list.json into _forever_",
                        installed.as_str(),
                        warning,
                        "imported 1 addon of list.json into _forever_: 1 installed",
                    ],
                    "{text}"
                );
            }
        }
    }
}

#[test]
fn update_reports_the_interface_check_of_an_updated_addon() {
    // One game each: the stub serves one zip behind the one release of `Questie/Questie`.
    for (name, zip, warning) in [
        ("BehindInterface", "behind-interface-1.0.0.zip", BEHIND_LINE),
        ("StaleInterface", "stale-interface-1.0.0.zip", STALE_LINE),
    ] {
        let game = managed_game(
            &format!("update-interface-{name}"),
            &[(name, "github:Questie/Questie", "1.2.2", "")],
        );
        let base = game.root.parent().expect("the base").to_path_buf();
        let stub = stub_with_forever_zip(&base, zip);
        let output = game
            .mrgl()
            .env("MRGL_HTTP_STUB", &stub)
            .args(["--serious", "update", name, "--yes", "--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        let text = stdout(&output);
        let lines: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with("updated "))
            .collect();
        let updated = format!("updated {name} from 1.2.2 to 1.2.3 (1 folder: {name}) in _forever_");
        assert_eq!(
            lines,
            [updated.as_str(), warning, "1 addon updated"],
            "{text}"
        );
    }

    // The JSON document carries both keys per addon: the check of the updated one, `null` for the rest.
    let game = managed_game(
        "update-interface-json",
        &[
            ("BehindInterface", "github:Questie/Questie", "1.2.2", ""),
            ("Plain", "github:Example/Prerelease", "1.0.0", ""),
        ],
    );
    let stub = stub_with_forever_zip(
        game.root.parent().expect("the base"),
        "behind-interface-1.0.0.zip",
    );
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--json", "update", "--all", "--yes", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let addon = |name: &str| {
        value["addons"]
            .as_array()
            .expect("the addons")
            .iter()
            .find(|addon| addon["name"] == name)
            .unwrap_or_else(|| panic!("{name} in {value}"))
            .clone()
    };
    let behind = addon("BehindInterface");
    assert_eq!(behind["updated"], true, "{value}");
    assert_eq!(behind["interface_out_of_date"], behind_json());
    assert_eq!(behind["interface_mismatch"], serde_json::Value::Null);
    let plain = addon("Plain");
    assert_eq!(plain["updated"], false);
    assert_eq!(plain["interface_out_of_date"], serde_json::Value::Null);
    assert_eq!(plain["interface_mismatch"], serde_json::Value::Null);
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
fn a_reinstall_of_the_installed_version_says_so_and_keeps_the_rollback() {
    // Task 083: the zip of the version the sidecar records is a reinstall, not an update from the previous
    // version to the same one; the previous version stays for `rollback`.
    let game = Game::new("reinstall");
    add_in(&game, &["one-folder-1.0.0.zip"]);
    let add = |zip: &str, args: &[&str]| {
        let output = game
            .mrgl()
            .args(args)
            .arg("add")
            .arg(zips().join(zip))
            .args(["--flavor", "forever"])
            .output()
            .expect("run mrgl");
        assert!(output.status.success(), "{}", stderr_last_line(&output));
        output
    };
    let output = add("one-folder-1.0.0.zip", &["--serious"]);
    assert_eq!(
        stdout_last_line(&output),
        "reinstalled OneFolder 1.0.0 (1 folder: OneFolder) in _forever_",
        "a reinstall of the first install"
    );
    let output = add("one-folder-1.1.0.zip", &["--serious"]);
    assert_eq!(
        stdout_last_line(&output),
        "updated OneFolder from 1.0.0 to 1.1.0 (1 folder: OneFolder) in _forever_",
        "a real update still says updated"
    );
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(zips().join("one-folder-1.1.0.zip"))
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "dry run: would reinstall OneFolder 1.1.0 (1 folder: OneFolder) in _forever_"
    );
    let output = add("one-folder-1.1.0.zip", &["--serious"]);
    assert_eq!(
        stdout_last_line(&output),
        "reinstalled OneFolder 1.1.0 (1 folder: OneFolder) in _forever_"
    );
    // The murloc speaks on the reinstall too: the line goes through the voice.
    let output = add("one-folder-1.1.0.zip", &[]);
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "two two-line messages: {text}");
    assert_eq!(
        lines[3],
        "reinstalled OneFolder 1.1.0 (1 folder: OneFolder) in _forever_"
    );

    let output = add("one-folder-1.1.0.zip", &["--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["addon"]["outcome"], "reinstalled");
    assert_eq!(value["addon"]["version"], "1.1.0");
    assert_eq!(value["addon"]["previous_version"], "1.0.0");
    let output = game
        .mrgl()
        .args(["--json", "add"])
        .arg(zips().join("one-folder-1.1.0.zip"))
        .args(["--flavor", "forever", "--dry-run"])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["addon"]["outcome"], "reinstalled");
    assert_eq!(value["addon"]["previous_version"], "1.0.0");
    assert_eq!(
        sidecar(&game)["addons"]["OneFolder"]["previous_version"],
        "1.0.0"
    );

    let output = run_in(&game, &["rollback", "onefolder"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "rolled back OneFolder from 1.1.0 to 1.0.0 (1 folder: OneFolder) in _forever_; pinned at 1.0.0, `mrgl unpin OneFolder` lets updates reach it again"
    );
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

/// One orphan reads `1 orphan … has no addon folder` (task 086: the line is the core's
/// `Listing::summary`, the window's toolbox prints it too, with its cleanup named instead of the command).
#[test]
fn savedvars_says_has_for_one_orphan() {
    let root = savedvars_game("one-orphan");
    let flavor = root.join("_forever_");
    let wtf = flavor.join("WTF");
    for gone in &SAVEDVARS_ORPHANS[..3] {
        std::fs::remove_file(wtf.join(gone)).expect("an orphan removed");
    }
    let total: u64 = snapshot(&wtf)
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
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "5 SavedVariables files of _forever_, {} in all; 1 orphan (90 B) has no addon folder: `mrgl savedvars --clean` moves it into a backup zip",
            savedvars_bytes(total)
        )
    );

    let home = root.join("home");
    let output = mrgl()
        .env("MRGL_HOME", &home)
        .args(["--serious", "savedvars", "--clean", "--flavor"])
        .arg(&flavor)
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{output:?}");
    let zip = std::fs::read_dir(home.join("backups").join("wowf"))
        .expect("the backups folder")
        .next()
        .expect("the zip")
        .expect("an entry")
        .path();
    assert_eq!(
        stdout_last_line(&output),
        format!(
            "moved 1 orphan of _forever_ (1 file, 90 B) into {}; unzip it into the WTF folder to put them back",
            zip.display()
        )
    );
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

/// A game with a fixture sidecar of twelve managed addons, one or two for every state of the update check,
/// each with its folder made by hand, and an unmanaged folder. The fixture answers serve the Forever zip of
/// `Questie/Questie` v1.2.3, the one update. Task 068: `Switched` has the prerelease of
/// `Example/Prerelease` on the stable channel (a channel switch) and `Delisted` a version the release list
/// no longer has; both are newer than the current release, not an update.
fn update_game(name: &str) -> Game {
    let entries = [
        (
            "Beta",
            "github:Example/Prerelease",
            "1.1.0-beta",
            r#", "channel": "prerelease""#,
        ),
        ("Delisted", "github:Example/Prerelease", "1.5.0", ""),
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
        ("Switched", "github:Example/Prerelease", "1.1.0-beta", ""),
        ("Zipped", r"C:\\Downloads\\Zipped-1.0.0.zip", "1.0.0", ""),
    ];
    managed_game(name, &entries)
}

/// A game whose sidecar manages the entries (folder, source, version, the entry's other fields), each
/// with its folder made by hand, and an unmanaged folder `ByHand`.
fn managed_game(name: &str, entries: &[(&str, &str, &str, &str)]) -> Game {
    let game = Game::new(name);
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
    let gone = lines.remove(9);
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
            "checking Beta against github:Example/Prerelease (1 of 12)",
            "Beta  1.1.0-beta  up to date",
            "checking Delisted against github:Example/Prerelease (2 of 12)",
            "Delisted  1.5.0  newer than the current release 1.0.0 (1.5.0 is no longer listed), left alone",
            "Dev  dev  not checked: a linked development folder",
            "checking Empty against github:Example/Empty (4 of 12)",
            "Empty  1.0.0  source gone: no release in github:Example/Empty",
            "checking Gone against github:Example/Missing (5 of 12)",
            "checking Guess against github:Example/NoReleaseJson (6 of 12)",
            "Guess  2.0.0 -> 3.0.0  guess only: no release.json names forever; the asset name ends in -forever, a guess; `mrgl add github:Example/NoReleaseJson` installs it anyway",
            "Ignored  1.0.0  ignored, not checked",
            "checking Pinned against github:Example/Notes (8 of 12)",
            "Pinned  2.0.0  pinned at 2.0.0, 2.1.0 is out, left alone",
            "checking Plain against github:Example/Prerelease (9 of 12)",
            "Plain  1.0.0  up to date",
            "checking Questie against github:Questie/Questie (10 of 12)",
            "Questie  1.2.2 -> 1.2.3  update available",
            "checking Switched against github:Example/Prerelease (11 of 12)",
            "Switched  1.1.0-beta  newer than the current release 1.0.0, left alone",
            "Zipped  1.0.0  not checked: a zip in hand; a newer zip goes in with mrgl add <zip>",
            "dry run: 1 update available, 2 up to date, 2 newer than the release, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked; nothing written",
        ],
        "{text}"
    );
    assert_eq!(game_snapshot(&game), before, "a dry run writes nothing");
    assert!(!game.home.join("wowf").join("staging").exists());
}

#[test]
fn update_does_not_ask_a_source_this_build_lacks_and_says_what_is_missing() {
    let game = managed_game(
        "update-not-configured",
        &[
            ("Clique", "curse:123", "1.0.0", ""),
            ("Questie", "github:Questie/Questie", "1.2.3", ""),
        ],
    );
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[1..3],
        [
            "Clique  1.0.0  not checked: source not configured: curse:123 (this build has no CurseForge source)",
            "checking Questie against github:Questie/Questie (2 of 2)",
        ],
        "{text}"
    );
}

#[test]
fn update_all_with_yes_updates_through_the_one_path_and_counts_the_rest() {
    let game = update_game("update-all");
    let before = sidecar(&game);
    let output = run_in(&game, &["update", "--all", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("updating "))
        .collect();
    assert_eq!(
        lines,
        [
            "updating Questie from 1.2.2 to 1.2.3",
            "downloading Questie-v1.2.3-forever.zip",
            "installing Questie-v1.2.3-forever.zip into _forever_",
            "updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in _forever_",
            "1 addon updated, 2 up to date, 2 newer than the release, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked",
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
    for name in [
        "Beta", "Delisted", "Pinned", "Ignored", "Guess", "Empty", "Switched", "Zipped",
    ] {
        assert_eq!(after["addons"][name], before["addons"][name], "{name}");
    }
    assert_eq!(staging_entries(&game), Vec::<String>::new());

    // A second run finds nothing to update.
    let output = run_in(&game, &["update", "--all", "--yes"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "nothing updated: 3 up to date, 2 newer than the release, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked"
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
            "update-available": 1, "up-to-date": 2, "installed-newer": 2, "pinned": 1, "ignored": 1,
            "guess-only": 1, "source-gone": 2, "not-checked": 2, "rate-limited": 0, "failed": 0,
            "pending": 0,
        })
    );
    assert_eq!(value["stopped"], serde_json::json!([]));
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
            ("Delisted", "installed-newer"),
            ("Dev", "not-checked"),
            ("Empty", "source-gone"),
            ("Gone", "source-gone"),
            ("Guess", "guess-only"),
            ("Ignored", "ignored"),
            ("Pinned", "pinned"),
            ("Plain", "up-to-date"),
            ("Questie", "update-available"),
            ("Switched", "installed-newer"),
            ("Zipped", "not-checked"),
        ]
    );
    let questie = &value["addons"][9];
    assert_eq!(questie["latest"], "1.2.3");
    assert_eq!(questie["updated"], true);
    assert_eq!(questie["addon"]["version"], "1.2.3");
    assert_eq!(questie["addon"]["previous_version"], "1.2.2");
    assert_eq!(questie["pick"]["asset"], "Questie-v1.2.3-forever.zip");
    assert_eq!(value["addons"][1]["updated"], false);
    assert_eq!(value["addons"][1]["latest"], "1.0.0");
    assert_eq!(value["addons"][1]["detail"], "1.5.0 is no longer listed");
    assert_eq!(value["addons"][10]["latest"], "1.0.0");
    assert_eq!(value["addons"][10]["detail"], serde_json::Value::Null);
    assert_eq!(value["addons"][7]["pinned"], "2.0.0");
    assert_eq!(value["addons"][7]["latest"], "2.1.0");
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
            "checking Pinned against github:Example/Notes (1 of 1)",
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

// --- task 077: a pinned addon newer than the current release is read against the version order ---

#[test]
fn update_reports_a_pin_newer_than_the_current_release_without_is_out() {
    // `Held` is pinned at the prerelease of `Example/Prerelease` on the stable channel (the release list's
    // order), `Kept` at a version the list no longer has (the lenient order), `Behind` below the release.
    let game = managed_game(
        "update-pinned-newer",
        &[
            (
                "Behind",
                "github:Example/Notes",
                "2.0.0",
                r#", "pinned": "2.0.0""#,
            ),
            (
                "Held",
                "github:Example/Prerelease",
                "1.1.0-beta",
                r#", "pinned": "1.1.0-beta""#,
            ),
            (
                "Kept",
                "github:Example/Prerelease",
                "1.5.0",
                r#", "pinned": "1.5.0""#,
            ),
        ],
    );
    let before = game_snapshot(&game);
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("checking "))
        .collect();
    assert_eq!(
        lines,
        [
            "Behind  2.0.0  pinned at 2.0.0, 2.1.0 is out, left alone",
            "Held  1.1.0-beta  pinned at 1.1.0-beta, newer than the current release 1.0.0, left alone",
            "Kept  1.5.0  pinned at 1.5.0, newer than the current release 1.0.0 (1.5.0 is no longer listed), left alone",
            "dry run: 3 pinned; nothing written",
        ],
        "{text}"
    );
    assert!(!text.contains("1.0.0 is out"), "{text}");
    assert_eq!(game_snapshot(&game), before, "a dry run writes nothing");

    let output = game
        .mrgl()
        .args([
            "--json",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let facts: Vec<(&str, &str, &str, &str)> = value["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| {
            (
                addon["name"].as_str().expect("a name"),
                addon["state"].as_str().expect("a state"),
                addon["latest"].as_str().expect("the current release"),
                addon["order"].as_str().expect("the order"),
            )
        })
        .collect();
    assert_eq!(
        facts,
        [
            ("Behind", "pinned", "2.1.0", "older"),
            ("Held", "pinned", "1.0.0", "newer"),
            ("Kept", "pinned", "1.0.0", "newer"),
        ]
    );
    assert_eq!(value["addons"][2]["detail"], "1.5.0 is no longer listed");
}

// --- task 068: the update check stops before GitHub's rate limit without a token ---

#[test]
fn update_stops_before_the_rate_limit_with_the_reset_time_and_counts_the_rest() {
    let game = managed_game(
        "update-budget",
        &[
            ("Alpha", "github:Example/Notes", "2.0.0", ""),
            ("Bravo", "github:Questie/Questie", "1.2.2", ""),
            ("Charlie", r"C:\\Downloads\\Charlie-1.0.0.zip", "1.0.0", ""),
            ("Delta", "github:Example/Prerelease", "1.0.0", ""),
            (
                "Echo",
                "github:Example/NoMetadata",
                "1.0.0",
                r#", "ignored": true"#,
            ),
        ],
    );
    // The first answer says no request is left this hour, as GitHub's headers do.
    let stub = game.home.join("stub-budget");
    copy_tree(&releases(), &stub);
    let reset = 1_792_598_400;
    std::fs::write(
        stub.join("api.github.com/repos/Example/Notes/releases.json.headers"),
        format!("X-RateLimit-Limit: 60\nX-RateLimit-Remaining: 0\nX-RateLimit-Reset: {reset}\n"),
    )
    .expect("the headers");
    let before = game_snapshot(&game);
    let stop = format!(
        "stopped asking GitHub before its rate limit of 60 requests an hour without a token: 2 addons not checked, the limit resets at {} (local time); set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour",
        mrgl_core::sources::local_time_text(reset)
    );
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "checking the managed addons of _forever_ against their sources",
            "checking Alpha against github:Example/Notes (1 of 5)",
            "Alpha  2.0.0 -> 2.1.0  update available",
            stop.as_str(),
            "Bravo  1.2.2  not checked (rate limit): not asked before GitHub's rate limit",
            "Charlie  1.0.0  not checked: a zip in hand; a newer zip goes in with mrgl add <zip>",
            "Delta  1.0.0  not checked (rate limit): not asked before GitHub's rate limit",
            "Echo  1.0.0  ignored, not checked",
            "dry run: 1 update available, 1 ignored, 1 not checked, 2 not checked (GitHub's rate limit); nothing written",
        ],
        "{text}"
    );
    assert_eq!(game_snapshot(&game), before, "a dry run writes nothing");
    let api: Vec<String> = std::fs::read_dir(game.home.join("cache/github/example"))
        .expect("the cache")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(api, ["notes"], "only the first addon's list was asked for");
    assert!(!game.home.join("cache/github/questie").exists());

    // The murloc speaks the stop as a warning; the JSON document carries it with the counts.
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["update", "--all", "--dry-run", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    let text = stdout(&output);
    assert!(
        text.contains(&format!("Mrgl? Mrgl…\n{stop}")),
        "the stop is a warning: {text}"
    );
    assert!(
        text.contains("Mrrrgl… mrgl mrgl.\nchecking Alpha against github:Example/Notes (1 of 5)"),
        "{text}"
    );
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args([
            "--json",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["counts"]["rate-limited"], 2);
    assert_eq!(value["counts"]["update-available"], 1);
    assert_eq!(value["stopped"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["stopped"][0]["source"], "github");
    assert_eq!(value["stopped"][0]["host"], "GitHub");
    assert_eq!(value["stopped"][0]["limit"], 60);
    assert_eq!(value["stopped"][0]["sent"], 1);
    assert_eq!(value["stopped"][0]["reset"], reset);
    assert_eq!(value["stopped"][0]["not_checked"], 2);
    assert_eq!(value["stopped"][0]["message"], stop.as_str());
    assert_eq!(value["addons"][1]["state"], "rate-limited");
    assert_eq!(value["addons"][1]["latest"], serde_json::Value::Null);
}

// --- task 087: `update` writes the last-check file the window reads ---

/// The kept answers of the `_forever_` flavor of a game, by addon name: the state and the version each was
/// given for; the file's head is checked on the way.
fn kept_answers(game: &Game) -> std::collections::BTreeMap<String, (String, String)> {
    let path = game.home.join("wowf").join("last-check.json");
    let text = std::fs::read_to_string(&path).expect("the last-check file");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(
        (&value["format"], &value["schema"], &value["flavor"]),
        (
            &serde_json::json!("mrgl-last-check"),
            &serde_json::json!(1),
            &serde_json::json!("wowf")
        ),
        "{text}"
    );
    value["addons"]
        .as_object()
        .expect("the addons")
        .iter()
        .map(|(name, kept)| {
            assert!(kept["checked_at"].is_string(), "{text}");
            (
                name.clone(),
                (
                    kept["state"].as_str().expect("a state").to_string(),
                    kept["version"].as_str().expect("a version").to_string(),
                ),
            )
        })
        .collect()
}

#[test]
fn update_keeps_a_gone_source_for_the_window_until_it_answers_again() {
    let game = managed_game(
        "last-check",
        &[
            ("Gone", "github:Example/Missing", "1.2.3", ""),
            ("Questie", "github:Questie/Questie", "1.2.3", ""),
        ],
    );
    let base = game.root.parent().expect("the base").to_path_buf();
    let kept = |state: &str, version: &str| (state.to_string(), version.to_string());

    // Offline with nothing cached: both checks fail, and both answers are kept.
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", base.join("no-such-stub"))
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
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert_eq!(
        kept_answers(&game),
        std::collections::BTreeMap::from([
            ("Gone".to_string(), kept("failed", "1.2.3")),
            ("Questie".to_string(), kept("failed", "1.2.3")),
        ])
    );

    // Online: the 404 replaces the failed answer, and an addon up to date drops its own.
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        kept_answers(&game),
        std::collections::BTreeMap::from([("Gone".to_string(), kept("source-gone", "1.2.3"))])
    );
    let text =
        std::fs::read_to_string(game.home.join("wowf").join("last-check.json")).expect("the file");
    let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let gone = &value["addons"]["Gone"];
    assert_eq!(gone["source"], "github:Example/Missing");
    assert!(
        gone["detail"]
            .as_str()
            .is_some_and(|detail| detail.ends_with("no such repository, or it is private")),
        "{text}"
    );

    // `update <name>` writes the file too: the source answers again, with the installed version, so the
    // kept answer goes while the version stays the same.
    let stub = base.join("stub-found-back");
    let _ = std::fs::remove_dir_all(&stub);
    copy_tree(&releases(), &stub);
    let repos = stub.join("api.github.com").join("repos");
    std::fs::create_dir_all(repos.join("Example").join("Missing")).expect("the repository");
    std::fs::copy(
        repos.join("Questie").join("Questie").join("releases.json"),
        repos.join("Example").join("Missing").join("releases.json"),
    )
    .expect("the release list");
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args([
            "--serious",
            "update",
            "Gone",
            "--yes",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(
        stdout(&output)
            .lines()
            .any(|line| line == "Gone  1.2.3  up to date"),
        "{}",
        stdout(&output)
    );
    assert_eq!(kept_answers(&game), std::collections::BTreeMap::new());
}

#[test]
fn update_warns_when_the_last_check_file_cannot_be_written_and_goes_on() {
    let game = managed_game(
        "last-check-blocked",
        &[("Gone", "github:Example/Missing", "1.2.3", "")],
    );
    // A folder where the file goes: the write fails, the check stands.
    std::fs::create_dir_all(game.home.join("wowf").join("last-check.json"))
        .expect("the blocking folder");
    let output = run_in(&game, &["update", "--all", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            .starts_with("the answers of this check are not kept for the window: ")),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "dry run: 1 source gone; nothing written"
    );
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

// --- task 066: the newest release with a Forever asset, and a listed version on page two ---

/// A copy of the fixture answers with two more repositories, each with the fixture `Questie` zip behind
/// its Forever asset: `Example/Behind`, whose newest release has a classic zip only, and `Example/Paged`,
/// whose first page holds 30 releases of 2.0.x and whose second page holds v1.0.0.
fn stub_with_older_forever_releases(base: &Path) -> PathBuf {
    let stub = base.join("stub-066");
    let _ = std::fs::remove_dir_all(&stub);
    copy_tree(&releases(), &stub);
    let questie = releases()
        .join("github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip");
    let release = |repo: &str, tag: &str, assets: &[&str]| {
        let assets: Vec<serde_json::Value> = assets
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "browser_download_url":
                        format!("https://github.com/Example/{repo}/releases/download/{tag}/{name}"),
                })
            })
            .collect();
        serde_json::json!({ "tag_name": tag, "draft": false, "prerelease": false, "assets": assets })
    };
    let write = |path: &str, text: String| {
        let path = stub.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the folder");
        std::fs::write(path, text).expect("the answer");
    };
    let serve = |repo: &str, tag: &str, asset: &str| {
        let folder = stub.join(format!("github.com/Example/{repo}/releases/download/{tag}"));
        std::fs::create_dir_all(&folder).expect("the release folder");
        std::fs::copy(&questie, folder.join(asset)).expect("the asset");
    };

    write(
        "api.github.com/repos/Example/Behind/releases.json",
        serde_json::json!([
            release("Behind", "v2.0.0", &["Behind-v2.0.0-classic.zip"]),
            release("Behind", "v1.9.0", &["Behind-v1.9.0-forever.zip"]),
        ])
        .to_string(),
    );
    serve("Behind", "v1.9.0", "Behind-v1.9.0-forever.zip");

    let first: Vec<serde_json::Value> = (0..30)
        .rev()
        .map(|n| {
            let tag = format!("v2.0.{n}");
            release("Paged", &tag, &[&format!("Paged-{tag}-forever.zip")])
        })
        .collect();
    write(
        "api.github.com/repos/Example/Paged/releases.json",
        serde_json::Value::Array(first).to_string(),
    );
    write(
        "api.github.com/repos/Example/Paged/releases.page-2.json",
        serde_json::json!([release("Paged", "v1.0.0", &["Paged-v1.0.0-forever.zip"])]).to_string(),
    );
    serve("Paged", "v1.0.0", "Paged-v1.0.0-forever.zip");
    stub
}

#[test]
fn add_takes_the_newest_release_with_a_forever_asset() {
    let game = Game::new("add-behind");
    let stub = stub_with_older_forever_releases(game.root.parent().expect("the base"));
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args([
            "--serious",
            "add",
            "github:Example/Behind",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.contains("github:Example/Behind v1.9.0 (Behind-v1.9.0-forever.zip")
            && text
                .contains("the newest release with a forever asset, 1 newer release without one"),
        "{text}"
    );
    assert_eq!(
        sidecar(&game)["addons"]["Questie"]["source"],
        "github:Example/Behind"
    );
    assert_eq!(sidecar(&game)["addons"]["Questie"]["version"], "1.9.0");
}

#[test]
fn import_exact_installs_a_listed_version_from_page_two() {
    let game = Game::new("import-exact-paged");
    let stub = stub_with_older_forever_releases(game.root.parent().expect("the base"));
    let list = write_list(
        &game,
        "paged.json",
        &[("Questie", "github:Example/Paged", "1.0.0")],
    );
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever", "--exact"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines()
            .any(|line| line == "Questie  installed 1.0.0 from github:Example/Paged"),
        "{text}"
    );
    assert_eq!(
        stdout_last_line(&output),
        "imported 1 addon of paged.json into _forever_: 1 installed"
    );
    assert_eq!(sidecar(&game)["addons"]["Questie"]["version"], "1.0.0");
    assert!(game.addon("Questie").join("Questie.toc").is_file());
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
            "set everyday names addons that are not installed in _forever_, kept in the set all the same: BagSorter, RaidFrames",
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
    assert_eq!(
        value["not_installed"],
        serde_json::json!(["BagSorter", "RaidFrames"])
    );

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

// Task 073: `set undo`, `set enable|disable|drop`, `--account` and the save warning, over the same
// `addon-sets` tree.

#[test]
fn set_undo_puts_the_last_apply_back_byte_for_byte() {
    let (root, home) = temp_game_root_with_sets("undo");
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

    let output = set_run(&root, &home, &["--serious", "set", "apply", "raid"]);
    assert!(output.status.success(), "{output:?}");
    assert_ne!(snapshot(&wtf), before);
    assert!(
        home.join("wowf")
            .join("set-undo")
            .join("undo.json")
            .is_file()
    );

    let output = set_run(&root, &home, &["--serious", "set", "undo"]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(
        lines[0],
        "Second Realm/Grrlwave: AddOns.txt put back as it was"
    );
    assert!(
        lines[1].starts_with("undid the apply of set raid to _forever_ (20"),
        "{text}"
    );
    assert!(lines[1].ends_with("): 1 AddOns.txt put back"), "{text}");
    assert_eq!(snapshot(&wtf), before, "byte for byte");

    let output = set_run(&root, &home, &["--serious", "set", "undo"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "nothing to undo in _forever_: no set apply is recorded; an apply that changes a file keeps what it replaced, and an undo puts it back once"
    );

    let output = set_run(&root, &home, &["--json", "set", "apply", "raid"]);
    assert!(output.status.success(), "{output:?}");
    let output = set_run(&root, &home, &["--json", "set", "undo"]);
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "set-undo");
    assert_eq!(value["name"], "raid");
    assert_eq!(value["restored"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["restored"][0]["name"], "Grrlwave");
    assert_eq!(value["removed"], serde_json::json!([]));
    assert_eq!(value["warnings"], serde_json::json!([]));
    assert_eq!(snapshot(&wtf), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn set_undo_is_refused_while_the_client_runs() {
    let (root, home) = temp_game_root_with_sets("undo-client");
    let wtf = root.join("_forever_").join("WTF");
    let output = set_run(&root, &home, &["--serious", "set", "save", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let before = snapshot(&wtf);
    let output = set_run(&root, &home, &["--serious", "set", "apply", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let applied = snapshot(&wtf);
    let Some(mut child) = spawn_stand_in(&root.join("_forever_")) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let output = set_run(&root, &home, &["--serious", "set", "undo"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf), applied, "nothing was written");
    child.stop();
    let output = set_run(&root, &home, &["--serious", "set", "undo"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(snapshot(&wtf), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn set_enable_disable_and_drop_edit_one_addon_of_a_stored_set() {
    let (root, home) = temp_game_root_with_sets("edit");
    let wtf = root.join("_forever_").join("WTF");
    std::fs::create_dir_all(root.join("_forever_/Interface/AddOns/BagSorter")).expect("addon");
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

    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "disable", "raid", "bagsorter"],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "BagSorter is disabled in set raid of _forever_ now, was enabled: 4 addons (2 enabled, 2 disabled); no AddOns.txt was touched"
        ]
    );
    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "disable", "raid", "BagSorter"],
    );
    assert_eq!(
        stdout_last_line(&output),
        "set raid of _forever_ has BagSorter disabled already, nothing changed"
    );
    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "enable", "raid", "NewMeter"],
    );
    assert_eq!(
        stdout(&output).lines().collect::<Vec<_>>(),
        [
            "NewMeter is not installed in _forever_; the set keeps it all the same",
            "added NewMeter to set raid of _forever_, enabled: 5 addons (3 enabled, 2 disabled); no AddOns.txt was touched",
        ]
    );
    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "drop", "raid", "QuestArrows"],
    );
    assert_eq!(
        stdout_last_line(&output),
        "dropped QuestArrows from set raid of _forever_: 4 addons (3 enabled, 1 disabled); no AddOns.txt was touched"
    );
    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "drop", "raid", "QuestArrows"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "set raid (flavor forever) has no addon QuestArrows; its addons: BagSorter, BossTimers, NewMeter, RaidFrames"
    );

    let output = set_run(
        &root,
        &home,
        &["--json", "set", "enable", "raid", "BagSorter"],
    );
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "set-edit");
    assert_eq!(value["name"], "raid");
    assert_eq!(value["addon"], "BagSorter");
    assert_eq!(value["action"], "enable");
    assert_eq!(value["before"], "disabled");
    assert_eq!(value["after"], "enabled");
    assert_eq!(value["changed"], true);
    assert_eq!(value["installed"], true);
    assert_eq!(
        value["set"]["addons"],
        serde_json::json!({
            "BagSorter": "enabled",
            "BossTimers": "enabled",
            "NewMeter": "enabled",
            "RaidFrames": "enabled",
        })
    );
    assert_eq!(snapshot(&wtf), before, "an edit never touches WTF");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn set_account_picks_a_character_under_two_accounts() {
    let (root, home) = temp_game_root_with_sets("account");
    let wtf = root.join("_forever_").join("WTF");
    let twin = "Account/MRGLTWO/Fixture Realm/Mrgltest/AddOns.txt";
    std::fs::create_dir_all(wtf.join("Account/MRGLTWO/Fixture Realm/Mrgltest")).expect("a twin");
    std::fs::write(wtf.join(twin), b"BagSorter: enabled\r\n").expect("its file");
    let output = set_run(&root, &home, &["--serious", "set", "save", "everyday"]);
    assert!(output.status.success(), "{output:?}");
    let mut file: serde_json::Value = serde_json::from_slice(
        &std::fs::read(home.join("wowf").join("sets.json")).expect("sets.json"),
    )
    .expect("JSON");
    file["sets"]["raid"] = serde_json::json!({
        "saved_at": "2026-10-22T00:00:00Z",
        "addons": { "BagSorter": "disabled" },
    });
    std::fs::write(home.join("wowf").join("sets.json"), file.to_string()).expect("written");
    let before = snapshot(&wtf);

    let args = |extra: &[&'static str]| {
        let mut args = vec![
            "--serious",
            "set",
            "apply",
            "raid",
            "--character",
            "Mrgltest",
            "--realm",
            "Fixture Realm",
        ];
        args.extend_from_slice(extra);
        args
    };
    let output = set_run(&root, &home, &args(&[]));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "character Mrgltest matches 2 characters (flavor forever): Fixture Realm (MRGLSETS), Fixture Realm (MRGLTWO); the same realm is under more than one account: pick the account too, with --account"
    );
    assert_eq!(snapshot(&wtf), before, "nothing was written");

    let output = set_run(&root, &home, &args(&["--account", "mrgltwo"]));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_last_line(&output),
        "applied set raid to Fixture Realm/Mrgltest of _forever_: 1 addon line changed"
    );
    let after = snapshot(&wtf);
    assert_eq!(after[SETS_FIRST], before[SETS_FIRST], "the other account");
    assert_eq!(after[twin], b"BagSorter: disabled\r\n");

    let output = set_run(
        &root,
        &home,
        &[
            "--json",
            "set",
            "save",
            "twin",
            "--character",
            "Mrgltest",
            "--account",
            "MRGLTWO",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["characters"][0]["account"], "MRGLTWO");
    assert_eq!(
        value["set"]["addons"],
        serde_json::json!({ "BagSorter": "disabled" })
    );

    let output = set_run(
        &root,
        &home,
        &["--serious", "set", "apply", "raid", "--account", "MRGLTWO"],
    );
    assert_eq!(output.status.code(), Some(2), "--account needs --character");
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

// --- task 075: the output survives a closed pipe ---

#[test]
fn list_json_into_a_reader_that_closes_early_ends_quietly_with_exit_0() {
    use std::io::Read;
    // Enough addons that the JSON document outgrows the pipe's buffer, so `mrgl` is still writing when the
    // reader goes, as `mrgl list --json | head -c 200` does.
    let game = Game::new("closed-pipe");
    for index in 0..600 {
        game.by_hand(&format!("ByHand{index:04}"));
    }
    let mut child = game
        .mrgl()
        .args(["--json", "list", "--flavor", "forever"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run mrgl");
    let mut start = [0u8; 200];
    child
        .stdout
        .take()
        .expect("the stdout pipe")
        .read_exact(&mut start)
        .expect("the start of the document");
    assert!(
        start.starts_with(b"{"),
        "{}",
        String::from_utf8_lossy(&start)
    );
    // The reader is dropped here: the pipe closes with the rest of the document unwritten.
    let output = child.wait_with_output().expect("mrgl ends");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(stderr, "", "nothing is said about the closed pipe");
}

// --- task 084: the addon in flight finishes before a closed pipe ends the run ---

#[test]
fn add_into_a_closed_pipe_installs_the_addon_in_flight_and_leaves_no_staging_folder() {
    let game = Game::new("closed-pipe-install");
    // The reader is gone before `mrgl` writes its first line, the `resolving` line inside the install: the
    // run goes on quietly until the addon is installed, then ends with 0.
    let (reader, writer) = std::io::pipe().expect("a pipe");
    drop(reader);
    let output = game
        .mrgl()
        .args(["add", "github:Questie/Questie", "--flavor", "forever"])
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("run mrgl");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stderr, "", "nothing is said about the closed pipe");
    assert!(
        game.addon("Questie").join("Questie.toc").is_file(),
        "the addon in flight is installed"
    );
    let state = std::fs::read_to_string(game.state_path()).expect("the sidecar is written");
    assert!(state.contains("github:Questie/Questie"), "{state}");
    let staging = game.home.join("wowf").join("staging");
    let left: Vec<String> = std::fs::read_dir(&staging)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(left, Vec::<String>::new(), "no staging folder is left");
}

/// Task 079: `mrgl.exe` carries the workspace version in its file properties, the `VS_VERSION_INFO`
/// resource `build.rs` writes next to the icon, read back from the built exe by the core's own PE reader.
#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn the_exe_carries_the_workspace_version_in_its_properties() {
    let number = |text: &str| text.parse::<u32>().expect("a version number");
    let expected = mrgl_core::flavor::BuildVersion {
        major: number(env!("CARGO_PKG_VERSION_MAJOR")),
        minor: number(env!("CARGO_PKG_VERSION_MINOR")),
        patch: number(env!("CARGO_PKG_VERSION_PATCH")),
        build: 0,
    };
    let read = mrgl_core::pe::read_file_version(Path::new(env!("CARGO_BIN_EXE_mrgl")))
        .expect("the exe has a version resource");
    assert_eq!(read, expected);
    assert_eq!(mrgl_core::VERSION, env!("CARGO_PKG_VERSION"), "one version");
}

// --- task 094: the WoWInterface source against the fixture answers under `sources/` ---

/// The fixture answers of the WoWInterface source (task 094), served by the folder stub.
fn sources_stub() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/sources")
}

impl Game {
    /// `mrgl` in this game with the stub of the sources of task 094 instead of the GitHub one.
    fn mrgl_sources(&self) -> Command {
        let mut command = self.mrgl();
        command.env("MRGL_HTTP_STUB", sources_stub());
        command
    }
}

#[test]
fn add_wowi_installs_the_projects_one_file_from_the_stub() {
    let game = Game::new("wowi-add");
    let output = game
        .mrgl_sources()
        .args(["--serious", "add", "wowi:1001", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "resolving wowi:1001 for forever",
            "downloading MrglCompat-2.0.0.zip",
            "installing MrglCompat-2.0.0.zip into _forever_",
            "pick for forever, a guess: wowi:1001 v2.0.0 (MrglCompat-2.0.0.zip, flavor forever): WoWInterface lists no forever compatibility (it lists 12.0.7, 1.15.7, 4.4.2); the project's one file is taken, a guess",
            "forever build: unknown (WoWInterface lists no Forever compatibility)",
            "installed MrglCompat 2.0.0 (1 folder: MrglCompat) into _forever_",
        ],
        "{text}"
    );
    assert!(game.addon("MrglCompat").join("MrglCompat.toc").is_file());
    let state = sidecar(&game);
    assert_eq!(state["addons"]["MrglCompat"]["source"], "wowi:1001");
    assert_eq!(state["addons"]["MrglCompat"]["version"], "2.0.0");
    assert!(
        game.home.join("cache/wowi/1001.json").is_file(),
        "the answer is cached"
    );
    assert!(game.home.join("cache/wowi/filelist.json").is_file());

    // A page address reads the same id; the dry run with the Forever compatibility is a metadata pick.
    let output = game
        .mrgl_sources()
        .args([
            "--serious",
            "add",
            "https://www.wowinterface.com/downloads/info1005-MrglForever.html",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "resolving wowi:1005 for forever",
            "pick for forever: wowi:1005 3.1.0 (MrglForever-3.1.0.zip, flavor forever): WoWInterface lists compatibility 1.60.1 (Made-up Forever Patch)",
            "release notes: 3.1.0 (https://www.wowinterface.com/downloads/info1005-MrglForever.html)",
            "forever build: yes (WoWInterface lists a Forever compatibility)",
            "dry run: nothing installed",
        ],
        "{text}"
    );
    let json = game
        .mrgl_sources()
        .args([
            "--json",
            "add",
            "wowi:1005",
            "--flavor",
            "forever",
            "--dry-run",
        ])
        .output()
        .expect("run mrgl");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("one JSON document");
    assert_eq!(value["pick"]["md5"], "c4839676a8a74ca3b0e44146b54a3477");
    assert_eq!(value["pick"]["guess"], false);
    assert_eq!(value["pick"]["forever_build"], "yes");
}

#[test]
fn add_wowi_reports_a_pending_file_and_refuses_a_wrong_md5_before_the_zip_opens() {
    let game = Game::new("wowi-refused");
    for dry_run in [true, false] {
        let mut command = game.mrgl_sources();
        command.args(["--serious", "add", "wowi:1003", "--flavor", "forever"]);
        if dry_run {
            command.arg("--dry-run");
        }
        let output = command.output().expect("run mrgl");
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            stderr_last_line(&output),
            "the newest file of wowi:1003 waits for approval on WoWInterface: nothing is installed until it is approved"
        );
    }
    let output = game
        .mrgl_sources()
        .args(["--serious", "add", "wowi:1004", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "refused MrglBadSum-1.0.0.zip of wowi:1004 1.0.0: its MD5 is bb4291ce1f730854f286d0707adf9a9d, the source names 0123456789abcdef0123456789abcdef; the download was damaged or the file changed, try again"
    );
    assert!(!game.addon("MrglBadSum").exists(), "nothing installed");
    assert!(!game.state_path().exists(), "no sidecar written");
    assert_eq!(staging_entries(&game), Vec::<String>::new());
}

#[test]
fn import_and_update_take_a_wowi_entry() {
    let game = Game::new("wowi-import");
    let base = game.root.parent().expect("the base").to_path_buf();
    let list = base.join("wowi.json");
    std::fs::write(
        &list,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [
                { "name": "MrglForever", "source": "wowi:1005", "version": "3.1.0",
                  "folders": ["MrglForever"] },
            ],
        })
        .to_string(),
    )
    .expect("the list");
    let output = game
        .mrgl_sources()
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let state = sidecar(&game);
    assert_eq!(state["addons"]["MrglForever"]["source"], "wowi:1005");
    assert_eq!(state["addons"]["MrglForever"]["version"], "3.1.0");
    assert!(game.addon("MrglForever").join("MrglForever.toc").is_file());

    // The update check asks WoWInterface: a metadata pick is an update, a guess is reported, a pending
    // file is a state of its own (task 104), not a failure: exit code 0.
    let game = managed_game(
        "wowi-update",
        &[
            ("MrglCompat", "wowi:1001", "1.9.0", ""),
            ("MrglForever", "wowi:1005", "3.0.0", ""),
            ("MrglPending", "wowi:1003", "1.0.0", ""),
        ],
    );
    let output = game
        .mrgl_sources()
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "checking the managed addons of _forever_ against their sources",
            "checking MrglCompat against wowi:1001 (1 of 3)",
            "MrglCompat  1.9.0 -> 2.0.0  guess only: WoWInterface lists no forever compatibility (it lists 12.0.7, 1.15.7, 4.4.2); the project's one file is taken, a guess; `mrgl add wowi:1001` installs it anyway",
            "checking MrglForever against wowi:1005 (2 of 3)",
            "MrglForever  3.0.0 -> 3.1.0  update available",
            "checking MrglPending against wowi:1003 (3 of 3)",
            "MrglPending  1.0.0  pending approval: the newest file waits for approval on WoWInterface; nothing to install until it is approved",
            "dry run: 1 update available, 1 pending approval, 1 guess only; nothing written",
        ],
        "{text}"
    );
    let output = game
        .mrgl_sources()
        .args([
            "--json",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(value["addons"][2]["name"], "MrglPending");
    assert_eq!(value["addons"][2]["state"], "pending");
    assert_eq!(
        value["addons"][2]["detail"],
        "the newest file waits for approval on WoWInterface; nothing to install until it is approved"
    );
    assert_eq!(value["counts"]["pending"], 1);
    assert_eq!(value["counts"]["failed"], 0);
    // Pending entries alone end with exit code 0; a failed check next to them still ends with 1.
    let pending = managed_game(
        "wowi-pending",
        &[
            ("MrglPending", "wowi:1003", "1.0.0", ""),
            ("Refused", "wago:MrglWg04", "1.0.0", ""),
        ],
    );
    let output = pending
        .mrgl_sources()
        .args([
            "--serious",
            "update",
            "--all",
            "--yes",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        stdout_last_line(&output),
        "nothing updated: 1 pending approval, 1 not checked"
    );
    let output = pending
        .mrgl_wago()
        .args([
            "--serious",
            "update",
            "--all",
            "--yes",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "1 addon of _forever_ failed: Refused; the other addons are done"
    );
    assert!(
        stdout(&output)
            .lines()
            .any(|line| line.starts_with("MrglPending  1.0.0  pending approval: ")),
        "{}",
        stdout(&output)
    );
    let output = game
        .mrgl_sources()
        .args([
            "--serious",
            "update",
            "MrglForever",
            "--yes",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(sidecar(&game)["addons"]["MrglForever"]["version"], "3.1.0");
}

// --- task 095: the Wago source against the fixture answers under `sources/addons.wago.io/` ---

/// A made-up Wago key: no test reads or uses a real one.
const WAGO_TEST_KEY: &str = "made-up-wago-key-for-the-cli-tests";

impl Game {
    /// The same game with a `_retail_` flavor next to `_forever_`, at build 12.1.5.
    fn with_retail(self) -> Game {
        std::fs::write(
            self.root.join(".build.info"),
            "Version!STRING:0|Product!STRING:0\n1.60.1.70245|wowf\n12.1.5.70448|wow\n",
        )
        .expect(".build.info");
        std::fs::create_dir_all(self.root.join("_retail_")).expect("the flavor folder");
        std::fs::write(
            self.root.join("_retail_").join(".flavor.info"),
            "Product Flavor!STRING:0\nwow\n",
        )
        .expect(".flavor.info");
        self
    }

    /// `mrgl` against the sources stub with the made-up Wago key.
    fn mrgl_wago(&self) -> Command {
        let mut command = self.mrgl_sources();
        command.env("MRGL_WAGO_KEY", WAGO_TEST_KEY);
        command
    }
}

#[test]
fn add_wago_installs_the_forever_and_the_retail_answer_of_one_url() {
    let game = Game::new("wago-add").with_retail();
    let output = game
        .mrgl_wago()
        .args(["--serious", "add", "wago:MrglWg01", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "resolving wago:MrglWg01 for forever",
            "pick for forever: wago:MrglWg01 2.0.0 (mrgl-wago-one-2.0.0.zip, flavor forever, interface 16001): Wago lists a stable release for forever (patch 1.60.1)",
            "downloading mrgl-wago-one-2.0.0.zip",
            "installing mrgl-wago-one-2.0.0.zip into _forever_",
            "forever build: yes (Wago lists a release for forever)",
            "installed MrglWagoOne 2.0.0 (1 folder: MrglWagoOne) into _forever_",
        ],
        "{text}"
    );
    let forever_toc = std::fs::read_to_string(game.addon("MrglWagoOne").join("MrglWagoOne.toc"))
        .expect("the Forever TOC");
    assert!(forever_toc.contains("## Interface: 16001"), "{forever_toc}");
    assert_eq!(
        sidecar(&game)["addons"]["MrglWagoOne"]["source"],
        "wago:MrglWg01"
    );

    let output = game
        .mrgl_wago()
        .args(["--serious", "add", "wago:MrglWg01", "--flavor", "retail"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "pick for retail: wago:MrglWg01 2.0.0 (mrgl-wago-one-2.0.0.zip, flavor retail, interface 120105): Wago lists a stable release for retail (patch 12.1.5)"),
        "{text}"
    );
    let retail_toc = std::fs::read_to_string(
        game.root
            .join("_retail_/Interface/AddOns/MrglWagoOne/MrglWagoOne.toc"),
    )
    .expect("the retail TOC");
    assert!(retail_toc.contains("## Interface: 120105"), "{retail_toc}");
    let retail_state: serde_json::Value = serde_json::from_slice(
        &std::fs::read(game.home.join("wow").join("state.json")).expect("the retail sidecar"),
    )
    .expect("JSON");
    assert_eq!(
        retail_state["addons"]["MrglWagoOne"]["source"],
        "wago:MrglWg01"
    );
    // The key reaches no output and no file under the home folder.
    assert!(!all_output(&output).contains(WAGO_TEST_KEY));
    for (path, bytes) in snapshot(&game.home) {
        assert!(
            !String::from_utf8_lossy(&bytes).contains(WAGO_TEST_KEY),
            "{path}"
        );
    }
}

#[test]
fn add_wago_names_the_key_on_a_401() {
    let game = Game::new("wago-401");
    let output = game
        .mrgl_wago()
        .args(["--serious", "add", "wago:MrglWg04", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "Wago answered 401 for https://addons.wago.io/api/external/addons/MrglWg04?game_version=forever: MRGL_WAGO_KEY is missing or not accepted: the Wago API needs your own key, `mrgl key set wago` stores it"
    );
    assert!(!all_output(&output).contains(WAGO_TEST_KEY));
}

#[test]
fn import_takes_a_wago_entry_and_update_stops_before_wagos_rate_limit() {
    let game = Game::new("wago-import");
    let list = game.root.parent().expect("the base").join("wago.json");
    std::fs::write(
        &list,
        serde_json::json!({
            "format": "mrgl-list",
            "schema": 1,
            "flavor": "wowf",
            "addons": [
                { "name": "MrglWagoOne", "source": "wago:MrglWg01", "version": "2.0.0",
                  "folders": ["MrglWagoOne"] },
            ],
        })
        .to_string(),
    )
    .expect("the list");
    let output = game
        .mrgl_wago()
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(sidecar(&game)["addons"]["MrglWagoOne"]["version"], "2.0.0");
    assert!(game.addon("MrglWagoOne").join("MrglWagoOne.toc").is_file());

    // The bulk answer (task 106) holds One and Three; Five is asked alone and its answer says no request
    // is left, so Two, which the bulk answer lacks, is not asked: the check stops before Wago's limit.
    let game = managed_game(
        "wago-budget",
        &[
            ("Five", "wago:MrglWg05", "0.9.0", ""),
            ("One", "wago:MrglWg01", "1.0.0", ""),
            ("Three", "wago:MrglWg03", "3.0.0", ""),
            ("Two", "wago:MrglWg02", "1.0.0", ""),
        ],
    );
    let output = game
        .mrgl_wago()
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "checking the managed addons of _forever_ against their sources",
            "checking Five against wago:MrglWg05 (1 of 4)",
            "Five  0.9.0 -> 1.0.0  update available",
            "checking One against wago:MrglWg01 (2 of 4)",
            "One  1.0.0 -> 2.0.0  update available",
            "checking Three against wago:MrglWg03 (3 of 4)",
            "Three  3.0.0 -> 3.2.0-alpha  update available",
            "stopped asking Wago before its rate limit of 60 requests: 1 addon not checked, the limit resets after a short wait (Wago does not say when); run the check again then",
            "Two  1.0.0  not checked (rate limit): not asked before Wago's rate limit",
            "dry run: 3 updates available, 1 not checked (Wago's rate limit); nothing written",
        ],
        "{text}"
    );
    // Without the key the entries are not checked and say what is missing.
    let output = game
        .mrgl_sources()
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "One  1.0.0  not checked: source not configured: wago:MrglWg01 (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)"),
        "{text}"
    );
}

// --- task 101: the update check stops per source at a rate limit ---

impl Game {
    /// `mrgl` against one stub of the GitHub answers and the sources of tasks 094 and 095 together, with the
    /// made-up Wago key.
    fn mrgl_every_source(&self) -> Command {
        let stub = self.home.join("stub-every-source");
        if !stub.is_dir() {
            copy_tree(&releases(), &stub);
            copy_tree(&sources_stub(), &stub);
        }
        let mut command = self.mrgl();
        command
            .env("MRGL_HTTP_STUB", &stub)
            .env("MRGL_WAGO_KEY", WAGO_TEST_KEY);
        command
    }
}

#[test]
fn update_stops_asking_wago_at_its_rate_limit_and_goes_on_asking_github() {
    // The first Wago answer says no request is left: the Wago entry after it is not asked, the GitHub entry
    // after that is.
    let game = managed_game(
        "budget-per-source",
        &[
            ("Five", "wago:MrglWg05", "1.0.0", ""),
            ("Late", "wago:MrglWg02", "1.0.0", ""),
            ("Questie", "github:Questie/Questie", "1.2.2", ""),
        ],
    );
    let stop = "stopped asking Wago before its rate limit of 60 requests: 1 addon not checked, the limit resets after a short wait (Wago does not say when); run the check again then";
    let output = game
        .mrgl_every_source()
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "checking the managed addons of _forever_ against their sources",
            "checking Five against wago:MrglWg05 (1 of 3)",
            "Five  1.0.0  up to date",
            stop,
            "Late  1.0.0  not checked (rate limit): not asked before Wago's rate limit",
            "checking Questie against github:Questie/Questie (3 of 3)",
            "Questie  1.2.2 -> 1.2.3  update available",
            "dry run: 1 update available, 1 up to date, 1 not checked (Wago's rate limit); nothing written",
        ],
        "{text}"
    );

    // The JSON names the source of the stop, and the GitHub entry as checked.
    let output = game
        .mrgl_every_source()
        .args([
            "--json",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let states: Vec<(&str, &str)> = value["addons"]
        .as_array()
        .expect("the addons")
        .iter()
        .map(|addon| {
            (
                addon["name"].as_str().unwrap_or_default(),
                addon["state"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            ("Five", "up-to-date"),
            ("Late", "rate-limited"),
            ("Questie", "update-available"),
        ]
    );
    assert_eq!(value["counts"]["rate-limited"], 1);
    assert_eq!(value["stopped"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["stopped"][0]["source"], "wago");
    assert_eq!(value["stopped"][0]["host"], "Wago");
    assert_eq!(value["stopped"][0]["not_checked"], 1);
    assert_eq!(value["stopped"][0]["message"], stop);
    assert_eq!(
        value["addons"][1]["detail"],
        "not asked before Wago's rate limit"
    );

    // `update --all` agrees: the GitHub update goes in, the Wago entry stays not checked, exit code 0.
    let output = game
        .mrgl_every_source()
        .args([
            "--serious",
            "update",
            "--all",
            "--yes",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(text.lines().any(|line| line == stop), "{text}");
    assert_eq!(
        stdout_last_line(&output),
        "1 addon updated, 1 up to date, 1 not checked (Wago's rate limit)"
    );
    assert_eq!(sidecar(&game)["addons"]["Questie"]["version"], "1.2.3");
    assert_eq!(sidecar(&game)["addons"]["Late"]["version"], "1.0.0");
}

// --- task 106: the Wago update check asks in bulk ---

#[test]
fn update_all_takes_the_wago_answers_from_one_bulk_request() {
    // A stub without the single answers of MrglWg01 and MrglWg03 for Forever: the check still answers
    // both, from the bulk answer, and writes them into the same cache files.
    let game = managed_game(
        "wago-bulk",
        &[
            ("One", "wago:MrglWg01", "1.0.0", ""),
            ("Three", "wago:MrglWg03", "3.0.0", ""),
        ],
    );
    let stub = game.home.join("stub-bulk");
    copy_tree(&sources_stub(), &stub);
    for id in ["MrglWg01", "MrglWg03"] {
        std::fs::remove_file(stub.join(format!(
            "addons.wago.io/api/external/addons/{id}@game_version=forever.json"
        )))
        .expect("the single answer goes");
    }
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .env("MRGL_WAGO_KEY", WAGO_TEST_KEY)
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
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "checking the managed addons of _forever_ against their sources",
            "checking One against wago:MrglWg01 (1 of 2)",
            "One  1.0.0 -> 2.0.0  update available",
            "checking Three against wago:MrglWg03 (2 of 2)",
            "Three  3.0.0 -> 3.2.0-alpha  update available",
            "dry run: 2 updates available; nothing written",
        ],
        "{text}"
    );
    for id in ["MrglWg01", "MrglWg03"] {
        let cached: serde_json::Value = serde_json::from_slice(
            &std::fs::read(game.home.join(format!("cache/wago/{id}.json"))).expect("the cache"),
        )
        .expect("JSON");
        assert_eq!(cached["answers"]["forever"]["addon"]["id"], id);
    }
    // `update <name>` keeps the single request, which this stub no longer answers.
    let output = game
        .mrgl()
        .env("MRGL_HTTP_STUB", &stub)
        .env("MRGL_WAGO_KEY", WAGO_TEST_KEY)
        .args([
            "--serious",
            "update",
            "One",
            "--dry-run",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert!(
        stdout(&output).lines().any(|line| line
            == "One  1.0.0  source gone: Wago answered 404 for https://addons.wago.io/api/external/addons/MrglWg01?game_version=forever: no such addon, or it was removed"),
        "{}",
        stdout(&output)
    );
}
