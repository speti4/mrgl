//! The flavor list the window draws (task 009): every flavor folder under the game root, each with its
//! addon rows in the shape of the data contract in `ui/mock-data.js`. A row carries the facts the core has
//! on the addon (the scan, the state sidecar, the dependency check, the Forever notes) and the view derives
//! the row's one state from them (`ui/state.js`); nothing here names a state.
//!
//! Everything is offline: the "update available" fact is the answer of the update check (`update::check`,
//! tasks 006 and 068, with its version order) run against the GitHub source through [`Offline`], an [`Http`]
//! that fails every request as the network does, so the source answers from its own cache by its offline
//! rule (task 041) and never opens a socket (task 010). The window's own check asks the sources and lays
//! its answer over this one ([`flavor_view`]'s `online`), and what it found that the cache cannot tell
//! again (a source gone, a failed check, the budget stop) is kept in the flavor's `last-check.json` and
//! laid over every later list until the next check replaces it (task 078, `mrgl_core::last_check`). The functions take the game root and the mrgl
//! home folder as arguments, so the tests run them on fixture folders without a window and without touching
//! the process's environment.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_core::dependencies::{self, AddonDependencies};
use mrgl_core::discover::{GameRoot, RootOrigin};
use mrgl_core::flavor::{self, Flavor, FlavorId, VersionSource};
use mrgl_core::forever::{ForeverBuild, ForeverStatus};
use mrgl_core::install::Engine;
use mrgl_core::last_check::LastCheck;
use mrgl_core::link;
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::sources::github::{self, GitHub};
use mrgl_core::sources::{Channel, Http, Pick, Request, Response, Source, SourceSpec, Target};
use mrgl_core::state::{Entry, FlavorHome, State};
use mrgl_core::update::{self, AddonUpdate, UpdateState, VersionOrder};
use mrgl_core::voice::{Kind, Voice};
use serde::Serialize;

use crate::commands::Message;

/// An [`Http`] that never reaches the network: every request fails as an unreachable network does, so the
/// GitHub source answers from its cached release list and `release.json` (task 041) without counting a
/// request against the rate limit (task 068) and without stamping the cache as confirmed, and a repository
/// it has no cache of is an error the list reads as "nothing known".
#[derive(Debug, Clone, Copy, Default)]
pub struct Offline;

impl Http for Offline {
    fn get(&self, request: &Request) -> mrgl_core::error::Result<Response> {
        Err(mrgl_core::Error::Network {
            url: request.url.clone(),
            reason: "the list reads the cache only".to_string(),
        })
    }
}

/// The answer of `flavors` and of a chosen root: the game root, where it came from, the flavors and the
/// message for the status bar.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlavorsAnswer {
    pub root: Option<String>,
    pub root_origin: Option<RootOrigin>,
    pub flavors: Vec<FlavorView>,
    pub message: Message,
}

/// One flavor folder as the sidebar and the header show it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlavorView {
    /// The `.flavor.info` id (`wowf`).
    pub id: String,
    /// The friendly name (`forever`), or the id for a flavor the manager does not know.
    pub name: String,
    /// The folder's own name (`_forever_`).
    pub folder: String,
    /// The installed build, when known.
    pub version: Option<String>,
    pub interface: Option<u32>,
    /// Where the build was read (task 019): `build-info` or `client-exe`.
    pub version_source: Option<&'static str>,
    /// The file it was read from: `.build.info` or the exe's name.
    pub version_file: Option<String>,
    /// The CurseForge app's `.curseclient` marker is in the folder.
    pub curse_client: bool,
    /// The flavor has its `Interface\AddOns` folder; a client that was never started has none (task 010).
    pub addons_folder: bool,
    /// What reading the flavor met: an unknown id, a build neither file has, a sidecar it could not read.
    pub warnings: Vec<String>,
    pub addons: Vec<Row>,
}

/// One addon folder of a flavor, the facts of a row.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Row {
    /// The folder under `Interface\AddOns`.
    pub folder: String,
    /// The TOC's title without its color escapes, else the folder.
    pub name: String,
    /// The installed version: the sidecar's for a managed addon, the TOC's `## Version` otherwise.
    pub version: Option<String>,
    /// A junction (task 005), or a folder the sidecar records from a `link:` source.
    pub linked: bool,
    /// Where the addon comes from: the sidecar's source, or a TOC hint for an unmanaged addon.
    pub source: Option<Badge>,
    /// The sidecar knows the folder.
    pub managed: bool,
    /// The source is only a TOC hint (`## X-Wago-ID` and the like), not adopted yet.
    pub guessed: bool,
    /// The update the check found (task 010): its version, when the check says `update-available`.
    pub update: Option<Update>,
    /// What the update check says of the addon (task 010): offline from the source's cache, or the
    /// window's last check; `None` for an unmanaged addon or when nothing is known.
    pub check: Option<CheckFact>,
    /// The release channel the addon follows (task 021); `None` for an unmanaged addon.
    pub channel: Option<Channel>,
    /// The version the addon is pinned at (task 021).
    pub pinned: Option<String>,
    /// The addon is ignored by the update check (task 021).
    pub ignored: bool,
    /// The Forever notes (task 011), on every row of the Forever flavor and on no other.
    pub forever: Option<ForeverStatus>,
    /// The required dependencies no addon folder of their own answers (task 029), a library embedded in
    /// the addon's folder included: the client does not count it (task 076).
    pub missing: Vec<String>,
}

/// A source badge: the kind the view colors it by and the id its tooltip names.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Badge {
    pub kind: &'static str,
    pub id: String,
}

/// What the update check says of one addon (task 010), as a row shows it: the state, its words, the
/// current release, how the installed version stands to it (task 077: a pinned row's note reads it), the
/// reason, and the check's data line for a tooltip.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckFact {
    pub state: UpdateState,
    pub label: &'static str,
    pub latest: Option<String>,
    pub order: Option<VersionOrder>,
    pub detail: Option<String>,
    pub line: String,
}

impl CheckFact {
    pub fn of(addon: &AddonUpdate) -> CheckFact {
        CheckFact {
            state: addon.state,
            label: addon.state.label(),
            latest: addon.latest.clone(),
            order: addon.order,
            detail: addon.detail.clone(),
            line: addon.line(),
        }
    }
}

/// The update an addon of the check has: the pick's version and release when the state is
/// `update-available`, nothing otherwise (a pinned, a newer installed or a guessed release is no update).
pub fn update_of(addon: &AddonUpdate) -> Option<Update> {
    if addon.state != UpdateState::UpdateAvailable {
        return None;
    }
    addon.pick.as_ref().map(|pick| Update {
        version: pick.version.clone(),
        release: pick.release.clone(),
        prerelease: pick.prerelease,
    })
}

/// An update the check found.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Update {
    /// The release's version (the tag without its leading `v`), as the install records it.
    pub version: String,
    /// The release's tag.
    pub release: String,
    /// The release is a prerelease (an addon on the prerelease channel, task 021).
    pub prerelease: bool,
}

/// The answer for a game root: its flavors with their rows, and the banner that sums them up. `home` is the
/// mrgl home folder (the sidecars and the release cache); `None` reads every addon as unmanaged.
pub fn load(
    root: Option<GameRoot>,
    warnings: Vec<String>,
    home: Option<&Path>,
    voice: Voice,
) -> FlavorsAnswer {
    let Some(root) = root else {
        let mut plain = "no game root: none remembered, none in the registry or the usual install folders; choose the game folder".to_string();
        append_warnings(&mut plain, &warnings);
        return FlavorsAnswer {
            root: None,
            root_origin: None,
            flavors: Vec::new(),
            message: Message::new(voice, Kind::Nothing, plain),
        };
    };
    let flavors = match flavor::list_flavors(&root.path) {
        Ok(flavors) if !flavors.is_empty() => flavors,
        Ok(_) => {
            return failed(
                &root,
                voice,
                mrgl_core::Error::NotAGameRoot {
                    path: root.path.clone(),
                },
            );
        }
        Err(error) => return failed(&root, voice, error),
    };
    let views: Vec<FlavorView> = flavors
        .into_iter()
        .map(|flavor| flavor_view(flavor, home, &[]))
        .collect();
    let count: usize = views.iter().map(|view| view.addons.len()).sum();
    let mut plain = format!(
        "{} under {}, {}",
        count_noun(views.len(), "flavor"),
        root.path.display(),
        count_noun(count, "addon")
    );
    if root.origin.is_discovered() {
        plain.push_str(&format!(
            " (found in {}; choose the game folder to remember it)",
            root.origin.description()
        ));
    }
    append_warnings(&mut plain, &warnings);
    FlavorsAnswer {
        root: Some(root.path.display().to_string()),
        root_origin: Some(root.origin),
        flavors: views,
        message: Message::new(voice, Kind::Banner, plain),
    }
}

/// The answer for a root that cannot be listed: no flavors, the reason as an error.
fn failed(root: &GameRoot, voice: Voice, error: mrgl_core::Error) -> FlavorsAnswer {
    FlavorsAnswer {
        root: Some(root.path.display().to_string()),
        root_origin: Some(root.origin),
        flavors: Vec::new(),
        message: Message::new(voice, Kind::Error, error),
    }
}

/// The run's warnings (a corrupt config) after the banner's facts.
fn append_warnings(plain: &mut String, warnings: &[String]) {
    for warning in warnings {
        plain.push_str(&format!("; {warning}"));
    }
}

/// `1 addon`, `2 addons`, `no addons`: the count-noun summary of `docs/nerglish.md`.
pub fn count_noun(count: usize, noun: &str) -> String {
    match count {
        0 => format!("no {noun}s"),
        1 => format!("1 {noun}"),
        count => format!("{count} {noun}s"),
    }
}

/// One flavor with its rows; a folder that cannot be scanned is a flavor without rows and a warning. The
/// rows' check facts are the update check's offline answer, with the answers the last online check kept
/// (a source gone, a failed check, the budget stop; task 078) laid over it by name, and `online` (the
/// addons of the check that just asked the sources) over both.
pub fn flavor_view(flavor: Flavor, home: Option<&Path>, online: &[AddonUpdate]) -> FlavorView {
    let mut warnings = flavor.warnings.clone();
    let id = flavor.id.clone();
    let mut view = FlavorView {
        id: id.as_str().to_string(),
        name: id.friendly_name().unwrap_or(id.as_str()).to_string(),
        folder: flavor.folder_name(),
        version: flavor.version.map(|version| version.to_string()),
        interface: flavor.interface(),
        version_source: flavor.version_source.as_ref().map(VersionSource::kind),
        version_file: flavor
            .version_source
            .as_ref()
            .map(|source| source.file_name().to_string()),
        curse_client: flavor.curse_client,
        addons_folder: scan::addons_dir(&flavor.folder).is_dir(),
        warnings: Vec::new(),
        addons: Vec::new(),
    };
    let scan = match scan::scan_flavor(&flavor) {
        Ok(scan) => scan,
        Err(error) => {
            warnings.push(error.to_string());
            view.warnings = warnings;
            return view;
        }
    };
    warnings.extend(scan.warnings.iter().cloned());
    let state = read_state(&id, home, &mut warnings);
    let github = GitHub::new(
        Offline,
        None,
        home.map(|home| home.join(github::CACHE_SUBDIR)),
    );
    let mut checked: BTreeMap<String, AddonUpdate> = BTreeMap::new();
    if let Some(home) = home {
        for addon in offline_check(&flavor, home, &github) {
            checked.insert(addon.name.to_ascii_lowercase(), addon);
        }
        // The last online check's answers the cache cannot give again (task 078).
        for addon in LastCheck::read(&FlavorHome::at(home, &id), &id).answers(&state) {
            checked.insert(addon.name.to_ascii_lowercase(), addon);
        }
    }
    for addon in online {
        checked.insert(addon.name.to_ascii_lowercase(), addon.clone());
    }
    view.addons = rows(&scan, &state, &github, &checked);
    view.warnings = warnings;
    view
}

/// The update check of a flavor against the source's cache alone ([`Offline`]): what the last check or
/// install left there, in the check's own order. An addon whose answer needed the network (no cache, or a
/// pin without a cached release) is left out: nothing is known of it.
fn offline_check(flavor: &Flavor, home: &Path, github: &GitHub<Offline>) -> Vec<AddonUpdate> {
    let engine = Engine::with_home(flavor.clone(), home);
    let Ok(check) = update::check(&engine, github, None, &mut |_| {}) else {
        return Vec::new();
    };
    check
        .addons
        .into_iter()
        .filter(|addon| match addon.state {
            UpdateState::Failed | UpdateState::RateLimited => false,
            UpdateState::Pinned => addon.latest.is_some(),
            _ => true,
        })
        .collect()
}

/// The flavor's sidecar: a missing or corrupt one is a fresh state (the scan is the truth, task 037), one
/// that cannot be read at all is a warning and a fresh state too.
fn read_state(id: &FlavorId, home: Option<&Path>, warnings: &mut Vec<String>) -> State {
    let Some(home) = home else {
        warnings.push("no mrgl home folder: every addon listed as unmanaged".to_string());
        return State::new(id);
    };
    match FlavorHome::at(home, id).read_state(id) {
        Ok(loaded) => loaded.state,
        Err(error) => {
            warnings.push(format!(
                "state sidecar not read, every addon listed as unmanaged: {error}"
            ));
            State::new(id)
        }
    }
}

/// The rows of a scan, in its order, against the sidecar, the update check's addons by their lower-case
/// name, and the cached releases (the Forever build).
pub fn rows<H: Http>(
    scan: &Scan,
    state: &State,
    github: &GitHub<H>,
    checked: &BTreeMap<String, AddonUpdate>,
) -> Vec<Row> {
    let checks = dependencies::check(scan, Some(state));
    let forever_flavor = scan.flavor.id == FlavorId::Forever;
    let mut forever: BTreeMap<String, ForeverStatus> = BTreeMap::new();
    scan.addons
        .iter()
        .map(|addon| {
            let managed = state.entry_for_folder(&addon.folder);
            let entry = managed.map(|(_, entry)| entry);
            let primary = entry.is_some_and(|entry| {
                entry
                    .folders
                    .first()
                    .is_some_and(|first| first.eq_ignore_ascii_case(&addon.folder))
            });
            let check = match managed {
                Some((name, _)) if primary => checked.get(&name.to_ascii_lowercase()),
                _ => None,
            };
            let forever = forever_flavor.then(|| match managed {
                Some((name, entry)) => forever
                    .entry(name.to_ascii_lowercase())
                    .or_insert_with(|| forever_status(github, entry))
                    .clone(),
                None => unknown_forever(),
            });
            row(addon, entry, check, forever, &checks)
        })
        .collect()
}

/// One row from the scan's addon, its sidecar entry and the facts looked up for the entry.
fn row(
    addon: &Addon,
    entry: Option<&Entry>,
    check: Option<&AddonUpdate>,
    forever: Option<ForeverStatus>,
    checks: &[AddonDependencies],
) -> Row {
    let toc = addon
        .chosen_toc()
        .or_else(|| addon.primary_toc())
        .map(|file| &file.toc);
    let name = toc
        .and_then(|toc| toc.title_plain.as_deref())
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(&addon.folder)
        .to_string();
    let version = match entry {
        Some(entry) => Some(entry.version.clone()),
        None => addon.version().map(str::to_string),
    };
    let linked_source = entry.and_then(|entry| link::linked_folder(&entry.source));
    let linked = addon.linked || linked_source.is_some();
    let (source, guessed) = match entry {
        Some(entry) => (Some(badge_of_source(&entry.source)), false),
        None if addon.linked => (
            Some(Badge {
                kind: "linked",
                id: link::link_target(&addon.path)
                    .map(|target| target.display().to_string())
                    .unwrap_or_default(),
            }),
            false,
        ),
        None => match toc.and_then(hinted_badge) {
            Some(badge) => (Some(badge), true),
            None => (None, false),
        },
    };
    let missing = checks
        .iter()
        .find(|check| check.folder == addon.folder)
        .map(|check| check.missing.clone())
        .unwrap_or_default();
    Row {
        folder: addon.folder.clone(),
        name,
        version,
        linked,
        source,
        managed: entry.is_some(),
        guessed,
        update: check.and_then(update_of),
        check: check.map(CheckFact::of),
        channel: entry.map(Entry::channel),
        pinned: entry.and_then(Entry::pinned).map(str::to_string),
        ignored: entry.is_some_and(Entry::ignored),
        forever,
        missing,
    }
}

/// The badge of a sidecar source: `github:<owner>/<repo>`, `link:<folder>`, or the path of a zip in hand.
pub fn badge_of_source(source: &str) -> Badge {
    if let Some(folder) = link::linked_folder(source) {
        return Badge {
            kind: "linked",
            id: folder.to_string(),
        };
    }
    if let Ok(SourceSpec::GitHub { owner, repo }) = SourceSpec::parse(source) {
        return Badge {
            kind: "github",
            id: format!("{owner}/{repo}"),
        };
    }
    let name = Path::new(source)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| source.to_string());
    Badge {
        kind: "zip",
        id: name,
    }
}

/// The source a TOC hints at for an unmanaged addon, in the order of D-002: Wago, WoWInterface, then
/// CurseForge.
fn hinted_badge(toc: &mrgl_core::toc::Toc) -> Option<Badge> {
    [
        ("wago", toc.wago_id.as_ref()),
        ("wowi", toc.wowi_id.as_ref()),
        ("curseforge", toc.curse_project_id.as_ref()),
    ]
    .into_iter()
    .find_map(|(kind, id)| {
        id.map(|id| Badge {
            kind,
            id: id.trim().to_string(),
        })
    })
}

/// The Forever status of a managed addon, as `mrgl list` gives it: the build signal from the source's
/// cache (GitHub only) and the curated note of its source spec.
fn forever_status<H: Http>(github: &GitHub<H>, entry: &Entry) -> ForeverStatus {
    let build = match SourceSpec::parse(&entry.source) {
        Ok(spec) => github.cached_forever_build(&spec, entry.channel()),
        Err(_) => ForeverBuild::Unknown,
    };
    ForeverStatus::for_source(&entry.source, build)
}

/// The Forever status of an addon mrgl does not manage: nothing says, no note.
fn unknown_forever() -> ForeverStatus {
    ForeverStatus {
        build: ForeverBuild::Unknown,
        note: None,
        kind: None,
        verified: None,
        source: None,
        stale: false,
    }
}

/// What a resolve through the window answers (task 009, the handover to 010): the source's own [`Pick`]
/// and the message that says it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolveAnswer {
    pub pick: Option<Pick>,
    pub message: Message,
}

/// Resolves a managed addon of a flavor folder through its source, on its channel, for the flavor's build:
/// the [`Pick`] the install engine takes. The source decides whether that goes to the network.
pub fn resolve<S: Source>(
    source: &S,
    flavor_folder: &Path,
    folder: &str,
    home: Option<&Path>,
    voice: Voice,
) -> ResolveAnswer {
    let error = |plain: String| ResolveAnswer {
        pick: None,
        message: Message::new(voice, Kind::Error, plain),
    };
    let flavor = match flavor::read_flavor(flavor_folder) {
        Ok(flavor) => flavor,
        Err(error_) => return error(error_.to_string()),
    };
    let mut warnings = Vec::new();
    let state = read_state(&flavor.id, home, &mut warnings);
    let Some((name, entry)) = state.entry_for_folder(folder) else {
        return error(format!(
            "{folder} is not managed by mrgl in {}",
            flavor.folder_name()
        ));
    };
    let spec = match SourceSpec::parse(&entry.source) {
        Ok(spec) => spec,
        Err(error_) => return error(format!("{name} cannot be resolved: {error_}")),
    };
    let target = Target::from_flavor(&flavor).with_channel(entry.channel());
    match source.resolve(&spec, &target) {
        Ok(pick) => {
            let kind = if pick.version == entry.version {
                Kind::Nothing
            } else {
                Kind::Success
            };
            let plain = if pick.version == entry.version {
                format!(
                    "{name} is up to date at {}: {}",
                    entry.version,
                    pick.summary()
                )
            } else {
                format!(
                    "{name} {} → {}: {}",
                    entry.version,
                    pick.version,
                    pick.summary()
                )
            };
            ResolveAnswer {
                message: Message::new(voice, kind, plain),
                pick: Some(pick),
            }
        }
        Err(error_) => error(error_.to_string()),
    }
}

/// The line for a flavor without its `Interface\AddOns` folder (task 010): the client was never started
/// for it, so there is nothing to check or update yet.
pub fn no_addons_folder_line(folder_name: &str) -> String {
    format!(
        "no Interface\\AddOns folder in {folder_name}: the client has not been started for this flavor yet, so there is nothing to check or update; start it once and the folder is there"
    )
}

/// The flavor folder of an id under a root: the known folder name, else the first flavor folder whose
/// `.flavor.info` says that id.
pub fn flavor_folder(root: &Path, id: &str) -> Option<PathBuf> {
    let id = FlavorId::parse(id);
    if let Some(name) = id.folder_name() {
        return Some(root.join(name));
    }
    flavor::list_flavors(root)
        .ok()?
        .into_iter()
        .find(|flavor| flavor.id == id)
        .map(|flavor| flavor.folder)
}
