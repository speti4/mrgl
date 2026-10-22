//! The SavedVariables of a flavor (task 025, M3 Toolbox): every `<Addon>.lua` the client keeps under
//! `WTF\Account\<ACCOUNT>\SavedVariables\` (account-wide) and `WTF\Account\<ACCOUNT>\<Realm>\<Character>\
//! SavedVariables\` (per character), each with its optional `<Addon>.lua.bak`, listed with its size, and the
//! cleanup of the orphans into a backup zip. A `.lua` and its `.lua.bak` are one entry; any other file or
//! folder there is neither listed nor touched, and a link is not followed.
//!
//! The orphan rule: a file is an orphan when no addon folder of its name is under `Interface\AddOns`. The
//! client names the file after the addon's folder, and Windows does not tell the cases apart, so the names
//! are compared without regard to case. An addon folder counts when the scan finds a TOC named after it
//! ([`crate::scan`]): a folder without one is not an addon the client loads, so its file is an orphan; a
//! folder the scan cannot read counts, since nothing can be told about it. A disabled addon keeps its folder,
//! so it is never an orphan (`AddOns.txt` is not read). A `Blizzard_` file belongs to the client's own
//! interface, which has no folder there, and is never an orphan ([`scan::is_client_addon`], the rule the
//! dependency check shares). With no `Interface\AddOns` folder at all, nothing is marked an orphan and a
//! warning says so: a missing folder says more about the path than about the addons.
//!
//! The cleanup is the third `WTF` write of D-002 and D-003. The guard of task 023 comes first (a running
//! client refuses it before anything is read); then the orphans and their `.bak` files go into a dated zip,
//! `SavedVariables-<date>.zip` in the backups folder of task 013, written by the backup's zip writer with the
//! entry names relative to `WTF`, so unzipping it into `WTF` puts every file back where it was. A file is
//! removed from `WTF` only once its entry in the finished zip has been read back and compared byte for byte:
//! the cleanup moves, it never deletes what the zip does not hold. `backup --restore` refuses such a zip
//! ([`Error::NotAWtfBackup`]): a restore replaces all of `WTF`, and this zip holds a few files.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use zip::ZipArchive;

use crate::backup;
use crate::client;
use crate::error::{Error, Result};
use crate::flavor::Flavor;
use crate::scan;
use crate::toc::TocSuffix;
use crate::voice::{Kind, count_noun};
use crate::wtf::{self, SAVED_VARIABLES_DIR, WtfFolder, link_warning};
use crate::zip_entry::size_text;

/// A cleanup zip's name: `SavedVariables-<date>.zip`, next to the `WTF` backups.
pub const CLEANUP_PREFIX: &str = "SavedVariables-";

/// What the AddOns folder says about a SavedVariables file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// An addon folder of its name is under `Interface\AddOns` (enabled or not).
    Installed,
    /// No addon folder of its name: `--clean` moves it.
    Orphan,
    /// The client's own (`Blizzard_…`), which has no addon folder.
    Client,
    /// There is no `Interface\AddOns` folder to compare with.
    Unknown,
}

/// One addon's SavedVariables in one place: the `.lua`, its `.lua.bak`, or both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedVariablesFile {
    /// The addon name the file carries, as on disk: `<Addon>` of `<Addon>.lua` (of the `.bak` when alone).
    pub addon: String,
    /// The account folder's name.
    pub account: String,
    /// The realm folder's name; `None` for an account-wide file.
    pub realm: Option<String>,
    /// The character folder's name; `None` for an account-wide file.
    pub character: Option<String>,
    /// The `.lua`, when there is one.
    pub lua: Option<PathBuf>,
    /// Its size; 0 without one.
    pub lua_bytes: u64,
    /// The `.lua.bak`, when there is one.
    pub bak: Option<PathBuf>,
    /// Its size; 0 without one.
    pub bak_bytes: u64,
    /// Both sizes together, what the listing sorts by.
    pub bytes: u64,
    pub status: Status,
}

impl SavedVariablesFile {
    /// Whether `--clean` moves it.
    pub fn is_orphan(&self) -> bool {
        self.status == Status::Orphan
    }

    /// The files on disk: the `.lua`, then the `.lua.bak`.
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.lua.iter().chain(self.bak.iter())
    }

    /// Where it is, for the plain lines: `account-wide`, or `realm / character`. The account folder's
    /// name is left out: on an older install it is the login name, and a pasted output should not carry
    /// it (`--json` has it in `account`).
    pub fn place(&self) -> String {
        match (&self.realm, &self.character) {
            (Some(realm), Some(character)) => format!("{realm} / {character}"),
            _ => "account-wide".to_string(),
        }
    }
}

/// The SavedVariables of a flavor, as listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    /// The flavor's `WTF` folder.
    pub wtf: PathBuf,
    /// Every entry, the largest first (then by account, realm, character and addon).
    pub files: Vec<SavedVariablesFile>,
    /// The bytes of every entry.
    pub bytes: u64,
    /// The orphans among them.
    pub orphans: usize,
    /// The bytes of the orphans.
    pub orphan_bytes: u64,
    /// Plain lines about what was not read or not compared: a link, an unreadable addon folder, a missing
    /// AddOns folder.
    pub warnings: Vec<String>,
}

/// What the listing's summary names as the way to move the orphans: the command line's
/// `mrgl savedvars --clean`, or the window's cleanup button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupBy {
    /// `mrgl savedvars`: "`mrgl savedvars --clean` moves them into a backup zip".
    Command,
    /// The window's toolbox: "the cleanup moves them into a backup zip".
    Window,
}

impl CleanupBy {
    const fn name(self) -> &'static str {
        match self {
            CleanupBy::Command => "`mrgl savedvars --clean`",
            CleanupBy::Window => "the cleanup",
        }
    }
}

impl Listing {
    /// The summary of a listing in the voice's terms (task 086: `mrgl savedvars` and the window's toolbox
    /// share it): the count, the size and the orphans, the verb agreeing with their count
    /// (`1 orphan (12 B) has no addon folder`, `2 orphans (50 B) have …`). A warning when there are
    /// orphans, a success without, nothing to say about an empty `WTF` folder.
    pub fn summary(&self, folder_name: &str, cleanup: CleanupBy) -> (Kind, String) {
        if self.files.is_empty() {
            return (
                Kind::Nothing,
                format!("no SavedVariables files in the WTF folder of {folder_name}"),
            );
        }
        let count = count_noun(self.files.len(), "SavedVariables file");
        let size = size_text(self.bytes);
        if self.orphans == 0 {
            return (
                Kind::Success,
                format!("{count} of {folder_name}, {size} in all, no orphans"),
            );
        }
        let (has, them) = if self.orphans == 1 {
            ("has", "it")
        } else {
            ("have", "them")
        };
        (
            Kind::Warning,
            format!(
                "{count} of {folder_name}, {size} in all; {} ({}) {has} no addon folder: {} moves {them} into a backup zip",
                count_noun(self.orphans, "orphan"),
                size_text(self.orphan_bytes),
                cleanup.name()
            ),
        )
    }
}

/// Lists a flavor's SavedVariables and marks each against its `Interface\AddOns` folder. A missing `WTF`
/// folder is an empty listing, never an error. Nothing is written.
pub fn list_saved_variables(flavor: &Flavor) -> Result<Listing> {
    let mut warnings = Vec::new();
    let installed = installed_names(
        &scan::addons_dir(&flavor.folder),
        &flavor.toc_chain(),
        &mut warnings,
    )?;
    list_at(&backup::wtf_dir(flavor), installed.as_ref(), warnings)
}

/// The names (lower case) of the addon folders a file can belong to: every folder the scan reads as an
/// addon and every folder it cannot read. `None`, with a warning, when there is no AddOns folder.
fn installed_names(
    addons_dir: &Path,
    chain: &[Option<TocSuffix>],
    warnings: &mut Vec<String>,
) -> Result<Option<HashSet<String>>> {
    if !addons_dir.is_dir() {
        warnings.push(format!(
            "no AddOns folder at {}: no SavedVariables file is marked an orphan",
            addons_dir.display()
        ));
        return Ok(None);
    }
    let addons = scan::scan_addons(addons_dir, chain)?;
    warnings.extend(addons.warnings);
    let mut names: HashSet<String> = addons
        .addons
        .iter()
        .map(|addon| addon.folder.to_lowercase())
        .collect();
    // A skipped folder with no TOC is not an addon; one the scan could not read might be.
    names.extend(
        addons
            .skipped
            .iter()
            .filter(|folder| fs::read_dir(addons_dir.join(folder)).is_err())
            .map(|folder| folder.to_lowercase()),
    );
    Ok(Some(names))
}

/// Lists the SavedVariables under a `WTF` folder against the installed names (`None`: unknown).
fn list_at(
    wtf: &Path,
    installed: Option<&HashSet<String>>,
    mut warnings: Vec<String>,
) -> Result<Listing> {
    let mut files = Vec::new();
    wtf::walk(wtf, &mut warnings, |folder, warnings| {
        let (place, folder) = match folder {
            WtfFolder::Account { account, folder } => (
                Place {
                    account,
                    realm: None,
                    character: None,
                },
                folder,
            ),
            WtfFolder::Character {
                account,
                realm,
                name,
                folder,
            } => (
                Place {
                    account,
                    realm: Some(realm),
                    character: Some(name),
                },
                folder,
            ),
        };
        collect(
            &folder.join(SAVED_VARIABLES_DIR),
            &place,
            installed,
            &mut files,
            warnings,
        )
    })?;
    files.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| a.account.cmp(&b.account))
            .then_with(|| a.realm.cmp(&b.realm))
            .then_with(|| a.character.cmp(&b.character))
            .then_with(|| a.addon.to_lowercase().cmp(&b.addon.to_lowercase()))
    });
    let orphans: Vec<&SavedVariablesFile> = files.iter().filter(|file| file.is_orphan()).collect();
    Ok(Listing {
        wtf: wtf.to_path_buf(),
        bytes: files.iter().map(|file| file.bytes).sum(),
        orphans: orphans.len(),
        orphan_bytes: orphans.iter().map(|file| file.bytes).sum(),
        files,
        warnings,
    })
}

/// The account, realm and character a SavedVariables folder belongs to.
struct Place<'a> {
    account: &'a str,
    realm: Option<&'a str>,
    character: Option<&'a str>,
}

// coverage: os-begin — built only by the arms below that the operating system alone provokes
fn io_error(context: &'static str, path: &Path, source: io::Error) -> Error {
    Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    }
}
// coverage: os-end

/// One `.lua` and one `.lua.bak` of an addon, as found in a folder.
#[derive(Default)]
struct Pair {
    lua: Option<(String, PathBuf, u64)>,
    bak: Option<(String, PathBuf, u64)>,
}

/// Collects the entries of one SavedVariables folder; a missing folder has none.
fn collect(
    dir: &Path,
    place: &Place<'_>,
    installed: Option<&HashSet<String>>,
    files: &mut Vec<SavedVariablesFile>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            warnings.push(link_warning(dir));
            return Ok(());
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error("reading WTF", dir, source)), // coverage: os
    }
    let entries = fs::read_dir(dir).map_err(|source| io_error("reading WTF", dir, source))?;
    let mut pairs: BTreeMap<String, Pair> = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error("reading WTF", dir, source))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((addon, is_bak)) = addon_of(&name) else {
            continue;
        };
        let file_type = entry
            .file_type()
            .map_err(|source| io_error("reading WTF", &path, source))?; // coverage: os
        if file_type.is_symlink() {
            warnings.push(link_warning(&path));
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let bytes = entry
            .metadata()
            .map_err(|source| io_error("reading WTF", &path, source))? // coverage: os
            .len();
        let pair = pairs.entry(addon.to_lowercase()).or_default();
        let found = Some((addon.to_string(), path, bytes));
        if is_bak {
            pair.bak = found;
        } else {
            pair.lua = found;
        }
    }
    for (key, pair) in pairs {
        let addon = pair
            .lua
            .as_ref()
            .or(pair.bak.as_ref())
            .map(|(addon, _, _)| addon.clone())
            .unwrap_or_default();
        let status = if scan::is_client_addon(&key) {
            Status::Client
        } else {
            match installed {
                None => Status::Unknown,
                Some(names) if names.contains(&key) => Status::Installed,
                Some(_) => Status::Orphan,
            }
        };
        let (lua, lua_bytes) = pair
            .lua
            .map(|(_, path, bytes)| (Some(path), bytes))
            .unwrap_or_default();
        let (bak, bak_bytes) = pair
            .bak
            .map(|(_, path, bytes)| (Some(path), bytes))
            .unwrap_or_default();
        files.push(SavedVariablesFile {
            addon,
            account: place.account.to_string(),
            realm: place.realm.map(str::to_string),
            character: place.character.map(str::to_string),
            lua,
            lua_bytes,
            bak,
            bak_bytes,
            bytes: lua_bytes + bak_bytes,
            status,
        });
    }
    Ok(())
}

/// The addon a SavedVariables file name belongs to, and whether it is the `.bak`: `Foo.lua` is `Foo`,
/// `Foo.lua.bak` is `Foo` too (the extensions in any case); any other name is `None`.
fn addon_of(name: &str) -> Option<(&str, bool)> {
    let lower = name.to_ascii_lowercase();
    let (length, is_bak) = if lower.ends_with(".lua.bak") {
        (".lua.bak".len(), true)
    } else if lower.ends_with(".lua") {
        (".lua".len(), false)
    } else {
        return None;
    };
    // The suffix is ASCII and ASCII lower-casing keeps every byte where it was, so the cut is on a
    // character boundary.
    let addon = &name[..name.len() - length];
    (!addon.is_empty()).then_some((addon, is_bak))
}

/// A cleanup that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cleaned {
    /// The zip the orphans went into; `None` when there was no orphan (nothing was written).
    pub zip: Option<PathBuf>,
    /// The orphans moved, as they were listed.
    pub moved: Vec<SavedVariablesFile>,
    /// The files moved, the `.lua` and the `.lua.bak` files counted apart.
    pub files: usize,
    /// Their bytes.
    pub bytes: u64,
    /// The listing's warnings.
    pub warnings: Vec<String>,
}

impl Cleaned {
    /// The summary of a cleanup in the voice's terms (task 086: `mrgl savedvars --clean` and the window's
    /// toolbox share it): what moved into which zip and how to put it back, or that there was no orphan.
    pub fn summary(&self, folder_name: &str) -> (Kind, String) {
        match &self.zip {
            None => (
                Kind::Nothing,
                format!("no orphans in the SavedVariables of {folder_name}, nothing moved"),
            ),
            Some(zip) => (
                Kind::Success,
                format!(
                    "moved {} of {folder_name} ({}, {}) into {}; unzip it into the WTF folder to put them back",
                    count_noun(self.moved.len(), "orphan"),
                    count_noun(self.files, "file"),
                    size_text(self.bytes),
                    zip.display()
                ),
            ),
        }
    }
}

/// Moves a flavor's orphan SavedVariables, with their `.bak` files, into `SavedVariables-<date>.zip` under
/// `into` (the backups folder, created when needed). In this order: the guard of task 023 (a running
/// client refuses the cleanup before anything is read), the listing, the zip written by the backup's zip
/// writer with the entry names relative to `WTF`, and every file removed from `WTF` once its entry has been
/// read back from the finished zip and found equal byte for byte. With no orphan nothing is written. A
/// failure after the zip is written stops the removals ([`Error::CleanupIncomplete`]): what was removed is
/// in the zip, the rest stays where it was.
pub fn clean_orphans(flavor: &Flavor, into: &Path) -> Result<Cleaned> {
    client::ensure_client_not_running(flavor)?;
    let listing = list_saved_variables(flavor)?;
    let moved: Vec<SavedVariablesFile> = listing
        .files
        .into_iter()
        .filter(SavedVariablesFile::is_orphan)
        .collect();
    let mut cleaned = Cleaned {
        zip: None,
        files: 0,
        bytes: 0,
        moved,
        warnings: listing.warnings,
    };
    if cleaned.moved.is_empty() {
        return Ok(cleaned);
    }
    let entries: Vec<(String, PathBuf)> = cleaned
        .moved
        .iter()
        .flat_map(SavedVariablesFile::paths)
        .map(|path| (entry_name(&listing.wtf, path), path.clone()))
        .collect();
    let (zip, bytes) = backup::zip_files(into, CLEANUP_PREFIX, &entries)?;
    remove_checked(&zip, &entries)?;
    cleaned.zip = Some(zip);
    cleaned.files = entries.len();
    cleaned.bytes = bytes;
    Ok(cleaned)
}

/// A file's entry name in the cleanup zip: its path under `WTF`, `/`-separated.
fn entry_name(wtf: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(wtf).unwrap_or(path);
    relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Removes every file whose entry in the finished zip reads back equal to it, in order; the first file
/// that differs or cannot be removed stops the rest.
fn remove_checked(zip: &Path, entries: &[(String, PathBuf)]) -> Result<()> {
    let stopped = |removed: usize, reason: String| Error::CleanupIncomplete {
        zip: zip.to_path_buf(),
        removed,
        total: entries.len(),
        reason,
    };
    let mut archive = File::open(zip)
        .map_err(|error| error.to_string())
        .and_then(|file| ZipArchive::new(BufReader::new(file)).map_err(|error| error.to_string()))
        .map_err(|reason| stopped(0, format!("reading {} back: {reason}", zip.display())))?;
    for (removed, (entry_name, path)) in entries.iter().enumerate() {
        let same = archive
            .by_name(entry_name)
            .map_err(io::Error::other)
            .and_then(|entry| {
                let file = File::open(path)?;
                same_bytes(entry, BufReader::new(file))
            })
            .map_err(|error| {
                stopped(
                    removed,
                    format!("checking {} against the zip: {error}", path.display()),
                )
            })?;
        if !same {
            return Err(stopped(
                removed,
                format!(
                    "{} differs from its copy in the zip, so it was not removed",
                    path.display()
                ),
            ));
        }
        fs::remove_file(path)
            .map_err(|error| stopped(removed, format!("removing {}: {error}", path.display())))?;
    }
    Ok(())
}

/// Whether two readers give the same bytes, read in chunks.
fn same_bytes(mut a: impl Read, mut b: impl Read) -> io::Result<bool> {
    let mut left = vec![0u8; 64 * 1024];
    let mut right = vec![0u8; 64 * 1024];
    loop {
        let count = read_full(&mut a, &mut left)?;
        if read_full(&mut b, &mut right[..count])? != count {
            return Ok(false);
        }
        if left[..count] != right[..count] {
            return Ok(false);
        }
        if count < left.len() {
            // `a` is at its end: `b` must be too.
            return Ok(b.read(&mut right[..1])? == 0);
        }
    }
}

/// Reads until the buffer is full or the reader ends; the bytes read.
fn read_full(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_file_name_gives_its_addon() {
        assert_eq!(addon_of("Questie.lua"), Some(("Questie", false)));
        assert_eq!(addon_of("Questie.lua.bak"), Some(("Questie", true)));
        assert_eq!(addon_of("Questie.LUA"), Some(("Questie", false)));
        assert_eq!(addon_of("Questie.Lua.BAK"), Some(("Questie", true)));
        assert_eq!(addon_of("DBM-Core.lua"), Some(("DBM-Core", false)));
        assert_eq!(addon_of("Ünïcode.lua"), Some(("Ünïcode", false)));
        assert_eq!(addon_of(".lua"), None);
        assert_eq!(addon_of("Questie.lua.old"), None);
        assert_eq!(addon_of("notes.txt"), None);
        assert_eq!(addon_of("Questie"), None);
    }

    #[test]
    fn an_entry_name_is_the_path_under_wtf_with_slashes() {
        let wtf = Path::new("Q:\\wow\\_forever_\\WTF");
        assert_eq!(
            entry_name(
                wtf,
                &wtf.join("Account")
                    .join("ACC")
                    .join("Some Realm")
                    .join("Char")
                    .join("SavedVariables")
                    .join("Gone.lua.bak")
            ),
            "Account/ACC/Some Realm/Char/SavedVariables/Gone.lua.bak"
        );
    }

    #[test]
    fn two_readers_compare_byte_for_byte_across_chunks() {
        let long: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
        assert!(same_bytes(&long[..], &long[..]).expect("read"));
        assert!(same_bytes(&b""[..], &b""[..]).expect("read"));
        assert!(!same_bytes(&long[..], &long[..long.len() - 1]).expect("read"));
        assert!(!same_bytes(&long[..long.len() - 1], &long[..]).expect("read"));
        assert!(!same_bytes(&long[..65_536], &long[..]).expect("read"));
        let mut other = long.clone();
        other[150_000] ^= 1;
        assert!(!same_bytes(&long[..], &other[..]).expect("read"));
    }

    fn entry(addon: &str, bytes: u64, status: Status) -> SavedVariablesFile {
        SavedVariablesFile {
            addon: addon.into(),
            account: "ACC".into(),
            realm: None,
            character: None,
            lua: Some(PathBuf::from(format!("{addon}.lua"))),
            lua_bytes: bytes,
            bak: None,
            bak_bytes: 0,
            bytes,
            status,
        }
    }

    fn listing(files: Vec<SavedVariablesFile>) -> Listing {
        let orphans: Vec<&SavedVariablesFile> =
            files.iter().filter(|file| file.is_orphan()).collect();
        Listing {
            wtf: PathBuf::from("WTF"),
            bytes: files.iter().map(|file| file.bytes).sum(),
            orphans: orphans.len(),
            orphan_bytes: orphans.iter().map(|file| file.bytes).sum(),
            files,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn the_listing_summary_agrees_with_the_orphan_count() {
        let one = listing(vec![
            entry("Kept", 100, Status::Installed),
            entry("Gone", 12, Status::Orphan),
        ]);
        assert_eq!(
            one.summary("_forever_", CleanupBy::Command),
            (
                Kind::Warning,
                "2 SavedVariables files of _forever_, 112 B in all; 1 orphan (12 B) has no addon folder: `mrgl savedvars --clean` moves it into a backup zip".to_string()
            )
        );
        assert_eq!(
            one.summary("_forever_", CleanupBy::Window).1,
            "2 SavedVariables files of _forever_, 112 B in all; 1 orphan (12 B) has no addon folder: the cleanup moves it into a backup zip"
        );
        let two = listing(vec![
            entry("Kept", 100, Status::Installed),
            entry("Gone", 12, Status::Orphan),
            entry("GoneToo", 38, Status::Orphan),
        ]);
        assert_eq!(
            two.summary("_forever_", CleanupBy::Window),
            (
                Kind::Warning,
                "3 SavedVariables files of _forever_, 150 B in all; 2 orphans (50 B) have no addon folder: the cleanup moves them into a backup zip".to_string()
            )
        );
        let none = listing(vec![entry("Kept", 2048, Status::Installed)]);
        assert_eq!(
            none.summary("_forever_", CleanupBy::Command),
            (
                Kind::Success,
                "1 SavedVariables file of _forever_, 2.0 KB in all, no orphans".to_string()
            )
        );
        assert_eq!(
            listing(Vec::new()).summary("_forever_", CleanupBy::Window),
            (
                Kind::Nothing,
                "no SavedVariables files in the WTF folder of _forever_".to_string()
            )
        );
    }

    #[test]
    fn the_cleanup_summary_names_the_zip_or_says_nothing_moved() {
        let zip = PathBuf::from("backups").join("SavedVariables-2026-10-22.zip");
        let cleaned =
            |moved: Vec<SavedVariablesFile>, files: usize, zip: Option<PathBuf>| Cleaned {
                bytes: moved.iter().map(|file| file.bytes).sum(),
                moved,
                files,
                zip,
                warnings: Vec::new(),
            };
        assert_eq!(
            cleaned(
                vec![entry("Gone", 12, Status::Orphan)],
                1,
                Some(zip.clone())
            )
            .summary("_forever_"),
            (
                Kind::Success,
                format!(
                    "moved 1 orphan of _forever_ (1 file, 12 B) into {}; unzip it into the WTF folder to put them back",
                    zip.display()
                )
            )
        );
        let two = vec![
            entry("Gone", 12, Status::Orphan),
            entry("GoneToo", 38, Status::Orphan),
        ];
        assert_eq!(
            cleaned(two, 3, Some(zip.clone())).summary("_forever_").1,
            format!(
                "moved 2 orphans of _forever_ (3 files, 50 B) into {}; unzip it into the WTF folder to put them back",
                zip.display()
            )
        );
        assert_eq!(
            cleaned(Vec::new(), 0, None).summary("_forever_"),
            (
                Kind::Nothing,
                "no orphans in the SavedVariables of _forever_, nothing moved".to_string()
            )
        );
    }

    #[test]
    fn a_place_reads_as_account_wide_or_the_character_path_without_the_account() {
        let mut file = SavedVariablesFile {
            addon: "Foo".into(),
            account: "ACC".into(),
            realm: None,
            character: None,
            lua: None,
            lua_bytes: 0,
            bak: None,
            bak_bytes: 0,
            bytes: 0,
            status: Status::Orphan,
        };
        assert_eq!(file.place(), "account-wide");
        file.realm = Some("Some Realm".into());
        file.character = Some("Char".into());
        assert_eq!(file.place(), "Some Realm / Char");
        let json = serde_json::to_value(&file).expect("serializes");
        assert_eq!(json["status"], "orphan");
        assert_eq!(json["realm"], "Some Realm");
        assert_eq!(json["account"], "ACC", "the account stays in the JSON");
    }

    /// A temporary folder under the system temp dir, removed before use and when dropped.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Temp {
            let dir =
                std::env::temp_dir().join(format!("mrgl-savedvars-{}-{name}", std::process::id()));
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

    /// `WTF\Account\<account>[\<realm>\<character>]\SavedVariables`, created.
    fn saved_variables(wtf: &Path, account: &str, character: Option<(&str, &str)>) -> PathBuf {
        let mut dir = wtf.join("Account").join(account);
        if let Some((realm, name)) = character {
            dir = dir.join(realm).join(name);
        }
        let dir = dir.join(SAVED_VARIABLES_DIR);
        fs::create_dir_all(&dir).expect("the folder");
        dir
    }

    #[test]
    fn files_of_one_size_sort_by_account_realm_character_and_addon() {
        let temp = Temp::new("sort");
        let wtf = temp.path().join("WTF");
        let account_a = saved_variables(&wtf, "A", None);
        let account_b = saved_variables(&wtf, "B", None);
        let char_one = saved_variables(&wtf, "A", Some(("Realm", "One")));
        let char_two = saved_variables(&wtf, "A", Some(("Realm", "Two")));
        let other_realm = saved_variables(&wtf, "A", Some(("Other", "One")));
        for (dir, name) in [
            (&account_b, "Bb.lua"),
            (&char_two, "Dd.lua"),
            (&other_realm, "Cc.lua"),
            (&char_one, "Cc.lua"),
            (&account_a, "zz.lua"),
            (&account_a, "Aa.lua"),
        ] {
            fs::write(dir.join(name), b"ten bytes!").expect("a file");
        }
        fs::write(account_a.join("Big.lua"), b"eleven bytes").expect("a file");
        let listing = list_at(&wtf, None, Vec::new()).expect("a listing");
        let order: Vec<String> = listing
            .files
            .iter()
            .map(|file| format!("{} {} {}", file.account, file.place(), file.addon))
            .collect();
        assert_eq!(
            order,
            [
                "A account-wide Big",
                "A account-wide Aa",
                "A account-wide zz",
                "A Other / One Cc",
                "A Realm / One Cc",
                "A Realm / Two Dd",
                "B account-wide Bb",
            ]
        );
        assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);
    }

    #[test]
    fn a_saved_variables_folder_that_is_a_file_and_an_entry_that_is_a_folder_are_passed_over() {
        let temp = Temp::new("shapes");
        let wtf = temp.path().join("WTF");
        let account = saved_variables(&wtf, "ACC", None);
        fs::write(account.join("Real.lua"), b"x").expect("a file");
        fs::create_dir_all(account.join("Folder.lua")).expect("a folder with the name");
        let character = wtf.join("Account").join("ACC").join("Realm").join("Char");
        fs::create_dir_all(&character).expect("the character");
        fs::write(character.join(SAVED_VARIABLES_DIR), b"a file, not a folder").expect("a file");
        let listing = list_at(&wtf, None, Vec::new()).expect("a listing");
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.addon.as_str())
            .collect();
        assert_eq!(names, ["Real"]);
        assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);
    }

    #[test]
    fn a_character_without_a_saved_variables_folder_is_passed_over_without_a_warning() {
        let temp = Temp::new("no-folder");
        let wtf = temp.path().join("WTF");
        let account = saved_variables(&wtf, "ACC", None);
        fs::write(account.join("Real.lua"), b"x").expect("a file");
        let character = wtf.join("Account").join("ACC").join("Realm").join("Char");
        fs::create_dir_all(&character).expect("the character, with no SavedVariables folder");
        let listing = list_at(&wtf, None, Vec::new()).expect("a listing");
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.addon.as_str())
            .collect();
        assert_eq!(names, ["Real"]);
        assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_as_the_folder_or_as_an_entry_is_a_warning_and_not_followed() {
        let temp = Temp::new("junctions");
        let wtf = temp.path().join("WTF");
        let elsewhere = temp.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).expect("a folder");
        fs::write(elsewhere.join("Linked.lua"), b"behind the junction").expect("a file");
        let account = saved_variables(&wtf, "ACC", None);
        fs::write(account.join("Real.lua"), b"x").expect("a file");
        let entry_link = account.join("Entry.lua");
        crate::link::create_junction(&entry_link, &elsewhere).expect("a junction");
        let character = wtf.join("Account").join("ACC").join("Realm").join("Char");
        fs::create_dir_all(&character).expect("the character");
        let folder_link = character.join(SAVED_VARIABLES_DIR);
        crate::link::create_junction(&folder_link, &elsewhere).expect("a junction");

        let listing = list_at(&wtf, None, Vec::new()).expect("a listing");
        let names: Vec<&str> = listing
            .files
            .iter()
            .map(|file| file.addon.as_str())
            .collect();
        assert_eq!(names, ["Real"]);
        assert_eq!(
            listing.warnings,
            [link_warning(&entry_link), link_warning(&folder_link)]
        );
        let _ = fs::remove_dir(&entry_link);
        let _ = fs::remove_dir(&folder_link);
    }

    #[test]
    fn an_entry_name_outside_wtf_keeps_the_plain_parts_of_the_path() {
        let wtf = Path::new("Q:\\wow\\_forever_\\WTF");
        assert_eq!(
            entry_name(wtf, Path::new("Q:\\elsewhere\\Gone.lua")),
            "elsewhere/Gone.lua"
        );
    }

    #[test]
    fn a_file_is_removed_only_once_its_copy_in_the_zip_reads_back_the_same() {
        let temp = Temp::new("remove-checked");
        let wtf = temp.path().join("WTF");
        let dir = saved_variables(&wtf, "ACC", None);
        let gone = dir.join("Gone.lua");
        let same = dir.join("Same.lua");
        fs::write(&gone, b"GoneDB = {}\n").expect("a file");
        fs::write(&same, b"SameDB = {}\n").expect("a file");
        let entries = vec![
            (entry_name(&wtf, &same), same.clone()),
            (entry_name(&wtf, &gone), gone.clone()),
        ];
        let (zip, _) = backup::zip_files(&temp.path().join("backups"), CLEANUP_PREFIX, &entries)
            .expect("a zip");

        // The zip gone: nothing is removed.
        let missing = temp.path().join("missing.zip");
        let error = remove_checked(&missing, &entries).expect_err("no zip");
        let Error::CleanupIncomplete {
            zip: named,
            removed,
            total,
            reason,
        } = &error
        else {
            panic!("{error}");
        };
        assert_eq!((named, *removed, *total), (&missing, 0, 2));
        assert!(
            reason.starts_with(&format!("reading {} back: ", missing.display())),
            "{reason}"
        );
        assert!(gone.exists() && same.exists());

        // A file that changed since the zip was written stays, and so does every file after it.
        fs::write(&gone, b"GoneDB = { changed = true }\n").expect("changed");
        let error = remove_checked(&zip, &entries).expect_err("differs");
        let Error::CleanupIncomplete {
            removed, reason, ..
        } = &error
        else {
            panic!("{error}");
        };
        assert_eq!(*removed, 1, "the first file was the same and went");
        assert_eq!(
            reason,
            &format!(
                "{} differs from its copy in the zip, so it was not removed",
                gone.display()
            )
        );
        assert!(!same.exists() && gone.exists());

        // An entry the zip does not hold: the check itself fails.
        fs::write(&same, b"SameDB = {}\n").expect("back");
        let other = dir.join("Other.lua");
        fs::write(&other, b"x").expect("a file");
        let unknown = vec![(entry_name(&wtf, &other), other.clone())];
        let error = remove_checked(&zip, &unknown).expect_err("not in the zip");
        assert!(
            error
                .to_string()
                .contains(&format!("checking {} against the zip: ", other.display())),
            "{error}"
        );
        assert!(other.exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_file_that_cannot_be_removed_stops_the_cleanup_after_the_ones_before_it() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = Temp::new("remove-locked");
        let wtf = temp.path().join("WTF");
        let dir = saved_variables(&wtf, "ACC", None);
        let first = dir.join("First.lua");
        let held = dir.join("Held.lua");
        fs::write(&first, b"x").expect("a file");
        fs::write(&held, b"y").expect("a file");
        let entries = vec![
            (entry_name(&wtf, &first), first.clone()),
            (entry_name(&wtf, &held), held.clone()),
        ];
        let (zip, _) = backup::zip_files(&temp.path().join("backups"), CLEANUP_PREFIX, &entries)
            .expect("a zip");
        // A handle that shares reads and not the removal: the check reads the file, the removal fails.
        const FILE_SHARE_READ: u32 = 0x1;
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&held)
            .expect("a handle that shares reads");
        let error = remove_checked(&zip, &entries).expect_err("held");
        drop(lock);
        let Error::CleanupIncomplete {
            removed, reason, ..
        } = &error
        else {
            panic!("{error}");
        };
        assert_eq!(*removed, 1);
        assert!(
            reason.starts_with(&format!("removing {}: ", held.display())),
            "{reason}"
        );
        assert!(!first.exists() && held.exists());
    }

    /// A reader that is interrupted once, then hands over its bytes, then fails.
    struct Flaky {
        bytes: Vec<u8>,
        interrupted: bool,
        handed: usize,
    }

    impl Read for Flaky {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            if self.handed == self.bytes.len() {
                return Err(io::Error::other("the reader broke"));
            }
            let count = buffer.len().min(self.bytes.len() - self.handed);
            buffer[..count].copy_from_slice(&self.bytes[self.handed..self.handed + count]);
            self.handed += count;
            Ok(count)
        }
    }

    #[test]
    fn a_read_that_is_interrupted_goes_on_and_one_that_fails_stops() {
        let mut flaky = Flaky {
            bytes: b"abc".to_vec(),
            interrupted: false,
            handed: 0,
        };
        let mut buffer = [0u8; 2];
        assert_eq!(read_full(&mut flaky, &mut buffer).expect("read"), 2);
        assert_eq!(&buffer, b"ab");
        let error = read_full(&mut flaky, &mut buffer).expect_err("broke after the last byte");
        assert_eq!(error.to_string(), "the reader broke");
    }
}
