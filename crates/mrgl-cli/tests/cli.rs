//! End-to-end tests of the `mrgl` exe: the voice's two-line rule, `--serious`, `MRGL_SERIOUS`, `--json`, the
//! exit codes, and `scan`, `list` and `root` over the synthetic flavors tree of `mrgl-core`. The tests assert
//! the plain line (docs/nerglish.md, rule 3); the murloc line is checked only for being there. `MRGL_HOME`
//! points every run at a temporary folder, so no test reads or writes the real config, and
//! `MRGL_DISCOVERY` is `off` or a stub (task 020), so no test reads the real registry or looks under
//! `Program Files`. The running-client tests (task 023) copy this test binary into a temporary flavor
//! folder and start it as a stand-in client, never a game file. The `backup` tests (task 013) copy the
//! synthetic fixture `WTF` tree into a temporary flavor folder and never touch a real one.

use std::io::Read;
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
        .args(["link", "SomeFolder", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "link / unlink is not implemented yet (task 005)"
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
        "1 addon in _classic_ (wow_classic 5.5.4.70531)"
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
        plain.starts_with("no game root for flavor era: none in the launcher's registry key"),
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
        "no game root: none remembered, none in the launcher's registry key or the usual install folders; run `mrgl root <folder>`"
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
    let mut child = spawn_stand_in(&game.root.join("_forever_"));
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

    child.kill().expect("killed");
    child.wait().expect("waited");
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
fn an_unknown_source_spec_and_update_are_plain_errors() {
    let output = mrgl()
        .args(["add", "wago:something", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "unknown source: wago:something (expected github:<owner>/<repo>)"
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

#[test]
fn add_without_dry_run_reports_the_pick_then_names_task_040() {
    let output = mrgl()
        .args([
            "--serious",
            "add",
            "github:Questie/Questie",
            "--flavor",
            "forever",
        ])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout_last_line(&output).starts_with("pick for forever: github:Questie/Questie v1.2.3"),
        "{}",
        stdout(&output)
    );
    assert_eq!(
        stderr_last_line(&output),
        "add (the install) is not implemented yet (task 040)"
    );
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

    let mut child = spawn_stand_in(&flavor);
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

    child.kill().expect("killed");
    child.wait().expect("waited");
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
            "LibMet  1.0.0  16001",
            "Needy  1.0.0  16001  missing: LibMissing",
            "NolibBuild  1.0.0  16001",
            "Needy needs the missing dependency LibMissing: install it or the addon does not load",
            "3 addons in _forever_ (wowf 1.60.1.70245)",
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
            "required": ["LibMet", "LibEmbedded", "LibMissing"],
            "met": ["LibMet"],
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
            .any(|line| line == "NolibBuild  1.0.0  16001  missing: LibEmbedded  forever:unknown"),
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
    assert!(
        lines[1].starts_with("root: ") && lines[1].ends_with(&format!("{root_name} (from --root)")),
        "{stdout}"
    );
    assert!(
        lines[2].starts_with("home: ") && lines[2].ends_with("doctor-lines"),
        "{stdout}"
    );
    let rest = [
        "webview2: 131.0.2903.70 (per-machine)",
        "github rate limit: 58 of 60 left, resets 2026-10-21T16:00:00Z",
        "api keys: MRGL_GITHUB_TOKEN set",
        "flavor: _forever_ (wowf)",
    ];
    assert_eq!(lines[3..7], rest, "{stdout}");
    assert!(
        lines[7].starts_with("  folder: ") && lines[7].ends_with("_forever_"),
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
    assert_eq!(lines[8..], flavor, "{stdout}");
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
    assert_eq!(lines[1], "root: none found");
    assert_eq!(lines[3], "webview2: unknown");
    assert_eq!(lines[4], "github rate limit: unknown (HTTP 404)");
    assert_eq!(lines[5], "api keys: MRGL_GITHUB_TOKEN not set");
    // The summary speaks through the voice: a murloc line, then the plain one.
    assert_eq!(lines.len(), 8, "{stdout}");
    assert!(lines[6].starts_with("Mrgl"), "{stdout}");
    assert_eq!(
        lines[7],
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
