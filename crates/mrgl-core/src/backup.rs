//! The toolbox (M3 Toolbox): WTF backup and restore (task 013); the addon sets (named lists of addons per
//! flavor, with enable and disable) are [`crate::sets`] (task 014). Nothing here touches
//! `Interface\AddOns` except through [`crate::install`].
//!
//! A backup is a plain zip a user can open by hand: every file of the flavor's `WTF` folder as an entry
//! with its path relative to `WTF` (`/` separators), deflated, its modification time in the zip's own
//! date fields (local time, as every zip tool writes them) and exactly, to the second, in the extended
//! timestamp field (`0x5455`); empty folders are directory entries. The zip is the manifest: no sidecar
//! entry, no index file. The default place is `backups\<flavor-id>\WTF-<date>.zip` under the mrgl home
//! (D-003), the date local, so the names sort by age; nothing here prunes that folder.
//!
//! A restore is the first of the three `WTF` writes of D-002 and D-003: it calls the guard of task 023
//! before its first write and refuses while the client runs; it checks every entry before anything moves
//! (an entry that would land outside `WTF` refuses the whole zip); it moves the current `WTF` aside into
//! the backups folder, unpacks, and on a failure puts the aside folder back. The aside folder stays
//! afterwards, so a restore the user regrets is itself reversible by hand.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use zip::write::FullFileOptions;
use zip::{CompressionMethod, DateTime, ExtraField, ZipArchive, ZipWriter};

use crate::client;
use crate::error::{Error, Result};
use crate::flavor::{Flavor, FlavorId};
use crate::home;
use crate::savedvars;
use crate::zip_entry::{self, CheckedEntry, ZipRole};

/// The settings folder of a flavor, under its folder.
pub const WTF_DIR: &str = "WTF";
/// The backups folder under the mrgl home: `backups\<flavor-id>\`.
pub const BACKUPS_DIR: &str = "backups";
/// A backup's name: `WTF-<date>.zip`.
pub const BACKUP_PREFIX: &str = "WTF-";
/// The name of a `WTF` folder a restore moved aside: `WTF-before-restore-<date>`.
pub const ASIDE_PREFIX: &str = "WTF-before-restore-";
/// The extended timestamp extra field (Info-ZIP, `0x5455`): the modification time as Unix seconds.
const EXTENDED_TIMESTAMP: u16 = 0x5455;

/// The backups folder of a flavor under a given home folder.
pub fn backups_dir_at(home_dir: &Path, flavor: &FlavorId) -> PathBuf {
    home_dir.join(BACKUPS_DIR).join(flavor.as_str())
}

/// The backups folder of a flavor under this process's home (`MRGL_HOME`, else `LOCALAPPDATA\mrgl`).
pub fn backups_dir(flavor: &FlavorId) -> Result<PathBuf> {
    Ok(backups_dir_at(&home::home_dir()?, flavor))
}

/// A flavor's `WTF` folder.
pub fn wtf_dir(flavor: &Flavor) -> PathBuf {
    flavor.folder.join(WTF_DIR)
}

/// A backup just written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Backup {
    /// The zip.
    pub zip: PathBuf,
    /// The files in it.
    pub files: usize,
    /// The folders in it, the empty ones included.
    pub folders: usize,
    /// The bytes of every file before compression.
    pub bytes: u64,
    /// Plain lines about what was left out: a symbolic link or a junction under `WTF`.
    pub warnings: Vec<String>,
}

/// Copies a flavor's `WTF` folder into a dated zip under `into` (the backups folder, created when
/// missing): `WTF-<date>.zip`, with a counter when a backup of the same second exists. The zip is written
/// under a `.part` name and renamed when complete, so a half-written backup never carries the name.
pub fn backup_wtf(flavor: &Flavor, into: &Path) -> Result<Backup> {
    let wtf = wtf_dir(flavor);
    if !wtf.is_dir() {
        return Err(Error::NoWtf {
            path: flavor.folder.clone(),
        });
    }
    let mut stats = Stats::default();
    let zip = dated_zip(into, BACKUP_PREFIX, |writer, part| {
        add_folder(writer, &wtf, "", part, &mut stats)
    })?;
    Ok(Backup {
        zip,
        files: stats.files,
        folders: stats.folders,
        bytes: stats.bytes,
        warnings: stats.warnings,
    })
}

/// Writes the given files into a dated zip under `into` (created when missing), `<prefix><date>.zip`, each
/// under its entry name (`/` separators), in the given order and as a backup writes them: deflated, with
/// its modification time. The zip and the bytes of every file before compression. The SavedVariables
/// cleanup of task 025 writes its zip here, so both zips of the backups folder come from one writer.
pub(crate) fn zip_files(
    into: &Path,
    prefix: &str,
    files: &[(String, PathBuf)],
) -> Result<(PathBuf, u64)> {
    let mut bytes = 0;
    let zip = dated_zip(into, prefix, |writer, part| {
        for (entry_name, path) in files {
            let metadata = fs::metadata(path).map_err(|source| Error::Io {
                context: "reading a WTF file",
                path: path.clone(),
                source,
            })?;
            bytes += add_file(writer, path, entry_name, &metadata, part)?;
        }
        Ok(())
    })?;
    Ok((zip, bytes))
}

/// A writer the zip's entries are added to.
type Writer = ZipWriter<BufWriter<File>>;

/// Writes a zip under a fresh dated name, `<prefix><date>.zip` under `into` (created when missing): the
/// entries `fill` adds, under a `.part` name, synced, then renamed, so a half-written zip never carries
/// the name; on a failure the `.part` file is removed.
fn dated_zip(
    into: &Path,
    prefix: &str,
    fill: impl FnOnce(&mut Writer, &Path) -> Result<()>,
) -> Result<PathBuf> {
    dated_zip_at(into, prefix, SystemTime::now(), fill)
}

/// [`dated_zip`] with the moment the name is dated by.
fn dated_zip_at(
    into: &Path,
    prefix: &str,
    now: SystemTime,
    fill: impl FnOnce(&mut Writer, &Path) -> Result<()>,
) -> Result<PathBuf> {
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    fs::create_dir_all(into).map_err(|source| io("creating the backups folder", into, source))?;
    let zip = fresh_path_at(into, prefix, ".zip", now)
        .map_err(|source| io("naming the backup", into, source))?;
    let part = zip.with_extension("zip.part");
    if let Err(error) = write_zip(&part, fill) {
        let _ = fs::remove_file(&part);
        return Err(error);
    }
    fs::rename(&part, &zip).map_err(|source| {
        let _ = fs::remove_file(&part);
        io("finishing the backup", &zip, source)
    })?;
    Ok(zip)
}

/// Writes a zip with the entries `fill` adds, finished and synced to the disk.
fn write_zip(zip: &Path, fill: impl FnOnce(&mut Writer, &Path) -> Result<()>) -> Result<()> {
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let file = File::create(zip).map_err(|source| io("writing the backup", zip, source))?;
    let mut writer = ZipWriter::new(BufWriter::new(file));
    fill(&mut writer, zip)?;
    writer
        .finish()
        // coverage: os-begin — a write to the zip the code just created
        .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?
        .into_inner()
        .map_err(|error| io("writing the backup", zip, error.into_error()))?
        .sync_all()
        .map_err(|source| io("writing the backup", zip, source))?;
    // coverage: os-end
    Ok(())
}

#[derive(Default)]
struct Stats {
    files: usize,
    folders: usize,
    bytes: u64,
    warnings: Vec<String>,
}

/// Adds a folder's entries, sorted by name, each under `prefix` (`Account/NAME/`): a subfolder is a
/// directory entry and then its own entries, a file is one deflated entry with its times, a symbolic
/// link or a junction is a warning and nothing else.
fn add_folder(
    writer: &mut Writer,
    dir: &Path,
    prefix: &str,
    zip: &Path,
    stats: &mut Stats,
) -> Result<()> {
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)
        .map_err(|source| io("reading WTF", dir, source))?
        .collect::<io::Result<_>>()
        .map_err(|source| io("reading WTF", dir, source))?; // coverage: os
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .map_err(|source| io("reading WTF", &path, source))?; // coverage: os
        if file_type.is_symlink() {
            stats.warnings.push(format!(
                "left out of the backup: {} is a link, not a folder or a file",
                path.display()
            ));
            continue;
        }
        let entry_name = format!("{prefix}{name}");
        if file_type.is_dir() {
            let options = FullFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .last_modified_time(dos_time(modified(&path)));
            writer
                .add_directory(format!("{entry_name}/"), options)
                // coverage: os-begin — a write to the zip the code just created
                .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
            // coverage: os-end
            stats.folders += 1;
            add_folder(writer, &path, &format!("{entry_name}/"), zip, stats)?;
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|source| io("reading WTF", &path, source))?; // coverage: os
        stats.bytes += add_file(writer, &path, &entry_name, &metadata, zip)?;
        stats.files += 1;
    }
    Ok(())
}

/// Adds one file as a deflated entry with its modification time (the zip's own date fields and the
/// extended timestamp field); the bytes copied.
fn add_file(
    writer: &mut Writer,
    path: &Path,
    entry_name: &str,
    metadata: &fs::Metadata,
    zip: &Path,
) -> Result<u64> {
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let modified = metadata.modified().ok();
    let mut options = FullFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(dos_time(modified))
        .large_file(metadata.len() >= u32::MAX as u64);
    if let Some(field) = extended_timestamp(modified) {
        options
            .add_extra_field(EXTENDED_TIMESTAMP, field, false)
            // coverage: os-begin — the writer refuses only an extra field past 64 KiB
            .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
        // coverage: os-end
    }
    writer
        .start_file(entry_name, options)
        .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
    let mut source = File::open(path).map_err(|source| io("reading a WTF file", path, source))?;
    io::copy(&mut source, writer).map_err(|source| io("writing the backup", zip, source))
}

/// A path's modification time, when the file system gives one.
fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// The zip's own date fields for a time: the local civil time, as every zip tool writes them; a missing
/// time or one outside the DOS range (1980–2107) is the zip's default.
fn dos_time(time: Option<SystemTime>) -> DateTime {
    time.map(clock::local_civil)
        .and_then(|civil| {
            DateTime::from_date_and_time(
                civil.year,
                civil.month,
                civil.day,
                civil.hour,
                civil.minute,
                civil.second,
            )
            .ok()
        })
        .unwrap_or_default()
}

/// The extended timestamp field: a flags byte (bit 0: the modification time is present) and the time as
/// Unix seconds, little-endian; `None` before 1970 or after 2106, where the field cannot hold it.
fn extended_timestamp(time: Option<SystemTime>) -> Option<[u8; 5]> {
    let seconds = time?.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let seconds = u32::try_from(seconds).ok()?;
    let [a, b, c, d] = seconds.to_le_bytes();
    Some([0b1, a, b, c, d])
}

/// A fresh name `<prefix><local date><suffix>` under a folder: the date as `YYYY-MM-DD-HHMMSS` of the
/// given moment, and a counter (`-2`, `-3`, …) when the name of that second is taken.
fn fresh_path_at(dir: &Path, prefix: &str, suffix: &str, now: SystemTime) -> io::Result<PathBuf> {
    let now = clock::local_civil(now);
    let stamp = format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        now.year, now.month, now.day, now.hour, now.minute, now.second
    );
    let first = dir.join(format!("{prefix}{stamp}{suffix}"));
    if !first.exists() {
        return Ok(first);
    }
    for counter in 2u32..1000 {
        let path = dir.join(format!("{prefix}{stamp}-{counter}{suffix}"));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "a thousand names of this second are taken under {}",
            dir.display()
        ),
    ))
}

/// One backup in a listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackupFile {
    /// The zip.
    pub path: PathBuf,
    /// Its file name, which `restore` takes as it is.
    pub name: String,
    /// The zip's size on disk.
    pub bytes: u64,
    /// The files in it; `None` when the zip cannot be read (the listing then carries a warning).
    pub files: Option<usize>,
    /// When the zip was last written, RFC 3339 in UTC (`2026-10-21T14:03:05Z`); `None` when unknown.
    pub modified: Option<String>,
}

/// The backups folder as listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    /// The folder.
    pub folder: PathBuf,
    /// Every `.zip` in it, by name (a dated name sorts by age).
    pub backups: Vec<BackupFile>,
    /// Every `WTF` folder a restore moved aside, by name.
    pub aside: Vec<PathBuf>,
    /// Plain lines about what could not be read.
    pub warnings: Vec<String>,
}

/// Lists a backups folder: a missing folder is an empty listing, never an error.
pub fn list_backups(folder: &Path) -> Result<Listing> {
    let mut listing = Listing {
        folder: folder.to_path_buf(),
        backups: Vec::new(),
        aside: Vec::new(),
        warnings: Vec::new(),
    };
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(listing),
        Err(source) => {
            return Err(Error::Io {
                context: "reading the backups folder",
                path: folder.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            // coverage: os-begin — an entry error mid-listing
            context: "reading the backups folder",
            path: folder.to_path_buf(),
            source,
        })?;
        // coverage: os-end
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name.starts_with(ASIDE_PREFIX) {
                listing.aside.push(path);
            }
            continue;
        }
        if !name.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        let metadata = entry.metadata().ok();
        let files = match count_files(&path) {
            Ok(files) => Some(files),
            Err(reason) => {
                listing
                    .warnings
                    .push(format!("unreadable backup: {}: {reason}", path.display()));
                None
            }
        };
        listing.backups.push(BackupFile {
            bytes: metadata.as_ref().map(fs::Metadata::len).unwrap_or_default(),
            modified: metadata
                .and_then(|metadata| metadata.modified().ok())
                .map(rfc3339_utc),
            path,
            name,
            files,
        });
    }
    listing.backups.sort_by(|a, b| a.name.cmp(&b.name));
    listing.aside.sort();
    Ok(listing)
}

/// The file entries of a zip.
fn count_files(zip: &Path) -> std::result::Result<usize, String> {
    let file = File::open(zip).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|error| error.to_string())?;
    let mut files = 0;
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| error.to_string())?;
        if !entry.is_dir() {
            files += 1;
        }
    }
    Ok(files)
}

/// A time as RFC 3339 in UTC, to the second.
pub fn rfc3339_utc(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_else(|before| -(before.duration().as_secs() as i64));
    let civil = clock::civil_from_unix(seconds);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        civil.year, civil.month, civil.day, civil.hour, civil.minute, civil.second
    )
}

/// A restore that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Restored {
    /// The zip.
    pub zip: PathBuf,
    /// The files written.
    pub files: usize,
    /// The folders created, the empty ones included.
    pub folders: usize,
    /// Where the previous `WTF` folder went; `None` when there was none.
    pub aside: Option<PathBuf>,
}

/// Puts a backup back as the flavor's `WTF` folder. In this order: the guard of task 023 (a running
/// client refuses the restore before anything is touched), every entry checked (one that would land
/// outside `WTF` refuses the whole zip), the current `WTF` moved aside into the backups folder as
/// `WTF-before-restore-<date>`, the zip unpacked into a new `WTF`. A failure while unpacking removes the
/// half-written folder and puts the aside one back ([`Error::RestoreFailed`]); when even that fails, the
/// aside folder stays where it is and the error names it ([`Error::RestoreRevertFailed`]). A cleanup zip of
/// the SavedVariables (`SavedVariables-<date>.zip`, task 025) is refused before anything else: it holds a
/// few files, not a `WTF` folder, and is unzipped into `WTF` by hand ([`Error::NotAWtfBackup`]).
/// The zip is bounded by [`zip_entry::MAX_UNPACKED`]; [`restore_wtf_bounded`] takes another bound.
pub fn restore_wtf(flavor: &Flavor, zip: &Path, backups: &Path) -> Result<Restored> {
    restore_wtf_bounded(flavor, zip, backups, zip_entry::MAX_UNPACKED)
}

/// [`restore_wtf`] with a bound on what the zip unpacks to (task 039; `backup --restore
/// --max-unpacked`): a zip whose entries declare more is refused with [`Error::ZipTooLarge`] before
/// anything moves; one that unpacks past the bound or past an entry's declared size while it is written
/// fails the restore like any other write ([`Error::RestoreFailed`], the bound's clause as the reason).
pub fn restore_wtf_bounded(
    flavor: &Flavor,
    zip: &Path,
    backups: &Path,
    max_unpacked: u64,
) -> Result<Restored> {
    restore_wtf_with(
        flavor,
        zip,
        backups,
        max_unpacked,
        SystemTime::now(),
        rename,
    )
}

/// `fs::rename` as the restore takes it.
fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

/// [`restore_wtf_bounded`] with the moment the aside folder is dated by and the rename that moves a
/// folder (`fs::rename` outside the tests, which answer it as another volume or a failure would).
fn restore_wtf_with(
    flavor: &Flavor,
    zip: &Path,
    backups: &Path,
    max_unpacked: u64,
    now: SystemTime,
    rename: impl Fn(&Path, &Path) -> io::Result<()>,
) -> Result<Restored> {
    if zip.file_name().is_some_and(|name| {
        name.to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(&savedvars::CLEANUP_PREFIX.to_ascii_lowercase())
    }) {
        return Err(Error::NotAWtfBackup {
            path: zip.to_path_buf(),
        });
    }
    client::ensure_client_not_running(flavor)?;
    let (mut archive, entries) = checked_entries(zip, max_unpacked)?;
    let wtf = wtf_dir(flavor);
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let aside = if wtf.exists() {
        fs::create_dir_all(backups)
            .map_err(|source| io("creating the backups folder", backups, source))?;
        let aside = fresh_path_at(backups, ASIDE_PREFIX, "", now)
            .map_err(|source| io("naming the aside folder", backups, source))?;
        move_tree_with(&wtf, &aside, &rename)
            .map_err(|source| io("moving WTF aside", &wtf, source))?;
        Some(aside)
    } else {
        None
    };
    let mut unpacked = zip_entry::Unpacked::new(zip, ZipRole::Backup, max_unpacked);
    match unpack(&mut archive, &entries, &wtf, &mut unpacked) {
        Ok((files, folders)) => Ok(Restored {
            zip: zip.to_path_buf(),
            files,
            folders,
            aside,
        }),
        Err(reason) => {
            // The bound's refusal (task 039) gives its own clause; anything else is the I/O error.
            let reason = match zip_entry::refusal(reason) {
                Ok(refusal) => refusal.excess.to_string(),
                Err(reason) => reason.to_string(),
            };
            let _ = fs::remove_dir_all(&wtf);
            let reverted = match &aside {
                Some(aside) => !wtf.exists() && move_tree_with(aside, &wtf, &rename).is_ok(),
                None => true,
            };
            match aside {
                Some(aside) if !reverted => Err(Error::RestoreRevertFailed {
                    path: zip.to_path_buf(),
                    reason: reason.to_string(),
                    aside,
                }),
                _ => Err(Error::RestoreFailed {
                    path: zip.to_path_buf(),
                    reason: reason.to_string(),
                }),
            }
        }
    }
}

/// Opens a backup and checks every entry before anything is written ([`zip_entry::checked_entries`]):
/// every name a relative path under `WTF`, no symbolic link, every entry readable.
fn checked_entries(
    zip: &Path,
    max_unpacked: u64,
) -> Result<(ZipArchive<BufReader<File>>, Vec<CheckedEntry>)> {
    let unreadable = |reason: String| Error::ZipUnreadable {
        role: ZipRole::Backup,
        path: zip.to_path_buf(),
        reason,
    };
    let file = match File::open(zip) {
        Ok(file) => file,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(Error::BackupNotFound {
                path: zip.to_path_buf(),
            });
        }
        Err(source) => return Err(unreadable(source.to_string())),
    };
    let mut archive =
        ZipArchive::new(BufReader::new(file)).map_err(|reason| unreadable(reason.to_string()))?;
    let entries = zip_entry::checked_entries(&mut archive, zip, ZipRole::Backup, max_unpacked)?;
    Ok((archive, entries))
}

/// Unpacks the checked entries under a fresh `WTF`: the folders, then every file with its modification
/// time from the extended timestamp field, else from the zip's own date fields read as local time. Every
/// file is read through the bound's counter (task 039).
fn unpack(
    archive: &mut ZipArchive<BufReader<File>>,
    entries: &[CheckedEntry],
    wtf: &Path,
    unpacked: &mut zip_entry::Unpacked,
) -> io::Result<(usize, usize)> {
    let context = |what: &str, path: &Path, error: io::Error| {
        io::Error::new(error.kind(), format!("{what} {}: {error}", path.display()))
    };
    fs::create_dir_all(wtf).map_err(|error| context("creating", wtf, error))?;
    let mut files = 0;
    let mut folders = 0;
    for checked in entries {
        let mut path = wtf.to_path_buf();
        path.extend(&checked.parts);
        if checked.is_dir {
            fs::create_dir_all(&path).map_err(|error| context("creating", &path, error))?;
            folders += 1;
            continue;
        }
        // A file entry has at least one component, so its path has a parent under `WTF`.
        let parent = path.parent().unwrap_or(wtf);
        fs::create_dir_all(parent).map_err(|error| context("creating", parent, error))?;
        let entry = archive.by_index(checked.index).map_err(io::Error::other)?;
        let modified = entry_modified(&entry);
        let file = File::create(&path).map_err(|error| context("writing", &path, error))?;
        let mut output = BufWriter::new(file);
        io::copy(&mut unpacked.entry(checked, entry), &mut output).map_err(|error| {
            match zip_entry::refusal(error) {
                Ok(refusal) => io::Error::other(refusal),
                Err(error) => context("writing", &path, error),
            }
        })?;
        let file = output
            .into_inner()
            .map_err(|error| context("writing", &path, error.into_error()))?; // coverage: os
        if let Some(modified) = modified {
            file.set_modified(modified)
                .map_err(|error| context("dating", &path, error))?; // coverage: os
        }
        files += 1;
    }
    Ok((files, folders))
}

/// An entry's modification time: the extended timestamp field when the zip carries one (exact, UTC),
/// else the zip's own date fields read as local time; `None` when neither says.
fn entry_modified<R: io::Read>(entry: &zip::read::ZipFile<'_, R>) -> Option<SystemTime> {
    let exact = entry.extra_data_fields().find_map(|field| match field {
        ExtraField::ExtendedTimestamp(stamp) => stamp.mod_time(),
        _ => None,
    });
    if let Some(seconds) = exact {
        return Some(UNIX_EPOCH + Duration::from_secs(u64::from(seconds)));
    }
    let dos = entry.last_modified()?;
    if dos == DateTime::default() {
        return None;
    }
    Some(clock::from_local_civil(&clock::Civil {
        year: dos.year(),
        month: dos.month(),
        day: dos.day(),
        hour: dos.hour(),
        minute: dos.minute(),
        second: dos.second(),
    }))
}

/// Moves a folder: one rename on the same volume (`rename`, `fs::rename` outside the tests); across
/// volumes (the game on one drive, the mrgl home on another) a copy of the tree and then the removal of
/// the original.
fn move_tree_with(
    from: &Path,
    to: &Path,
    rename: impl Fn(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    match rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_tree(from, to)?;
            fs::remove_dir_all(from)
        }
        Err(error) => Err(error),
    }
}

/// Copies a folder tree, the file times included.
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Civil time, the calendar fields of a moment in UTC or in local time: the arithmetic behind the dated
/// names and the zip timestamps, without a date crate. The proleptic Gregorian conversions are Howard
/// Hinnant's `days_from_civil` / `civil_from_days`; on Windows the time-zone conversion is the Windows
/// SDK's (`SystemTimeToTzSpecificLocalTime`, the daylight rule of the moment itself), and the other
/// targets use UTC as local time until the macOS preview (M6).
pub mod clock {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// A calendar moment, to the second.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Civil {
        pub year: u16,
        pub month: u8,
        pub day: u8,
        pub hour: u8,
        pub minute: u8,
        pub second: u8,
    }

    /// The civil fields of Unix seconds, in UTC.
    pub fn civil_from_unix(seconds: i64) -> Civil {
        let days = seconds.div_euclid(86_400);
        let rest = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Civil {
            year: year.clamp(0, i64::from(u16::MAX)) as u16,
            month,
            day,
            hour: (rest / 3600) as u8,
            minute: (rest % 3600 / 60) as u8,
            second: (rest % 60) as u8,
        }
    }

    /// The Unix seconds of civil fields taken as UTC.
    pub fn unix_from_civil(civil: &Civil) -> i64 {
        days_from_civil(i64::from(civil.year), civil.month, civil.day) * 86_400
            + i64::from(civil.hour) * 3600
            + i64::from(civil.minute) * 60
            + i64::from(civil.second)
    }

    /// Days since 1970-01-01 of a proleptic Gregorian date.
    fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
        let month = i64::from(month);
        let year = if month <= 2 { year - 1 } else { year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month_index = if month > 2 { month - 3 } else { month + 9 };
        let day_of_year = (153 * month_index + 2) / 5 + i64::from(day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The proleptic Gregorian date of days since 1970-01-01.
    fn civil_from_days(days: i64) -> (i64, u8, u8) {
        let days = days + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u8;
        let month = if month_index < 10 {
            month_index + 3
        } else {
            month_index - 9
        } as u8;
        (if month <= 2 { year + 1 } else { year }, month, day)
    }

    /// A moment as the local civil time of this machine.
    pub fn local_civil(time: SystemTime) -> Civil {
        let seconds = unix_seconds(time);
        let utc = civil_from_unix(seconds);
        system::to_local(&utc).unwrap_or(utc)
    }

    /// A moment from the local civil time of this machine (a zip's own date fields).
    pub fn from_local_civil(local: &Civil) -> SystemTime {
        let utc = system::to_utc(local).unwrap_or(*local);
        let seconds = unix_from_civil(&utc);
        if seconds >= 0 {
            UNIX_EPOCH + Duration::from_secs(seconds as u64)
        } else {
            UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
        }
    }

    fn unix_seconds(time: SystemTime) -> i64 {
        match time.duration_since(UNIX_EPOCH) {
            Ok(elapsed) => elapsed.as_secs() as i64,
            Err(before) => -(before.duration().as_secs() as i64),
        }
    }

    #[cfg(windows)]
    mod system {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::Time::{
            SystemTimeToTzSpecificLocalTime, TzSpecificLocalTimeToSystemTime,
        };

        use super::Civil;

        fn system_time(civil: &Civil) -> SYSTEMTIME {
            SYSTEMTIME {
                wYear: civil.year,
                wMonth: u16::from(civil.month),
                wDayOfWeek: 0,
                wDay: u16::from(civil.day),
                wHour: u16::from(civil.hour),
                wMinute: u16::from(civil.minute),
                wSecond: u16::from(civil.second),
                wMilliseconds: 0,
            }
        }

        fn civil(time: &SYSTEMTIME) -> Civil {
            Civil {
                year: time.wYear,
                month: time.wMonth as u8,
                day: time.wDay as u8,
                hour: time.wHour as u8,
                minute: time.wMinute as u8,
                second: time.wSecond as u8,
            }
        }

        /// UTC to the local time of the current time zone; `None` when Windows cannot convert it (a year
        /// outside 1601–30827).
        pub(super) fn to_local(utc: &Civil) -> Option<Civil> {
            let input = system_time(utc);
            // SAFETY: a null time-zone pointer means the current zone; both structs are plain C data
            // that live for the call.
            let mut output: SYSTEMTIME = unsafe { std::mem::zeroed() };
            let ok =
                unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &input, &mut output) };
            (ok != 0).then(|| civil(&output))
        }

        /// The local time of the current time zone to UTC; `None` when Windows cannot convert it.
        pub(super) fn to_utc(local: &Civil) -> Option<Civil> {
            let input = system_time(local);
            // SAFETY: as for `to_local`.
            let mut output: SYSTEMTIME = unsafe { std::mem::zeroed() };
            let ok =
                unsafe { TzSpecificLocalTimeToSystemTime(std::ptr::null(), &input, &mut output) };
            (ok != 0).then(|| civil(&output))
        }
    }

    #[cfg(not(windows))]
    mod system {
        //! No time zone outside Windows yet: local time is UTC.

        use super::Civil;

        pub(super) fn to_local(_utc: &Civil) -> Option<Civil> {
            None
        }

        pub(super) fn to_utc(_local: &Civil) -> Option<Civil> {
            None
        }
    }

    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    mod tests {
        use super::*;

        #[test]
        fn civil_and_unix_round_trip_across_leap_years_and_centuries() {
            for seconds in [
                0,
                951_782_400,   // 2000-02-29 00:00:00
                1_709_164_799, // 2024-02-28 23:59:59
                1_709_251_200, // 2024-03-01 00:00:00
                1_761_068_165, // 2025-10-21 17:36:05
                4_102_444_800, // 2100-01-01 00:00:00
                -86_400,       // 1969-12-31
            ] {
                let civil = civil_from_unix(seconds);
                assert_eq!(unix_from_civil(&civil), seconds, "{civil:?}");
            }
            assert_eq!(
                civil_from_unix(951_782_400),
                Civil {
                    year: 2000,
                    month: 2,
                    day: 29,
                    hour: 0,
                    minute: 0,
                    second: 0
                }
            );
            assert_eq!(
                civil_from_unix(1_709_164_799),
                Civil {
                    year: 2024,
                    month: 2,
                    day: 28,
                    hour: 23,
                    minute: 59,
                    second: 59
                }
            );
        }

        #[test]
        fn local_time_round_trips_through_the_zone() {
            let now = SystemTime::now();
            let seconds = unix_seconds(now);
            let local = local_civil(now);
            let back = from_local_civil(&local);
            assert_eq!(unix_seconds(back), seconds, "{local:?}");
        }

        #[test]
        fn moments_before_1970_and_past_the_zone_range_keep_their_civil_fields() {
            let before = UNIX_EPOCH - Duration::from_secs(86_401);
            assert_eq!(unix_seconds(before), -86_401);
            let local = local_civil(before);
            assert_eq!(local.year, 1969, "{local:?}");
            assert_eq!(unix_seconds(from_local_civil(&local)), -86_401, "{local:?}");
            // A year the zone conversion refuses (Windows converts 1601 to 30827): UTC is the local time.
            let far = Civil {
                year: 30828,
                month: 1,
                day: 1,
                hour: 12,
                minute: 0,
                second: 0,
            };
            let seconds = unix_from_civil(&far);
            let moment = UNIX_EPOCH + Duration::from_secs(seconds as u64);
            assert_eq!(local_civil(moment), far);
            assert_eq!(unix_seconds(from_local_civil(&far)), seconds);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::cell::Cell;
    use std::io::Write;

    use super::*;

    /// A temporary folder under the system temp dir, removed before use and when dropped.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Temp {
            let dir =
                std::env::temp_dir().join(format!("mrgl-backup-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("the folder");
            Temp(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A fixed moment and its stamp in a dated name.
    fn moment() -> (SystemTime, String) {
        let now = UNIX_EPOCH + Duration::from_secs(1_761_068_165);
        let civil = clock::local_civil(now);
        let stamp = format!(
            "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
            civil.year, civil.month, civil.day, civil.hour, civil.minute, civil.second
        );
        (now, stamp)
    }

    /// The names of a folder's entries, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
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

    /// A zip written by the test with the given entries (name, content).
    fn write_test_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = ZipWriter::new(File::create(path).expect("the zip"));
        let options =
            zip::write::SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, content) in entries {
            writer.start_file(*name, options).expect("a file");
            writer.write_all(content).expect("content");
        }
        writer.finish().expect("finished");
    }

    #[test]
    fn the_extended_timestamp_is_flags_and_little_endian_seconds() {
        let time = UNIX_EPOCH + Duration::from_secs(0x0102_0304);
        assert_eq!(extended_timestamp(Some(time)), Some([1, 4, 3, 2, 1]));
        assert_eq!(extended_timestamp(None), None);
        assert_eq!(
            extended_timestamp(Some(UNIX_EPOCH - Duration::from_secs(1))),
            None
        );
    }

    #[test]
    fn times_render_as_rfc3339_utc() {
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + Duration::from_secs(1_761_068_165)),
            "2025-10-21T17:36:05Z"
        );
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH - Duration::from_secs(1)),
            "1969-12-31T23:59:59Z"
        );
    }

    #[test]
    fn a_dos_time_is_the_local_civil_time_to_two_seconds() {
        let time = UNIX_EPOCH + Duration::from_secs(1_761_068_165);
        let local = clock::local_civil(time);
        let dos = dos_time(Some(time));
        assert_eq!(dos.year(), local.year);
        assert_eq!(dos.month(), local.month);
        assert_eq!(dos.day(), local.day);
        assert_eq!(dos.hour(), local.hour);
        assert_eq!(dos.minute(), local.minute);
        assert_eq!(dos.second(), local.second & !1);
        assert_eq!(dos_time(None), DateTime::default());
        // Before 1980 and after 2107 the zip's own fields cannot hold the time.
        assert_eq!(dos_time(Some(UNIX_EPOCH)), DateTime::default());
        let past_2107 = clock::unix_from_civil(&clock::Civil {
            year: 2108,
            month: 7,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
        });
        assert_eq!(
            dos_time(Some(UNIX_EPOCH + Duration::from_secs(past_2107 as u64))),
            DateTime::default()
        );
    }

    #[test]
    fn a_fresh_name_counts_up_within_a_second_and_a_thousand_names_are_too_many() {
        let temp = Temp::new("names");
        let dir = temp.path();
        let (now, stamp) = moment();
        let first = fresh_path_at(dir, BACKUP_PREFIX, ".zip", now).expect("a name");
        assert_eq!(first, dir.join(format!("WTF-{stamp}.zip")));
        fs::write(&first, b"").expect("taken");
        let second = fresh_path_at(dir, BACKUP_PREFIX, ".zip", now).expect("a name");
        assert_eq!(second, dir.join(format!("WTF-{stamp}-2.zip")));
        fs::write(&second, b"").expect("taken");
        let third = fresh_path_at(dir, BACKUP_PREFIX, ".zip", now).expect("a name");
        assert_eq!(third, dir.join(format!("WTF-{stamp}-3.zip")));
        for counter in 3..1000 {
            fs::write(dir.join(format!("WTF-{stamp}-{counter}.zip")), b"").expect("taken");
        }
        let error = fresh_path_at(dir, BACKUP_PREFIX, ".zip", now).expect_err("every name taken");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            error.to_string(),
            format!(
                "a thousand names of this second are taken under {}",
                dir.display()
            )
        );
    }

    #[test]
    fn a_backups_path_that_is_a_file_or_has_every_name_taken_refuses_the_zip() {
        let temp = Temp::new("dated-zip-names");
        let (now, stamp) = moment();
        let fill = |_: &mut Writer, _: &Path| Ok(());

        let file = temp.path().join("a-file");
        fs::write(&file, b"").expect("a file");
        let error = dated_zip_at(&file, BACKUP_PREFIX, now, fill).expect_err("a file");
        assert!(
            matches!(&error, Error::Io { context: "creating the backups folder", path, .. } if path == &file),
            "{error}"
        );

        let full = temp.path().join("full");
        fs::create_dir_all(&full).expect("the folder");
        fs::write(full.join(format!("WTF-{stamp}.zip")), b"").expect("taken");
        for counter in 2..1000 {
            fs::write(full.join(format!("WTF-{stamp}-{counter}.zip")), b"").expect("taken");
        }
        let error = dated_zip_at(&full, BACKUP_PREFIX, now, fill).expect_err("every name taken");
        assert!(
            matches!(&error, Error::Io { context: "naming the backup", path, .. } if path == &full),
            "{error}"
        );
        assert_eq!(names(&full).len(), 999, "nothing was written");
    }

    #[test]
    fn a_part_name_that_cannot_be_written_or_is_gone_before_the_rename_fails_the_zip() {
        let temp = Temp::new("dated-zip-part");
        let into = temp.path().join("backups");
        let (now, stamp) = moment();
        let zip = into.join(format!("WTF-{stamp}.zip"));
        let part = into.join(format!("WTF-{stamp}.zip.part"));

        // A folder where the `.part` file goes: the zip cannot be created.
        fs::create_dir_all(&part).expect("a folder in the way");
        let error = dated_zip_at(&into, BACKUP_PREFIX, now, |_, _| Ok(())).expect_err("in the way");
        assert!(
            matches!(&error, Error::Io { context: "writing the backup", path, .. } if path == &part),
            "{error}"
        );
        assert!(!zip.exists());
        fs::remove_dir(&part).expect("out of the way");

        // The `.part` file gone while it is filled: the rename to the zip's name fails, no zip is left.
        let error = dated_zip_at(&into, BACKUP_PREFIX, now, |_, part| {
            fs::remove_file(part).expect("removed while open");
            Ok(())
        })
        .expect_err("gone");
        assert!(
            matches!(&error, Error::Io { context: "finishing the backup", path, .. } if path == &zip),
            "{error}"
        );
        assert_eq!(names(&into), Vec::<String>::new());
    }

    #[test]
    fn a_file_that_cannot_be_read_or_a_repeated_entry_name_leaves_no_part_file() {
        let temp = Temp::new("zip-files");
        let into = temp.path().join("backups");
        let missing = temp.path().join("missing.lua");
        let error = zip_files(
            &into,
            savedvars::CLEANUP_PREFIX,
            &[("A.lua".into(), missing.clone())],
        )
        .expect_err("missing");
        assert!(
            matches!(&error, Error::Io { context: "reading a WTF file", path, .. } if path == &missing),
            "{error}"
        );
        assert_eq!(
            names(&into),
            Vec::<String>::new(),
            "the part file is removed"
        );

        let file = temp.path().join("Twice.lua");
        fs::write(&file, b"x").expect("a file");
        let twice = [
            ("Twice.lua".to_string(), file.clone()),
            ("Twice.lua".into(), file),
        ];
        let error = zip_files(&into, savedvars::CLEANUP_PREFIX, &twice).expect_err("twice");
        assert!(
            matches!(
                &error,
                Error::Io {
                    context: "writing the backup",
                    ..
                }
            ),
            "{error}"
        );
        assert!(error.to_string().contains("Duplicate filename"), "{error}");
        assert_eq!(
            names(&into),
            Vec::<String>::new(),
            "the part file is removed"
        );
    }

    /// A flavor folder with a small `WTF` tree (a root file and a file under an account folder) and a
    /// backup of it under `backups`; the flavor as `read_flavor` would give it.
    fn flavor_with_backup(temp: &Temp) -> (Flavor, PathBuf, PathBuf) {
        let folder = temp.path().join("_forever_");
        let wtf = folder.join(WTF_DIR);
        fs::create_dir_all(wtf.join("Account").join("ACC")).expect("the tree");
        fs::write(wtf.join("Config.wtf"), b"SET portal \"EU\"\r\n").expect("a file");
        fs::write(wtf.join("Account").join("ACC").join("x.txt"), b"x").expect("a file");
        let flavor = Flavor {
            id: FlavorId::Forever,
            folder,
            version: None,
            version_source: None,
            curse_client: false,
            warnings: Vec::new(),
        };
        let backups = temp.path().join("backups");
        let backup = backup_wtf(&flavor, &backups).expect("a backup");
        (flavor, backups, backup.zip)
    }

    /// The files under a folder, relative, `/`-separated, sorted.
    fn tree(dir: &Path) -> Vec<String> {
        fn walk(dir: &Path, prefix: &str, into: &mut Vec<String>) {
            for name in names(dir) {
                let path = dir.join(&name);
                if path.is_dir() {
                    walk(&path, &format!("{prefix}{name}/"), into);
                } else {
                    into.push(format!("{prefix}{name}"));
                }
            }
        }
        let mut into = Vec::new();
        walk(dir, "", &mut into);
        into
    }

    #[test]
    fn a_wtf_on_another_volume_is_copied_aside_and_removed() {
        let temp = Temp::new("restore-volumes");
        let (flavor, backups, zip) = flavor_with_backup(&temp);
        let wtf = wtf_dir(&flavor);
        fs::write(wtf.join("extra.txt"), b"not in the backup").expect("a change");
        let other_volume = |_: &Path, _: &Path| Err(io::Error::from(io::ErrorKind::CrossesDevices));
        let (now, _) = moment();
        let restored = restore_wtf_with(&flavor, &zip, &backups, u64::MAX, now, other_volume)
            .expect("restored");
        let aside = restored.aside.expect("the previous WTF went aside");
        assert_eq!(
            tree(&aside),
            ["Account/ACC/x.txt", "Config.wtf", "extra.txt"],
            "the copy"
        );
        assert_eq!(tree(&wtf), ["Account/ACC/x.txt", "Config.wtf"]);
        assert_eq!(restored.files, 2);
        assert_eq!(restored.folders, 2);
    }

    #[test]
    fn a_rename_that_fails_refuses_the_move_aside_before_anything_is_written() {
        let temp = Temp::new("restore-rename");
        let (flavor, backups, zip) = flavor_with_backup(&temp);
        let wtf = wtf_dir(&flavor);
        let before = tree(&wtf);
        let refused = |_: &Path, _: &Path| Err(io::Error::from(io::ErrorKind::PermissionDenied));
        let (now, _) = moment();
        let error =
            restore_wtf_with(&flavor, &zip, &backups, u64::MAX, now, refused).expect_err("refused");
        assert!(
            matches!(&error, Error::Io { context: "moving WTF aside", path, .. } if path == &wtf),
            "{error}"
        );
        assert_eq!(tree(&wtf), before);
        assert_eq!(names(&backups).len(), 1, "the zip alone: no aside folder");
    }

    #[test]
    fn a_thousand_aside_names_of_one_second_refuse_the_restore() {
        let temp = Temp::new("restore-names");
        let (flavor, backups, zip) = flavor_with_backup(&temp);
        let (now, stamp) = moment();
        fs::create_dir_all(backups.join(format!("{ASIDE_PREFIX}{stamp}"))).expect("taken");
        for counter in 2..1000 {
            fs::create_dir_all(backups.join(format!("{ASIDE_PREFIX}{stamp}-{counter}")))
                .expect("taken");
        }
        let before = tree(&wtf_dir(&flavor));
        let error = restore_wtf_with(&flavor, &zip, &backups, u64::MAX, now, rename)
            .expect_err("every name taken");
        assert!(
            matches!(&error, Error::Io { context: "naming the aside folder", path, .. } if path == &backups),
            "{error}"
        );
        assert_eq!(tree(&wtf_dir(&flavor)), before);
    }

    /// A zip whose second entry cannot be written: `Account` is a file first, then a folder is wanted
    /// under it.
    fn clash_zip(dir: &Path) -> PathBuf {
        let zip = dir.join("clash.zip");
        write_test_zip(
            &zip,
            &[
                ("Account", &b"a file where a folder goes"[..]),
                ("Account/ACC/x.txt", &b"x"[..]),
            ],
        );
        zip
    }

    #[test]
    fn a_revert_that_fails_names_the_aside_folder() {
        let temp = Temp::new("restore-revert");
        let (flavor, backups, _) = flavor_with_backup(&temp);
        let wtf = wtf_dir(&flavor);
        let before = tree(&wtf);
        let zip = clash_zip(&backups);
        // The move aside goes through; the move back is refused.
        let renames = Cell::new(0);
        let once = |from: &Path, to: &Path| {
            renames.set(renames.get() + 1);
            if renames.get() == 1 {
                fs::rename(from, to)
            } else {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
        };
        let (now, _) = moment();
        let error = restore_wtf_with(&flavor, &zip, &backups, u64::MAX, now, once)
            .expect_err("the revert fails");
        let Error::RestoreRevertFailed {
            path,
            reason,
            aside,
        } = &error
        else {
            panic!("{error}");
        };
        assert_eq!(path, &zip);
        assert!(reason.starts_with("creating "), "{reason}");
        assert_eq!(aside.parent(), Some(backups.as_path()));
        assert_eq!(
            error.to_string(),
            format!(
                "restoring {} failed and the previous WTF could not be put back: {reason}; it is at {}",
                zip.display(),
                aside.display()
            )
        );
        assert_eq!(renames.get(), 2);
        assert!(!wtf.exists(), "the half-written WTF is removed");
        assert_eq!(tree(aside), before, "the previous WTF waits aside");
    }

    #[cfg(windows)]
    #[test]
    fn a_half_written_wtf_that_cannot_be_removed_names_the_aside_folder() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = Temp::new("restore-locked");
        let (flavor, backups, _) = flavor_with_backup(&temp);
        let wtf = wtf_dir(&flavor);
        let before = tree(&wtf);
        let zip = clash_zip(&backups);
        // Once the previous WTF is aside, a file held open under the new one keeps it from being removed.
        let lock = std::cell::RefCell::new(None);
        let then_lock = |from: &Path, to: &Path| {
            fs::rename(from, to)?;
            fs::create_dir_all(&wtf)?;
            let held = wtf.join("held.txt");
            fs::write(&held, b"")?;
            *lock.borrow_mut() = Some(
                fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(&held)?,
            );
            Ok(())
        };
        let (now, _) = moment();
        let error = restore_wtf_with(&flavor, &zip, &backups, u64::MAX, now, then_lock)
            .expect_err("the half-written WTF stays");
        let Error::RestoreRevertFailed { aside, .. } = &error else {
            panic!("{error}");
        };
        assert!(wtf.join("held.txt").exists(), "the half-written WTF stays");
        assert_eq!(tree(aside), before, "the previous WTF waits aside");
        drop(lock.borrow_mut().take());
    }
}
