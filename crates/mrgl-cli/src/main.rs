//! The `mrgl` command line. Every human-readable line goes through the voice (D-005): a murloc line and a
//! plain English line, `--serious` or `MRGL_SERIOUS=1` for the plain line alone, `--json` for machines.
//! Exit codes: 0 success, 1 an error the plain line explains, 2 a usage error (clap's own).
//!
//! `scan`, `list` and `root` are task 002; the other subcommands follow the M1 plan (`tasks/roadmap.md`)
//! and answer "not implemented yet" with their task id until that task lands, so the help text and the
//! voice are testable from the first commit.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mrgl_core::client;
use mrgl_core::flavor::{self, FlavorId};
use mrgl_core::home::{self, Config};
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::voice::{Kind, Voice};
use mrgl_core::{Error, VERSION};

/// The environment variable that names the game folder for one run.
const ROOT_ENV: &str = "MRGL_ROOT";

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
        /// name or id; otherwise `MRGL_ROOT`, then the folder `mrgl root` remembered.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Remember the World of Warcraft folder (the one with `_retail_`, `_forever_` …), or print the
    /// remembered one.
    Root {
        /// The game folder to remember; without it, the remembered one is printed.
        folder: Option<PathBuf>,
    },
    /// Install an addon from a source: `github:<owner>/<repo>`, later `wago:`, `wowi:`, `curse:` or a zip path.
    Add {
        /// The source spec.
        source: String,
        /// The flavor id or folder to install into.
        #[arg(long)]
        flavor: Option<String>,
        /// Resolve and report, write nothing.
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
    /// Back up a flavor's WTF folder, or restore one (the toolbox).
    Backup {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Where the backup goes, or comes from with `--restore`.
        #[arg(long)]
        to: Option<PathBuf>,
        /// Restore from `--to` instead of backing up.
        #[arg(long)]
        restore: bool,
    },
}

impl Command {
    /// The "not yet" answer of a command whose task has not landed: its name and the task.
    fn not_yet(&self) -> Option<Error> {
        let (command, task) = match self {
            Command::Scan { .. } | Command::List { .. } | Command::Root { .. } => return None,
            Command::Add { .. } => ("add", "003"),
            Command::Update { all: true, .. } => ("update --all", "006"),
            Command::Update { .. } | Command::Remove { .. } => ("update / remove", "004"),
            Command::Link { .. } | Command::Unlink { .. } => ("link / unlink", "005"),
            Command::Backup { .. } => ("backup", "013"),
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
                "no game root for flavor {spec}: pass --root <folder>, set {ROOT_ENV} or run `mrgl root <folder>`"
            )
        })?;
        return Ok(root.join(folder_name));
    }
    let path = Path::new(spec);
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }
    Err(format!(
        "unknown flavor: {spec} (forever, retail, classic, era, a .flavor.info id or a flavor folder)"
    ))
}

/// The game root: the flag, then `MRGL_ROOT`, then the remembered one; `None` when nothing names it.
fn game_root(
    root_flag: Option<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<Option<PathBuf>, String> {
    if let Some(root) = root_flag {
        return Ok(Some(root));
    }
    if let Some(root) = std::env::var_os(ROOT_ENV).filter(|value| !value.is_empty()) {
        return Ok(Some(PathBuf::from(root)));
    }
    match home::read_config() {
        Ok(loaded) => {
            warnings.extend(loaded.warning);
            Ok(loaded.config.root)
        }
        Err(Error::NoHome) => Ok(None),
        Err(other) => Err(other.to_string()),
    }
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

/// `mrgl root [<folder>]`: remember a game folder, or print the remembered one.
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
            let loaded = home::read_config().map_err(|error| error.to_string())?;
            let warnings: Vec<String> = loaded.warning.into_iter().collect();
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "root",
                    "root": loaded.config.root,
                    "flavors": [],
                    "saved": false,
                    "warnings": warnings,
                }));
            } else {
                for warning in &warnings {
                    reporter.say(Kind::Warning, warning);
                }
                match loaded.config.root {
                    Some(root) => {
                        reporter.say(Kind::Success, format!("game root: {}", root.display()));
                    }
                    None => reporter.say(
                        Kind::Nothing,
                        "no game root remembered: run `mrgl root <folder>`",
                    ),
                }
            }
        }
    }
    Ok(())
}
