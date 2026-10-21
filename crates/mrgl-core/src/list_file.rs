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
//! resolves every `github:` entry through the source and installs the current release through the one
//! install path of the engine ([`Engine::install_from_source`], task 055);
//! the version in the file is what was installed at the export, reported next to what the import brings.
//! A zip in hand is reported as not importable (the zip is on the other machine), a linked folder (`link:`)
//! is skipped, and an addon the flavor has already (by name, by source, or a folder of the entry that is
//! there) is reported and left alone at whatever version it is.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::install::{Engine, PickInstalled, SourceInstalled, SourceRequest, Wanted};
use crate::interface::{self, InterfaceMismatch};
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

/// What kind of source a list entry names, as far as the import is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntrySource {
    /// `github:<owner>/<repo>`: resolved and installed.
    GitHub(SourceSpec),
    /// A zip in hand: its path is on the machine that exported the list.
    Zip,
    /// A linked development folder (`link:…`, task 005): skipped.
    Linked,
    /// A source the import does not handle yet (`wago:`, `wowi:`, `curse:`).
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
            return EntrySource::GitHub(spec);
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
/// deleted by hand is left out (the sidecar itself is not written).
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
    Ok(Export {
        list: ListFile::from_state(&state, Some(state::now_rfc3339())),
        warnings,
    })
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
    /// A zip in hand or a source the import does not handle.
    NotImportable,
    /// A linked folder.
    Skipped,
    /// The resolve, the download or the install failed; the detail says why.
    Failed,
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
/// ([`Engine::install_from_source`]); `dry_run` resolves and reports ([`Engine::resolve`]), and writes
/// nothing under `Interface\AddOns` or in the sidecar. An entry that fails is reported as failed and the
/// import goes on; only a sidecar or AddOns folder that cannot be read at all is an error.
pub fn import(
    engine: &Engine,
    source: &dyn Source,
    list: &ListFile,
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
            EntrySource::GitHub(spec) => Some(spec),
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
                "a source the import does not handle yet"
            };
            items.push(item(Outcome::NotImportable, None, detail.to_string()));
            continue;
        };
        // The one install path of task 055, under the list's name for the addon.
        let request = SourceRequest {
            spec: spec.clone(),
            wanted: Wanted::Current,
            name: Some(entry.name.clone()),
        };
        if dry_run {
            items.push(match engine.resolve(source, &request) {
                Ok(pick) => item(
                    Outcome::WouldInstall,
                    Some(pick.version.clone()),
                    pick.summary(),
                ),
                Err(error) => item(Outcome::Failed, None, error.to_string()),
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
                done.warnings = installed.warnings;
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
            Err(error) => items.push(item(Outcome::Failed, None, error.to_string())),
        }
    }

    if items.iter().any(|item| item.outcome == Outcome::Installed) {
        let after = scan::scan_flavor(&engine.flavor)?;
        for item in items
            .iter_mut()
            .filter(|item| item.outcome == Outcome::Installed)
        {
            item.interface_mismatch = item.folders.first().and_then(|primary| {
                after
                    .addons
                    .iter()
                    .find(|addon| addon.folder.eq_ignore_ascii_case(primary))
                    .and_then(|addon| interface::check_addon(addon, engine.flavor.version))
            });
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
            EntrySource::GitHub(SourceSpec::parse("github:Questie/Questie").expect("a spec"))
        );
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
        assert_eq!(EntrySource::of("wago:abc123"), EntrySource::Unsupported);
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

        let dry = import(&engine, &source, &list, true).expect("a dry run");
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

        let report = import(&engine, &source, &list, false).expect("imports");
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

        let again = import(&engine, &source, &list, false).expect("imports again");
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
        let report = import(&engine, &source, &list, false).expect("imports");
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
}
