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
//! interface, which has no folder there, and is never an orphan. With no `Interface\AddOns` folder at all,
//! nothing is marked an orphan and a warning says so: a missing folder says more about the path than about
//! the addons.
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

/// The folder of the account folders, under `WTF`.
pub const ACCOUNT_DIR: &str = "Account";
/// The folder of the SavedVariables files, under an account or a character folder.
pub const SAVED_VARIABLES_DIR: &str = "SavedVariables";
/// A cleanup zip's name: `SavedVariables-<date>.zip`, next to the `WTF` backups.
pub const CLEANUP_PREFIX: &str = "SavedVariables-";
/// The name prefix of the client's own SavedVariables (its built-in interface), never an orphan.
pub const CLIENT_PREFIX: &str = "Blizzard_";

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

    /// Where it is, for the plain lines: the account, or `account / realm / character`.
    pub fn place(&self) -> String {
        match (&self.realm, &self.character) {
            (Some(realm), Some(character)) => {
                format!("{} / {realm} / {character}", self.account)
            }
            _ => self.account.clone(),
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
    for (account, account_dir) in subfolders(&wtf.join(ACCOUNT_DIR), &mut warnings)? {
        let place = Place {
            account: &account,
            realm: None,
            character: None,
        };
        collect(
            &account_dir.join(SAVED_VARIABLES_DIR),
            &place,
            installed,
            &mut files,
            &mut warnings,
        )?;
        for (realm, realm_dir) in subfolders(&account_dir, &mut warnings)? {
            if realm.eq_ignore_ascii_case(SAVED_VARIABLES_DIR) {
                continue;
            }
            for (character, character_dir) in subfolders(&realm_dir, &mut warnings)? {
                let place = Place {
                    account: &account,
                    realm: Some(&realm),
                    character: Some(&character),
                };
                collect(
                    &character_dir.join(SAVED_VARIABLES_DIR),
                    &place,
                    installed,
                    &mut files,
                    &mut warnings,
                )?;
            }
        }
    }
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

fn io_error(context: &'static str, path: &Path, source: io::Error) -> Error {
    Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    }
}

/// The subfolders of a folder by name, with their paths; a missing folder has none, and a link is a
/// warning and not followed.
fn subfolders(dir: &Path, warnings: &mut Vec<String>) -> Result<Vec<(String, PathBuf)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error("reading WTF", dir, source)),
    };
    let mut folders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error("reading WTF", dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| io_error("reading WTF", &path, source))?;
        if file_type.is_symlink() {
            warnings.push(link_warning(&path));
        } else if file_type.is_dir() {
            folders.push((entry.file_name().to_string_lossy().into_owned(), path));
        }
    }
    folders.sort();
    Ok(folders)
}

fn link_warning(path: &Path) -> String {
    format!(
        "not read: {} is a link, not a folder or a file",
        path.display()
    )
}

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
        Err(source) => return Err(io_error("reading WTF", dir, source)),
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
            .map_err(|source| io_error("reading WTF", &path, source))?;
        if file_type.is_symlink() {
            warnings.push(link_warning(&path));
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let bytes = entry
            .metadata()
            .map_err(|source| io_error("reading WTF", &path, source))?
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
        let status = if key.starts_with(&CLIENT_PREFIX.to_lowercase()) {
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

    #[test]
    fn a_place_reads_as_the_account_or_the_character_path() {
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
        assert_eq!(file.place(), "ACC");
        file.realm = Some("Some Realm".into());
        file.character = Some("Char".into());
        assert_eq!(file.place(), "ACC / Some Realm / Char");
        let json = serde_json::to_value(&file).expect("serializes");
        assert_eq!(json["status"], "orphan");
        assert_eq!(json["realm"], "Some Realm");
    }
}
