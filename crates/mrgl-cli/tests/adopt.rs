//! End-to-end tests of `mrgl adopt` (tasks 097, 098) over a copy of the synthetic game folder
//! `mrgl-core/tests/fixtures/adopt/`: the report of every header shape against the folder stub of the
//! GitHub, WoWInterface and Wago answers (the fixture `releases/` and `sources/` trees copied into one
//! temporary folder), the `.curseclient` marker, the JSON document and the Wago key; `--apply` with the
//! sidecar it writes, the update over an adopted addon, a second apply, the marker guard with and without
//! `--alongside-curseforge`, and the warning of `add`, `import` and `update` in a marked folder. Every run points
//! `MRGL_HOME` and `MRGL_ROOT` at a temporary folder, turns the key store off and reads no registry, as
//! `tests/cli.rs` does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The made-up Wago key of these tests; no output may carry it.
const WAGO_TEST_KEY: &str = "made-up-wago-key-for-the-adopt-tests";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures")
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn all_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stderr_last_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

/// A copy of the adopt fixture's game folder, the stub of every source and a home, all temporary.
struct Game {
    base: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let base =
            std::env::temp_dir().join(format!("mrgl-cli-adopt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        copy_tree(&fixtures().join("adopt"), &base.join("game"));
        copy_tree(&fixtures().join("releases"), &base.join("stub"));
        copy_tree(&fixtures().join("sources"), &base.join("stub"));
        Game { base }
    }

    fn root(&self) -> PathBuf {
        self.base.join("game")
    }

    fn home(&self) -> PathBuf {
        self.base.join("home")
    }

    fn state_path(&self, flavor_id: &str) -> PathBuf {
        self.home().join(flavor_id).join("state.json")
    }

    fn sidecar(&self, flavor_id: &str) -> serde_json::Value {
        let text = std::fs::read_to_string(self.state_path(flavor_id)).expect("the sidecar");
        serde_json::from_str(&text).expect("the sidecar parses")
    }

    fn addon(&self, flavor_folder: &str, folder: &str) -> PathBuf {
        self.root()
            .join(flavor_folder)
            .join("Interface")
            .join("AddOns")
            .join(folder)
    }

    /// `mrgl` in this game, every source answered by the stub, no key set.
    fn mrgl(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mrgl"));
        command.env_remove("MRGL_SERIOUS");
        command.env_remove("MRGL_GITHUB_TOKEN");
        command.env_remove("MRGL_WAGO_KEY");
        command.env_remove("MRGL_CURSEFORGE_KEY");
        command.env("MRGL_KEY_STORE", "off");
        command.env("MRGL_DISCOVERY", "off");
        command.env("MRGL_HTTP_STUB", self.base.join("stub"));
        command.env("MRGL_HOME", self.home());
        command.env("MRGL_ROOT", self.root());
        command
    }

    /// The same with the made-up Wago key.
    fn mrgl_wago(&self) -> Command {
        let mut command = self.mrgl();
        command.env("MRGL_WAGO_KEY", WAGO_TEST_KEY);
        command
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

#[test]
fn adopt_reports_every_header_shape_against_the_stubs() {
    let game = Game::new("report");
    let output = game
        .mrgl()
        .args(["--serious", "adopt", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "MrglCurseOnly 0.4.2: only with CurseForge, needs MRGL_CURSEFORGE_KEY",
            "  curse:4242 from X-Curse-Project-ID: needs MRGL_CURSEFORGE_KEY",
            "MrglElsewhere 0.1 (2 folders: MrglElsewhere, MrglElsewhere_Options): adopt as wowi:1006",
            "  wowi:1006 from X-WoWI-ID: forever build unknown (WoWInterface lists no forever compatibility (it lists 11.2.7))",
            "MrglForever 3.0.0: adopt as wowi:1005",
            "  wowi:1005 from X-WoWI-ID: forever build yes (WoWInterface lists compatibility 1.60.1 (Made-up Forever Patch))",
            "MrglNoForever 4.0.0: no forever build at its sources",
            "  github:Example/MainlineOnly from X-Website: forever build no (no asset for flavor forever in github:Example/MainlineOnly v4.0.0 (release.json names mainline, standard; assets: Only-v4.0.0.zip, release.json; looked at 1 release))",
            "MrglNoHint 2.2: no known source",
            "MrglNoList 1.0.0: adopt as wowi:1002",
            "  wago:MrglWg02 from X-Wago-ID: forever build no (Wago lists no release for forever); needs MRGL_WAGO_KEY",
            "  wowi:1002 from X-WoWI-ID: forever build unknown (WoWInterface lists no compatibility for the project)",
            "  curse:4243 from X-Curse-Project-ID: needs MRGL_CURSEFORGE_KEY",
            "MrglPage 1.0: no known source",
            "  page https://www.example.invalid/mrgl-page from X-Website, not a source mrgl reads",
            "MrglSite (no version): adopt as github:Example/Nightly",
            "  github:Example/Nightly from X-Website: forever build yes (release.json names forever (interface 16001))",
            "MrglWagoOne 1.0.0 (2 folders: MrglWagoOne, MrglWagoOne_Options): needs MRGL_WAGO_KEY",
            "  wago:MrglWg01 from X-Wago-ID: forever build yes (Wago lists a release for forever); needs MRGL_WAGO_KEY",
            "Questie 1.0.0 (2 folders: Questie, QuestieDB): adopt as github:Questie/Questie",
            "  github:Questie/Questie from the GitHub mapping: forever build yes (release.json names forever (interface 16001))",
            "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; adopt --apply adopts 5 of them",
        ],
        "{text}"
    );
    // The report writes no sidecar.
    assert!(!game.home().join("wowf").join("state.json").exists());
}

#[test]
fn adopt_speaks_murloc_on_the_summary() {
    let game = Game::new("murloc");
    let output = game
        .mrgl()
        .args(["adopt", "--flavor", "retail"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[lines.len() - 2], "Mrglglglgl!", "{text}");
    assert_eq!(
        lines[lines.len() - 1],
        "1 unmanaged addon in _retail_: 1 can be adopted without CurseForge, 0 only with CurseForge, 0 with no known source; adopt --apply adopts 1 of them"
    );
}

#[test]
fn adopt_reports_the_curseforge_marker() {
    let game = Game::new("marker");
    let output = game
        .mrgl()
        .args(["--serious", "adopt", "--flavor", "retail"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "MrglCompat 1.0.0: adopt as wowi:1001",
            "  wowi:1001 from X-WoWI-ID: retail build yes (WoWInterface lists compatibility 12.0.7 (Made-up Patch))",
            "the CurseForge app manages _retail_ (its .curseclient marker is there): a folder belongs to one manager, so adopt --apply writes nothing there without --alongside-curseforge",
            "1 unmanaged addon in _retail_: 1 can be adopted without CurseForge, 0 only with CurseForge, 0 with no known source; adopt --apply adopts 1 of them",
        ],
        "{text}"
    );
    // The marker stays where the CurseForge app put it.
    assert!(game.root().join("_retail_").join(".curseclient").is_file());
}

#[test]
fn adopt_json_is_one_document_and_the_wago_key_adopts_the_wago_addon() {
    let game = Game::new("json");
    let output = game
        .mrgl_wago()
        .args(["--json", "adopt", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(!all_output(&output).contains(WAGO_TEST_KEY));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(value["kind"], "adopt");
    assert_eq!(value["curse_client"], false);
    assert_eq!(value["apply"], false);
    assert_eq!(value["adopted"], serde_json::json!([]));
    assert_eq!(
        value["counts"],
        serde_json::json!({
            "addons": 10,
            "without_curseforge": 6,
            "only_curseforge": 1,
            "no_build": 1,
            "no_source": 2,
            "adoptable_now": 6,
        })
    );
    let addons = value["addons"].as_array().expect("addons");
    let wago = addons
        .iter()
        .find(|addon| addon["name"] == "MrglWagoOne")
        .expect("the Wago addon");
    assert_eq!(wago["adopt_as"], "wago:MrglWg01");
    assert_eq!(wago["class"], "without-curseforge");
    assert_eq!(
        wago["folders"],
        serde_json::json!(["MrglWagoOne", "MrglWagoOne_Options"])
    );
    assert_eq!(
        wago["sources"][0],
        serde_json::json!({
            "spec": "wago:MrglWg01",
            "source": "wago",
            "from": "wago-id",
            "build": "yes",
            "why": "Wago lists a release for forever",
            "needs": null,
            "missing": null,
            "taken_by": null,
        })
    );
    let curse = addons
        .iter()
        .find(|addon| addon["name"] == "MrglCurseOnly")
        .expect("the CurseForge addon");
    assert_eq!(curse["class"], "only-curseforge");
    assert_eq!(curse["adopt_as"], serde_json::Value::Null);
    assert_eq!(curse["needs"], serde_json::json!(["MRGL_CURSEFORGE_KEY"]));
    assert_eq!(curse["sources"][0]["needs"], "MRGL_CURSEFORGE_KEY");
    let site = addons
        .iter()
        .find(|addon| addon["name"] == "MrglSite")
        .expect("the website addon");
    assert_eq!(site["version"], serde_json::Value::Null);
    let page = addons
        .iter()
        .find(|addon| addon["name"] == "MrglPage")
        .expect("the page addon");
    assert_eq!(page["class"], "no-source");
    assert_eq!(
        page["pages"],
        serde_json::json!(["https://www.example.invalid/mrgl-page"])
    );
}

#[test]
fn adopt_needs_a_flavor() {
    let game = Game::new("no-flavor");
    let output = game
        .mrgl()
        .args(["--serious", "adopt"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_last_line(&output).starts_with("no flavor given"));
}

#[test]
fn adopt_apply_writes_the_sidecar_and_an_update_installs_over_an_adopted_addon() {
    let game = Game::new("apply");
    let before = std::fs::read(
        game.addon("_forever_", "MrglWagoOne")
            .join("MrglWagoOne.toc"),
    )
    .expect("the TOC");
    let output = game
        .mrgl_wago()
        .args(["--serious", "adopt", "--flavor", "forever", "--apply"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(!all_output(&output).contains(WAGO_TEST_KEY));
    let text = stdout(&output);
    let tail: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.starts_with("adopted "))
        .collect();
    assert_eq!(
        tail,
        [
            "adopted MrglElsewhere 0.1 as wowi:1006 (2 folders: MrglElsewhere, MrglElsewhere_Options)",
            "adopted MrglForever 3.0.0 as wowi:1005 (1 folder: MrglForever)",
            "adopted MrglNoList 1.0.0 as wowi:1002 (1 folder: MrglNoList)",
            "adopted MrglSite unknown as github:Example/Nightly (1 folder: MrglSite)",
            "adopted MrglWagoOne 1.0.0 as wago:MrglWg01 (2 folders: MrglWagoOne, MrglWagoOne_Options)",
            "adopted Questie 1.0.0 as github:Questie/Questie (2 folders: Questie, QuestieDB)",
            "MrglCurseOnly not adopted: only with CurseForge, needs MRGL_CURSEFORGE_KEY",
            "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; adopted 6 addons into the sidecar, no download and no folder changed: the next update installs each from its source",
        ],
        "{text}"
    );
    // No folder changed, nothing downloaded.
    assert_eq!(
        std::fs::read(
            game.addon("_forever_", "MrglWagoOne")
                .join("MrglWagoOne.toc")
        )
        .expect("the TOC"),
        before
    );
    assert!(!game.home().join("wowf").join("cache").exists());
    let state = game.sidecar("wowf");
    let entry = &state["addons"]["MrglWagoOne"];
    assert_eq!(entry["source"], "wago:MrglWg01");
    assert_eq!(entry["version"], "1.0.0");
    assert_eq!(
        entry["folders"],
        serde_json::json!(["MrglWagoOne", "MrglWagoOne_Options"])
    );
    assert_eq!(entry["files"].as_object().expect("files").len(), 4);
    assert_eq!(state["addons"]["MrglSite"]["version"], "unknown");
    assert!(state["addons"].get("MrglCurseOnly").is_none());

    // `list` reads the adopted addons' folders as managed.
    let output = game
        .mrgl()
        .args(["--json", "list", "--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert_eq!(listed["counts"]["managed"], 9);

    // The update check takes every adopted entry, and one update installs the source's build cleanly.
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
    assert!(
        text.lines()
            .any(|line| line == "MrglWagoOne  1.0.0 -> 2.0.0  update available"),
        "{text}"
    );
    assert!(!text.contains("not checked"), "{text}");
    let output = game
        .mrgl_wago()
        .args([
            "--serious",
            "update",
            "MrglWagoOne",
            "--flavor",
            "forever",
            "--yes",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert!(
        text.lines().any(|line| line
            == "updated MrglWagoOne from 1.0.0 to 2.0.0 (1 folder: MrglWagoOne) in _forever_, 1 folder of the previous version removed (MrglWagoOne_Options)"),
        "{text}"
    );
    let state = game.sidecar("wowf");
    let entry = &state["addons"]["MrglWagoOne"];
    assert_eq!(entry["version"], "2.0.0");
    assert_eq!(entry["previous_version"], "1.0.0");
    assert_eq!(entry["folders"], serde_json::json!(["MrglWagoOne"]));
    assert!(!game.addon("_forever_", "MrglWagoOne_Options").exists());
    assert_ne!(
        std::fs::read(
            game.addon("_forever_", "MrglWagoOne")
                .join("MrglWagoOne.toc")
        )
        .expect("the TOC"),
        before
    );
}

#[test]
fn a_second_apply_changes_nothing() {
    let game = Game::new("second");
    let output = game
        .mrgl()
        .args(["--serious", "adopt", "--flavor", "forever", "--apply"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let first = std::fs::read(game.state_path("wowf")).expect("the sidecar");
    let output = game
        .mrgl()
        .args(["--serious", "adopt", "--flavor", "forever", "--apply"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert_eq!(
        std::fs::read(game.state_path("wowf")).expect("the sidecar"),
        first
    );
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        &lines[lines.len() - 3..],
        [
            "MrglCurseOnly not adopted: only with CurseForge, needs MRGL_CURSEFORGE_KEY",
            "MrglWagoOne not adopted: needs MRGL_WAGO_KEY",
            "5 unmanaged addons in _forever_: 1 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; nothing adopted, the sidecar is unchanged",
        ],
        "{text}"
    );
}

#[test]
fn apply_in_a_folder_the_curseforge_app_manages_needs_the_flag() {
    let game = Game::new("guard");
    let output = game
        .mrgl()
        .args(["--serious", "adopt", "--flavor", "retail", "--apply"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_last_line(&output),
        "the CurseForge app manages _retail_ (its .curseclient marker is there) and a folder belongs to one manager: adopt --apply wrote nothing; --alongside-curseforge adopts its addons anyway, and the CurseForge app may still update those folders"
    );
    assert!(!game.state_path("wow").exists());

    // The flag needs --apply.
    let output = game
        .mrgl()
        .args(["adopt", "--flavor", "retail", "--alongside-curseforge"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(2));

    let output = game
        .mrgl()
        .args([
            "--serious",
            "adopt",
            "--flavor",
            "retail",
            "--apply",
            "--alongside-curseforge",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let text = stdout(&output);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "MrglCompat 1.0.0: adopt as wowi:1001",
            "  wowi:1001 from X-WoWI-ID: retail build yes (WoWInterface lists compatibility 12.0.7 (Made-up Patch))",
            "adopted MrglCompat 1.0.0 as wowi:1001 (1 folder: MrglCompat)",
            "adopting alongside the CurseForge app in _retail_: it may still update or flag the adopted folders, and its .curseclient marker stays where it is",
            "1 unmanaged addon in _retail_: 1 can be adopted without CurseForge, 0 only with CurseForge, 0 with no known source; adopted 1 addon into the sidecar, no download and no folder changed: the next update installs each from its source",
        ],
        "{text}"
    );
    assert_eq!(
        game.sidecar("wow")["addons"]["MrglCompat"]["source"],
        "wowi:1001"
    );
    // The marker stays.
    assert!(game.root().join("_retail_").join(".curseclient").is_file());

    // The same refusal in JSON is an error document and exit 1.
    let other = Game::new("guard-json");
    let output = other
        .mrgl()
        .args(["--json", "adopt", "--flavor", "retail", "--apply"])
        .output()
        .expect("run mrgl");
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value =
        serde_json::from_str(&stderr_last_line(&output)).expect("an error document");
    assert_eq!(error["kind"], "error");
    assert!(!other.state_path("wow").exists());
}

#[test]
fn add_import_and_update_warn_in_a_folder_the_curseforge_app_manages() {
    const WARNING: &str = "the CurseForge app manages _retail_ too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager";
    let game = Game::new("warn");
    let zip = fixtures().join("zips").join("one-folder-1.0.0.zip");
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(&zip)
        .args(["--flavor", "retail"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout(&output).lines().any(|line| line == WARNING));

    let list = game.base.join("list.json");
    std::fs::write(
        &list,
        r#"{"format": "mrgl-list", "schema": 1, "flavor": "wow", "addons": [
            {"name": "MrglCompat", "source": "wowi:1001", "version": "2.0.0", "folders": ["MrglCompat"]}
        ]}"#,
    )
    .expect("the list");
    let output = game
        .mrgl()
        .args(["--serious", "import"])
        .arg(&list)
        .args(["--flavor", "retail", "--dry-run"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(stdout(&output).lines().any(|line| line == WARNING));

    let output = game
        .mrgl()
        .args([
            "--json",
            "update",
            "--all",
            "--dry-run",
            "--flavor",
            "retail",
        ])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one document");
    assert!(
        value["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning == WARNING),
        "{value}"
    );

    // A folder without the marker has no such line.
    let output = game
        .mrgl()
        .args(["--serious", "add"])
        .arg(&zip)
        .args(["--flavor", "forever"])
        .output()
        .expect("run mrgl");
    assert!(output.status.success(), "{}", stderr_last_line(&output));
    assert!(!stdout(&output).contains(".curseclient"));
}
