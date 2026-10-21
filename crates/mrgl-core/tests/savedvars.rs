//! The SavedVariables listing and the orphan cleanup (task 025) over the synthetic fixture `WTF` tree of task
//! 013, copied into a temporary flavor folder with made-up SavedVariables added next to it and a made-up
//! `Interface\AddOns`: the sizes per account and per character, the orphan rule (a disabled addon, a name
//! that differs only by case and a `Blizzard_` file are never orphans; a folder without a TOC does not count
//! as an addon), the cleanup's byte-for-byte round trip through its zip with nothing else under `WTF`
//! touched, the restore's refusal of that zip, and the refusal while the stand-in client of task 023 runs
//! (the system's `ping.exe` copied into the flavor folder and started, never a game file and never a copy
//! of this test binary, task 063).

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use mrgl_core::Error;
use mrgl_core::backup::{self, WTF_DIR};
use mrgl_core::flavor::{self, Flavor};
use mrgl_core::savedvars::{self, CLEANUP_PREFIX, Status};

fn fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wtf/WTF")
}

const ACCOUNT_SV: &str = "Account/MRGLFIXTURE/SavedVariables";
const CHARACTER_SV: &str = "Account/MRGLFIXTURE/Fixture Realm/Mrgltest/SavedVariables";

/// The files the cleanup moves, relative to `WTF`.
const ORPHAN_FILES: [&str; 4] = [
    "Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua",
    "Account/MRGLFIXTURE/SavedVariables/GoneAddon.lua.bak",
    "Account/MRGLFIXTURE/SavedVariables/NoTocFolder.lua",
    "Account/MRGLFIXTURE/Fixture Realm/Mrgltest/SavedVariables/GoneToo.lua.bak",
];

/// `size` bytes of a made-up SavedVariables file.
fn lua(name: &str, size: usize) -> Vec<u8> {
    let mut text = format!("{name}DB = {{\n").into_bytes();
    text.resize(size - 2, b'x');
    text.extend_from_slice(b"}\n");
    text
}

/// A synthetic game folder under the temp dir with a `_forever_` flavor: the fixture `WTF` tree with these
/// SavedVariables added, and an AddOns folder with `MrglFixture`, `MrglCase`, `DisabledAddon` (disabled in
/// the character's `AddOns.txt`) and `NoTocFolder` (no TOC).
///
/// Account-wide: `MrglFixture` (`.lua` and `.bak`, the fixture's), `GoneAddon` (`.lua` and `.bak`, no
/// folder: an orphan), `Blizzard_Console` (the client's), `mrglcase` (the `MrglCase` folder in another
/// case), `NoTocFolder` (a folder without a TOC: an orphan) and a `notes.txt` that is not a SavedVariables
/// file. Per character: `MrglFixture` (the fixture's), `DisabledAddon` and a lone `GoneToo.lua.bak` (an
/// orphan).
fn savedvars_game(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("mrgl-savedvars-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let flavor = root.join("_forever_");
    std::fs::create_dir_all(&flavor).expect("the flavor folder");
    std::fs::write(
        root.join(".build.info"),
        "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\n",
    )
    .expect(".build.info");
    std::fs::write(
        flavor.join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    )
    .expect(".flavor.info");
    let wtf = flavor.join(WTF_DIR);
    copy_tree(&fixture_wtf(), &wtf);
    for (folder, file, size) in [
        (ACCOUNT_SV, "GoneAddon.lua", 300),
        (ACCOUNT_SV, "GoneAddon.lua.bak", 200),
        (ACCOUNT_SV, "Blizzard_Console.lua", 40),
        (ACCOUNT_SV, "mrglcase.lua", 30),
        (ACCOUNT_SV, "NoTocFolder.lua", 120),
        (ACCOUNT_SV, "notes.txt", 50),
        (CHARACTER_SV, "DisabledAddon.lua", 60),
        (CHARACTER_SV, "GoneToo.lua.bak", 90),
    ] {
        std::fs::write(wtf.join(folder).join(file), lua(file, size)).expect("a file");
    }
    let addons_txt = wtf.join("Account/MRGLFIXTURE/Fixture Realm/Mrgltest/AddOns.txt");
    let mut text = std::fs::read(&addons_txt).expect("AddOns.txt");
    text.extend_from_slice(b"DisabledAddon: disabled\n");
    std::fs::write(&addons_txt, text).expect("AddOns.txt");
    let addons = flavor.join("Interface").join("AddOns");
    for folder in ["MrglFixture", "MrglCase", "DisabledAddon"] {
        std::fs::create_dir_all(addons.join(folder)).expect("an addon folder");
        std::fs::write(
            addons.join(folder).join(format!("{folder}.toc")),
            format!("## Interface: 11508\n## Title: {folder}\n"),
        )
        .expect("a TOC");
    }
    std::fs::create_dir_all(addons.join("NoTocFolder")).expect("a folder");
    std::fs::write(
        addons.join("NoTocFolder").join("readme.txt"),
        "no TOC here\n",
    )
    .expect("a file");
    root
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

fn read(root: &Path) -> Flavor {
    flavor::read_flavor(&root.join("_forever_")).expect("a flavor")
}

fn wtf_of(root: &Path) -> PathBuf {
    root.join("_forever_").join(WTF_DIR)
}

fn backups_of(root: &Path) -> PathBuf {
    root.join("home").join("backups").join("wowf")
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).expect("a file").len()
}

/// The zip's file entries, by name, with their bytes.
fn zip_entries(zip: &Path) -> BTreeMap<String, Vec<u8>> {
    let file = std::fs::File::open(zip).expect("the zip");
    let mut archive = zip::ZipArchive::new(file).expect("a readable zip");
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("an entry");
        if entry.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("the bytes");
        entries.insert(entry.name().expect("a name").into_owned(), bytes);
    }
    entries
}

#[test]
fn the_listing_has_every_file_with_its_size_and_marks_the_orphans() {
    let root = savedvars_game("listing");
    let flavor = read(&root);
    let wtf = wtf_of(&root);
    let before = snapshot(&wtf);

    let listing = savedvars::list_saved_variables(&flavor).expect("a listing");
    assert_eq!(listing.wtf, wtf);
    assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);
    let found: BTreeMap<(String, String), Status> = listing
        .files
        .iter()
        .map(|file| ((file.place(), file.addon.clone()), file.status))
        .collect();
    let account = "MRGLFIXTURE".to_string();
    let character = "MRGLFIXTURE / Fixture Realm / Mrgltest".to_string();
    let expected: BTreeMap<(String, String), Status> = [
        ((account.clone(), "MrglFixture"), Status::Installed),
        ((account.clone(), "GoneAddon"), Status::Orphan),
        ((account.clone(), "Blizzard_Console"), Status::Client),
        ((account.clone(), "mrglcase"), Status::Installed),
        ((account.clone(), "NoTocFolder"), Status::Orphan),
        ((character.clone(), "MrglFixture"), Status::Installed),
        ((character.clone(), "DisabledAddon"), Status::Installed),
        ((character.clone(), "GoneToo"), Status::Orphan),
    ]
    .into_iter()
    .map(|((place, addon), status)| ((place, addon.to_string()), status))
    .collect();
    assert_eq!(found, expected, "notes.txt is not a SavedVariables file");
    assert_eq!(listing.files.len(), 8, "a .lua and its .bak are one entry");

    // The sizes: every entry is its .lua and its .bak together, the largest first.
    let gone = listing
        .files
        .iter()
        .find(|file| file.addon == "GoneAddon")
        .expect("GoneAddon");
    assert_eq!(
        (gone.lua_bytes, gone.bak_bytes, gone.bytes),
        (300, 200, 500)
    );
    assert_eq!(gone.realm, None);
    assert_eq!(gone.character, None);
    let fixture_account = wtf.join(ACCOUNT_SV).join("MrglFixture.lua");
    let fixture = listing
        .files
        .iter()
        .find(|file| file.addon == "MrglFixture" && file.character.is_none())
        .expect("MrglFixture");
    assert_eq!(fixture.lua.as_deref(), Some(fixture_account.as_path()));
    assert_eq!(
        fixture.bytes,
        file_size(&fixture_account) + file_size(&wtf.join(ACCOUNT_SV).join("MrglFixture.lua.bak"))
    );
    let lone = listing
        .files
        .iter()
        .find(|file| file.addon == "GoneToo")
        .expect("GoneToo");
    assert_eq!(lone.lua, None);
    assert_eq!((lone.lua_bytes, lone.bak_bytes, lone.bytes), (0, 90, 90));
    assert_eq!(lone.realm.as_deref(), Some("Fixture Realm"));
    assert_eq!(lone.character.as_deref(), Some("Mrgltest"));
    assert!(
        listing
            .files
            .windows(2)
            .all(|pair| pair[0].bytes >= pair[1].bytes),
        "sorted by size"
    );
    assert_eq!(listing.files[0].addon, "GoneAddon");
    assert_eq!(
        listing.bytes,
        listing.files.iter().map(|file| file.bytes).sum::<u64>()
    );
    assert_eq!(listing.orphans, 3);
    assert_eq!(listing.orphan_bytes, 500 + 120 + 90);

    let json = serde_json::to_value(&listing).expect("serializes");
    assert_eq!(json["files"][0]["addon"], "GoneAddon");
    assert_eq!(json["files"][0]["status"], "orphan");
    assert_eq!(json["orphans"], 3);

    assert_eq!(snapshot(&wtf), before, "a listing writes nothing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_cleanup_moves_the_orphans_into_a_zip_byte_for_byte_and_touches_nothing_else() {
    let root = savedvars_game("clean");
    let flavor = read(&root);
    let wtf = wtf_of(&root);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);

    let cleaned = savedvars::clean_orphans(&flavor, &backups).expect("cleaned");
    let zip = cleaned.zip.clone().expect("a zip");
    assert_eq!(zip.parent(), Some(backups.as_path()));
    let name = zip.file_name().expect("a name").to_string_lossy();
    assert!(
        name.starts_with(CLEANUP_PREFIX) && name.ends_with(".zip"),
        "{name}"
    );
    assert_eq!(
        name.len(),
        "SavedVariables-2026-10-22-101010.zip".len(),
        "{name}"
    );
    assert!(!backups.join(format!("{name}.part")).exists());
    assert_eq!(cleaned.moved.len(), 3);
    assert!(cleaned.moved.iter().all(|file| file.is_orphan()));
    assert_eq!(cleaned.files, 4);
    assert_eq!(cleaned.bytes, 500 + 120 + 90);

    // The zip holds exactly the orphans and their .bak files, under their paths in WTF, byte for byte.
    let entries = zip_entries(&zip);
    let expected: BTreeMap<String, Vec<u8>> = ORPHAN_FILES
        .iter()
        .map(|path| (path.to_string(), before[*path].clone()))
        .collect();
    assert_eq!(entries, expected);

    // Moved, not copied: the orphans are gone and nothing else under WTF changed.
    let mut without_orphans = before.clone();
    for path in ORPHAN_FILES {
        without_orphans.remove(path);
    }
    let after = snapshot(&wtf);
    assert_eq!(after, without_orphans);

    // Unzipped into WTF by hand, the files are back where they were.
    for (entry, bytes) in &entries {
        std::fs::write(wtf.join(entry), bytes).expect("unzipped");
    }
    assert_eq!(snapshot(&wtf), before, "the round trip");

    // Once more: the orphans are back, so a second cleanup moves them into a second zip; after it, a
    // third has nothing to move and writes nothing.
    let second = savedvars::clean_orphans(&flavor, &backups).expect("cleaned again");
    assert_ne!(second.zip, Some(zip.clone()));
    let third = savedvars::clean_orphans(&flavor, &backups).expect("nothing to clean");
    assert_eq!(third.zip, None);
    assert!(third.moved.is_empty());
    assert_eq!(third.files, 0);
    let zips = std::fs::read_dir(&backups).expect("the backups").count();
    assert_eq!(zips, 2, "no zip for nothing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn without_an_addons_folder_nothing_is_an_orphan_and_nothing_moves() {
    let root = savedvars_game("no-addons");
    std::fs::remove_dir_all(root.join("_forever_").join("Interface")).expect("no AddOns folder");
    let flavor = read(&root);
    let wtf = wtf_of(&root);
    let before = snapshot(&wtf);

    let listing = savedvars::list_saved_variables(&flavor).expect("a listing");
    assert_eq!(listing.orphans, 0);
    assert!(
        listing
            .files
            .iter()
            .all(|file| file.status == Status::Unknown || file.status == Status::Client),
        "{:?}",
        listing.files
    );
    assert_eq!(listing.warnings.len(), 1, "{:?}", listing.warnings);
    assert!(
        listing.warnings[0].starts_with("no AddOns folder at ")
            && listing.warnings[0].ends_with(": no SavedVariables file is marked an orphan"),
        "{}",
        listing.warnings[0]
    );

    let backups = backups_of(&root);
    let cleaned = savedvars::clean_orphans(&flavor, &backups).expect("nothing to clean");
    assert_eq!(cleaned.zip, None);
    assert!(!backups.exists(), "no backups folder for nothing");
    assert_eq!(snapshot(&wtf), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_flavor_without_wtf_has_no_savedvariables() {
    let root = savedvars_game("no-wtf");
    std::fs::remove_dir_all(wtf_of(&root)).expect("no WTF");
    let flavor = read(&root);
    let listing = savedvars::list_saved_variables(&flavor).expect("a listing");
    assert!(listing.files.is_empty());
    assert_eq!(listing.bytes, 0);
    let cleaned = savedvars::clean_orphans(&flavor, &backups_of(&root)).expect("nothing");
    assert_eq!(cleaned.zip, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_restore_refuses_the_cleanup_zip() {
    let root = savedvars_game("restore");
    let flavor = read(&root);
    let backups = backups_of(&root);
    let zip = savedvars::clean_orphans(&flavor, &backups)
        .expect("cleaned")
        .zip
        .expect("a zip");
    let wtf = wtf_of(&root);
    let before = snapshot(&wtf);
    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("refused");
    assert!(
        matches!(&error, Error::NotAWtfBackup { path } if path == &zip),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "not a WTF backup: {} holds the SavedVariables a cleanup moved; unzip it into the WTF folder by hand to put them back",
            zip.display()
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing moved");
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
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
fn the_cleanup_is_refused_while_the_client_runs_and_the_listing_is_not() {
    let root = savedvars_game("client");
    let flavor = read(&root);
    let wtf = wtf_of(&root);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);

    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let error = savedvars::clean_orphans(&flavor, &backups).expect_err("refused");
    assert!(
        matches!(&error, Error::ClientRunning { flavor, client } if flavor == "_forever_" && client.pid == child.id()),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(snapshot(&wtf), before, "nothing was touched");
    assert!(!backups.exists(), "no zip either");
    // The listing only reads WTF, so it goes on while the client runs.
    let listing = savedvars::list_saved_variables(&flavor).expect("a listing");
    assert_eq!(listing.orphans, 3);

    child.stop();
    let cleaned = savedvars::clean_orphans(&flavor, &backups).expect("cleaned");
    assert!(cleaned.zip.is_some());
    let _ = std::fs::remove_dir_all(&root);
}
