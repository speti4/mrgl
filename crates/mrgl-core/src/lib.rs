//! mrgl-core: the library behind the `mrgl` command line and the window.
//!
//! The modules follow the roadmap (`tasks/roadmap.md`): [`flavor`] reads a game folder's identity,
//! [`scan`] and [`toc`] list what is installed, [`sources`] fetch release metadata and zips, [`install`]
//! swaps addon folders into `Interface\AddOns`, [`state`] is the sidecar cache (D-003), [`backup`] is the
//! toolbox (WTF backups, addon sets) and [`voice`] is the murloc table every message goes through (D-005).
//! Nothing here prints or reads the network on its own: the callers (the CLI, the window) decide how to
//! talk and when to go online, which keeps every module testable against the fixtures under
//! `tests/fixtures/`.

pub mod backup;
pub mod error;
pub mod flavor;
pub mod home;
pub mod install;
pub mod scan;
pub mod sources;
pub mod state;
pub mod toc;
pub mod voice;

pub use error::Error;

/// The crate version, stamped from the workspace `Cargo.toml`, the one place it is written.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
