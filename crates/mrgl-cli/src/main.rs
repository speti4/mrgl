//! The `mrgl` command line. Every human-readable line goes through the voice (D-005): a murloc line and a
//! plain English line, `--serious` or `MRGL_SERIOUS=1` for the plain line alone, `--json` for machines.
//! Exit codes: 0 success, 1 an error the plain line explains, 2 a usage error (clap's own).
//!
//! `scan`, `list` and `root` are task 002, the game root discovery behind them is task 020;
//! `add github:<owner>/<repo>` resolves a release (task 003); `add <file.zip>` and `remove` are task 004
//! (the install engine); `backup` (the WTF backup, the listing and the restore) is task 013; the other
//! subcommands follow the M1 plan (`tasks/roadmap.md`) and answer "not implemented yet" with their task id
//! until that task lands, so the help text and the voice are testable from the first commit.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::discover::{self, GameRoot, ROOT_ENV, RootOrigin};
use mrgl_core::flavor::{self, FlavorId, VersionSource};
use mrgl_core::home::{self, Config};
use mrgl_core::install::{Engine, Metadata};
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::{self, Source, SourceSpec, Target};
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
            | Command::Backup { .. } => return None,
            Command::Update { all: true, .. } => ("update --all", "006"),
            Command::Update { .. } => ("update", "006"),
            Command::Link { .. } | Command::Unlink { .. } => ("link / unlink", "005"),
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
            report_scan(reporter, "scan", &scan, Vec::new());
        }
        Command::List { flavor, root } => {
            let mut warnings = Vec::new();
            let folder = flavor_folder(flavor.as_deref(), root, &mut warnings)?;
            let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
            report_scan(reporter, "list", &scan, warnings);
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
fn report_scan(reporter: &Reporter, kind: &str, scan: &Scan, mut warnings: Vec<String>) {
    warnings.extend(scan.flavor.warnings.iter().cloned());
    warnings.extend(scan.warnings.iter().cloned());
    let client = match client::client_running(&scan.flavor) {
        Ok(client) => client,
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": kind,
            "flavor": &scan.flavor,
            "addons": &scan.addons,
            "skipped": &scan.skipped,
            "client_running": client,
            "warnings": warnings,
        }));
        return;
    }
    for addon in &scan.addons {
        reporter.line(addon_line(addon));
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
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "add",
            "source": &spec,
            "flavor": &target.flavor,
            "build": target.version.map(|version| version.to_string()),
            "dry_run": dry_run,
            "pick": &pick,
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
