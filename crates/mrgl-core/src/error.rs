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

    /// A zip the install engine or a restore was given cannot be read: not a zip, a damaged one, or an
    /// entry that cannot be opened for reading (encrypted, a compression method this build does not read);
    /// the role names it a zip or a backup (tasks 004, 013, 044: [`crate::zip_entry`]).
    #[error("not a readable {}: {path} ({reason})", .role.noun())]
    ZipUnreadable {
        role: crate::zip_entry::ZipRole,
        path: PathBuf,
        reason: String,
    },

    /// A zip entry would land outside the folder it is unpacked under (the staging folder, `WTF`): `..`,
    /// an absolute path, a drive letter, a character or a device name Windows refuses, a symbolic link
    /// (tasks 004, 013, 044: [`crate::zip_entry`]). Nothing of the zip is written when one is found.
    #[error("refused {path}: the entry `{entry}` would land outside {}", .role.folder())]
    ZipEntryEscapes {
        role: crate::zip_entry::ZipRole,
        path: PathBuf,
        entry: String,
    },

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

    /// Nothing to back up: the flavor folder has no `WTF` folder (task 013).
    #[error("nothing to back up: no WTF folder in {path}")]
    NoWtf { path: PathBuf },

    /// The backup zip a restore was given does not exist (task 013).
    #[error("backup not found: {path}")]
    BackupNotFound { path: PathBuf },

    /// A restore failed and the previous `WTF` is back as it was (task 013).
    #[error("restoring {path} failed and WTF is as it was: {reason}")]
    RestoreFailed { path: PathBuf, reason: String },

    /// A restore failed and the previous `WTF` could not be put back; it waits under the aside path.
    #[error(
        "restoring {path} failed and the previous WTF could not be put back: {reason}; it is at {aside}"
    )]
    RestoreRevertFailed {
        path: PathBuf,
        reason: String,
        aside: PathBuf,
    },

    /// `link` or `unlink` on a target without directory junctions (task 005).
    #[error("junctions are Windows-only: link and unlink work on Windows")]
    JunctionsWindowsOnly,

    /// A development folder with no addon in it: no subfolder with a TOC named after it (task 005).
    #[error("no addon in {path}: no subfolder with a TOC named after it")]
    NoAddonInDevFolder { path: PathBuf },

    /// An install or a removal over an addon `link` made (task 005): `unlink` is the way out, never
    /// `remove`, and an install never replaces a junction.
    #[error(
        "refused to {action} {name} (flavor {flavor}): it is linked from {folder}; `mrgl unlink` with that folder removes the link"
    )]
    LinkedAddon {
        action: &'static str,
        name: String,
        flavor: String,
        folder: String,
    },

    /// An install over a junction or a symbolic link that mrgl did not make (task 005).
    #[error(
        "refused to install over {folder} (flavor {flavor}): it is a junction or a symbolic link mrgl did not make, to {target}; it is yours to remove"
    )]
    ForeignJunction {
        folder: String,
        flavor: String,
        target: String,
    },

    /// A file `import` was given is not a list file `export` wrote: not JSON, another format or schema
    /// (task 015).
    #[error("not a readable addon list: {path} ({reason})")]
    ListUnreadable { path: PathBuf, reason: String },

    /// A pin, an ignore or a channel change on a folder the sidecar does not know (task 021): only an
    /// addon mrgl installed carries these marks.
    #[error(
        "addon not managed by mrgl: {name} (flavor {flavor}); it was installed by hand or by another manager, and only an addon mrgl installed can be {action}"
    )]
    NotManagedToMark {
        name: String,
        flavor: String,
        action: &'static str,
    },
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
