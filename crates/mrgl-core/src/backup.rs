//! The toolbox (M3 Toolbox): WTF backup and restore (task 013), later addon sets (named lists of addons
//! per flavor, with enable and disable) and the export and import of an addon list. Nothing here touches
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
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    fs::create_dir_all(into).map_err(|source| io("creating the backups folder", into, source))?;
    let zip = fresh_path(into, BACKUP_PREFIX, ".zip")
        .map_err(|source| io("naming the backup", into, source))?;
    let part = zip.with_extension("zip.part");
    let result = write_zip(&wtf, &part);
    let (files, folders, bytes, warnings) = match result {
        Ok(stats) => stats,
        Err(error) => {
            let _ = fs::remove_file(&part);
            return Err(error);
        }
    };
    fs::rename(&part, &zip).map_err(|source| {
        let _ = fs::remove_file(&part);
        io("finishing the backup", &zip, source)
    })?;
    Ok(Backup {
        zip,
        files,
        folders,
        bytes,
        warnings,
    })
}

/// Writes the zip of a `WTF` folder; the counts of files and folders, the bytes and the warnings.
fn write_zip(wtf: &Path, zip: &Path) -> Result<(usize, usize, u64, Vec<String>)> {
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let file = File::create(zip).map_err(|source| io("writing the backup", zip, source))?;
    let mut writer = ZipWriter::new(BufWriter::new(file));
    let mut stats = Stats::default();
    add_folder(&mut writer, wtf, "", zip, &mut stats)?;
    writer
        .finish()
        .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?
        .into_inner()
        .map_err(|error| io("writing the backup", zip, error.into_error()))?
        .sync_all()
        .map_err(|source| io("writing the backup", zip, source))?;
    Ok((stats.files, stats.folders, stats.bytes, stats.warnings))
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
    writer: &mut ZipWriter<BufWriter<File>>,
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
        .map_err(|source| io("reading WTF", dir, source))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry
            .file_type()
            .map_err(|source| io("reading WTF", &path, source))?;
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
                .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
            stats.folders += 1;
            add_folder(writer, &path, &format!("{entry_name}/"), zip, stats)?;
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|source| io("reading WTF", &path, source))?;
        let modified = metadata.modified().ok();
        let mut options = FullFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(dos_time(modified))
            .large_file(metadata.len() >= u32::MAX as u64);
        if let Some(field) = extended_timestamp(modified) {
            options
                .add_extra_field(EXTENDED_TIMESTAMP, field, false)
                .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
        }
        writer
            .start_file(&entry_name, options)
            .map_err(|reason| io("writing the backup", zip, io::Error::other(reason)))?;
        let mut source =
            File::open(&path).map_err(|source| io("reading a WTF file", &path, source))?;
        let copied = io::copy(&mut source, writer)
            .map_err(|source| io("writing the backup", zip, source))?;
        stats.files += 1;
        stats.bytes += copied;
    }
    Ok(())
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

/// A fresh name `<prefix><local date><suffix>` under a folder: the date as `YYYY-MM-DD-HHMMSS`, and a
/// counter (`-2`, `-3`, …) when the name of this second is taken.
fn fresh_path(dir: &Path, prefix: &str, suffix: &str) -> io::Result<PathBuf> {
    let now = clock::local_civil(SystemTime::now());
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
            context: "reading the backups folder",
            path: folder.to_path_buf(),
            source,
        })?;
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

/// One checked entry of a backup: its index, its path components under `WTF` and whether it is a folder.
struct CheckedEntry {
    index: usize,
    parts: Vec<String>,
    is_dir: bool,
}

/// Puts a backup back as the flavor's `WTF` folder. In this order: the guard of task 023 (a running
/// client refuses the restore before anything is touched), every entry checked (one that would land
/// outside `WTF` refuses the whole zip), the current `WTF` moved aside into the backups folder as
/// `WTF-before-restore-<date>`, the zip unpacked into a new `WTF`. A failure while unpacking removes the
/// half-written folder and puts the aside one back ([`Error::RestoreFailed`]); when even that fails, the
/// aside folder stays where it is and the error names it ([`Error::RestoreRevertFailed`]).
pub fn restore_wtf(flavor: &Flavor, zip: &Path, backups: &Path) -> Result<Restored> {
    client::ensure_client_not_running(flavor)?;
    let (mut archive, entries) = checked_entries(zip)?;
    let wtf = wtf_dir(flavor);
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    let aside = if wtf.exists() {
        fs::create_dir_all(backups)
            .map_err(|source| io("creating the backups folder", backups, source))?;
        let aside = fresh_path(backups, ASIDE_PREFIX, "")
            .map_err(|source| io("naming the aside folder", backups, source))?;
        move_tree(&wtf, &aside).map_err(|source| io("moving WTF aside", &wtf, source))?;
        Some(aside)
    } else {
        None
    };
    match unpack(&mut archive, &entries, &wtf) {
        Ok((files, folders)) => Ok(Restored {
            zip: zip.to_path_buf(),
            files,
            folders,
            aside,
        }),
        Err(reason) => {
            let _ = fs::remove_dir_all(&wtf);
            let reverted = match &aside {
                Some(aside) => !wtf.exists() && move_tree(aside, &wtf).is_ok(),
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

/// Opens a backup and checks every entry before anything is written: the name must be a relative path
/// under `WTF` ([`safe_entry_path`]) and the entry must not be a symbolic link.
fn checked_entries(zip: &Path) -> Result<(ZipArchive<BufReader<File>>, Vec<CheckedEntry>)> {
    let unreadable = |reason: String| Error::BackupUnreadable {
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
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|reason| unreadable(format!("entry {index}: {reason}")))?;
        let raw = entry
            .name()
            .map_err(|reason| unreadable(format!("entry {index}: {reason}")))?
            .into_owned();
        let escapes = || Error::BackupEntryEscapes {
            path: zip.to_path_buf(),
            entry: raw.clone(),
        };
        let parts = safe_entry_path(&raw).ok_or_else(escapes)?;
        if entry.is_symlink() {
            return Err(escapes());
        }
        if parts.is_empty() {
            continue;
        }
        entries.push(CheckedEntry {
            index,
            parts,
            is_dir: entry.is_dir(),
        });
    }
    Ok((archive, entries))
}

/// Unpacks the checked entries under a fresh `WTF`: the folders, then every file with its modification
/// time from the extended timestamp field, else from the zip's own date fields read as local time.
fn unpack(
    archive: &mut ZipArchive<BufReader<File>>,
    entries: &[CheckedEntry],
    wtf: &Path,
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| context("creating", parent, error))?;
        }
        let mut entry = archive.by_index(checked.index).map_err(io::Error::other)?;
        let modified = entry_modified(&entry);
        let file = File::create(&path).map_err(|error| context("writing", &path, error))?;
        let mut output = BufWriter::new(file);
        io::copy(&mut entry, &mut output).map_err(|error| context("writing", &path, error))?;
        let file = output
            .into_inner()
            .map_err(|error| context("writing", &path, error.into_error()))?;
        if let Some(modified) = modified {
            file.set_modified(modified)
                .map_err(|error| context("dating", &path, error))?;
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

/// A zip entry name as path components under `WTF`, or `None` when it would land outside it: an
/// absolute path, a drive letter, a `..` component, a component with a character Windows refuses or a
/// reserved device name. `\` counts as a separator, `.` and empty components are dropped.
pub fn safe_entry_path(name: &str) -> Option<Vec<String>> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in name.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            part => {
                if part
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
                    || is_reserved_device_name(part)
                {
                    return None;
                }
                parts.push(part.to_string());
            }
        }
    }
    Some(parts)
}

/// `CON`, `NUL`, `COM1` … `LPT9`, with or without an extension: a file Windows will not create.
fn is_reserved_device_name(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit()
                && stem.as_bytes()[3] != b'0'
        }
    }
}

/// Moves a folder: one rename on the same volume; across volumes (the game on one drive, the mrgl home
/// on another) a copy of the tree and then the removal of the original.
fn move_tree(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_path_stays_under_wtf() {
        assert_eq!(
            safe_entry_path("Account/NAME/SavedVariables/X.lua").as_deref(),
            Some(&["Account", "NAME", "SavedVariables", "X.lua"].map(String::from)[..])
        );
        assert_eq!(
            safe_entry_path("Account\\NAME\\./AddOns.txt").as_deref(),
            Some(&["Account", "NAME", "AddOns.txt"].map(String::from)[..])
        );
        assert_eq!(
            safe_entry_path("Account/").as_deref(),
            Some(&["Account".to_string()][..])
        );
        assert_eq!(safe_entry_path("").as_deref(), Some(&[][..]));
        assert_eq!(safe_entry_path("../outside.txt"), None);
        assert_eq!(safe_entry_path("Account/../../outside.txt"), None);
        assert_eq!(safe_entry_path("/etc/passwd"), None);
        assert_eq!(safe_entry_path("C:/Windows/x"), None);
        assert_eq!(safe_entry_path("Account/NUL"), None);
        assert_eq!(safe_entry_path("Account/com1.txt"), None);
        assert_eq!(safe_entry_path("Account/bad<name>.lua"), None);
        assert_eq!(
            safe_entry_path("COM10").as_deref(),
            Some(&["COM10".to_string()][..])
        );
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
        assert_eq!(dos_time(Some(UNIX_EPOCH)), DateTime::default());
    }

    #[test]
    fn a_fresh_name_counts_up_within_a_second() {
        let dir = std::env::temp_dir().join(format!("mrgl-backup-names-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the folder");
        let first = fresh_path(&dir, BACKUP_PREFIX, ".zip").expect("a name");
        let name = first.file_name().expect("a name").to_string_lossy();
        assert!(name.starts_with("WTF-") && name.ends_with(".zip"), "{name}");
        assert_eq!(name.len(), "WTF-2026-10-21-193605.zip".len(), "{name}");
        fs::write(&first, b"").expect("taken");
        let second = fresh_path(&dir, BACKUP_PREFIX, ".zip").expect("a name");
        assert_ne!(second, first);
        assert!(
            second
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .ends_with("-2.zip"),
            "{}",
            second.display()
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
