//! The addon sets (task 014) over the synthetic fixture `WTF` tree with two characters
//! (`tests/fixtures/addon-sets/`): every `AddOns.txt` round-trips byte for byte, a save reads the
//! characters' states and leaves out what they disagree on, an apply changes exactly the listed addons in
//! the client's own format and leaves every other byte of `WTF` as it was, a second apply writes nothing,
//! and an apply with the stand-in client running (the pattern of the 023 and 063 tests: a copy of the
//! system's `ping.exe` in the flavor folder, never a game file) is refused before anything is written.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use mrgl_core::Error;
use mrgl_core::flavor::{self, Flavor, FlavorId};
use mrgl_core::sets::{self, AddonState, AddonsTxt, SETS_FILE};

fn fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/addon-sets/WTF")
}

/// A synthetic game folder under the temp dir with a `_forever_` flavor, the fixture `WTF` tree copied in
/// and an mrgl home folder next to it.
fn temp_game_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mrgl-sets-test-{}-{name}", std::process::id()));
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
    copy_tree(&fixture_wtf(), &wtf_of(&root));
    root
}

fn wtf_of(root: &Path) -> PathBuf {
    root.join("_forever_").join("WTF")
}

fn sets_path_of(root: &Path) -> PathBuf {
    sets::sets_path_at(&root.join("home"), &FlavorId::parse("wowf"))
}

fn read(root: &Path) -> Flavor {
    flavor::read_flavor(&root.join("_forever_")).expect("a flavor")
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

/// A tree as relative paths (`/` separators, a folder with a trailing `/`) to the bytes of every file.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("read") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if path.is_dir() {
                into.insert(format!("{prefix}{name}/"), Vec::new());
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

const FIRST: &str = "Account/MRGLSETS/Fixture Realm/Mrgltest/AddOns.txt";
const SECOND: &str = "Account/MRGLSETS/Second Realm/Grrlwave/AddOns.txt";

/// Writes a sets file with one set, as `set save` would have.
fn write_set(root: &Path, name: &str, addons: &[(&str, AddonState)]) {
    let mut file = sets::read_sets_at(&sets_path_of(root), &FlavorId::parse("wowf")).expect("read");
    file.sets.insert(
        name.to_string(),
        sets::AddonSet {
            saved_at: "2026-10-22T00:00:00Z".to_string(),
            addons: addons
                .iter()
                .map(|(addon, state)| (addon.to_string(), *state))
                .collect(),
        },
    );
    sets::write_sets_at(&sets_path_of(root), &file).expect("written");
}

#[test]
fn the_characters_are_listed_and_every_addons_txt_round_trips_byte_for_byte() {
    let root = temp_game_root("roundtrip");
    let characters = sets::characters(&read(&root)).expect("characters");
    let labels: Vec<String> = characters.iter().map(sets::Character::label).collect();
    assert_eq!(
        labels,
        [
            "MRGLSETS/Fixture Realm/Mrgltest",
            "MRGLSETS/Second Realm/Grrlwave"
        ],
        "the account's SavedVariables is no realm"
    );
    for character in &characters {
        let bytes = std::fs::read(character.addons_txt()).expect("AddOns.txt");
        assert_eq!(AddonsTxt::parse(&bytes).to_bytes(), bytes);
    }
    let second = AddonsTxt::parse(&std::fs::read(characters[1].addons_txt()).expect("bytes"));
    assert_eq!(
        second.entries(),
        vec![
            ("QuestArrows".to_string(), AddonState::Enabled),
            ("BagSorter".to_string(), AddonState::Enabled),
            ("BossTimers".to_string(), AddonState::Disabled),
        ],
        "the unknown line is no entry"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_save_keeps_what_every_character_agrees_on_and_replaces_a_set_of_the_same_name() {
    let root = temp_game_root("save");
    let flavor = read(&root);
    let before = snapshot(&wtf_of(&root));
    let path = sets_path_of(&root);
    assert_eq!(path, root.join("home").join("wowf").join(SETS_FILE));

    let saved = sets::save_set(&flavor, &path, " everyday ").expect("saved");
    assert_eq!(saved.name, "everyday");
    assert!(!saved.replaced);
    assert_eq!(saved.characters, 2);
    assert_eq!(
        saved.set.addons,
        BTreeMap::from([
            ("BagSorter".to_string(), AddonState::Enabled),
            ("RaidFrames".to_string(), AddonState::Enabled),
        ])
    );
    assert_eq!(saved.conflicts, ["BossTimers", "QuestArrows"]);
    assert_eq!(snapshot(&wtf_of(&root)), before, "a save only reads WTF");

    let file: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("sets.json")).expect("JSON");
    assert_eq!(file["format"], "mrgl-sets");
    assert_eq!(file["schema"], 1);
    assert_eq!(file["flavor"], "wowf");
    assert_eq!(file["sets"]["everyday"]["addons"]["BagSorter"], "enabled");

    let again = sets::save_set(&flavor, &path, "EVERYDAY").expect("saved again");
    assert!(again.replaced);
    let file = sets::read_sets_at(&path, &flavor.id).expect("read");
    assert_eq!(
        file.sets.keys().collect::<Vec<_>>(),
        ["EVERYDAY"],
        "the old name is gone"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_apply_changes_exactly_the_listed_addons_in_the_clients_format() {
    let root = temp_game_root("apply");
    let flavor = read(&root);
    std::fs::create_dir_all(root.join("_forever_/Interface/AddOns/RaidFrames")).expect("addon");
    std::fs::create_dir_all(root.join("_forever_/Interface/AddOns/BossTimers")).expect("addon");
    write_set(
        &root,
        "raid",
        &[
            ("RaidFrames", AddonState::Enabled),
            ("BossTimers", AddonState::Enabled),
            ("BagSorter", AddonState::Disabled),
            ("NewMeter", AddonState::Enabled),
        ],
    );
    let before = snapshot(&wtf_of(&root));

    let applied = sets::apply_set(&flavor, &sets_path_of(&root), "Raid").expect("applied");
    assert_eq!(applied.name, "raid");
    assert_eq!(applied.characters.len(), 2);
    assert_eq!(
        applied.characters[0].changed,
        [
            ("BagSorter".to_string(), AddonState::Disabled),
            ("NewMeter".to_string(), AddonState::Enabled),
        ]
    );
    assert_eq!(
        applied.characters[1].changed,
        [
            ("BagSorter".to_string(), AddonState::Disabled),
            ("BossTimers".to_string(), AddonState::Enabled),
            ("NewMeter".to_string(), AddonState::Enabled),
            ("RaidFrames".to_string(), AddonState::Enabled),
        ]
    );
    assert_eq!(applied.changed(), 6);
    assert!(applied.characters.iter().all(|change| !change.created));
    assert_eq!(applied.not_installed, ["BagSorter", "NewMeter"]);

    let after = snapshot(&wtf_of(&root));
    assert_eq!(
        after[FIRST],
        b"RaidFrames: enabled\r\nBossTimers: enabled\r\nQuestArrows: disabled\r\nBagSorter: disabled\r\nNewMeter: enabled\r\n"
    );
    assert_eq!(
        after[SECOND],
        b"QuestArrows: enabled\n# a line the client never wrote, kept as it is\nBagSorter: disabled\nBossTimers: enabled\nNewMeter: enabled\nRaidFrames: enabled\n"
    );
    let unchanged = |tree: &BTreeMap<String, Vec<u8>>| {
        tree.iter()
            .filter(|(path, _)| !path.ends_with("AddOns.txt"))
            .map(|(path, bytes)| (path.clone(), bytes.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        unchanged(&after),
        unchanged(&before),
        "nothing else under WTF moved"
    );

    // A second apply finds nothing to change and writes nothing: the files keep their times.
    let times: Vec<_> = [FIRST, SECOND]
        .iter()
        .map(|file| {
            std::fs::metadata(wtf_of(&root).join(file))
                .and_then(|metadata| metadata.modified())
                .expect("a time")
        })
        .collect();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let again = sets::apply_set(&flavor, &sets_path_of(&root), "raid").expect("applied again");
    assert_eq!(again.changed(), 0);
    assert_eq!(snapshot(&wtf_of(&root)), after);
    for (file, time) in [FIRST, SECOND].iter().zip(times) {
        let now = std::fs::metadata(wtf_of(&root).join(file))
            .and_then(|metadata| metadata.modified())
            .expect("a time");
        assert_eq!(now, time, "{file} was not written");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_character_without_addons_txt_gets_one() {
    let root = temp_game_root("create");
    let flavor = read(&root);
    let fresh = wtf_of(&root).join("Account/MRGLSETS/Second Realm/Newbie");
    std::fs::create_dir_all(&fresh).expect("a character folder");
    write_set(&root, "solo", &[("QuestArrows", AddonState::Enabled)]);
    let applied = sets::apply_set(&flavor, &sets_path_of(&root), "solo").expect("applied");
    let newbie = applied
        .characters
        .iter()
        .find(|change| change.character.name == "Newbie")
        .expect("the new character");
    assert!(newbie.created);
    assert_eq!(
        std::fs::read(fresh.join("AddOns.txt")).expect("written"),
        b"QuestArrows: enabled\r\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unknown_sets_no_characters_and_a_foreign_sets_file_are_errors() {
    let root = temp_game_root("errors");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    let error = sets::apply_set(&flavor, &path, "raid").expect_err("no set");
    assert_eq!(
        error.to_string(),
        "set not found: raid (flavor forever); no set is saved yet: `mrgl set save <name>` saves one"
    );
    write_set(&root, "raid", &[("RaidFrames", AddonState::Enabled)]);
    let error = sets::remove_set(&flavor.id, &path, "solo").expect_err("no such set");
    assert_eq!(
        error.to_string(),
        "set not found: solo (flavor forever); the saved sets: raid"
    );
    let (name, removed) = sets::remove_set(&flavor.id, &path, "raid").expect("removed");
    assert_eq!(name, "raid");
    assert_eq!(removed.enabled(), 1);
    assert!(
        sets::read_sets_at(&path, &flavor.id)
            .expect("read")
            .sets
            .is_empty()
    );

    std::fs::write(&path, "{ not json").expect("a broken file");
    let error = sets::save_set(&flavor, &path, "raid").expect_err("unreadable");
    assert!(matches!(error, Error::SetsUnreadable { .. }), "{error}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("kept"),
        "{ not json",
        "never overwritten"
    );
    std::fs::write(
        &path,
        r#"{"format":"mrgl-sets","schema":1,"flavor":"wowf","sets":{"x":{"saved_at":"","addons":{"A\r\nB":"enabled"}}}}"#,
    )
    .expect("a hostile file");
    let error = sets::apply_set(&flavor, &path, "x").expect_err("refused");
    assert!(
        error.to_string().contains("which is not an addon name"),
        "{error}"
    );

    std::fs::remove_dir_all(wtf_of(&root)).expect("no WTF");
    std::fs::remove_file(&path).expect("no sets");
    let error = sets::save_set(&flavor, &path, "raid").expect_err("no characters");
    assert!(
        matches!(&error, Error::NoCharacters { path } if path == &wtf_of(&root)),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
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
fn an_apply_is_refused_while_the_client_runs_and_a_save_is_not() {
    let root = temp_game_root("client");
    let flavor = read(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));

    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let error = sets::apply_set(&flavor, &sets_path_of(&root), "raid").expect_err("refused");
    assert_eq!(
        error.to_string(),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was touched");
    // A save only reads WTF, so it goes on while the client runs.
    sets::save_set(&flavor, &sets_path_of(&root), "while-running").expect("saved");

    child.stop();
    let applied = sets::apply_set(&flavor, &sets_path_of(&root), "raid").expect("applied");
    assert_eq!(applied.changed(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

/// A write that fails part way (the second character's file is read-only, so the rename over it is
/// refused) puts the first character's file back as it was.
#[cfg(windows)]
#[test]
fn a_failed_write_puts_the_files_written_before_it_back() {
    let root = temp_game_root("revert");
    let flavor = read(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let locked = wtf_of(&root).join(SECOND);
    let mut permissions = std::fs::metadata(&locked).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&locked, permissions.clone()).expect("read-only");

    let error = sets::apply_set(&flavor, &sets_path_of(&root), "raid").expect_err("failed");
    assert!(
        matches!(&error, Error::SetApplyFailed { set, reverted: true, .. } if set == "raid"),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .ends_with("; every AddOns.txt is as it was"),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "the first file is back");

    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&locked, permissions).expect("writable again");
    let _ = std::fs::remove_dir_all(&root);
}
