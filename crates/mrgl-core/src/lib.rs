//! mrgl-core: the library behind the `mrgl` command line and the window.
//!
//! The modules follow the roadmap (`tasks/roadmap.md`): [`flavor`] reads a game folder's identity,
//! [`scan`] and [`toc`] list what is installed, [`sources`] fetch release metadata and zips, [`install`]
//! swaps addon folders into `Interface\AddOns`, [`link`] makes the junctions of the addons one develops
//! (`link:` entries the engine never installs over), [`state`] is the sidecar cache (D-003), [`backup`]
//! is the toolbox (WTF backups, addon sets), [`client`] finds the running game client by its executable's
//! path (the guard of every `WTF` write), [`home`] is the mrgl home folder with the remembered game root,
//! [`discover`] resolves a run's game root (the flag, `MRGL_ROOT`, the remembered one, then the launcher's
//! registry key and the usual install folders), [`dependencies`] checks the required dependencies of the
//! scanned addons, [`forever`] holds the Forever notes (the derived build signal and the curated file
//! compiled in, task 011), [`doctor`] is the diagnostic report a bug report carries, [`list_file`] is the
//! addon list `export` writes and `import` installs from and [`interface`] checks a TOC's `## Interface`
//! against the flavor's build (task 015), [`update`] is the update check of the managed addons
//! against their sources (task 006), [`voice`] is the murloc table every message goes through (D-005)
//! and [`zip_entry`] is the one check every zip entry passes before the install engine or a restore writes
//! it.
//! [`savedvars`] lists the SavedVariables files by size and moves the orphans into a backup zip (task 025).
//! Nothing here prints or reads the network on its own: the callers (the CLI, the window) decide how to
//! talk and when to go online, which keeps every module testable against the fixtures under
//! `tests/fixtures/`.

pub mod backup;
pub mod client;
pub mod dependencies;
pub mod discover;
pub mod doctor;
pub mod error;
pub mod flavor;
pub mod forever;
pub mod home;
pub mod install;
pub mod interface;
pub mod link;
pub mod list_file;
pub mod pe;
pub mod savedvars;
pub mod scan;
pub mod sources;
pub mod state;
pub mod toc;
pub mod update;
pub mod voice;
pub mod zip_entry;

pub use error::Error;

/// The crate version, stamped from the workspace `Cargo.toml`, the one place it is written.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
