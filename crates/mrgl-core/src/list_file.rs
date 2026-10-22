//! The addon list (task 015): a flavor's managed addons as a JSON file, written by `mrgl export` and read
//! back by `mrgl import` into another flavor or onto another machine.
//!
//! ```json
//! {
//!   "format": "mrgl-list",
//!   "schema": 1,
//!   "flavor": "wowf",
//!   "exported_at": "2026-10-21T14:03:05Z",
//!   "addons": [
//!     { "name": "Questie", "source": "github:Questie/Questie", "version": "1.2.3", "folders": ["Questie"] }
//!   ]
//! }
//! ```
//!
//! One entry per managed addon of the state sidecar (the entries whose folders were deleted by hand are
//! left out, D-003), with the sidecar's name, source and version and the addon's folders. The import
//! resolves every entry of a source spec (`github:`, `wago:`, `wowi:`, `curse:`) through the run's sources
//! and installs the current release through the one install path of the engine
//! ([`Engine::install_from_source`], task 055); the version in the file is what was installed at the
//! export, reported next to what the import brings. An entry of a source the run cannot ask (this build
//! does not have it, or its key is not set) is reported as "source not configured" with what is missing
//! (task 092, [`Source::missing`]). A zip in hand is reported as not importable (the zip is on the other
//! machine), a linked folder (`link:`) is skipped, and an addon the flavor has already (by name, by source,
//! or a folder of the entry that is there) is reported and left alone at whatever version it is.
//!
//! A list is shareable (task 028): it names sources and versions, never a path of one machine. Reading a
//! list refuses an entry whose source is a zip file, a linked development folder (`link:`) or another local
//! or absolute path, or whose folder is a path rather than a folder name, and the error names every such
//! entry ([`ListEntry::not_shareable`]); the export leaves an addon from a zip in hand or a linked folder
//! out of the list, with a warning that names it. A list written by hand or kept in the repo
//! (`lists/forever-starter.json`) goes through the same check; the import's outcomes for a zip or a
//! linked entry stay for a list built in code.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dependencies::AddonDependencies;
use crate::error::{Error, Result};
use crate::install::{Engine, PickInstalled, SourceInstalled, SourceRequest, Wanted};
use crate::interface::{InterfaceMismatch, InterfaceOutOfDate};
use crate::scan;
use crate::sources::{Source, SourceSpec};
use crate::state::{self, State};

/// The `format` of a list file.
pub const FORMAT: &str = "mrgl-list";
/// The schema of a list file; a file with another number is refused.
pub const SCHEMA: u32 = 1;

/// A list file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListFile {
    /// [`FORMAT`].
    pub format: String,
    /// [`SCHEMA`].
    pub schema: u32,
    /// The flavor id the list was exported from, as `.flavor.info` spells it (`wowf`).
    pub flavor: String,
    /// When the list was written, RFC 3339 in UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<String>,
    /// The managed addons, by name.
    #[serde(default)]
    pub addons: Vec<ListEntry>,
}

/// One managed addon of a list file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListEntry {
    /// The sidecar's name of the addon.
    pub name: String,
    /// The source spec as the sidecar records it: `github:<owner>/<repo>`, a zip path, `link:…`.
    pub source: String,
    /// The version installed at the export.
    pub version: String,
    /// The addon's folders, the primary one first.
    #[serde(default)]
    pub folders: Vec<String>,
}

impl ListFile {
    /// The list of a sidecar's managed addons, sorted by name as the sidecar keeps them.
    pub fn from_state(state: &State, exported_at: Option<String>) -> ListFile {
        ListFile {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: state.flavor.clone(),
            exported_at,
            addons: state
                .addons
                .iter()
                .map(|(name, entry)| ListEntry {
                    name: name.clone(),
                    source: entry.source.clone(),
                    version: entry.version.clone(),
                    folders: entry.folders.clone(),
                })
                .collect(),
        }
    }

    /// The file's text: pretty JSON with a final newline.
    pub fn to_json(&self) -> String {
        let text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        text + "\n"
    }

    /// Reads a list from its text; the path is for the error. Another format or schema is refused.
    pub fn parse(text: &str, path: &Path) -> Result<ListFile> {
        let unreadable = |reason: String| Error::ListUnreadable {
            path: path.to_path_buf(),
            reason,
        };
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let list: ListFile =
            serde_json::from_str(text).map_err(|reason| unreadable(reason.to_string()))?;
        if list.format != FORMAT {
            return Err(unreadable(format!(
                "format {} instead of {FORMAT}",
                list.format
            )));
        }
        if list.schema != SCHEMA {
            return Err(unreadable(format!(
                "schema {} instead of {SCHEMA}",
                list.schema
            )));
        }
        let refused: Vec<String> = list
            .addons
            .iter()
            .filter_map(|entry| {
                entry
                    .not_shareable()
                    .map(|reason| format!("{}: {reason}", entry.name))
            })
            .collect();
        if !refused.is_empty() {
            return Err(Error::ListNotShareable {
                path: path.to_path_buf(),
                entries: refused.join("; "),
            });
        }
        Ok(list)
    }

    /// Reads a list file.
    pub fn read(path: &Path) -> Result<ListFile> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            context: "reading the addon list",
            path: path.to_path_buf(),
            source,
        })?;
        ListFile::parse(&text, path)
    }

    /// Writes the list file, creating its folder.
    pub fn write(&self, path: &Path) -> Result<()> {
        let io = |path: &Path, source| Error::Io {
            context: "writing the addon list",
            path: path.to_path_buf(),
            source,
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|source| io(parent, source))?;
        }
        std::fs::write(path, self.to_json()).map_err(|source| io(path, source))
    }
}

impl ListEntry {
    /// Why the entry has no place in a shareable list (task 028), as a clause after its name, or `None`
    /// when it names a source and plain folder names only: a zip file, a linked development folder and any
    /// other local or absolute path belong to one machine and mean nothing on another.
    pub fn not_shareable(&self) -> Option<String> {
        let source = self.source.trim();
        match EntrySource::of(source) {
            EntrySource::Linked => {
                return Some(format!(
                    "its source {source} is a linked development folder"
                ));
            }
            EntrySource::Zip if source.to_ascii_lowercase().ends_with(".zip") => {
                return Some(format!("its source {source} is a zip file"));
            }
            EntrySource::Zip => return Some(format!("its source {source} is a local path")),
            EntrySource::Spec(_) | EntrySource::Unsupported => {}
        }
        self.folders
            .iter()
            .find(|folder| {
                let folder = folder.trim();
                folder.contains(['/', '\\', ':']) || folder == "." || folder == ".."
            })
            .map(|folder| format!("its folder {folder} is a path, not a folder name"))
    }
}

/// What kind of source a list entry or a sidecar entry names, as far as the import and the update check
/// are concerned. Whether the run can ask a spec's source is the sources' answer, not the spec's
/// ([`Source::missing`], task 092): a spec of a source this build has is resolved and installed, one of a
/// source not configured is reported with what is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntrySource {
    /// A source spec: `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>` or `curse:<id>`.
    Spec(SourceSpec),
    /// A zip in hand: its path is on the machine that exported the list.
    Zip,
    /// A linked development folder (`link:…`, task 005): skipped.
    Linked,
    /// Anything else: no spec mrgl reads.
    Unsupported,
}

impl EntrySource {
    /// The kind of a sidecar source spec.
    pub fn of(source: &str) -> EntrySource {
        let trimmed = source.trim();
        if trimmed
            .get(..5)
            .is_some_and(|head| head.eq_ignore_ascii_case("link:"))
        {
            return EntrySource::Linked;
        }
        if let Ok(spec) = SourceSpec::parse(trimmed) {
            return EntrySource::Spec(spec);
        }
        let looks_like_a_path = trimmed.contains('\\')
            || trimmed.contains('/')
            || trimmed.as_bytes().get(1) == Some(&b':') && !trimmed.contains("://");
        if trimmed.to_ascii_lowercase().ends_with(".zip") || looks_like_a_path {
            return EntrySource::Zip;
        }
        EntrySource::Unsupported
    }
}

/// The export of a flavor: its list, and the plain warnings of reading the sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub list: ListFile,
    pub warnings: Vec<String>,
}

/// The list of a flavor's managed addons: the sidecar against a scan of the AddOns folder, so an addon
/// deleted by hand is left out (the sidecar itself is not written). An addon from a zip in hand or a
/// linked folder is left out too, with a warning, so the list stays shareable (task 028).
pub fn export(engine: &Engine) -> Result<Export> {
    let loaded = engine.load_state()?;
    let mut state = loaded.state;
    let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
    let scan = scan::scan_flavor(&engine.flavor)?;
    for name in state.reconcile(&scan) {
        warnings.push(format!(
            "{name} was deleted by hand since its install and is left out of the list"
        ));
    }
    let mut list = ListFile::from_state(&state, Some(state::now_rfc3339()));
    list.addons.retain(|entry| match entry.not_shareable() {
        Some(reason) => {
            warnings.push(format!("{} is left out of the list: {reason}", entry.name));
            false
        }
        None => true,
    });
    Ok(Export { list, warnings })
}

/// What the import did with one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Resolved and installed.
    Installed,
    /// A dry run: resolved, would be installed.
    WouldInstall,
    /// The flavor has it already; left alone.
    AlreadyInstalled,
    /// A zip in hand or a source spec mrgl does not read.
    NotImportable,
    /// A source the run cannot ask (task 092): this build does not have it, or its key is not set; the
    /// detail says which.
    SourceNotConfigured,
    /// A linked folder.
    Skipped,
    /// The resolve, the download or the install failed; the detail says why.
    Failed,
    /// An exact import (task 057): the source no longer offers the version the list names; skipped.
    VersionNotOffered,
}

/// One entry of the list and what the import did with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Imported {
    /// The entry's name.
    pub name: String,
    /// The entry's source.
    pub source: String,
    /// The version the list names.
    pub listed_version: String,
    pub outcome: Outcome,
    /// The version installed (or that would be), or the one the flavor already has.
    pub version: Option<String>,
    /// The folders written (or that the flavor already has).
    pub folders: Vec<String>,
    /// The plain reason: the pick, why it is not importable, the error.
    pub detail: String,
    /// The installed addon's TOC names no interface of the flavor's build.
    pub interface_mismatch: Option<InterfaceMismatch>,
    /// The installed addon's TOC is behind the flavor's build: the client's `Out of date` (task 082).
    pub interface_out_of_date: Option<InterfaceOutOfDate>,
    /// The dependency check of each installed folder after the whole import (task 090); empty for an
    /// entry that installed nothing. A missing dependency is a line in `warnings` too.
    pub dependencies: Vec<AddonDependencies>,
    /// Plain lines the install engine reported for this entry.
    pub warnings: Vec<String>,
}

/// What an import did, entry by entry, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub items: Vec<Imported>,
    /// Plain lines about the whole run: a list from another flavor, a rebuilt sidecar.
    pub warnings: Vec<String>,
}

impl ImportReport {
    /// How many entries ended with an outcome.
    pub fn count(&self, outcome: Outcome) -> usize {
        self.items
            .iter()
            .filter(|item| item.outcome == outcome)
            .count()
    }
}

/// What the flavor already has of an entry: by name, by the same source, or a folder of the entry.
struct Present<'a> {
    state: &'a State,
    folders: Vec<String>,
    added_names: HashSet<String>,
    added_sources: HashSet<String>,
}

impl Present<'_> {
    /// The version and the folders the flavor has of an entry, or why it counts as there.
    fn find(
        &self,
        entry: &ListEntry,
        spec: Option<&SourceSpec>,
    ) -> Option<(Option<String>, String)> {
        let key = |text: &str| text.to_ascii_lowercase();
        if self.added_names.contains(&key(&entry.name))
            || spec.is_some_and(|spec| self.added_sources.contains(&key(&spec.to_string())))
        {
            return Some((None, "installed earlier in this import".to_string()));
        }
        if let Some((name, known)) = self.state.entry(&entry.name) {
            return Some((
                Some(known.version.clone()),
                format!("{name} {} is managed already", known.version),
            ));
        }
        if let Some(spec) = spec
            && let Some((name, known)) = self.state.addons.iter().find(|(_, known)| {
                SourceSpec::parse(&known.source)
                    .is_ok_and(|other| key(&other.to_string()) == key(&spec.to_string()))
            })
        {
            return Some((
                Some(known.version.clone()),
                format!("{name} {} comes from the same source", known.version),
            ));
        }
        let folder = entry.folders.iter().find(|folder| {
            self.folders
                .iter()
                .any(|known| known.eq_ignore_ascii_case(folder))
        })?;
        Some((
            None,
            format!("its folder {folder} is there, installed by hand or by another manager"),
        ))
    }
}

/// Imports a list into the engine's flavor: every `github:` entry the flavor does not have is resolved
/// through the source and installed through the engine's one install path under the list's name for it
/// ([`Engine::install_from_source`]): its current release, or with `exact` the release of the version the
/// list names (task 057; an entry whose version the source no longer offers is
/// [`Outcome::VersionNotOffered`] and skipped); `dry_run` resolves and reports ([`Engine::resolve`]), and
/// writes nothing under `Interface\AddOns` or in the sidecar. An entry that fails is reported as failed and the
/// import goes on; only a sidecar or AddOns folder that cannot be read at all is an error.
pub fn import(
    engine: &Engine,
    source: &dyn Source,
    list: &ListFile,
    exact: bool,
    dry_run: bool,
) -> Result<ImportReport> {
    let loaded = engine.load_state()?;
    let mut state = loaded.state;
    let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
    let before = scan::scan_flavor(&engine.flavor)?;
    state.reconcile(&before);
    if list.flavor != engine.flavor.id.as_str() {
        warnings.push(format!(
            "the list comes from flavor {}, imported into {}: every pick is made for {}",
            list.flavor,
            engine.flavor.id,
            engine.flavor.folder_name()
        ));
    }
    let mut present = Present {
        state: &state,
        folders: before
            .addons
            .iter()
            .map(|addon| addon.folder.clone())
            .chain(before.skipped.iter().cloned())
            .collect(),
        added_names: HashSet::new(),
        added_sources: HashSet::new(),
    };
    let mut items = Vec::new();
    for entry in &list.addons {
        let kind = EntrySource::of(&entry.source);
        let item = |outcome, version: Option<String>, detail: String| Imported {
            name: entry.name.clone(),
            source: entry.source.clone(),
            listed_version: entry.version.clone(),
            outcome,
            version,
            folders: Vec::new(),
            detail,
            interface_mismatch: None,
            interface_out_of_date: None,
            dependencies: Vec::new(),
            warnings: Vec::new(),
        };
        if kind == EntrySource::Linked {
            items.push(item(
                Outcome::Skipped,
                None,
                "a linked development folder; link it again with mrgl link".to_string(),
            ));
            continue;
        }
        let spec = match &kind {
            EntrySource::Spec(spec) => Some(spec),
            _ => None,
        };
        if let Some((version, detail)) = present.find(entry, spec) {
            items.push(item(Outcome::AlreadyInstalled, version, detail));
            continue;
        }
        let Some(spec) = spec else {
            let detail = if kind == EntrySource::Zip {
                "a zip in hand on the machine of the export; add it again with mrgl add <zip>"
            } else {
                "not a source spec mrgl reads"
            };
            items.push(item(Outcome::NotImportable, None, detail.to_string()));
            continue;
        };
        // Task 092: a source this run cannot ask is named with what is missing, before any request.
        if let Some(missing) = source.missing(spec) {
            items.push(item(Outcome::SourceNotConfigured, None, missing));
            continue;
        }
        // The one install path of task 055, under the list's name for the addon; an exact import asks
        // for the listed version (task 057).
        let request = SourceRequest {
            spec: spec.clone(),
            wanted: if exact {
                Wanted::Version(entry.version.clone())
            } else {
                Wanted::Current
            },
            name: Some(entry.name.clone()),
        };
        let failed = |error: Error| match error {
            Error::VersionNotOffered { .. } => {
                item(Outcome::VersionNotOffered, None, error.to_string())
            }
            error => item(Outcome::Failed, None, error.to_string()),
        };
        if dry_run {
            items.push(match engine.resolve(source, &request) {
                Ok(pick) => {
                    let mut would = item(
                        Outcome::WouldInstall,
                        Some(pick.version.clone()),
                        pick.summary(),
                    );
                    // Task 041: a pick from the cached release list, offline, says so.
                    would.warnings.extend(pick.stale_warning());
                    would
                }
                Err(error) => failed(error),
            });
            continue;
        }
        match engine.install_from_source(source, &request, &mut |_| {}) {
            Ok(SourceInstalled {
                pick,
                outcome: PickInstalled::Installed(installed),
            }) => {
                present
                    .added_names
                    .insert(installed.name.to_ascii_lowercase());
                present
                    .added_sources
                    .insert(spec.to_string().to_ascii_lowercase());
                let mut done = item(
                    Outcome::Installed,
                    Some(installed.version.clone()),
                    pick.summary(),
                );
                done.folders = installed.folders;
                done.warnings = pick.stale_warning().into_iter().collect();
                done.warnings.extend(installed.warnings);
                items.push(done);
            }
            Ok(SourceInstalled {
                outcome: PickInstalled::AlreadyInstalled(current),
                ..
            }) => items.push(item(
                Outcome::AlreadyInstalled,
                Some(current.entry.version.clone()),
                format!(
                    "{} {} is managed already",
                    current.name, current.entry.version
                ),
            )),
            Err(error) => items.push(failed(error)),
        }
    }

    // The checks of the install summary of `add` and `update` (tasks 029, 082, 090), over the flavor as
    // the whole import left it: the interface check of each installed addon and its missing dependencies.
    let installed: Vec<&mut Imported> = items
        .iter_mut()
        .filter(|item| item.outcome == Outcome::Installed)
        .collect();
    if !installed.is_empty() {
        let folders: Vec<&[String]> = installed
            .iter()
            .map(|item| item.folders.as_slice())
            .collect();
        let checks = engine.check_installed_each(&folders);
        for (item, checks) in installed.into_iter().zip(checks) {
            item.warnings.extend(checks.warnings);
            item.warnings.extend(
                checks
                    .dependencies
                    .iter()
                    .filter_map(AddonDependencies::warning),
            );
            item.interface_mismatch = checks.mismatch;
            item.interface_out_of_date = checks.out_of_date;
            item.dependencies = checks.dependencies;
        }
    }
    Ok(ImportReport { items, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavor::{BuildVersion, Flavor, FlavorId};
    use crate::install::Metadata;
    use crate::sources::{Pick, Target};
    use crate::state::Entry;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-list-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn zips() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zips")
    }

    fn entry(source: &str, version: &str, folders: &[&str]) -> Entry {
        Entry {
            source: source.to_string(),
            version: version.to_string(),
            previous_version: None,
            folders: folders.iter().map(|folder| folder.to_string()).collect(),
            files: BTreeMap::new(),
            installed_at: "2026-10-21T14:03:05Z".to_string(),
            nolib: None,
            pinned: None,
            ignored: false,
            channel: crate::sources::Channel::Stable,
        }
    }

    /// A forever flavor folder with no addons, and its engine under a temporary home.
    fn engine(name: &str) -> (PathBuf, Engine) {
        let base = temp(name);
        let folder = base.join("game").join("_forever_");
        std::fs::create_dir_all(&folder).expect("the flavor folder");
        let flavor = Flavor {
            id: FlavorId::Forever,
            folder,
            version: BuildVersion::parse("1.60.1.70245"),
            version_source: None,
            curse_client: false,
            warnings: Vec::new(),
        };
        let engine = Engine::with_home(flavor, &base.join("home"));
        (base, engine)
    }

    /// A source that answers every spec with one pick and serves one fixture zip.
    struct OneZip {
        zip: PathBuf,
        version: &'static str,
    }

    impl Source for OneZip {
        fn resolve(&self, spec: &SourceSpec, _target: &Target) -> Result<Pick> {
            if spec.to_string() == "github:Example/Missing" {
                return Err(Error::NoRelease {
                    spec: spec.to_string(),
                });
            }
            Ok(Pick {
                source: spec.to_string(),
                release: format!("v{}", self.version),
                version: self.version.to_string(),
                published: None,
                prerelease: false,
                asset: "Some-Asset.zip".to_string(),
                url: "https://github.test/Some-Asset.zip".to_string(),
                size: None,
                flavor: "forever".to_string(),
                interface: Some(16001),
                guess: false,
                reason: "release.json names forever (interface 16001)".to_string(),
                notes: None,
                page: None,
                nolib: false,
                forever_build: crate::forever::ForeverBuild::Unknown,
                stale: None,
            })
        }

        fn fetch(&self, _pick: &Pick, to: &Path) -> Result<()> {
            std::fs::create_dir_all(to.parent().expect("a parent")).expect("the folder");
            std::fs::copy(&self.zip, to).expect("the copy");
            Ok(())
        }
    }

    #[test]
    fn a_list_round_trips_and_refuses_another_format_or_schema() {
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert(
            "SeveralFolders".to_string(),
            entry(
                "github:Example/Several",
                "2.0.0",
                &["SeveralFolders", "SeveralFolders_Options"],
            ),
        );
        let list = ListFile::from_state(&state, Some("2026-10-21T14:03:05Z".to_string()));
        let text = list.to_json();
        assert!(text.contains("\"format\": \"mrgl-list\""), "{text}");
        assert!(text.contains("\"schema\": 1"), "{text}");
        assert!(text.contains("\"flavor\": \"wowf\""), "{text}");
        let path = Path::new("list.json");
        assert_eq!(ListFile::parse(&text, path).expect("parses"), list);
        assert_eq!(
            ListFile::parse(&format!("\u{feff}{text}"), path).expect("a BOM is fine"),
            list
        );

        let other = text.replace("mrgl-list", "something-else");
        let error = ListFile::parse(&other, path).expect_err("another format");
        assert_eq!(
            error.to_string(),
            "not a readable addon list: list.json (format something-else instead of mrgl-list)"
        );
        let newer = text.replace("\"schema\": 1", "\"schema\": 2");
        assert!(
            ListFile::parse(&newer, path)
                .expect_err("another schema")
                .to_string()
                .ends_with("(schema 2 instead of 1)")
        );
        assert!(ListFile::parse("{ not json", path).is_err());
    }

    #[test]
    fn the_source_kind_follows_the_spec() {
        assert_eq!(
            EntrySource::of("github:Questie/Questie"),
            EntrySource::Spec(SourceSpec::parse("github:Questie/Questie").expect("a spec"))
        );
        // Task 092: every source's spec is a spec; whether the run can ask it is the sources' answer.
        for spec in ["wago:abc123", "wowi:123", "curse:42"] {
            assert_eq!(
                EntrySource::of(spec),
                EntrySource::Spec(SourceSpec::parse(spec).expect("a spec"))
            );
        }
        assert_eq!(
            EntrySource::of("link:D:\\dev\\MyAddon"),
            EntrySource::Linked
        );
        assert_eq!(EntrySource::of("LINK:somewhere"), EntrySource::Linked);
        assert_eq!(
            EntrySource::of("C:\\Downloads\\SomeAddon-1.2.3.zip"),
            EntrySource::Zip
        );
        assert_eq!(
            EntrySource::of("D:\\addons\\no-extension"),
            EntrySource::Zip
        );
        assert_eq!(
            EntrySource::of("bitbucket:abc123"),
            EntrySource::Unsupported
        );
    }

    #[test]
    fn an_entry_of_a_source_not_configured_says_what_is_missing() {
        let (base, engine) = engine("not-configured");
        let entry = |name: &str, source: &str| ListEntry {
            name: name.to_string(),
            source: source.to_string(),
            version: "1.0.0".to_string(),
            folders: vec![name.to_string()],
        };
        let list = ListFile {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: "wowf".to_string(),
            exported_at: None,
            addons: vec![
                entry("OneFolder", "github:Example/One"),
                entry("Wowi", "wowi:123"),
                entry("Odd", "bitbucket:abc"),
            ],
        };
        let sources = crate::sources::Sources::none().with(
            crate::sources::SourceKind::GitHub,
            OneZip {
                zip: zips().join("one-folder-1.2.0.zip"),
                version: "1.2.0",
            },
        );
        for dry_run in [true, false] {
            let report = import(&engine, &sources, &list, false, dry_run).expect("imports");
            let outcomes: Vec<(Outcome, &str)> = report
                .items
                .iter()
                .map(|item| (item.outcome, item.detail.as_str()))
                .skip(1)
                .collect();
            assert_eq!(
                outcomes,
                [
                    (
                        Outcome::SourceNotConfigured,
                        "this build has no WoWInterface source"
                    ),
                    (Outcome::NotImportable, "not a source spec mrgl reads"),
                ]
            );
        }
        assert_eq!(
            serde_json::to_value(Outcome::SourceNotConfigured).expect("json"),
            "source-not-configured"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn export_leaves_out_what_was_deleted_by_hand() {
        let (base, engine) = engine("export");
        let installed = engine
            .install(
                &zips().join("one-folder-1.0.0.zip"),
                &Metadata {
                    source: "github:Example/One".to_string(),
                    version: Some("1.0.0".to_string()),
                    name: None,
                    nolib: None,
                },
            )
            .expect("installs");
        assert_eq!(installed.name, "OneFolder");
        let mut state = engine.load_state().expect("reads").state;
        state.addons.insert(
            "Gone".to_string(),
            entry("github:Example/Gone", "1.0.0", &["Gone"]),
        );
        engine.home.write_state(&state).expect("writes");

        let export = export(&engine).expect("exports");
        assert_eq!(export.list.flavor, "wowf");
        assert_eq!(
            export.list.addons,
            [ListEntry {
                name: "OneFolder".to_string(),
                source: "github:Example/One".to_string(),
                version: "1.0.0".to_string(),
                folders: vec!["OneFolder".to_string()],
            }]
        );
        assert_eq!(
            export.warnings,
            ["Gone was deleted by hand since its install and is left out of the list"]
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn import_installs_github_entries_and_reports_the_rest() {
        let (base, engine) = engine("import");
        std::fs::create_dir_all(engine.addons_dir().join("ByHand")).expect("a folder by hand");
        std::fs::write(
            engine.addons_dir().join("ByHand").join("ByHand.toc"),
            "## Interface: 16001\n",
        )
        .expect("its TOC");
        let list = ListFile {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: "wow".to_string(),
            exported_at: None,
            addons: vec![
                ListEntry {
                    name: "OneFolder".to_string(),
                    source: "github:Example/One".to_string(),
                    version: "0.9.0".to_string(),
                    folders: vec!["OneFolder".to_string()],
                },
                ListEntry {
                    name: "ByHand".to_string(),
                    source: "github:Example/ByHand".to_string(),
                    version: "1.0.0".to_string(),
                    folders: vec!["ByHand".to_string()],
                },
                ListEntry {
                    name: "Zipped".to_string(),
                    source: "C:\\Downloads\\Zipped-1.0.0.zip".to_string(),
                    version: "1.0.0".to_string(),
                    folders: vec!["Zipped".to_string()],
                },
                ListEntry {
                    name: "Dev".to_string(),
                    source: "link:D:\\dev\\Dev".to_string(),
                    version: "dev".to_string(),
                    folders: vec!["Dev".to_string()],
                },
                ListEntry {
                    name: "Missing".to_string(),
                    source: "github:Example/Missing".to_string(),
                    version: "1.0.0".to_string(),
                    folders: vec!["Missing".to_string()],
                },
            ],
        };
        let source = OneZip {
            zip: zips().join("one-folder-1.2.0.zip"),
            version: "1.2.0",
        };

        let dry = import(&engine, &source, &list, false, true).expect("a dry run");
        let outcomes: Vec<Outcome> = dry.items.iter().map(|item| item.outcome).collect();
        assert_eq!(
            outcomes,
            [
                Outcome::WouldInstall,
                Outcome::AlreadyInstalled,
                Outcome::NotImportable,
                Outcome::Skipped,
                Outcome::Failed
            ]
        );
        assert_eq!(dry.items[0].version.as_deref(), Some("1.2.0"));
        assert_eq!(
            dry.warnings,
            [
                "the list comes from flavor wow, imported into wowf: every pick is made for _forever_"
            ]
        );
        assert!(!engine.addons_dir().join("OneFolder").exists());
        assert!(
            !engine.home.state_path().exists(),
            "a dry run writes nothing"
        );

        let report = import(&engine, &source, &list, false, false).expect("imports");
        assert_eq!(report.count(Outcome::Installed), 1);
        assert_eq!(report.count(Outcome::Failed), 1);
        let one = &report.items[0];
        assert_eq!(one.outcome, Outcome::Installed);
        assert_eq!(one.version.as_deref(), Some("1.2.0"));
        assert_eq!(one.listed_version, "0.9.0");
        assert_eq!(one.folders, ["OneFolder"]);
        assert_eq!(one.interface_mismatch, None, "16001 fits 1.60");
        assert_eq!(
            report.items[4].detail,
            "no release in github:Example/Missing"
        );
        let state = engine.load_state().expect("reads").state;
        let (_, installed) = state.entry("OneFolder").expect("the entry");
        assert_eq!(installed.source, "github:Example/One");
        assert_eq!(installed.version, "1.2.0");
        assert!(state.entry("ByHand").is_none(), "left alone");
        assert_eq!(
            std::fs::read_dir(engine.home.staging_dir())
                .map(|entries| entries.count())
                .unwrap_or_default(),
            0,
            "the download is gone"
        );

        let again = import(&engine, &source, &list, false, false).expect("imports again");
        assert_eq!(again.items[0].outcome, Outcome::AlreadyInstalled);
        assert_eq!(again.items[0].version.as_deref(), Some("1.2.0"));
        assert_eq!(again.count(Outcome::Installed), 0);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_installed_addon_with_another_interface_carries_the_mismatch() {
        let (base, engine) = engine("mismatch");
        let list = ListFile {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: "wowf".to_string(),
            exported_at: None,
            addons: vec![ListEntry {
                name: "RootToc".to_string(),
                source: "github:Example/RootToc".to_string(),
                version: "3.0.0".to_string(),
                folders: vec!["RootToc".to_string()],
            }],
        };
        let source = OneZip {
            zip: zips().join("root-toc-3.0.0.zip"),
            version: "3.0.0",
        };
        let report = import(&engine, &source, &list, false, false).expect("imports");
        let mismatch = report.items[0]
            .interface_mismatch
            .as_ref()
            .expect("120105 is not for 1.60");
        assert_eq!(
            mismatch.line(),
            "RootToc: interface 120105 is not for the build 1.60.1.70245"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    // --- task 028: the shareable list ---

    fn list_entry(name: &str, source: &str, folders: &[&str]) -> ListEntry {
        ListEntry {
            name: name.to_string(),
            source: source.to_string(),
            version: "1.0.0".to_string(),
            folders: folders.iter().map(|folder| folder.to_string()).collect(),
        }
    }

    #[test]
    fn an_entry_with_a_path_is_not_shareable() {
        let shareable =
            |source: &str, folders: &[&str]| list_entry("Some", source, folders).not_shareable();
        assert_eq!(shareable("github:Questie/Questie", &["Questie"]), None);
        assert_eq!(
            shareable("https://github.com/Questie/Questie", &["Questie"]),
            None
        );
        assert_eq!(shareable("wago:abc123", &["Some"]), None, "not a path");
        assert_eq!(shareable("github:Example/NoFolders", &[]), None);
        assert_eq!(
            shareable("C:\\Downloads\\Some-1.2.3.zip", &["Some"]).as_deref(),
            Some("its source C:\\Downloads\\Some-1.2.3.zip is a zip file")
        );
        assert_eq!(
            shareable("Some-1.2.3.zip", &["Some"]).as_deref(),
            Some("its source Some-1.2.3.zip is a zip file")
        );
        assert_eq!(
            shareable("link:D:\\dev\\Some", &["Some"]).as_deref(),
            Some("its source link:D:\\dev\\Some is a linked development folder")
        );
        for path in [
            "D:\\addons\\Some",
            "\\\\server\\share\\Some",
            "/home/someone/Some",
        ] {
            assert_eq!(
                shareable(path, &["Some"]),
                Some(format!("its source {path} is a local path"))
            );
        }
        for folder in ["..\\Some", "C:\\AddOns\\Some", "Some/Inner", ".."] {
            assert_eq!(
                shareable("github:Example/Some", &["Some", folder]),
                Some(format!("its folder {folder} is a path, not a folder name"))
            );
        }
    }

    #[test]
    fn reading_a_list_refuses_every_entry_with_a_path_by_name() {
        let mut list = ListFile {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: "wowf".to_string(),
            exported_at: None,
            addons: vec![
                list_entry("Questie", "github:Questie/Questie", &["Questie"]),
                list_entry("Zipped", "C:\\Downloads\\Zipped-1.0.0.zip", &["Zipped"]),
                list_entry("Dev", "link:D:\\dev\\Dev", &["Dev"]),
            ],
        };
        let path = Path::new("shared.json");
        let error = ListFile::parse(&list.to_json(), path).expect_err("not shareable");
        assert!(matches!(error, Error::ListNotShareable { .. }), "{error}");
        assert_eq!(
            error.to_string(),
            "not a shareable addon list: shared.json (Zipped: its source C:\\Downloads\\Zipped-1.0.0.zip is a zip file; Dev: its source link:D:\\dev\\Dev is a linked development folder); a list names sources and versions, never a file or a folder of one machine"
        );

        list.addons.truncate(1);
        assert_eq!(
            ListFile::parse(&list.to_json(), path).expect("shareable"),
            list
        );
    }

    #[test]
    fn export_leaves_out_a_zip_in_hand_and_a_linked_folder() {
        let (base, engine) = engine("export-shareable");
        engine
            .install(
                &zips().join("one-folder-1.0.0.zip"),
                &Metadata {
                    source: "github:Example/One".to_string(),
                    version: Some("1.0.0".to_string()),
                    name: None,
                    nolib: None,
                },
            )
            .expect("installs");
        let mut state = engine.load_state().expect("reads").state;
        for (name, source) in [
            ("Zipped", "C:\\Downloads\\Zipped-1.0.0.zip"),
            ("Dev", "link:D:\\dev\\Dev"),
        ] {
            let folder = engine.addons_dir().join(name);
            std::fs::create_dir_all(&folder).expect("the folder");
            std::fs::write(folder.join(format!("{name}.toc")), "## Interface: 16001\n")
                .expect("its TOC");
            state
                .addons
                .insert(name.to_string(), entry(source, "1.0.0", &[name]));
        }
        engine.home.write_state(&state).expect("writes");

        let export = export(&engine).expect("exports");
        let names: Vec<&str> = export
            .list
            .addons
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, ["OneFolder"]);
        assert_eq!(
            export.warnings,
            [
                "Dev is left out of the list: its source link:D:\\dev\\Dev is a linked development folder",
                "Zipped is left out of the list: its source C:\\Downloads\\Zipped-1.0.0.zip is a zip file",
            ]
        );
        let path = Path::new("exported.json");
        assert_eq!(
            ListFile::parse(&export.list.to_json(), path).expect("what export writes reads back"),
            export.list
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
