//! The `mrgl` command line. Every human-readable line goes through the voice (D-005): a murloc line and a
//! plain English line, `--serious` or `MRGL_SERIOUS=1` for the plain line alone, `--json` for machines.
//! Exit codes: 0 success, 1 an error the plain line explains, 2 a usage error (clap's own); a reader that
//! closes the pipe early ends the run with 0 ([`emit`], task 075).
//!
//! `scan`, `list` and `root` are task 002, the game root discovery behind them is task 020;
//! `add github:<owner>/<repo>` resolves a release (task 003) and installs it (task 040); `add <file.zip>` and
//! `remove` are task 004
//! (the install engine); `list` reads the state sidecar for the managed and the unmanaged addons (task 037);
//! `backup` (the WTF backup, the listing and the restore) is task 013; the Forever
//! notes on `add` and on `list` (the derived build signal and the curated note) are task 011; `link` and
//! `unlink` (the dev addon junctions) are task 005; `export` and `import` (the addon list) and the
//! interface warning of `list` and `add` are task 015; `pin`, `unpin`, `ignore`, `unignore` and
//! `channel`, with their marks in `list`, are task 021; `rollback` (the previous version from the kept
//! zip, then pinned) is task 022; `savedvars` (the SavedVariables sizes and the orphan cleanup) is task
//! 025; every source-backed install takes the one install path of the engine (task 055); `update` (the
//! update check and `update --all`) is task 006; `set` (the addon sets) is task 014, its
//! `--character` and `--realm` task 027, `set undo`, `set enable|disable|drop` and `--account` task 073;
//! `adopt` (where the unmanaged addons of a flavor come from) is task 097, its `--apply` (their sidecar
//! entries, the `.curseclient` guard and the warning of `add`, `import` and `update` in a folder the
//! CurseForge app manages) task 098.

// The `coverage(off)` attribute of the documented exclusions (docs/development.md, "Coverage"); the feature
// gate exists only under cargo-llvm-cov's nightly cfg, so the stable build never sees it.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::collections::BTreeMap;
use std::io::{BufRead, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mrgl_core::adopt;
use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::dependencies::{self, AddonDependencies};
use mrgl_core::discover::{self, GameRoot, ROOT_ENV, RootOrigin};
use mrgl_core::flavor::{self, Flavor, FlavorId, VersionSource};
use mrgl_core::forever::{ForeverBuild, ForeverStatus, STALE_AFTER_DAYS};
use mrgl_core::home::{self, Config};
use mrgl_core::install::{
    AlreadyInstalled, Engine, Folded, InstallOutcome, Installed, InstalledChecks, Mark, Metadata,
    PickInstalled, PickStep, SourceInstalled, SourceRequest,
};
use mrgl_core::interface::{self, InterfaceMismatch, InterfaceOutOfDate};
use mrgl_core::keys;
use mrgl_core::last_check;
use mrgl_core::link;
use mrgl_core::list_file::{self, ListFile, Outcome};
use mrgl_core::savedvars::{self, SavedVariablesFile, Status as SavedStatus};
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::sets::{self, AddonSet};
use mrgl_core::sources::{self, Channel, Source, SourceKind, SourceSpec, Sources, Target};
use mrgl_core::state::{Entry, FlavorHome, State, Status};
use mrgl_core::update::{self, CheckProgress, SummaryOf, UpdateState};
use mrgl_core::voice::{Kind, Voice};
use mrgl_core::zip_entry;
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
    /// List the addons installed in a flavor: managed by mrgl (with the source and the installed version)
    /// or unmanaged (with the TOC's source hints).
    List {
        /// The flavor: `forever`, `retail`, `classic` or `era`, a `.flavor.info` id (`wowf`, `wow`,
        /// `wow_classic`, `wow_classic_era`) or a flavor folder.
        #[arg(long)]
        flavor: Option<String>,
        /// The World of Warcraft folder (the one with `_retail_`, `_forever_` …) for a flavor named by
        /// name or id; otherwise `MRGL_ROOT`, then the folder `mrgl root` remembered, then the
        /// registry (the launcher's key, the game's Uninstall entry) and the usual install folders.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Remember the World of Warcraft folder (the one with `_retail_`, `_forever_` …), or print the one a
    /// run would use and where it comes from.
    Root {
        /// The game folder to remember; without it, the one a run would use is printed: `MRGL_ROOT`, the
        /// remembered one, or the one found in the registry or the usual install folders (found, not
        /// remembered).
        folder: Option<PathBuf>,
    },
    /// Install an addon from a source: the path of a zip in hand, or a source spec (the current release's
    /// asset for the flavor; an addon installed already at that version is left as it is):
    /// `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>` or `curse:<id>`; a zip in hand with `--as <spec>`
    /// records that source, so the update check asks it.
    Add {
        /// The source spec (`github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>`, `curse:<id>`, or a page
        /// address: github.com/<owner>/<repo>, www.wowinterface.com/downloads/info<id>,
        /// www.curseforge.com/projects/<id>, or addons.wago.io/addons/<slug>, recorded as `wago:<id>`), or a
        /// zip path (a `.zip` name or an existing file).
        source: String,
        /// The flavor to install into: `forever`, `retail`, `classic`, `era`, a `.flavor.info` id or a
        /// flavor folder. The installed build is read from the game folder (`MRGL_ROOT` or `mrgl root`)
        /// when there is one, which decides the game type of the progressing classic.
        #[arg(long)]
        flavor: Option<String>,
        /// Resolve the release and report the pick, write nothing.
        #[arg(long)]
        dry_run: bool,
        /// The bound on what the zip unpacks to, `2G` unless given: `3G`, `500M` or a number of bytes
        /// (steps of 1024), for a known large addon.
        #[arg(long, value_name = "SIZE", value_parser = parse_max_unpacked)]
        max_unpacked: Option<u64>,
        /// With a zip in hand: the source spec to record for it (`curse:<id>`, `wowi:<id>`, `wago:<id>`,
        /// `github:<owner>/<repo>`) instead of the zip's path, so the update check asks that source; for a
        /// CurseForge project whose author allows no download outside CurseForge, the zip from its page.
        #[arg(long = "as", value_name = "SPEC")]
        as_spec: Option<String>,
    },
    /// Check the managed addons against their sources, each on its own channel, and update them after one
    /// confirmation: a pinned addon is reported and left alone, an ignored one skipped.
    Update {
        /// Every managed addon of the flavor.
        #[arg(long, conflicts_with = "name", required_unless_present = "name")]
        all: bool,
        /// One managed addon by name, or one of its folders.
        name: Option<String>,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Check and report, write nothing.
        #[arg(long)]
        dry_run: bool,
        /// Update without asking (scripts; needed without a terminal and with --json).
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
    /// Link the addons of a development folder, or one addon folder, into a flavor with junctions (no
    /// admin rights needed).
    Link {
        /// A folder whose subfolders are addons (each with its TOC), or an addon folder (a TOC named after
        /// it).
        folder: PathBuf,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Remove the junctions `link` made.
    Unlink {
        /// The development folder or the addon folder `link` was given.
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
        /// The bound on what the restored backup unpacks to, `2G` unless given: `3G`, `500M` or a
        /// number of bytes (steps of 1024).
        #[arg(
            long,
            value_name = "SIZE",
            value_parser = parse_max_unpacked,
            requires = "restore",
            conflicts_with = "list"
        )]
        max_unpacked: Option<u64>,
    },
    /// Print a diagnostic report for a bug report: the game root, each flavor's build, AddOns folder,
    /// sidecar, cache and client, the WebView2 version, the GitHub rate limit and which API keys are set
    /// (never their values); paths under your profile folder show as `~`.
    Doctor {
        /// Report one flavor only: `forever`, `retail`, `classic`, `era` or a `.flavor.info` id.
        #[arg(long)]
        flavor: Option<String>,
        /// The World of Warcraft folder; otherwise the one every command resolves (`MRGL_ROOT`, the
        /// remembered one, the registry, the usual install folders).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Write the flavor's managed addons (source and version of each) into a list file, or print it; an
    /// addon from a zip in hand or a linked folder is left out, so the list can be shared.
    Export {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// The list file to write; without it the list is printed.
        #[arg(long, value_name = "FILE")]
        to: Option<PathBuf>,
    },
    /// Install the addons of a list file `export` wrote: every `github:` entry the flavor does not have
    /// yet, at its current release, or with `--exact` at the version the list names.
    /// A list that names a zip or another local path is refused.
    Import {
        /// The list file.
        file: PathBuf,
        /// The flavor id or folder to install into.
        #[arg(long)]
        flavor: Option<String>,
        /// Install the version the list names for each entry (its release), not the current release; an
        /// entry whose version the source no longer offers is reported and skipped.
        #[arg(long)]
        exact: bool,
        /// Resolve every entry and report what would happen, write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Pin a managed addon at a version: the update check reports newer releases and leaves it alone.
    Pin {
        /// The addon by name, or one of its folders.
        name: String,
        /// The version to pin at; without it, the installed version.
        version: Option<String>,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Unpin a managed addon: updates reach it again.
    Unpin {
        /// The addon by name, or one of its folders.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Ignore a managed addon: the update check skips it and counts it.
    Ignore {
        /// The addon by name, or one of its folders.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Stop ignoring a managed addon.
    Unignore {
        /// The addon by name, or one of its folders.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Put a managed addon on a release channel: `prerelease` follows the beta builds of its GitHub
    /// releases, `stable` (the default) takes a prerelease only when there is nothing else.
    Channel {
        /// The addon by name, or one of its folders.
        name: String,
        /// `stable` or `prerelease`.
        #[arg(value_parser = parse_channel)]
        channel: Channel,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Roll a managed addon back to its previous version from the zip mrgl kept, and pin it there
    /// (`unpin` lets updates reach it again; a second rollback goes forward again).
    Rollback {
        /// The addon by name, or one of its folders.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// The bound on what the kept zip unpacks to, `2G` unless given: `3G`, `500M` or a number of
        /// bytes (steps of 1024), for a known large addon.
        #[arg(long, value_name = "SIZE", value_parser = parse_max_unpacked)]
        max_unpacked: Option<u64>,
    },
    /// List a flavor's SavedVariables files by size, per account and per character, and mark the
    /// orphans: files whose addon folder is no longer under Interface\AddOns (a disabled addon keeps its
    /// folder and is never one).
    Savedvars {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Move the orphans and their .bak files into backups\<flavor-id>\SavedVariables-<date>.zip under
        /// the mrgl home (%LOCALAPPDATA%\mrgl); unzip it into WTF to put them back. Refused while the
        /// client runs.
        #[arg(long)]
        clean: bool,
    },
    /// Addon sets: save the enabled and disabled addons of the characters' AddOns.txt under a name, apply
    /// a set to every character, undo the last apply, edit, list, show or remove sets (the toolbox). An
    /// apply and an undo are refused while the client runs.
    Set {
        #[command(subcommand)]
        action: SetAction,
    },
    /// Report where the unmanaged addons of a flavor come from: per addon (its folders together) the sources
    /// its TOCs name (X-Wago-ID, X-WoWI-ID, X-Curse-Project-ID, X-Website on github.com), then the curated
    /// GitHub mapping, each asked whether it publishes a build for the flavor, and how many addons can be
    /// adopted without CurseForge, only with it, or have no known source; a source without its key reads
    /// "needs <KEY NAME>". With --apply, mrgl manages them from then on: a folder belongs to one manager.
    Adopt {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Write a sidecar entry for every addon with a source this run can ask (its source, its TOC
        /// version, its folders and their file hashes): no download and no folder changed, the next update
        /// installs each from its source. A CurseForge-only addon waits for its source.
        #[arg(long)]
        apply: bool,
        /// Adopt in a folder the CurseForge app manages (its .curseclient marker), which --apply refuses
        /// otherwise; the app may still update those folders, and its marker stays.
        #[arg(long, requires = "apply")]
        alongside_curseforge: bool,
    },
    /// The API keys of the sources (github, wago, curse): store one in the Windows Credential Manager from
    /// the standard input, remove it, or list where each comes from. A key is never an argument and never
    /// printed; MRGL_GITHUB_TOKEN, MRGL_WAGO_KEY and MRGL_CURSEFORGE_KEY in the environment come first.
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
}

/// What `mrgl key` does (task 093).
#[derive(Subcommand, Debug)]
enum KeyAction {
    /// Store a source's key as mrgl:<source> in the Windows Credential Manager, read from the standard
    /// input (pipe it in: `Get-Clipboard | mrgl key set wago`; in a terminal, paste it and press Enter).
    Set {
        /// The source: github, wago or curse.
        source: String,
        /// Anything after the source: refused without being echoed, a key is never an argument.
        #[arg(hide = true, num_args = 0.., allow_hyphen_values = true)]
        rest: Vec<String>,
    },
    /// Remove a source's key from the Windows Credential Manager.
    Clear {
        /// The source: github, wago or curse.
        source: String,
    },
    /// Every key by name and where it comes from: the environment, the credential store, or not set.
    List,
}

/// A `--max-unpacked` size (task 039): `3G`, `500M`, a number of bytes ([`zip_entry::parse_size`]).
fn parse_max_unpacked(text: &str) -> Result<u64, String> {
    zip_entry::parse_size(text).ok_or_else(|| {
        format!("not a size: {text}: a whole number of bytes, or with K, M, G or T (3G, 500M)")
    })
}

/// A channel name on the command line: `stable` or `prerelease` (also `beta`).
fn parse_channel(text: &str) -> Result<Channel, String> {
    Channel::parse(text).ok_or_else(|| format!("unknown channel {text}: stable or prerelease"))
}

/// Where the command line writes.
#[derive(Debug, Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

/// Windows' `ERROR_NO_DATA`, "the pipe is being closed": a write into a pipe whose reader is gone.
#[cfg(windows)]
const ERROR_NO_DATA: i32 = 232;

/// Whether a write failed because the reader closed the pipe (`mrgl list --json | head -c 200`).
fn is_closed_pipe(error: &std::io::Error) -> bool {
    #[cfg(windows)]
    if error.raw_os_error() == Some(ERROR_NO_DATA) {
        return true;
    }
    error.kind() == std::io::ErrorKind::BrokenPipe
}

/// Whether an addon's install is in flight (task 084): a closed pipe then waits for it ([`InFlight`]).
static IN_FLIGHT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Whether a write found the pipe closed while an install was in flight.
static PIPE_CLOSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// An addon's install in flight (task 084): while the guard lives, a closed pipe does not end the run at
/// the next write ([`emit`] drops the line and goes on), so the swap, the sidecar write and the removal of
/// the run's staging folders finish; when the guard goes, a pipe that closed meanwhile ends the run
/// quietly with code 0, before the next addon starts.
struct InFlight;

impl InFlight {
    fn begin() -> InFlight {
        IN_FLIGHT.store(true, std::sync::atomic::Ordering::SeqCst);
        InFlight
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT.store(false, std::sync::atomic::Ordering::SeqCst);
        if PIPE_CLOSED.load(std::sync::atomic::Ordering::SeqCst) {
            exit_quietly();
        }
    }
}

/// Ends the run with code 0 and not a word more: the reader of the pipe is gone. Under cargo-llvm-cov the
/// profile is written first: `std::process::exit` ends the process without the C runtime's exit handlers,
/// one of which writes the profile, so a run that ends here would otherwise leave none and its lines would
/// read as never run.
fn exit_quietly() -> ! {
    #[cfg(coverage)]
    {
        unsafe extern "C" {
            fn __llvm_profile_write_file() -> i32;
        }
        // SAFETY: the profile runtime is linked into every instrumented build; the call takes nothing and
        // writes the counters of this process to the profile file.
        unsafe {
            __llvm_profile_write_file();
        }
    }
    std::process::exit(0)
}

/// The one writer of the command line (task 075): every line and document goes to stdout or stderr
/// through here. A closed pipe ends the run: nobody is left to read the rest, so the process exits quietly
/// with code 0 instead of panicking as `println!` does; while an install is in flight the exit waits for
/// it to finish ([`InFlight`], task 084). Any other write error panics as `println!` would.
fn emit(stream: Stream, text: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let written = match stream {
        Stream::Stdout => {
            let mut out = std::io::stdout().lock();
            out.write_fmt(text).and_then(|()| out.flush())
        }
        Stream::Stderr => {
            let mut err = std::io::stderr().lock();
            err.write_fmt(text).and_then(|()| err.flush())
        }
    };
    if let Err(error) = written {
        if is_closed_pipe(&error) {
            if IN_FLIGHT.load(std::sync::atomic::Ordering::SeqCst) {
                PIPE_CLOSED.store(true, std::sync::atomic::Ordering::SeqCst);
                return;
            }
            exit_quietly();
        }
        let name = match stream {
            Stream::Stdout => "stdout",
            Stream::Stderr => "stderr",
        };
        panic!("failed printing to {name}: {error}");
    }
}

/// Prints the two-line messages, the plain data lines, or JSON when asked, through [`emit`].
struct Reporter {
    voice: Voice,
    json: bool,
}

impl Reporter {
    fn say(&self, kind: Kind, plain: impl std::fmt::Display) {
        if self.json {
            let value = serde_json::json!({ "kind": kind.as_str(), "message": plain.to_string() });
            emit(Stream::Stdout, format_args!("{value}\n"));
        } else {
            emit(
                Stream::Stdout,
                format_args!("{}\n", self.voice.say(kind, plain)),
            );
        }
    }

    fn fail(&self, plain: impl std::fmt::Display) {
        if self.json {
            let value =
                serde_json::json!({ "kind": Kind::Error.as_str(), "message": plain.to_string() });
            emit(Stream::Stderr, format_args!("{value}\n"));
        } else {
            self.say_on_stderr(Kind::Error, plain);
        }
    }

    /// A two-line message on stderr whatever the mode (the warnings and the summary of `export` without a
    /// file, whose stdout is the list).
    fn say_on_stderr(&self, kind: Kind, plain: impl std::fmt::Display) {
        emit(
            Stream::Stderr,
            format_args!("{}\n", self.voice.say(kind, plain)),
        );
    }

    /// A plain data line (one addon of a listing); human mode only, the JSON document carries the data.
    fn line(&self, text: impl std::fmt::Display) {
        emit(Stream::Stdout, format_args!("{text}\n"));
    }

    /// The one JSON document of a command; its `kind` is a fixed name.
    fn document(&self, value: serde_json::Value) {
        debug_assert!(value.get("kind").is_some(), "a document names its kind");
        emit(Stream::Stdout, format_args!("{value}\n"));
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
            emit(
                Stream::Stdout,
                format_args!(
                    "{}\n",
                    serde_json::json!({ "name": "mrgl", "version": VERSION })
                ),
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
    match command {
        Command::Scan { folder } => {
            let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
            report_scan(reporter, "scan", &scan, Vec::new(), &[], None, None);
        }
        Command::List { flavor, root } => {
            let mut warnings = Vec::new();
            let (folder, root) = flavor_folder_and_root(flavor.as_deref(), root, &mut warnings)?;
            let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
            let dependencies = check_dependencies(&scan, &mut warnings);
            let forever = forever_statuses(&scan, &mut warnings);
            let state = list_state(&scan.flavor, &mut warnings);
            let view = ListView {
                state: &state,
                root: root.as_ref(),
            };
            report_scan(
                reporter,
                "list",
                &scan,
                warnings,
                &dependencies,
                forever.as_ref(),
                Some(view),
            );
        }
        Command::Root { folder } => root_command(reporter, folder)?,
        Command::Add {
            source,
            flavor,
            dry_run,
            max_unpacked,
            as_spec,
        } if is_zip_in_hand(&source) => add_zip(
            reporter,
            &source,
            flavor.as_deref(),
            dry_run,
            max_unpacked,
            as_spec.as_deref(),
        )?,
        Command::Add {
            as_spec: Some(_), ..
        } => return Err(AS_NEEDS_A_ZIP.to_string()),
        Command::Add {
            source,
            flavor,
            dry_run,
            max_unpacked,
            as_spec: None,
        } => add_command(reporter, &source, flavor.as_deref(), dry_run, max_unpacked)?,
        Command::Remove { name, flavor } => remove_command(reporter, &name, flavor.as_deref())?,
        Command::Backup {
            flavor,
            to,
            list,
            restore,
            yes,
            max_unpacked,
        } => backup_command(
            reporter,
            flavor.as_deref(),
            to,
            list,
            restore,
            yes,
            max_unpacked,
        )?,
        Command::Doctor { flavor, root } => doctor_command(reporter, flavor.as_deref(), root)?,
        Command::Link { folder, flavor } => link_command(reporter, &folder, flavor.as_deref())?,
        Command::Unlink { folder, flavor } => unlink_command(reporter, &folder, flavor.as_deref())?,
        Command::Export { flavor, to } => export_command(reporter, flavor.as_deref(), to)?,
        Command::Import {
            file,
            flavor,
            exact,
            dry_run,
        } => import_command(reporter, &file, flavor.as_deref(), exact, dry_run)?,
        Command::Pin {
            name,
            version,
            flavor,
        } => mark_command(reporter, &name, flavor.as_deref(), Mark::Pin(version))?,
        Command::Unpin { name, flavor } => {
            mark_command(reporter, &name, flavor.as_deref(), Mark::Unpin)?
        }
        Command::Ignore { name, flavor } => {
            mark_command(reporter, &name, flavor.as_deref(), Mark::Ignore)?
        }
        Command::Unignore { name, flavor } => {
            mark_command(reporter, &name, flavor.as_deref(), Mark::Unignore)?
        }
        Command::Channel {
            name,
            channel,
            flavor,
        } => mark_command(reporter, &name, flavor.as_deref(), Mark::Channel(channel))?,
        Command::Rollback {
            name,
            flavor,
            max_unpacked,
        } => rollback_command(reporter, &name, flavor.as_deref(), max_unpacked)?,
        Command::Savedvars { flavor, clean } => {
            savedvars_command(reporter, flavor.as_deref(), clean)?
        }
        Command::Update {
            all: _,
            name,
            flavor,
            dry_run,
            yes,
        } => update_command(reporter, name.as_deref(), flavor.as_deref(), dry_run, yes)?,
        Command::Set { action } => set_command(reporter, action)?,
        Command::Key { action } => key_command(
            reporter,
            action,
            &*keys::store_of_process(),
            &|name| std::env::var_os(name),
            &mut std::io::stdin().lock(),
            std::io::stdin().is_terminal(),
        )?,
        Command::Adopt {
            flavor,
            apply,
            alongside_curseforge,
        } => adopt_command(reporter, flavor.as_deref(), apply, alongside_curseforge)?,
    }
    Ok(())
}

/// `mrgl key set|clear <source>` and `mrgl key list` (task 093): a key from the standard input into the
/// key store (the Windows Credential Manager, `mrgl:<source>`), out of it, or every key by name with where
/// it comes from. The value is never an argument and never printed, in the text, the JSON or an error.
/// The store, the environment and the input are the process's ([`run`]); `terminal` says whether the
/// input is one, which `set` prompts and reads one line from (a pipe is read whole).
fn key_command(
    reporter: &Reporter,
    action: KeyAction,
    store: &dyn keys::KeyStore,
    env: &dyn Fn(&str) -> Option<std::ffi::OsString>,
    input: &mut dyn BufRead,
    terminal: bool,
) -> Result<(), String> {
    let in_env =
        |name: &str| env(name).is_some_and(|value| !value.to_string_lossy().trim().is_empty());
    match action {
        KeyAction::List => {
            let states = keys::list(env, store);
            let available = store.available().err().map(|error| error.to_string());
            let set = states.iter().filter(|state| state.origin.is_set()).count();
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "keys",
                    "api_keys": &states,
                    "store": store.name(),
                    "store_unavailable": available,
                }));
                return Ok(());
            }
            for state in &states {
                reporter.line(state.text());
            }
            if let Some(reason) = &available {
                reporter.say(Kind::Warning, reason);
            }
            let kind = if set == 0 {
                Kind::Nothing
            } else {
                Kind::Success
            };
            reporter.say(
                kind,
                format!("{set} of {} keys set, none shown", states.len()),
            );
            Ok(())
        }
        KeyAction::Set { source, rest } => {
            if !rest.is_empty() {
                return Err(format!(
                    "a key is never an argument, the shell's history would keep it: pipe it in (Get-Clipboard | mrgl key set {source}); nothing stored"
                ));
            }
            let kind = keys::keyed_source(&source).map_err(|error| error.to_string())?;
            let name = keys::name_of(kind).unwrap_or_default();
            let target = keys::target_name(kind);
            store.available().map_err(|error| error.to_string())?;
            if terminal && !reporter.json {
                reporter.say(
                    Kind::Progress,
                    format!(
                        "paste the {name} and press Enter; it goes to {}",
                        store.name()
                    ),
                );
            }
            let value = keys::read_key(input, terminal).map_err(|error| error.to_string())?;
            keys::set(store, kind, &value).map_err(|error| error.to_string())?;
            let warnings: Vec<String> = in_env(name)
                .then(|| format!("{name} is set in the environment too, which comes first"))
                .into_iter()
                .collect();
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "key",
                    "action": "set",
                    "source": kind,
                    "name": name,
                    "target": target,
                    "changed": true,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            for warning in &warnings {
                reporter.say(Kind::Warning, warning);
            }
            reporter.say(
                Kind::Success,
                format!("{name} stored in {} as {target}", store.name()),
            );
            Ok(())
        }
        KeyAction::Clear { source } => {
            let kind = keys::keyed_source(&source).map_err(|error| error.to_string())?;
            let name = keys::name_of(kind).unwrap_or_default();
            let target = keys::target_name(kind);
            let removed = keys::clear(store, kind).map_err(|error| error.to_string())?;
            let warnings: Vec<String> = in_env(name)
                .then(|| format!("{name} is still set in the environment"))
                .into_iter()
                .collect();
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "key",
                    "action": "clear",
                    "source": kind,
                    "name": name,
                    "target": target,
                    "changed": removed,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            for warning in &warnings {
                reporter.say(Kind::Warning, warning);
            }
            if removed {
                reporter.say(
                    Kind::Success,
                    format!("{name} removed from {} ({target})", store.name()),
                );
            } else {
                reporter.say(
                    Kind::Nothing,
                    format!("no {name} in {} ({target}), nothing removed", store.name()),
                );
            }
            Ok(())
        }
    }
}

/// `mrgl adopt --flavor <flavor> [--apply [--alongside-curseforge]]` (tasks 097, 098): the source of every
/// unmanaged addon of the flavor, through the router and the compiled-in GitHub mapping
/// ([`adopt::report`]): a line per addon and one per source under it, the `.curseclient` marker and the
/// warnings, and the summary of the counts; `--json` is the `adopt` document. Without `--apply` it reads the
/// sidecar and writes nothing (the sources keep their own caches); with it, [`adopt::apply`] puts an entry
/// per adoptable addon into the sidecar, which is written once when one was adopted. A flavor folder with the
/// CurseForge app's marker refuses `--apply` before anything is asked unless `--alongside-curseforge` says
/// so, which is then a warning; the marker is never touched.
fn adopt_command(
    reporter: &Reporter,
    spec: Option<&str>,
    apply: bool,
    alongside_curseforge: bool,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let folder = flavor_folder(spec, None, &mut warnings)?;
    let scan = scan::scan(&folder).map_err(|error| error.to_string())?;
    if apply && scan.flavor.curse_client && !alongside_curseforge {
        return Err(adopt::marker_refusal(&scan.flavor, adopt::AdoptBy::Command));
    }
    warnings.extend(scan.flavor.warnings.iter().cloned());
    warnings.extend(scan.warnings.iter().cloned());
    let engine = Engine::new(scan.flavor.clone()).map_err(|error| error.to_string())?;
    let loaded = engine.load_state().map_err(|error| error.to_string())?;
    warnings.extend(loaded.warning);
    let mut state = loaded.state;
    state.reconcile(&scan);
    let sources = Sources::from_env();
    let report = adopt::report(&scan, &state, &sources, &|kind| {
        keys::of_process(kind).is_some()
    });
    warnings.extend(report.left_out_lines());
    let applied = if apply {
        let applied = adopt::apply(&report, &mut state);
        if !applied.adopted.is_empty() {
            engine
                .home
                .write_state(&state)
                .map_err(|error| error.to_string())?;
        }
        Some(applied)
    } else {
        None
    };
    // The marker (task 098): the report names it; an apply alongside the CurseForge app warns instead.
    let marker = if apply && report.curse_client {
        Some(adopt::alongside_warning(&report.flavor))
    } else {
        report.marker_line(adopt::AdoptBy::Command)
    };
    if reporter.json {
        let applied = applied.clone().unwrap_or_default();
        reporter.document(serde_json::json!({
            "kind": "adopt",
            "flavor": &report.flavor,
            "curse_client": report.curse_client,
            "apply": apply,
            "addons": &report.addons,
            "left_out": &report.left_out,
            "counts": &report.counts,
            "adopted": &applied.adopted,
            "skipped": &applied.skipped,
            "warnings": marker.into_iter().chain(warnings).collect::<Vec<_>>(),
        }));
        return Ok(());
    }
    let flavor_name = report.flavor_name();
    for addon in &report.addons {
        reporter.line(addon.line(flavor_name));
        for line in addon.detail_lines(flavor_name) {
            reporter.line(line);
        }
    }
    if let Some(applied) = &applied {
        for adopted in &applied.adopted {
            reporter.line(adopted.line());
        }
    }
    if let Some(line) = marker {
        reporter.say(Kind::Warning, line);
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, summary) = match &applied {
        Some(applied) => {
            for skipped in &applied.skipped {
                reporter.say(Kind::Warning, skipped.line());
            }
            report.applied_summary(applied)
        }
        None => report.summary(adopt::AdoptBy::Command),
    };
    reporter.say(kind, summary);
    Ok(())
}

/// The flavor folder a `--flavor` spec names: a friendly name or an id under the game root, or a folder.
fn flavor_folder(
    spec: Option<&str>,
    root_flag: Option<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<PathBuf, String> {
    flavor_folder_and_root(spec, root_flag, warnings).map(|(folder, _)| folder)
}

/// [`flavor_folder`] with the game root it was found under (and where that came from); `None` for a
/// flavor folder given as a path.
fn flavor_folder_and_root(
    spec: Option<&str>,
    root_flag: Option<PathBuf>,
    warnings: &mut Vec<String>,
) -> Result<(PathBuf, Option<GameRoot>), String> {
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
                "no game root for flavor {spec}: none in the registry or the usual install folders; pass --root <folder>, set {ROOT_ENV} or run `mrgl root <folder>`"
            )
        })?;
        if root.origin.is_discovered() {
            warnings.push(root.found_line());
        }
        return Ok((root.path.join(folder_name), Some(root)));
    }
    let path = Path::new(spec);
    if path.is_dir() {
        return Ok((path.to_path_buf(), None));
    }
    Err(format!(
        "unknown flavor: {spec} (forever, retail, classic, era, a .flavor.info id or a flavor folder)"
    ))
}

/// The game root every command resolves the same way (task 020, `discover::resolve_root_of_process`):
/// the flag, then `MRGL_ROOT`, then the remembered one, then the registry (the launcher's key, the
/// game's Uninstall entry) and the usual install folders; `None` when nothing names it. A config warning is pushed; a discovered root carries
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

/// What `list` lays over a scan (task 037): the flavor's state sidecar and the game root the flavor was
/// found under. `scan` has none of it and stays the raw folder view.
struct ListView<'a> {
    state: &'a State,
    root: Option<&'a GameRoot>,
}

/// The flavor's sidecar for `list`, read through the install engine (D-003). A missing sidecar is a fresh
/// state, and so is a corrupt one, rebuilt without a word: the scan is the truth, every addon reads as
/// unmanaged, and the next install or removal rewrites the file and says so once. A sidecar that cannot
/// be read at all (no mrgl home, a locked file) is a warning, and every addon reads as unmanaged too.
fn list_state(flavor: &Flavor, warnings: &mut Vec<String>) -> State {
    match Engine::new(flavor.clone()).and_then(|engine| engine.load_state()) {
        Ok(loaded) => loaded.state,
        Err(error) => {
            warnings.push(format!(
                "state sidecar not read, every addon listed as unmanaged: {error}"
            ));
            State::new(&flavor.id)
        }
    }
}

/// Prints a scan: the addon lines, the warnings and the summary, or the one JSON document. A client
/// running from the flavor folder (task 023) is named in the summary and is `client_running` in the JSON.
/// `dependencies` is the dependency check of the scan (task 029; empty for `scan`, which reads the folder
/// only). A Forever listing (task 011) marks every addon through [`mark_forever`].
/// With the flavor's sidecar (`list`), a pinned, an ignored and a prerelease addon carry their marks
/// (task 021).
/// With a [`ListView`] (`list`, task 037) every addon is managed or unmanaged, the summary counts both,
/// and the JSON carries the counts and the game root with its origin.
fn report_scan(
    reporter: &Reporter,
    kind: &str,
    scan: &Scan,
    mut warnings: Vec<String>,
    dependencies: &[AddonDependencies],
    forever: Option<&BTreeMap<String, ForeverStatus>>,
    list: Option<ListView>,
) {
    warnings.extend(scan.flavor.warnings.iter().cloned());
    warnings.extend(scan.warnings.iter().cloned());
    let client = match client::client_running(&scan.flavor) {
        Ok(client) => client,
        // coverage: os-begin — the system's process list could not be taken
        Err(error) => {
            warnings.push(error.to_string());
            None
        } // coverage: os-end
    };
    let mismatches = interface::check_scan(scan);
    let out_of_date = interface::out_of_date_scan(scan);
    // `classify` keeps the scan's order: one status per addon, or none at all for `scan`.
    let statuses = list.as_ref().map(|view| view.state.classify(scan));
    let status_of = |index: usize| statuses.as_ref().map(|statuses| &statuses[index]);
    let managed = statuses
        .as_ref()
        .map(|statuses| statuses.iter().filter(|status| status.is_managed()).count());
    let mut lines = Vec::new();
    let mut objects = Vec::new();
    for (index, addon) in scan.addons.iter().enumerate() {
        let mut line = addon_line(addon, status_of(index));
        let mut object = addon_json(addon, status_of(index));
        // The marks of task 021 (pinned, ignored, prerelease) come first, from the sidecar entry.
        if let Some(view) = &list {
            let entry = view
                .state
                .entry_for_folder(&addon.folder)
                .map(|(_, entry)| entry);
            let (suffix, fields) = marks(entry);
            line.push_str(&suffix);
            for (field, value) in fields {
                object[field] = value;
            }
        }
        mark_dependencies(dependencies, &addon.folder, &mut line, &mut object);
        mark_forever(forever, addon, Some(&mut line), Some(&mut object));
        lines.push(with_interface_note(line, addon, &mismatches, &out_of_date));
        objects.push(object);
    }
    if reporter.json {
        let mut document = serde_json::json!({
            "kind": kind,
            "flavor": &scan.flavor,
            "addons": objects,
            "skipped": &scan.skipped,
            "client_running": client,
            "interface_mismatches": &mismatches,
            "interface_out_of_date": &out_of_date,
            "warnings": warnings,
        });
        if let (Some(view), Some(managed)) = (&list, managed) {
            document["counts"] = serde_json::json!({
                "addons": scan.addons.len(),
                "managed": managed,
                "unmanaged": scan.addons.len() - managed,
            });
            document["root"] = serde_json::json!(view.root.map(|root| &root.path));
            document["root_origin"] = serde_json::json!(view.root.map(|root| root.origin));
        }
        reporter.document(document);
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
    if let Some(warning) = interface_summary(&mismatches) {
        reporter.say(Kind::Warning, warning);
    }
    if let Some(warning) = out_of_date_summary(&out_of_date) {
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
    if let Some(managed) = managed.filter(|_| !scan.addons.is_empty()) {
        summary.push_str(&format!(
            ": {managed} managed by mrgl, {} unmanaged",
            scan.addons.len() - managed
        ));
    }
    match scan.skipped.len() {
        0 => {}
        1 => summary.push_str(", 1 folder without a TOC skipped"),
        count => summary.push_str(&format!(", {count} folders without a TOC skipped")),
    }
    if let Some(view) = &list {
        summary.push_str(&marks_summary(scan, view.state));
    }
    let kind = if scan.addons.is_empty() {
        Kind::Nothing
    } else {
        Kind::Success
    };
    reporter.say(kind, summary);
}

/// The marks of task 021 on one addon of `list`, the one place they are spelled: the suffix of its line
/// (`pinned at 1.2.0`, `ignored`, `prerelease`, each after two spaces like the line's other parts; empty
/// for an unmarked or unmanaged addon) and the fields of its JSON object (`pinned`, a version or null;
/// `ignored`; `channel`, null for an unmanaged addon).
fn marks(entry: Option<&Entry>) -> (String, serde_json::Map<String, serde_json::Value>) {
    let mut suffix = String::new();
    if let Some(pinned) = entry.and_then(Entry::pinned) {
        suffix.push_str(&format!("  pinned at {pinned}"));
    }
    if entry.is_some_and(Entry::ignored) {
        suffix.push_str("  ignored");
    }
    if entry.map(Entry::channel) == Some(Channel::Prerelease) {
        suffix.push_str("  prerelease");
    }
    let mut fields = serde_json::Map::new();
    fields.insert(
        "pinned".into(),
        serde_json::json!(entry.and_then(Entry::pinned)),
    );
    fields.insert(
        "ignored".into(),
        serde_json::json!(entry.is_some_and(Entry::ignored)),
    );
    fields.insert(
        "channel".into(),
        serde_json::json!(entry.map(Entry::channel)),
    );
    (suffix, fields)
}

/// The counts of task 021 for `list`'s summary, one per managed addon whatever its folder count:
/// `, 1 pinned, 1 ignored, 1 on the prerelease channel`, each only when it is not zero.
fn marks_summary(scan: &Scan, state: &State) -> String {
    let mut names: Vec<&str> = scan
        .addons
        .iter()
        .filter_map(|addon| state.entry_for_folder(&addon.folder))
        .map(|(name, _)| name)
        .collect();
    names.sort_unstable();
    names.dedup();
    let entries: Vec<&Entry> = names
        .iter()
        .filter_map(|name| state.entry(name).map(|(_, entry)| entry))
        .collect();
    let pinned = entries
        .iter()
        .filter(|entry| entry.pinned().is_some())
        .count();
    let ignored = entries.iter().filter(|entry| entry.ignored()).count();
    let prerelease = entries
        .iter()
        .filter(|entry| entry.channel() == Channel::Prerelease)
        .count();
    let mut summary = String::new();
    if pinned > 0 {
        summary.push_str(&format!(", {pinned} pinned"));
    }
    if ignored > 0 {
        summary.push_str(&format!(", {ignored} ignored"));
    }
    if prerelease > 0 {
        summary.push_str(&format!(", {prerelease} on the prerelease channel"));
    }
    summary
}

/// The version an addon line and its JSON object show: the installed one for a managed addon (the
/// sidecar's), the primary TOC's `## Version` otherwise.
fn shown_version<'a>(addon: &'a Addon, status: Option<&Status<'a>>) -> Option<&'a str> {
    match status.and_then(|status| status.entry) {
        Some(entry) => Some(entry.version.as_str()),
        None => addon.version(),
    }
}

/// One addon as a plain line, the one builder of `scan` and `list`: folder, version, interface list,
/// then for `list` (a status) `managed` with the source spec as the sidecar has it (`managed by <name>`
/// for a secondary folder of a managed addon) or `unmanaged`, then the TOC's source hints of an addon
/// mrgl does not manage, then the flags. A later mark is one more part pushed before the join.
fn addon_line(addon: &Addon, status: Option<&Status>) -> String {
    let toc = addon.primary_toc().map(|file| &file.toc);
    let version = shown_version(addon, status).unwrap_or("-").to_string();
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
    match status {
        Some(Status {
            name: Some(name),
            entry: Some(entry),
            ..
        }) => {
            if name.eq_ignore_ascii_case(&addon.folder) {
                parts.push("managed".to_string());
            } else {
                parts.push(format!("managed by {name}"));
            }
            parts.push(entry.source.clone());
        }
        Some(_) => parts.push("unmanaged".to_string()),
        None => {}
    }
    if !hints.is_empty() && !status.is_some_and(Status::is_managed) {
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
    object["dependencies"] = serde_json::json!(check);
}

/// The dependency check of a scan (task 029), with the `nolib` flags of the flavor's sidecar: a sidecar
/// that cannot be read is a warning and leaves every flag unknown. Every addon with a missing required
/// dependency adds its warning line.
fn check_dependencies(scan: &Scan, warnings: &mut Vec<String>) -> Vec<AddonDependencies> {
    let id = &scan.flavor.id;
    // A corrupt sidecar is `list_state`'s to report once (task 037: rebuilt without a word); here it
    // only leaves every flag unknown.
    let state = match FlavorHome::new(id).and_then(|home| home.read_state(id)) {
        Ok(loaded) => Some(loaded.state),
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    let checks = dependencies::check(scan, state.as_ref());
    warnings.extend(checks.iter().filter_map(AddonDependencies::warning));
    checks
}

/// Task 011: the Forever status of every managed addon of a Forever listing, by its primary folder in lower
/// case; `None` for another flavor. Offline: the build signal comes from each source's cache alone (task
/// 092: [`Source::cached_forever_build`]; GitHub's release list and the `release.json` the last `add`
/// fetched), else from what the sidecar recorded at install (task 096: CurseForge, which keeps no cache;
/// [`sources::entry_forever_build`]), so without either, or from a source that does not name Forever, it is
/// `unknown`.
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
    // A corrupt sidecar is `list_state`'s to report (task 037: rebuilt without a word).
    let sources = Sources::from_env();
    for entry in loaded.state.addons.values() {
        let Some(folder) = entry.folders.first() else {
            continue;
        };
        let build = sources::entry_forever_build(&sources, entry);
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
    if let Some(object) = object {
        object["forever"] = serde_json::to_value(status).unwrap_or_default();
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
fn forever_message(status: &ForeverStatus, source: &str) -> (Kind, String) {
    // Each source says it its own way (task 094): GitHub by `release.json`, WoWInterface by its list.
    let kind = SourceSpec::parse(source).map_or(SourceKind::GitHub, |spec| spec.kind());
    let why = sources::forever_why(kind, status.build);
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

/// The interface check of task 015 on an addon line: `interface mismatch` appended when the addon's TOC
/// names no interface of the flavor's build, `out of date` (task 076, the client's words) when it names
/// only lower ones of the build's major version.
fn with_interface_note(
    mut line: String,
    addon: &Addon,
    mismatches: &[InterfaceMismatch],
    out_of_date: &[InterfaceOutOfDate],
) -> String {
    if mismatches
        .iter()
        .any(|mismatch| mismatch.folder == addon.folder)
    {
        line.push_str("  ");
        line.push_str(InterfaceMismatch::LABEL);
    }
    if out_of_date.iter().any(|stale| stale.folder == addon.folder) {
        line.push_str("  ");
        line.push_str(InterfaceOutOfDate::LABEL);
    }
    line
}

/// The warning under a listing with interface mismatches: how many, the build, each folder's numbers.
fn interface_summary(mismatches: &[InterfaceMismatch]) -> Option<String> {
    let first = mismatches.first()?;
    let count = match mismatches.len() {
        1 => "1 addon names".to_string(),
        count => format!("{count} addons name"),
    };
    let each: Vec<String> = mismatches
        .iter()
        .map(|mismatch| format!("{} ({})", mismatch.folder, mismatch.interface_list()))
        .collect();
    Some(format!(
        "{count} no interface of the build {}: {}; made for another flavor or an older version of the game?",
        first.build,
        each.join(", ")
    ))
}

/// The warning under a listing with out-of-date addons (task 076): how many, the build and its interface,
/// each folder's highest number of the build's major.
fn out_of_date_summary(out_of_date: &[InterfaceOutOfDate]) -> Option<String> {
    let first = out_of_date.first()?;
    let (count, them) = match out_of_date.len() {
        1 => ("1 addon is".to_string(), "it"),
        count => (format!("{count} addons are"), "them"),
    };
    let each: Vec<String> = out_of_date
        .iter()
        .map(|stale| format!("{} ({})", stale.folder, stale.highest))
        .collect();
    Some(format!(
        "{count} out of date for the build {} (interface {}): {}; the client lists {them} as out of date",
        first.build,
        first.interface,
        each.join(", ")
    ))
}

/// One addon as a JSON object, the one builder of `scan` and `list`: the scan's fields (`folder`, `path`,
/// `tocs`, `other_tocs`, `chosen`, `linked`), and for `list` (a status) `managed`, `name` (the managed
/// addon that owns the folder), `source` (its spec as the sidecar has it) and `version` (the installed
/// one, else the TOC's `## Version`), `null` where unknown. A later field is one more index assignment: an
/// `Addon` serializes to an object, and `Null` would become one on the first field.
fn addon_json(addon: &Addon, status: Option<&Status>) -> serde_json::Value {
    let mut object = serde_json::to_value(addon).unwrap_or_default();
    if let Some(status) = status {
        object["managed"] = status.is_managed().into();
        object["name"] = serde_json::json!(status.name);
        object["source"] = serde_json::json!(status.entry.map(|entry| &entry.source));
        object["version"] = serde_json::json!(shown_version(addon, Some(status)));
    }
    object
}

/// The spec `add` resolves and records for the one typed (task 105): a Wago page address becomes
/// `wago:<id>` through the slug list of the target's game version, and a progress line says so; an unknown
/// slug is the error that names the game version. Every other spec is its own.
fn recorded_spec(
    reporter: &Reporter,
    sources: &Sources,
    spec: SourceSpec,
    target: &Target,
) -> Result<SourceSpec, String> {
    let recorded = sources
        .canonical_spec(&spec, target)
        .map_err(|error| error.to_string())?;
    if recorded != spec && !reporter.json {
        reporter.say(Kind::Progress, format!("{spec} is {recorded}"));
    }
    Ok(recorded)
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
/// pick; without `--dry-run` the pick is downloaded and installed through the engine's one install path
/// (tasks 040 and 055, [`Engine::install_from_source`]), and an addon the sidecar has from that source at
/// that version already is reported as such, with nothing downloaded.
fn add_command(
    reporter: &Reporter,
    source: &str,
    flavor: Option<&str>,
    dry_run: bool,
    max_unpacked: Option<u64>,
) -> Result<(), String> {
    let spec = SourceSpec::parse(source).map_err(|error| error.to_string())?;
    // Task 092: a source this run cannot ask names what is missing before anything else is read.
    let sources = Sources::from_env();
    if let Some(error) = sources::not_configured(&sources, &spec) {
        return Err(error.to_string());
    }
    let mut warnings = Vec::new();
    if !dry_run {
        let engine = with_bound(engine_for(flavor, &mut warnings)?, max_unpacked);
        warnings.extend(adopt::curse_client_warning(&engine.flavor));
        let folder_name = engine.flavor.folder_name();
        let spec = recorded_spec(
            reporter,
            &sources,
            spec,
            &Target::from_flavor(&engine.flavor),
        )?;
        let SourceInstalled { pick, outcome } = engine
            .install_from_source(
                &sources,
                &SourceRequest::current(spec.clone()),
                &mut install_progress(reporter, &engine, &mut warnings),
            )
            .map_err(|error| error.to_string())?;
        // The same companions as `add <file.zip>`, from the one summary helper of task 055: the
        // running-client warning (task 038), the dependency check (task 029) and the interface check
        // (task 015) of the installed folders.
        let (installed, summary, client_warning, kind) = match &outcome {
            PickInstalled::Installed(installed) => (
                true,
                install_summary(&engine, installed, &pick.source, &mut warnings),
                installed.client_warning.clone(),
                Kind::Success,
            ),
            PickInstalled::AlreadyInstalled(current) => {
                warnings.extend(current.warnings.iter().cloned());
                (
                    false,
                    already_installed_summary(current, &folder_name),
                    None,
                    Kind::Nothing,
                )
            }
        };
        // The Forever signal and note of task 011, on the install as on the dry run.
        let forever = (engine.flavor.id == FlavorId::Forever)
            .then(|| ForeverStatus::for_source(&pick.source, pick.forever_build));
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "add",
                "source": &spec,
                "flavor": &engine.flavor.id,
                "build": engine.flavor.version.map(|version| version.to_string()),
                "dry_run": false,
                "pick": &pick,
                "forever": forever,
                "installed": installed,
                "already_installed": !installed,
                "addon": summary.addon,
                "client_warning": client_warning,
                "interface_mismatch": summary.mismatch,
                "interface_out_of_date": summary.out_of_date,
                "warnings": warnings,
            }));
            return Ok(());
        }
        for warning in client_warning.iter().chain(&warnings) {
            reporter.say(Kind::Warning, warning);
        }
        summary.say_interface(reporter, &folder_name);
        if let Some(status) = forever.as_ref() {
            let (kind, text) = forever_message(status, &pick.source);
            reporter.say(kind, text);
        }
        reporter.say(kind, summary.line);
        return Ok(());
    }
    let target = add_target(flavor, &mut warnings)?;
    let name = target.flavor_name().to_string();
    let spec = recorded_spec(reporter, &sources, spec, &target)?;
    if !reporter.json {
        reporter.say(Kind::Progress, format!("resolving {spec} for {name}"));
    }
    let pick = sources
        .resolve(&spec, &target)
        .map_err(|error| error.to_string())?;
    // Task 041: a pick from the cached release list, offline, is a warning on the screen and in the JSON.
    warnings.extend(pick.stale_warning());
    let forever = (target.flavor == FlavorId::Forever)
        .then(|| ForeverStatus::for_source(&pick.source, pick.forever_build));
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "add",
            "source": &spec,
            "flavor": &target.flavor,
            "build": target.version.map(|version| version.to_string()),
            "dry_run": true,
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
        if let Some(status) = forever.as_ref() {
            let (kind, text) = forever_message(status, &pick.source);
            reporter.say(kind, text);
        }
        reporter.say(Kind::Nothing, "dry run: nothing installed");
    }
    Ok(())
}

/// The progress of the one install path (task 055, [`Engine::install_from_source`]), the same lines for
/// every caller: resolving, the pick, downloading, installing. A pick that is a guess is pushed to the
/// warnings instead, so it is in the JSON document too and on the screen with the other warnings; in
/// `--json` mode nothing else is said. The callback holds the install's [`InFlight`] guard (task 084): it
/// is a temporary of the engine call's statement, so a pipe that closes during the install ends the run
/// once the engine call returns, with the addon in flight installed and its staging folders gone.
fn install_progress<'a>(
    reporter: &'a Reporter,
    engine: &Engine,
    warnings: &'a mut Vec<String>,
) -> impl FnMut(PickStep<'_>) + 'a {
    let name = Target::from_flavor(&engine.flavor)
        .flavor_name()
        .to_string();
    let folder_name = engine.flavor.folder_name();
    let flight = InFlight::begin();
    move |step| {
        let _in_flight = &flight;
        // Task 041: a pick from the cached release list, offline, is a warning on the screen and in the JSON.
        if let PickStep::Picked { pick } = step {
            warnings.extend(pick.stale_warning());
        }
        install_step(reporter, &name, &folder_name, warnings, step);
    }
}

/// One progress step of [`install_progress`].
fn install_step(
    reporter: &Reporter,
    name: &str,
    folder_name: &str,
    warnings: &mut Vec<String>,
    step: PickStep<'_>,
) {
    match step {
        PickStep::Picked { pick } if pick.guess => {
            warnings.push(format!("pick for {name}, a guess: {}", pick.summary()));
        }
        _ if reporter.json => {}
        PickStep::Resolving { spec, .. } => {
            reporter.say(Kind::Progress, format!("resolving {spec} for {name}"));
        }
        PickStep::Picked { pick } => reporter.say(
            Kind::Progress,
            format!("pick for {name}: {}", pick.summary()),
        ),
        PickStep::Downloading { pick, .. } => {
            reporter.say(Kind::Progress, format!("downloading {}", pick.asset));
        }
        PickStep::Installing { pick, .. } => reporter.say(
            Kind::Progress,
            format!("installing {} into {folder_name}", pick.asset),
        ),
    }
}

/// What every install path reports about an addon it installed (task 055): the `addon` block of the
/// JSON document, the interface check of the primary folder (tasks 015, 082: a mismatch or out of date)
/// and the plain summary line.
struct InstallSummary {
    addon: serde_json::Value,
    mismatch: Option<InterfaceMismatch>,
    out_of_date: Option<InterfaceOutOfDate>,
    line: String,
}

impl InstallSummary {
    /// The interface check's warning after the install, at most one line (an addon is a mismatch or out
    /// of date, never both).
    fn interface_lines(&self, folder_name: &str) -> Vec<String> {
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

    /// Says the interface check's warning through the voice.
    fn say_interface(&self, reporter: &Reporter, folder_name: &str) {
        for line in self.interface_lines(folder_name) {
            reporter.say(Kind::Warning, line);
        }
    }
}

/// The one summary of an install, for `add <file.zip>`, `add github:` and `update` (task 055): the
/// engine's warnings and those of the dependency check of the installed folders (task 029) go to
/// `warnings`; the `addon` block names the addon as the sidecar records it now; the line is `installed
/// <name> <version> (<folders>) into <flavor folder>`, `updated <name> from <previous> to <version>
/// (<folders>) in <flavor folder>`, or `reinstalled <name> <version> (<folders>) in <flavor folder>` for the
/// version the sidecar recorded already (task 083), with the folders of the previous version that went.
fn install_summary(
    engine: &Engine,
    installed: &Installed,
    source: &str,
    warnings: &mut Vec<String>,
) -> InstallSummary {
    warnings.extend(installed.warnings.iter().cloned());
    let checks = engine.check_installed(&installed.folders);
    warnings.extend(checks.warnings.iter().cloned());
    warnings.extend(
        checks
            .dependencies
            .iter()
            .filter_map(AddonDependencies::warning),
    );
    let InstalledChecks {
        dependencies,
        mismatch,
        out_of_date,
        ..
    } = checks;
    let addon = serde_json::json!({
        "name": installed.name,
        "version": installed.version,
        "outcome": installed.outcome,
        "previous_version": installed.previous_version,
        "source": source,
        "folders": installed.folders,
        "removed_folders": installed.removed_folders,
        "dependencies": dependencies,
    });
    let folder_name = engine.flavor.folder_name();
    let folders = format!(
        "{}: {}",
        count_folders(installed.folders.len()),
        installed.folders.join(", ")
    );
    let mut line = match (installed.outcome, &installed.previous_version) {
        (InstallOutcome::Reinstalled, _) => format!(
            "reinstalled {} {} ({folders}) in {folder_name}",
            installed.name, installed.version
        ),
        (InstallOutcome::Updated, Some(previous)) => format!(
            "updated {} from {previous} to {} ({folders}) in {folder_name}",
            installed.name, installed.version
        ),
        _ => format!(
            "installed {} {} ({folders}) into {folder_name}",
            installed.name, installed.version
        ),
    };
    if !installed.removed_folders.is_empty() {
        line.push_str(&format!(
            ", {} of the previous version removed ({})",
            count_folders(installed.removed_folders.len()),
            installed.removed_folders.join(", ")
        ));
    }
    InstallSummary {
        addon,
        mismatch,
        out_of_date,
        line,
    }
}

/// The summary of an addon the one install path found installed already at the pick's version: the
/// sidecar's entry as the `addon` block, nothing removed, and the nothing-to-do line.
fn already_installed_summary(current: &AlreadyInstalled, folder_name: &str) -> InstallSummary {
    InstallSummary {
        addon: serde_json::json!({
            "name": current.name,
            "version": current.entry.version,
            // Nothing was installed (task 083: the outcome of an install, `null` here).
            "outcome": null,
            "previous_version": current.entry.previous_version,
            "source": current.entry.source,
            "folders": current.entry.folders,
            "removed_folders": [],
        }),
        mismatch: None,
        out_of_date: None,
        line: current.nothing_to_do_line(folder_name),
    }
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
            let folder = absolute(&folder)?;
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
                        "no game root: none remembered, none in the registry or the usual install folders; run `mrgl root <folder>`",
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

/// The engine with the `--max-unpacked` bound when one was given (task 039), else as it is.
fn with_bound(engine: Engine, max_unpacked: Option<u64>) -> Engine {
    match max_unpacked {
        Some(bytes) => engine.with_max_unpacked(bytes),
        None => engine,
    }
}

/// The refusal of `--as` without a zip in hand (task 096).
const AS_NEEDS_A_ZIP: &str =
    "--as goes with a zip in hand: mrgl add <file.zip> --as <spec> records the spec as its source";

/// The spec `add <file.zip> --as <spec>` records (task 096), in its canonical spelling; a Wago page address
/// is refused, since only a request turns it into its id.
fn spec_of_zip(as_spec: &str) -> Result<String, String> {
    match SourceSpec::parse(as_spec).map_err(|error| error.to_string())? {
        SourceSpec::WagoPage { .. } => Err(format!(
            "--as takes a spec, not a page address: {as_spec} (use wago:<id>, the X-Wago-ID line of the addon's TOC)"
        )),
        spec => Ok(spec.to_string()),
    }
}

/// `mrgl add <file.zip> --flavor <id> [--as <spec>]`: the fourth source of D-002, a zip in hand, through the install
/// engine, recorded with its path, or with the spec `--as` names (task 096: a CurseForge project whose author
/// allows no download outside CurseForge installs from the zip downloaded from its page, and the update check
/// asks its source; the version is the TOC's); `--dry-run` stages the zip, says what it would install and
/// writes nothing. A client running from the flavor folder is a warning line and `client_warning` in the JSON
/// (task 038), never a refusal.
fn add_zip(
    reporter: &Reporter,
    source: &str,
    flavor: Option<&str>,
    dry_run: bool,
    max_unpacked: Option<u64>,
    as_spec: Option<&str>,
) -> Result<(), String> {
    let recorded = as_spec.map(spec_of_zip).transpose()?;
    let mut warnings = Vec::new();
    let engine = with_bound(engine_for(flavor, &mut warnings)?, max_unpacked);
    warnings.extend(adopt::curse_client_warning(&engine.flavor));
    let zip = absolute(Path::new(source))?;
    let zip_name = file_name_of(&zip);
    let metadata = Metadata {
        source: recorded.unwrap_or_else(|| zip.display().to_string()),
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
        warnings.extend(staged.sweep_warnings.iter().cloned());
        engine
            .refuse_links(&loaded.state, &staged.folders())
            .map_err(|error| error.to_string())?;
        // What the install would record (task 083): a reinstall of the recorded version keeps the entry's
        // previous version, as the commit does.
        let replaced = loaded.state.entry(&staged.name).map(|(_, entry)| entry);
        let outcome = InstallOutcome::of(replaced, &staged.version);
        let previous = InstallOutcome::previous_version(replaced, &staged.version);
        let folders = staged.folders();
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "add",
                "flavor": &engine.flavor,
                "dry_run": true,
                "addon": {
                    "name": staged.name,
                    "version": staged.version,
                    "outcome": outcome,
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
            let what = match (outcome, &previous) {
                (InstallOutcome::Reinstalled, _) => {
                    format!("reinstall {} {}", staged.name, staged.version)
                }
                (InstallOutcome::Updated, Some(previous)) => format!(
                    "update {} from {previous} to {}",
                    staged.name, staged.version
                ),
                _ => format!("install {} {}", staged.name, staged.version),
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
    let summary = install_summary(&engine, &installed, &metadata.source, &mut warnings);
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "add",
            "flavor": &engine.flavor,
            "dry_run": false,
            "addon": summary.addon,
            "client_warning": installed.client_warning,
            "interface_mismatch": summary.mismatch,
            "interface_out_of_date": summary.out_of_date,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in installed.client_warning.iter().chain(&warnings) {
        reporter.say(Kind::Warning, warning);
    }
    reporter.say(Kind::Success, &summary.line);
    summary.say_interface(reporter, &folder_name);
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
/// development folder, or for the one addon a folder with a TOC named after it is (task 062), recorded as
/// `link:<folder>`; a name in the way is a warning, never replaced.
fn link_command(reporter: &Reporter, folder: &Path, flavor: Option<&str>) -> Result<(), String> {
    link::ensure_supported().map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let folder_name = engine.flavor.folder_name();
    if !reporter.json {
        let what = match link::shape_of(folder) {
            link::Shape::AddonFolder => "the addon folder",
            link::Shape::DevelopmentFolder => "the addons of",
        };
        reporter.say(
            Kind::Progress,
            format!("linking {what} {} into {folder_name}", folder.display()),
        );
    }
    let linked = link::link(&engine, folder).map_err(|error| error.to_string())?;
    warnings.extend(linked.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "link",
            "flavor": &engine.flavor,
            "folder": linked.folder,
            "shape": linked.shape,
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
    if linked.shape == link::Shape::AddonFolder
        && let [addon] = linked.linked.as_slice()
    {
        let (kind, summary) = if addon.already_linked {
            (
                Kind::Nothing,
                format!(
                    "{} is linked from {} into {folder_name} already",
                    addon.name,
                    linked.folder.display()
                ),
            )
        } else {
            (
                Kind::Success,
                format!(
                    "linked the addon {} from {} into {folder_name}",
                    addon.name,
                    linked.folder.display()
                ),
            )
        };
        reporter.say(kind, summary);
        return Ok(());
    }
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

/// `mrgl pin <name> [version]`, `unpin`, `ignore`, `unignore` and `channel <name> <channel>`, each with
/// `--flavor`: one mark on a managed addon's sidecar entry. A mark that is set already writes nothing and
/// says so; a name the sidecar does not know is an error. Nothing in the game folder changes.
fn mark_command(
    reporter: &Reporter,
    name: &str,
    flavor: Option<&str>,
    mark: Mark,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let kind = mark.as_str();
    let marked = engine
        .apply_mark(name, &mark)
        .map_err(|error| error.to_string())?;
    warnings.extend(marked.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": kind,
            "flavor": &engine.flavor,
            "addon": {
                "name": marked.name,
                "version": marked.version,
                "pinned": marked.pinned,
                "ignored": marked.ignored,
                "channel": marked.channel,
            },
            "changed": marked.changed,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, plain) = mark.line(&marked, &engine.flavor.folder_name());
    reporter.say(kind, plain);
    Ok(())
}

/// `mrgl rollback <name> --flavor <id>` (task 022): the previous version's zip from the cache through the
/// install engine, then a pin at that version. The JSON document carries the addon as it is now: the
/// version rolled back to, the version rolled back from as the previous one, and the pin.
fn rollback_command(
    reporter: &Reporter,
    name: &str,
    flavor: Option<&str>,
    max_unpacked: Option<u64>,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = with_bound(engine_for(flavor, &mut warnings)?, max_unpacked);
    let folder_name = engine.flavor.folder_name();
    if !reporter.json {
        reporter.say(
            Kind::Progress,
            format!("rolling back {name} in {folder_name} from the kept zip"),
        );
    }
    let rolled = engine.rollback(name).map_err(|error| error.to_string())?;
    warnings.extend(rolled.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "rollback",
            "flavor": &engine.flavor,
            "addon": {
                "name": rolled.name,
                "version": rolled.version,
                "previous_version": rolled.previous_version,
                "pinned": rolled.pinned,
                "source": rolled.source,
                "folders": rolled.folders,
                "removed_folders": rolled.removed_folders,
            },
            "warnings": warnings,
        }));
        return Ok(());
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let mut summary = format!(
        "rolled back {} from {} to {} ({}: {}) in {folder_name}",
        rolled.name,
        rolled.previous_version,
        rolled.version,
        count_folders(rolled.folders.len()),
        rolled.folders.join(", ")
    );
    if !rolled.removed_folders.is_empty() {
        summary.push_str(&format!(
            ", {} of {} removed ({})",
            count_folders(rolled.removed_folders.len()),
            rolled.previous_version,
            rolled.removed_folders.join(", ")
        ));
    }
    summary.push_str(&format!(
        "; pinned at {}, `mrgl unpin {}` lets updates reach it again",
        rolled.pinned, rolled.name
    ));
    reporter.say(Kind::Success, summary);
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
    max_unpacked: Option<u64>,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let folder = flavor_folder(spec, None, &mut warnings)?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| error.to_string())?;
    warnings.extend(flavor.warnings.iter().cloned());
    let backups = match to {
        Some(to) => absolute(&to)?,
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
                zip_entry::size_text(entry.bytes),
                entry.modified.as_deref().unwrap_or("-")
            ));
        }
        for aside in &listing.aside {
            reporter.line(format!(
                "{}  the WTF folder a restore moved aside",
                file_name_of(aside)
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
        let zip_name = file_name_of(&zip);
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
            if !confirmed(&mut std::io::stdin().lock()) {
                reporter.say(Kind::Nothing, "restore cancelled, nothing changed");
                return Ok(());
            }
        }
        let bound = max_unpacked.unwrap_or(zip_entry::MAX_UNPACKED);
        let restored = backup::restore_wtf_bounded(&flavor, &zip, &backups, bound)
            .map_err(|error| error.to_string())?;
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
            zip_entry::size_text(backup.bytes),
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

/// Reads one line of the input (the standard input): `y` or `yes`, in any case, is a confirmation;
/// anything else, an empty line, a closed input or one that cannot be read is not.
fn confirmed(input: &mut dyn BufRead) -> bool {
    let mut answer = String::new();
    if input.read_line(&mut answer).is_err() {
        return false;
    }
    let answer = answer.trim();
    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
}

/// A path as typed, made absolute against the current folder, for a line and for a file that stays put
/// whatever folder a later run starts in; the error is the plain line.
fn absolute(path: &Path) -> Result<PathBuf, String> {
    match std::path::absolute(path) {
        Ok(path) => Ok(path),
        // coverage: os-begin — the system's own refusal of a path: the empty path, the one case std names,
        // never reaches here, since clap refuses an empty value first
        Err(error) => Err(format!("resolving {}: {error}", path.display())),
        // coverage: os-end
    }
}

/// A path's file name for a line, the whole path when it has none (a root, a path ending in `..`).
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// `mrgl export --flavor <id> [--to <file>]`: the flavor's managed addons as a list file (task 015).
/// Without `--to` the list itself goes to the standard output, so it can be redirected, and the messages
/// go to the standard error.
fn export_command(
    reporter: &Reporter,
    flavor: Option<&str>,
    to: Option<PathBuf>,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    let export = list_file::export(&engine).map_err(|error| error.to_string())?;
    warnings.extend(export.warnings.iter().cloned());
    let list = &export.list;
    let file = match to {
        Some(to) => {
            let to = absolute(&to)?;
            list.write(&to).map_err(|error| error.to_string())?;
            Some(to)
        }
        None => None,
    };
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "export",
            "flavor": &engine.flavor,
            "file": file,
            "list": list,
            "warnings": warnings,
        }));
        return Ok(());
    }
    let folder_name = engine.flavor.folder_name();
    let count = match list.addons.len() {
        0 => "no managed addons".to_string(),
        1 => "1 managed addon".to_string(),
        count => format!("{count} managed addons"),
    };
    let kind = if list.addons.is_empty() {
        Kind::Nothing
    } else {
        Kind::Success
    };
    let Some(file) = file else {
        emit(Stream::Stdout, format_args!("{}", list.to_json()));
        for warning in &warnings {
            reporter.say_on_stderr(Kind::Warning, warning);
        }
        reporter.say_on_stderr(kind, format!("exported {count} of {folder_name}"));
        return Ok(());
    };
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    reporter.say(
        kind,
        format!("exported {count} of {folder_name} to {}", file.display()),
    );
    Ok(())
}

/// `mrgl import <file> --flavor <id> [--dry-run]`: every `github:` entry of a list file that the flavor
/// does not have is resolved and installed (task 015); a zip in hand is not importable, a linked folder
/// is skipped, an addon the flavor has is left alone. `--exact` installs the version the list names
/// (task 057): an entry whose version the source no longer offers is reported, skipped and counted.
/// `--dry-run` resolves and reports, and writes nothing. An entry that fails does not stop the others;
/// the command then ends with exit code 1.
fn import_command(
    reporter: &Reporter,
    file: &Path,
    flavor: Option<&str>,
    exact: bool,
    dry_run: bool,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    warnings.extend(adopt::curse_client_warning(&engine.flavor));
    let path = absolute(file)?;
    let list = ListFile::read(&path).map_err(|error| error.to_string())?;
    let file_name = file_name_of(&path);
    let folder_name = engine.flavor.folder_name();
    let count = match list.addons.len() {
        1 => "1 addon".to_string(),
        count => format!("{count} addons"),
    };
    if !reporter.json {
        let verb = if dry_run {
            "dry run: reading"
        } else {
            "importing"
        };
        let versions = if exact {
            " at the versions it names"
        } else {
            ""
        };
        reporter.say(
            Kind::Progress,
            format!("{verb} {count} from {file_name} into {folder_name}{versions}"),
        );
    }
    let sources = Sources::from_env();
    let report = list_file::import(&engine, &sources, &list, exact, dry_run)
        .map_err(|error| error.to_string())?;
    warnings.extend(report.warnings.iter().cloned());
    for item in &report.items {
        warnings.extend(item.warnings.iter().cloned());
    }
    let failed: Vec<&str> = report
        .items
        .iter()
        .filter(|item| item.outcome == Outcome::Failed)
        .map(|item| item.name.as_str())
        .collect();

    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "import",
            "flavor": &engine.flavor,
            "file": &path,
            "list_flavor": &list.flavor,
            "exact": exact,
            "dry_run": dry_run,
            "addons": &report.items,
            "warnings": warnings,
        }));
    } else {
        for item in &report.items {
            reporter.line(import_line(item));
        }
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        for item in &report.items {
            if let Some(mismatch) = &item.interface_mismatch {
                reporter.say(Kind::Warning, mismatch.after_install_line(&folder_name));
            }
            if let Some(out_of_date) = &item.interface_out_of_date {
                reporter.say(Kind::Warning, out_of_date.after_install_line(&folder_name));
            }
        }
        let counts: Vec<String> = [
            (Outcome::Installed, "installed"),
            (Outcome::WouldInstall, "to install"),
            (Outcome::AlreadyInstalled, "already installed"),
            (Outcome::NotImportable, "not importable"),
            (Outcome::SourceNotConfigured, "source not configured"),
            (Outcome::Skipped, "skipped"),
            (Outcome::Failed, "failed"),
            (Outcome::VersionNotOffered, "version not offered"),
        ]
        .into_iter()
        .filter_map(|(outcome, label)| match report.count(outcome) {
            0 => None,
            count => Some(format!("{count} {label}")),
        })
        .collect();
        let kind = if !failed.is_empty()
            || report.count(Outcome::VersionNotOffered) > 0
            || report.count(Outcome::SourceNotConfigured) > 0
        {
            Kind::Warning
        } else if report.count(Outcome::Installed) + report.count(Outcome::WouldInstall) > 0 {
            Kind::Success
        } else {
            Kind::Nothing
        };
        let summary = if list.addons.is_empty() {
            format!("no addons in {file_name}, nothing to import")
        } else if dry_run {
            format!(
                "dry run: {count} of {file_name} for {folder_name}: {}; nothing written",
                counts.join(", ")
            )
        } else {
            format!(
                "imported {count} of {file_name} into {folder_name}: {}",
                counts.join(", ")
            )
        };
        reporter.say(kind, summary);
    }
    if failed.is_empty() {
        return Ok(());
    }
    let count = match failed.len() {
        1 => "1 addon".to_string(),
        count => format!("{count} addons"),
    };
    Err(format!(
        "{count} of {file_name} failed: {}; the other entries are done",
        failed.join(", ")
    ))
}

/// One entry of an import as a plain data line: the name, then what happened and why.
fn import_line(item: &list_file::Imported) -> String {
    let listed = match &item.version {
        Some(version) if *version != item.listed_version => {
            format!(", the list had {}", item.listed_version)
        }
        _ => String::new(),
    };
    let version = item.version.as_deref().unwrap_or("-");
    let what = match item.outcome {
        Outcome::Installed => format!("installed {version} from {}{listed}", item.source),
        Outcome::WouldInstall => format!("would install {version} from {}{listed}", item.source),
        Outcome::AlreadyInstalled => format!("already installed, left alone: {}", item.detail),
        Outcome::NotImportable => format!("not importable ({}): {}", item.source, item.detail),
        Outcome::SourceNotConfigured => {
            format!("source not configured ({}): {}", item.source, item.detail)
        }
        Outcome::Skipped => format!("skipped ({}): {}", item.source, item.detail),
        Outcome::Failed => format!("failed: {}", item.detail),
        Outcome::VersionNotOffered => format!(
            "skipped, version {} not offered: {}",
            item.listed_version, item.detail
        ),
    };
    format!("{}  {what}", item.name)
}

/// What `update` did with one addon the check found an update for.
enum Applied {
    Updated(Box<InstallSummary>),
    /// Task 089: the one install path found the pick's version installed already, under the addon's own
    /// entry or under another entry of the same source, and installed nothing.
    InstalledAlready(Box<InstalledAlready>),
    /// Task 108: an install earlier in the run took every folder of the addon's entry over and dropped the
    /// entry, so nothing is left to update: the addon is folded into that install's addon.
    Folded(Box<Folded>),
    Failed(String),
}

/// An update the one install path answered with "installed already" (task 089): the entry it found and
/// the line `update` says about it.
struct InstalledAlready {
    /// The entry as the sidecar has it: the `addon` block of the nothing-to-do summary.
    summary: InstallSummary,
    /// Whether the entry found is the updated addon's own.
    same_entry: bool,
    kind: Kind,
    line: String,
}

impl InstalledAlready {
    /// The answer for the addon `name` at `version` (task 089). Its own entry at the pick's version (another
    /// run updated it since the check) is the nothing-to-do line; another entry of the same source (a renamed
    /// addon next to its old name) is a warning that names that entry and says the addon stays as it is;
    /// the words are the core's (`AlreadyInstalled::update_line`), which the window says too (task 099).
    fn of(name: &str, version: &str, current: &AlreadyInstalled, folder_name: &str) -> Self {
        let (kind, line) = current.update_line(name, version, folder_name);
        InstalledAlready {
            summary: already_installed_summary(current, folder_name),
            same_entry: current.is_entry_of(name),
            kind,
            line,
        }
    }

    /// The `installed_already` object of the addon in `update --json`.
    fn json(&self) -> serde_json::Value {
        let mut value = self.summary.addon.clone();
        value["same_entry"] = self.same_entry.into();
        value["message"] = self.line.clone().into();
        value
    }
}

/// `mrgl update --all | <name> --flavor <id> [--dry-run] [--yes]` (task 006): the update check of the core
/// (`update::check`) over every managed addon or the one named, a data line per addon, then one
/// confirmation for the updates it found (`--yes` answers it; without a terminal or with `--json` it is
/// needed), then each update through the one install path of the engine (task 055) with the pick the
/// check made, and the summary that counts every state (`2 addons updated, 12 up to date, 1 pinned, 1
/// ignored`). `--dry-run` checks and writes nothing. An update or a check that fails does not stop the
/// others; the command then ends with exit code 1. Task 068: a progress line before each addon the source
/// is asked about and its data line as soon as it is checked; a stop before a source's rate limit is one
/// warning line with the reset time, and the addons it leaves are counted per source (task 101: `2 not
/// checked (GitHub's rate limit)`, the other sources still asked), a warning, not a failure.
/// Task 108: an addon whose entry an install earlier in the run dropped (that
/// install's zip carried every folder of it) is not installed again: `update` says it is folded into
/// that addon.
fn update_command(
    reporter: &Reporter,
    name: Option<&str>,
    flavor: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<(), String> {
    let mut warnings = Vec::new();
    let engine = engine_for(flavor, &mut warnings)?;
    warnings.extend(adopt::curse_client_warning(&engine.flavor));
    let folder_name = engine.flavor.folder_name();
    if !reporter.json {
        let what = name.map_or("the managed addons".to_string(), |name| name.to_string());
        reporter.say(
            Kind::Progress,
            format!("checking {what} of {folder_name} against their sources"),
        );
    }
    let sources = Sources::from_env();
    // Task 068: a progress line before each addon the source is asked about, its data line once it is
    // checked, and the one line of a stop before the rate limit, as they happen.
    let check = update::check(&engine, &sources, name, &mut |step| {
        if reporter.json {
            return;
        }
        match step {
            CheckProgress::Checking {
                index,
                total,
                name,
                source,
            } => reporter.say(
                Kind::Progress,
                format!("checking {name} against {source} ({index} of {total})"),
            ),
            CheckProgress::Stopped { stop } => reporter.say(Kind::Warning, stop.line()),
            CheckProgress::Checked { addon, .. } => reporter.line(addon.line()),
        }
    })
    .map_err(|error| error.to_string())?;
    warnings.extend(check.warnings.iter().cloned());
    // Task 087: what the check found that the cache cannot tell again (a source gone, a failed check, the
    // stop before the rate limit) is kept for the window, as its own check keeps it (task 078), and an
    // addon the check answered otherwise drops its kept answer; a dry run is a check too.
    let state = engine
        .load_state()
        .map(|loaded| loaded.state)
        .unwrap_or_else(|_| State::new(&engine.flavor.id)); // coverage: os — the check read it a moment ago
    if let Err(error) =
        last_check::record_check(&engine.home, &engine.flavor.id, &state, &check.addons)
    {
        warnings.push(format!(
            "the answers of this check are not kept for the window: {error}"
        ));
    }
    // Task 041: a pick from the cached release list, offline, is a warning here as on `add`.
    warnings.extend(
        check
            .addons
            .iter()
            .filter_map(|addon| addon.pick.as_ref()?.stale_warning()),
    );
    let updates: Vec<&update::AddonUpdate> = check.updates().collect();
    let available = updates.len();
    // The updates this run installs: none on a dry run, else every one after `--yes` or the confirmation.
    let to_apply = if dry_run {
        Vec::new()
    } else {
        let stdin = std::io::stdin();
        confirmed_updates(
            reporter,
            updates,
            yes,
            &folder_name,
            &mut stdin.lock(),
            stdin.is_terminal(),
        )?
    };

    let mut applied: BTreeMap<String, Applied> = BTreeMap::new();
    // Task 108: the entries the installs of this run dropped, folded into the installed addon.
    let mut folded: Vec<Folded> = Vec::new();
    // Every update carries its request: the check made the pick, and only a spec source is checked.
    for (addon, request) in to_apply
        .iter()
        .filter_map(|addon| Some((addon, addon.request()?)))
    {
        if let Some(into) = folded
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(&addon.name))
        {
            if !reporter.json {
                let (kind, line) = into.update_line(&folder_name);
                reporter.say(kind, line);
            }
            applied.insert(addon.name.clone(), Applied::Folded(Box::new(into.clone())));
            continue;
        }
        if !reporter.json {
            reporter.say(
                Kind::Progress,
                format!(
                    "updating {} from {} to {}",
                    addon.name,
                    addon.version,
                    addon.latest.as_deref().unwrap_or_default()
                ),
            );
        }
        let outcome = engine.install_from_source(
            &sources,
            &request,
            &mut install_progress(reporter, &engine, &mut warnings),
        );
        let result = match outcome {
            Ok(SourceInstalled {
                pick,
                outcome: PickInstalled::Installed(installed),
            }) => {
                if let Some(line) = &installed.client_warning
                    && !warnings.contains(line)
                {
                    warnings.push(line.clone());
                }
                let summary = install_summary(&engine, &installed, &pick.source, &mut warnings);
                if !reporter.json {
                    reporter.say(Kind::Success, &summary.line);
                    // Task 082: the interface check of the updated addon, as `add` says it.
                    summary.say_interface(reporter, &folder_name);
                }
                folded.extend(installed.folded.iter().cloned());
                Applied::Updated(Box::new(summary))
            }
            Ok(SourceInstalled {
                outcome: PickInstalled::AlreadyInstalled(current),
                ..
            }) => {
                warnings.extend(current.warnings.iter().cloned());
                let already =
                    InstalledAlready::of(&addon.name, &addon.version, &current, &folder_name);
                if !reporter.json {
                    reporter.say(already.kind, &already.line);
                }
                Applied::InstalledAlready(Box::new(already))
            }
            Err(error) => {
                let line = format!("{}: {error}", addon.name);
                if !reporter.json {
                    reporter.say(Kind::Error, &line);
                }
                Applied::Failed(line)
            }
        };
        applied.insert(addon.name.clone(), result);
    }

    let updated = applied
        .values()
        .filter(|result| matches!(result, Applied::Updated(_)))
        .count();
    let installed_already = applied
        .values()
        .filter(|result| matches!(result, Applied::InstalledAlready(_)))
        .count();
    let folded_count = applied
        .values()
        .filter(|result| matches!(result, Applied::Folded(_)))
        .count();
    let mut failed: Vec<&str> = applied
        .iter()
        .filter(|(_, result)| matches!(result, Applied::Failed(_)))
        .map(|(name, _)| name.as_str())
        .collect();
    failed.extend(
        check
            .addons
            .iter()
            .filter(|addon| addon.state == UpdateState::Failed)
            .map(|addon| addon.name.as_str()),
    );

    if reporter.json {
        let addons: Vec<serde_json::Value> = check
            .addons
            .iter()
            .map(|addon| {
                let mut value = serde_json::to_value(addon).unwrap_or_default();
                let summary = match applied.get(&addon.name) {
                    Some(Applied::Updated(summary)) => Some(summary),
                    _ => None,
                };
                let error = match applied.get(&addon.name) {
                    Some(Applied::Failed(line)) => Some(line.clone()),
                    _ => None,
                };
                // Task 089: the entry the one install path found at the pick's version, `null` otherwise.
                let already = match applied.get(&addon.name) {
                    Some(Applied::InstalledAlready(already)) => Some(already.json()),
                    _ => None,
                };
                // Task 108: the addon an earlier install of the run folded this entry into, `null`
                // otherwise.
                let folded_into = match applied.get(&addon.name) {
                    Some(Applied::Folded(into)) => Some(serde_json::json!({
                        "name": into.into,
                        "version": into.into_version,
                        "folders": into.folders,
                        "message": into.update_line(&folder_name).1,
                    })),
                    _ => None,
                };
                value["updated"] = summary.is_some().into();
                value["addon"] = summary.map(|summary| summary.addon.clone()).into();
                value["installed_already"] = already.into();
                value["folded_into"] = folded_into.into();
                // Task 082: the interface check of the updated addon, `null` when it passes or was not
                // updated.
                value["interface_mismatch"] =
                    serde_json::to_value(summary.and_then(|summary| summary.mismatch.as_ref()))
                        .unwrap_or_default();
                value["interface_out_of_date"] =
                    serde_json::to_value(summary.and_then(|summary| summary.out_of_date.as_ref()))
                        .unwrap_or_default();
                value["error"] = error.into();
                value
            })
            .collect();
        let counts: serde_json::Map<String, serde_json::Value> = UpdateState::ALL
            .iter()
            .map(|state| {
                let key = serde_json::to_value(state)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_default();
                (key, check.count(*state).into())
            })
            .collect();
        // Tasks 068, 101: the stops before a source's rate limit, one per source with its line; empty when
        // the check asked about every addon.
        let stopped: Vec<serde_json::Value> = check
            .stopped
            .iter()
            .map(|stop| {
                let mut value = serde_json::to_value(stop).unwrap_or_default();
                value["message"] = stop.line().into();
                value
            })
            .collect();
        reporter.document(serde_json::json!({
            "kind": "update",
            "flavor": &engine.flavor,
            "dry_run": dry_run,
            "addons": addons,
            "updated": updated,
            "installed_already": installed_already,
            "folded": folded_count,
            "counts": counts,
            "stopped": stopped,
            "warnings": warnings,
        }));
    } else {
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        let of = if dry_run {
            SummaryOf::DryRun
        } else {
            SummaryOf::Update {
                updated,
                left: available - to_apply.len(),
                installed_already,
                folded: folded_count,
            }
        };
        let (kind, summary) = check.summary(of, &folder_name);
        reporter.say(kind, summary);
    }
    if failed.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} of {folder_name} failed: {}; the other addons are done",
        count_addons(failed.len()),
        failed.join(", ")
    ))
}

/// The updates a run installs, after the one confirmation of `update` (task 006): every one with `--yes`,
/// and nothing when there is nothing to update; else the question on the terminal, answered through
/// [`confirmed`], and a no is said as the cancel and leaves none (the summary counts them as cancelled).
/// Without a terminal, or with `--json`, nobody can answer: the error names `--yes`. The list comes back as
/// it came, so the caller installs exactly what was confirmed.
fn confirmed_updates<T>(
    reporter: &Reporter,
    updates: Vec<T>,
    yes: bool,
    folder_name: &str,
    input: &mut dyn BufRead,
    terminal: bool,
) -> Result<Vec<T>, String> {
    if yes || updates.is_empty() {
        return Ok(updates);
    }
    let count = count_addons(updates.len());
    if reporter.json {
        return Err(format!(
            "{count} to update, nothing updated: --json needs --yes, nobody is there to answer the confirmation"
        ));
    }
    if !terminal {
        return Err(format!(
            "{count} to update, nothing updated: no terminal to answer the confirmation; --yes updates without asking"
        ));
    }
    reporter.say(
        Kind::Warning,
        format!("update {count} in {folder_name}? type y to go on"),
    );
    if confirmed(input) {
        return Ok(updates);
    }
    reporter.say(Kind::Nothing, "update cancelled, nothing changed");
    Ok(Vec::new())
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

/// `mrgl savedvars --flavor <id> [--clean]` (task 025): the flavor's SavedVariables files, the largest
/// first, one data line each with the orphans marked, and a summary; `--clean` moves the orphans and their
/// `.bak` files into `SavedVariables-<date>.zip` in the backups folder under the mrgl home, refused while
/// the client runs (task 023).
fn savedvars_command(reporter: &Reporter, spec: Option<&str>, clean: bool) -> Result<(), String> {
    let mut warnings = Vec::new();
    let folder = flavor_folder(spec, None, &mut warnings)?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| error.to_string())?;
    warnings.extend(flavor.warnings.iter().cloned());
    let folder_name = flavor.folder_name();

    if clean {
        let backups = backup::backups_dir(&flavor.id).map_err(|error| error.to_string())?;
        let cleaned =
            savedvars::clean_orphans(&flavor, &backups).map_err(|error| error.to_string())?;
        warnings.extend(cleaned.warnings.iter().cloned());
        if reporter.json {
            reporter.document(serde_json::json!({
                "kind": "savedvars-clean",
                "flavor": &flavor,
                "zip": &cleaned.zip,
                "moved": &cleaned.moved,
                "files": cleaned.files,
                "bytes": cleaned.bytes,
                "warnings": warnings,
            }));
            return Ok(());
        }
        for file in &cleaned.moved {
            reporter.line(savedvars_line(file));
        }
        for warning in &warnings {
            reporter.say(Kind::Warning, warning);
        }
        let (kind, summary) = cleaned.summary(&folder_name);
        reporter.say(kind, summary);
        return Ok(());
    }

    let listing = savedvars::list_saved_variables(&flavor).map_err(|error| error.to_string())?;
    warnings.extend(listing.warnings.iter().cloned());
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "savedvars",
            "flavor": &flavor,
            "wtf": &listing.wtf,
            "files": &listing.files,
            "bytes": listing.bytes,
            "orphans": listing.orphans,
            "orphan_bytes": listing.orphan_bytes,
            "warnings": warnings,
        }));
        return Ok(());
    }
    for file in &listing.files {
        reporter.line(savedvars_line(file));
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, summary) = listing.summary(&folder_name, savedvars::CleanupBy::Command);
    reporter.say(kind, summary);
    Ok(())
}

/// One data line of `savedvars`: the size (the `.lua` and its `.bak` together), the addon, where it is,
/// and `orphan` or `client` (the client's own file) when it is one.
fn savedvars_line(file: &SavedVariablesFile) -> String {
    let mark = match file.status {
        SavedStatus::Orphan => "  orphan",
        SavedStatus::Client => "  client",
        SavedStatus::Installed | SavedStatus::Unknown => "",
    };
    format!(
        "{:>9}  {}  {}{mark}",
        zip_entry::size_text(file.bytes),
        file.addon,
        file.place()
    )
}

/// The verbs of `mrgl set` (task 014).
#[derive(Subcommand, Debug)]
enum SetAction {
    /// Save the addons the characters' AddOns.txt name, enabled or disabled, as a set; an addon the
    /// characters disagree on is left out, and a set of the same name is replaced.
    Save {
        /// The set's name.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Read one character's AddOns.txt only, by name (the character's folder under WTF).
        #[arg(long)]
        character: Option<String>,
        /// The realm of `--character`, for a name more than one realm has.
        #[arg(long, requires = "character")]
        realm: Option<String>,
        /// The account folder of `--character`, for a realm and name under more than one account.
        #[arg(long, requires = "character")]
        account: Option<String>,
    },
    /// Apply a set to every character, or to one with `--character`: exactly the set's addons get its
    /// state in AddOns.txt, every other line stays as it is; the files it replaces are kept for `set
    /// undo`. Refused while the client runs.
    Apply {
        /// The set's name.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
        /// Apply to one character only, by name; every other character's AddOns.txt stays as it is.
        #[arg(long)]
        character: Option<String>,
        /// The realm of `--character`, for a name more than one realm has.
        #[arg(long, requires = "character")]
        realm: Option<String>,
        /// The account folder of `--character`, for a realm and name under more than one account.
        #[arg(long, requires = "character")]
        account: Option<String>,
    },
    /// Put back the AddOns.txt files the last apply replaced, byte for byte, and remove the ones it
    /// created; once. Refused while the client runs.
    Undo {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Enable one addon in a stored set (added when the set does not have it); no AddOns.txt is touched.
    Enable {
        /// The set's name.
        name: String,
        /// The addon's folder name.
        addon: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Disable one addon in a stored set (added when the set does not have it); no AddOns.txt is touched.
    Disable {
        /// The set's name.
        name: String,
        /// The addon's folder name.
        addon: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Drop one addon from a stored set, so an apply leaves its lines alone; no AddOns.txt is touched.
    Drop {
        /// The set's name.
        name: String,
        /// The addon's folder name.
        addon: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// List the flavor's sets.
    List {
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Print a set's addons, one `Name: state` line each.
    Show {
        /// The set's name.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Remove a set; no AddOns.txt is touched.
    Remove {
        /// The set's name.
        name: String,
        /// The flavor id or folder.
        #[arg(long)]
        flavor: Option<String>,
    },
}

/// `mrgl set save|apply|undo|enable|disable|drop|list|show|remove … --flavor <id>` (tasks 014 and 073):
/// the sets live in `sets.json` under the flavor's folder of the mrgl home; an apply rewrites the
/// characters' `AddOns.txt` and keeps what it replaced for `undo`, and both are refused while the client
/// runs (task 023), before anything is written; an edit changes `sets.json` only.
fn set_command(reporter: &Reporter, action: SetAction) -> Result<(), String> {
    let spec = match &action {
        SetAction::Save { flavor, .. }
        | SetAction::Apply { flavor, .. }
        | SetAction::Undo { flavor }
        | SetAction::Enable { flavor, .. }
        | SetAction::Disable { flavor, .. }
        | SetAction::Drop { flavor, .. }
        | SetAction::List { flavor }
        | SetAction::Show { flavor, .. }
        | SetAction::Remove { flavor, .. } => flavor.clone(),
    };
    let mut warnings = Vec::new();
    let folder = flavor_folder(spec.as_deref(), None, &mut warnings)?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| error.to_string())?;
    warnings.extend(flavor.warnings.iter().cloned());
    let path = sets::sets_path(&flavor.id).map_err(|error| error.to_string())?;
    let folder_name = flavor.folder_name();
    let warn = |warnings: &[String]| {
        for warning in warnings {
            reporter.say(Kind::Warning, warning);
        }
    };
    match action {
        SetAction::Save {
            name,
            character,
            realm,
            account,
            ..
        } => {
            let target = set_target(character, realm);
            let saved =
                sets::save_set_in_account(&flavor, &path, &name, &target, account.as_deref())
                    .map_err(set_error)?;
            warnings.extend(saved.warnings.iter().cloned());
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "set-save",
                    "flavor": &flavor,
                    "path": &path,
                    "name": &saved.name,
                    "set": &saved.set,
                    "replaced": saved.replaced,
                    "characters": &saved.characters,
                    "conflicts": &saved.conflicts,
                    "not_installed": &saved.not_installed,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            if !saved.conflicts.is_empty() {
                warnings.push(format!(
                    "left out of set {}, the characters disagree on them: {}",
                    saved.name,
                    saved.conflicts.join(", ")
                ));
            }
            if !saved.not_installed.is_empty() {
                warnings.push(format!(
                    "set {} names addons that are not installed in {folder_name}, kept in the set all the same: {}",
                    saved.name,
                    saved.not_installed.join(", ")
                ));
            }
            warn(&warnings);
            let mut summary = format!(
                "saved set {} of {folder_name} from {}: {}",
                saved.name,
                characters_text(&target, &saved.characters),
                count_set(&saved.set)
            );
            if saved.replaced {
                summary.push_str("; it replaces the set saved before under that name");
            }
            reporter.say(Kind::Success, summary);
        }
        SetAction::Apply {
            name,
            character,
            realm,
            account,
            ..
        } => {
            let target = set_target(character, realm);
            if !reporter.json {
                let whom = match &target {
                    sets::Target::EveryCharacter => "every character".to_string(),
                    sets::Target::Character {
                        name,
                        realm: Some(realm),
                    } => format!("the character {name} on {realm}"),
                    sets::Target::Character { name, realm: None } => {
                        format!("the character {name}")
                    }
                };
                reporter.say(
                    Kind::Progress,
                    format!("applying set {name} to {whom} of {folder_name}"),
                );
            }
            let applied =
                sets::apply_set_in_account(&flavor, &path, &name, &target, account.as_deref())
                    .map_err(set_error)?;
            report_applied(reporter, &flavor, &target, &applied, warnings);
        }
        SetAction::Undo { .. } => {
            let undone = sets::undo_apply(&flavor, &path).map_err(|error| error.to_string())?;
            warnings.extend(undone.warnings.iter().cloned());
            report_undone(reporter, &flavor, &undone, warnings);
        }
        SetAction::Enable { name, addon, .. } => {
            let edited = sets::edit_set(&flavor, &path, &name, &addon, sets::SetEdit::Enable)
                .map_err(|error| error.to_string())?;
            report_edited(reporter, &flavor, &edited, warnings);
        }
        SetAction::Disable { name, addon, .. } => {
            let edited = sets::edit_set(&flavor, &path, &name, &addon, sets::SetEdit::Disable)
                .map_err(|error| error.to_string())?;
            report_edited(reporter, &flavor, &edited, warnings);
        }
        SetAction::Drop { name, addon, .. } => {
            let edited = sets::edit_set(&flavor, &path, &name, &addon, sets::SetEdit::Drop)
                .map_err(|error| error.to_string())?;
            report_edited(reporter, &flavor, &edited, warnings);
        }
        SetAction::List { .. } => {
            let file = sets::read_sets_at(&path, &flavor.id).map_err(|error| error.to_string())?;
            if reporter.json {
                let listed: Vec<serde_json::Value> = file
                    .sets
                    .iter()
                    .map(|(name, set)| {
                        serde_json::json!({
                            "name": name,
                            "saved_at": &set.saved_at,
                            "enabled": set.enabled(),
                            "disabled": set.disabled(),
                            "addons": &set.addons,
                        })
                    })
                    .collect();
                reporter.document(serde_json::json!({
                    "kind": "set-list",
                    "flavor": &flavor,
                    "path": &path,
                    "sets": listed,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            for (name, set) in &file.sets {
                reporter.line(format!(
                    "{name}  {}  saved {}",
                    count_set(set),
                    set.saved_at
                ));
            }
            warn(&warnings);
            let (kind, note) = file.summary(&path, &folder_name);
            reporter.say(kind, note);
        }
        SetAction::Show { name, .. } => {
            let (name, set) =
                sets::find_set(&flavor.id, &path, &name).map_err(|error| error.to_string())?;
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "set-show",
                    "flavor": &flavor,
                    "name": name,
                    "set": &set,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            for (addon, state) in &set.addons {
                reporter.line(format!("{addon}: {state}"));
            }
            warn(&warnings);
            reporter.say(
                Kind::Success,
                format!(
                    "set {name} of {folder_name}: {}, saved {}",
                    count_set(&set),
                    set.saved_at
                ),
            );
        }
        SetAction::Remove { name, .. } => {
            let (name, set) =
                sets::remove_set(&flavor.id, &path, &name).map_err(|error| error.to_string())?;
            if reporter.json {
                reporter.document(serde_json::json!({
                    "kind": "set-remove",
                    "flavor": &flavor,
                    "name": name,
                    "set": &set,
                    "warnings": warnings,
                }));
                return Ok(());
            }
            warn(&warnings);
            reporter.say(
                Kind::Success,
                format!(
                    "removed set {name} of {folder_name} ({}); no AddOns.txt was touched",
                    count_set(&set)
                ),
            );
        }
    }
    Ok(())
}

/// The outcome of `set apply`: one data line per character, a warning for the addons that are not
/// installed, and the summary; or the one JSON document.
fn report_applied(
    reporter: &Reporter,
    flavor: &Flavor,
    target: &sets::Target,
    applied: &sets::Applied,
    mut warnings: Vec<String>,
) {
    let folder_name = flavor.folder_name();
    if reporter.json {
        let characters: Vec<serde_json::Value> = applied
            .characters
            .iter()
            .map(|change| {
                let changed: Vec<serde_json::Value> = change
                    .changed
                    .iter()
                    .map(|(name, state)| serde_json::json!({ "name": name, "state": state }))
                    .collect();
                serde_json::json!({
                    "account": &change.character.account,
                    "realm": &change.character.realm,
                    "name": &change.character.name,
                    "addons_txt": change.character.addons_txt(),
                    "created": change.created,
                    "changed": changed,
                })
            })
            .collect();
        reporter.document(serde_json::json!({
            "kind": "set-apply",
            "flavor": flavor,
            "name": &applied.name,
            "characters": characters,
            "changed": applied.changed(),
            "not_installed": &applied.not_installed,
            "warnings": warnings,
        }));
        return;
    }
    for change in &applied.characters {
        let what = if change.changed.is_empty() {
            "unchanged".to_string()
        } else {
            let lines: Vec<String> = change
                .changed
                .iter()
                .map(|(name, state)| format!("{name} {state}"))
                .collect();
            let created = if change.created {
                " (a new AddOns.txt)"
            } else {
                ""
            };
            format!("{}{created}", lines.join(", "))
        };
        reporter.line(format!("{}: {what}", change.character.label()));
    }
    warnings.extend(applied.not_installed_warning(&folder_name));
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, summary) = applied.summary(target, &folder_name);
    reporter.say(kind, summary);
}

/// The outcome of `set undo` (task 073): one data line per character, the warnings and the summary; or
/// the one JSON document.
fn report_undone(
    reporter: &Reporter,
    flavor: &Flavor,
    undone: &sets::Undone,
    warnings: Vec<String>,
) {
    let folder_name = flavor.folder_name();
    if reporter.json {
        let listed = |characters: &[sets::Character]| -> Vec<serde_json::Value> {
            characters
                .iter()
                .map(|character| {
                    serde_json::json!({
                        "account": &character.account,
                        "realm": &character.realm,
                        "name": &character.name,
                        "addons_txt": character.addons_txt(),
                    })
                })
                .collect()
        };
        reporter.document(serde_json::json!({
            "kind": "set-undo",
            "flavor": flavor,
            "name": &undone.set,
            "applied_at": &undone.applied_at,
            "restored": listed(&undone.restored),
            "removed": listed(&undone.removed),
            "warnings": warnings,
        }));
        return;
    }
    for character in &undone.restored {
        reporter.line(format!(
            "{}: AddOns.txt put back as it was",
            character.label()
        ));
    }
    for character in &undone.removed {
        reporter.line(format!(
            "{}: AddOns.txt removed, the apply had created it",
            character.label()
        ));
    }
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, summary) = undone.summary(&folder_name);
    reporter.say(kind, summary);
}

/// The outcome of `set enable|disable|drop` (task 073): a warning for an addon that is not installed and
/// the summary; or the one JSON document.
fn report_edited(
    reporter: &Reporter,
    flavor: &Flavor,
    edited: &sets::Edited,
    mut warnings: Vec<String>,
) {
    let folder_name = flavor.folder_name();
    if reporter.json {
        reporter.document(serde_json::json!({
            "kind": "set-edit",
            "flavor": flavor,
            "name": &edited.name,
            "addon": &edited.addon,
            "action": edited.edit,
            "before": edited.before,
            "after": edited.after,
            "changed": edited.changed,
            "installed": edited.installed,
            "set": &edited.set,
            "warnings": warnings,
        }));
        return;
    }
    warnings.extend(edited.not_installed(&folder_name));
    for warning in &warnings {
        reporter.say(Kind::Warning, warning);
    }
    let (kind, summary) = edited.summary(&folder_name);
    reporter.say(kind, summary);
}

/// The error of `set save` and `set apply` as the command line says it: the core's words, and for one
/// realm and name under two accounts the flag that picks the account after them (task 091; the core's
/// words name no flag, so the window shows them as they are).
fn set_error(error: Error) -> String {
    match &error {
        Error::AmbiguousCharacter {
            by: sets::CharacterPick::Account,
            ..
        } => format!("{error}, with --account"),
        _ => error.to_string(),
    }
}

/// The target of `--character` and `--realm` (task 027): one character, or every character without them.
fn set_target(character: Option<String>, realm: Option<String>) -> sets::Target {
    match character {
        Some(name) => sets::Target::Character { name, realm },
        None => sets::Target::EveryCharacter,
    }
}

/// The characters a save read or an apply wrote, for a summary: `2 characters`, or the one character's
/// `<account>/<realm>/<name>` when the target named it.
fn characters_text(target: &sets::Target, characters: &[sets::Character]) -> String {
    match (target, characters) {
        (sets::Target::Character { .. }, [one]) => one.label(),
        _ => count_characters(characters.len()),
    }
}

/// `3 addons (2 enabled, 1 disabled)`: the core's [`AddonSet::counted`], the line the window's edit
/// summary uses too.
fn count_set(set: &AddonSet) -> String {
    set.counted()
}

fn count_characters(count: usize) -> String {
    if count == 1 {
        "1 character".to_string()
    } else {
        format!("{count} characters")
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use mrgl_core::keys::KeyStore;

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
            forever_message(&status(ForeverBuild::Yes, false, false), "github:o/r"),
            (
                Kind::Success,
                "forever build: yes (release.json names forever)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::Unknown, false, false), "github:o/r"),
            (
                Kind::Warning,
                "forever build: unknown (no release.json says)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::Yes, true, false), "github:o/r"),
            (
                Kind::Warning,
                "forever build: yes (release.json names forever); note: threat colors are off (verified 2026-06-01, https://example.test/note)".to_string()
            )
        );
        assert_eq!(
            forever_message(&status(ForeverBuild::No, true, true), "github:o/r").1,
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
        // The line alone, without an object (a text listing marks lines, the document objects).
        let mut line = "Line".to_string();
        mark_forever(Some(&forever), addon, Some(&mut line), None);
        assert_eq!(line, "Line  forever:yes,note,stale");
    }

    #[test]
    fn a_max_unpacked_size_is_parsed_and_anything_else_is_refused_with_the_forms() {
        assert_eq!(parse_max_unpacked("3G"), Ok(3 << 30));
        assert_eq!(parse_max_unpacked("500M"), Ok(500 << 20));
        assert_eq!(
            parse_max_unpacked("lots"),
            Err(
                "not a size: lots: a whole number of bytes, or with K, M, G or T (3G, 500M)"
                    .to_string()
            )
        );
        assert_eq!(parse_channel("beta"), Ok(Channel::Prerelease));
        assert_eq!(
            parse_channel("nightly"),
            Err("unknown channel nightly: stable or prerelease".to_string())
        );
    }

    #[test]
    fn a_closed_pipe_is_the_windows_no_data_error_or_a_broken_pipe_and_nothing_else() {
        #[cfg(windows)]
        assert!(is_closed_pipe(&std::io::Error::from_raw_os_error(
            ERROR_NO_DATA
        )));
        assert!(is_closed_pipe(&std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "the reader is gone"
        )));
        assert!(!is_closed_pipe(&std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "a read-only handle"
        )));
        assert!(!is_closed_pipe(&std::io::Error::from_raw_os_error(5)));
    }

    /// An input whose every read fails, as a standard input handle that cannot be read does.
    struct UnreadableInput;

    impl std::io::Read for UnreadableInput {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("the standard input cannot be read"))
        }
    }

    impl BufRead for UnreadableInput {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Err(std::io::Error::other("the standard input cannot be read"))
        }

        fn consume(&mut self, _: usize) {}
    }

    #[test]
    fn a_confirmation_is_y_or_yes_in_any_case_and_nothing_else() {
        for (answer, confirmed_by) in [
            ("y\n", true),
            ("YES\r\n", true),
            ("  yes  \n", true),
            ("n\n", false),
            ("yes please\n", false),
            ("\n", false),
            ("", false),
        ] {
            assert_eq!(
                confirmed(&mut std::io::Cursor::new(answer)),
                confirmed_by,
                "{answer:?}"
            );
        }
        assert!(!confirmed(&mut UnreadableInput));
    }

    /// A reporter of the plain line only, so a unit test's output stays one line per message.
    fn serious(json: bool) -> Reporter {
        Reporter {
            voice: Voice::new(true),
            json,
        }
    }

    #[test]
    fn the_key_prompt_is_said_on_a_terminal_and_one_line_of_it_is_the_key() {
        let store = keys::MemoryStore::new();
        let env = |_: &str| None;
        let action = || KeyAction::Set {
            source: "wago".to_string(),
            rest: Vec::new(),
        };
        // A terminal: the prompt, then the first line alone is the key.
        let mut input = std::io::Cursor::new("pasted-key\nnot-read\n");
        key_command(&serious(false), action(), &store, &env, &mut input, true)
            .expect("the key is stored");
        assert_eq!(
            store.read(SourceKind::Wago).expect("the store reads"),
            Some("pasted-key".to_string())
        );
        // The same on a terminal in JSON mode: no prompt (the document is the one output), the key read.
        let mut input = std::io::Cursor::new("second-key\n");
        key_command(&serious(true), action(), &store, &env, &mut input, true)
            .expect("the key is stored");
        assert_eq!(
            store.read(SourceKind::Wago).expect("the store reads"),
            Some("second-key".to_string())
        );
        // A pipe is read whole, so two lines are not one key.
        let mut input = std::io::Cursor::new("pasted-key\nsecond\n");
        let error = key_command(&serious(false), action(), &store, &env, &mut input, false)
            .expect_err("two lines are refused");
        assert_eq!(
            error,
            "no key read from the standard input: the standard input holds more than one word or line, not one key"
        );
        assert!(!error.contains("pasted-key"));
    }

    #[test]
    fn the_update_confirmation_hands_the_updates_back_on_a_yes_and_none_on_a_no() {
        let reporter = serious(false);
        let updates = vec!["OneFolder", "SeveralFolders"];
        // `--yes`, and nothing to update: no question asked, the input untouched.
        let mut input = std::io::Cursor::new("n\n");
        assert_eq!(
            confirmed_updates(
                &reporter,
                updates.clone(),
                true,
                "_forever_",
                &mut input,
                false
            ),
            Ok(updates.clone())
        );
        assert_eq!(
            confirmed_updates(
                &reporter,
                Vec::<&str>::new(),
                false,
                "_forever_",
                &mut input,
                false
            ),
            Ok(Vec::new())
        );
        assert_eq!(input.position(), 0, "nothing read");
        // On a terminal the answer decides.
        let mut input = std::io::Cursor::new("y\n");
        assert_eq!(
            confirmed_updates(
                &reporter,
                updates.clone(),
                false,
                "_forever_",
                &mut input,
                true
            ),
            Ok(updates.clone())
        );
        let mut input = std::io::Cursor::new("n\n");
        assert_eq!(
            confirmed_updates(
                &reporter,
                updates.clone(),
                false,
                "_forever_",
                &mut input,
                true
            ),
            Ok(Vec::new())
        );
        // Nobody can answer: without a terminal, or in JSON mode.
        let mut input = std::io::Cursor::new("y\n");
        assert_eq!(
            confirmed_updates(&reporter, updates.clone(), false, "_forever_", &mut input, false),
            Err("2 addons to update, nothing updated: no terminal to answer the confirmation; --yes updates without asking".to_string())
        );
        assert_eq!(
            confirmed_updates(&serious(true), updates, false, "_forever_", &mut input, true),
            Err("2 addons to update, nothing updated: --json needs --yes, nobody is there to answer the confirmation".to_string())
        );
        assert_eq!(input.position(), 0, "nothing read");
    }

    #[test]
    fn a_file_name_is_the_last_part_of_a_path_or_the_whole_path_without_one() {
        assert_eq!(
            file_name_of(Path::new("backups/WTF-2026.zip")),
            "WTF-2026.zip"
        );
        assert_eq!(file_name_of(Path::new("")), "");
        assert_eq!(file_name_of(Path::new("..")), "..");
        #[cfg(windows)]
        assert_eq!(file_name_of(Path::new(r"C:\")), r"C:\");
    }

    #[test]
    fn an_import_line_names_what_happened_to_the_entry_and_why() {
        let item = |outcome, version: Option<&str>, detail: &str| list_file::Imported {
            name: "Thing".to_string(),
            source: "svn:Example/Thing".to_string(),
            listed_version: "1.0.0".to_string(),
            outcome,
            version: version.map(str::to_string),
            folders: Vec::new(),
            detail: detail.to_string(),
            interface_mismatch: None,
            interface_out_of_date: None,
            dependencies: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(
            import_line(&item(
                Outcome::NotImportable,
                None,
                "not a source spec mrgl reads"
            )),
            "Thing  not importable (svn:Example/Thing): not a source spec mrgl reads"
        );
        assert_eq!(
            import_line(&item(
                Outcome::Skipped,
                None,
                "a linked development folder; link it again with mrgl link"
            )),
            "Thing  skipped (svn:Example/Thing): a linked development folder; link it again with mrgl link"
        );
        assert_eq!(
            import_line(&item(Outcome::Installed, Some("1.1.0"), "")),
            "Thing  installed 1.1.0 from svn:Example/Thing, the list had 1.0.0"
        );
    }

    #[test]
    fn the_counts_say_one_in_the_singular() {
        assert_eq!(count_characters(1), "1 character");
        assert_eq!(count_characters(2), "2 characters");
        assert_eq!(count_addons(1), "1 addon");
        assert_eq!(count_folders(0), "0 folders");
    }
}
