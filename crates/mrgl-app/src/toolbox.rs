//! The window's toolbox of a flavor (task 074): the addon sets with the characters a set can be applied to
//! (tasks 014 and 027), the undo of the last apply, the edit of a stored set and the account of a
//! character that sits under two accounts (task 085 over task 073), and the SavedVariables with their
//! sizes and the cleanup of the orphans (task 025). Plain functions over the core, which take the game
//! root, the flavor and the mrgl home folder as arguments, so `tests/toolbox.rs` runs them on a made-up
//! flavor without a window; the commands of [`crate::commands`] call them on a blocking thread.
//!
//! The `WTF` writes (the apply, the undo, the cleanup) run the running-client guard of task 023 first
//! (`client::ensure_client_not_running`) and answer its refusal through the voice before anything else
//! happens; the core calls the same guard again before its first write. An edit writes `sets.json` only
//! and runs while the client runs, as on the command line. Every summary is the core's (`Applied`,
//! `Undone`, `Edited`, `Listing` and `Cleaned`, tasks 085 and 086), the line the command line prints too.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::flavor::{self, Flavor};
use mrgl_core::savedvars::{self, CleanupBy, SavedVariablesFile, Status};
use mrgl_core::scan;
use mrgl_core::sets::{self, AddonState, Character, SetEdit, Target, UndoRecord, Undone};
use mrgl_core::voice::{Kind, Voice};
use mrgl_core::zip_entry::size_text;
use serde::Serialize;

use crate::commands::Message;
use crate::list;

/// The one toolbox write the window allows at a time: a set apply or a cleanup.
#[derive(Debug, Default)]
pub struct ToolboxRun {
    running: AtomicBool,
}

impl ToolboxRun {
    /// Starts a write: `None` while another one runs. Dropping the guard ends it.
    pub fn start(&self) -> Option<ToolboxGuard<'_>> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        Some(ToolboxGuard(self))
    }

    /// Whether a write is going.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
}

/// A toolbox write in progress; dropping it ends the write.
#[derive(Debug)]
pub struct ToolboxGuard<'a>(&'a ToolboxRun);

impl Drop for ToolboxGuard<'_> {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Release);
    }
}

/// The toolbox of one flavor as the panel draws it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolboxView {
    /// The `.flavor.info` id (`wowf`).
    pub flavor: String,
    /// The flavor folder's name (`_forever_`).
    pub folder: String,
    /// The saved sets, by name.
    pub sets: Vec<SetView>,
    /// The line under the set picker: how many sets there are and where, or that there is none yet.
    pub sets_note: Message,
    /// The characters of the flavor's `WTF` folder, sorted; a set goes to every one of them or to one.
    pub characters: Vec<CharacterView>,
    /// What an undo puts back: the last apply that changed a file (task 085); `None` when there is
    /// nothing to undo.
    pub undo: Option<UndoView>,
    pub saved_variables: SavedVariablesView,
    /// The client running from the flavor folder, as a warning: both writes are refused while it runs.
    /// `None` when no client runs.
    pub client: Option<Message>,
}

/// One saved set.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SetView {
    pub name: String,
    /// When it was saved, RFC 3339 in UTC.
    pub saved_at: String,
    pub enabled: usize,
    pub disabled: usize,
    /// Every addon of the set with the state an apply gives it, by name.
    pub addons: Vec<SetAddon>,
}

/// One addon of a set.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SetAddon {
    pub name: String,
    pub state: AddonState,
    /// A folder of its name is under `Interface\AddOns`; an apply writes the line either way.
    pub installed: bool,
}

/// A character a set can be applied to. The account folder's name is left out, as in the command line's
/// plain lines (on an older install it is the login name), unless another account has a character of the
/// same realm and name: then the pick needs it (task 085).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CharacterView {
    pub name: String,
    pub realm: String,
    /// `<realm>/<character>`.
    pub label: String,
    /// The account folder's name, only when the realm and the name sit under two accounts.
    pub account: Option<String>,
}

impl CharacterView {
    /// A character as the window names it, among the flavor's characters `all`.
    fn among(character: &Character, all: &[Character]) -> CharacterView {
        CharacterView::of_parts(&character.account, &character.realm, &character.name, all)
    }

    fn of_parts(account: &str, realm: &str, name: &str, all: &[Character]) -> CharacterView {
        let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
        let twin = all.iter().any(|other| {
            !same(&other.account, account) && same(&other.realm, realm) && same(&other.name, name)
        });
        CharacterView {
            name: name.to_string(),
            realm: realm.to_string(),
            label: format!("{realm}/{name}"),
            account: twin.then(|| account.to_string()),
        }
    }
}

/// An undo of the last apply (task 085): in the toolbox, what pressing it puts back; in an answer, what it
/// put back.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UndoView {
    /// The set that was applied, its name as stored.
    pub set: String,
    /// When it was applied, RFC 3339 in UTC.
    pub applied_at: String,
    /// The characters whose `AddOns.txt` goes back to what it was before the apply.
    pub restore: Vec<CharacterView>,
    /// The characters whose `AddOns.txt` the apply created, which the undo removes.
    pub remove: Vec<CharacterView>,
}

impl UndoView {
    fn of_record(record: &UndoRecord, all: &[Character]) -> UndoView {
        let (kept, created): (Vec<_>, Vec<_>) =
            record.files.iter().partition(|file| file.kept.is_some());
        let views = |files: Vec<&sets::UndoFile>| -> Vec<CharacterView> {
            files
                .iter()
                .map(|file| {
                    CharacterView::of_parts(&file.account, &file.realm, &file.character, all)
                })
                .collect()
        };
        UndoView {
            set: record.set.clone(),
            applied_at: record.applied_at.clone(),
            restore: views(kept),
            remove: views(created),
        }
    }

    fn of_undone(undone: &Undone, all: &[Character]) -> UndoView {
        let views = |characters: &[Character]| -> Vec<CharacterView> {
            characters
                .iter()
                .map(|character| CharacterView::among(character, all))
                .collect()
        };
        UndoView {
            set: undone.set.clone(),
            applied_at: undone.applied_at.clone(),
            restore: views(&undone.restored),
            remove: views(&undone.removed),
        }
    }
}

/// The SavedVariables of the flavor, the largest first.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SavedVariablesView {
    pub files: Vec<SavedVariablesRow>,
    /// The bytes of every entry, and the same as text (`16.0 KB`).
    pub bytes: u64,
    pub size: String,
    pub orphans: usize,
    pub orphan_bytes: u64,
    pub orphan_size: String,
    /// The line over the table: the count, the size and the orphans, as `mrgl savedvars` sums them up.
    pub summary: Message,
}

/// One addon's SavedVariables in one place: the `.lua`, its `.lua.bak`, or both.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SavedVariablesRow {
    pub addon: String,
    /// `account-wide`, or `<realm> / <character>`.
    pub place: String,
    pub realm: Option<String>,
    pub character: Option<String>,
    /// The `.lua` and its `.lua.bak` together, and the same as text.
    pub bytes: u64,
    pub size: String,
    pub lua: bool,
    pub bak: bool,
    /// `installed`, `orphan` (the cleanup moves it), `client` (the client's own) or `unknown` (no AddOns
    /// folder to compare with).
    pub status: Status,
}

impl SavedVariablesRow {
    fn of(file: &SavedVariablesFile) -> SavedVariablesRow {
        SavedVariablesRow {
            addon: file.addon.clone(),
            place: file.place(),
            realm: file.realm.clone(),
            character: file.character.clone(),
            bytes: file.bytes,
            size: size_text(file.bytes),
            lua: file.lua.is_some(),
            bak: file.bak.is_some(),
            status: file.status,
        }
    }
}

/// What a character's `AddOns.txt` got from an apply.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChangeView {
    pub character: CharacterView,
    /// Every addon whose line changed or was added, with its new state; empty when the file already
    /// matched.
    pub changed: Vec<SetAddonChange>,
    /// The character had no `AddOns.txt`, and the apply wrote one.
    pub created: bool,
}

/// One line an apply changed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SetAddonChange {
    pub name: String,
    pub state: AddonState,
}

/// The answer of the toolbox commands: the toolbox as it is now (`None` when the flavor cannot be read),
/// what an apply changed per character, what an undo put back, the zip a cleanup wrote, and the messages
/// in order, the warnings first and the summary last.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolboxAnswer {
    pub toolbox: Option<ToolboxView>,
    pub changes: Vec<ChangeView>,
    pub undone: Option<UndoView>,
    pub zip: Option<String>,
    pub messages: Vec<Message>,
}

impl ToolboxAnswer {
    /// An answer of one message and no toolbox: nothing could be read or done.
    pub fn refused(message: Message) -> ToolboxAnswer {
        ToolboxAnswer {
            toolbox: None,
            changes: Vec::new(),
            undone: None,
            zip: None,
            messages: vec![message],
        }
    }

    /// An answer with the toolbox as it is now and its messages, nothing else.
    fn drawn(toolbox: Option<ToolboxView>, messages: Vec<Message>) -> ToolboxAnswer {
        ToolboxAnswer {
            toolbox,
            changes: Vec::new(),
            undone: None,
            zip: None,
            messages,
        }
    }
}

/// The refusal of a toolbox write while another one runs.
pub fn busy_line() -> &'static str {
    "a set apply, undo or edit or a SavedVariables cleanup is running already; wait for it to end"
}

/// The refusal of an update run while a toolbox write goes.
pub fn toolbox_busy_line() -> &'static str {
    "a set apply, undo or edit or a SavedVariables cleanup is running; the update waits until it ends"
}

/// The refusal of a cleanup while an update runs: an install swaps addon folders, and a folder that is
/// away for a moment would make its SavedVariables look like an orphan.
pub fn updating_line() -> &'static str {
    "an update is running; the SavedVariables cleanup waits until it ends"
}

/// The flavor of a game root by its id, or the error message that says why there is none.
fn read(root: &Path, flavor_id: &str, voice: Voice) -> Result<Flavor, Message> {
    let refused = |plain: String| Message::new(voice, Kind::Error, plain);
    let folder = list::flavor_folder(root, flavor_id).ok_or_else(|| {
        refused(format!(
            "no flavor {flavor_id}: no such flavor folder under {}",
            root.display()
        ))
    })?;
    flavor::read_flavor(&folder).map_err(|error| refused(error.to_string()))
}

/// The toolbox of a flavor: its sets (from `<home>\<flavor-id>\sets.json`), its characters, its
/// SavedVariables listing and whether its client runs. Reads only. A part that cannot be read is empty and
/// its error is among the messages; the rest is drawn all the same.
pub fn load(root: &Path, flavor_id: &str, home: Option<&Path>, voice: Voice) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let mut messages = Vec::new();
    let view = view_of(&flavor, home, voice, &mut messages);
    ToolboxAnswer::drawn(Some(view), messages)
}

/// The toolbox as it is now, after a write: what cannot be read again is left to the next load.
fn view_after(flavor: &Flavor, home: Option<&Path>, voice: Voice) -> Option<ToolboxView> {
    Some(view_of(flavor, home, voice, &mut Vec::new()))
}

/// The toolbox view of a read flavor; what cannot be read goes into `messages`.
fn view_of(
    flavor: &Flavor,
    home: Option<&Path>,
    voice: Voice,
    messages: &mut Vec<Message>,
) -> ToolboxView {
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let folder_name = flavor.folder_name();

    let (sets, sets_note) = match home {
        None => (
            Vec::new(),
            say(
                Kind::Error,
                format!("no sets of {folder_name}: {}", mrgl_core::Error::NoHome),
            ),
        ),
        Some(home) => {
            let path = sets::sets_path_at(home, &flavor.id);
            match sets::read_sets_at(&path, &flavor.id) {
                Ok(file) => {
                    let addons_dir = scan::addons_dir(&flavor.folder);
                    let views: Vec<SetView> = file
                        .sets
                        .iter()
                        .map(|(name, set)| SetView {
                            name: name.clone(),
                            saved_at: set.saved_at.clone(),
                            enabled: set.enabled(),
                            disabled: set.disabled(),
                            addons: set
                                .addons
                                .iter()
                                .map(|(addon, state)| SetAddon {
                                    name: addon.clone(),
                                    state: *state,
                                    installed: addons_dir.join(addon).is_dir(),
                                })
                                .collect(),
                        })
                        .collect();
                    let note = match views.len() {
                        0 => say(
                            Kind::Nothing,
                            format!(
                                "no sets of {folder_name} yet: `mrgl set save <name>` saves one"
                            ),
                        ),
                        1 => say(
                            Kind::Success,
                            format!("1 set of {folder_name} in {}", path.display()),
                        ),
                        count => say(
                            Kind::Success,
                            format!("{count} sets of {folder_name} in {}", path.display()),
                        ),
                    };
                    (views, note)
                }
                Err(error) => {
                    let message = say(Kind::Error, error.to_string());
                    messages.push(message.clone());
                    (Vec::new(), message)
                }
            }
        }
    };

    let all = match sets::characters(flavor) {
        Ok(characters) => characters,
        Err(error) => {
            messages.push(say(Kind::Error, error.to_string()));
            Vec::new()
        }
    };
    let characters = all
        .iter()
        .map(|character| CharacterView::among(character, &all))
        .collect();

    let undo = home.and_then(|home| {
        match sets::undo_record(flavor, &sets::sets_path_at(home, &flavor.id)) {
            Ok(record) => record.map(|record| UndoView::of_record(&record, &all)),
            Err(error) => {
                messages.push(say(Kind::Error, error.to_string()));
                None
            }
        }
    });

    let saved_variables = match savedvars::list_saved_variables(flavor) {
        Ok(listing) => {
            for warning in &listing.warnings {
                messages.push(say(Kind::Warning, warning.clone()));
            }
            SavedVariablesView {
                files: listing.files.iter().map(SavedVariablesRow::of).collect(),
                bytes: listing.bytes,
                size: size_text(listing.bytes),
                orphans: listing.orphans,
                orphan_bytes: listing.orphan_bytes,
                orphan_size: size_text(listing.orphan_bytes),
                summary: said(voice, listing.summary(&folder_name, CleanupBy::Window)),
            }
        }
        Err(error) => {
            let message = say(Kind::Error, error.to_string());
            messages.push(message.clone());
            SavedVariablesView {
                files: Vec::new(),
                bytes: 0,
                size: size_text(0),
                orphans: 0,
                orphan_bytes: 0,
                orphan_size: size_text(0),
                summary: message,
            }
        }
    };

    let client = match client::client_running(flavor) {
        Ok(None) => None,
        Ok(Some(running)) => Some(say(Kind::Warning, running.refusal(flavor).to_string())),
        Err(error) => Some(say(Kind::Warning, error.to_string())),
    };

    ToolboxView {
        flavor: flavor.id.as_str().to_string(),
        folder: folder_name,
        sets,
        sets_note,
        characters,
        undo,
        saved_variables,
        client,
    }
}

/// A summary line of the core (`kind`, plain line) as the window's message.
fn said(voice: Voice, (kind, plain): (Kind, String)) -> Message {
    Message::new(voice, kind, plain)
}

/// The guard of task 023 before a `WTF` write: the refusal line while the client runs, or while the
/// process list cannot be taken (a write is refused rather than risked).
fn guard(flavor: &Flavor, voice: Voice) -> Result<(), Message> {
    client::ensure_client_not_running(flavor)
        .map_err(|error| Message::new(voice, Kind::Error, error.to_string()))
}

/// A pick's text: trimmed, `None` when empty.
fn picked(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// The target of the window's pick: every character without a name, else the one character
/// [`sets::find_character_in_account`] finds by name (and realm) among the flavor's characters, within the
/// account when the pick names one (task 085), pinned to its exact name and realm.
fn target_of(
    flavor: &Flavor,
    character: Option<&str>,
    realm: Option<&str>,
    account: Option<&str>,
) -> mrgl_core::error::Result<Target> {
    let Some(name) = picked(character) else {
        return Ok(Target::EveryCharacter);
    };
    let characters = sets::characters(flavor)?;
    if characters.is_empty() {
        return Err(mrgl_core::Error::NoCharacters {
            path: backup::wtf_dir(flavor),
        });
    }
    let found = sets::find_character_in_account(
        &characters,
        name,
        picked(realm),
        picked(account),
        &flavor.id,
    )?;
    Ok(Target::Character {
        name: found.name,
        realm: Some(found.realm),
    })
}

/// The window's pick of characters for an apply (tasks 074 and 085): every character without a name, else
/// one by name, with its realm, and its account when the same realm and name sit under two accounts.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pick<'a> {
    pub character: Option<&'a str>,
    pub realm: Option<&'a str>,
    pub account: Option<&'a str>,
}

/// Applies a stored set to every character of the flavor (`character` is `None`) or to the one the
/// window picked, through `sets::apply_set_in_account`: the guard first, then the pick, then the apply,
/// which writes only the `AddOns.txt` files that change. The answer carries the toolbox as it is now (with
/// the undo of this apply), what changed per character, and the not-installed warning and the summary of
/// `mrgl set apply`.
pub fn apply(
    root: &Path,
    flavor_id: &str,
    set: &str,
    pick: Pick<'_>,
    home: Option<&Path>,
    voice: Voice,
) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let refused =
        |message: Message| ToolboxAnswer::drawn(view_after(&flavor, home, voice), vec![message]);
    if let Err(message) = guard(&flavor, voice) {
        return refused(message);
    }
    let Some(home) = home else {
        return refused(say(
            Kind::Error,
            format!("no set applied: {}", mrgl_core::Error::NoHome),
        ));
    };
    let account = picked(pick.account);
    let target = match target_of(&flavor, pick.character, pick.realm, account) {
        Ok(target) => target,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };
    let path = sets::sets_path_at(home, &flavor.id);
    let applied = match sets::apply_set_in_account(&flavor, &path, set, &target, account) {
        Ok(applied) => applied,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };
    let all = sets::characters(&flavor).unwrap_or_default();

    let folder_name = flavor.folder_name();
    let mut messages = Vec::new();
    if !applied.not_installed.is_empty() {
        messages.push(say(
            Kind::Warning,
            format!(
                "set {} names addons that are not installed in {folder_name}, their lines are written all the same: {}",
                applied.name,
                applied.not_installed.join(", ")
            ),
        ));
    }
    messages.push(said(voice, applied.summary(&target, &folder_name)));
    let changes = applied
        .characters
        .iter()
        .map(|change| ChangeView {
            character: CharacterView::among(&change.character, &all),
            changed: change
                .changed
                .iter()
                .map(|(name, state)| SetAddonChange {
                    name: name.clone(),
                    state: *state,
                })
                .collect(),
            created: change.created,
        })
        .collect();
    ToolboxAnswer {
        changes,
        ..ToolboxAnswer::drawn(view_after(&flavor, Some(home), voice), messages)
    }
}

/// Undoes the last apply that changed a file (task 085 over task 073), through `sets::undo_apply`: the
/// guard first, then the undo, which puts every `AddOns.txt` the apply replaced back byte for byte and
/// removes the ones it created. The answer carries the toolbox as it is now (nothing left to undo),
/// `undone`, and the warnings and the summary of `mrgl set undo`.
pub fn undo(root: &Path, flavor_id: &str, home: Option<&Path>, voice: Voice) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let refused =
        |message: Message| ToolboxAnswer::drawn(view_after(&flavor, home, voice), vec![message]);
    if let Err(message) = guard(&flavor, voice) {
        return refused(message);
    }
    let Some(home) = home else {
        return refused(say(
            Kind::Error,
            format!("nothing undone: {}", mrgl_core::Error::NoHome),
        ));
    };
    let undone = match sets::undo_apply(&flavor, &sets::sets_path_at(home, &flavor.id)) {
        Ok(undone) => undone,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };
    let mut messages: Vec<Message> = undone
        .warnings
        .iter()
        .map(|warning| say(Kind::Warning, warning.clone()))
        .collect();
    messages.push(said(voice, undone.summary(&flavor.folder_name())));
    let all = sets::characters(&flavor).unwrap_or_default();
    ToolboxAnswer {
        undone: Some(UndoView::of_undone(&undone, &all)),
        ..ToolboxAnswer::drawn(view_after(&flavor, Some(home), voice), messages)
    }
}

/// Enables, disables or drops one addon of a stored set (task 085 over task 073), through
/// `sets::edit_set`: `sets.json` only, no `AddOns.txt` is touched, so it is not behind the running-client
/// guard, as `mrgl set enable|disable|drop` is not. `action` is `enable`, `disable` or `drop`. The answer
/// carries the toolbox as it is now, and the not-installed warning and the summary of the command line.
pub fn edit(
    root: &Path,
    flavor_id: &str,
    set: &str,
    addon: &str,
    action: &str,
    home: Option<&Path>,
    voice: Voice,
) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let refused =
        |message: Message| ToolboxAnswer::drawn(view_after(&flavor, home, voice), vec![message]);
    let edit = match action {
        "enable" => SetEdit::Enable,
        "disable" => SetEdit::Disable,
        "drop" => SetEdit::Drop,
        other => {
            return refused(say(
                Kind::Error,
                format!("no set edit {other}: enable, disable or drop"),
            ));
        }
    };
    let Some(home) = home else {
        return refused(say(
            Kind::Error,
            format!("no set edited: {}", mrgl_core::Error::NoHome),
        ));
    };
    let path = sets::sets_path_at(home, &flavor.id);
    let edited = match sets::edit_set(&flavor, &path, set, addon, edit) {
        Ok(edited) => edited,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };
    let folder_name = flavor.folder_name();
    let mut messages: Vec<Message> = edited
        .not_installed(&folder_name)
        .into_iter()
        .map(|warning| say(Kind::Warning, warning))
        .collect();
    messages.push(said(voice, edited.summary(&folder_name)));
    ToolboxAnswer::drawn(view_after(&flavor, Some(home), voice), messages)
}

/// Moves the flavor's orphan SavedVariables, with their `.bak` files, into `SavedVariables-<date>.zip` in
/// the flavor's backups folder under the mrgl home, through `savedvars::clean_orphans`: the guard first,
/// then the cleanup, which removes a file only once its copy in the finished zip reads back equal. The
/// answer carries the toolbox as it is now, the zip, and the warnings and the summary of `mrgl savedvars
/// --clean`.
pub fn clean(root: &Path, flavor_id: &str, home: Option<&Path>, voice: Voice) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let answer = |messages: Vec<Message>, zip: Option<String>| ToolboxAnswer {
        zip,
        ..ToolboxAnswer::drawn(view_after(&flavor, home, voice), messages)
    };
    if let Err(message) = guard(&flavor, voice) {
        return answer(vec![message], None);
    }
    let Some(home) = home else {
        return answer(
            vec![say(
                Kind::Error,
                format!(
                    "no SavedVariables moved, the backup zip has nowhere to go: {}",
                    mrgl_core::Error::NoHome
                ),
            )],
            None,
        );
    };
    let backups = backup::backups_dir_at(home, &flavor.id);
    let cleaned = match savedvars::clean_orphans(&flavor, &backups) {
        Ok(cleaned) => cleaned,
        Err(error) => return answer(vec![say(Kind::Error, error.to_string())], None),
    };
    let mut messages: Vec<Message> = cleaned
        .warnings
        .iter()
        .map(|warning| say(Kind::Warning, warning.clone()))
        .collect();
    messages.push(said(voice, cleaned.summary(&flavor.folder_name())));
    let zip = cleaned.zip.as_ref().map(|zip| zip.display().to_string());
    answer(messages, zip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_toolbox_write_at_a_time() {
        let run = ToolboxRun::default();
        assert!(!run.is_running());
        let guard = run.start().expect("the first write starts");
        assert!(run.is_running());
        assert!(run.start().is_none(), "a second write waits");
        drop(guard);
        assert!(!run.is_running());
        assert!(run.start().is_some(), "the next write starts");
    }

    #[test]
    fn a_refused_answer_carries_its_one_message() {
        let answer =
            ToolboxAnswer::refused(Message::new(Voice::new(true), Kind::Warning, busy_line()));
        assert_eq!(answer.toolbox, None);
        assert!(answer.changes.is_empty() && answer.zip.is_none());
        assert_eq!(answer.messages.len(), 1);
        assert_eq!(answer.messages[0].kind, "warning");
        assert_eq!(answer.messages[0].plain, busy_line());
    }
}
