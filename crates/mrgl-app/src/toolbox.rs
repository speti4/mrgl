//! The window's toolbox of a flavor (task 074): the addon sets with the characters a set can be applied to
//! (tasks 014 and 027), the undo of the last apply, the edit of a stored set and the account of a
//! character that sits under two accounts (task 085 over task 073), and the SavedVariables with their
//! sizes and the cleanup of the orphans (task 025), and the adopt report of the flavor's unmanaged addons
//! with its apply (task 109 over tasks 097 and 098). Plain functions over the core, which take the game
//! root, the flavor, the mrgl home folder (and for adopt the sources) as arguments, so `tests/toolbox.rs`
//! and `tests/adopt.rs` run them on made-up flavors without a window; the commands of [`crate::commands`]
//! call them on a blocking thread.
//!
//! The `WTF` writes (the apply, the undo, the cleanup) run the running-client guard of task 023 first
//! (`client::ensure_client_not_running`) and answer its refusal through the voice before anything else
//! happens; the core calls the same guard again before its first write. An edit writes `sets.json` only
//! and runs while the client runs, as on the command line. Every summary is the core's (`Applied`,
//! `Undone`, `Edited`, `Listing` and `Cleaned`, tasks 085 and 086; the set list note `SetsFile::summary`
//! and the apply's `Applied::not_installed_warning`, task 091), the line the command line prints too, and
//! so is an error of the pick (the two-account error names no flag, task 091).
//!
//! Adopt writes the state sidecar only, never a `WTF` file, so it is not behind the running-client guard,
//! as `mrgl adopt --apply` is not; its lines are `adopt.rs`'s, the command line's, and so is the guard of
//! a folder the CurseForge app manages: the window's confirmation dialog stands for
//! `--alongside-curseforge`. The summary, the marker line and the refusal name the window's Adopt button
//! and its confirmation instead of the command line's flags (`AdoptBy::Window`, task 112). The card's
//! report carries its sources' answers (`adopt::Answers`), and the apply takes them back: it and the report
//! after it ask no source again while they stand (`Answers::stand_for`, an hour; task 113).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mrgl_core::adopt::{self, AdoptBy, Adoptable, Answers, Counts, Report};
use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::flavor::{self, Flavor};
use mrgl_core::install::Engine;
use mrgl_core::savedvars::{self, CleanupBy, SavedVariablesFile, Status};
use mrgl_core::scan::{self, Scan};
use mrgl_core::sets::{self, AddonState, Character, SetEdit, Target, UndoRecord, Undone};
use mrgl_core::sources::{Source, SourceKind};
use mrgl_core::state::State;
use mrgl_core::voice::{Kind, Voice};
use mrgl_core::zip_entry::size_text;
use serde::Serialize;

use crate::commands::Message;
use crate::list::{self, FlavorView};

/// The one toolbox write the window allows at a time: a set apply, undo or edit, an adopt or a cleanup.
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
    "a set apply, undo or edit, an adopt or a SavedVariables cleanup is running already; wait for it to end"
}

/// The refusal of an update run while a toolbox write goes.
pub fn toolbox_busy_line() -> &'static str {
    "a set apply, undo or edit, an adopt or a SavedVariables cleanup is running; the update waits until it ends"
}

/// The refusal of an add of a zip while a toolbox write goes (task 116): an adopt writes the flavor's
/// sidecar too, a cleanup reads the AddOns folders an install swaps.
pub fn add_waits_line() -> &'static str {
    "a set apply, undo or edit, an adopt or a SavedVariables cleanup is running; the zip waits until it ends"
}

/// The refusal of a mark while a toolbox write goes: an adopt writes the flavor's sidecar too (task 109).
pub fn marks_wait_line() -> &'static str {
    "a set apply, undo or edit, an adopt or a SavedVariables cleanup is running; pin, ignore and the channel wait until it ends"
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
                    (views, said(voice, file.summary(&path, &folder_name)))
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
        // coverage: os-begin — the system's process list could not be taken
        Err(error) => Some(say(Kind::Warning, error.to_string())),
        // coverage: os-end
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
    let mut messages: Vec<Message> = applied
        .not_installed_warning(&folder_name)
        .into_iter()
        .map(|warning| say(Kind::Warning, warning))
        .collect();
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

/// The adopt card of the toolbox (task 109 over tasks 097 and 098): the adopt report of the flavor as
/// `mrgl adopt` prints it, a row per unmanaged addon with its folder group, its sources and what adopt
/// makes of it, the counts, the summary and the CurseForge app's marker.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdoptView {
    /// The `.flavor.info` id (`wowf`).
    pub flavor: String,
    /// The flavor folder's name (`_forever_`).
    pub folder: String,
    /// The CurseForge app's `.curseclient` marker is in the flavor folder: the apply asks first.
    pub curse_client: bool,
    /// The marker's warning (the core's `Report::marker_line`); `None` without the marker.
    pub marker: Option<Message>,
    /// The unmanaged addons, by name.
    pub rows: Vec<AdoptRow>,
    /// The unmanaged folders left out: a junction or link mrgl did not make.
    pub left_out: Vec<String>,
    pub counts: Counts,
    /// The summary of the report (the core's `Report::summary`).
    pub summary: Message,
    /// The sources' answers the report was built from (task 113): the card hands them to the apply, which
    /// asks no source again while they stand.
    pub answers: Answers,
}

/// One unmanaged addon of the report: the core's row (`name`, `folders`, `version`, `sources`, `pages`,
/// `class`, `adopt_as`, `needs`, as `mrgl adopt --json` writes it) and its lines.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdoptRow {
    #[serde(flatten)]
    pub addon: Adoptable,
    /// The addon's line (`Adoptable::line`).
    pub line: String,
    /// What adopt makes of it (`Adoptable::verdict`): `adopt as <spec>`, what the run lacks, or why it
    /// has no source.
    pub verdict: String,
    /// A line per source and per page under it (`Adoptable::detail_lines`, without the indent).
    pub details: Vec<String>,
}

/// The answer of the adopt commands: the report as it is now (`None` when nothing could be read, or the
/// apply was refused before it asked a source), the flavor with its rows as they are now after an apply
/// (the adopted folders managed), the `adopted …` line of every entry the apply wrote, and the messages in
/// order, the warnings first and the summary last.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdoptAnswer {
    pub adopt: Option<AdoptView>,
    pub flavor: Option<FlavorView>,
    pub adopted: Vec<String>,
    pub messages: Vec<Message>,
}

impl AdoptAnswer {
    /// An answer of one message and nothing else.
    pub fn refused(message: Message) -> AdoptAnswer {
        AdoptAnswer {
            adopt: None,
            flavor: None,
            adopted: Vec::new(),
            messages: vec![message],
        }
    }
}

/// What the card sends with an apply: the dialog's confirmation and the answers of the report it shows.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdoptRequest<'a> {
    /// The confirmation of the dialog in a folder the CurseForge app manages, the window's
    /// `--alongside-curseforge`.
    pub alongside: bool,
    /// The sources' answers of the report the card shows (task 113); `None` asks every source again.
    pub answers: Option<&'a Answers>,
}

/// The refusal of an adopt apply while an update runs: both write the flavor's sidecar.
pub fn adopt_updating_line() -> &'static str {
    "an update is running; the adopt waits until it ends"
}

/// The report of a flavor as the card draws it, with the answers it was built from.
fn adopt_view(report: &Report, answers: &Answers, voice: Voice) -> AdoptView {
    let flavor_name = report.flavor_name();
    AdoptView {
        flavor: report.flavor.id.as_str().to_string(),
        folder: report.flavor.folder_name(),
        curse_client: report.curse_client,
        marker: report
            .marker_line(AdoptBy::Window)
            .map(|line| Message::new(voice, Kind::Warning, line)),
        rows: report
            .addons
            .iter()
            .map(|addon| AdoptRow {
                line: addon.line(flavor_name),
                verdict: addon.verdict(flavor_name),
                details: addon
                    .detail_lines(flavor_name)
                    .iter()
                    .map(|line| line.trim_start().to_string())
                    .collect(),
                addon: addon.clone(),
            })
            .collect(),
        left_out: report.left_out.clone(),
        counts: report.counts,
        summary: said(voice, report.summary(AdoptBy::Window)),
        answers: answers.clone(),
    }
}

/// What a report starts from: the scan of the flavor folder, the engine on the mrgl home, the sidecar
/// with the entries whose folders are gone dropped (as `mrgl adopt` reads it), and the warnings met.
struct Adopting {
    scan: Scan,
    engine: Engine,
    state: State,
    warnings: Vec<String>,
}

fn adopting(flavor: &Flavor, home: &Path) -> Result<Adopting, String> {
    let scan = scan::scan(&flavor.folder).map_err(|error| error.to_string())?;
    let mut warnings: Vec<String> = scan.flavor.warnings.clone();
    warnings.extend(scan.warnings.iter().cloned());
    let engine = Engine::with_home(scan.flavor.clone(), home);
    let loaded = engine.load_state().map_err(|error| error.to_string())?;
    warnings.extend(loaded.warning);
    let mut state = loaded.state;
    state.reconcile(&scan);
    Ok(Adopting {
        scan,
        engine,
        state,
        warnings,
    })
}

/// The adopt report of a flavor (task 109 over task 097, `adopt::report`): every unmanaged addon with its
/// folder group and its sources, each asked through `sources` whether it has a build for the flavor;
/// `has_key` tells a missing key from a source this build lacks. Reads the sidecar and writes nothing (the
/// sources keep their own caches). The answer carries the report with the sources' answers it was built
/// from (`adopt::Answers`, task 113), and the warnings and the summary of `mrgl adopt`.
pub fn adopt_report(
    root: &Path,
    flavor_id: &str,
    home: Option<&Path>,
    sources: &dyn Source,
    has_key: &dyn Fn(SourceKind) -> bool,
    voice: Voice,
) -> AdoptAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return AdoptAnswer::refused(message),
    };
    let refused = |plain: String| AdoptAnswer::refused(Message::new(voice, Kind::Error, plain));
    let Some(home) = home else {
        return refused(format!("no adopt report: {}", mrgl_core::Error::NoHome));
    };
    let Adopting {
        scan,
        state,
        warnings,
        ..
    } = match adopting(&flavor, home) {
        Ok(adopting) => adopting,
        Err(plain) => return refused(plain),
    };
    let (report, answers) =
        adopt::report_answering(&scan, &state, sources, has_key, Answers::new(&flavor.id));
    let view = adopt_view(&report, &answers, voice);
    let mut messages: Vec<Message> = warnings
        .into_iter()
        .chain(report.left_out_lines())
        .map(|line| Message::new(voice, Kind::Warning, line))
        .collect();
    messages.push(view.summary.clone());
    AdoptAnswer {
        adopt: Some(view),
        flavor: None,
        adopted: Vec::new(),
        messages,
    }
}

/// Adopts the report's addons (task 109 over task 098, `adopt::apply`): one sidecar entry per addon with a
/// source to be adopted as, no download and no folder change, the sidecar written once when an entry was
/// added. A flavor folder with the CurseForge app's marker is refused before any source is asked
/// (`adopt::marker_refusal`) unless the request's `alongside` says so, the window's confirmation of
/// `--alongside-curseforge`; then the entries are written and the answer warns
/// (`adopt::alongside_warning`). The marker is never touched. The answer carries the report as it is now
/// (the adopted addons are managed, so they are no rows any more), the flavor's rows as they are now, the
/// `adopted …` lines, and the warnings and the summary of `mrgl adopt --apply`.
///
/// The request's answers are those of the report the card shows (task 113): the report the apply writes
/// from and the report after it are built over the folders and the sidecar as they are now and those
/// answers, so no source is asked again (one that arrived since is). Answers of another flavor, an hour old
/// or older (`adopt::ANSWERS_MAX_AGE_SECONDS`), or none, are asked again first, as the command line asks.
pub fn adopt_apply(
    root: &Path,
    flavor_id: &str,
    request: AdoptRequest<'_>,
    home: Option<&Path>,
    sources: &dyn Source,
    has_key: &dyn Fn(SourceKind) -> bool,
    voice: Voice,
) -> AdoptAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return AdoptAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let Some(home) = home else {
        return AdoptAnswer::refused(say(
            Kind::Error,
            format!("nothing adopted: {}", mrgl_core::Error::NoHome),
        ));
    };
    if flavor.curse_client && !request.alongside {
        return AdoptAnswer::refused(say(
            Kind::Error,
            adopt::marker_refusal(&flavor, AdoptBy::Window),
        ));
    }
    let Adopting {
        scan,
        engine,
        mut state,
        warnings,
    } = match adopting(&flavor, home) {
        Ok(adopting) => adopting,
        Err(plain) => return AdoptAnswer::refused(say(Kind::Error, plain)),
    };
    let answers = request
        .answers
        .filter(|answers| answers.stand_for(&flavor.id))
        .cloned()
        .unwrap_or_else(|| Answers::new(&flavor.id));
    let (report, answers) = adopt::report_answering(&scan, &state, sources, has_key, answers);
    let applied = adopt::apply(&report, &mut state);
    if !applied.adopted.is_empty()
        && let Err(error) = engine.home.write_state(&state)
    {
        return AdoptAnswer {
            adopt: Some(adopt_view(&report, &answers, voice)),
            ..AdoptAnswer::refused(say(Kind::Error, error.to_string()))
        };
    }

    let mut messages: Vec<Message> = Vec::new();
    if report.curse_client {
        messages.push(say(Kind::Warning, adopt::alongside_warning(&report.flavor)));
    }
    messages.extend(
        warnings
            .into_iter()
            .chain(report.left_out_lines())
            .chain(applied.skipped.iter().map(|skipped| skipped.line()))
            .map(|line| say(Kind::Warning, line)),
    );
    messages.push(said(voice, report.applied_summary(&applied)));
    // The report as it is now, over the written sidecar and the same answers: the adopted folders are
    // managed, so only the rest are rows.
    let (now, answers) = if applied.adopted.is_empty() {
        (report, answers)
    } else {
        adopt::report_answering(&scan, &state, sources, has_key, answers)
    };
    AdoptAnswer {
        adopt: Some(adopt_view(&now, &answers, voice)),
        flavor: Some(list::flavor_view(scan.flavor.clone(), Some(home), &[])),
        adopted: applied
            .adopted
            .iter()
            .map(|adopted| adopted.line())
            .collect(),
        messages,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
