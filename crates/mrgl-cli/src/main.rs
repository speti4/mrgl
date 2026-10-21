//! The `mrgl` command line. Every human-readable line goes through the voice (D-005): a murloc line and a
//! plain English line, `--serious` or `MRGL_SERIOUS=1` for the plain line alone, `--json` for machines.
//! Exit codes: 0 success, 1 an error the plain line explains, 2 a usage error (clap's own).
//!
//! The subcommands are the M1 plan (`tasks/roadmap.md`); each one answers "not implemented yet" with its
//! task id until that task lands, so the help text and the voice are testable from the first commit.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
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
    /// The command's name and the task that builds it, for the "not yet" answer.
    fn planned(&self) -> (&'static str, &'static str) {
        match self {
            Command::Scan { .. } | Command::List { .. } => ("scan / list", "002"),
            Command::Add { .. } => ("add", "003"),
            Command::Update { all: true, .. } => ("update --all", "006"),
            Command::Update { .. } | Command::Remove { .. } => ("update / remove", "004"),
            Command::Link { .. } | Command::Unlink { .. } => ("link / unlink", "005"),
            Command::Backup { .. } => ("backup", "013"),
        }
    }
}

/// Prints the two-line messages, or JSON when asked.
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
            let value = serde_json::json!({ "kind": "error", "message": plain.to_string() });
            eprintln!("{value}");
        } else {
            eprintln!("{}", self.voice.say(Kind::Error, plain));
        }
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

    let (name, task) = command.planned();
    reporter.fail(Error::NotImplemented {
        command: name,
        task,
    });
    ExitCode::from(1)
}
