//! The install engine (task 004): a zip is unpacked into a staging folder under the flavor's mrgl home
//! (every entry checked so nothing lands outside it), its addon folders are found (several per zip, as the
//! BigWigs packager writes them, or one addon whose TOC sits at the zip's root), and they are swapped into
//! `Interface\AddOns` one folder at a time with a revert when a swap fails, so the folder is either the old
//! version or the new one. The state sidecar ([`crate::state`]) records what was installed from where, and
//! the cache keeps the zip of the current and the previous version for a later rollback. Only
//! `Interface\AddOns` is written in the game folder.
//!
//! The entry takes a zip path and the install metadata (source, version): a source such as GitHub hands
//! over a downloaded file and nothing else, and a zip in hand is the same call with its path as the source.
//!
//! A running game client is a warning, never a refusal (task 038): the commit and the removal ask the
//! running-client check of [`crate::client`] once, before the first folder moves, and a client running from
//! the flavor folder (or a process list that cannot be taken) becomes [`Installed::client_warning`] or
//! [`Removed::client_warning`]. The client reads the addon folders again at `/reload` or the next login, so
//! the write goes on; only a `WTF` write is refused while it runs. The engine asks the system's process list
//! unless [`Engine::with_processes`] hands it another (the tests' stand-in client).

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::client::{self, ProcessList, RunningClient, SystemProcesses};
use crate::error::{Error, Result};
use crate::flavor::{Flavor, FlavorId};
use crate::scan;
use crate::state::{self, Entry, FlavorHome, LoadedState, State};
use crate::toc::{self, TocName};
use crate::zip_entry::{self, ZipRole};

/// What is known about a zip before it is installed; the engine fills what is missing from the zip.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// The source spec, as the sidecar records it: `github:<owner>/<repo>`, later `wago:…`, `wowi:…`,
    /// `curse:…`, or the path of a zip in hand.
    pub source: String,
    /// The version the source names; `None` takes the primary folder's `## Version`, else `unknown`.
    pub version: Option<String>,
    /// The addon's name, the sidecar's key; `None` derives it from the folders (the one whose name the
    /// others extend, `BigWigs` for `BigWigs_Core` and `BigWigs_Options`, else the first by name).
    pub name: Option<String>,
}

/// One addon folder found in a staged zip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedAddon {
    /// The folder name under `Interface\AddOns`.
    pub folder: String,
    /// Where the folder waits in the staging folder.
    pub path: PathBuf,
    /// `## Version` of its TOC, when it has one.
    pub version: Option<String>,
    /// Every file in the folder, relative with `/` separators, and its SHA-256 as hex.
    pub files: BTreeMap<String, String>,
}

/// A zip unpacked and checked, waiting for the swap. Dropping it removes the staging folder, except after a
/// failed revert, when the folder holds the previous version and stays.
#[derive(Debug)]
pub struct Staged {
    /// The zip it came from.
    pub zip: PathBuf,
    /// This install's folder under `staging\`.
    pub dir: PathBuf,
    /// The addon folders, the primary one first.
    pub addons: Vec<StagedAddon>,
    /// The addon's name, the sidecar's key.
    pub name: String,
    /// The version to record.
    pub version: String,
    /// The source to record.
    pub source: String,
    keep: bool,
}

impl Staged {
    /// The folder names, the primary one first.
    pub fn folders(&self) -> Vec<String> {
        self.addons
            .iter()
            .map(|addon| addon.folder.clone())
            .collect()
    }

    /// Removes the staging folder without installing (a dry run).
    pub fn discard(self) {}
}

impl Drop for Staged {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// What an install did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: String,
    /// The version that was installed before, when the addon was managed already.
    pub previous_version: Option<String>,
    /// The folders written, the primary one first.
    pub folders: Vec<String>,
    /// The folders of the previous version that this one no longer ships, removed.
    pub removed_folders: Vec<String>,
    /// Plain lines about what went on around the install: a rebuilt sidecar, a cache that could not be
    /// written, a folder taken over from another managed addon.
    pub warnings: Vec<String>,
    /// The plain warning line when the client ran from the flavor folder during the install (it loads the
    /// new folders at `/reload`), or when the process list could not be taken; `None` otherwise.
    pub client_warning: Option<String>,
}

/// What a removal did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    pub name: String,
    pub version: String,
    pub folders: Vec<String>,
    pub warnings: Vec<String>,
    /// The plain warning line when the client ran from the flavor folder during the removal (the removed
    /// addons stay loaded until `/reload`), or when the process list could not be taken; `None` otherwise.
    pub client_warning: Option<String>,
}

/// The process list the engine asks for a running client: the system's by default, a stub in tests.
#[derive(Clone)]
struct Processes(Arc<dyn ProcessList + Send + Sync>);

impl std::fmt::Debug for Processes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Processes")
    }
}

impl Default for Processes {
    fn default() -> Self {
        Processes(Arc::new(SystemProcesses))
    }
}

/// The engine for one flavor: its game folder and its folder under the mrgl home.
#[derive(Debug, Clone)]
pub struct Engine {
    pub flavor: Flavor,
    pub home: FlavorHome,
    processes: Processes,
}

impl Engine {
    /// The engine of a flavor, with the mrgl home of this process (`MRGL_HOME`, else `LOCALAPPDATA\mrgl`).
    pub fn new(flavor: Flavor) -> Result<Engine> {
        let home = FlavorHome::new(&flavor.id)?;
        Ok(Engine {
            flavor,
            home,
            processes: Processes::default(),
        })
    }

    /// The engine of a flavor with an explicit mrgl home folder (the tests' temporary one).
    pub fn with_home(flavor: Flavor, home: &Path) -> Engine {
        let home = FlavorHome::at(home, &flavor.id);
        Engine {
            flavor,
            home,
            processes: Processes::default(),
        }
    }

    /// The same engine asking another process list for a running client (task 038): a
    /// [`client::FixedProcesses`] stub in tests, so a stand-in client needs no real process.
    pub fn with_processes(mut self, processes: impl ProcessList + Send + Sync + 'static) -> Engine {
        self.processes = Processes(Arc::new(processes));
        self
    }

    /// The running-client check before a write under `Interface\AddOns`, once per call: the line `line`
    /// writes for a client running from the flavor folder, the process-list error as a line when the list
    /// cannot be taken (the write goes on either way), `None` when no client runs.
    fn client_warning(&self, line: impl Fn(&RunningClient, &Flavor) -> String) -> Option<String> {
        match client::client_running_among(self.processes.0.as_ref(), &self.flavor.folder) {
            Ok(Some(running)) => Some(line(&running, &self.flavor)),
            Ok(None) => None,
            Err(error) => Some(error.to_string()),
        }
    }

    /// The flavor's `Interface\AddOns`.
    pub fn addons_dir(&self) -> PathBuf {
        scan::addons_dir(&self.flavor.folder)
    }

    /// The sidecar as it is on disk (a missing or corrupt one is fresh, with a warning for the corrupt).
    pub fn load_state(&self) -> Result<LoadedState> {
        self.home.read_state(&self.flavor.id)
    }

    /// Unpacks a zip into a fresh folder under `staging\`, refusing any entry that would land outside it,
    /// finds the addon folders and settles the name and the version. Nothing in the game folder changes.
    pub fn stage(&self, zip: &Path, metadata: &Metadata) -> Result<Staged> {
        let stem = zip
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "zip".to_string());
        let dir = fresh_staging_dir(&self.home.staging_dir(), &stem)?;
        let mut staged = Staged {
            zip: zip.to_path_buf(),
            dir,
            addons: Vec::new(),
            name: String::new(),
            version: String::new(),
            source: metadata.source.clone(),
            keep: false,
        };
        let unpacked = staged.dir.join("unpacked");
        let files = unpack(zip, &unpacked)?;
        let mut addons = find_addons(&unpacked, &stem, &files)?;
        let name = match &metadata.name {
            Some(name) => name.clone(),
            None => derive_name(
                &addons
                    .iter()
                    .map(|addon| addon.folder.as_str())
                    .collect::<Vec<_>>(),
            )
            .to_string(),
        };
        addons.sort_by_key(|addon| {
            (
                !addon.folder.eq_ignore_ascii_case(&name),
                addon.folder.to_ascii_lowercase(),
            )
        });
        let version = metadata
            .version
            .clone()
            .filter(|version| !version.trim().is_empty())
            .or_else(|| addons.first().and_then(|addon| addon.version.clone()))
            .unwrap_or_else(|| "unknown".to_string());
        staged.addons = addons;
        staged.name = name;
        staged.version = version;
        Ok(staged)
    }

    /// Swaps the staged folders into `Interface\AddOns` one at a time, removes the folders the previous
    /// version had and this one lacks, records the install in the state (and writes it) and keeps the zip
    /// in the cache. When a swap fails, every folder moved so far is put back and the error says so.
    pub fn commit(&self, mut staged: Staged, state: &mut State) -> Result<Installed> {
        let addons_dir = self.addons_dir();
        std::fs::create_dir_all(&addons_dir).map_err(|source| Error::Io {
            context: "creating the AddOns folder",
            path: addons_dir.clone(),
            source,
        })?;
        let previous = state
            .entry(&staged.name)
            .map(|(name, entry)| (name.to_string(), entry.clone()));
        let new_folders = staged.folders();
        let stale: Vec<String> = previous
            .iter()
            .flat_map(|(_, entry)| entry.folders.iter())
            .filter(|folder| !contains_folder(&new_folders, folder))
            .cloned()
            .collect();

        // The running-client check (task 023), once per commit and before the first folder moves: a running
        // client loads the new folders at `/reload`, so it is a warning, never a refusal (task 038).
        let client_warning = self.client_warning(RunningClient::install_warning);

        let previous_dir = staged.dir.join("previous");
        let mut journal: Vec<(PathBuf, PathBuf)> = Vec::new();
        let outcome = swap_all(&addons_dir, &staged, &stale, &previous_dir, &mut journal);
        if let Err((folder, reason)) = outcome {
            for (from, to) in journal.iter().rev() {
                if let Err(revert_error) = move_dir(to, from) {
                    staged.keep = true;
                    return Err(Error::SwapRevertFailed {
                        folder,
                        reason: format!("{reason}; moving {} back: {revert_error}", to.display()),
                        staging: staged.dir.clone(),
                    });
                }
            }
            return Err(Error::SwapFailed {
                folder,
                reason: reason.to_string(),
            });
        }

        let mut warnings = Vec::new();
        let mut files = BTreeMap::new();
        for addon in &staged.addons {
            for (path, hash) in &addon.files {
                files.insert(format!("{}/{path}", addon.folder), hash.clone());
            }
        }
        let previous_version = previous.as_ref().and_then(|(_, entry)| {
            if entry.version == staged.version {
                entry.previous_version.clone()
            } else {
                Some(entry.version.clone())
            }
        });
        let entry = Entry {
            source: staged.source.clone(),
            version: staged.version.clone(),
            previous_version: previous_version.clone(),
            folders: new_folders.clone(),
            files,
            installed_at: state::now_rfc3339(),
        };
        if let Some((old_name, _)) = &previous {
            state.addons.remove(old_name);
        }
        take_over_folders(state, &staged.name, &new_folders, &mut warnings);
        state.addons.insert(staged.name.clone(), entry);
        self.home.write_state(state)?;
        self.cache_zip(&staged, previous_version.as_deref(), &mut warnings);
        Ok(Installed {
            name: staged.name.clone(),
            version: staged.version.clone(),
            previous_version,
            folders: new_folders,
            removed_folders: stale,
            warnings,
            client_warning,
        })
    }

    /// Installs a zip: reads the sidecar (dropping the entries whose folders were deleted by hand), stages
    /// the zip, swaps the folders in and records the install.
    pub fn install(&self, zip: &Path, metadata: &Metadata) -> Result<Installed> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let scan = scan::scan_flavor(&self.flavor)?;
        let dropped = state.reconcile(&scan);
        let staged = self.stage(zip, metadata)?;
        let mut installed = self.commit(staged, &mut state)?;
        let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
        warnings.extend(dropped.into_iter().map(|name| {
            format!("{name} was deleted by hand since its install and is no longer managed")
        }));
        warnings.append(&mut installed.warnings);
        installed.warnings = warnings;
        Ok(installed)
    }

    /// Removes a managed addon: its folders under `Interface\AddOns`, its entry and its cache. A name the
    /// sidecar does not know is an error that says whether the folder exists.
    pub fn remove(&self, name: &str) -> Result<Removed> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
        let Some((known, entry)) = state
            .entry(name)
            .map(|(known, entry)| (known.to_string(), entry.clone()))
        else {
            let flavor = flavor_label(&self.flavor.id);
            let name = name.to_string();
            return Err(if self.addons_dir().join(&name).is_dir() {
                Error::NotManaged { name, flavor }
            } else {
                Error::AddonNotFound { name, flavor }
            });
        };
        let client_warning = self.client_warning(RunningClient::remove_warning);
        let addons_dir = self.addons_dir();
        for folder in &entry.folders {
            let path = addons_dir.join(folder);
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {}
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(Error::Io {
                        context: "removing the addon folder",
                        path,
                        source,
                    });
                }
            }
        }
        state.addons.remove(&known);
        self.home.write_state(&state)?;
        let cache = self.home.addon_cache_dir(&known);
        if let Err(error) = std::fs::remove_dir_all(&cache)
            && error.kind() != io::ErrorKind::NotFound
        {
            warnings.push(format!(
                "the cache of {known} could not be removed: {}: {error}",
                cache.display()
            ));
        }
        Ok(Removed {
            name: known,
            version: entry.version,
            folders: entry.folders,
            warnings,
            client_warning,
        })
    }

    /// Copies the zip into `cache\<addon>\<version>.zip` and prunes the folder to this version and the
    /// previous one. Best effort: a failure is a warning, the install stands.
    fn cache_zip(
        &self,
        staged: &Staged,
        previous_version: Option<&str>,
        warnings: &mut Vec<String>,
    ) {
        let dir = self.home.addon_cache_dir(&staged.name);
        let target = self.home.cached_zip(&staged.name, &staged.version);
        let same_file = match (
            std::fs::canonicalize(&staged.zip),
            std::fs::canonicalize(&target),
        ) {
            (Ok(source), Ok(existing)) => source == existing,
            _ => false,
        };
        let copied = if same_file {
            Ok(())
        } else {
            std::fs::create_dir_all(&dir)
                .and_then(|()| std::fs::copy(&staged.zip, &target).map(|_| ()))
        };
        if let Err(error) = copied {
            warnings.push(format!(
                "the zip could not be kept for a later rollback: {}: {error}",
                target.display()
            ));
            return;
        }
        let keep: Vec<PathBuf> = [Some(staged.version.as_str()), previous_version]
            .into_iter()
            .flatten()
            .map(|version| self.home.cached_zip(&staged.name, version))
            .collect();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !keep.iter().any(|kept| kept == &path) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// The swap itself: every new folder in, every stale folder out, journaled as `(from, to)` moves so a
/// failure can be reverted in reverse order. The error names the folder and the I/O reason.
fn swap_all(
    addons_dir: &Path,
    staged: &Staged,
    stale: &[String],
    previous_dir: &Path,
    journal: &mut Vec<(PathBuf, PathBuf)>,
) -> std::result::Result<(), (String, io::Error)> {
    let move_away = |folder: &str, journal: &mut Vec<(PathBuf, PathBuf)>| {
        let target = addons_dir.join(folder);
        if std::fs::symlink_metadata(&target).is_ok() {
            let away = previous_dir.join(folder);
            move_dir(&target, &away).map_err(|error| (folder.to_string(), error))?;
            journal.push((target, away));
        }
        Ok(())
    };
    for addon in &staged.addons {
        move_away(&addon.folder, journal)?;
        let target = addons_dir.join(&addon.folder);
        move_dir(&addon.path, &target).map_err(|error| (addon.folder.clone(), error))?;
        journal.push((addon.path.clone(), target));
    }
    for folder in stale {
        move_away(folder, journal)?;
    }
    Ok(())
}

/// Folders of other managed addons that the new install now owns leave those entries; an entry left with
/// no folder is dropped. Both are warnings.
fn take_over_folders(
    state: &mut State,
    name: &str,
    folders: &[String],
    warnings: &mut Vec<String>,
) {
    let mut emptied = Vec::new();
    for (other, entry) in state.addons.iter_mut() {
        if other.eq_ignore_ascii_case(name) {
            continue;
        }
        let before = entry.folders.len();
        entry
            .folders
            .retain(|folder| !contains_folder(folders, folder));
        if entry.folders.len() != before {
            warnings.push(format!(
                "{other} lost {} to {name}",
                plural(before - entry.folders.len(), "folder")
            ));
        }
        if entry.folders.is_empty() {
            emptied.push(other.clone());
        }
    }
    for other in emptied {
        state.addons.remove(&other);
        warnings.push(format!(
            "{other} is no longer managed: {name} replaced every folder of it"
        ));
    }
}

fn contains_folder(folders: &[String], folder: &str) -> bool {
    folders
        .iter()
        .any(|known| known.eq_ignore_ascii_case(folder))
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// The flavor as a message names it: the friendly name, else the id.
fn flavor_label(id: &FlavorId) -> String {
    id.friendly_name().unwrap_or(id.as_str()).to_string()
}

/// A fresh folder under `staging\`, named after the zip and the clock, never one that exists.
fn fresh_staging_dir(staging: &Path, stem: &str) -> Result<PathBuf> {
    let io = |path: &Path, source| Error::Io {
        context: "creating the staging folder",
        path: path.to_path_buf(),
        source,
    };
    std::fs::create_dir_all(staging).map_err(|source| io(staging, source))?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let stem = state::file_safe(stem);
    for attempt in 0u32.. {
        let dir = staging.join(format!("{stem}-{nanos}-{attempt}"));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(io(&dir, source)),
        }
    }
    unreachable!("the attempts never run out")
}

/// Unpacks a zip under a folder once [`zip_entry::checked_entries`] has passed every entry. Returns every
/// file as its relative path with `/` separators and its SHA-256.
fn unpack(zip: &Path, into: &Path) -> Result<BTreeMap<String, String>> {
    let unreadable = |reason: String| Error::ZipUnreadable {
        role: ZipRole::Addon,
        path: zip.to_path_buf(),
        reason,
    };
    let file = std::fs::File::open(zip).map_err(|source| unreadable(source.to_string()))?;
    let mut archive = zip::ZipArchive::new(io::BufReader::new(file))
        .map_err(|reason| unreadable(reason.to_string()))?;
    let entries = zip_entry::checked_entries(&mut archive, zip, ZipRole::Addon)?;
    let io = |context: &'static str, path: &Path, source| Error::Io {
        context,
        path: path.to_path_buf(),
        source,
    };
    std::fs::create_dir_all(into)
        .map_err(|source| io("creating the staging folder", into, source))?;
    let mut files = BTreeMap::new();
    for checked in entries {
        let relative = checked.parts.join("/");
        let mut path = into.to_path_buf();
        path.extend(&checked.parts);
        if checked.is_dir {
            std::fs::create_dir_all(&path)
                .map_err(|source| io("creating a staged folder", &path, source))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|source| io("creating a staged folder", parent, source))?;
        }
        let mut entry = archive
            .by_index(checked.index)
            .map_err(|reason| unreadable(format!("entry {}: {reason}", checked.index)))?;
        let output = std::fs::File::create(&path)
            .map_err(|source| io("writing a staged file", &path, source))?;
        let hash = copy_hashing(&mut entry, io::BufWriter::new(output))
            .map_err(|source| io("writing a staged file", &path, source))?;
        files.insert(relative, hash);
    }
    Ok(files)
}

/// Copies a reader into a writer and returns the SHA-256 of what went through, as hex.
fn copy_hashing(reader: &mut impl Read, mut writer: impl Write) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
    }
    writer.flush()?;
    Ok(hex(&hasher.finalize()))
}

/// The SHA-256 of a file on disk, as hex.
pub fn sha256_of(path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    copy_hashing(&mut file, io::sink())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The addon folders in an unpacked zip: a `.toc` at the root makes the root one addon, named after its
/// TOC; otherwise every subfolder with a TOC of its own name is one; a zip whose single folder wraps the
/// addon folders (a source archive) is looked into one level down.
fn find_addons(
    unpacked: &Path,
    zip_stem: &str,
    files: &BTreeMap<String, String>,
) -> Result<Vec<StagedAddon>> {
    let mut dir = unpacked.to_path_buf();
    let mut prefix = String::new();
    for _level in 0..2 {
        let (tocs, folders) = list_dir(&dir)?;
        if !tocs.is_empty() {
            let name = root_toc_name(&tocs, zip_stem);
            return Ok(vec![staged_addon(name, &dir, &prefix, files)]);
        }
        let addons: Vec<StagedAddon> = folders
            .iter()
            .filter(|folder| has_own_toc(&dir.join(folder), folder))
            .map(|folder| {
                staged_addon(
                    folder.clone(),
                    &dir.join(folder),
                    &format!("{prefix}{folder}/"),
                    files,
                )
            })
            .collect();
        if !addons.is_empty() {
            return Ok(addons);
        }
        if let [wrapper] = folders.as_slice() {
            prefix.push_str(wrapper);
            prefix.push('/');
            dir = dir.join(wrapper);
            continue;
        }
        break;
    }
    Err(Error::NoAddonInZip {
        path: unpacked.to_path_buf(),
    })
}

/// The `.toc` file names and the subfolder names directly in a folder, sorted.
fn list_dir(dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let io = |source| Error::Io {
        context: "reading the staging folder",
        path: dir.to_path_buf(),
        source,
    };
    let mut tocs = Vec::new();
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry.file_type().map_err(io)?;
        if file_type.is_dir() {
            folders.push(name);
        } else if name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(".toc") {
            tocs.push(name);
        }
    }
    tocs.sort_by_key(|name| name.to_ascii_lowercase());
    folders.sort_by_key(|name| name.to_ascii_lowercase());
    Ok((tocs, folders))
}

/// Whether a folder holds a TOC of its own name: `<Folder>.toc` or `<Folder>_<Suffix>.toc`.
fn has_own_toc(dir: &Path, folder: &str) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            matches!(
                toc::classify_toc_name(folder, &entry.file_name().to_string_lossy()),
                Some(TocName::Base | TocName::Suffix(_) | TocName::Legacy(_))
            )
        })
    })
}

/// The addon name of TOCs at a zip's root: the stem every other TOC extends with a suffix (`Foo` for
/// `Foo.toc` and `Foo_Vanilla.toc`), else the zip's own stem.
fn root_toc_name(tocs: &[String], zip_stem: &str) -> String {
    let stems: Vec<&str> = tocs.iter().map(|name| &name[..name.len() - 4]).collect();
    let mut candidates: Vec<&str> = stems
        .iter()
        .copied()
        .filter(|stem| {
            tocs.iter().all(|other| {
                matches!(
                    toc::classify_toc_name(stem, other),
                    Some(TocName::Base | TocName::Suffix(_) | TocName::Legacy(_))
                )
            })
        })
        .collect();
    candidates.sort_by_key(|stem| stem.len());
    candidates
        .first()
        .map(|stem| stem.to_string())
        .unwrap_or_else(|| zip_stem.to_string())
}

/// A staged addon from its folder: the TOC version and the files under its prefix in the unpack list.
fn staged_addon(
    folder: String,
    path: &Path,
    prefix: &str,
    files: &BTreeMap<String, String>,
) -> StagedAddon {
    let files: BTreeMap<String, String> = files
        .iter()
        .filter_map(|(relative, hash)| {
            relative
                .strip_prefix(prefix)
                .map(|rest| (rest.to_string(), hash.clone()))
        })
        .collect();
    StagedAddon {
        version: toc_version(path, &folder),
        folder,
        path: path.to_path_buf(),
        files,
    }
}

/// `## Version` of a folder's TOC: the base `<Folder>.toc` first, else the first suffix one by name.
fn toc_version(dir: &Path, folder: &str) -> Option<String> {
    let mut names: Vec<(bool, String)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            match toc::classify_toc_name(folder, &name) {
                Some(TocName::Base) => Some((false, name)),
                Some(TocName::Suffix(_) | TocName::Legacy(_)) => Some((true, name)),
                _ => None,
            }
        })
        .collect();
    names.sort_by_key(|(suffixed, name)| (*suffixed, name.to_ascii_lowercase()));
    names.into_iter().find_map(|(_, name)| {
        let bytes = std::fs::read(dir.join(name)).ok()?;
        toc::parse_toc(&String::from_utf8_lossy(&bytes)).version
    })
}

/// The addon name of a set of folders: the folder whose name every other one extends with `_` or `-`
/// (`BigWigs` for `BigWigs_Core`), else the first by name.
pub fn derive_name<'a>(folders: &[&'a str]) -> &'a str {
    let mut sorted: Vec<&str> = folders.to_vec();
    sorted.sort_by_key(|folder| (folder.len(), folder.to_ascii_lowercase()));
    let extends = |base: &str, other: &str| {
        other.len() > base.len()
            && other[..base.len()].eq_ignore_ascii_case(base)
            && matches!(other.as_bytes()[base.len()], b'_' | b'-')
    };
    sorted
        .iter()
        .copied()
        .find(|base| {
            sorted
                .iter()
                .all(|other| other.eq_ignore_ascii_case(base) || extends(base, other))
        })
        .unwrap_or_else(|| {
            sorted.sort_by_key(|folder| folder.to_ascii_lowercase());
            sorted.first().copied().unwrap_or("")
        })
}

/// Moves a folder: a rename on the same volume, a copy and a delete across volumes (the mrgl home on one
/// drive, the game on another). A failed copy removes what it had copied before it reports.
fn move_dir(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if crosses_devices(&error) => {
            if let Err(copy_error) = copy_dir_all(from, to) {
                let _ = std::fs::remove_dir_all(to);
                return Err(copy_error);
            }
            std::fs::remove_dir_all(from)
        }
        Err(error) => Err(error),
    }
}

fn crosses_devices(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::CrossesDevices
        || (cfg!(windows) && error.raw_os_error() == Some(17))
}

fn copy_dir_all(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first rule difference of [`crate::zip_entry`]: the escape fixture's TOC comes before the
    /// escaping entry, and the engine used to write it into staging before the refusal. Now nothing is.
    #[test]
    fn an_escaping_zip_unpacks_nothing() {
        let zip =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zips/escaping-path.zip");
        let into = std::env::temp_dir().join(format!("mrgl-unpack-escape-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&into);
        let error = unpack(&zip, &into).expect_err("refused");
        assert!(
            matches!(
                &error,
                Error::ZipEntryEscapes {
                    role: ZipRole::Addon,
                    ..
                }
            ),
            "{error}"
        );
        assert!(!into.exists(), "nothing was unpacked");
    }

    #[test]
    fn the_name_is_the_folder_the_others_extend_else_the_first() {
        assert_eq!(
            derive_name(&["BigWigs_Core", "BigWigs", "BigWigs_Options"]),
            "BigWigs"
        );
        assert_eq!(
            derive_name(&["DBM-GUI", "DBM-Core", "DBM-StatusBarTimers"]),
            "DBM-Core"
        );
        assert_eq!(derive_name(&["Zed", "Alpha"]), "Alpha");
        assert_eq!(derive_name(&["Solo"]), "Solo");
        assert_eq!(derive_name(&["foo_x", "Foo"]), "Foo");
        assert_eq!(derive_name(&["FooBar", "Foo"]), "Foo");
        assert_eq!(derive_name(&[]), "");
    }

    #[test]
    fn root_tocs_name_the_addon_by_their_common_stem() {
        let tocs = |names: &[&str]| {
            names
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            root_toc_name(
                &tocs(&["Foo.toc", "Foo_Vanilla.toc", "Foo_Mainline.toc"]),
                "foo-1.0"
            ),
            "Foo"
        );
        assert_eq!(
            root_toc_name(&tocs(&["Foo_Vanilla.toc"]), "x"),
            "Foo_Vanilla"
        );
        assert_eq!(
            root_toc_name(&tocs(&["Foo.toc", "Bar.toc"]), "foo-1.0"),
            "foo-1.0"
        );
    }

    #[test]
    fn hashes_are_sha256_hex() {
        let mut input: &[u8] = b"abc";
        let hash = copy_hashing(&mut input, io::sink()).expect("hashes");
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(plural(1, "folder"), "1 folder");
        assert_eq!(plural(3, "folder"), "3 folders");
        assert_eq!(flavor_label(&FlavorId::Forever), "forever");
        assert_eq!(
            flavor_label(&FlavorId::Other("wow_mrgl".into())),
            "wow_mrgl"
        );
    }
}
