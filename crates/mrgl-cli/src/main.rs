//! The `mrgl` command line. Every human-readable line goes through the voice (D-005): a murloc line and a
//! plain English line, `--serious` or `MRGL_SERIOUS=1` for the plain line alone, `--json` for machines.
//! Exit codes: 0 success, 1 an error the plain line explains, 2 a usage error (clap's own).
//!
//! `scan`, `list` and `root` are task 002, the game root discovery behind them is task 020;
//! `add github:<owner>/<repo>` resolves a release (task 003); `add <file.zip>` and `remove` are task 004
//! (the install engine); `backup` (the WTF backup, the listing and the restore) is task 013; the Forever
//! notes on `add` and on `list` (the derived build signal and the curated note) are task 011; `link` and
//! `unlink` (the dev addon junctions) are task 005; the other subcommands follow the M1 plan
//! (`tasks/roadmap.md`) and answer "not implemented yet" with their task id until that task lands, so the
//! help text and the voice are testable from the first commit.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::dependencies::{self, AddonDependencies};
use mrgl_core::discover::{self, GameRoot, ROOT_ENV, RootOrigin};
use mrgl_core::flavor::{self, FlavorId, VersionSource};
use mrgl_core::forever::{ForeverBuild, ForeverStatus, STALE_AFTER_DAYS};
use mrgl_core::home::{self, Config};
use mrgl_core::install::{Engine, Metadata};
use mrgl_core::link;
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::{self, Source, SourceSpec, Target};
use mrgl_core::state::FlavorHome;
use mrgl_core::voice::{Kind, Voice};
use mrgl_core::{Error, VERSION};

#[derive(Parser, Debug)]
#[command(
    name = "mrgl",
    about = "A flavor-correct World of Warcraft addon manager, Forever-first.",
    disable_version_flag = true
)]
struct Cli {
    /// Print the version and exit.
    #[arg(short = 'V', long)]
    version: bool,

    /// Plain English only: drop the murloc line (also MRGL_SERIOUS=1).
    #[arg(long, global = true)]
    serious: bool,

    /// Machine-readable output: plain JSON, no murloc, no prose.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read a game flavor folder and list the installed addons with their TOC data.
    Scan {
        /// The flavor folder, for example `C:\Program Files (x86)\World of Warcraft\_forever_`.
        folder: PathBuf,
    },
    /// List the addons installed in a flavor, with their sources and versions where known.
    List {
        /// The flavor: `forever`, `retail`, `classic` or `era`, a `.flavor.info` id (`wowf`, `wow`,
        /// `wow_classic`, `wow_classic_era`) or a flavor folder.
        #[arg(long)]
        flavor: Option<String>,
        /// The World of Warcraft folder (the one with `_retail_`, `_forever_` …) for a flavor named by
        /// name or id; otherwise `MRGL_ROOT`, then the folder `mrgl root` remembered, then the
        /// launcher's registry key and the usual install folders.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Remember the World of Warcraft folder (the one with `_retail_`, `_forever_` …), or print the one a
    /// run would use and where it comes from.
    Root {
        /// The game folder to remember; without it, the one a run would use is printed: `MRGL_ROOT`, the
        /// remembered one, or the one found in the launcher's registry key or the usual install folders
        /// (found, not remembered).
        folder: Option<PathBuf>,
    },
    /// Install an addon from a source: the path of a zip in hand (installed), or `github:<owner>/<repo>`
    /// (task 003 resolves the release; the install from the pick is task 040), later `wago:`, `wowi:`,
    /// `curse:`.
    Add {
        /// The source spec (`github:<owner>/<repo>`), or a zip path (a `.zip` name or an existing file).
        source: String,
        /// The flavor to install into: `forever`, `retail`, `classic`, `era`, a `.flavor.info` id or a
        /// flavor folder. The installed build is read from the game folder (`MRGL_ROOT` or `mrgl root`)
        /// when there is one, which decides the game type of the progressing classic.
        #[arg(long)]
        flavor: Option<String>,
        /// Resolve the release and report the pick, write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check the installed addons against their sources and update them.
    Update {
        /// Every addon of the flavor, not one by name.
        #[arg(long)]
        all: bool,
        /// One addon by name.
        name: Option<String>,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Resolve and report, write nothing.
        #[arg(long)]
        dry_run: bool,
        /// Skip the confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Remove an installed addon's folders and forget it.
    Remove {
        /// The addon by name.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Link the addons of a development folder into a flavor with junctions (no admin rights needed).
    Link {
        /// A folder whose subfolders are addons (each with its TOC).
        folder: PathBuf,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Remove the junctions `link` made.
    Unlink {
        /// The development folder `link` was given.
        folder: PathBuf,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Back up a flavor's WTF folder into a dated zip, list the backups, or restore one (the toolbox).
    Backup {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// The backups folder: where a new backup goes, what `--list` lists and where `--restore` looks a
        /// bare name up; by default `backups\<flavor-id>` under the mrgl home (%LOCALAPPDATA%\mrgl).
        #[arg(long, value_name = "FOLDER")]
        to: Option<PathBuf>,
        /// List the backups instead of making one.
        #[arg(long, conflicts_with = "restore")]
        list: bool,
        /// Restore this backup (a zip path, or a name from `--list`) as the flavor's WTF folder; the
        /// current WTF is moved aside into the backups folder first. Refused while the client runs.
        #[arg(long, value_name = "ZIP")]
        restore: Option<PathBuf>,
        /// Skip the restore's confirmation (scripts; required with --json).
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Print a diagnostic report for a bug report: the game root, each flavor's build, AddOns folder,
    /// sidecar, cache and client, the WebView2 version, the GitHub rate limit and which API keys are set
    /// (never their values); paths under your profile folder show as `~`.
    Doctor {
        /// Report one flavor only: `forever`, `retail`, `classic`, `era` or a `.flavor.info` id.
        #[arg(long)]
        flavor: Option<String>,
        /// The World of Warcraft folder; otherwise the one every command resolves (`MRGL_ROOT`, the
        /// remembered one, the launcher's registry key, the usual install folders).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

impl Command {
    /// The "not yet" answer of a command whose task has not landed: its name and the task.
    fn not_yet(&self) -> Option<Error> {
        let (command, task) = match self {
            Command::Scan { .. }
            | Command::List { .. }
            | Command::Root { .. }
            | Command::Add { .. }
            | Command::Remove { .. }
            | Command::Link { .. }
            | Command::Unlink { .. }
            | Command::Backup { .. } => return None,
            Command::Doctor { .. } => return None,
            Command::Update { all: true, .. } => ("update --all", "006"),
            Command::Update { .. } => ("update", "006"),
        };
        Some(Error::NotImplemented { command, task })
    }
}

/// Prints the two-line messages, the plain data lines, or JSON when asked.
struct Reporter {
    voice: Voice,
    json: bool,
}

impl Reporter {
    fn say(&self, kind: Kind, plain: impl std::fmt::Display) {
        if self.json {
            let value = serde_json::json!({ "kind": kind.as_str(), "message": plain.to_string() });
            println!("{value}");
        } else {
            println!("{}", self.voice.say(kind, plain));
        }
    }

    fn fail(&self, plain: impl std::fmt::Display) {
        if self.json {
            let value =
                serde_json::json!({ "kind": Kind::Error.as_str(), "message": plain.to_string() });
            eprintln!("{value}");
        } else {
            eprintln!("{}", self.voice.say(Kind::Error, plain));
        }
    }

    /// A plain data line (one addon of a listing); human mode only, the JSON document carries the data.
    fn line(&self, text: impl std::fmt::Display) {
        println!("{text}");
    }

    /// The one JSON document of a command; its `kind` is a fixed name.
    fn document(&self, value: serde_json::Value) {
        debug_assert!(value.get("kind").is_some(), "a document names its kind");
        println!("{value}");
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let reporter = Reporter {
        voice: Voice::from_env(cli.serious),
        json: cli.json,
    };

    if cli.version {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({ "name": "mrgl", "version": VERSION })
            );
        } else {
            reporter.say(Kind::Banner, format!("mrgl {VERSION}"));
        }
        return ExitCode::SUCCESS;
    }

    let Some(command) = cli.command else {
        reporter.say(Kind::Banner, format!("mrgl {VERSION}: try `mrgl --help`"));
        return ExitCode::SUCCESS;
    };

    match run(command, &reporter) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            reporter.fail(message);
            ExitCode::from(1)
        }
    }
}

/// Runs a command; the error is the plain line of the failure.
fn run(command: Command, reporter: &Reporter) -> Result<(), String> {
    if let Some(not_yet) = command.not_yet() {
        return Err(not_yet.to_string());
    }
    match command {
        Command::Scan { folder } => {
            let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
            report_scan(reporter, "scan", &scan, Vec::new(), &[], None);
        }
        Command::List { flavor, root } => {
            let mut warnings = Vec::new();
            let folder = flavor_folder(flavor.as_deref(), root, &mut warnings)?;
            let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
            let dependencies = check_dependencies(&scan, &mut warnings);
            let forever = forever_statuses(&scan, &mut warnings);
            report_scan(
                reporter,
                "list",
                &scan,
                warnings,
                &dependencies,
                forever.as_ref(),
            );
        }
        Command::Root { folder } => root_command(reporter, folder)?,
        Command::Add {
            source,
            flavor,
            dry_run,
        } if is_zip_in_hand(&source) => add_zip(reporter, &source, flavor.as_deref(), dry_run)?,
        Command::Add {
            source,
            flavor,
            dry_run,
        } => add_command(reporter, &source, flavor.as_deref(), dry_run)?,
        Command::Remove { name, flavor } => remove_command(reporter, &name, flavor.as_deref())?,
        Command::Backup {
            flavor,
            to,
            list,
            restore,
            yes,
        } => backup_command(reporter, flavor.as_deref(), to, list, restore, yes)?,
        Command::Doctor { flavor, root } => doctor_command(reporter, flavor.as_deref(), root)?,
        Command::Link { folder, flavor } => link_command(reporter, &folder, flavor.as_deref())?,
        Command::Unlink { folder, flavor } => unlink_command(reporter, &folder, flavor.as_deref())?,
        _ => unreachable!("every other command answered with its task"),
    }
    Ok(())
}

/// The flavor folder a `--flavor` spec names: a friendly name or an id under the game root, or a folder.
fn flavor_folder(
    spec: Option<&str>,
    root_flag: Option<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<PathBuf, String> {
    let Some(spec) = spec else {
        return Err(
            "no flavor given: --flavor forever, retail, classic, era, a .flavor.info id or a flavor folder"
                .to_string(),
        );
    };
    if let Some(id) = FlavorId::from_spec(spec) {
        let folder_name = id.folder_name().unwrap_or_default();
        let root = game_root(root_flag, warnings)?.ok_or_else(|| {
            format!(
                "no game root for flavor {spec}: none in the launcher's registry key or the usual install folders; pass --root <folder>, set {ROOT_ENV} or run `mrgl root <folder>`"
            )
        })?;
        if root.origin.is_discovered() {
            warnings.push(root.found_line());
        }
        return Ok(root.path.join(folder_name));
    }
    let path = Path::new(spec);
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }
    Err(format!(
        "unknown flavor: {spec} (forever, retail, classic, era, a .flavor.info id or a flavor folder)"
    ))
}

/// The game root every command resolves the same way (task 020, `discover::resolve_root_of_process`):
/// the flag, then `MRGL_ROOT`, then the remembered one, then the launcher's registry key and the usual
/// install folders; `None` when nothing names it. A config warning is pushed; a discovered root carries
/// its origin for the caller to report.
fn game_root(
    root_flag: Option<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<Option<GameRoot>, String> {
    let resolution =
        discover::resolve_root_of_process(root_flag).map_err(|error| error.to_string())?;
    warnings.extend(resolution.warnings);
    Ok(resolution.root)
}

/// Prints a scan: the addon lines, the warnings and the summary, or the one JSON document. A client
/// running from the flavor folder (task 023) is named in the summary and is `client_running` in the JSON.
/// `dependencies` is the dependency check of the scan (task 029; empty for `scan`, which reads the folder
/// only). A Forever listing (task 011) marks every addon through [`mark_forever`].
fn report_scan(
    reporter: &Reporter,
    kind: &str,
    scan: &Scan,
    mut warnings: Vec<String>,
    dependencies: &[AddonDependencies],
    forever: Option<&BTreeMap<String, ForeverStatus>>,
) {
    warnings.extend(scan.flavor.warnings.iter().cloned());
    warnings.extend(scan.warnings.iter().cloned());
    let client = match client::client_running(&scan.flavor) {
        Ok(client) => client,
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    let mut lines = Vec::new();
    let mut objects = Vec::new();
    for addon in &scan.addons {
        let mut line = addon_line(addon);
        let mut object = serde_json::to_value(addon).unwrap_or_default();
        mark_dependencies(dependencies, &addon.folder, &mut line, &mut object);
        mark_forever(forever, addon, Some(&mut line), Some(&mut object));
        lines.push(line);
        objects.push(object);
    }
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": kind,
            "flavor": &scan.flavor,
            "addons": objects,
            "skipped": &scan.skipped,
            "client_running": client,
            "warnings": warnings,
        }));
        return;
    }
    for line in lines {
        reporter.line(line);
    }
    for (folder, note) in forever
        .into_iter()
        .flatten()
        .filter_map(|(folder, status)| Some((folder, forever_note(status)?)))
    {
        let folder = scan
            .addons
            .iter()
            .find(|addon| addon.folder.eq_ignore_ascii_case(folder))
            .map_or(folder.as_str(), |addon| addon.folder.as_str());
        reporter.say(Kind::Warning, format!("forever note on {folder}: {note}"));
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let mut build = match scan.flavor.version {
        Some(version) => format!("{} {version}", scan.flavor.id),
        None => format!("{}, build unknown", scan.flavor.id),
    };
    // The version came from the client exe, not from `.build.info` (task 019): worth a word, since
    // the two can disagree while the launcher updates.
    if let Some(VersionSource::ClientExe(exe)) = &scan.flavor.version_source {
        build.push_str(&format!(", version read from {exe}"));
    }
    if scan.flavor.curse_client {
        build.push_str(", managed by the CurseForge app");
    }
    if client.is_some() {
        build.push_str(", client running");
    }
    let mut summary = match scan.addons.len() {
        0 => format!("no addons in {} ({build})", scan.flavor.folder_name()),
        1 => format!("1 addon in {} ({build})", scan.flavor.folder_name()),
        count => format!("{count} addons in {} ({build})", scan.flavor.folder_name()),
    };
    match scan.skipped.len() {
        0 => {}
        1 => summary.push_str(", 1 folder without a TOC skipped"),
        count => summary.push_str(&format!(", {count} folders without a TOC skipped")),
    }
    let kind = if scan.addons.is_empty() {
        Kind::Nothing
    } else {
        Kind::Success
    };
    reporter.say(kind, summary);
}

/// One addon as a plain line: folder, version, interface list, the source hints and the flags.
fn addon_line(addon: &Addon) -> String {
    let toc = addon.primary_toc().map(|file| &file.toc);
    let version = addon.version().unwrap_or("-").to_string();
    let interface = toc
        .map(|toc| {
            toc.interface
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|list| !list.is_empty())
        .unwrap_or_else(|| "-".to_string());
    let mut parts = vec![addon.folder.clone(), version, interface];
    let hints: Vec<&str> = [
        ("curse", toc.and_then(|toc| toc.curse_project_id.as_ref())),
        ("wago", toc.and_then(|toc| toc.wago_id.as_ref())),
        ("wowi", toc.and_then(|toc| toc.wowi_id.as_ref())),
    ]
    .into_iter()
    .filter_map(|(name, id)| id.map(|_| name))
    .collect();
    if !hints.is_empty() {
        parts.push(hints.join(","));
    }
    if addon.has_legacy() {
        parts.push("legacy".to_string());
    }
    if addon.linked {
        parts.push("linked".to_string());
    }
    if addon.chosen.is_none() {
        parts.push("no toc for this flavor".to_string());
    }
    parts.join("  ")
}

/// Task 029's mark on one addon of a listing: a `missing: …` suffix on its line and the `dependencies`
/// field of its JSON object, from the check's result for the folder (nothing when the scan was not
/// checked).
fn mark_dependencies(
    checks: &[AddonDependencies],
    folder: &str,
    line: &mut String,
    object: &mut serde_json::Value,
) {
    let Some(check) = checks.iter().find(|check| check.folder == folder) else {
        return;
    };
    if check.has_missing() {
        line.push_str(&format!("  missing: {}", check.missing.join(", ")));
    }
    if let Some(object) = object.as_object_mut() {
        object.insert("dependencies".to_string(), serde_json::json!(check));
    }
}

/// The dependency check of a scan (task 029), with the `nolib` flags of the flavor's sidecar: a sidecar
/// that cannot be read is a warning and leaves every flag unknown. Every addon with a missing required
/// dependency adds its warning line.
fn check_dependencies(scan: &Scan, warnings: &mut Vec<String>) -> Vec<AddonDependencies> {
    let id = &scan.flavor.id;
    let state = match FlavorHome::new(id).and_then(|home| home.read_state(id)) {
        Ok(loaded) => {
            warnings.extend(loaded.warning);
            Some(loaded.state)
        }
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    let checks = dependencies::check(scan, state.as_ref());
    warnings.extend(checks.iter().filter_map(AddonDependencies::warning));
    checks
}

/// The dependency check after an install (task 029): the flavor's folder as it is now, the results of the
/// folders just installed, and a warning line for each with a missing required dependency.
fn installed_dependencies(
    engine: &Engine,
    folders: &[String],
    warnings: &mut Vec<String>,
) -> Vec<AddonDependencies> {
    let scan = match scan::scan_flavor(&engine.flavor) {
        Ok(scan) => scan,
        Err(error) => {
            warnings.push(error.to_string());
            return Vec::new();
        }
    };
    let state = engine.load_state().ok().map(|loaded| loaded.state);
    let checks: Vec<AddonDependencies> = dependencies::check(&scan, state.as_ref())
        .into_iter()
        .filter(|check| {
            folders
                .iter()
                .any(|folder| folder.eq_ignore_ascii_case(&check.folder))
        })
        .collect();
    warnings.extend(checks.iter().filter_map(AddonDependencies::warning));
    checks
}

/// Task 011: the Forever status of every managed addon of a Forever listing, by its primary folder in lower
/// case; `None` for another flavor. Offline: the build signal comes from the GitHub source's cache alone (the
/// release list and the `release.json` the last `add` fetched), so without a cached list it is `unknown`.
fn forever_statuses(
    scan: &Scan,
    warnings: &mut Vec<String>,
) -> Option<BTreeMap<String, ForeverStatus>> {
    if scan.flavor.id != FlavorId::Forever {
        return None;
    }
    let mut statuses = BTreeMap::new();
    let loaded =
        match FlavorHome::new(&scan.flavor.id).and_then(|home| home.read_state(&scan.flavor.id)) {
            Ok(loaded) => loaded,
            Err(error) => {
                warnings.push(error.to_string());
                return Some(statuses);
            }
        };
    warnings.extend(loaded.warning);
    let github = GitHub::from_env(sources::http_from_env());
    for entry in loaded.state.addons.values() {
        let Some(folder) = entry.folders.first() else {
            continue;
        };
        let build = match SourceSpec::parse(&entry.source) {
            Ok(spec) => github.cached_forever_build(&spec),
            Err(_) => ForeverBuild::Unknown,
        };
        statuses.insert(
            folder.to_ascii_lowercase(),
            ForeverStatus::for_source(&entry.source, build),
        );
    }
    Some(statuses)
}

/// Task 011: the Forever mark of a listed addon, appended to its line (`forever:yes`, with `,note` and
/// `,stale` when a curated note applies) or set as the `forever` field of its JSON object (`null` for an
/// unmanaged addon). Nothing outside a Forever listing (`forever` is `None`).
fn mark_forever(
    forever: Option<&BTreeMap<String, ForeverStatus>>,
    addon: &Addon,
    line: Option<&mut String>,
    object: Option<&mut serde_json::Value>,
) {
    let Some(forever) = forever else {
        return;
    };
    let status = forever.get(&addon.folder.to_ascii_lowercase());
    if let (Some(line), Some(status)) = (line, status) {
        line.push_str(&format!("  forever:{}", status.build));
        if status.note.is_some() {
            line.push_str(",note");
        }
        if status.stale {
            line.push_str(",stale");
        }
    }
    if let Some(serde_json::Value::Object(object)) = object {
        object.insert(
            "forever".to_string(),
            serde_json::to_value(status).unwrap_or_default(),
        );
    }
}

/// Task 011: the curated note as a plain clause, `<reason> (verified <date>, <source>)`, with the stale
/// warning when the date is older than the age; `None` without a note.
fn forever_note(status: &ForeverStatus) -> Option<String> {
    let reason = status.note.as_deref()?;
    let verified = status.verified.as_deref().unwrap_or("-");
    let stale = if status.stale {
        format!(", stale: older than {STALE_AFTER_DAYS} days")
    } else {
        String::new()
    };
    let source = status.source.as_deref().unwrap_or("-");
    Some(format!("{reason} (verified {verified}{stale}, {source})"))
}

/// Task 011: the Forever line of an `add`, after the pick: the derived build signal and the curated note.
/// A success when the source publishes a Forever build and no note applies, a warning otherwise.
fn forever_message(status: &ForeverStatus) -> (Kind, String) {
    let why = match status.build {
        ForeverBuild::Yes => "release.json names forever",
        ForeverBuild::No => "release.json names no forever flavor",
        ForeverBuild::Unknown => "no release.json says",
    };
    let mut text = format!("forever build: {} ({why})", status.build);
    if let Some(note) = forever_note(status) {
        text.push_str(&format!("; note: {note}"));
    }
    let kind = if status.build == ForeverBuild::Yes && status.note.is_none() {
        Kind::Success
    } else {
        Kind::Warning
    };
    (kind, text)
}

/// The target of an `add`: the flavor the spec names and, when a game folder is known and holds that
/// flavor, its installed build. A flavor folder path gives both; a name or an id alone gives the flavor
/// only, so the pick for the progressing classic is a guess.
fn add_target(spec: Option<&str>, warnings: &mut Vec<String>) -> Result<Target, String> {
    let Some(spec) = spec else {
        return Err(
            "no flavor given: --flavor forever, retail, classic, era, a .flavor.info id or a flavor folder"
                .to_string(),
        );
    };
    if let Some(id) = FlavorId::from_spec(spec) {
        let folder_name = id.folder_name().unwrap_or_default();
        if let Some(root) = game_root(None, warnings)?
            && let Ok(flavor) = flavor::read_flavor(&root.path.join(folder_name))
        {
            if root.origin.is_discovered() {
                warnings.push(root.found_line());
            }
            warnings.extend(flavor.warnings.iter().cloned());
            return Ok(Target::from_flavor(&flavor));
        }
        return Ok(Target::new(id));
    }
    let path = Path::new(spec);
    if path.is_dir() {
        let flavor = flavor::read_flavor(path).map_err(|error| error.to_string())?;
        warnings.extend(flavor.warnings.iter().cloned());
        return Ok(Target::from_flavor(&flavor));
    }
    Err(format!(
        "unknown flavor: {spec} (forever, retail, classic, era, a .flavor.info id or a flavor folder)"
    ))
}

/// `mrgl add <source> --flavor <flavor> [--dry-run]`: resolve the release for the flavor and report the
/// pick; the install from the pick is task 040, so without `--dry-run` the command says so and fails.
fn add_command(
    reporter: &Reporter,
    source: &str,
    flavor: Option<&str>,
    dry_run: bool,
) -> Result<(), String> {
    let spec = SourceSpec::parse(source).map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let target = add_target(flavor, &mut warnings)?;
    let name = target.flavor_name().to_string();
    if !reporter.json {
        reporter.say(Kind::Progress, format!("resolving {spec} for {name}"));
    }
    let github = GitHub::from_env(sources::http_from_env());
    let pick = github
        .resolve(&spec, &target)
        .map_err(|error| error.to_string())?;
    let forever = (target.flavor == FlavorId::Forever)
        .then(|| ForeverStatus::for_source(&pick.source, pick.forever_build));
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "add",
            "source": &spec,
            "flavor": &target.flavor,
            "build": target.version.map(|version| version.to_string()),
            "dry_run": dry_run,
            "pick": &pick,
            "forever": forever,
            "installed": false,
            "warnings": warnings,
        }));
    } else {
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        if pick.guess {
            reporter.say(
                Kind::Warning,
                format!("pick for {name}, a guess: {}", pick.summary()),
            );
        } else {
            reporter.say(
                Kind::Success,
                format!("pick for {name}: {}", pick.summary()),
            );
        }
        say_release_notes(reporter, &pick);
        if let Some(status) = forever.as_ref().filter(|_| dry_run) {
            let (kind, text) = forever_message(status);
            reporter.say(kind, text);
        }
        if dry_run {
            reporter.say(Kind::Nothing, "dry run: nothing installed");
        }
    }
    if dry_run {
        return Ok(());
    }
    Err(Error::NotImplemented {
        command: "add (the install)",
        task: "040",
    }
    .to_string())
}

/// The release notes under the pick (task 030): the notes line, then the release page, as one message of the
/// neutral kind (something to read, not an outcome); nothing at all when the release body says nothing.
fn say_release_notes(reporter: &Reporter, pick: &sources::Pick) {
    let Some(notes) = &pick.notes else {
        return;
    };
    match &pick.page {
        Some(page) => reporter.say(Kind::Nothing, format!("release notes: {notes} ({page})")),
        None => reporter.say(Kind::Nothing, format!("release notes: {notes}")),
    }
}

/// `mrgl root [<folder>]`: remember a game folder, or print the one a run would use with its origin
/// (`MRGL_ROOT`, the remembered one, or a discovered one, which is reported and never saved).
fn root_command(reporter: &Reporter, folder: Option<PathBuf>) -> Result<(), String> {
    match folder {
        Some(folder) => {
            let folder = std::path::absolute(&folder)
                .map_err(|error| format!("resolving {}: {error}", folder.display()))?;
            let flavors = flavor::list_flavors(&folder).map_err(|error| error.to_string())?;
            if flavors.is_empty() {
                return Err(Error::NotAGameRoot { path: folder }.to_string());
            }
            home::write_config(&Config {
                root: Some(folder.clone()),
            })
            .map_err(|error| error.to_string())?;
            let ids: Vec<String> = flavors.iter().map(|flavor| flavor.id.to_string()).collect();
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "root",
                    "root": folder,
                    "root_origin": RootOrigin::Remembered,
                    "flavors": ids,
                    "saved": true,
                    "warnings": [],
                }));
            } else {
                reporter.say(
                    Kind::Success,
                    format!(
                        "game root saved: {} (flavors: {})",
                        folder.display(),
                        ids.join(", ")
                    ),
                );
            }
        }
        None => {
            let mut warnings = Vec::new();
            let root = game_root(None, &mut warnings)?;
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "root",
                    "root": root.as_ref().map(|root| &root.path),
                    "root_origin": root.as_ref().map(|root| root.origin),
                    "flavors": [],
                    "saved": false,
                    "warnings": warnings,
                }));
            } else {
                for warning in &warnings {
                    reporter.say(Kind::Warning, warning);
                }
                match root {
                    Some(root) if root.origin.is_discovered() => {
                        reporter.say(Kind::Success, root.found_line());
                    }
                    Some(root) => reporter.say(
                        Kind::Success,
                        format!(
                            "game root: {} ({})",
                            root.path.display(),
                            root.origin.description()
                        ),
                    ),
                    None => reporter.say(
                        Kind::Nothing,
                        "no game root: none remembered, none in the launcher's registry key or the usual install folders; run `mrgl root <folder>`",
                    ),
                }
            }
        }
    }
    Ok(())
}

/// Whether an `add` target is a zip in hand rather than a source spec: a `.zip` name or an existing file.
fn is_zip_in_hand(source: &str) -> bool {
    source.to_ascii_lowercase().ends_with(".zip") || Path::new(source).is_file()
}

/// The install engine of the flavor a `--flavor` spec names.
fn engine_for(spec: Option<&str>, warnings: &mut Vec<String>) -> Result<Engine, String> {
    let folder = flavor_folder(spec, None, warnings)?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| error.to_string())?;
    warnings.extend(flavor.warnings.iter().cloned());
    Engine::new(flavor).map_err(|error| error.to_string())
}

/// `mrgl add <file.zip> --flavor <id>`: the fourth source of D-002, a zip in hand, through the install
/// engine; `--dry-run` stages the zip, says what it would install and writes nothing. A client running from
/// the flavor folder is a warning line and `client_warning` in the JSON (task 038), never a refusal.
fn add_zip(
    reporter: &Reporter,
    source: &str,
    flavor: Option<&str>,
    dry_run: bool,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let zip =
        std::path::absolute(source).map_err(|error| format!("resolving {source}: {error}"))?;
    let zip_name = zip
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| zip.display().to_string());
    let metadata = Metadata {
        source: zip.display().to_string(),
        version: None,
        name: None,
        nolib: None,
    };
    let folder_name = engine.flavor.folder_name();
    if !reporter.json {
        reporter.say(
            Kind::Progress,
            format!("installing {zip_name} into {folder_name}"),
        );
    }

    if dry_run {
        let loaded = engine.load_state().map_err(|error| error.to_string())?;
        warnings.extend(loaded.warning);
        let staged = engine
            .stage(&zip, &metadata)
            .map_err(|error| error.to_string())?;
        engine
            .refuse_links(&loaded.state, &staged.folders())
            .map_err(|error| error.to_string())?;
        let previous = loaded
            .state
            .entry(&staged.name)
            .map(|(_, entry)| entry.version.clone());
        let folders = staged.folders();
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "add",
                "flavor": &engine.flavor,
                "dry_run": true,
                "addon": {
                    "name": staged.name,
                    "version": staged.version,
                    "previous_version": previous,
                    "source": staged.source,
                    "folders": folders,
                    "removed_folders": [],
                },
                "warnings": warnings,
            }));
        } else {
            for warning in &warnings {
                reporter.say(Kind::Warning, warning);
            }
            let what = match &previous {
                Some(previous) => format!(
                    "update {} from {previous} to {}",
                    staged.name, staged.version
                ),
                None => format!("install {} {}", staged.name, staged.version),
            };
            reporter.say(
                Kind::Success,
                format!(
                    "dry run: would {what} ({}: {}) in {folder_name}",
                    count_folders(folders.len()),
                    folders.join(", ")
                ),
            );
        }
        staged.discard();
        return Ok(());
    }

    let installed = engine
        .install(&zip, &metadata)
        .map_err(|error| error.to_string())?;
    warnings.extend(installed.warnings.iter().cloned());
    let dependencies = installed_dependencies(&engine, &installed.folders, &mut warnings);
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "add",
            "flavor": &engine.flavor,
            "dry_run": false,
            "addon": {
                "name": installed.name,
                "version": installed.version,
                "previous_version": installed.previous_version,
                "source": metadata.source,
                "folders": installed.folders,
                "removed_folders": installed.removed_folders,
                "dependencies": dependencies,
            },
            "client_warning": installed.client_warning,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in installed.client_warning.iter().chain(&warnings) {
        reporter.say(Kind::Warning, warning);
    }
    let mut summary = match &installed.previous_version {
        Some(previous) => format!(
            "updated {} from {previous} to {} ({}: {}) in {folder_name}",
            installed.name,
            installed.version,
            count_folders(installed.folders.len()),
            installed.folders.join(", ")
        ),
        None => format!(
            "installed {} {} ({}: {}) into {folder_name}",
            installed.name,
            installed.version,
            count_folders(installed.folders.len()),
            installed.folders.join(", ")
        ),
    };
    if !installed.removed_folders.is_empty() {
        summary.push_str(&format!(
            ", {} of the previous version removed ({})",
            count_folders(installed.removed_folders.len()),
            installed.removed_folders.join(", ")
        ));
    }
    reporter.say(Kind::Success, summary);
    Ok(())
}

/// `mrgl remove <name> --flavor <id>`: a managed addon's folders, its entry and its cache go; a running
/// client is a warning, as for `add`.
fn remove_command(reporter: &Reporter, name: &str, flavor: Option<&str>) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let removed = engine.remove(name).map_err(|error| error.to_string())?;
    warnings.extend(removed.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "remove",
            "flavor": &engine.flavor,
            "addon": {
                "name": removed.name,
                "version": removed.version,
                "folders": removed.folders,
            },
            "client_warning": removed.client_warning,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in removed.client_warning.iter().chain(&warnings) {
        reporter.say(Kind::Warning, warning);
    }
    reporter.say(
        Kind::Success,
        format!(
            "removed {} {} ({}: {}) from {}",
            removed.name,
            removed.version,
            count_folders(removed.folders.len()),
            removed.folders.join(", "),
            engine.flavor.folder_name()
        ),
    );
    Ok(())
}

/// `mrgl link <folder> --flavor <id>` (task 005): a junction under `Interface\AddOns` for every addon of a
/// development folder, recorded as `link:<folder>`; a name in the way is a warning, never replaced.
fn link_command(reporter: &Reporter, folder: &Path, flavor: Option<&str>) -> Result<(), String> {
    link::ensure_supported().map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let folder_name = engine.flavor.folder_name();
    if !reporter.json {
        reporter.say(
            Kind::Progress,
            format!(
                "linking the addons of {} into {folder_name}",
                folder.display()
            ),
        );
    }
    let linked = link::link(&engine, folder).map_err(|error| error.to_string())?;
    warnings.extend(linked.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "link",
            "flavor": &engine.flavor,
            "folder": linked.folder,
            "source": linked.source,
            "linked": linked.linked,
            "skipped": linked.skipped,
            "without_toc": linked.without_toc,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    for skipped in &linked.skipped {
        reporter.say(
            Kind::Warning,
            format!("skipped {}: {}", skipped.name, skipped.reason),
        );
    }
    let names: Vec<&str> = linked
        .linked
        .iter()
        .map(|addon| addon.name.as_str())
        .collect();
    let already = linked
        .linked
        .iter()
        .filter(|addon| addon.already_linked)
        .count();
    let mut summary = if names.is_empty() {
        format!(
            "linked no addon from {} into {folder_name}",
            linked.folder.display()
        )
    } else {
        format!(
            "linked {} from {} into {folder_name}: {}",
            count_addons(names.len()),
            linked.folder.display(),
            names.join(", ")
        )
    };
    if already > 0 {
        summary.push_str(&format!(", {already} of them linked already"));
    }
    if !linked.skipped.is_empty() {
        summary.push_str(&format!(", {} skipped", linked.skipped.len()));
    }
    match linked.without_toc.len() {
        0 => {}
        1 => summary.push_str(", 1 folder without a TOC left out"),
        count => summary.push_str(&format!(", {count} folders without a TOC left out")),
    }
    let kind = if !linked.skipped.is_empty() {
        Kind::Warning
    } else if already == names.len() {
        Kind::Nothing
    } else {
        Kind::Success
    };
    reporter.say(kind, summary);
    Ok(())
}

/// `mrgl unlink <folder> --flavor <id>` (task 005): the junctions `link` made from the folder go, and
/// their entries; a real folder or a junction pointing elsewhere stays.
fn unlink_command(reporter: &Reporter, folder: &Path, flavor: Option<&str>) -> Result<(), String> {
    link::ensure_supported().map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let folder_name = engine.flavor.folder_name();
    let unlinked = link::unlink(&engine, folder).map_err(|error| error.to_string())?;
    warnings.extend(unlinked.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "unlink",
            "flavor": &engine.flavor,
            "folder": unlinked.folder,
            "source": unlinked.source,
            "unlinked": unlinked.unlinked,
            "kept": unlinked.kept,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    for kept in &unlinked.kept {
        reporter.say(
            Kind::Warning,
            format!("kept {}: {}", kept.name, kept.reason),
        );
    }
    if unlinked.unlinked.is_empty() && unlinked.kept.is_empty() {
        reporter.say(
            Kind::Nothing,
            format!(
                "nothing linked from {} in {folder_name}",
                unlinked.folder.display()
            ),
        );
        return Ok(());
    }
    let mut summary = format!(
        "unlinked {} of {} from {folder_name}",
        count_addons(unlinked.unlinked.len()),
        unlinked.folder.display()
    );
    if !unlinked.unlinked.is_empty() {
        summary.push_str(&format!(": {}", unlinked.unlinked.join(", ")));
    }
    if !unlinked.kept.is_empty() {
        summary.push_str(&format!(", {} kept", unlinked.kept.len()));
    }
    let kind = if unlinked.kept.is_empty() {
        Kind::Success
    } else {
        Kind::Warning
    };
    reporter.say(kind, summary);
    Ok(())
}

fn count_addons(count: usize) -> String {
    if count == 1 {
        "1 addon".to_string()
    } else {
        format!("{count} addons")
    }
}

fn count_folders(count: usize) -> String {
    if count == 1 {
        "1 folder".to_string()
    } else {
        format!("{count} folders")
    }
}

/// `mrgl backup --flavor <id> [--to <folder>]`: the flavor's `WTF` folder into a dated zip under the
/// backups folder; `--list` lists that folder; `--restore <zip>` puts a backup back after a confirmation
/// (`--yes` answers it for a script, and is required with `--json`). The restore is refused while the
/// client runs (task 023), before anything is written.
fn backup_command(
    reporter: &Reporter,
    spec: Option<&str>,
    to: Option<PathBuf>,
    list: bool,
    restore: Option<PathBuf>,
    yes: bool,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let folder = flavor_folder(spec, None, &mut warnings)?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| error.to_string())?;
    warnings.extend(flavor.warnings.iter().cloned());
    let backups = match to {
        Some(to) => std::path::absolute(&to)
            .map_err(|error| format!("resolving {}: {error}", to.display()))?,
        None => backup::backups_dir(&flavor.id).map_err(|error| error.to_string())?,
    };
    let folder_name = flavor.folder_name();

    if list {
        let listing = backup::list_backups(&backups).map_err(|error| error.to_string())?;
        warnings.extend(listing.warnings.iter().cloned());
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "backup-list",
                "flavor": &flavor,
                "folder": &listing.folder,
                "backups": &listing.backups,
                "aside": &listing.aside,
                "warnings": warnings,
            }));
            return Ok(());
        }
        for entry in &listing.backups {
            let files = match entry.files {
                Some(1) => "1 file".to_string(),
                Some(count) => format!("{count} files"),
                None => "unreadable".to_string(),
            };
            reporter.line(format!(
                "{}  {files}  {}  {}",
                entry.name,
                human_bytes(entry.bytes),
                entry.modified.as_deref().unwrap_or("-")
            ));
        }
        for aside in &listing.aside {
            reporter.line(format!(
                "{}  the WTF folder a restore moved aside",
                aside
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| aside.display().to_string())
            ));
        }
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        let mut summary = match listing.backups.len() {
            0 => format!(
                "no backups of {folder_name} in {}",
                listing.folder.display()
            ),
            1 => format!("1 backup of {folder_name} in {}", listing.folder.display()),
            count => format!(
                "{count} backups of {folder_name} in {}",
                listing.folder.display()
            ),
        };
        match listing.aside.len() {
            0 => {}
            1 => summary.push_str(", 1 WTF folder moved aside by a restore"),
            count => summary.push_str(&format!(", {count} WTF folders moved aside by restores")),
        }
        let kind = if listing.backups.is_empty() {
            Kind::Nothing
        } else {
            Kind::Success
        };
        reporter.say(kind, summary);
        return Ok(());
    }

    if let Some(zip) = restore {
        let zip = resolve_backup(zip, &backups);
        let zip_name = zip
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| zip.display().to_string());
        if !yes {
            if reporter.json {
                return Err(
                    "restore needs --yes with --json: nobody is there to answer the confirmation"
                        .to_string(),
                );
            }
            reporter.say(
                Kind::Warning,
                format!(
                    "restore {zip_name} as the WTF folder of {folder_name}? the current WTF is moved aside into {} first; type y to go on",
                    backups.display()
                ),
            );
            if !confirmed() {
                reporter.say(Kind::Nothing, "restore cancelled, nothing changed");
                return Ok(());
            }
        }
        let restored =
            backup::restore_wtf(&flavor, &zip, &backups).map_err(|error| error.to_string())?;
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "restore",
                "flavor": &flavor,
                "zip": &restored.zip,
                "files": restored.files,
                "folders": restored.folders,
                "aside": &restored.aside,
                "warnings": warnings,
            }));
            return Ok(());
        }
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        let files = match restored.files {
            1 => "1 file".to_string(),
            count => format!("{count} files"),
        };
        let previous = match &restored.aside {
            Some(aside) => format!("the previous WTF is at {}", aside.display()),
            None => "there was no WTF folder before".to_string(),
        };
        reporter.say(
            Kind::Success,
            format!("restored WTF of {folder_name} from {zip_name}: {files}; {previous}"),
        );
        return Ok(());
    }

    if !reporter.json {
        reporter.say(
            Kind::Progress,
            format!("backing up WTF of {folder_name} into {}", backups.display()),
        );
    }
    let backup = backup::backup_wtf(&flavor, &backups).map_err(|error| error.to_string())?;
    warnings.extend(backup.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "backup",
            "flavor": &flavor,
            "zip": &backup.zip,
            "files": backup.files,
            "folders": backup.folders,
            "bytes": backup.bytes,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let files = match backup.files {
        1 => "1 file".to_string(),
        count => format!("{count} files"),
    };
    reporter.say(
        Kind::Success,
        format!(
            "backed up WTF of {folder_name}: {files}, {}, in {}",
            human_bytes(backup.bytes),
            backup.zip.display()
        ),
    );
    Ok(())
}

/// The zip `--restore` names: a path as given, or a bare name (one from `--list`) under the backups
/// folder when no such path exists where the command runs.
fn resolve_backup(zip: PathBuf, backups: &Path) -> PathBuf {
    let bare = zip.components().count() == 1 && !zip.is_absolute();
    if bare && !zip.exists() {
        return backups.join(zip);
    }
    std::path::absolute(&zip).unwrap_or(zip)
}

/// Reads one line of the standard input: `y` or `yes`, in any case, is a confirmation; anything else,
/// an empty line or a closed input is not.
fn confirmed() -> bool {
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer).is_err() {
        return false;
    }
    let answer = answer.trim();
    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
}

/// A byte count for a human: `512 B`, `34.2 KB`, `1.5 MB`, `2.0 GB`.
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = UNITS[0];
    for next in &UNITS[1..] {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    format!("{value:.1} {unit}")
}

/// `mrgl doctor [--flavor <flavor>] [--root <folder>]`: the diagnostic report (task 024). The plain output
/// is a block the user pastes into a bug report, one fact per line like the data lines of a listing, and
/// the voice's two-line summary after it; `--json` is the one document. The report never fails: what it
/// cannot read is `unknown` in it, so only an unknown `--flavor` is an error.
fn doctor_command(
    reporter: &Reporter,
    flavor: Option<&str>,
    root: Option<PathBuf>,
) -> Result<(), String> {
    let only = match flavor {
        Some(spec) => Some(FlavorId::from_spec(spec).ok_or_else(|| {
            format!("unknown flavor: {spec} (forever, retail, classic, era or a .flavor.info id)")
        })?),
        None => None,
    };
    let report = mrgl_core::doctor::report_of_process(root, only);
    if reporter.json {
        let mut value = serde_json::to_value(&report).map_err(|error| error.to_string())?;
        value["kind"] = "doctor".into();
        reporter.document(value);
        return Ok(());
    }
    for line in report.lines() {
        reporter.line(line);
    }
    let (kind, summary) = report.summary();
    reporter.say(kind, summary);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(build: ForeverBuild, note: bool, stale: bool) -> ForeverStatus {
        ForeverStatus {
            build,
            note: note.then(|| "threat colors are off".to_string()),
            kind: note.then(|| "combat-log-events".to_string()),
            verified: note.then(|| "2026-06-01".to_string()),
            source: note.then(|| "https://example.test/note".to_string()),
            stale,
        }
    }

    #[test]
    fn the_forever_line_names_the_build_and_the_note() {
        assert_eq!(
            forever_message(&status(ForeverBuild::Yes, false, false)),
            (
                Kind::Success,
                "forever build: yes (release.json names forever)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::Unknown, false, false)),
            (
                Kind::Warning,
                "forever build: unknown (no release.json says)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::Yes, true, false)),
            (
                Kind::Warning,
                "forever build: yes (release.json names forever); note: threat colors are off (verified 2026-06-01, https://example.test/note)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::No, true, true)).1,
            "forever build: no (release.json names no forever flavor); note: threat colors are off (verified 2026-06-01, stale: older than 90 days, https://example.test/note)"
        );
    }

    #[test]
    fn a_listed_addon_carries_its_mark_on_the_line_and_in_the_json() {
        let scan = scan::scan(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../mrgl-core/tests/fixtures/flavors/_retail_"),
        )
        .expect("a fixture flavor folder");
        let addon = scan.addons.first().expect("an addon in the fixture");
        let mut forever = BTreeMap::new();
        forever.insert(
            addon.folder.to_ascii_lowercase(),
            status(ForeverBuild::Yes, true, true),
        );
        let mut line = "Line".to_string();
        let mut object = serde_json::json!({ "folder": addon.folder });
        mark_forever(Some(&forever), addon, Some(&mut line), Some(&mut object));
        assert_eq!(line, "Line  forever:yes,note,stale");
        assert_eq!(object["forever"]["build"], "yes");
        assert_eq!(object["forever"]["note"], "threat colors are off");
        assert_eq!(object["forever"]["stale"], true);

        // An unmanaged addon of a Forever listing: no mark on the line, `null` in the JSON.
        let empty = BTreeMap::new();
        let mut line = "Line".to_string();
        let mut object = serde_json::json!({});
        mark_forever(Some(&empty), addon, Some(&mut line), Some(&mut object));
        assert_eq!(line, "Line");
        assert_eq!(object["forever"], serde_json::Value::Null);
        assert!(object.as_object().unwrap().contains_key("forever"));
        // Another flavor: nothing at all.
        let mut object = serde_json::json!({});
        mark_forever(None, addon, None, Some(&mut object));
        assert!(!object.as_object().unwrap().contains_key("forever"));
    }
}
