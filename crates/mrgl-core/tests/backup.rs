//! The WTF backup and restore (task 013) over the synthetic fixture tree: a backup round-trips the
//! folder byte for byte with the file times kept, a restore refuses an entry that would land outside
//! `WTF` or that it cannot unpack before anything moves (the shared check of `zip_entry`), a failed restore puts the previous `WTF` back, the listing reads the
//! backups folder, and a restore with the stand-in client running (the pattern of the 023 tests: this test
//! binary copied into the flavor folder and started, never a game file) is refused with the documented
//! line.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::{Child, Command, Stdio};
use std::time::{Duration, UNIX_EPOCH};

use mrgl_core::Error;
use mrgl_core::backup::{self, ASIDE_PREFIX, BACKUP_PREFIX, WTF_DIR};
use mrgl_core::flavor::{self, Flavor, FlavorId};
use mrgl_core::zip_entry::ZipRole;

fn fixture_wtf() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wtf/WTF")
}

/// A synthetic game folder under the temp dir with a `_forever_` flavor; `with_wtf` copies the fixture
/// tree in, with an empty folder next to it and dated files, so the round trip has times to keep.
fn temp_game_root(name: &str, with_wtf: bool) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mrgl-backup-test-{}-{name}", std::process::id()));
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
    if with_wtf {
        let wtf = root.join("_forever_").join(WTF_DIR);
        copy_tree(&fixture_wtf(), &wtf);
        std::fs::create_dir_all(wtf.join("Account").join("MRGLFIXTURE").join("EmptyFolder"))
            .expect("an empty folder");
        date_files(&wtf, 1_700_000_000);
    }
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

/// Every file gets its own second, counting up from `start` in the order of the snapshot.
fn date_files(dir: &Path, start: u64) {
    for (index, (relative, _)) in snapshot(dir).iter().enumerate() {
        if relative.ends_with('/') {
            continue;
        }
        let time = UNIX_EPOCH + Duration::from_secs(start + index as u64);
        std::fs::File::options()
            .write(true)
            .open(dir.join(relative))
            .expect("open")
            .set_modified(time)
            .expect("dated");
    }
}

/// A tree as relative paths (`/` separators, a folder with a trailing `/`) to the bytes and the
/// modification time (to the second) of every file.
fn snapshot(dir: &Path) -> BTreeMap<String, (Vec<u8>, Option<u64>)> {
    fn walk(dir: &Path, prefix: &str, into: &mut BTreeMap<String, (Vec<u8>, Option<u64>)>) {
        for entry in std::fs::read_dir(dir).expect("read") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if path.is_dir() {
                into.insert(format!("{prefix}{name}/"), (Vec::new(), None));
                walk(&path, &format!("{prefix}{name}/"), into);
            } else {
                let bytes = std::fs::read(&path).expect("bytes");
                let modified = std::fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|elapsed| elapsed.as_secs());
                into.insert(format!("{prefix}{name}"), (bytes, modified));
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

fn backups_of(root: &Path) -> PathBuf {
    root.join("home").join("backups").join("wowf")
}

#[test]
fn a_backup_round_trips_the_wtf_tree_byte_for_byte_with_its_times() {
    let root = temp_game_root("roundtrip", true);
    let flavor = read(&root);
    let before = snapshot(&root.join("_forever_").join(WTF_DIR));
    assert_eq!(before.len(), 8 + 7, "8 files and 7 folders: {before:?}");

    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    assert_eq!(backup.files, 8);
    assert_eq!(backup.folders, 7);
    assert_eq!(
        backup.bytes,
        before
            .values()
            .map(|(bytes, _)| bytes.len() as u64)
            .sum::<u64>()
    );
    assert!(backup.warnings.is_empty(), "{:?}", backup.warnings);
    let name = backup.zip.file_name().expect("a name").to_string_lossy();
    assert!(
        name.starts_with(BACKUP_PREFIX) && name.ends_with(".zip"),
        "{name}"
    );
    assert_eq!(backup.zip.parent(), Some(backups.as_path()));
    assert!(!backups.join(format!("{name}.part")).exists());

    // Into a second flavor folder, which has no WTF yet: the restore creates it and moves nothing aside.
    let other = temp_game_root("roundtrip-target", false);
    let target = read(&other);
    let restored =
        backup::restore_wtf(&target, &backup.zip, &backups_of(&other)).expect("restored");
    assert_eq!(restored.files, 8);
    assert_eq!(restored.folders, 7);
    assert_eq!(restored.aside, None);
    let after = snapshot(&other.join("_forever_").join(WTF_DIR));
    assert_eq!(after, before, "byte for byte, with the times");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn a_restore_moves_the_current_wtf_aside_and_lists_it() {
    let root = temp_game_root("aside", true);
    let flavor = read(&root);
    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    std::fs::write(wtf.join("Config.wtf"), b"SET portal \"changed\"\r\n").expect("changed");
    std::fs::write(wtf.join("extra.txt"), b"not in the backup").expect("extra");

    let listing = backup::list_backups(&backups).expect("a listing");
    assert_eq!(listing.backups.len(), 1);
    assert_eq!(listing.backups[0].path, backup.zip);
    assert_eq!(listing.backups[0].files, Some(8));
    assert!(listing.backups[0].bytes > 0);
    assert!(
        listing.backups[0]
            .modified
            .as_deref()
            .is_some_and(|text| text.ends_with('Z')),
        "{:?}",
        listing.backups[0].modified
    );
    assert!(listing.aside.is_empty());

    let restored = backup::restore_wtf(&flavor, &backup.zip, &backups).expect("restored");
    let aside = restored.aside.expect("the previous WTF went aside");
    assert_eq!(aside.parent(), Some(backups.as_path()));
    assert!(
        aside
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .starts_with(ASIDE_PREFIX)
    );
    assert_eq!(
        std::fs::read(aside.join("extra.txt")).expect("the aside copy"),
        b"not in the backup"
    );
    assert_eq!(
        snapshot(&wtf),
        before,
        "the backup is back, the changes are aside"
    );

    let listing = backup::list_backups(&backups).expect("a listing");
    assert_eq!(listing.aside, vec![aside]);
    assert_eq!(listing.backups.len(), 1);

    let missing = backup::list_backups(&backups.join("no-such-folder")).expect("empty");
    assert!(missing.backups.is_empty() && missing.aside.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// A zip written by the test with the given entries (name, content); a name with a trailing `/` is a folder.
fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).expect("the zip");
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in entries {
        if name.ends_with('/') {
            writer
                .add_directory(*name, options)
                .expect("a folder entry");
        } else {
            writer.start_file(*name, options).expect("a file entry");
            writer.write_all(content).expect("content");
        }
    }
    writer.finish().expect("finished");
}

#[test]
fn an_entry_that_would_escape_wtf_refuses_the_whole_zip_before_anything_moves() {
    let root = temp_game_root("escape", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    std::fs::create_dir_all(&backups).expect("the backups folder");
    for (file, entries) in [
        (
            "dotdot.zip",
            vec![
                ("Config.wtf", &b"fine"[..]),
                ("../outside.txt", &b"escapes"[..]),
            ],
        ),
        (
            "absolute.zip",
            vec![
                ("Config.wtf", &b"fine"[..]),
                ("/etc/passwd", &b"escapes"[..]),
            ],
        ),
        ("drive.zip", vec![("C:\\Windows\\x.txt", &b"escapes"[..])]),
        ("device.zip", vec![("Account/NUL", &b"escapes"[..])]),
    ] {
        let zip = backups.join(file);
        write_zip(&zip, &entries);
        let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err(file);
        assert!(
            matches!(
                &error,
                Error::ZipEntryEscapes { role: ZipRole::Backup, path, .. } if path == &zip
            ),
            "{file}: {error}"
        );
        let text = error.to_string();
        assert!(
            text.starts_with("refused ") && text.ends_with("would land outside WTF"),
            "{text}"
        );
        assert_eq!(snapshot(&wtf), before, "{file}: nothing moved");
        assert!(!root.join("outside.txt").exists());
        assert!(!root.join("_forever_").join("outside.txt").exists());
    }
    let listing = backup::list_backups(&backups).expect("a listing");
    assert!(listing.aside.is_empty(), "nothing was moved aside");

    let not_a_zip = backups.join("not-a-zip.zip");
    std::fs::write(&not_a_zip, b"plain text").expect("written");
    let error = backup::restore_wtf(&flavor, &not_a_zip, &backups).expect_err("unreadable");
    assert!(
        matches!(
            &error,
            Error::ZipUnreadable {
                role: ZipRole::Backup,
                ..
            }
        ),
        "{error}"
    );
    assert!(
        error.to_string().starts_with("not a readable backup: "),
        "{error}"
    );

    // An entry the restore cannot unpack (encrypted) refuses the zip at the check, before WTF moves
    // aside (task 044: the restore used to find it half way through the unpack).
    let encrypted = backups.join("encrypted.zip");
    {
        use zip::unstable::write::FileOptionsExt;
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&encrypted).expect("the zip"));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer
            .start_file("Config.wtf", stored)
            .expect("a file entry");
        writer.write_all(b"fine").expect("content");
        let locked = stored
            .with_deprecated_encryption(b"password")
            .expect("ZipCrypto");
        writer
            .start_file("Account/secret.lua", locked)
            .expect("an encrypted entry");
        writer.write_all(b"-- encrypted").expect("content");
        writer.finish().expect("finished");
    }
    let error = backup::restore_wtf(&flavor, &encrypted, &backups).expect_err("encrypted");
    assert!(
        matches!(&error, Error::ZipUnreadable { role: ZipRole::Backup, path, .. } if path == &encrypted),
        "{error}"
    );
    let listing = backup::list_backups(&backups).expect("a listing");
    assert!(listing.aside.is_empty(), "nothing was moved aside");
    let missing = backups.join("no-such.zip");
    let error = backup::restore_wtf(&flavor, &missing, &backups).expect_err("missing");
    assert_eq!(
        error.to_string(),
        format!("backup not found: {}", missing.display())
    );
    assert_eq!(snapshot(&wtf), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_restore_that_fails_half_way_puts_the_previous_wtf_back() {
    let root = temp_game_root("revert", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    std::fs::create_dir_all(&backups).expect("the backups folder");
    // `Account` is a file first, then a folder is wanted under it: the second entry cannot be written.
    let zip = backups.join("clash.zip");
    write_zip(
        &zip,
        &[
            ("Account", &b"a file where a folder goes"[..]),
            ("Account/MRGLFIXTURE/AddOns.txt", &b"x"[..]),
        ],
    );
    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("fails half way");
    assert!(
        matches!(&error, Error::RestoreFailed { path, .. } if path == &zip),
        "{error}"
    );
    assert!(
        error.to_string().starts_with("restoring ")
            && error.to_string().contains(" failed and WTF is as it was: "),
        "{error}"
    );
    assert_eq!(snapshot(&wtf), before, "the previous WTF is back");
    let listing = backup::list_backups(&backups).expect("a listing");
    assert!(
        listing.aside.is_empty(),
        "the aside folder went back: {listing:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_flavor_without_wtf_has_nothing_to_back_up() {
    let root = temp_game_root("no-wtf", false);
    let flavor = read(&root);
    let error = backup::backup_wtf(&flavor, &backups_of(&root)).expect_err("nothing");
    assert!(matches!(&error, Error::NoWtf { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        format!(
            "nothing to back up: no WTF folder in {}",
            flavor.folder.display()
        )
    );
    assert!(!backups_of(&root).exists(), "no backups folder for nothing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_default_backups_folder_is_under_the_home() {
    assert_eq!(
        backup::backups_dir_at(Path::new("Q:\\home"), &FlavorId::Forever),
        PathBuf::from("Q:\\home\\backups\\wowf")
    );
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
fn a_restore_is_refused_while_the_client_runs_and_a_backup_is_not() {
    let root = temp_game_root("client", true);
    let flavor = read(&root);
    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);

    let Some(mut child) = spawn_stand_in(&flavor.folder) else {
        eprintln!("skipped: no ping.exe in the system folder to start the stand-in client");
        return;
    };
    let error = backup::restore_wtf(&flavor, &backup.zip, &backups).expect_err("refused");
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
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    // A backup only reads WTF, so it goes on while the client runs.
    let second = backup::backup_wtf(&flavor, &backups).expect("a backup while the client runs");
    assert_ne!(second.zip, backup.zip);

    child.stop();
    let restored = backup::restore_wtf(&flavor, &backup.zip, &backups).expect("restored");
    assert!(restored.aside.is_some());
    let _ = std::fs::remove_dir_all(&root);
}

/// The fixture is a few hand-written files in the shape of a `WTF` folder, never a real one: a
/// `Config.wtf`, an account with its caches and SavedVariables, a realm and a character with its
/// `AddOns.txt` and SavedVariables.
#[test]
fn the_fixture_tree_is_synthetic_and_small() {
    let snapshot = snapshot(&fixture_wtf());
    let bytes: usize = snapshot.values().map(|(bytes, _)| bytes.len()).sum();
    assert!(bytes < 4096, "the fixture WTF is {bytes} bytes");
    assert!(snapshot.contains_key("Config.wtf"));
    assert!(snapshot.contains_key("Account/MRGLFIXTURE/Fixture Realm/Mrgltest/AddOns.txt"));
    assert!(snapshot.contains_key("Account/MRGLFIXTURE/SavedVariables/MrglFixture.lua"));
}

// --- task 039: the bound on the unpacked size of a zip ---

use mrgl_core::zip_entry::Excess;

fn fixture_zip(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/zips")
        .join(name)
}

#[test]
fn a_backup_declaring_more_than_the_bound_is_refused_before_anything_moves() {
    let root = temp_game_root("declares-too-much", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    let zip = fixture_zip("wtf-declares-too-much.zip");
    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("refused");
    assert!(
        matches!(
            &error,
            Error::ZipTooLarge {
                role: ZipRole::Backup,
                path,
                excess: Excess::Declared { .. },
            } if path == &zip
        ),
        "{error}"
    );
    assert_eq!(snapshot(&wtf), before, "nothing moved");
    assert!(!backups.exists(), "nothing moved aside");

    // The bound lowered under the fixture tree's own backup refuses it the same way.
    let own = backup::backup_wtf(&flavor, &backups).expect("a backup");
    let error = backup::restore_wtf_bounded(&flavor, &own.zip, &backups, own.bytes - 1)
        .expect_err("one byte short");
    assert!(
        matches!(
            &error,
            Error::ZipTooLarge {
                excess: Excess::Declared { declared, .. },
                ..
            } if *declared == own.bytes
        ),
        "{error}"
    );
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    assert_eq!(snapshot(&wtf), before);

    // A higher bound for a backup the user trusts: the real bytes are a few.
    let restored = backup::restore_wtf_bounded(&flavor, &zip, &backups, 4 << 30)
        .expect("restored under 4 GiB");
    assert_eq!(restored.files, 1);
    assert_eq!(
        std::fs::read(wtf.join("Config.wtf")).expect("Config.wtf"),
        b"SET portal \"EU\"\r\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_backup_that_lies_about_its_size_fails_the_restore_and_wtf_comes_back() {
    let root = temp_game_root("lies-about-its-size", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    let zip = fixture_zip("wtf-lies-about-its-size.zip");
    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("refused");
    assert!(
        matches!(&error, Error::RestoreFailed { path, .. } if path == &zip),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "restoring {} failed and WTF is as it was: the entry `Config.wtf` unpacks to more than the 16 B it declares, so the bound of 2.0 GB cannot hold (0 B written, then removed)",
            zip.display()
        )
    );
    assert_eq!(snapshot(&wtf), before, "the previous WTF is back");
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}

// --- task 117: the arms the fixture round trip does not reach ---

/// The modification time of a file, to the second since 1970; `None` before 1970.
fn modified_seconds(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
}

#[cfg(windows)]
#[test]
fn a_junction_under_wtf_is_left_out_of_the_backup_with_a_warning() {
    let root = temp_game_root("junction", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("a folder");
    std::fs::write(elsewhere.join("linked.txt"), b"behind the junction").expect("a file");
    let link = wtf.join("Account").join("Linked");
    mrgl_core::link::create_junction(&link, &elsewhere).expect("a junction");

    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    assert_eq!(backup.files, 8, "nothing behind the junction");
    assert_eq!(backup.folders, 7);
    assert_eq!(
        backup.warnings,
        [format!(
            "left out of the backup: {} is a link, not a folder or a file",
            link.display()
        )]
    );
    let names: Vec<String> = {
        let file = std::fs::File::open(&backup.zip).expect("the zip");
        let mut archive = zip::ZipArchive::new(file).expect("a readable zip");
        (0..archive.len())
            .map(|index| {
                archive
                    .by_index_raw(index)
                    .expect("an entry")
                    .name()
                    .expect("a name")
                    .into_owned()
            })
            .collect()
    };
    assert!(
        names.iter().all(|name| !name.contains("Linked")),
        "{names:?}"
    );
    let _ = std::fs::remove_dir(&link);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_locked_file_or_folder_under_wtf_fails_the_backup_and_leaves_no_part_file() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let root = temp_game_root("locked-backup", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let backups = backups_of(&root);

    let config = wtf.join("Config.wtf");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&config)
        .expect("a handle with no sharing");
    let error = backup::backup_wtf(&flavor, &backups).expect_err("the file cannot be read");
    assert!(
        matches!(&error, Error::Io { context: "reading a WTF file", path, .. } if path == &config),
        "{error}"
    );
    drop(lock);

    let account = wtf.join("Account").join("MRGLFIXTURE");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(&account)
        .expect("a folder handle with no sharing");
    let error = backup::backup_wtf(&flavor, &backups).expect_err("the folder cannot be read");
    assert!(
        matches!(&error, Error::Io { context: "reading WTF", path, .. } if path == &account),
        "{error}"
    );
    drop(lock);

    let leftovers: Vec<String> = std::fs::read_dir(&backups)
        .expect("the backups folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(leftovers.is_empty(), "no part file: {leftovers:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_dated_before_1970_carries_no_time_in_the_backup_and_gets_the_restore_time() {
    let root = temp_game_root("before-1970", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    std::fs::File::options()
        .write(true)
        .open(wtf.join("Config.wtf"))
        .expect("open")
        .set_modified(UNIX_EPOCH - Duration::from_secs(86_400))
        .expect("dated 1969");
    assert_eq!(modified_seconds(&wtf.join("Config.wtf")), None);

    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    {
        let file = std::fs::File::open(&backup.zip).expect("the zip");
        let mut archive = zip::ZipArchive::new(file).expect("a readable zip");
        let entry = archive.by_name("Config.wtf").expect("the entry");
        assert_eq!(entry.extra_data_fields().count(), 0, "no timestamp field");
        assert_eq!(entry.last_modified(), Some(zip::DateTime::default()));
    }

    let other = temp_game_root("before-1970-target", false);
    let started = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("now")
        .as_secs();
    backup::restore_wtf(&read(&other), &backup.zip, &backups_of(&other)).expect("restored");
    let restored = modified_seconds(&other.join("_forever_").join(WTF_DIR).join("Config.wtf"))
        .expect("a time");
    assert!(restored >= started, "the file keeps the restore's time");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn the_zips_own_date_is_read_as_local_time_without_the_timestamp_field() {
    let root = temp_game_root("dos-times", false);
    let flavor = read(&root);
    let started = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("now")
        .as_secs();
    let restored = backup::restore_wtf(
        &flavor,
        &fixture_zip("wtf-dos-times.zip"),
        &backups_of(&root),
    )
    .expect("restored");
    assert_eq!((restored.files, restored.folders), (3, 0));
    let wtf = root.join("_forever_").join(WTF_DIR);
    let account = wtf.join("Account").join("MRGLFIXTURE");
    // No date at all, and the default date: the file keeps the restore's time.
    for file in [wtf.join("Config.wtf"), account.join("config-cache.wtf")] {
        let modified = modified_seconds(&file).expect("a time");
        assert!(modified >= started, "{}", file.display());
    }
    // The zip's own date fields, read as local time; the NTFS field next to them is passed over.
    let dated = backup::clock::from_local_civil(&backup::clock::Civil {
        year: 2024,
        month: 3,
        day: 1,
        hour: 12,
        minute: 30,
        second: 10,
    });
    assert_eq!(
        modified_seconds(&account.join("bindings-cache.wtf")),
        Some(
            dated
                .duration_since(UNIX_EPOCH)
                .expect("after 1970")
                .as_secs()
        )
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_listing_passes_over_other_entries_and_warns_about_an_unreadable_zip() {
    let root = temp_game_root("listing", false);
    let backups = backups_of(&root);
    std::fs::create_dir_all(backups.join("other-folder")).expect("a folder");
    std::fs::write(backups.join("notes.txt"), b"not a zip").expect("a file");
    let broken = backups.join("WTF-2026-01-01-000000.zip");
    write_zip(&broken, &[("Config.wtf", &b"fine"[..])]);
    // The central directory still reads; the first entry's local header does not.
    let mut bytes = std::fs::read(&broken).expect("the zip");
    bytes[..4].copy_from_slice(b"XXXX");
    std::fs::write(&broken, bytes).expect("broken");

    let listing = backup::list_backups(&backups).expect("a listing");
    assert_eq!(listing.backups.len(), 1);
    assert_eq!(listing.backups[0].path, broken);
    assert_eq!(listing.backups[0].files, None);
    assert!(listing.aside.is_empty(), "a folder of another name");
    assert_eq!(listing.warnings.len(), 1, "{:?}", listing.warnings);
    assert!(
        listing.warnings[0].starts_with(&format!("unreadable backup: {}: ", broken.display())),
        "{}",
        listing.warnings[0]
    );

    let error = backup::list_backups(&backups.join("notes.txt")).expect_err("a file");
    assert!(
        matches!(&error, Error::Io { context: "reading the backups folder", path, .. } if path == &backups.join("notes.txt")),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_restore_that_fails_into_a_flavor_without_wtf_leaves_none() {
    let root = temp_game_root("no-wtf-revert", false);
    let flavor = read(&root);
    let backups = backups_of(&root);
    std::fs::create_dir_all(&backups).expect("the backups folder");
    let zip = backups.join("clash.zip");
    write_zip(
        &zip,
        &[
            ("Account", &b"a file where a folder goes"[..]),
            ("Account/MRGLFIXTURE/AddOns.txt", &b"x"[..]),
        ],
    );
    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("fails half way");
    assert!(
        matches!(&error, Error::RestoreFailed { path, .. } if path == &zip),
        "{error}"
    );
    assert!(
        !root.join("_forever_").join(WTF_DIR).exists(),
        "the half-written WTF is removed"
    );
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_backups_path_that_is_a_file_or_a_zip_that_is_a_folder_refuses_the_restore() {
    let root = temp_game_root("restore-paths", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");

    let error = backup::restore_wtf(&flavor, &backups, &backups).expect_err("a folder");
    assert!(
        matches!(&error, Error::ZipUnreadable { role: ZipRole::Backup, path, .. } if path == &backups),
        "{error}"
    );

    let file = root.join("a-file");
    std::fs::write(&file, b"").expect("a file");
    let error = backup::restore_wtf(&flavor, &backup.zip, &file).expect_err("a file");
    assert!(
        matches!(&error, Error::Io { context: "creating the backups folder", path, .. } if path == &file),
        "{error}"
    );
    assert_eq!(snapshot(&wtf), before, "nothing moved");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_corrupt_entry_fails_the_restore_and_wtf_comes_back() {
    let root = temp_game_root("corrupt", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);
    let backups = backups_of(&root);
    std::fs::create_dir_all(&backups).expect("the backups folder");
    let zip = backups.join("corrupt.zip");
    write_zip(&zip, &[("Config.wtf", &b"SET portal \"EU\"\r\n"[..])]);
    // One byte of the stored data flipped: the entry opens, and its checksum fails as it is read.
    let mut bytes = std::fs::read(&zip).expect("the zip");
    let at = bytes
        .windows(6)
        .position(|window| window == b"portal")
        .expect("the data");
    bytes[at] = b'P';
    std::fs::write(&zip, bytes).expect("corrupt");

    let error = backup::restore_wtf(&flavor, &zip, &backups).expect_err("fails while writing");
    assert!(
        matches!(&error, Error::RestoreFailed { path, reason } if path == &zip && reason.starts_with("writing ")),
        "{error}"
    );
    assert_eq!(snapshot(&wtf), before, "the previous WTF is back");
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(windows)]
#[test]
fn a_locked_file_under_wtf_refuses_the_move_aside() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = temp_game_root("locked-restore", true);
    let flavor = read(&root);
    let wtf = root.join("_forever_").join(WTF_DIR);
    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    let before = snapshot(&wtf);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(wtf.join("Config.wtf"))
        .expect("a handle with no sharing");
    let error = backup::restore_wtf(&flavor, &backup.zip, &backups).expect_err("cannot move");
    assert!(
        matches!(&error, Error::Io { context: "moving WTF aside", path, .. } if path == &wtf),
        "{error}"
    );
    drop(lock);
    assert_eq!(snapshot(&wtf), before, "nothing moved");
    assert!(
        backup::list_backups(&backups)
            .expect("a listing")
            .aside
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}
