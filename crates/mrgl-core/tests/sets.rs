//! The addon sets (task 014) over the synthetic fixture `WTF` tree with two characters
//! (`tests/fixtures/addon-sets/`): every `AddOns.txt` round-trips byte for byte, a save reads the
//! characters' states and leaves out what they disagree on, an apply changes exactly the listed addons in
//! the client's own format and leaves every other byte of `WTF` as it was, a second apply writes nothing,
//! and an apply with the stand-in client running (the pattern of the 023 and 063 tests: a copy of the
//! system's `ping.exe` in the flavor folder, never a game file) is refused before anything is written.
//! A set applied to one chosen character (task 027) leaves the other character's file byte for byte, and
//! a name on two realms is refused with the realms listed until `realm` narrows it. An apply round-trips
//! through its undo byte for byte, an edit changes only the named addon, one realm and name under two
//! accounts resolve with the account, and a save keeps an addon that is not installed (task 073).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_core::Error;
use mrgl_core::flavor::{self, Flavor, FlavorId};
use mrgl_core::sets::{self, AddonState, AddonsTxt, SETS_FILE, SetEdit, Target};
#[cfg(windows)]
use mrgl_test_support::stand_in::{SKIPPED, spawn_stand_in};

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
        ["Fixture Realm/Mrgltest", "Second Realm/Grrlwave"],
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

    let saved =
        sets::save_set(&flavor, &path, " everyday ", &Target::EveryCharacter).expect("saved");
    assert_eq!(saved.name, "everyday");
    assert!(!saved.replaced);
    assert_eq!(saved.characters.len(), 2);
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

    let again =
        sets::save_set(&flavor, &path, "EVERYDAY", &Target::EveryCharacter).expect("saved again");
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

    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "Raid",
        &Target::EveryCharacter,
    )
    .expect("applied");
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
    let again = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &Target::EveryCharacter,
    )
    .expect("applied again");
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
    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "solo",
        &Target::EveryCharacter,
    )
    .expect("applied");
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
    let error =
        sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("no set");
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
    let error =
        sets::save_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("unreadable");
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
    let error = sets::apply_set(&flavor, &path, "x", &Target::EveryCharacter).expect_err("refused");
    assert!(
        error.to_string().contains("which is not an addon name"),
        "{error}"
    );

    std::fs::remove_dir_all(wtf_of(&root)).expect("no WTF");
    std::fs::remove_file(&path).expect("no sets");
    let error =
        sets::save_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("no characters");
    assert!(
        matches!(&error, Error::NoCharacters { path } if path == &wtf_of(&root)),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn an_apply_is_refused_while_the_client_runs_and_a_save_is_not() {
    let root = temp_game_root("client");
    let flavor = read(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));

    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("{SKIPPED}");
        return;
    };
    let error = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &Target::EveryCharacter,
    )
    .expect_err("refused");
    assert_eq!(
        error.to_string(),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was touched");
    // A save only reads WTF, so it goes on while the client runs.
    sets::save_set(
        &flavor,
        &sets_path_of(&root),
        "while-running",
        &Target::EveryCharacter,
    )
    .expect("saved");

    child.stop();
    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &Target::EveryCharacter,
    )
    .expect("applied");
    assert_eq!(applied.changed(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

/// A write that fails part way (the last character's file is read-only, so the rename over it is
/// refused) puts the files written before it back as they were: the first character's bytes, and no
/// file where the apply created one (task 117). The record of the apply before is back too.
#[cfg(windows)]
#[test]
fn a_failed_write_puts_the_files_written_before_it_back() {
    let root = temp_game_root("revert");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "solo", &[("QuestArrows", AddonState::Enabled)]);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    sets::apply_set(&flavor, &path, "solo", &Target::EveryCharacter).expect("the apply before");
    let record_before =
        std::fs::read(sets::undo_dir_of(&path).join(sets::UNDO_FILE)).expect("its record");
    // A character without AddOns.txt, first by name: the apply creates its file before the failure.
    std::fs::create_dir_all(wtf_of(&root).join("Account/MRGLSETS/Fixture Realm/Aaa"))
        .expect("a character folder");
    let before = snapshot(&wtf_of(&root));
    let locked = wtf_of(&root).join(SECOND);
    let mut permissions = std::fs::metadata(&locked).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&locked, permissions.clone()).expect("read-only");

    let error = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &Target::EveryCharacter,
    )
    .expect_err("failed");
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
    assert_eq!(
        snapshot(&wtf_of(&root)),
        before,
        "the first file is back, the created one is gone"
    );
    assert_eq!(
        std::fs::read(sets::undo_dir_of(&path).join(sets::UNDO_FILE)).expect("a record"),
        record_before,
        "the record of the apply before is back"
    );

    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&locked, permissions).expect("writable again");
    let _ = std::fs::remove_dir_all(&root);
}

/// The first apply of a flavor fails with every file put back (task 117): no record is left behind, so
/// there is nothing to undo, as before the apply.
#[cfg(windows)]
#[test]
fn a_failed_first_apply_leaves_no_record_behind() {
    let root = temp_game_root("revert-first");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let locked = wtf_of(&root).join(FIRST);
    let mut permissions = std::fs::metadata(&locked).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&locked, permissions.clone()).expect("read-only");

    let error =
        sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("failed");
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&locked, permissions).expect("writable again");
    assert!(
        matches!(&error, Error::SetApplyFailed { set, reverted: true, .. } if set == "raid"),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "every file is as it was");
    assert!(
        !sets::undo_dir_of(&path).exists(),
        "no record of an apply that wrote nothing"
    );
    let error = sets::undo_apply(&flavor, &path).expect_err("nothing to undo");
    assert!(matches!(&error, Error::NothingToUndo { .. }), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

// Task 027: a set applied to one chosen character; the other character's file stays byte for byte.

fn one(name: &str, realm: Option<&str>) -> Target {
    Target::Character {
        name: name.to_string(),
        realm: realm.map(str::to_string),
    }
}

fn modified(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .expect("a time")
}

#[test]
fn an_apply_to_one_character_leaves_the_other_byte_for_byte() {
    let root = temp_game_root("one-character");
    let flavor = read(&root);
    write_set(
        &root,
        "raid",
        &[
            ("BagSorter", AddonState::Disabled),
            ("BossTimers", AddonState::Enabled),
        ],
    );
    let before = snapshot(&wtf_of(&root));
    let second_time = modified(&wtf_of(&root).join(SECOND));
    std::thread::sleep(std::time::Duration::from_millis(20));

    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("grrlWAVE", None),
    )
    .expect("applied");
    assert_eq!(applied.characters.len(), 1);
    assert_eq!(
        applied.characters[0].character.label(),
        "Second Realm/Grrlwave"
    );
    let after = snapshot(&wtf_of(&root));
    assert_eq!(
        after[FIRST], before[FIRST],
        "the other character, byte for byte"
    );
    assert_eq!(
        after[SECOND],
        b"QuestArrows: enabled\n# a line the client never wrote, kept as it is\nBagSorter: disabled\nBossTimers: enabled"
    );
    assert_ne!(modified(&wtf_of(&root).join(SECOND)), second_time);

    // The other way round: the first character only, the second keeps what it has now.
    let first_time = modified(&wtf_of(&root).join(FIRST));
    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", Some("fixture realm")),
    )
    .expect("applied");
    assert_eq!(
        applied.characters[0].changed,
        [("BagSorter".to_string(), AddonState::Disabled)]
    );
    let again = snapshot(&wtf_of(&root));
    assert_eq!(again[SECOND], after[SECOND]);
    assert_ne!(again[FIRST], after[FIRST]);
    assert_ne!(modified(&wtf_of(&root).join(FIRST)), first_time);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_save_from_one_character_takes_that_characters_file() {
    let root = temp_game_root("save-one");
    let flavor = read(&root);
    let saved = sets::save_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", None),
    )
    .expect("saved");
    assert_eq!(saved.characters.len(), 1);
    assert!(saved.conflicts.is_empty());
    assert_eq!(
        saved.set.addons,
        BTreeMap::from([
            ("BagSorter".to_string(), AddonState::Enabled),
            ("BossTimers".to_string(), AddonState::Enabled),
            ("QuestArrows".to_string(), AddonState::Disabled),
            ("RaidFrames".to_string(), AddonState::Enabled),
        ])
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_name_on_two_realms_is_refused_with_the_realms_and_a_realm_narrows_it() {
    let root = temp_game_root("ambiguous");
    let flavor = read(&root);
    let twin = wtf_of(&root).join("Account/MRGLSETS/Second Realm/Mrgltest");
    std::fs::create_dir_all(&twin).expect("a second Mrgltest");
    std::fs::write(twin.join("AddOns.txt"), b"BagSorter: enabled\r\n").expect("its file");
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));

    let error = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("mrgltest", None),
    )
    .expect_err("ambiguous");
    assert!(
        matches!(&error, Error::AmbiguousCharacter { count: 2, .. }),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "character mrgltest matches 2 characters (flavor forever): Fixture Realm, Second Realm; name one with --realm"
    );
    let error = sets::save_set(
        &flavor,
        &sets_path_of(&root),
        "other",
        &one("Mrgltest", None),
    )
    .expect_err("ambiguous for a save too");
    assert!(
        matches!(&error, Error::AmbiguousCharacter { .. }),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was written");

    let error = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", Some("Third Realm")),
    )
    .expect_err("no such realm");
    assert_eq!(
        error.to_string(),
        "character not found: Mrgltest on Third Realm (flavor forever); the characters: Mrgltest (Fixture Realm), Grrlwave (Second Realm), Mrgltest (Second Realm)"
    );

    let applied = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", Some("Second Realm")),
    )
    .expect("narrowed");
    assert_eq!(applied.characters.len(), 1);
    let after = snapshot(&wtf_of(&root));
    assert_eq!(after[FIRST], before[FIRST]);
    assert_eq!(after[SECOND], before[SECOND]);
    assert_eq!(
        after["Account/MRGLSETS/Second Realm/Mrgltest/AddOns.txt"],
        b"BagSorter: disabled\r\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn one_realm_under_two_accounts_names_the_accounts() {
    let character = |account: &str| sets::Character {
        account: account.to_string(),
        realm: "Fixture Realm".to_string(),
        name: "Mrgltest".to_string(),
        folder: PathBuf::from(account),
    };
    let characters = [character("MRGLONE"), character("MRGLTWO")];
    let error = sets::find_character(
        &characters,
        "Mrgltest",
        Some("Fixture Realm"),
        &FlavorId::parse("wowf"),
    )
    .expect_err("ambiguous");
    assert_eq!(
        error.to_string(),
        "character Mrgltest matches 2 characters (flavor forever): Fixture Realm (MRGLONE), Fixture Realm (MRGLTWO); the same realm is under more than one account: pick the account too"
    );
    let found = sets::find_character_in_account(
        &characters,
        "Mrgltest",
        Some("Fixture Realm"),
        Some("mrgltwo"),
        &FlavorId::parse("wowf"),
    )
    .expect("the account tells them apart");
    assert_eq!(found.account, "MRGLTWO");
}

#[cfg(windows)]
#[test]
fn an_apply_to_one_character_is_refused_while_the_client_runs() {
    let root = temp_game_root("one-client");
    let flavor = read(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("{SKIPPED}");
        return;
    };
    let error = sets::apply_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", None),
    )
    .expect_err("refused");
    assert!(matches!(&error, Error::ClientRunning { .. }), "{error}");
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was touched");
    child.stop();
    let _ = std::fs::remove_dir_all(&root);
}

// Task 073: the undo of an apply, the edit of a stored set, `--account`, and the save warning.

const NEWBIE: &str = "Account/MRGLSETS/Second Realm/Newbie/AddOns.txt";

#[test]
fn an_apply_round_trips_through_undo_byte_for_byte() {
    let root = temp_game_root("undo");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    // A character without AddOns.txt: the apply creates one, the undo removes it again.
    std::fs::create_dir_all(wtf_of(&root).join("Account/MRGLSETS/Second Realm/Newbie"))
        .expect("a character folder");
    write_set(
        &root,
        "raid",
        &[
            ("RaidFrames", AddonState::Enabled),
            ("BagSorter", AddonState::Disabled),
            ("NewMeter", AddonState::Enabled),
        ],
    );
    let before = snapshot(&wtf_of(&root));

    let error = sets::undo_apply(&flavor, &path).expect_err("nothing yet");
    assert!(matches!(&error, Error::NothingToUndo { .. }), "{error}");
    assert_eq!(sets::undo_record(&flavor, &path).expect("read"), None);

    let applied =
        sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    assert_eq!(applied.characters.len(), 3);
    let undo_dir = sets::undo_dir_of(&path);
    assert_eq!(undo_dir, root.join("home").join("wowf").join("set-undo"));
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(undo_dir.join(sets::UNDO_FILE)).expect("the record"))
            .expect("JSON");
    assert_eq!(record["format"], "mrgl-set-undo");
    assert_eq!(record["set"], "raid");
    assert_eq!(record["files"].as_array().map(Vec::len), Some(3));
    let applied_tree = snapshot(&wtf_of(&root));
    // Task 085: the record as the window reads it, before the undo, without a write.
    let named = sets::undo_record(&flavor, &path)
        .expect("read")
        .expect("a record");
    assert_eq!((named.set.as_str(), named.files.len()), ("raid", 3));
    assert_eq!(
        named
            .files
            .iter()
            .filter(|file| file.kept.is_none())
            .count(),
        1,
        "the file the apply created"
    );
    assert_eq!(snapshot(&wtf_of(&root)), applied_tree);
    assert!(applied_tree.contains_key(NEWBIE));

    // A second apply of the same set changes nothing and keeps the record of the first.
    let again = sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("again");
    assert_eq!(again.changed(), 0);

    let undone = sets::undo_apply(&flavor, &path).expect("undone");
    assert_eq!(undone.set, "raid");
    let restored: Vec<String> = undone.restored.iter().map(sets::Character::label).collect();
    assert_eq!(
        restored,
        ["Fixture Realm/Mrgltest", "Second Realm/Grrlwave"]
    );
    let removed: Vec<String> = undone.removed.iter().map(sets::Character::label).collect();
    assert_eq!(removed, ["Second Realm/Newbie"]);
    assert!(undone.warnings.is_empty(), "{:?}", undone.warnings);
    assert_eq!(
        snapshot(&wtf_of(&root)),
        before,
        "every AddOns.txt byte for byte, the created one gone"
    );
    assert!(!undo_dir.exists(), "the record is used once");
    let error = sets::undo_apply(&flavor, &path).expect_err("used once");
    assert_eq!(
        error.to_string(),
        "nothing to undo in _forever_: no set apply is recorded; an apply that changes a file keeps what it replaced, and an undo puts it back once"
    );

    // Two applies: the undo puts back the second one's files only (the last apply).
    write_set(&root, "solo", &[("QuestArrows", AddonState::Enabled)]);
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("raid");
    let after_raid = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "solo", &one("Mrgltest", None)).expect("solo");
    assert_ne!(snapshot(&wtf_of(&root)), after_raid);
    let undone = sets::undo_apply(&flavor, &path).expect("undone");
    assert_eq!(undone.set, "solo");
    assert_eq!(snapshot(&wtf_of(&root)), after_raid);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_undo_warns_about_a_file_changed_since_the_apply_and_puts_it_back_all_the_same() {
    let root = temp_game_root("undo-changed");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    // The client wrote the first character's file again at logout.
    std::fs::write(wtf_of(&root).join(FIRST), b"RaidFrames: disabled\r\n").expect("written");
    let undone = sets::undo_apply(&flavor, &path).expect("undone");
    assert_eq!(
        undone.warnings,
        [
            "Fixture Realm/Mrgltest: AddOns.txt changed since the apply (the client writes it at logout); put back as it was before the apply all the same"
        ]
    );
    assert_eq!(snapshot(&wtf_of(&root)), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_undo_record_that_names_a_path_is_refused() {
    let root = temp_game_root("undo-path");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    let record_path = sets::undo_dir_of(&path).join(sets::UNDO_FILE);
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record_path).expect("the record")).expect("JSON");
    record["files"][0]["realm"] = serde_json::json!("..");
    std::fs::write(&record_path, record.to_string()).expect("written");
    let applied = snapshot(&wtf_of(&root));
    let error = sets::undo_apply(&flavor, &path).expect_err("refused");
    assert!(matches!(&error, Error::SetUndoUnreadable { .. }), "{error}");
    assert_eq!(snapshot(&wtf_of(&root)), applied, "nothing was written");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn an_undo_is_refused_while_the_client_runs() {
    let root = temp_game_root("undo-client");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    let applied = snapshot(&wtf_of(&root));
    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("{SKIPPED}");
        return;
    };
    let error = sets::undo_apply(&flavor, &path).expect_err("refused");
    assert_eq!(
        error.to_string(),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf_of(&root)), applied, "nothing was touched");
    assert!(sets::undo_dir_of(&path).join(sets::UNDO_FILE).is_file());
    child.stop();
    sets::undo_apply(&flavor, &path).expect("undone");
    assert_eq!(snapshot(&wtf_of(&root)), before);
    let _ = std::fs::remove_dir_all(&root);
}

/// The system's process list, which starts the stand-in client once it has answered the first time: the
/// guard before a `WTF` write finds no client, the second look after it finds the one that started during
/// the write (task 124).
#[cfg(windows)]
struct StartsAClient {
    folder: PathBuf,
    started: std::cell::RefCell<Option<mrgl_test_support::stand_in::StandIn>>,
}

#[cfg(windows)]
impl StartsAClient {
    fn new(folder: &Path) -> StartsAClient {
        StartsAClient {
            folder: folder.to_path_buf(),
            started: std::cell::RefCell::new(None),
        }
    }

    fn pid(&self) -> Option<u32> {
        self.started.borrow().as_ref().map(|child| child.id())
    }
}

#[cfg(windows)]
impl mrgl_core::client::ProcessList for StartsAClient {
    fn executables(&self) -> std::io::Result<Vec<mrgl_core::client::RunningProcess>> {
        let answer = mrgl_core::client::SystemProcesses.executables();
        let mut started = self.started.borrow_mut();
        if started.is_none() {
            *started = spawn_stand_in(&self.folder);
        }
        answer
    }
}

#[cfg(windows)]
#[test]
fn a_client_that_started_during_an_apply_or_an_undo_is_a_warning_once_it_is_done() {
    let root = temp_game_root("client-meanwhile");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let line = |write: &str, pid: u32| {
        format!(
            "a client started in _forever_ during {write} (client-stand-in.exe, pid {pid}): it writes WTF again when it logs out, over what {write} wrote; close it and run {write} again if that is lost"
        )
    };

    let processes = StartsAClient::new(&flavor.folder);
    let applied = sets::apply_set_among(
        &processes,
        &flavor,
        &path,
        "raid",
        &Target::EveryCharacter,
        None,
    )
    .expect("applied: the write cannot be held back once done");
    let Some(pid) = processes.pid() else {
        eprintln!("{SKIPPED}");
        return;
    };
    assert!(applied.changed() > 0);
    assert_eq!(applied.warnings, [line("the set apply", pid)]);
    drop(processes);

    let processes = StartsAClient::new(&flavor.folder);
    let undone = sets::undo_apply_among(&processes, &flavor, &path).expect("undone");
    let pid = processes.pid().expect("started as before");
    assert_eq!(undone.warnings, [line("the set undo", pid)]);
    assert_eq!(
        snapshot(&wtf_of(&root)),
        before,
        "the undo is done all the same"
    );
    drop(processes);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_edit_changes_only_the_named_addon_in_the_sets_file() {
    let root = temp_game_root("edit");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    std::fs::create_dir_all(root.join("_forever_/Interface/AddOns/BagSorter")).expect("addon");
    write_set(
        &root,
        "raid",
        &[
            ("BagSorter", AddonState::Disabled),
            ("BossTimers", AddonState::Enabled),
        ],
    );
    write_set(&root, "solo", &[("QuestArrows", AddonState::Enabled)]);
    let wtf_before = snapshot(&wtf_of(&root));
    let sets_before = sets::read_sets_at(&path, &flavor.id).expect("read");

    let edited =
        sets::edit_set(&flavor, &path, "RAID", "bagsorter", SetEdit::Enable).expect("enabled");
    assert_eq!(edited.name, "raid");
    assert_eq!(edited.addon, "BagSorter", "the set's spelling");
    assert_eq!(edited.before, Some(AddonState::Disabled));
    assert_eq!(edited.after, Some(AddonState::Enabled));
    assert!(edited.changed && edited.installed);
    let file = sets::read_sets_at(&path, &flavor.id).expect("read");
    assert_eq!(
        file.sets["raid"].addons,
        BTreeMap::from([
            ("BagSorter".to_string(), AddonState::Enabled),
            ("BossTimers".to_string(), AddonState::Enabled),
        ])
    );
    assert_eq!(file.sets["solo"], sets_before.sets["solo"], "the other set");

    // The state it has already: nothing is written.
    let bytes = std::fs::read(&path).expect("sets.json");
    let same = sets::edit_set(&flavor, &path, "raid", "BagSorter", SetEdit::Enable).expect("same");
    assert!(!same.changed);
    assert_eq!(std::fs::read(&path).expect("sets.json"), bytes);

    // A disable of an addon the set does not have adds it; a drop takes one out.
    let added =
        sets::edit_set(&flavor, &path, "raid", "NewMeter", SetEdit::Disable).expect("added");
    assert_eq!(added.before, None);
    assert!(!added.installed);
    let dropped =
        sets::edit_set(&flavor, &path, "raid", "BossTimers", SetEdit::Drop).expect("dropped");
    assert_eq!(dropped.after, None);
    assert_eq!(
        dropped.set.addons,
        BTreeMap::from([
            ("BagSorter".to_string(), AddonState::Enabled),
            ("NewMeter".to_string(), AddonState::Disabled),
        ])
    );
    let error =
        sets::edit_set(&flavor, &path, "raid", "Missing", SetEdit::Drop).expect_err("not there");
    assert_eq!(
        error.to_string(),
        "set raid (flavor forever) has no addon Missing; its addons: BagSorter, NewMeter"
    );
    let error =
        sets::edit_set(&flavor, &path, "raid", "a:b", SetEdit::Enable).expect_err("no name");
    assert!(matches!(&error, Error::AddonName { .. }), "{error}");
    let error =
        sets::edit_set(&flavor, &path, "nope", "BagSorter", SetEdit::Enable).expect_err("no set");
    assert!(matches!(&error, Error::SetNotFound { .. }), "{error}");
    assert_eq!(
        snapshot(&wtf_of(&root)),
        wtf_before,
        "an edit never touches WTF"
    );
    assert!(
        !sets::undo_dir_of(&path).exists(),
        "and keeps no undo record"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The fixture with a second account folder that has the same realm and character as the first.
fn with_twin_account(root: &Path) -> &'static str {
    const TWIN: &str = "Account/MRGLTWO/Fixture Realm/Mrgltest/AddOns.txt";
    let twin = wtf_of(root).join(TWIN);
    std::fs::create_dir_all(twin.parent().expect("a folder")).expect("the twin's folder");
    std::fs::write(&twin, b"BagSorter: enabled\r\nTwinOnly: enabled\r\n").expect("its file");
    TWIN
}

#[test]
fn one_realm_and_name_under_two_accounts_resolve_with_the_account() {
    let root = temp_game_root("account");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    let twin = with_twin_account(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let mrgltest = one("Mrgltest", Some("Fixture Realm"));

    let error = sets::apply_set(&flavor, &path, "raid", &mrgltest).expect_err("ambiguous");
    assert!(
        error
            .to_string()
            .ends_with("more than one account: pick the account too"),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before);

    let applied = sets::apply_set_in_account(&flavor, &path, "raid", &mrgltest, Some("mrgltwo"))
        .expect("the account picks one");
    assert_eq!(applied.characters.len(), 1);
    assert_eq!(applied.characters[0].character.account, "MRGLTWO");
    let after = snapshot(&wtf_of(&root));
    assert_eq!(after[FIRST], before[FIRST], "the other account's file");
    assert_eq!(after[twin], b"BagSorter: disabled\r\nTwinOnly: enabled\r\n");

    let saved = sets::save_set_in_account(
        &flavor,
        &path,
        "twin",
        &one("Mrgltest", None),
        Some("MRGLTWO"),
    )
    .expect("saved from the twin");
    assert_eq!(saved.characters.len(), 1);
    assert!(saved.set.addons.contains_key("TwinOnly"));

    let error = sets::apply_set_in_account(&flavor, &path, "raid", &mrgltest, Some("Nobody"))
        .expect_err("no such account");
    assert_eq!(
        error.to_string(),
        "no character under the account Nobody (flavor forever); the accounts with characters: MRGLSETS, MRGLTWO"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_save_keeps_an_addon_that_is_not_installed_and_names_it() {
    let root = temp_game_root("save-missing");
    let flavor = read(&root);
    std::fs::create_dir_all(root.join("_forever_/Interface/AddOns/BagSorter")).expect("addon");
    let saved = sets::save_set(
        &flavor,
        &sets_path_of(&root),
        "raid",
        &one("Mrgltest", None),
    )
    .expect("saved");
    assert_eq!(
        saved.not_installed,
        ["BossTimers", "QuestArrows", "RaidFrames"]
    );
    assert_eq!(saved.set.addons.len(), 4, "every addon is kept");
    let file = sets::read_sets_at(&sets_path_of(&root), &flavor.id).expect("read");
    assert!(file.sets["raid"].addons.contains_key("RaidFrames"));
    let _ = std::fs::remove_dir_all(&root);
}

// --- task 117: the arms the fixture round trips do not reach ---

#[test]
fn a_sets_file_of_another_format_schema_or_flavor_is_refused_and_one_that_is_a_folder_is_an_error()
{
    let root = temp_game_root("sets-file");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("the home");
    for (text, reason) in [
        (
            r#"{"format":"other","schema":1,"flavor":"wowf"}"#,
            "format other instead of mrgl-sets",
        ),
        (
            r#"{"format":"mrgl-sets","schema":2,"flavor":"wowf"}"#,
            "schema 2 instead of 1",
        ),
        (
            r#"{"format":"mrgl-sets","schema":1,"flavor":"wow"}"#,
            "flavor wow instead of wowf",
        ),
    ] {
        std::fs::write(&path, text).expect("a sets file");
        let error = sets::read_sets_at(&path, &flavor.id).expect_err(reason);
        assert!(
            matches!(&error, Error::SetsUnreadable { path: named, reason: given } if named == &path && given == reason),
            "{error}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("kept"),
            text,
            "never overwritten"
        );
    }
    let error = sets::find_set(&flavor.id, &path, "raid").expect_err("unreadable");
    assert!(matches!(&error, Error::SetsUnreadable { .. }), "{error}");

    std::fs::remove_file(&path).expect("no sets file");
    let error = sets::find_set(&flavor.id, &path, "raid").expect_err("no set");
    assert!(matches!(&error, Error::SetNotFound { .. }), "{error}");
    let characters = sets::targets(&flavor, &Target::EveryCharacter).expect("the characters");
    assert_eq!(characters.len(), 2);

    std::fs::create_dir_all(&path).expect("a folder where the file goes");
    let error = sets::read_sets_at(&path, &flavor.id).expect_err("a folder");
    assert!(
        matches!(&error, Error::Io { context: "reading the sets file", path: named, .. } if named == &path),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_sets_file_cannot_be_written_under_a_file_or_over_a_folder_or_at_a_root() {
    let root = temp_game_root("sets-write");
    let path = sets_path_of(&root);
    let file = sets::SetsFile::new(&FlavorId::parse("wowf"));
    let parent = path.parent().expect("a folder").to_path_buf();
    std::fs::create_dir_all(parent.parent().expect("the home")).expect("the home");
    std::fs::write(&parent, b"a file where the folder goes").expect("a file");
    let error = sets::write_sets_at(&path, &file).expect_err("under a file");
    assert!(
        matches!(&error, Error::Io { context: "creating the flavor's folder under the mrgl home", path: named, .. } if named == &parent),
        "{error}"
    );
    std::fs::remove_file(&parent).expect("gone");

    std::fs::create_dir_all(&path).expect("a folder where the file goes");
    let error = sets::write_sets_at(&path, &file).expect_err("over a folder");
    assert!(
        matches!(&error, Error::Io { context: "writing the sets file", path: named, .. } if named == &path),
        "{error}"
    );
    assert!(path.is_dir(), "the folder stays");

    // A path with no parent: no folder to create, and no file to write.
    let drive_root = Path::new(if cfg!(windows) { "C:\\" } else { "/" });
    let error = sets::write_sets_at(drive_root, &file).expect_err("a root");
    assert!(
        matches!(&error, Error::Io { context: "writing the sets file", path: named, .. } if named == drive_root),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_save_passes_over_a_character_without_addons_txt_and_a_line_that_names_no_addon() {
    let root = temp_game_root("save-shapes");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    std::fs::create_dir_all(wtf_of(&root).join("Account/MRGLSETS/Second Realm/Newbie"))
        .expect("a character folder without AddOns.txt");
    let first = wtf_of(&root)
        .join("Account")
        .join("MRGLSETS")
        .join("Fixture Realm")
        .join("Mrgltest")
        .join("AddOns.txt");
    let mut bytes = std::fs::read(&first).expect("AddOns.txt");
    bytes.extend_from_slice(b"Bad/Name: enabled\r\n");
    std::fs::write(&first, bytes).expect("a line that names a path");

    let saved = sets::save_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("saved");
    assert_eq!(
        saved.characters.len(),
        3,
        "the character without a file is read as empty"
    );
    assert!(!saved.set.addons.contains_key("Bad/Name"));
    assert_eq!(
        saved.warnings,
        [format!(
            "left out of set raid: `Bad/Name` in {} is not an addon name",
            first.display()
        )]
    );

    // An AddOns.txt that is a folder cannot be read.
    let newbie = wtf_of(&root).join(NEWBIE);
    std::fs::create_dir_all(&newbie).expect("a folder where the file goes");
    let error =
        sets::save_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("a folder");
    assert!(
        matches!(&error, Error::Io { context: "reading AddOns.txt", path: named, .. } if named == &newbie),
        "{error}"
    );
    std::fs::remove_dir(&newbie).expect("gone");

    // No addon line anywhere: nothing to save.
    for file in [FIRST, SECOND] {
        std::fs::write(wtf_of(&root).join(file), b"# nothing the client wrote\r\n").expect("empty");
    }
    let error =
        sets::save_set(&flavor, &path, "empty", &Target::EveryCharacter).expect_err("nothing");
    assert!(matches!(&error, Error::NothingToSave { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        "nothing to save: no AddOns.txt under WTF of _forever_ names an addon"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_undo_record_that_cannot_be_kept_refuses_the_apply_before_anything_is_written() {
    let root = temp_game_root("undo-keep");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    let fresh = sets::undo_dir_of(&path).with_file_name("set-undo.new");
    std::fs::write(&fresh, b"a file where the fresh record goes").expect("a file");
    let error =
        sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("in the way");
    assert!(
        matches!(&error, Error::Io { context: "keeping the undo record of the apply", path: named, .. } if named == &fresh),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was written");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_record_that_cannot_be_set_aside_or_replaced_refuses_the_apply_before_anything_is_written() {
    use mrgl_test_support::locked;
    let root = temp_game_root("undo-swap");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    let undo_dir = sets::undo_dir_of(&path);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    write_set(&root, "solo", &[("QuestArrows", AddonState::Enabled)]);
    sets::apply_set(&flavor, &path, "solo", &Target::EveryCharacter).expect("the apply before");
    let before = snapshot(&wtf_of(&root));

    // The record before cannot move aside: a file held open under it.
    let lock = locked::file(&undo_dir.join(sets::UNDO_FILE));
    let error = sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("held");
    drop(lock);
    assert!(
        matches!(&error, Error::Io { context: "keeping the undo record of the apply", path: named, .. } if named == &undo_dir),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was written");
    assert!(
        undo_dir.join(sets::UNDO_FILE).is_file(),
        "the record before stays"
    );
    assert!(!undo_dir.with_file_name("set-undo.new").exists());
    assert!(!undo_dir.with_file_name("set-undo.old").exists());

    // A junction whose target is gone where the record goes: nothing to set aside, and the fresh record
    // cannot take the place.
    std::fs::remove_dir_all(&undo_dir).expect("no record");
    let target = root.join("gone");
    std::fs::create_dir_all(&target).expect("a folder");
    mrgl_core::link::create_junction(&undo_dir, &target).expect("a junction");
    std::fs::remove_dir(&target).expect("the target gone");
    let error =
        sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect_err("in the way");
    assert!(
        matches!(&error, Error::Io { context: "keeping the undo record of the apply", path: named, .. } if named == &undo_dir),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), before, "nothing was written");
    assert!(!undo_dir.with_file_name("set-undo.new").exists());
    let _ = std::fs::remove_dir(&undo_dir);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_undo_record_of_another_format_or_flavor_or_with_a_kept_file_missing_is_refused() {
    let root = temp_game_root("undo-record");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    let applied = snapshot(&wtf_of(&root));
    let undo_dir = sets::undo_dir_of(&path);
    let record_path = undo_dir.join(sets::UNDO_FILE);
    let original = std::fs::read_to_string(&record_path).expect("the record");
    let record: serde_json::Value = serde_json::from_str(&original).expect("JSON");

    let mut other_format = record.clone();
    other_format["format"] = serde_json::json!("mrgl-other");
    let mut other_schema = record.clone();
    other_schema["schema"] = serde_json::json!(2);
    let mut other_flavor = record.clone();
    other_flavor["flavor"] = serde_json::json!("wow");
    for (edited, reason) in [
        (
            other_format,
            "format mrgl-other schema 1 instead of mrgl-set-undo schema 1",
        ),
        (
            other_schema,
            "format mrgl-set-undo schema 2 instead of mrgl-set-undo schema 1",
        ),
        (other_flavor, "flavor wow instead of wowf"),
    ] {
        std::fs::write(&record_path, edited.to_string()).expect("written");
        let error = sets::undo_apply(&flavor, &path).expect_err(reason);
        assert!(
            matches!(&error, Error::SetUndoUnreadable { path: named, reason: given } if named == &undo_dir && given == reason),
            "{error}"
        );
        let error = sets::undo_record(&flavor, &path).expect_err(reason);
        assert!(matches!(&error, Error::SetUndoUnreadable { .. }), "{error}");
    }
    std::fs::write(&record_path, "{ not json").expect("written");
    let error = sets::undo_apply(&flavor, &path).expect_err("not JSON");
    assert!(matches!(&error, Error::SetUndoUnreadable { .. }), "{error}");

    std::fs::write(&record_path, &original).expect("the record again");
    std::fs::remove_file(undo_dir.join("1-before.txt")).expect("the kept file gone");
    let error = sets::undo_apply(&flavor, &path).expect_err("a kept file missing");
    assert!(
        matches!(&error, Error::SetUndoUnreadable { reason, .. } if reason.starts_with("1-before.txt: ")),
        "{error}"
    );

    std::fs::remove_file(&record_path).expect("no record file");
    std::fs::create_dir_all(&record_path).expect("a folder where the record goes");
    let error = sets::undo_apply(&flavor, &path).expect_err("a folder");
    assert!(
        matches!(&error, Error::Io { context: "reading the undo record", path: named, .. } if named == &record_path),
        "{error}"
    );
    assert_eq!(snapshot(&wtf_of(&root)), applied, "nothing was written");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_undo_passes_over_a_character_folder_that_is_gone_and_a_file_that_is_as_it_was() {
    let root = temp_game_root("undo-shapes");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    // The client put the first file back as it was; the second character is gone.
    let first = wtf_of(&root).join(FIRST);
    std::fs::write(&first, &before[FIRST]).expect("as it was");
    let first_time = modified(&first);
    std::fs::remove_dir_all(wtf_of(&root).join("Account/MRGLSETS/Second Realm/Grrlwave"))
        .expect("the character gone");
    std::thread::sleep(std::time::Duration::from_millis(20));

    let undone = sets::undo_apply(&flavor, &path).expect("undone");
    assert_eq!(
        undone.warnings,
        [
            "Fixture Realm/Mrgltest: AddOns.txt changed since the apply (the client writes it at logout); put back as it was before the apply all the same",
            "Second Realm/Grrlwave: the character folder is gone, nothing to put back",
        ]
    );
    let restored: Vec<String> = undone.restored.iter().map(sets::Character::label).collect();
    assert_eq!(restored, ["Fixture Realm/Mrgltest"]);
    assert!(undone.removed.is_empty());
    assert_eq!(
        undone.summary("_forever_").0,
        mrgl_core::voice::Kind::Warning
    );
    assert_eq!(
        modified(&first),
        first_time,
        "a file that is as it was is not written"
    );
    assert_eq!(std::fs::read(&first).expect("bytes"), before[FIRST]);
    assert!(!sets::undo_dir_of(&path).exists(), "the record is used");
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_failed_undo_write_puts_the_files_written_before_it_back_and_keeps_the_record() {
    use mrgl_test_support::locked;
    let root = temp_game_root("undo-revert");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    let applied = snapshot(&wtf_of(&root));
    let second = wtf_of(&root)
        .join("Account")
        .join("MRGLSETS")
        .join("Second Realm")
        .join("Grrlwave")
        .join("AddOns.txt");
    // A handle that shares reads and not the replacement: the undo reads the file, the write over it
    // fails.
    let lock = locked::file_sharing_reads(&second);
    let error = sets::undo_apply(&flavor, &path).expect_err("the second file is held");
    drop(lock);
    assert!(
        matches!(&error, Error::SetUndoFailed { set, reverted: true, .. } if set == "raid"),
        "{error}"
    );
    let text = error.to_string();
    assert!(
        text.starts_with(&format!("undoing set raid failed: {}: ", second.display()))
            && text.ends_with("; every AddOns.txt is as it was before the undo"),
        "{text}"
    );
    assert_eq!(
        snapshot(&wtf_of(&root)),
        applied,
        "the first file is back as applied"
    );
    assert!(
        sets::undo_dir_of(&path).join(sets::UNDO_FILE).is_file(),
        "the record stays for the next undo"
    );
    sets::undo_apply(&flavor, &path).expect("undone once the file is free");
    assert_eq!(snapshot(&wtf_of(&root)), before);
    let _ = std::fs::remove_dir_all(&root);
}

/// An undo whose removal of a created file fails (task 117): the files put back before it are put back
/// as they were at the start of the undo, a file gone since the apply gone again, and the record stays.
#[cfg(windows)]
#[test]
fn a_failed_removal_in_an_undo_puts_the_earlier_files_back_as_they_were() {
    use mrgl_test_support::locked;
    let root = temp_game_root("undo-revert-removal");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    std::fs::create_dir_all(wtf_of(&root).join("Account/MRGLSETS/Second Realm/Newbie"))
        .expect("a character folder");
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    // The first character's file is gone since the apply; the created one is held.
    std::fs::remove_file(wtf_of(&root).join(FIRST)).expect("the first file gone");
    let applied = snapshot(&wtf_of(&root));
    let created = wtf_of(&root)
        .join("Account")
        .join("MRGLSETS")
        .join("Second Realm")
        .join("Newbie")
        .join("AddOns.txt");
    let held = locked::file_sharing_reads(&created);
    let error = sets::undo_apply(&flavor, &path).expect_err("the created file is held");
    drop(held);
    assert!(
        matches!(&error, Error::SetUndoFailed { set, reverted: true, .. } if set == "raid"),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .starts_with(&format!("undoing set raid failed: {}: ", created.display())),
        "{error}"
    );
    assert_eq!(
        snapshot(&wtf_of(&root)),
        applied,
        "the first file is gone again, the second is as the apply left it"
    );
    assert!(
        sets::undo_dir_of(&path).join(sets::UNDO_FILE).is_file(),
        "the record stays for another undo"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn an_undo_record_that_cannot_be_removed_is_a_warning() {
    use mrgl_test_support::locked;
    let root = temp_game_root("undo-stays");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "raid", &[("BagSorter", AddonState::Disabled)]);
    let before = snapshot(&wtf_of(&root));
    sets::apply_set(&flavor, &path, "raid", &Target::EveryCharacter).expect("applied");
    let undo_dir = sets::undo_dir_of(&path);
    let extra = undo_dir.join("extra.txt");
    std::fs::write(&extra, b"not part of the record").expect("a file");
    let lock = locked::file(&extra);
    let undone = sets::undo_apply(&flavor, &path).expect("undone");
    drop(lock);
    assert_eq!(undone.warnings.len(), 1, "{:?}", undone.warnings);
    assert!(
        undone.warnings[0].starts_with(&format!(
            "the undo record {} could not be removed (",
            undo_dir.display()
        )) && undone.warnings[0].ends_with("); an undo now would put the same files back again"),
        "{}",
        undone.warnings[0]
    );
    assert_eq!(
        undone.summary("_forever_").0,
        mrgl_core::voice::Kind::Warning
    );
    assert_eq!(
        snapshot(&wtf_of(&root)),
        before,
        "the files are back all the same"
    );
    assert!(undo_dir.join(sets::UNDO_FILE).is_file(), "the record stays");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_drop_from_a_set_without_addons_lists_none() {
    let root = temp_game_root("edit-empty");
    let flavor = read(&root);
    let path = sets_path_of(&root);
    write_set(&root, "empty", &[]);
    let error =
        sets::edit_set(&flavor, &path, "empty", "BagSorter", SetEdit::Drop).expect_err("not there");
    assert_eq!(
        error.to_string(),
        "set empty (flavor forever) has no addon BagSorter; its addons: none"
    );
    let _ = std::fs::remove_dir_all(&root);
}
