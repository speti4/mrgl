//! The window's toolbox of a flavor (task 074): the addon sets with the characters a set can be applied to
//! (tasks 014 and 027), and the SavedVariables with their sizes and the cleanup of the orphans (task 025).
//! Plain functions over the core, which take the game root, the flavor and the mrgl home folder as
//! arguments, so `tests/toolbox.rs` runs them on a made-up flavor without a window; the commands of
//! [`crate::commands`] call them on a blocking thread.
//!
//! Both writes run the running-client guard of task 023 first (`client::ensure_client_not_running`) and
//! answer its refusal through the voice before anything else happens; the core's `sets::apply_set` and
//! `savedvars::clean_orphans` call the same guard again before their first write. Every line here is the
//! line `mrgl set apply` and `mrgl savedvars` print for the same outcome.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mrgl_core::backup;
use mrgl_core::client;
use mrgl_core::flavor::{self, Flavor};
use mrgl_core::savedvars::{self, SavedVariablesFile, Status};
use mrgl_core::scan;
use mrgl_core::sets::{self, AddonState, Applied, Character, Target};
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
/// plain lines: on an older install it is the login name.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CharacterView {
    pub name: String,
    pub realm: String,
    /// `<realm>/<character>`.
    pub label: String,
}

impl CharacterView {
    fn of(character: &Character) -> CharacterView {
        CharacterView {
            name: character.name.clone(),
            realm: character.realm.clone(),
            label: character.label(),
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
/// what an apply changed per character, the zip a cleanup wrote, and the messages in order, the warnings
/// first and the summary last.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolboxAnswer {
    pub toolbox: Option<ToolboxView>,
    pub changes: Vec<ChangeView>,
    pub zip: Option<String>,
    pub messages: Vec<Message>,
}

impl ToolboxAnswer {
    /// An answer of one message and no toolbox: nothing could be read or done.
    pub fn refused(message: Message) -> ToolboxAnswer {
        ToolboxAnswer {
            toolbox: None,
            changes: Vec::new(),
            zip: None,
            messages: vec![message],
        }
    }
}

/// The refusal of a toolbox write while another one runs.
pub fn busy_line() -> &'static str {
    "a set apply or a SavedVariables cleanup is running already; wait for it to end"
}

/// The refusal of an update run while a toolbox write goes.
pub fn toolbox_busy_line() -> &'static str {
    "a set apply or a SavedVariables cleanup is running; the update waits until it ends"
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
    ToolboxAnswer {
        toolbox: Some(view),
        changes: Vec::new(),
        zip: None,
        messages,
    }
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

    let characters = match sets::characters(flavor) {
        Ok(characters) => characters.iter().map(CharacterView::of).collect(),
        Err(error) => {
            messages.push(say(Kind::Error, error.to_string()));
            Vec::new()
        }
    };

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
                summary: listing_summary(&listing, &folder_name, voice),
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
        saved_variables,
        client,
    }
}

/// The summary of a listing, as `mrgl savedvars` writes it.
fn listing_summary(listing: &savedvars::Listing, folder_name: &str, voice: Voice) -> Message {
    let count = match listing.files.len() {
        1 => "1 SavedVariables file".to_string(),
        count => format!("{count} SavedVariables files"),
    };
    if listing.files.is_empty() {
        Message::new(
            voice,
            Kind::Nothing,
            format!("no SavedVariables files in the WTF folder of {folder_name}"),
        )
    } else if listing.orphans == 0 {
        Message::new(
            voice,
            Kind::Success,
            format!(
                "{count} of {folder_name}, {} in all, no orphans",
                size_text(listing.bytes)
            ),
        )
    } else {
        Message::new(
            voice,
            Kind::Warning,
            format!(
                "{count} of {folder_name}, {} in all; {} ({}) have no addon folder: the cleanup moves them into a backup zip",
                size_text(listing.bytes),
                list::count_noun(listing.orphans, "orphan"),
                size_text(listing.orphan_bytes)
            ),
        )
    }
}

/// The guard of task 023 before a `WTF` write: the refusal line while the client runs, or while the
/// process list cannot be taken (a write is refused rather than risked).
fn guard(flavor: &Flavor, voice: Voice) -> Result<(), Message> {
    client::ensure_client_not_running(flavor)
        .map_err(|error| Message::new(voice, Kind::Error, error.to_string()))
}

/// The target of the window's pick: every character without a name, else the one character
/// [`sets::find_character`] finds by name (and realm) among the flavor's characters, pinned to its exact
/// name and realm.
fn target_of(
    flavor: &Flavor,
    character: Option<&str>,
    realm: Option<&str>,
) -> mrgl_core::error::Result<Target> {
    let Some(name) = character.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(Target::EveryCharacter);
    };
    let realm = realm.map(str::trim).filter(|realm| !realm.is_empty());
    let characters = sets::characters(flavor)?;
    if characters.is_empty() {
        return Err(mrgl_core::Error::NoCharacters {
            path: backup::wtf_dir(flavor),
        });
    }
    let found = sets::find_character(&characters, name, realm, &flavor.id)?;
    Ok(Target::Character {
        name: found.name,
        realm: Some(found.realm),
    })
}

/// Applies a stored set to every character of the flavor (`character` is `None`) or to the one the
/// window picked, through `sets::apply_set`: the guard first, then the pick, then the apply, which writes
/// only the `AddOns.txt` files that change. The answer carries the toolbox as it is now, what changed per
/// character, and the not-installed warning and the summary of `mrgl set apply`.
pub fn apply(
    root: &Path,
    flavor_id: &str,
    set: &str,
    character: Option<&str>,
    realm: Option<&str>,
    home: Option<&Path>,
    voice: Voice,
) -> ToolboxAnswer {
    let flavor = match read(root, flavor_id, voice) {
        Ok(flavor) => flavor,
        Err(message) => return ToolboxAnswer::refused(message),
    };
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let refused = |message: Message| ToolboxAnswer {
        toolbox: view_after(&flavor, home, voice),
        changes: Vec::new(),
        zip: None,
        messages: vec![message],
    };
    if let Err(message) = guard(&flavor, voice) {
        return refused(message);
    }
    let Some(home) = home else {
        return refused(say(
            Kind::Error,
            format!("no set applied: {}", mrgl_core::Error::NoHome),
        ));
    };
    let target = match target_of(&flavor, character, realm) {
        Ok(target) => target,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };
    let path = sets::sets_path_at(home, &flavor.id);
    let applied = match sets::apply_set(&flavor, &path, set, &target) {
        Ok(applied) => applied,
        Err(error) => return refused(say(Kind::Error, error.to_string())),
    };

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
    messages.push(apply_summary(&applied, &target, &folder_name, voice));
    let changes = applied
        .characters
        .iter()
        .map(|change| ChangeView {
            character: CharacterView::of(&change.character),
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
        toolbox: view_after(&flavor, Some(home), voice),
        changes,
        zip: None,
        messages,
    }
}

/// The summary of an apply, as `mrgl set apply` writes it.
fn apply_summary(applied: &Applied, target: &Target, folder_name: &str, voice: Voice) -> Message {
    let characters = match (target, applied.characters.as_slice()) {
        (Target::Character { .. }, [one]) => one.character.label(),
        (_, characters) => list::count_noun(characters.len(), "character"),
    };
    let matched = match target {
        Target::EveryCharacter => format!("every character of {folder_name} ({characters})"),
        Target::Character { .. } => format!("{characters} of {folder_name}"),
    };
    match applied.changed() {
        0 => Message::new(
            voice,
            Kind::Nothing,
            format!(
                "set {} already matches {matched}, nothing written",
                applied.name
            ),
        ),
        1 => Message::new(
            voice,
            Kind::Success,
            format!(
                "applied set {} to {characters} of {folder_name}: 1 addon line changed",
                applied.name
            ),
        ),
        count => Message::new(
            voice,
            Kind::Success,
            format!(
                "applied set {} to {characters} of {folder_name}: {count} addon lines changed",
                applied.name
            ),
        ),
    }
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
        toolbox: view_after(&flavor, home, voice),
        changes: Vec::new(),
        zip,
        messages,
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
    let folder_name = flavor.folder_name();
    let mut messages: Vec<Message> = cleaned
        .warnings
        .iter()
        .map(|warning| say(Kind::Warning, warning.clone()))
        .collect();
    let zip = cleaned.zip.as_ref().map(|zip| zip.display().to_string());
    match &zip {
        None => messages.push(say(
            Kind::Nothing,
            format!("no orphans in the SavedVariables of {folder_name}, nothing moved"),
        )),
        Some(zip) => messages.push(say(
            Kind::Success,
            format!(
                "moved {} of {folder_name} ({}, {}) into {zip}; unzip it into the WTF folder to put them back",
                list::count_noun(cleaned.moved.len(), "orphan"),
                list::count_noun(cleaned.files, "file"),
                size_text(cleaned.bytes),
            ),
        )),
    }
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
