//! The install engine (task 004): a zip is unpacked into a staging folder under the flavor's mrgl home
//! (every entry checked so nothing lands outside it), its addon folders are found (several per zip, as the
//! BigWigs packager writes them, or one addon whose TOC sits at the zip's root), and they are swapped into
//! `Interface\AddOns` one folder at a time with a revert when a swap fails, so the folder is either the old
//! version or the new one. The state sidecar ([`crate::state`]) records what was installed from where, and
//! the cache keeps the zip of the current and the previous version for the rollback ([`Engine::rollback`],
//! task 022: the previous version's cached zip through the same install call, then pinned). Only
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
//!
//! Every download and every unpacked zip gets a run folder of its own under `staging\` ([`RunDir`], task
//! 084), locked while the run holds it and removed on every path out; [`Engine::stage`] first sweeps the
//! leftovers of runs that ended before their guard went ([`staging::sweep`]), never a folder another
//! running mrgl holds.
//!
//! [`Engine::install_from_source`] is the one path of every source-backed install (task 055: `add
//! github:`, `import`, `update`): it resolves the request on the addon's own channel (task 021), finds an
//! addon the sidecar has from that source at that version already (and stops there), downloads the asset
//! into a fresh folder under `staging\` and sends it through the same stage and commit as a zip in hand
//! (task 040). [`Engine::install_pick`] is the same path for a pick in hand.
//!
//! The link guard (task 005, [`Engine::refuse_links`]) runs in [`Engine::commit`], before the first folder
//! moves, so every path that stages and commits (a zip, a source, a rollback) refuses to install over a
//! linked addon or a junction mrgl did not make, among the staged folders and the folders of the previous
//! version the new one drops (task 067).

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::client::{self, ProcessList, RunningClient, SystemProcesses};
use crate::dependencies::{self, AddonDependencies};
use crate::error::{Error, Result};
use crate::flavor::{Flavor, FlavorId};
use crate::interface::{self, InterfaceMismatch, InterfaceOutOfDate};
use crate::scan;
use crate::sources::{Channel, Pick, Source, SourceSpec, Target};
use crate::staging::{self, RunDir};
use crate::state::{self, Entry, FlavorHome, LoadedState, State};
use crate::toc::{self, TocName};
use crate::voice::Kind;
use crate::zip_entry::{self, ZipRole};

/// What is known about a zip before it is installed; the engine fills what is missing from the zip.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    /// The source spec, as the sidecar records it: `github:<owner>/<repo>`, later `wago:…`, `wowi:…`,
    /// `curse:…`, or the path of a zip in hand.
    pub source: String,
    /// The version the source names; `None` takes the primary folder's `## Version`, else `unknown`.
    pub version: Option<String>,
    /// The addon's name, the sidecar's key; `None` derives it from the folders ([`derive_name`]: the one
    /// whose name the others extend, `BigWigs` for `BigWigs_Core` and `BigWigs_Options`, else the one the
    /// zip's stem or the source's repository names, else the first by name).
    pub name: Option<String>,
    /// The pick's `nolib` flag ([`crate::sources::Pick::nolib`]), recorded on the sidecar entry for the
    /// dependency check (task 029); `None` when the source does not say (a zip in hand).
    pub nolib: Option<bool>,
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
/// failed revert, when the folder holds the previous version and stays (its lock released, the sweep of
/// task 084 never removes a folder with `previous\` in it).
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
    /// The `nolib` flag to record ([`Metadata::nolib`]).
    pub nolib: Option<bool>,
    /// The warnings of the sweep before this stage (task 084): a leftover folder under `staging\` that
    /// could not be removed. The commit carries them into [`Installed::warnings`].
    pub sweep_warnings: Vec<String>,
    keep: bool,
    run: RunDir,
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
        // The run folder goes with its guard, right after this; a failed revert's folder stays.
        if self.keep {
            self.run.keep();
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

/// What a pin, an unpin, an ignore, an unignore or a channel change did (task 021): the entry as it is
/// now, and whether it changed (the sidecar is written only then).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    /// The addon's name as the sidecar spells it.
    pub name: String,
    /// The installed version.
    pub version: String,
    pub pinned: Option<String>,
    pub ignored: bool,
    pub channel: Channel,
    /// Whether the entry changed; `false` means it was so already and nothing was written.
    pub changed: bool,
    pub warnings: Vec<String>,
}

/// One mark on a managed addon's sidecar entry (task 021), as the command line's `pin`, `unpin`, `ignore`,
/// `unignore` and `channel` and the window's row menu (task 010) ask for it: [`Engine::apply_mark`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark {
    /// Pin at a version, or at the installed one when `None`.
    Pin(Option<String>),
    Unpin,
    Ignore,
    Unignore,
    /// Follow a release channel.
    Channel(Channel),
}

impl Mark {
    /// The mark's name, as the JSON documents and the window's IPC call it.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Mark::Pin(_) => "pin",
            Mark::Unpin => "unpin",
            Mark::Ignore => "ignore",
            Mark::Unignore => "unignore",
            Mark::Channel(_) => "channel",
        }
    }

    /// The message of a mark: a success when it changed the entry, nothing to do when it was so already.
    pub fn line(&self, marked: &Marked, folder_name: &str) -> (Kind, String) {
        let name = &marked.name;
        let pinned = marked.pinned.as_deref().unwrap_or_default();
        let channel = marked.channel;
        let plain = match (self, marked.changed) {
            (Mark::Pin(_), true) if pinned != marked.version => format!(
                "pinned {name} at {pinned} in {folder_name} ({} is installed)",
                marked.version
            ),
            (Mark::Pin(_), true) => format!("pinned {name} at {pinned} in {folder_name}"),
            (Mark::Pin(_), false) => {
                format!("{name} is already pinned at {pinned} in {folder_name}")
            }
            (Mark::Unpin, true) => format!("unpinned {name} in {folder_name}"),
            (Mark::Unpin, false) => format!("{name} is not pinned in {folder_name}"),
            (Mark::Ignore, true) => format!("ignoring {name} in {folder_name}"),
            (Mark::Ignore, false) => format!("{name} is already ignored in {folder_name}"),
            (Mark::Unignore, true) => format!("{name} is no longer ignored in {folder_name}"),
            (Mark::Unignore, false) => format!("{name} is not ignored in {folder_name}"),
            (Mark::Channel(_), true) => {
                format!("{name} follows the {channel} channel in {folder_name}")
            }
            (Mark::Channel(_), false) => {
                format!("{name} is already on the {channel} channel in {folder_name}")
            }
        };
        let kind = if marked.changed {
            Kind::Success
        } else {
            Kind::Nothing
        };
        (kind, plain)
    }
}

/// A step of [`Engine::install_from_source`], for the caller's progress line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickStep<'a> {
    /// The source is about to be asked for the release (none for a pick in hand).
    Resolving {
        spec: &'a SourceSpec,
        target: &'a Target,
    },
    /// The source answered with this pick (none for a pick in hand).
    Picked { pick: &'a Pick },
    /// The asset is about to be downloaded into this file under the staging folder.
    Downloading { pick: &'a Pick, to: &'a Path },
    /// The asset is downloaded; the engine's install call starts.
    Installing { pick: &'a Pick, zip: &'a Path },
}

/// Which release of a source an install takes (task 055).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wanted {
    /// The current release on the addon's channel: the channel of its sidecar entry, `stable` for an
    /// addon the sidecar does not have.
    Current,
    /// A pick made already (by the caller, or by the update check), installed as it is: no second request.
    Pick(Box<Pick>),
    /// The release that carries this version, on any channel (task 057, `import --exact`):
    /// [`Source::resolve_version`], [`crate::Error::VersionNotOffered`] when the source has none.
    Version(String),
}

/// An install from a source, as the one path takes it (task 055).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRequest {
    /// The source.
    pub spec: SourceSpec,
    /// Which of its releases.
    pub wanted: Wanted,
    /// The name the sidecar records the addon by when it has no entry from this source (the list file's
    /// name on `import`); `None` derives it from the folders. An entry from the source keeps its name.
    pub name: Option<String>,
}

impl SourceRequest {
    /// The current release of a source, under the name the sidecar knows it by or the folders give it.
    pub fn current(spec: SourceSpec) -> SourceRequest {
        SourceRequest {
            spec,
            wanted: Wanted::Current,
            name: None,
        }
    }
}

/// What [`Engine::install_from_source`] did: the pick it took and what came of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInstalled {
    pub pick: Pick,
    pub outcome: PickInstalled,
}

/// What an install from a source's pick did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickInstalled {
    /// The addon was installed, or updated from the version the sidecar had (the engine's normal path).
    Installed(Installed),
    /// The sidecar has the addon from this source at the pick's version and its folders are there: nothing
    /// was downloaded and nothing moved.
    AlreadyInstalled(AlreadyInstalled),
}

/// A managed addon the sidecar has from a source, at the version a pick names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyInstalled {
    /// The sidecar's key.
    pub name: String,
    /// The sidecar's entry as it stands.
    pub entry: Entry,
    /// Plain lines about the sidecar (a corrupt one read as fresh).
    pub warnings: Vec<String>,
}

/// What a rollback did (task 022): the version installed now is the previous one, pinned, and the version
/// rolled back from is the previous one in its turn, so a second rollback goes forward again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolledBack {
    /// The addon's name as the sidecar spells it.
    pub name: String,
    /// The version installed now, the one rolled back to.
    pub version: String,
    /// The version rolled back from, now the entry's previous version.
    pub previous_version: String,
    /// The pin the rollback set: the version rolled back to.
    pub pinned: String,
    /// The entry's source, unchanged by the rollback.
    pub source: String,
    /// The folders written, the primary one first.
    pub folders: Vec<String>,
    /// The folders of the version rolled back from that the version rolled back to does not ship, removed.
    pub removed_folders: Vec<String>,
    pub warnings: Vec<String>,
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

/// What [`Engine::check_installed`] found about the folders of an install.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledChecks {
    /// The dependency check of each installed folder (task 029).
    pub dependencies: Vec<AddonDependencies>,
    /// The primary folder names no interface of the flavor's build (task 015).
    pub mismatch: Option<InterfaceMismatch>,
    /// The primary folder names only interface numbers of the build's major below the build's own: the
    /// client's `Out of date` (tasks 076, 082). Never set together with `mismatch`.
    pub out_of_date: Option<InterfaceOutOfDate>,
    /// Why there is no check: the flavor's folder could not be scanned.
    pub warnings: Vec<String>,
}

impl InstalledChecks {
    /// The warning lines, in order: a scan that failed, each missing dependency, then the interface check.
    pub fn lines(&self, folder_name: &str) -> Vec<String> {
        let mut lines = self.warnings.clone();
        lines.extend(
            self.dependencies
                .iter()
                .filter_map(AddonDependencies::warning),
        );
        lines.extend(self.interface_lines(folder_name));
        lines
    }

    /// The interface check's warning after an install (tasks 015, 076, 082): the mismatch or the out-of-date
    /// line, at most one, since an addon is never both.
    pub fn interface_lines(&self, folder_name: &str) -> Vec<String> {
        self.mismatch
            .iter()
            .map(|mismatch| mismatch.after_install_line(folder_name))
            .chain(
                self.out_of_date
                    .iter()
                    .map(|out_of_date| out_of_date.after_install_line(folder_name)),
            )
            .collect()
    }
}

/// The engine for one flavor: its game folder and its folder under the mrgl home.
#[derive(Debug, Clone)]
pub struct Engine {
    pub flavor: Flavor,
    pub home: FlavorHome,
    processes: Processes,
    max_unpacked: u64,
}

impl Engine {
    /// The engine of a flavor, with the mrgl home of this process (`MRGL_HOME`, else `LOCALAPPDATA\mrgl`).
    pub fn new(flavor: Flavor) -> Result<Engine> {
        let home = FlavorHome::new(&flavor.id)?;
        Ok(Engine {
            flavor,
            home,
            processes: Processes::default(),
            max_unpacked: zip_entry::MAX_UNPACKED,
        })
    }

    /// The engine of a flavor with an explicit mrgl home folder (the tests' temporary one).
    pub fn with_home(flavor: Flavor, home: &Path) -> Engine {
        let home = FlavorHome::at(home, &flavor.id);
        Engine {
            flavor,
            home,
            processes: Processes::default(),
            max_unpacked: zip_entry::MAX_UNPACKED,
        }
    }

    /// The same engine with another bound on what a zip unpacks to (task 039; `--max-unpacked`): every
    /// stage of this engine, the rollback's included, refuses a zip past it ([`Error::ZipTooLarge`]).
    pub fn with_max_unpacked(mut self, bytes: u64) -> Engine {
        self.max_unpacked = bytes;
        self
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

    /// The checks an install reports about the folders it just installed, the command line's summary and
    /// the window's update alike (tasks 015, 029, 079, 082): the dependency check of those folders against
    /// the flavor as it is now, and the interface check of the first one, the primary folder (a mismatch
    /// or out of date). One scan; a scan
    /// that fails is a warning and no check.
    pub fn check_installed(&self, folders: &[String]) -> InstalledChecks {
        let scan = match scan::scan_flavor(&self.flavor) {
            Ok(scan) => scan,
            Err(error) => {
                return InstalledChecks {
                    warnings: vec![error.to_string()],
                    ..InstalledChecks::default()
                };
            }
        };
        let state = self.load_state().ok().map(|loaded| loaded.state);
        let installed = |folder: &str| folders.iter().any(|one| one.eq_ignore_ascii_case(folder));
        let dependencies = dependencies::check(&scan, state.as_ref())
            .into_iter()
            .filter(|check| installed(&check.folder))
            .collect();
        let primary = folders.first().and_then(|primary| {
            scan.addons
                .iter()
                .find(|addon| addon.folder.eq_ignore_ascii_case(primary))
        });
        let mismatch = primary.and_then(|addon| interface::check_addon(addon, self.flavor.version));
        let out_of_date =
            primary.and_then(|addon| interface::out_of_date_addon(addon, self.flavor.version));
        InstalledChecks {
            dependencies,
            mismatch,
            out_of_date,
            warnings: Vec::new(),
        }
    }

    /// Unpacks a zip into a fresh folder under `staging\`, refusing any entry that would land outside it,
    /// finds the addon folders and settles the name and the version. Nothing in the game folder changes.
    /// The leftovers of earlier runs under `staging\` are swept first (task 084, [`staging::sweep`]).
    pub fn stage(&self, zip: &Path, metadata: &Metadata) -> Result<Staged> {
        let stem = zip
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "zip".to_string());
        let sweep_warnings = staging::sweep(&self.home.staging_dir());
        let run = RunDir::create(&self.home.staging_dir(), &stem)?;
        let mut staged = Staged {
            zip: zip.to_path_buf(),
            dir: run.path().to_path_buf(),
            addons: Vec::new(),
            name: String::new(),
            version: String::new(),
            source: metadata.source.clone(),
            nolib: metadata.nolib,
            sweep_warnings,
            keep: false,
            run,
        };
        let unpacked = staged.dir.join("unpacked");
        let files = unpack(zip, &unpacked, self.max_unpacked)?;
        let mut addons = find_addons(&unpacked, &stem, &files)?;
        let name = match &metadata.name {
            Some(name) => name.clone(),
            None => derive_name(
                &addons
                    .iter()
                    .map(|addon| addon.folder.as_str())
                    .collect::<Vec<_>>(),
                &name_hints(&stem, &metadata.source),
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
    /// Before anything moves, a staged folder or a folder of the previous version this one drops that is a
    /// linked addon or a junction mrgl did not make is refused (tasks 005 and 067,
    /// [`Engine::refuse_links`]): here, so no path that stages and commits can skip it, and a dropped
    /// folder is never moved aside into the staging folder and deleted with it.
    /// The entry replaced is the one of the staged name, else the one that records the same addon under
    /// another name (task 077, [`renamed_entry`]): its pin, ignored mark, channel and previous version
    /// carry over to the new name, and its cached zips move to the new name's cache folder.
    pub fn commit(&self, mut staged: Staged, state: &mut State) -> Result<Installed> {
        let new_folders = staged.folders();
        let previous = state
            .entry(&staged.name)
            .or_else(|| renamed_entry(state, &staged.name, &new_folders))
            .map(|(name, entry)| (name.to_string(), entry.clone()));
        let stale: Vec<String> = previous
            .iter()
            .flat_map(|(_, entry)| entry.folders.iter())
            .filter(|folder| !contains_folder(&new_folders, folder))
            .cloned()
            .collect();
        let guarded: Vec<String> = new_folders.iter().chain(&stale).cloned().collect();
        self.refuse_links(state, &guarded)?;
        let addons_dir = self.addons_dir();
        std::fs::create_dir_all(&addons_dir).map_err(|source| Error::Io {
            context: "creating the AddOns folder",
            path: addons_dir.clone(),
            source,
        })?;

        // The running-client check (task 023), once per commit and before the first folder moves: a running
        // client loads the new folders at `/reload`, so it is a warning, never a refusal (task 038).
        let client_warning = self.client_warning(RunningClient::install_warning);

        let previous_dir = staged.dir.join(staging::PREVIOUS_DIR);
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

        let mut warnings = std::mem::take(&mut staged.sweep_warnings);
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
            nolib: staged.nolib,
            pinned: None,
            ignored: false,
            channel: Channel::Stable,
        };
        let entry = keep_marks(
            entry,
            &staged.name,
            previous.as_ref().map(|(_, entry)| entry),
            &mut warnings,
        );
        let renamed = previous
            .as_ref()
            .map(|(old_name, _)| old_name.clone())
            .filter(|old_name| !old_name.eq_ignore_ascii_case(&staged.name));
        if let Some((old_name, _)) = &previous {
            state.addons.remove(old_name);
        }
        if let Some(old_name) = &renamed {
            warnings.push(format!(
                "{old_name} is managed as {} now, with its marks and its previous version",
                staged.name
            ));
        }
        take_over_folders(state, &staged.name, &new_folders, &mut warnings);
        state.addons.insert(staged.name.clone(), entry);
        self.home.write_state(state)?;
        if let Some(old_name) = &renamed {
            self.carry_cache(old_name, &staged.name, &mut warnings);
        }
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
    /// the zip, swaps the folders in and records the install; the commit refuses a linked addon or a
    /// junction mrgl did not make (task 005).
    pub fn install(&self, zip: &Path, metadata: &Metadata) -> Result<Installed> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let scan = scan::scan_flavor(&self.flavor)?;
        let dropped = state.reconcile(&scan);
        let staged = self.stage(zip, metadata)?;
        let installed = self.commit(staged, &mut state)?;
        Ok(with_run_warnings(installed, loaded.warning, dropped))
    }

    /// The target an addon from a source is picked for: the engine's flavor and build, on the channel of
    /// the addon's sidecar entry (task 021): the entry from that source, else the entry of that name, else
    /// `stable`.
    pub fn target_for(&self, state: &State, source: &str, name: Option<&str>) -> Target {
        let channel = managed_entry(state, source, name)
            .map(|(_, entry)| entry.channel)
            .unwrap_or_default();
        Target::from_flavor(&self.flavor).with_channel(channel)
    }

    /// The pick a request asks for, made for the engine's flavor on the addon's own channel
    /// ([`Engine::target_for`]): what the one install path would install, for a dry run. Reads the
    /// sidecar, writes nothing (the source may write its own cache).
    pub fn resolve<S: Source + ?Sized>(&self, source: &S, request: &SourceRequest) -> Result<Pick> {
        let state = self.load_state()?.state;
        let target = self.target_for(&state, &request.spec.to_string(), request.name.as_deref());
        resolve_on(source, request, &target)
    }

    /// The one install path of a source (task 055): `add github:`, `import` and `update` all come here.
    /// The request is resolved on the addon's own channel ([`Engine::target_for`]); when the sidecar has
    /// an addon from the pick's source (ASCII case-insensitive, as GitHub's names are) at the pick's version
    /// and its folders are there, nothing is downloaded and nothing moves. Otherwise the asset is downloaded
    /// through the source into a fresh folder under `staging\`, staged and committed with the pick's source
    /// and version as the metadata (the TOC's `## Version` stays the fallback), under the name the sidecar
    /// knows the addon by when it came from this source, else the request's; an addon at another version
    /// is updated by that same commit, which also refuses a linked addon or a foreign junction. The
    /// download is removed once the install returns, whatever it returned; the engine keeps its own copy
    /// in the cache.
    pub fn install_from_source<S: Source + ?Sized>(
        &self,
        source: &S,
        request: &SourceRequest,
        progress: &mut dyn FnMut(PickStep<'_>),
    ) -> Result<SourceInstalled> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let pick = match &request.wanted {
            Wanted::Pick(pick) => (**pick).clone(),
            _ => {
                let target =
                    self.target_for(&state, &request.spec.to_string(), request.name.as_deref());
                progress(PickStep::Resolving {
                    spec: &request.spec,
                    target: &target,
                });
                let pick = resolve_on(source, request, &target)?;
                progress(PickStep::Picked { pick: &pick });
                pick
            }
        };
        let scan = scan::scan_flavor(&self.flavor)?;
        let dropped = state.reconcile(&scan);
        let managed = managed_entry(&state, &pick.source, None)
            .map(|(name, entry)| (name.to_string(), entry.clone()));
        if let Some((name, entry)) = &managed
            && entry.version == pick.version
        {
            let outcome = PickInstalled::AlreadyInstalled(AlreadyInstalled {
                name: name.clone(),
                entry: entry.clone(),
                warnings: loaded.warning.into_iter().collect(),
            });
            return Ok(SourceInstalled { pick, outcome });
        }
        let download = RunDir::create(
            &self.home.staging_dir(),
            &format!("download-{}", pick.asset),
        )?;
        let zip = download.path().join(state::file_safe(&pick.asset));
        progress(PickStep::Downloading {
            pick: &pick,
            to: &zip,
        });
        source.fetch(&pick, &zip)?;
        progress(PickStep::Installing {
            pick: &pick,
            zip: &zip,
        });
        let metadata = Metadata {
            source: pick.source.clone(),
            version: Some(pick.version.clone()),
            name: managed.map(|(name, _)| name).or(request.name.clone()),
            nolib: Some(pick.nolib),
        };
        let staged = self.stage(&zip, &metadata)?;
        let installed = self.commit(staged, &mut state)?;
        let outcome =
            PickInstalled::Installed(with_run_warnings(installed, loaded.warning, dropped));
        Ok(SourceInstalled { pick, outcome })
    }

    /// The one install path ([`Engine::install_from_source`]) for a pick in hand (task 040): no request
    /// to the source but the download.
    pub fn install_pick<S: Source + ?Sized>(
        &self,
        source: &S,
        pick: &Pick,
        progress: &mut dyn FnMut(PickStep<'_>),
    ) -> Result<PickInstalled> {
        let request = SourceRequest {
            spec: SourceSpec::parse(&pick.source)?,
            wanted: Wanted::Pick(Box::new(pick.clone())),
            name: None,
        };
        self.install_from_source(source, &request, progress)
            .map(|installed| installed.outcome)
    }

    /// Removes a managed addon: its folders under `Interface\AddOns`, its entry and its cache. A name the
    /// sidecar does not know is an error that says whether the folder exists; a linked addon is refused,
    /// `unlink` takes it out (task 005).
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
        self.refuse_remove_linked(&known, &entry)?;
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

    /// Pins a managed addon at a version, or at the installed one when none is given (task 021; the
    /// rollback of task 022 pins the version it rolls back to).
    pub fn pin(&self, name: &str, version: Option<&str>) -> Result<Marked> {
        self.mark(name, "pinned", |entry| entry.pin(version))
    }

    /// Unpins a managed addon.
    pub fn unpin(&self, name: &str) -> Result<Marked> {
        self.mark(name, "unpinned", Entry::unpin)
    }

    /// Marks a managed addon ignored: the update check skips it.
    pub fn ignore(&self, name: &str) -> Result<Marked> {
        self.mark(name, "ignored", Entry::ignore)
    }

    /// Clears the ignored mark of a managed addon.
    pub fn unignore(&self, name: &str) -> Result<Marked> {
        self.mark(name, "unignored", Entry::unignore)
    }

    /// Puts a managed addon on a release channel.
    pub fn set_channel(&self, name: &str, channel: Channel) -> Result<Marked> {
        self.mark(name, "put on a channel", |entry| entry.set_channel(channel))
    }

    /// One of the five marks above by its [`Mark`] (task 010: the command line and the window call this).
    pub fn apply_mark(&self, name: &str, mark: &Mark) -> Result<Marked> {
        match mark {
            Mark::Pin(version) => self.pin(name, version.as_deref()),
            Mark::Unpin => self.unpin(name),
            Mark::Ignore => self.ignore(name),
            Mark::Unignore => self.unignore(name),
            Mark::Channel(channel) => self.set_channel(name, *channel),
        }
    }

    /// Rolls a managed addon back to its previous version (task 022): the entry's `previous_version` and
    /// its zip in the cache go through the same stage and swap as an install, with the entry's source
    /// unchanged, and the addon is pinned at that version so an update does not bring the version rolled
    /// back from in again (`unpin` is the step back). The cache keeps both zips, the one rolled back to (now
    /// current) and the one rolled back from (now previous), so a second rollback goes forward again.
    /// A name the sidecar does not know, an entry with no previous version and a missing cached zip are
    /// errors, each before anything is written.
    pub fn rollback(&self, name: &str) -> Result<RolledBack> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let scan = scan::scan_flavor(&self.flavor)?;
        let dropped = state.reconcile(&scan);
        let flavor = flavor_label(&self.flavor.id);
        let Some((known, entry)) = state
            .find(name)
            .map(|(known, entry)| (known.to_string(), entry.clone()))
        else {
            let name = name.to_string();
            return Err(if self.addons_dir().join(&name).is_dir() {
                Error::NotManagedToMark {
                    name,
                    flavor,
                    action: "rolled back",
                }
            } else {
                Error::AddonNotFound { name, flavor }
            });
        };
        let Some(target) = entry
            .previous_version
            .clone()
            .filter(|previous| previous != &entry.version)
        else {
            return Err(Error::NoPreviousVersion {
                name: known,
                version: entry.version,
                flavor,
            });
        };
        let zip = self.home.cached_zip(&known, &target);
        if !zip.is_file() {
            return Err(Error::RollbackZipMissing {
                name: known,
                version: target,
                path: zip,
            });
        }
        let metadata = Metadata {
            source: entry.source.clone(),
            version: Some(target.clone()),
            name: Some(known.clone()),
            // The `nolib` flag of task 029 stays what the entry has: the rollback changes the version, not the
            // build's shape.
            nolib: entry.nolib,
        };
        let staged = self.stage(&zip, &metadata)?;
        // The pin goes on before the commit, which keeps the replaced entry's marks: one sidecar write,
        // and no warning that the pin differs from the version just installed.
        if let Some(current) = state.addons.get_mut(&known) {
            current.pin(Some(&target));
        }
        let mut installed = self.commit(staged, &mut state)?;
        let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
        warnings.extend(dropped.into_iter().map(|name| {
            format!("{name} was deleted by hand since its install and is no longer managed")
        }));
        warnings.append(&mut installed.warnings);
        Ok(RolledBack {
            name: installed.name,
            version: installed.version,
            previous_version: installed.previous_version.unwrap_or(entry.version),
            pinned: target,
            source: entry.source,
            folders: installed.folders,
            removed_folders: installed.removed_folders,
            warnings,
        })
    }

    /// Reads the sidecar, applies one change to the entry a name means (by name, else by folder) and
    /// writes the sidecar when the change moved something. A name the sidecar does not know is an error
    /// that says whether the folder exists. Nothing in the game folder is touched.
    fn mark(
        &self,
        name: &str,
        action: &'static str,
        change: impl FnOnce(&mut Entry) -> bool,
    ) -> Result<Marked> {
        let loaded = self.load_state()?;
        let mut state = loaded.state;
        let warnings: Vec<String> = loaded.warning.into_iter().collect();
        let Some((known, entry)) = state.find_mut(name) else {
            let flavor = flavor_label(&self.flavor.id);
            let name = name.to_string();
            return Err(if self.addons_dir().join(&name).is_dir() {
                Error::NotManagedToMark {
                    name,
                    flavor,
                    action,
                }
            } else {
                Error::AddonNotFound { name, flavor }
            });
        };
        let changed = change(entry);
        let marked = Marked {
            name: known,
            version: entry.version.clone(),
            pinned: entry.pinned.clone(),
            ignored: entry.ignored,
            channel: entry.channel,
            changed,
            warnings,
        };
        if changed {
            self.home.write_state(&state)?;
        }
        Ok(marked)
    }

    /// Moves the zips an addon's cache folder keeps under its old name into the folder of its new name
    /// (task 077), so the rollback finds the previous version's zip after a reinstall under a new name; a
    /// zip the new folder has already stays where it is. Best effort: a failure is a warning, the install
    /// stands, and the old folder goes once it is empty.
    fn carry_cache(&self, old_name: &str, new_name: &str, warnings: &mut Vec<String>) {
        let from = self.home.addon_cache_dir(old_name);
        let Ok(entries) = std::fs::read_dir(&from) else {
            return;
        };
        let to = self.home.addon_cache_dir(new_name);
        for entry in entries.flatten() {
            let target = to.join(entry.file_name());
            if target.exists() {
                continue;
            }
            let moved =
                std::fs::create_dir_all(&to).and_then(|()| std::fs::rename(entry.path(), &target));
            if let Err(error) = moved {
                warnings.push(format!(
                    "the cached zip of {old_name} could not move to {}: {error}",
                    target.display()
                ));
            }
        }
        let _ = std::fs::remove_dir(&from);
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

/// A reinstall keeps the pin, the ignored mark and the channel of the entry it replaces (task 021): they
/// are the user's word about the addon, not about one version. A pin at another version than the one just
/// installed stays, with a warning that names it.
fn keep_marks(
    mut entry: Entry,
    name: &str,
    previous: Option<&Entry>,
    warnings: &mut Vec<String>,
) -> Entry {
    let Some(previous) = previous else {
        return entry;
    };
    entry.pinned = previous.pinned.clone();
    entry.ignored = previous.ignored;
    entry.channel = previous.channel;
    if let Some(pinned) = entry.pinned.as_deref()
        && pinned != entry.version
    {
        warnings.push(format!(
            "{name} is pinned at {pinned} and {} is installed now; the pin stays until `mrgl unpin {name}`",
            entry.version
        ));
    }
    entry
}

/// The entry that records the addon a commit stages under another name (task 077): a zip in hand is named
/// by the rule of task 065 now, while the sidecar may know the same folders under the name an older rule
/// gave. It is the entry that records the folder the staged name names while its own name is a folder of
/// the staged addon, both ASCII case-insensitive: each name points at the other's folders, so a library
/// folder one addon ships and another addon records is never taken for the addon. A folder belongs to one
/// entry at most, so at most one entry answers.
fn renamed_entry<'a>(
    state: &'a State,
    name: &str,
    folders: &[String],
) -> Option<(&'a str, &'a Entry)> {
    let (old_name, entry) = state.entry_for_folder(name)?;
    (!old_name.eq_ignore_ascii_case(name) && contains_folder(folders, old_name))
        .then_some((old_name, entry))
}

/// The sidecar entry of an addon from a source: the entry whose source is that spec (ASCII
/// case-insensitive), else the entry of the name given. The one lookup of the install path (task 055).
fn managed_entry<'a>(
    state: &'a State,
    source: &str,
    name: Option<&str>,
) -> Option<(&'a str, &'a Entry)> {
    state
        .addons
        .iter()
        .find(|(_, entry)| entry.source.eq_ignore_ascii_case(source))
        .map(|(name, entry)| (name.as_str(), entry))
        .or_else(|| name.and_then(|name| state.entry(name)))
}

/// The pick a request asks for, on a target: the source's current release, the pick in hand, or the
/// release of a version.
fn resolve_on<S: Source + ?Sized>(
    source: &S,
    request: &SourceRequest,
    target: &Target,
) -> Result<Pick> {
    match &request.wanted {
        Wanted::Current => source.resolve(&request.spec, target),
        Wanted::Pick(pick) => Ok((**pick).clone()),
        Wanted::Version(version) => source.resolve_version(&request.spec, target, version),
    }
}

/// An install's warnings after the run's own: the sidecar read as fresh, then the addons deleted by hand
/// since their install, then what the commit said.
fn with_run_warnings(
    mut installed: Installed,
    sidecar: Option<String>,
    dropped: Vec<String>,
) -> Installed {
    let mut warnings: Vec<String> = sidecar.into_iter().collect();
    warnings.extend(dropped.into_iter().map(|name| {
        format!("{name} was deleted by hand since its install and is no longer managed")
    }));
    warnings.append(&mut installed.warnings);
    installed.warnings = warnings;
    installed
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

/// Unpacks a zip under a folder once [`zip_entry::checked_entries`] has passed every entry, every entry
/// read through the bound's counter (task 039). Returns every file as its relative path with `/`
/// separators and its SHA-256. On an error the caller removes the folder ([`Staged`]'s drop).
fn unpack(zip: &Path, into: &Path, max_unpacked: u64) -> Result<BTreeMap<String, String>> {
    let unreadable = |reason: String| Error::ZipUnreadable {
        role: ZipRole::Addon,
        path: zip.to_path_buf(),
        reason,
    };
    let file = std::fs::File::open(zip).map_err(|source| unreadable(source.to_string()))?;
    let mut archive = zip::ZipArchive::new(io::BufReader::new(file))
        .map_err(|reason| unreadable(reason.to_string()))?;
    let entries = zip_entry::checked_entries(&mut archive, zip, ZipRole::Addon, max_unpacked)?;
    let mut unpacked = zip_entry::Unpacked::new(zip, ZipRole::Addon, max_unpacked);
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
        let entry = archive
            .by_index(checked.index)
            .map_err(|reason| unreadable(format!("entry {}: {reason}", checked.index)))?;
        let output = std::fs::File::create(&path)
            .map_err(|source| io("writing a staged file", &path, source))?;
        let hash = copy_hashing(
            &mut unpacked.entry(&checked, entry),
            io::BufWriter::new(output),
        )
        .map_err(|source| {
            zip_entry::refusal(source)
                .unwrap_or_else(|source| io("writing a staged file", &path, source))
        })?;
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

/// The names a zip suggests for its addon (task 065), the strongest first: the zip's stem without its
/// version and flavor suffix ([`stem_name`]: `DBM-Core` for `DBM-Core-12.0.11-classic`), then the
/// repository of a `github:<owner>/<repo>` source. [`derive_name`] takes a folder that answers one of them.
pub fn name_hints(zip_stem: &str, source: &str) -> Vec<String> {
    let mut hints = Vec::new();
    let stem = stem_name(zip_stem);
    if !stem.is_empty() {
        hints.push(stem.to_string());
    }
    if let Ok(SourceSpec::GitHub { repo, .. }) = SourceSpec::parse(source) {
        hints.push(repo);
    }
    hints
}

/// A zip's stem without the version and what follows it, or without a trailing flavor or `nolib` part
/// when it names no version: the `-`-separated parts before the first one that starts with a digit or
/// with `v` and a digit (`Questie` for `Questie-v1.2.3-forever`), else the parts before the trailing
/// flavor suffixes (`Addon` for `Addon-forever-nolib`).
pub fn stem_name(stem: &str) -> &str {
    let is_version = |part: &str| {
        let digits = part
            .strip_prefix(['v', 'V'])
            .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(part);
        digits.starts_with(|c: char| c.is_ascii_digit())
    };
    let mut end = 0;
    for (index, part) in stem.split('-').enumerate() {
        if index > 0 && is_version(part) {
            return &stem[..end];
        }
        end += if index == 0 {
            part.len()
        } else {
            part.len() + 1
        };
    }
    let mut name = stem;
    while let Some((head, last)) = name.rsplit_once('-') {
        let suffix = last.eq_ignore_ascii_case("nolib")
            || crate::sources::github::FLAVOR_SUFFIXES
                .iter()
                .any(|flavor| last.eq_ignore_ascii_case(flavor));
        if !suffix || head.is_empty() {
            break;
        }
        name = head;
    }
    name
}

/// The addon name of a set of folders: the folder whose name every other one extends with `_` or `-`
/// (`BigWigs` for `BigWigs_Core`), else the folder a hint names, ASCII case-insensitive, the first hint
/// that names one ([`name_hints`]: `DBM-Core` of `DBM-Core-12.0.11.zip` among `DBM-Brawlers`, `DBM-Core`
/// and `DBM-GUI`, task 065), else the first by name.
pub fn derive_name<'a>(folders: &[&'a str], hints: &[String]) -> &'a str {
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
        .or_else(|| {
            hints.iter().find_map(|hint| {
                sorted
                    .iter()
                    .copied()
                    .find(|folder| folder.eq_ignore_ascii_case(hint))
            })
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
        let error = unpack(&zip, &into, zip_entry::MAX_UNPACKED).expect_err("refused");
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
            derive_name(&["BigWigs_Core", "BigWigs", "BigWigs_Options"], &[]),
            "BigWigs"
        );
        assert_eq!(
            derive_name(&["DBM-GUI", "DBM-Core", "DBM-StatusBarTimers"], &[]),
            "DBM-Core"
        );
        assert_eq!(derive_name(&["Zed", "Alpha"], &[]), "Alpha");
        assert_eq!(derive_name(&["Solo"], &[]), "Solo");
        assert_eq!(derive_name(&["foo_x", "Foo"], &[]), "Foo");
        assert_eq!(derive_name(&["FooBar", "Foo"], &[]), "Foo");
        assert_eq!(derive_name(&[], &[]), "");
    }

    #[test]
    fn a_hint_names_the_addon_when_no_folder_is_extended_by_the_others() {
        let dbm = ["DBM-Brawlers", "DBM-Core", "DBM-GUI", "DBM-StatusBarTimers"];
        assert_eq!(derive_name(&dbm, &[]), "DBM-Brawlers", "the old fallback");
        let hints = |stem: &str, source: &str| name_hints(stem, source);
        assert_eq!(
            derive_name(
                &dbm,
                &hints("DBM-Core-12.0.11", "C:\\zips\\DBM-Core-12.0.11.zip")
            ),
            "DBM-Core"
        );
        assert_eq!(
            derive_name(&dbm, &hints("dbm-core-12.0.11-classic", "")),
            "DBM-Core",
            "ASCII case-insensitive, the flavor suffix gone with the version"
        );
        assert_eq!(
            derive_name(&dbm, &hints("download", "github:Someone/DBM-Core")),
            "DBM-Core",
            "the repository when the stem names no folder"
        );
        assert_eq!(
            derive_name(&dbm, &hints("DBM-GUI-1.0", "github:Someone/DBM-Core")),
            "DBM-GUI",
            "the stem before the repository"
        );
        assert_eq!(
            derive_name(
                &dbm,
                &hints("DeadlyBossMods-1.0", "github:Someone/Elsewhere")
            ),
            "DBM-Brawlers",
            "no hint names a folder: the first by name"
        );
        assert_eq!(
            derive_name(
                &["BigWigs_Core", "BigWigs", "BigWigs_Options"],
                &hints("BigWigs_Options-1.0", "")
            ),
            "BigWigs",
            "the folder the others extend comes first"
        );
    }

    #[test]
    fn the_stem_name_drops_the_version_and_the_flavor() {
        assert_eq!(stem_name("DBM-Core-12.0.11"), "DBM-Core");
        assert_eq!(stem_name("DBM-Core-12.0.11-classic"), "DBM-Core");
        assert_eq!(stem_name("Questie-v1.2.3-forever"), "Questie");
        assert_eq!(stem_name("several-folders-2.0.0"), "several-folders");
        assert_eq!(stem_name("Addon-forever-nolib"), "Addon");
        assert_eq!(stem_name("Addon-Mists"), "Addon");
        assert_eq!(stem_name("Vuhdo-Options"), "Vuhdo-Options");
        assert_eq!(stem_name("2fast"), "2fast", "the first part is the name");
        assert_eq!(stem_name("forever"), "forever");
        assert_eq!(stem_name(""), "");
        assert_eq!(name_hints("", "zip"), Vec::<String>::new());
        assert_eq!(
            name_hints("A-v1.0.0-forever", "github:o/Repo"),
            ["A", "Repo"]
        );
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
