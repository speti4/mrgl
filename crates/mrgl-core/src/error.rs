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

    /// The game client runs from the flavor folder: nothing under `WTF` is written while it does (task
    /// 023; D-002, D-003). The guard is [`crate::client::ensure_client_not_running`].
    #[error("the client is running in {flavor} ({}, pid {}): close the game first, nothing under WTF is written while it runs", .client.exe_name(), .client.pid)]
    ClientRunning {
        /// The flavor folder's name (`_forever_`).
        flavor: String,
        client: crate::client::RunningClient,
    },

    /// The running processes could not be listed, so a running client cannot be ruled out.
    #[error("cannot list the running processes: {source}")]
    ProcessList {
        #[source]
        source: std::io::Error,
    },

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

    /// A file the install engine was given is not a zip it can read (task 004).
    #[error("not a readable zip: {path} ({reason})")]
    ZipUnreadable { path: PathBuf, reason: String },

    /// A zip entry would land outside the staging folder: `..`, an absolute path, a drive letter (task 004).
    #[error("refused {path}: the entry `{entry}` would land outside the staging folder")]
    ZipEntryEscapes { path: PathBuf, entry: String },

    /// A zip with no addon folder in it: nothing with a `.toc` at the root or one level down (task 004).
    #[error("no addon in {path}: no folder with a .toc at the root or one level down")]
    NoAddonInZip { path: PathBuf },

    /// A name that is neither a managed addon nor a folder of the flavor (task 004).
    #[error("addon not found: {name} (flavor {flavor})")]
    AddonNotFound { name: String, flavor: String },

    /// A folder the sidecar does not know: installed by hand or by another manager (task 004).
    #[error(
        "addon not managed by mrgl: {name} (flavor {flavor}); it was installed by hand or by another manager, so its folder is yours to delete"
    )]
    NotManaged { name: String, flavor: String },

    /// A folder swap failed and the AddOns folder was put back as it was (task 004).
    #[error("installing {folder} failed and the AddOns folder is as it was: {reason}")]
    SwapFailed { folder: String, reason: String },

    /// A folder swap failed and the revert failed too; the previous folders wait under the staging path.
    #[error(
        "installing {folder} failed and the revert failed too: {reason}; the previous folders are under {staging}"
    )]
    SwapRevertFailed {
        folder: String,
        reason: String,
        staging: PathBuf,
    },

    /// A source spec the manager cannot read: not `github:<owner>/<repo>` (task 003).
    #[error("unknown source: {spec} ({hint})")]
    UnknownSource { spec: String, hint: &'static str },

    /// The network: no connection, a timeout, a TLS failure, a body cut short.
    #[error("cannot reach {url}: {reason}")]
    Network { url: String, reason: String },

    /// A source's server answered with a status the manager cannot work with; the hint says what to do.
    #[error("{host} answered {status} for {url}: {hint}")]
    SourceStatus {
        host: &'static str,
        status: u16,
        url: String,
        hint: String,
    },

    /// A source's answer is not the document the manager expected (a release list, a `release.json`).
    #[error("unreadable {what} from {spec}: {reason}")]
    SourceDocument {
        what: &'static str,
        spec: String,
        reason: String,
    },

    /// The repository has no release to pick from (drafts do not count).
    #[error("no release in {spec}")]
    NoRelease { spec: String },

    /// The release has no asset for the flavor, neither by its `release.json` nor by an asset name.
    #[error("no asset for flavor {flavor} in {spec} {release} ({detail})")]
    NoAssetForFlavor {
        spec: String,
        release: String,
        flavor: String,
        detail: String,
    },
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
