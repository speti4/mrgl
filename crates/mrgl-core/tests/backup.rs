//! The WTF backup and restore (task 013) over the synthetic fixture tree: a backup round-trips the
//! folder byte for byte with the file times kept, a restore refuses an entry that would land outside
//! `WTF` or that it cannot unpack before anything moves (the shared check of `zip_entry`), a failed restore puts the previous `WTF` back, the listing reads the
//! backups folder, and a restore with the stand-in client running (the pattern of the 023 tests: this test
//! binary copied into the flavor folder and started, never a game file) is refused with the documented
//! line.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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
fn a_restore_is_refused_while_the_client_runs_and_a_backup_is_not() {
    let root = temp_game_root("client", true);
    let flavor = read(&root);
    let backups = backups_of(&root);
    let backup = backup::backup_wtf(&flavor, &backups).expect("a backup");
    let wtf = root.join("_forever_").join(WTF_DIR);
    let before = snapshot(&wtf);

    let mut child = spawn_stand_in(&flavor.folder);
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

    child.kill().expect("killed");
    child.wait().expect("waited");
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
