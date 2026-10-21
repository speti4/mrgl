//! The error type of the crate. Its `Display` is the plain English line of a message (D-005): the callers
//! wrap it in the voice's error kind, so an error text is written once, here, in lower case and without a
//! trailing full stop, as `docs/nerglish.md` asks.

use std::path::PathBuf;

/// Everything that can go wrong in the core, one variant per cause the user can act on.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A folder is not a game flavor folder: no `.flavor.info`, or nothing readable in it.
    #[error("not a game flavor folder: {path} ({reason})")]
    NotAFlavorFolder { path: PathBuf, reason: String },

    /// A folder is not a game folder: no flavor folder with a `.flavor.info` under it.
    #[error("not a game folder: {path} (no flavor folder with a .flavor.info in it)")]
    NotAGameRoot { path: PathBuf },

    /// Nowhere to keep the config and the sidecars: neither `MRGL_HOME` nor `LOCALAPPDATA` is set.
    #[error("no home folder for mrgl: neither MRGL_HOME nor LOCALAPPDATA is set")]
    NoHome,

    /// A command is planned but not built yet; the task id says where.
    #[error("{command} is not implemented yet (task {task})")]
    NotImplemented {
        command: &'static str,
        task: &'static str,
    },

    /// An I/O error with the path it happened on.
    #[error("{context}: {path}: {source}")]
    Io {
        context: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
