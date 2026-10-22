//! The window's toolbox (task 074) on a made-up Forever flavor: the `WTF` tree of the addon-sets fixture
//! (two characters on two realms) copied into a temporary game root, three addon folders, a few made-up
//! SavedVariables files (two of them orphans) and a sets file under a temporary mrgl home. The toolbox's
//! listing, a set applied to every character and to one, the orphans moved into the dated zip, and both
//! writes refused while the stand-in client runs (task 063: a copy of the system's `ping.exe` in the flavor
//! folder, never a game file). Nothing touches a real game folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::{Child, Command, Stdio};

use mrgl_app_lib::toolbox::{self, ToolboxAnswer, ToolboxView};
use mrgl_core::flavor::FlavorId;
use mrgl_core::sets::{self, AddonSet, AddonState, AddonsTxt, SetsFile};
use mrgl_core::voice::Voice;

const BUILD_INFO: &str = "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\n";
const ACCOUNT: &str = "MRGLSETS";

fn fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/addon-sets/WTF")
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

/// A tree as relative paths (`/` separators) to the bytes of every file.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("read") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, &format!("{prefix}{name}/"), into);
            } else {
                into.insert(
                    format!("{prefix}{name}"),
                    std::fs::read(&path).expect("bytes"),
                );
            }
        }
    }
    let mut into = BTreeMap::new();
    walk(dir, "", &mut into);
    into
}

/// A temporary game root with a Forever flavor and a mrgl home folder, removed at the end.
struct Game {
    dir: PathBuf,
}

impl Game {
    /// The flavor: `RaidFrames`, `BossTimers` and `QuestArrows` installed, the fixture `WTF` with its two characters
    /// (`Fixture Realm/Mrgltest`, `Second Realm/Grrlwave`), the SavedVariables `RaidFrames` (account-wide,
    /// from the fixture), `OldMeter` with its `.bak` (account-wide, an orphan), `Blizzard_Console` (the
    /// client's own), `BossTimers` (Mrgltest) and `GoneAddon` (Grrlwave, an orphan); the sets `raid` and
    /// `solo` in the home folder.
    fn new(name: &str) -> Game {
        let dir =
            std::env::temp_dir().join(format!("mrgl-app-toolbox-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = Game { dir };
        let flavor = game.flavor_folder();
        std::fs::create_dir_all(&flavor).expect("the flavor folder");
        std::fs::write(game.root().join(".build.info"), BUILD_INFO).expect(".build.info");
        std::fs::write(
            flavor.join(".flavor.info"),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect(".flavor.info");
        for addon in ["RaidFrames", "BossTimers", "QuestArrows"] {
            let folder = flavor.join("Interface/AddOns").join(addon);
            std::fs::create_dir_all(&folder).expect("the addon folder");
            std::fs::write(
                folder.join(format!("{addon}.toc")),
                format!("## Interface: 11508\n## Title: {addon}\n## Version: 1.0.0\n"),
            )
            .expect("the TOC");
        }
        copy_tree(&fixture_wtf(), &game.wtf());
        let account = game.wtf().join("Account").join(ACCOUNT);
        let write = |path: PathBuf, text: &str| {
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("the folder");
            std::fs::write(path, text).expect("the file");
        };
        write(
            account.join("SavedVariables/OldMeter.lua"),
            "OldMeterDB = {\n\t[\"version\"] = 3,\n}\n",
        );
        write(
            account.join("SavedVariables/OldMeter.lua.bak"),
            "OldMeterDB = {\n\t[\"version\"] = 2,\n}\n",
        );
        write(
            account.join("SavedVariables/Blizzard_Console.lua"),
            "Blizzard_Console_SavedVars = {}\n",
        );
        write(
            account.join("Fixture Realm/Mrgltest/SavedVariables/BossTimers.lua"),
            "BossTimersCharDB = {\n\t[\"bars\"] = true,\n}\n",
        );
        write(
            account.join("Second Realm/Grrlwave/SavedVariables/GoneAddon.lua"),
            "GoneAddonDB = nil\n",
        );
        let mut file = SetsFile::new(&FlavorId::parse("wowf"));
        let set = |addons: &[(&str, AddonState)]| AddonSet {
            saved_at: "2026-10-22T10:00:00Z".to_string(),
            addons: addons
                .iter()
                .map(|(name, state)| (name.to_string(), *state))
                .collect(),
        };
        file.sets.insert(
            "raid".to_string(),
            set(&[
                ("RaidFrames", AddonState::Enabled),
                ("BossTimers", AddonState::Enabled),
                ("QuestArrows", AddonState::Disabled),
            ]),
        );
        file.sets.insert(
            "solo".to_string(),
            set(&[
                ("BossTimers", AddonState::Disabled),
                ("Missing", AddonState::Enabled),
            ]),
        );
        sets::write_sets_at(&game.sets_path(), &file).expect("the sets file");
        game
    }

    fn root(&self) -> PathBuf {
        self.dir.join("World of Warcraft")
    }

    fn flavor_folder(&self) -> PathBuf {
        self.root().join("_forever_")
    }

    fn wtf(&self) -> PathBuf {
        self.flavor_folder().join("WTF")
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn sets_path(&self) -> PathBuf {
        sets::sets_path_at(&self.home(), &FlavorId::parse("wowf"))
    }

    fn character(&self, realm: &str, name: &str) -> PathBuf {
        self.wtf()
            .join("Account")
            .join(ACCOUNT)
            .join(realm)
            .join(name)
    }

    fn addons_txt(&self, realm: &str, name: &str) -> Vec<u8> {
        std::fs::read(self.character(realm, name).join("AddOns.txt")).expect("AddOns.txt")
    }

    fn state_of(&self, realm: &str, name: &str, addon: &str) -> Option<AddonState> {
        AddonsTxt::parse(&self.addons_txt(realm, name)).state_of(addon)
    }

    fn backups(&self) -> PathBuf {
        self.home().join("backups").join("wowf")
    }

    fn load(&self) -> ToolboxAnswer {
        toolbox::load(&self.root(), "wowf", Some(&self.home()), Voice::new(true))
    }

    fn apply(&self, set: &str, character: Option<&str>, realm: Option<&str>) -> ToolboxAnswer {
        toolbox::apply(
            &self.root(),
            "wowf",
            set,
            character,
            realm,
            Some(&self.home()),
            Voice::new(true),
        )
    }

    fn clean(&self) -> ToolboxAnswer {
        toolbox::clean(&self.root(), "wowf", Some(&self.home()), Voice::new(true))
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn plains(answer: &ToolboxAnswer) -> Vec<(&str, &str)> {
    answer
        .messages
        .iter()
        .map(|message| (message.kind, message.plain.as_str()))
        .collect()
}

fn view(answer: &ToolboxAnswer) -> &ToolboxView {
    answer.toolbox.as_ref().expect("the toolbox")
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).expect("metadata").len()
}

#[test]
fn the_toolbox_lists_the_sets_the_characters_and_the_saved_variables() {
    let game = Game::new("load");
    let answer = game.load();
    assert!(answer.messages.is_empty(), "{:?}", plains(&answer));
    let toolbox = view(&answer);
    assert_eq!(
        (toolbox.flavor.as_str(), toolbox.folder.as_str()),
        ("wowf", "_forever_")
    );
    assert_eq!(toolbox.client, None, "no client runs");

    let names: Vec<&str> = toolbox.sets.iter().map(|set| set.name.as_str()).collect();
    assert_eq!(names, ["raid", "solo"]);
    assert_eq!(toolbox.sets_note.kind, "success");
    assert_eq!(
        toolbox.sets_note.plain,
        format!("2 sets of _forever_ in {}", game.sets_path().display())
    );
    let solo = &toolbox.sets[1];
    assert_eq!((solo.enabled, solo.disabled), (1, 1));
    let addons: Vec<(&str, AddonState, bool)> = solo
        .addons
        .iter()
        .map(|addon| (addon.name.as_str(), addon.state, addon.installed))
        .collect();
    assert_eq!(
        addons,
        [
            ("BossTimers", AddonState::Disabled, true),
            ("Missing", AddonState::Enabled, false)
        ]
    );

    let characters: Vec<(&str, &str, &str)> = toolbox
        .characters
        .iter()
        .map(|character| {
            (
                character.name.as_str(),
                character.realm.as_str(),
                character.label.as_str(),
            )
        })
        .collect();
    assert_eq!(
        characters,
        [
            ("Mrgltest", "Fixture Realm", "Fixture Realm/Mrgltest"),
            ("Grrlwave", "Second Realm", "Second Realm/Grrlwave"),
        ]
    );

    let saved = &toolbox.saved_variables;
    let mut rows: Vec<(&str, &str, &str, bool)> = saved
        .files
        .iter()
        .map(|row| {
            let status = match row.status {
                mrgl_core::savedvars::Status::Installed => "installed",
                mrgl_core::savedvars::Status::Orphan => "orphan",
                mrgl_core::savedvars::Status::Client => "client",
                mrgl_core::savedvars::Status::Unknown => "unknown",
            };
            (row.addon.as_str(), row.place.as_str(), status, row.bak)
        })
        .collect();
    rows.sort_unstable();
    assert_eq!(
        rows,
        [
            ("Blizzard_Console", "account-wide", "client", false),
            ("BossTimers", "Fixture Realm / Mrgltest", "installed", false),
            ("GoneAddon", "Second Realm / Grrlwave", "orphan", false),
            ("OldMeter", "account-wide", "orphan", true),
            ("RaidFrames", "account-wide", "installed", false),
        ]
    );
    let account = game.wtf().join("Account").join(ACCOUNT);
    let orphan_bytes = file_size(&account.join("SavedVariables/OldMeter.lua"))
        + file_size(&account.join("SavedVariables/OldMeter.lua.bak"))
        + file_size(&account.join("Second Realm/Grrlwave/SavedVariables/GoneAddon.lua"));
    assert_eq!((saved.orphans, saved.orphan_bytes), (2, orphan_bytes));
    assert_eq!(saved.orphan_size, format!("{orphan_bytes} B"));
    assert_eq!(saved.summary.kind, "warning");
    assert_eq!(
        saved.summary.plain,
        format!(
            "5 SavedVariables files of _forever_, {} B in all; 2 orphans ({orphan_bytes} B) have no addon folder: the cleanup moves them into a backup zip",
            saved.bytes
        )
    );
}

#[test]
fn a_set_goes_to_every_character_and_says_what_is_not_installed() {
    let game = Game::new("every");
    let answer = game.apply("solo", None, None);
    assert_eq!(
        plains(&answer),
        [
            (
                "warning",
                "set solo names addons that are not installed in _forever_, their lines are written all the same: Missing"
            ),
            (
                "success",
                "applied set solo to 2 characters of _forever_: 3 addon lines changed"
            ),
        ]
    );
    for (realm, name) in [("Fixture Realm", "Mrgltest"), ("Second Realm", "Grrlwave")] {
        assert_eq!(
            game.state_of(realm, name, "BossTimers"),
            Some(AddonState::Disabled)
        );
        assert_eq!(
            game.state_of(realm, name, "Missing"),
            Some(AddonState::Enabled)
        );
    }
    let changes: Vec<(&str, usize, bool)> = answer
        .changes
        .iter()
        .map(|change| {
            (
                change.character.label.as_str(),
                change.changed.len(),
                change.created,
            )
        })
        .collect();
    assert_eq!(
        changes,
        [
            ("Fixture Realm/Mrgltest", 2, false),
            ("Second Realm/Grrlwave", 1, false)
        ]
    );
    assert!(answer.toolbox.is_some(), "the toolbox as it is now");

    let again = game.apply("SOLO", None, None);
    assert_eq!(
        plains(&again),
        [
            (
                "warning",
                "set solo names addons that are not installed in _forever_, their lines are written all the same: Missing"
            ),
            (
                "nothing",
                "set solo already matches every character of _forever_ (2 characters), nothing written"
            ),
        ]
    );
}

#[test]
fn a_set_goes_to_one_character_and_leaves_the_other_byte_for_byte() {
    let game = Game::new("one");
    let other = game.addons_txt("Fixture Realm", "Mrgltest");
    let answer = game.apply("raid", Some("grrlwave"), None);
    assert_eq!(
        plains(&answer),
        [(
            "success",
            "applied set raid to Second Realm/Grrlwave of _forever_: 3 addon lines changed"
        )]
    );
    assert_eq!(game.addons_txt("Fixture Realm", "Mrgltest"), other);
    for (addon, state) in [
        ("RaidFrames", AddonState::Enabled),
        ("BossTimers", AddonState::Enabled),
        ("QuestArrows", AddonState::Disabled),
    ] {
        assert_eq!(
            game.state_of("Second Realm", "Grrlwave", addon),
            Some(state),
            "{addon}"
        );
    }
    assert_eq!(answer.changes.len(), 1);
    assert_eq!(answer.changes[0].character.label, "Second Realm/Grrlwave");

    let again = game.apply("raid", Some("Grrlwave"), Some("second realm"));
    assert_eq!(
        plains(&again),
        [(
            "nothing",
            "set raid already matches Second Realm/Grrlwave of _forever_, nothing written"
        )]
    );
}

#[test]
fn an_unknown_character_or_set_is_the_core_s_error() {
    let game = Game::new("unknown");
    let before = snapshot(&game.wtf());
    let answer = game.apply("raid", Some("Nobody"), None);
    assert_eq!(answer.messages.len(), 1);
    assert_eq!(answer.messages[0].kind, "error");
    assert!(
        answer.messages[0]
            .plain
            .starts_with("character not found: Nobody (flavor "),
        "{}",
        answer.messages[0].plain
    );
    assert!(
        answer.messages[0]
            .plain
            .ends_with("the characters: Mrgltest (Fixture Realm), Grrlwave (Second Realm)"),
        "{}",
        answer.messages[0].plain
    );
    assert!(answer.toolbox.is_some(), "the panel stays drawn");
    let answer = game.apply("nope", None, None);
    assert_eq!(answer.messages[0].kind, "error");
    assert!(
        answer.messages[0]
            .plain
            .starts_with("set not found: nope (flavor ")
            && answer.messages[0]
                .plain
                .ends_with("the saved sets: raid, solo"),
        "{}",
        answer.messages[0].plain
    );
    assert_eq!(snapshot(&game.wtf()), before, "nothing written");
    let answer = toolbox::load(
        &game.root(),
        "nothing",
        Some(&game.home()),
        Voice::new(true),
    );
    assert_eq!(answer.toolbox, None);
    assert_eq!(
        plains(&answer),
        [(
            "error",
            format!(
                "no flavor nothing: no such flavor folder under {}",
                game.root().display()
            )
            .as_str()
        )]
    );
}

#[test]
fn the_cleanup_moves_the_orphans_into_the_dated_zip() {
    let game = Game::new("clean");
    let account = game.wtf().join("Account").join(ACCOUNT);
    let orphans = [
        account.join("SavedVariables/OldMeter.lua"),
        account.join("SavedVariables/OldMeter.lua.bak"),
        account.join("Second Realm/Grrlwave/SavedVariables/GoneAddon.lua"),
    ];
    let bytes: u64 = orphans.iter().map(|path| file_size(path)).sum();
    let answer = game.clean();
    let zip = PathBuf::from(answer.zip.as_deref().expect("a zip"));
    assert_eq!(zip.parent(), Some(game.backups().as_path()));
    let zip_name = zip
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    assert!(
        zip_name.starts_with("SavedVariables-") && zip_name.ends_with(".zip"),
        "{zip_name}"
    );
    assert!(zip.is_file());
    assert_eq!(
        plains(&answer),
        [(
            "success",
            format!(
                "moved 2 orphans of _forever_ (3 files, {bytes} B) into {}; unzip it into the WTF folder to put them back",
                zip.display()
            )
            .as_str()
        )]
    );
    for orphan in &orphans {
        assert!(!orphan.exists(), "{} moved", orphan.display());
    }
    for kept in [
        account.join("SavedVariables/RaidFrames.lua"),
        account.join("SavedVariables/Blizzard_Console.lua"),
        account.join("Fixture Realm/Mrgltest/SavedVariables/BossTimers.lua"),
    ] {
        assert!(kept.is_file(), "{} kept", kept.display());
    }
    let saved = &view(&answer).saved_variables;
    assert_eq!((saved.files.len(), saved.orphans), (3, 0));
    assert_eq!(saved.summary.kind, "success");

    let again = game.clean();
    assert_eq!(again.zip, None);
    assert_eq!(
        plains(&again),
        [(
            "nothing",
            "no orphans in the SavedVariables of _forever_, nothing moved"
        )]
    );
}

#[test]
fn without_a_home_folder_nothing_is_written() {
    let game = Game::new("no-home");
    let before = snapshot(&game.wtf());
    let answer = toolbox::load(&game.root(), "wowf", None, Voice::new(true));
    let toolbox = view(&answer);
    assert!(toolbox.sets.is_empty());
    assert_eq!(toolbox.sets_note.kind, "error");
    assert!(
        toolbox
            .sets_note
            .plain
            .starts_with("no sets of _forever_: ")
    );
    let answer = toolbox::clean(&game.root(), "wowf", None, Voice::new(true));
    assert_eq!(answer.messages[0].kind, "error");
    assert!(
        answer.messages[0]
            .plain
            .starts_with("no SavedVariables moved, the backup zip has nowhere to go: "),
        "{}",
        answer.messages[0].plain
    );
    let answer = toolbox::apply(
        &game.root(),
        "wowf",
        "raid",
        None,
        None,
        None,
        Voice::new(true),
    );
    assert!(answer.messages[0].plain.starts_with("no set applied: "));
    assert_eq!(snapshot(&game.wtf()), before);
}

#[test]
fn the_answer_serializes_with_the_keys_of_the_contract() {
    let game = Game::new("keys");
    let answer = game.apply("raid", Some("Grrlwave"), None);
    let value = serde_json::to_value(&answer).expect("serializes");
    let keys = |value: &serde_json::Value| -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    // The keys of the contract in the header of ui/mock-data.js.
    assert_eq!(keys(&value), ["changes", "messages", "toolbox", "zip"]);
    let toolbox = &value["toolbox"];
    assert_eq!(
        keys(toolbox),
        [
            "characters",
            "client",
            "flavor",
            "folder",
            "saved_variables",
            "sets",
            "sets_note"
        ]
    );
    assert_eq!(
        keys(&toolbox["sets"][0]),
        ["addons", "disabled", "enabled", "name", "saved_at"]
    );
    assert_eq!(
        keys(&toolbox["sets"][0]["addons"][0]),
        ["installed", "name", "state"]
    );
    assert_eq!(keys(&toolbox["characters"][0]), ["label", "name", "realm"]);
    assert_eq!(
        keys(&toolbox["saved_variables"]),
        [
            "bytes",
            "files",
            "orphan_bytes",
            "orphan_size",
            "orphans",
            "size",
            "summary"
        ]
    );
    assert_eq!(
        keys(&toolbox["saved_variables"]["files"][0]),
        [
            "addon",
            "bak",
            "bytes",
            "character",
            "lua",
            "place",
            "realm",
            "size",
            "status"
        ]
    );
    assert_eq!(
        keys(&value["changes"][0]),
        ["changed", "character", "created"]
    );
    assert_eq!(keys(&value["changes"][0]["changed"][0]), ["name", "state"]);
    assert_eq!(value["changes"][0]["changed"][0]["state"], "enabled");
    let statuses: Vec<&str> = toolbox["saved_variables"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| file["status"].as_str().expect("a status"))
        .collect();
    assert!(statuses.contains(&"orphan") && statuses.contains(&"client"));
    assert!(
        serde_json::to_string(&value).is_ok_and(|text| !text.contains(ACCOUNT)),
        "the account folder's name stays out of the window's answer"
    );
}

/// The stand-in client (task 063): a copy of the signed `ping.exe` of the system folder, named
/// `client-stand-in.exe` in the flavor folder and pinging the loopback address for an hour, so it runs
/// until the test stops it. Never a copy of this test binary. `None` when the system folder has no
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

/// The running stand-in client: `stop` kills and waits for it, and so does a drop, so a failing assertion
/// leaves no process behind.
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
fn both_writes_are_refused_while_the_stand_in_client_runs() {
    let game = Game::new("client");
    let Some(mut client) = spawn_stand_in(&game.flavor_folder()) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let refusal = format!(
        "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
        client.id()
    );
    let before = snapshot(&game.wtf());

    let loaded = game.load();
    let note = view(&loaded).client.as_ref().expect("the client note");
    assert_eq!(
        (note.kind, note.plain.as_str()),
        ("warning", refusal.as_str())
    );

    for answer in [
        game.apply("raid", None, None),
        game.apply("raid", Some("Grrlwave"), None),
        game.clean(),
    ] {
        assert_eq!(plains(&answer), [("error", refusal.as_str())]);
        assert!(answer.changes.is_empty() && answer.zip.is_none());
        assert!(answer.toolbox.is_some(), "the panel stays drawn");
    }
    assert_eq!(snapshot(&game.wtf()), before, "nothing under WTF written");
    assert!(!game.backups().exists(), "no zip written");

    client.stop();
    assert_eq!(view(&game.load()).client, None);
    let answer = game.apply("raid", None, None);
    assert_eq!(answer.messages.last().expect("a summary").kind, "success");
    assert!(game.clean().zip.is_some());
}
