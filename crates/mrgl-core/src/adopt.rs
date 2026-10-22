//! Adopt (task 097): what source each unmanaged addon of a flavor has. An addon folder the state sidecar does
//! not know is unmanaged (D-003); its TOCs may name where it comes from, in this order: `## X-Wago-ID`,
//! `## X-WoWI-ID`, `## X-Curse-Project-ID` (each read only when it has the id's shape, [`toc::is_wago_id`],
//! [`toc::is_number_id`]) and `## X-Website` on github.com ([`toc::github_repo_of_website`]; another website
//! is reported as a page, never as a source), then the curated GitHub mapping compiled in from
//! `data/github-sources.json` ([`GithubMapping`]), keyed by folder name. Each source is asked whether it
//! publishes a build for the flavor ([`Source::offers`], Forever included), and a source this run cannot ask
//! says what it lacks: a key reads `needs <KEY NAME>` ([`keys::name_of`]).
//!
//! The folders of one addon form one row, the folder group: the folders the chosen source names
//! ([`Source::folders`]: WoWInterface's `UIDir`), else the folders whose TOCs or mapping name the same source,
//! else the folder alone; only unmanaged folders that are not a junction join a group. The row's name is the
//! install engine's ([`install::derive_name`]).
//!
//! The report reads and never writes (the sources may write their own caches). Its counts measure the
//! roadmap's M4 goal: how many addons can be adopted without CurseForge, how many only with it, how many
//! have no known source (and how many have sources with no build for the flavor). The source `--apply`
//! writes is the first, in the order above, that this run can ask and that answers `yes`, else the
//! first that answers `unknown`; a CurseForge-only addon is never adopted before its source can be asked.
//!
//! `--apply` (task 098, [`apply`]) writes one sidecar entry per such addon: the spec, the TOC's version
//! (`unknown` without one), the folder group and the hashes of its files as they are, with no download and
//! no folder change, so the next update installs the source's build over it. A flavor folder with the
//! CurseForge app's `.curseclient` marker is refused without `--alongside-curseforge` ([`marker_refusal`]),
//! and every install path warns there ([`curse_client_warning`]); the marker is never touched.
//!
//! The summary, the marker line and the refusal name what adopts in the caller's words ([`AdoptBy`], task
//! 112): the command line's `adopt --apply` and `--alongside-curseforge`, or the window's Adopt button and
//! the confirmation of its dialog.
//!
//! The window applies the report it shows (task 113): the report records its sources' answers
//! ([`Answers`], through [`Answering`]), the card hands them back, and the apply and the report after it
//! are built over the folders and the sidecar as they are then and those answers, asking no source again
//! while the answers are younger than [`ANSWERS_MAX_AGE_SECONDS`]. The command line reads once and writes.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::flavor::{Flavor, FlavorId};
use crate::install;
use crate::keys;
use crate::scan::{self, Addon, Scan};
use crate::sources::{BuildOffer, Channel, Offered, Source, SourceKind, SourceSpec, Target};
use crate::state::{self, Entry, State};
use crate::toc;
use crate::voice::{Kind, count_noun};

/// The curated GitHub mapping as it is compiled in.
pub const MAPPING_JSON: &str = include_str!("../../../data/github-sources.json");
/// The schema of the mapping; the schema check test holds the file to it.
pub const MAPPING_SCHEMA: u32 = 1;

/// The curated GitHub mapping: an addon folder's name to the repository whose releases publish its zips.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubMapping {
    /// [`MAPPING_SCHEMA`].
    pub schema: u32,
    /// What the file is and how an entry joins, for a reader of the raw JSON.
    #[serde(default)]
    pub about: Option<String>,
    /// Folder name to `github:<owner>/<repo>`; every folder of a multi-folder addon is a key of its own.
    pub folders: BTreeMap<String, String>,
}

impl GithubMapping {
    /// Parses a mapping file.
    pub fn parse(text: &str) -> Result<GithubMapping, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    /// The source of a folder, ASCII case-insensitive, as the file writes it.
    pub fn source_of(&self, folder: &str) -> Option<&str> {
        self.folders
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(folder))
            .map(|(_, source)| source.as_str())
    }

    /// Everything wrong with the file, one line each; empty when it holds to the schema: the schema number,
    /// every key a folder name (no path separator, no surrounding space, unique without regard to case),
    /// every value a canonical `github:<owner>/<repo>` spec.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.schema != MAPPING_SCHEMA {
            problems.push(format!(
                "schema {} (expected {MAPPING_SCHEMA})",
                self.schema
            ));
        }
        let mut seen: Vec<String> = Vec::new();
        for (folder, source) in &self.folders {
            let name_ok = !folder.is_empty()
                && folder.trim() == folder
                && !folder.contains(['/', '\\', ':'])
                && folder != "."
                && folder != "..";
            if !name_ok {
                problems.push(format!("folder {folder:?}: not a folder name"));
            }
            let lower = folder.to_ascii_lowercase();
            if seen.contains(&lower) {
                problems.push(format!(
                    "folder {folder}: named twice without regard to case"
                ));
            }
            seen.push(lower);
            match SourceSpec::parse(source) {
                Ok(spec @ SourceSpec::GitHub { .. }) if spec.to_string() == *source => {}
                Ok(SourceSpec::GitHub { owner, repo }) => problems.push(format!(
                    "folder {folder}: {source} is not canonical (github:{owner}/{repo})"
                )),
                Ok(_) => problems.push(format!(
                    "folder {folder}: {source} is not a github:<owner>/<repo> spec"
                )),
                Err(error) => problems.push(format!("folder {folder}: {error}")),
            }
        }
        problems
    }
}

/// The mapping compiled into this build. The schema check test parses the same text, so an empty mapping
/// here (an unreadable file) cannot reach a release.
pub fn mapping() -> &'static GithubMapping {
    static MAPPING: OnceLock<GithubMapping> = OnceLock::new();
    MAPPING.get_or_init(|| GithubMapping::parse(MAPPING_JSON).unwrap_or_default())
}

/// Where a source of an unmanaged addon was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hint {
    /// `## X-Wago-ID`.
    WagoId,
    /// `## X-WoWI-ID`.
    WowiId,
    /// `## X-Curse-Project-ID`.
    CurseProjectId,
    /// `## X-Website` on github.com.
    Website,
    /// The curated GitHub mapping, by the folder's name.
    Mapping,
}

impl Hint {
    /// The hint as a line names it: the TOC header, or the mapping.
    pub const fn label(self) -> &'static str {
        match self {
            Hint::WagoId => "X-Wago-ID",
            Hint::WowiId => "X-WoWI-ID",
            Hint::CurseProjectId => "X-Curse-Project-ID",
            Hint::Website => "X-Website",
            Hint::Mapping => "the GitHub mapping",
        }
    }
}

/// One source of an unmanaged addon and what it says about the flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// The spec, canonical (`wago:<id>`, `wowi:<id>`, `curse:<id>`, `github:<owner>/<repo>`).
    pub spec: SourceSpec,
    /// The source the spec names.
    pub source: SourceKind,
    /// Where the spec was found.
    pub from: Hint,
    /// Whether the source publishes a build for the flavor.
    pub build: Offered,
    /// Why, in plain words.
    pub why: String,
    /// The key this run lacks to ask the source (`MRGL_CURSEFORGE_KEY`); `None` when it has it or the
    /// source needs none.
    pub needs: Option<&'static str>,
    /// What this run lacks to ask the source, as the `source not configured` clause ([`Source::missing`]);
    /// `None` when it can ask.
    pub missing: Option<String>,
    /// The managed addon whose source this spec is already: adopting it a second time is never offered.
    pub taken_by: Option<String>,
}

impl Candidate {
    /// Whether `--apply` can write this source: the run can ask it and no managed addon has it.
    pub fn usable(&self) -> bool {
        self.missing.is_none() && self.taken_by.is_none()
    }

    /// The source's line under its addon (two spaces in): the spec, where it was found, the build answer
    /// for the flavor and why, and what the run lacks (`needs MRGL_CURSEFORGE_KEY`).
    pub fn line(&self, flavor_name: &str) -> String {
        let mut line = format!("  {} from {}: ", self.spec, self.from.label());
        let lacks = self
            .needs
            .map(|key| format!("needs {key}"))
            .or_else(|| self.missing.clone());
        match (&lacks, self.build) {
            (Some(lacks), Offered::Unknown) => line.push_str(lacks),
            (lacks, build) => {
                line.push_str(&format!("{flavor_name} build {build} ({})", self.why));
                if let Some(lacks) = lacks {
                    line.push_str(&format!("; {lacks}"));
                }
            }
        }
        if let Some(name) = &self.taken_by {
            line.push_str(&format!("; the source of the managed {name} already"));
        }
        line
    }
}

/// What an unmanaged addon's sources add up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// A source other than CurseForge does not say `no`.
    WithoutCurseforge,
    /// Only CurseForge does not say `no`.
    OnlyCurseforge,
    /// Every source says it has no build for the flavor.
    NoBuild,
    /// No source at all (a page at most).
    NoSource,
}

/// One unmanaged addon: its folder group, its version and its sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Adoptable {
    /// The addon's name, the install engine's rule over its folders ([`install::derive_name`]).
    pub name: String,
    /// The folder group, the named folder first.
    pub folders: Vec<String>,
    /// The `## Version` of the named folder's TOC; `None` when it has none or an unsubstituted token.
    pub version: Option<String>,
    /// The sources, in the order of the hints, then the mapping.
    pub sources: Vec<Candidate>,
    /// The `## X-Website` pages that name no source.
    pub pages: Vec<String>,
    pub class: Class,
    /// The spec `--apply` writes for the addon; `None` when no source can be written in this run.
    pub adopt_as: Option<String>,
    /// The keys that would let the run write a source it cannot write now.
    pub needs: Vec<&'static str>,
}

impl Adoptable {
    /// The addon's line: name, version, folders when there are several, and what adopt makes of it.
    pub fn line(&self, flavor_name: &str) -> String {
        let mut line = format!(
            "{} {}",
            self.name,
            self.version.as_deref().unwrap_or("(no version)")
        );
        if self.folders.len() > 1 {
            line.push_str(&format!(
                " ({}: {})",
                count_noun(self.folders.len(), "folder"),
                self.folders.join(", ")
            ));
        }
        line.push_str(&format!(": {}", self.verdict(flavor_name)));
        line
    }

    /// What adopt makes of the addon: `adopt as <spec>`, what the run lacks, or why it has no source.
    pub fn verdict(&self, flavor_name: &str) -> String {
        let needs = (!self.needs.is_empty()).then(|| format!("needs {}", self.needs.join(" or ")));
        match (&self.adopt_as, self.class) {
            (Some(spec), _) => format!("adopt as {spec}"),
            (None, Class::WithoutCurseforge) => {
                needs.unwrap_or_else(|| "no source this run can write".to_string())
            }
            (None, Class::OnlyCurseforge) => match needs {
                Some(needs) => format!("only with CurseForge, {needs}"),
                None => "only with CurseForge".to_string(),
            },
            (None, Class::NoBuild) => format!("no {flavor_name} build at its sources"),
            (None, Class::NoSource) => "no known source".to_string(),
        }
    }

    /// The addon's lines under it: one per source, then one per page that names no source.
    pub fn detail_lines(&self, flavor_name: &str) -> Vec<String> {
        self.sources
            .iter()
            .map(|candidate| candidate.line(flavor_name))
            .chain(
                self.pages
                    .iter()
                    .map(|page| format!("  page {page} from X-Website, not a source mrgl reads")),
            )
            .collect()
    }
}

/// The counts of a report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Unmanaged addons (folder groups).
    pub addons: usize,
    pub without_curseforge: usize,
    pub only_curseforge: usize,
    pub no_build: usize,
    pub no_source: usize,
    /// The addons `--apply` would write in this run.
    pub adoptable_now: usize,
}

/// What adopts, as the summary, the marker line and the refusal name it (task 112, as
/// `savedvars::CleanupBy` names the way to clean up): the command line's flags, or the window's button and
/// the confirmation of its dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptBy {
    /// `mrgl adopt`: `adopt --apply`, and `--alongside-curseforge` in a folder the CurseForge app manages.
    Command,
    /// The window's toolbox: the Adopt button, and the confirmation of its dialog in such a folder.
    Window,
}

impl AdoptBy {
    /// What writes the entries.
    const fn apply(self) -> &'static str {
        match self {
            AdoptBy::Command => "adopt --apply",
            AdoptBy::Window => "the Adopt button",
        }
    }

    /// What lets it write them in a folder the CurseForge app manages.
    const fn alongside(self) -> &'static str {
        match self {
            AdoptBy::Command => "--alongside-curseforge",
            AdoptBy::Window => "the confirmation to adopt alongside the CurseForge app",
        }
    }
}

/// The adopt report of one flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub flavor: Flavor,
    /// The CurseForge app's `.curseclient` marker is in the flavor folder.
    pub curse_client: bool,
    /// The unmanaged addons, by name.
    pub addons: Vec<Adoptable>,
    /// Unmanaged folders left out: a junction or link mrgl did not make.
    pub left_out: Vec<String>,
    pub counts: Counts,
}

impl Report {
    /// The flavor as the lines name it: the friendly name (`forever`), else the id.
    pub fn flavor_name(&self) -> &str {
        flavor_name(&self.flavor)
    }

    /// The summary line of the report alone and its kind: [`Report::counts_line`] and how many addons
    /// `--apply` (or the window's Adopt button, [`AdoptBy`]) would adopt in this run.
    pub fn summary(&self, by: AdoptBy) -> (Kind, String) {
        let (kind, mut line) = self.counts_line();
        if self.counts.adoptable_now > 0 {
            line.push_str(&format!(
                "; {} adopts {} of them",
                by.apply(),
                self.counts.adoptable_now
            ));
        }
        (kind, line)
    }

    /// The summary line of an `--apply` run (task 098): [`Report::counts_line`] and what was written.
    pub fn applied_summary(&self, applied: &Applied) -> (Kind, String) {
        let (_, mut line) = self.counts_line();
        if applied.adopted.is_empty() {
            line.push_str("; nothing adopted, the sidecar is unchanged");
            return (Kind::Nothing, line);
        }
        line.push_str(&format!(
            "; adopted {} into the sidecar, no download and no folder changed: the next update installs each from its source",
            count_noun(applied.adopted.len(), "addon")
        ));
        (Kind::Success, line)
    }

    /// The counts line and its kind: the three counts of the M4 goal, and the fourth when an addon's
    /// sources have no build for the flavor.
    pub fn counts_line(&self) -> (Kind, String) {
        let folder = self.flavor.folder_name();
        let counts = &self.counts;
        if counts.addons == 0 {
            return (
                Kind::Nothing,
                format!("no unmanaged addons in {folder}: nothing to adopt"),
            );
        }
        let mut line = format!(
            "{} in {folder}: {} can be adopted without CurseForge, {} only with CurseForge, {} with no known source",
            count_noun(counts.addons, "unmanaged addon"),
            counts.without_curseforge,
            counts.only_curseforge,
            counts.no_source
        );
        if counts.no_build > 0 {
            line.push_str(&format!(
                ", {} with no {} build at their sources",
                counts.no_build,
                flavor_name(&self.flavor)
            ));
        }
        (Kind::Success, line)
    }

    /// The warning line of a flavor the CurseForge app manages, naming what adopts ([`AdoptBy`]); `None`
    /// without its marker.
    pub fn marker_line(&self, by: AdoptBy) -> Option<String> {
        self.curse_client.then(|| {
            format!(
                "the CurseForge app manages {} (its .curseclient marker is there): a folder belongs to one manager, so {} writes nothing there without {}",
                self.flavor.folder_name(),
                by.apply(),
                by.alongside()
            )
        })
    }

    /// The warning lines of the folders left out.
    pub fn left_out_lines(&self) -> Vec<String> {
        self.left_out
            .iter()
            .map(|folder| format!("{folder} is a junction or link mrgl did not make: left out"))
            .collect()
    }
}

/// One addon `--apply` wrote into the sidecar (task 098).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Adopted {
    pub name: String,
    /// The source spec of the entry.
    pub source: String,
    /// The TOC's version, or `unknown`.
    pub version: String,
    pub folders: Vec<String>,
    /// How many files the entry records with their hashes.
    pub files: usize,
}

impl Adopted {
    /// The addon's line: `adopted <name> <version> as <spec> (<n> folders: …)`.
    pub fn line(&self) -> String {
        format!(
            "adopted {} {} as {} ({}: {})",
            self.name,
            self.version,
            self.source,
            count_noun(self.folders.len(), "folder"),
            self.folders.join(", ")
        )
    }
}

/// An addon with a source that `--apply` did not write, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotAdopted {
    pub name: String,
    pub reason: String,
}

impl NotAdopted {
    /// The warning line: `<name> not adopted: <reason>`.
    pub fn line(&self) -> String {
        format!("{} not adopted: {}", self.name, self.reason)
    }
}

/// What `--apply` did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub adopted: Vec<Adopted>,
    pub skipped: Vec<NotAdopted>,
}

/// The entries of `--apply` (task 098), put into a state the caller then writes: one per row the report
/// adopts as a source, with that spec, the TOC's `## Version` (`unknown` when it has none), the folder
/// group and the SHA-256 of every file in it as it is now (`<Folder>/<path>`, a link inside not followed),
/// no pin, no mark, the stable channel. No download and no folder change: the next update finds the
/// version differ from the source's and installs the source's build over the folders, as for any update.
/// A row with a source the run cannot write (a missing key, CurseForge only) is skipped with its reason; a
/// row without a source is no line. A name or a folder the state knows already is skipped too, so a second
/// run over the same report and state changes nothing.
pub fn apply(report: &Report, state: &mut State) -> Applied {
    let addons_dir = scan::addons_dir(&report.flavor.folder);
    let flavor_name = report.flavor_name();
    let mut applied = Applied::default();
    for row in &report.addons {
        let skip = |reason: String| NotAdopted {
            name: row.name.clone(),
            reason,
        };
        let Some(spec) = &row.adopt_as else {
            if matches!(row.class, Class::WithoutCurseforge | Class::OnlyCurseforge) {
                applied.skipped.push(skip(row.verdict(flavor_name)));
            }
            continue;
        };
        if let Some((known, _)) = state.entry(&row.name) {
            applied
                .skipped
                .push(skip(format!("a managed addon is named {known} already")));
            continue;
        }
        if let Some((owner, _)) = row
            .folders
            .iter()
            .find_map(|folder| state.entry_for_folder(folder))
        {
            applied
                .skipped
                .push(skip(format!("{owner} manages one of its folders already")));
            continue;
        }
        let files = match hash_folders(&addons_dir, &row.folders) {
            Ok(files) => files,
            Err(error) => {
                applied
                    .skipped
                    .push(skip(format!("its files could not be read: {error}")));
                continue;
            }
        };
        let version = row
            .version
            .clone()
            .filter(|version| !version.trim().is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        applied.adopted.push(Adopted {
            name: row.name.clone(),
            source: spec.clone(),
            version: version.clone(),
            folders: row.folders.clone(),
            files: files.len(),
        });
        state.addons.insert(
            row.name.clone(),
            Entry {
                source: spec.clone(),
                version,
                previous_version: None,
                folders: row.folders.clone(),
                files,
                installed_at: state::now_rfc3339(),
                nolib: None,
                pinned: None,
                ignored: false,
                channel: Channel::Stable,
                forever_build: None,
            },
        );
    }
    applied
}

/// Every file of the folders under an AddOns folder as `<Folder>/<path>` with `/` separators and its
/// SHA-256, as the install engine records them; a link inside a folder is not followed.
fn hash_folders(addons_dir: &Path, folders: &[String]) -> io::Result<BTreeMap<String, String>> {
    fn walk(dir: &Path, prefix: &str, files: &mut BTreeMap<String, String>) -> io::Result<()> {
        let mut entries: Vec<std::fs::DirEntry> =
            std::fs::read_dir(dir)?.collect::<io::Result<_>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = entry.file_type()?;
            let relative = format!("{prefix}/{name}");
            if kind.is_dir() {
                walk(&entry.path(), &relative, files)?;
            } else if kind.is_file() {
                files.insert(relative, install::sha256_of(&entry.path())?);
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    for folder in folders {
        walk(&addons_dir.join(folder), folder, &mut files)?;
    }
    Ok(files)
}

/// The refusal of `--apply` in a folder the CurseForge app manages, without `--alongside-curseforge`
/// (task 098), naming what adopts ([`AdoptBy`]).
pub fn marker_refusal(flavor: &Flavor, by: AdoptBy) -> String {
    format!(
        "the CurseForge app manages {} (its .curseclient marker is there) and a folder belongs to one manager: {} wrote nothing; {} adopts its addons anyway, and the CurseForge app may still update those folders",
        flavor.folder_name(),
        by.apply(),
        by.alongside()
    )
}

/// The warning of `--apply --alongside-curseforge` in a folder the CurseForge app manages (task 098).
pub fn alongside_warning(flavor: &Flavor) -> String {
    format!(
        "adopting alongside the CurseForge app in {}: it may still update or flag the adopted folders, and its .curseclient marker stays where it is",
        flavor.folder_name()
    )
}

/// The warning of an install path (`add`, `import`, `update`) in a folder the CurseForge app manages
/// (task 098); `None` without its marker.
pub fn curse_client_warning(flavor: &Flavor) -> Option<String> {
    flavor.curse_client.then(|| {
        format!(
            "the CurseForge app manages {} too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager",
            flavor.folder_name()
        )
    })
}

/// The flavor as a line names it: the friendly name, else the id.
fn flavor_name(flavor: &Flavor) -> &str {
    flavor.id.friendly_name().unwrap_or(flavor.id.as_str())
}

/// One unmanaged folder with what its TOCs and the mapping name.
struct FolderHints<'a> {
    addon: &'a Addon,
    /// The sources, in the hint order, each once.
    specs: Vec<(SourceSpec, Hint)>,
    /// The `## X-Website` pages that name no source.
    pages: Vec<String>,
}

impl FolderHints<'_> {
    /// Whether the folder's TOCs or the mapping name a spec.
    fn names(&self, spec: &str) -> bool {
        self.specs
            .iter()
            .any(|(known, _)| known.to_string().eq_ignore_ascii_case(spec))
    }
}

/// The sources one folder's TOCs and the mapping name, in the hint order, each once, and the pages of
/// its `## X-Website` lines that name no source.
fn hints_of<'a>(addon: &'a Addon, mapping: &GithubMapping) -> FolderHints<'a> {
    let mut tocs: Vec<&toc::Toc> = addon
        .primary_toc()
        .map(|file| &file.toc)
        .into_iter()
        .collect();
    for file in &addon.tocs {
        if !tocs.iter().any(|known| std::ptr::eq(*known, &file.toc)) {
            tocs.push(&file.toc);
        }
    }
    let mut specs: Vec<(SourceSpec, Hint)> = Vec::new();
    let mut pages: Vec<String> = Vec::new();
    let push = |text: String, hint: Hint, specs: &mut Vec<(SourceSpec, Hint)>| {
        if let Ok(spec) = SourceSpec::parse(&text)
            && !specs
                .iter()
                .any(|(known, _)| known.to_string().eq_ignore_ascii_case(&spec.to_string()))
        {
            specs.push((spec, hint));
        }
    };
    for toc in &tocs {
        if let Some(id) = toc.wago_id_checked() {
            push(format!("wago:{id}"), Hint::WagoId, &mut specs);
        }
    }
    for toc in &tocs {
        if let Some(id) = toc.wowi_id_checked() {
            push(format!("wowi:{id}"), Hint::WowiId, &mut specs);
        }
    }
    for toc in &tocs {
        if let Some(id) = toc.curse_project_id_checked() {
            push(format!("curse:{id}"), Hint::CurseProjectId, &mut specs);
        }
    }
    for toc in &tocs {
        let Some(website) = toc.website.as_deref().map(str::trim) else {
            continue;
        };
        match toc::github_repo_of_website(website) {
            Some(repo) => push(format!("github:{repo}"), Hint::Website, &mut specs),
            None if !website.is_empty() && !pages.iter().any(|page| page == website) => {
                pages.push(website.to_string())
            }
            None => {}
        }
    }
    if let Some(source) = mapping.source_of(&addon.folder) {
        push(source.to_string(), Hint::Mapping, &mut specs);
    }
    FolderHints {
        addon,
        specs,
        pages,
    }
}

/// The adopt report of a scanned flavor against its sidecar (task 097): every unmanaged addon with its
/// folder group, its sources and their answers for the flavor, through the router (or any [`Source`]) and
/// the compiled-in mapping. `has_key` says whether this run has a source's key, which tells a missing key
/// (`needs <KEY NAME>`) from a source this build does not have.
pub fn report(
    scan: &Scan,
    state: &State,
    sources: &dyn Source,
    has_key: &dyn Fn(SourceKind) -> bool,
) -> Report {
    report_with(scan, state, sources, has_key, mapping())
}

/// [`report`] with a mapping of the caller's (the tests' synthetic one).
pub fn report_with(
    scan: &Scan,
    state: &State,
    sources: &dyn Source,
    has_key: &dyn Fn(SourceKind) -> bool,
    mapping: &GithubMapping,
) -> Report {
    let target = Target::from_flavor(&scan.flavor);
    let unmanaged: Vec<&Addon> = scan
        .addons
        .iter()
        .filter(|addon| state.entry_for_folder(&addon.folder).is_none())
        .collect();
    let left_out: Vec<String> = unmanaged
        .iter()
        .filter(|addon| addon.linked)
        .map(|addon| addon.folder.clone())
        .collect();
    let folders: Vec<FolderHints> = unmanaged
        .into_iter()
        .filter(|addon| !addon.linked)
        .map(|addon| hints_of(addon, mapping))
        .collect();

    let mut offers: BTreeMap<String, BuildOffer> = BTreeMap::new();
    let mut candidate = |spec: &SourceSpec, hint: Hint| -> Candidate {
        let key = spec.to_string();
        let offer = offers
            .entry(key.clone())
            .or_insert_with(|| sources.offers(spec, &target))
            .clone();
        let missing = sources.missing(spec);
        let needs = missing
            .as_ref()
            .and_then(|_| keys::name_of(spec.kind()))
            .filter(|_| !has_key(spec.kind()));
        let taken_by = state
            .addons
            .iter()
            .find(|(_, entry)| entry.source.eq_ignore_ascii_case(&key))
            .map(|(name, _)| name.clone());
        Candidate {
            spec: spec.clone(),
            source: spec.kind(),
            from: hint,
            build: offer.build,
            why: offer.why,
            needs,
            missing,
            taken_by,
        }
    };

    let mut grouped: Vec<String> = Vec::new();
    let is_grouped = |grouped: &Vec<String>, folder: &str| {
        grouped
            .iter()
            .any(|known| known.eq_ignore_ascii_case(folder))
    };
    let mut rows: Vec<Adoptable> = Vec::new();
    // The folders with a source first, so a folder without one joins the group a source names before it is
    // a row of its own.
    for hinted in folders.iter().filter(|folder| !folder.specs.is_empty()) {
        let addon = hinted.addon;
        if is_grouped(&grouped, &addon.folder) {
            continue;
        }
        let candidates: Vec<Candidate> = hinted
            .specs
            .iter()
            .map(|(spec, hint)| candidate(spec, *hint))
            .collect();
        let choose = |want: Offered| {
            candidates
                .iter()
                .position(|candidate| candidate.usable() && candidate.build == want)
        };
        let chosen = choose(Offered::Yes).or_else(|| choose(Offered::Unknown));
        let group_spec = chosen
            .or_else(|| {
                candidates
                    .iter()
                    .position(|candidate| candidate.build != Offered::No)
            })
            .unwrap_or(0);
        let group_spec = &candidates[group_spec].spec;
        let joinable = |folder: &str| {
            !folder.eq_ignore_ascii_case(&addon.folder) && !is_grouped(&grouped, folder)
        };
        let mut group: Vec<String> = vec![addon.folder.clone()];
        match sources.folders(group_spec) {
            Some(named)
                if named
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&addon.folder)) =>
            {
                for other in &folders {
                    let folder = &other.addon.folder;
                    if joinable(folder)
                        && named.iter().any(|name| name.eq_ignore_ascii_case(folder))
                    {
                        group.push(folder.clone());
                    }
                }
            }
            _ => {
                let key = group_spec.to_string();
                for other in &folders {
                    if joinable(&other.addon.folder) && other.names(&key) {
                        group.push(other.addon.folder.clone());
                    }
                }
            }
        }
        grouped.extend(group.iter().cloned());
        let mut hints: Vec<String> = Vec::new();
        if let Some(hint) = group_spec.name_hint() {
            hints.push(hint.to_string());
        }
        hints.push(addon.folder.clone());
        let name = {
            let names: Vec<&str> = group.iter().map(String::as_str).collect();
            install::derive_name(&names, &hints).to_string()
        };
        group.sort_by_key(|folder| {
            (
                !folder.eq_ignore_ascii_case(&name),
                folder.to_ascii_lowercase(),
            )
        });
        let named = folders
            .iter()
            .find(|other| other.addon.folder.eq_ignore_ascii_case(&name))
            .map_or(addon, |other| other.addon);
        let mut row_pages = hinted.pages.clone();
        for other in &folders {
            if group
                .iter()
                .any(|folder| folder.eq_ignore_ascii_case(&other.addon.folder))
            {
                for page in &other.pages {
                    if !row_pages.contains(page) {
                        row_pages.push(page.clone());
                    }
                }
            }
        }
        rows.push(row(
            name,
            group,
            named.version().map(str::to_string),
            candidates,
            row_pages,
            chosen,
        ));
    }
    for alone in &folders {
        let addon = alone.addon;
        if is_grouped(&grouped, &addon.folder) {
            continue;
        }
        grouped.push(addon.folder.clone());
        rows.push(row(
            addon.folder.clone(),
            vec![addon.folder.clone()],
            addon.version().map(str::to_string),
            Vec::new(),
            alone.pages.clone(),
            None,
        ));
    }
    rows.sort_by_key(|row| row.name.to_ascii_lowercase());

    let mut counts = Counts {
        addons: rows.len(),
        ..Counts::default()
    };
    for row in &rows {
        match row.class {
            Class::WithoutCurseforge => counts.without_curseforge += 1,
            Class::OnlyCurseforge => counts.only_curseforge += 1,
            Class::NoBuild => counts.no_build += 1,
            Class::NoSource => counts.no_source += 1,
        }
        if row.adopt_as.is_some() {
            counts.adoptable_now += 1;
        }
    }
    Report {
        flavor: scan.flavor.clone(),
        curse_client: scan.flavor.curse_client,
        addons: rows,
        left_out,
        counts,
    }
}

/// One row from its parts: the class from the sources' answers, the spec `--apply` writes, and the keys
/// that would let it write one when it cannot.
fn row(
    name: String,
    folders: Vec<String>,
    version: Option<String>,
    sources: Vec<Candidate>,
    pages: Vec<String>,
    chosen: Option<usize>,
) -> Adoptable {
    let offers = |candidate: &&Candidate| candidate.build != Offered::No;
    let class = if sources
        .iter()
        .filter(offers)
        .any(|candidate| candidate.source != SourceKind::CurseForge)
    {
        Class::WithoutCurseforge
    } else if sources.iter().any(|candidate| offers(&candidate)) {
        Class::OnlyCurseforge
    } else if !sources.is_empty() {
        Class::NoBuild
    } else {
        Class::NoSource
    };
    let adopt_as = chosen.map(|index| sources[index].spec.to_string());
    let mut needs: Vec<&'static str> = Vec::new();
    if adopt_as.is_none() {
        for key in sources
            .iter()
            .filter(offers)
            .filter_map(|candidate| candidate.needs)
        {
            if !needs.contains(&key) {
                needs.push(key);
            }
        }
    }
    Adoptable {
        name,
        folders,
        version,
        sources,
        pages,
        class,
        adopt_as,
        needs,
    }
}

/// How long the sources' answers of a shown report stand for the apply that follows it (task 113): an
/// hour, the shortest cache rule of the sources (WoWInterface's project answer, `Cache-Control:
/// max-age=3600`; its file list and Wago's slug lists keep a day, GitHub asks again with its `ETag`). An
/// older report is asked again first.
pub const ANSWERS_MAX_AGE_SECONDS: i64 = 3_600;

/// One source's answer to whether it publishes a build for the flavor, as a report met it (task 113).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub build: Offered,
    pub why: String,
    /// What the run lacked to ask the source then ([`Source::missing`]); when that changed since (a key set
    /// or removed), the source is asked again.
    pub missing: Option<String>,
}

/// The sources' answers a report was built from (task 113): per spec its build answer and the folders it
/// names, for one flavor, with when the asking began. The window's card carries them from the report it
/// shows to the apply, so the apply and the report after it ask no source again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answers {
    /// The `.flavor.info` id the answers are for; answers of another flavor are asked again.
    pub flavor: String,
    /// When the first source was asked, RFC 3339 in UTC.
    pub asked_at: String,
    /// Per canonical spec, its answer ([`Source::offers`]).
    pub offers: BTreeMap<String, Answer>,
    /// Per canonical spec, the folders it names ([`Source::folders`]), `null` for none.
    pub folders: BTreeMap<String, Option<Vec<String>>>,
}

impl Answers {
    /// No answer yet, for a flavor, asked from now on.
    pub fn new(flavor: &FlavorId) -> Answers {
        Answers {
            flavor: flavor.as_str().to_string(),
            asked_at: state::now_rfc3339(),
            ..Answers::default()
        }
    }

    /// Whether the answers stand for an apply in a flavor now: the flavor's own, asked less than
    /// [`ANSWERS_MAX_AGE_SECONDS`] ago (a time ahead of the clock, or one that does not read, does not).
    pub fn stand_for(&self, flavor: &FlavorId) -> bool {
        self.stand_for_at(flavor, &state::now_rfc3339())
    }

    /// [`Answers::stand_for`] at a given time.
    pub fn stand_for_at(&self, flavor: &FlavorId, now: &str) -> bool {
        let age = crate::sources::unix_from_rfc3339(now)
            .zip(crate::sources::unix_from_rfc3339(&self.asked_at))
            .map(|(now, then)| now - then);
        self.flavor == flavor.as_str()
            && age.is_some_and(|age| (0..ANSWERS_MAX_AGE_SECONDS).contains(&age))
    }
}

/// A source that answers [`Source::offers`] and [`Source::folders`] from the answers it holds and asks
/// the source under it for the rest, holding what it gets (task 113); the rest it hands down. Built for
/// [`report`]: a report through it asks each spec at most once over the answers' life.
pub struct Answering<'a> {
    inner: &'a dyn Source,
    answers: std::cell::RefCell<Answers>,
}

impl<'a> Answering<'a> {
    pub fn new(inner: &'a dyn Source, answers: Answers) -> Answering<'a> {
        Answering {
            inner,
            answers: std::cell::RefCell::new(answers),
        }
    }

    /// The answers held, those carried in and those asked.
    pub fn into_answers(self) -> Answers {
        self.answers.into_inner()
    }
}

impl Source for Answering<'_> {
    fn resolve(
        &self,
        spec: &SourceSpec,
        target: &Target,
    ) -> crate::error::Result<crate::sources::Pick> {
        self.inner.resolve(spec, target)
    }

    fn fetch(&self, pick: &crate::sources::Pick, to: &Path) -> crate::error::Result<()> {
        self.inner.fetch(pick, to)
    }

    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        self.inner.missing(spec)
    }

    fn offers(&self, spec: &SourceSpec, target: &Target) -> BuildOffer {
        let key = spec.to_string();
        let missing = self.inner.missing(spec);
        if let Some(held) = self.answers.borrow().offers.get(&key)
            && held.missing == missing
        {
            return BuildOffer::new(held.build, held.why.clone());
        }
        let offer = self.inner.offers(spec, target);
        self.answers.borrow_mut().offers.insert(
            key,
            Answer {
                build: offer.build,
                why: offer.why.clone(),
                missing,
            },
        );
        offer
    }

    fn folders(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        let key = spec.to_string();
        if let Some(held) = self.answers.borrow().folders.get(&key) {
            return held.clone();
        }
        let folders = self.inner.folders(spec);
        self.answers
            .borrow_mut()
            .folders
            .insert(key, folders.clone());
        folders
    }
}

/// [`report`] over held answers (task 113): the sources are asked only for what `answers` does not hold,
/// and the answers come back with what was asked added, for the next report of the same card.
pub fn report_answering(
    scan: &Scan,
    state: &State,
    sources: &dyn Source,
    has_key: &dyn Fn(SourceKind) -> bool,
    answers: Answers,
) -> (Report, Answers) {
    let answering = Answering::new(sources, answers);
    let report = report(scan, state, &answering, has_key);
    (report, answering.into_answers())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_stand_for_their_flavor_for_an_hour() {
        let forever = FlavorId::Forever;
        let answers = Answers {
            flavor: "wowf".to_string(),
            asked_at: "2026-10-22T10:00:00Z".to_string(),
            ..Answers::default()
        };
        assert!(answers.stand_for_at(&forever, "2026-10-22T10:00:00Z"));
        assert!(answers.stand_for_at(&forever, "2026-10-22T10:59:59Z"));
        assert!(
            !answers.stand_for_at(&forever, "2026-10-22T11:00:00Z"),
            "an hour old"
        );
        assert!(
            !answers.stand_for_at(&forever, "2026-10-22T09:59:59Z"),
            "ahead of the clock"
        );
        assert!(
            !answers.stand_for_at(&FlavorId::Retail, "2026-10-22T10:00:00Z"),
            "another flavor"
        );
        let unread = Answers {
            asked_at: "yesterday".to_string(),
            ..answers.clone()
        };
        assert!(!unread.stand_for_at(&forever, "2026-10-22T10:00:00Z"));
        assert!(Answers::new(&forever).stand_for(&forever));
    }

    #[test]
    fn the_compiled_in_mapping_holds_to_the_schema() {
        let parsed = GithubMapping::parse(MAPPING_JSON).expect("data/github-sources.json parses");
        assert_eq!(&parsed, mapping());
        assert_eq!(parsed.problems(), Vec::<String>::new());
        assert!(
            parsed
                .about
                .as_deref()
                .is_some_and(|about| about.contains("pull request")),
            "the about line says how an entry joins"
        );
        // Seeded from the Forever starter list: every folder of it maps to its source.
        let starter: serde_json::Value =
            serde_json::from_str(include_str!("../../../lists/forever-starter.json"))
                .expect("the starter list parses");
        for addon in starter["addons"].as_array().expect("addons") {
            let source = addon["source"].as_str().expect("source");
            for folder in addon["folders"].as_array().expect("folders") {
                let folder = folder.as_str().expect("folder");
                assert_eq!(parsed.source_of(folder), Some(source), "{folder}");
            }
        }
    }

    #[test]
    fn the_mapping_check_names_every_problem() {
        let bad = r#"{
            "schema": 2,
            "folders": {
                "Good": "github:Owner/Repo",
                "good": "github:Owner/Repo",
                "Some/Path": "github:Owner/Repo",
                "Wago": "wago:L6Jb2R6v",
                "Odd": "https://github.com/Owner/Repo",
                "Broken": "github:Owner"
            }
        }"#;
        let problems = GithubMapping::parse(bad).expect("parses").problems();
        assert_eq!(problems.len(), 6, "{problems:#?}");
        assert!(problems[0].starts_with("schema 2"));
        assert!(GithubMapping::parse(r#"{"schema": 1, "folders": {}, "extra": 1}"#).is_err());
    }

    #[test]
    fn a_folder_finds_its_source_without_regard_to_case() {
        let mapping = GithubMapping::parse(
            r#"{"schema": 1, "folders": {"Questie": "github:Questie/Questie"}}"#,
        )
        .expect("parses");
        assert_eq!(mapping.source_of("questie"), Some("github:Questie/Questie"));
        assert_eq!(mapping.source_of("QuestieDB"), None);
    }
}
