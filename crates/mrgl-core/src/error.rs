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

    /// A source spec the manager cannot read: none of `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>`,
    /// `curse:<id>` or a page address they are read from (tasks 003, 092).
    #[error("unknown source: {spec} ({hint})")]
    UnknownSource { spec: String, hint: &'static str },

    /// A spec of a source this run cannot ask (task 092): this build does not have it, or its key is not
    /// set; `missing` says which ([`crate::sources::Source::missing`]).
    #[error("source not configured: {spec} ({missing})")]
    SourceNotConfigured { spec: String, missing: String },

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

    /// A development folder with no addon in it: no subfolder with a TOC named after it (task 005), and
    /// no TOC named after the folder itself (task 062).
    #[error("no addon in {path}: no TOC named after it and no subfolder with one")]
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

    /// A rollback of an addon that has no previous version: installed once and not updated through mrgl
    /// since (task 022).
    #[error(
        "nothing to roll back to: {name} {version} has no previous version (flavor {flavor}); mrgl keeps the version an update replaces"
    )]
    NoPreviousVersion {
        name: String,
        version: String,
        flavor: String,
    },

    /// The cached zip of the version a rollback goes to is gone: deleted by hand, or never written (task
    /// 022). Nothing was changed.
    #[error(
        "cannot roll back {name} to {version}: its zip is not in the cache ({path}); install that version again with `mrgl add <zip>`"
    )]
    RollbackZipMissing {
        name: String,
        version: String,
        path: PathBuf,
    },

    /// A SavedVariables cleanup stopped part way (task 025): every file it removed from `WTF` had its
    /// copy in the zip read back and compared first; the others are still where they were.
    #[error(
        "cleaning the SavedVariables stopped: {reason}; {removed} of {total} files were removed from WTF after their copy in {zip} was checked, the others are still in WTF"
    )]
    CleanupIncomplete {
        zip: PathBuf,
        removed: usize,
        total: usize,
        reason: String,
    },

    /// A restore was given the zip of a SavedVariables cleanup (task 025): a restore replaces all of `WTF`,
    /// and that zip holds a few files; it is unzipped into `WTF` by hand.
    #[error(
        "not a WTF backup: {path} holds the SavedVariables a cleanup moved; unzip it into the WTF folder by hand to put them back"
    )]
    NotAWtfBackup { path: PathBuf },

    /// A zip that unpacks to more than the bound (task 039, [`crate::zip_entry::MAX_UNPACKED`] unless
    /// `--max-unpacked` raises it): its entries declare more, found before anything is written, or it
    /// unpacked past the bound or past an entry's declared size while being written, and what it wrote was
    /// removed. The role says whose zip it is (the engine's, a restore's: [`crate::zip_entry`]).
    #[error("refused {path}: {excess}")]
    ZipTooLarge {
        role: crate::zip_entry::ZipRole,
        path: PathBuf,
        excess: crate::zip_entry::Excess,
    },

    /// A release asset the release list names answers 404 on download: removed or renamed after the list
    /// was read (task 046). The release list's own 404 stays a [`Error::SourceStatus`] with its hint.
    #[error(
        "the asset {asset} of {spec} {release} is missing: {host} answered 404 for {url} (removed or renamed after the release was listed)"
    )]
    AssetMissing {
        host: &'static str,
        spec: String,
        release: String,
        asset: String,
        url: String,
    },

    /// A downloaded asset whose byte count is not the size the release list names: cut short or replaced
    /// (task 046). It is refused before anything opens it, so nothing is installed.
    #[error(
        "refused {asset} of {spec} {release}: {got} bytes downloaded, the release list names {expected}; the download was cut short or the asset changed, try again"
    )]
    AssetSizeMismatch {
        spec: String,
        release: String,
        asset: String,
        expected: u64,
        got: u64,
    },

    /// The source has no release of the version asked for (task 057, `import --exact`): it was deleted, or
    /// it was never published under that tag.
    #[error("no release {version} in {spec}")]
    VersionNotOffered { spec: String, version: String },

    /// A list file with an entry that names a zip file, a linked folder or another path of one machine
    /// (task 028): a shareable list names sources and versions only. `entries` names each such entry and
    /// why, `; `-separated. Nothing was installed.
    #[error(
        "not a shareable addon list: {path} ({entries}); a list names sources and versions, never a file or a folder of one machine"
    )]
    ListNotShareable { path: PathBuf, entries: String },
    /// A sets file that is not one mrgl wrote for this flavor: not JSON, another format, schema or
    /// flavor, or an addon name no `AddOns.txt` line could carry (task 014). It is the user's data, so it
    /// is never overwritten.
    #[error(
        "not a readable sets file: {path} ({reason}); mrgl leaves it as it is, fix or delete it by hand"
    )]
    SetsUnreadable { path: PathBuf, reason: String },

    /// A set name that cannot be stored: empty, too long, a control character in it (task 014).
    #[error("not a set name: `{name}` ({reason})")]
    SetName { name: String, reason: String },

    /// A set name the flavor's sets file does not have (task 014); `known` lists the saved ones.
    #[error("set not found: {name} (flavor {flavor}); {known}")]
    SetNotFound {
        name: String,
        flavor: String,
        known: String,
    },

    /// A `WTF` folder with no character folder in it (`Account\<account>\<realm>\<character>`), or none
    /// at all (task 014).
    #[error(
        "no character in {path}: no Account\\<account>\\<realm>\\<character> folder; log in with a character once first"
    )]
    NoCharacters { path: PathBuf },

    /// A set save that found no addon line in any character's `AddOns.txt` (task 014).
    #[error("nothing to save: no AddOns.txt under WTF of {flavor} names an addon")]
    NothingToSave { flavor: String },

    /// A set apply whose write failed (task 014); `reverted` says whether the files written before it
    /// are back as they were.
    #[error(
        "applying set {set} failed: {reason}; {}",
        if *.reverted { "every AddOns.txt is as it was" } else { "the AddOns.txt files written before it could not all be put back" }
    )]
    SetApplyFailed {
        set: String,
        reason: String,
        reverted: bool,
    },

    /// A character name (and realm) no character folder under `WTF` has (task 027); `known` lists them.
    #[error("character not found: {name} (flavor {flavor}); the characters: {known}")]
    CharacterNotFound {
        name: String,
        flavor: String,
        known: String,
    },

    /// A character name more than one character folder has (task 027): the realms (or realms and
    /// accounts) are listed, and the hint says how to pick one (`by`, task 091: the account's hint names
    /// no flag).
    #[error("character {name} matches {count} characters (flavor {flavor}): {candidates}; {by}")]
    AmbiguousCharacter {
        name: String,
        count: usize,
        flavor: String,
        candidates: String,
        by: crate::sets::CharacterPick,
    },

    /// An account name (`--account`, task 073) no account folder with a character under `WTF\Account`
    /// has; `known` lists the accounts that have one.
    #[error(
        "no character under the account {account} (flavor {flavor}); the accounts with characters: {known}"
    )]
    AccountNotFound {
        account: String,
        flavor: String,
        known: String,
    },

    /// `set undo` with no apply recorded since the last undo (task 073).
    #[error(
        "nothing to undo in {flavor}: no set apply is recorded; an apply that changes a file keeps what it replaced, and an undo puts it back once"
    )]
    NothingToUndo { flavor: String },

    /// An undo record that is not one mrgl wrote for this flavor, or whose kept files are missing (task
    /// 073). Nothing is written.
    #[error(
        "not a readable undo record: {path} ({reason}); nothing was written, delete the folder by hand to start over"
    )]
    SetUndoUnreadable { path: PathBuf, reason: String },

    /// A set undo whose write failed (task 073); `reverted` says whether the files written before it are
    /// back as they were.
    #[error(
        "undoing set {set} failed: {reason}; {}",
        if *.reverted { "every AddOns.txt is as it was before the undo" } else { "the AddOns.txt files written before it could not all be put back" }
    )]
    SetUndoFailed {
        set: String,
        reason: String,
        reverted: bool,
    },

    /// An addon name an edit of a set names that no `AddOns.txt` line could carry (task 073).
    #[error("not an addon name: `{name}` (an AddOns.txt line cannot carry it)")]
    AddonName { name: String },

    /// `set drop` of an addon the set does not have (task 073); `known` lists the set's addons.
    #[error("set {set} (flavor {flavor}) has no addon {addon}; its addons: {known}")]
    AddonNotInSet {
        set: String,
        addon: String,
        flavor: String,
        known: String,
    },
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
