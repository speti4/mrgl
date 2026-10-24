//! Addon sets (task 014): named lists of addons per flavor, each addon `enabled` or `disabled`, kept in
//! `sets.json` under the flavor's folder of the mrgl home (`<home>\<flavor-id>\sets.json`, D-003) and
//! switched by rewriting the characters' `AddOns.txt` under `WTF`, never by deleting or moving a folder.
//!
//! `AddOns.txt` is the client's own file, one per character under
//! `WTF\Account\<account>\<realm>\<character>\`, one `Name: enabled` or `Name: disabled` line per addon. The
//! client reads it at login and writes it again at logout. [`AddonsTxt`] reads it as bytes and writes back
//! exactly what it read when nothing changes: the line endings (CRLF or LF, a missing last one), a byte
//! order mark, the order and every line it does not understand stay as they were, and a line it changes
//! keeps everything but the state word. A file that would not change is never written, so its time stays
//! too.
//!
//! An apply is the second of the three `WTF` writes of D-002 and D-003: it calls the guard of task 023
//! before its first write and refuses while the client runs (the client would write its own state over the
//! set at logout anyway). It changes exactly the addons the set lists and leaves every other line as it is;
//! an addon of the set that a file does not name gets a line at the end of that file. Every new content is
//! worked out before the first write, each file is written through a temporary file next to it, and a
//! failure part way puts the files already written back as they were.
//!
//! A save or an apply takes a [`Target`]: every character of the flavor, or one picked by name (and
//! realm) from the `WTF` account folders (task 027, [`find_character`]); an apply to one character leaves
//! every other character's file byte for byte. The `_in_account` functions (task 073) narrow the
//! characters to one account folder first, for a realm and name that sit under two accounts.
//!
//! An apply that changes a file keeps an undo record (task 073) under the flavor's folder of the mrgl
//! home, next to the sets file (`<home>\<flavor-id>\set-undo\`): the bytes of every `AddOns.txt` it
//! replaced, the names of the ones it created and what it wrote, for the last such apply only (an apply
//! that writes nothing leaves the record of the one before). [`undo_apply`] puts those files back byte for
//! byte and removes the ones the apply created, once, behind the same guard of task 023.
//!
//! [`edit_set`] enables, disables or drops one addon of a stored set in `sets.json` only (task 073); a save
//! keeps an addon that is no longer installed and names it ([`Saved::not_installed`]).
//!
//! `sets.json` is the user's own data, not a cache: a file that cannot be read is an error that names it,
//! and mrgl never overwrites it.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::backup::WTF_DIR;
use crate::client;
use crate::error::{Error, Result};
use crate::flavor::{Flavor, FlavorId};
use crate::home;
use crate::scan;
use crate::state;
use crate::voice::{Kind, count_noun};
use crate::wtf::{self, ACCOUNT_DIR, WtfFolder};
use crate::zip_entry;

/// The sets file's name under the flavor's folder of the mrgl home.
pub const SETS_FILE: &str = "sets.json";
/// The `format` value of a sets file.
pub const SETS_FORMAT: &str = "mrgl-sets";
/// The schema of `sets.json`; a file with another number is refused, never rewritten.
pub const SETS_SCHEMA: u32 = 1;
/// The client's per-character addon state file.
pub const ADDONS_TXT: &str = "AddOns.txt";
/// The longest set name, in characters.
pub const SET_NAME_MAX_CHARS: usize = 64;
/// The folder of the last apply's undo record, next to the sets file (task 073).
pub const UNDO_DIR: &str = "set-undo";
/// The undo record's file in [`UNDO_DIR`].
pub const UNDO_FILE: &str = "undo.json";
/// The `format` value of an undo record.
pub const UNDO_FORMAT: &str = "mrgl-set-undo";
/// The schema of an undo record; a record with another number is refused.
pub const UNDO_SCHEMA: u32 = 1;

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// An addon's state in a set and in `AddOns.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddonState {
    Enabled,
    Disabled,
}

impl AddonState {
    /// The word the client writes: `enabled` or `disabled`.
    pub const fn as_str(self) -> &'static str {
        match self {
            AddonState::Enabled => "enabled",
            AddonState::Disabled => "disabled",
        }
    }

    /// The state of a word as the client writes it, ASCII case-insensitive.
    pub fn parse(word: &[u8]) -> Option<AddonState> {
        if word.eq_ignore_ascii_case(b"enabled") {
            Some(AddonState::Enabled)
        } else if word.eq_ignore_ascii_case(b"disabled") {
            Some(AddonState::Disabled)
        } else {
            None
        }
    }
}

impl std::fmt::Display for AddonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A line's ending as the file has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    CrLf,
    Lf,
    /// The last line of a file that does not end in a line break.
    None,
}

impl Ending {
    const fn bytes(self) -> &'static [u8] {
        match self {
            Ending::CrLf => b"\r\n",
            Ending::Lf => b"\n",
            Ending::None => b"",
        }
    }
}

/// One line of the file: its bytes without the ending, and the ending.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    content: Vec<u8>,
    ending: Ending,
}

/// An addon line: where the name and the state word are in the line's bytes, and the state.
struct EntrySpans {
    name: Range<usize>,
    value: Range<usize>,
    state: AddonState,
}

/// A line's addon entry: `Name: enabled` or `Name: disabled`, spaces around either part allowed, a byte
/// order mark before the name skipped; any other line is no entry and stays as it is.
fn entry_spans(content: &[u8]) -> Option<EntrySpans> {
    let start = if content.starts_with(BOM) {
        BOM.len()
    } else {
        0
    };
    let colon = start + content[start..].iter().position(|&byte| byte == b':')?;
    let name = trimmed(content, start..colon);
    if name.is_empty() {
        return None;
    }
    let value = trimmed(content, colon + 1..content.len());
    let state = AddonState::parse(&content[value.clone()])?;
    Some(EntrySpans { name, value, state })
}

/// A range without the ASCII spaces and tabs at either end.
fn trimmed(content: &[u8], range: Range<usize>) -> Range<usize> {
    let blank = |byte: &u8| *byte == b' ' || *byte == b'\t';
    let mut start = range.start;
    let mut end = range.end;
    while start < end && blank(&content[start]) {
        start += 1;
    }
    while end > start && blank(&content[end - 1]) {
        end -= 1;
    }
    start..end
}

/// A character's `AddOns.txt` as bytes, line by line: [`AddonsTxt::to_bytes`] gives back exactly the
/// bytes [`AddonsTxt::parse`] was given until a state changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddonsTxt {
    lines: Vec<Line>,
}

impl AddonsTxt {
    /// Reads the file's bytes; nothing is refused, a line that is no addon entry is kept as it is.
    pub fn parse(bytes: &[u8]) -> AddonsTxt {
        let mut lines = Vec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            match rest.iter().position(|&byte| byte == b'\n') {
                Some(end) => {
                    let (content, ending) = if end > 0 && rest[end - 1] == b'\r' {
                        (&rest[..end - 1], Ending::CrLf)
                    } else {
                        (&rest[..end], Ending::Lf)
                    };
                    lines.push(Line {
                        content: content.to_vec(),
                        ending,
                    });
                    rest = &rest[end + 1..];
                }
                None => {
                    lines.push(Line {
                        content: rest.to_vec(),
                        ending: Ending::None,
                    });
                    rest = &[];
                }
            }
        }
        AddonsTxt { lines }
    }

    /// The file's bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in &self.lines {
            bytes.extend_from_slice(&line.content);
            bytes.extend_from_slice(line.ending.bytes());
        }
        bytes
    }

    /// The line ending a new line gets: the file's first one, CRLF (as the client writes it on Windows)
    /// when the file has none.
    fn ending(&self) -> Ending {
        self.lines
            .iter()
            .map(|line| line.ending)
            .find(|ending| *ending != Ending::None)
            .unwrap_or(Ending::CrLf)
    }

    /// Every addon entry in file order, the name as the file spells it.
    pub fn entries(&self) -> Vec<(String, AddonState)> {
        self.lines
            .iter()
            .filter_map(|line| {
                let spans = entry_spans(&line.content)?;
                let name = String::from_utf8_lossy(&line.content[spans.name]).into_owned();
                Some((name, spans.state))
            })
            .collect()
    }

    /// An addon's state as the file has it (ASCII case-insensitive name, the first line that names it).
    pub fn state_of(&self, name: &str) -> Option<AddonState> {
        self.lines.iter().find_map(|line| {
            let spans = entry_spans(&line.content)?;
            line.content[spans.name]
                .eq_ignore_ascii_case(name.as_bytes())
                .then_some(spans.state)
        })
    }

    /// Sets an addon's state: every line that names it with the other state gets the state word and keeps
    /// everything else (a line with the state already, in any case, stays as it is), and a file that does
    /// not name it gets a `Name: state` line at the end. `true` when the bytes changed.
    pub fn set_state(&mut self, name: &str, state: AddonState) -> bool {
        let mut named = false;
        let mut changed = false;
        for line in &mut self.lines {
            let Some(spans) = entry_spans(&line.content) else {
                continue;
            };
            if !line.content[spans.name].eq_ignore_ascii_case(name.as_bytes()) {
                continue;
            }
            named = true;
            if spans.state != state {
                line.content.splice(spans.value, state.as_str().bytes());
                changed = true;
            }
        }
        if named {
            return changed;
        }
        let ending = self.ending();
        if let Some(last) = self.lines.last_mut()
            && last.ending == Ending::None
        {
            last.ending = ending;
        }
        self.lines.push(Line {
            content: format!("{name}: {state}").into_bytes(),
            ending,
        });
        true
    }
}

/// A character as the `WTF` folder knows it: `WTF\Account\<account>\<realm>\<character>\`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Character {
    /// The account folder's name.
    pub account: String,
    /// The realm folder's name.
    pub realm: String,
    /// The character folder's name.
    pub name: String,
    /// The character's folder.
    pub folder: PathBuf,
}

impl Character {
    /// The character's `AddOns.txt` (it may not exist yet).
    pub fn addons_txt(&self) -> PathBuf {
        self.folder.join(ADDONS_TXT)
    }

    /// `<realm>/<character>`, for the plain lines. The account folder's name is left out: on an older
    /// install it is the login name, and a pasted output should not carry it (`--json` has it).
    pub fn label(&self) -> String {
        format!("{}/{}", self.realm, self.name)
    }
}

/// The characters of a flavor's `WTF` folder.
pub fn characters(flavor: &Flavor) -> Result<Vec<Character>> {
    characters_in(&flavor.folder.join(WTF_DIR))
}

/// The characters under a `WTF` folder, sorted by account, realm and name: every folder two levels under
/// an account folder (`<realm>\<character>`), the account's `SavedVariables` never a realm ([`wtf::walk`]).
/// Files, links and junctions are passed over; a missing `WTF` or `Account` folder is no character, never
/// an error.
pub fn characters_in(wtf: &Path) -> Result<Vec<Character>> {
    let mut characters = Vec::new();
    // A link is passed over without a word here: a set has no warnings to carry it.
    let mut links = Vec::new();
    wtf::walk(wtf, &mut links, |folder, _| {
        if let WtfFolder::Character {
            account,
            realm,
            name,
            folder,
        } = folder
        {
            characters.push(Character {
                account: account.to_string(),
                realm: realm.to_string(),
                name: name.to_string(),
                folder: folder.to_path_buf(),
            });
        }
        Ok(())
    })?;
    characters.sort();
    Ok(characters)
}

/// The characters a save reads or an apply writes (task 027).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Target {
    /// Every character of the flavor (task 014).
    #[default]
    EveryCharacter,
    /// One character by its folder's name, case-insensitive; `realm` narrows a name that more than one
    /// realm has.
    Character { name: String, realm: Option<String> },
}

/// The characters of a target: every character of the flavor, or the one [`find_character`] picks.
/// No character at all is [`Error::NoCharacters`].
pub fn targets(flavor: &Flavor, target: &Target) -> Result<Vec<Character>> {
    targets_in_account(flavor, target, None)
}

/// The characters of a target within one account folder (`--account`, task 073; compared ignoring
/// case), or within every account without one. An account with no character is
/// [`Error::AccountNotFound`] with the accounts listed.
pub fn targets_in_account(
    flavor: &Flavor,
    target: &Target,
    account: Option<&str>,
) -> Result<Vec<Character>> {
    let characters = characters(flavor)?;
    if characters.is_empty() {
        return Err(Error::NoCharacters {
            path: flavor.folder.join(WTF_DIR),
        });
    }
    let characters = in_account(&characters, account, &flavor.id)?;
    match target {
        Target::EveryCharacter => Ok(characters),
        Target::Character { name, realm } => {
            find_character(&characters, name, realm.as_deref(), &flavor.id).map(|found| vec![found])
        }
    }
}

/// [`find_character`] within one account folder (task 073): the same name on the same realm under two
/// accounts is told apart by the account.
pub fn find_character_in_account(
    characters: &[Character],
    name: &str,
    realm: Option<&str>,
    account: Option<&str>,
    flavor: &FlavorId,
) -> Result<Character> {
    let characters = in_account(characters, account, flavor)?;
    find_character(&characters, name, realm, flavor)
}

/// The characters under one account folder, its name compared ignoring case; all of them without an
/// account.
fn in_account(
    characters: &[Character],
    account: Option<&str>,
    flavor: &FlavorId,
) -> Result<Vec<Character>> {
    let Some(account) = account else {
        return Ok(characters.to_vec());
    };
    let wanted = account.trim().to_lowercase();
    let found: Vec<Character> = characters
        .iter()
        .filter(|character| character.account.to_lowercase() == wanted)
        .cloned()
        .collect();
    if found.is_empty() {
        let mut known: Vec<&str> = characters
            .iter()
            .map(|character| character.account.as_str())
            .collect();
        known.dedup();
        return Err(Error::AccountNotFound {
            account: account.to_string(),
            flavor: flavor_label(flavor),
            known: known.join(", "),
        });
    }
    Ok(found)
}

/// What tells apart the characters an [`Error::AmbiguousCharacter`] lists, and its hint: the realm (`name
/// one with --realm`), or the account, when one realm and name sit under two accounts (task 073). The
/// account's hint names no flag (task 091): the window picks the account in its own way, and the command
/// line adds its `--account` next to the core's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterPick {
    Realm,
    Account,
}

impl std::fmt::Display for CharacterPick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CharacterPick::Realm => "name one with --realm",
            CharacterPick::Account => {
                "the same realm is under more than one account: pick the account too"
            }
        })
    }
}

/// One character by name (and realm, when given), both compared ignoring case. A name no character has
/// is [`Error::CharacterNotFound`] with the characters listed; a name more than one character has is
/// [`Error::AmbiguousCharacter`] with their realms listed (with the account too, when one realm is under
/// two accounts).
pub fn find_character(
    characters: &[Character],
    name: &str,
    realm: Option<&str>,
    flavor: &FlavorId,
) -> Result<Character> {
    let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
    let matches: Vec<&Character> = characters
        .iter()
        .filter(|character| same(&character.name, name))
        .filter(|character| realm.is_none_or(|realm| same(&character.realm, realm)))
        .collect();
    match matches.as_slice() {
        [found] => Ok((*found).clone()),
        [] => {
            let known: Vec<String> = characters
                .iter()
                .map(|character| format!("{} ({})", character.name, character.realm))
                .collect();
            Err(Error::CharacterNotFound {
                name: match realm {
                    Some(realm) => format!("{name} on {realm}"),
                    None => name.to_string(),
                },
                flavor: flavor_label(flavor),
                known: known.join(", "),
            })
        }
        several => {
            let mut realms: Vec<&str> = several
                .iter()
                .map(|character| character.realm.as_str())
                .collect();
            realms.sort_unstable();
            realms.dedup();
            let (candidates, by) = if realms.len() == several.len() {
                (realms.join(", "), CharacterPick::Realm)
            } else {
                let labels: Vec<String> = several
                    .iter()
                    .map(|character| format!("{} ({})", character.realm, character.account))
                    .collect();
                (labels.join(", "), CharacterPick::Account)
            };
            Err(Error::AmbiguousCharacter {
                name: name.to_string(),
                count: several.len(),
                flavor: flavor_label(flavor),
                candidates,
                by,
            })
        }
    }
}

/// One named set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonSet {
    /// When it was saved, RFC 3339 in UTC.
    pub saved_at: String,
    /// Every addon of the set with the state an apply gives it, by name.
    pub addons: BTreeMap<String, AddonState>,
}

impl AddonSet {
    /// The addons the set enables.
    pub fn enabled(&self) -> usize {
        self.count(AddonState::Enabled)
    }

    /// The addons the set disables.
    pub fn disabled(&self) -> usize {
        self.count(AddonState::Disabled)
    }

    /// `3 addons (2 enabled, 1 disabled)`, for a plain line.
    pub fn counted(&self) -> String {
        format!(
            "{} ({} enabled, {} disabled)",
            count_noun(self.addons.len(), "addon"),
            self.enabled(),
            self.disabled()
        )
    }

    fn count(&self, state: AddonState) -> usize {
        self.addons
            .values()
            .filter(|value| **value == state)
            .count()
    }
}

/// The sets file of a flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetsFile {
    /// Always [`SETS_FORMAT`].
    pub format: String,
    /// Always [`SETS_SCHEMA`].
    pub schema: u32,
    /// The flavor id the sets belong to.
    pub flavor: String,
    /// The sets by name.
    #[serde(default)]
    pub sets: BTreeMap<String, AddonSet>,
}

impl SetsFile {
    /// No set yet.
    pub fn new(flavor: &FlavorId) -> SetsFile {
        SetsFile {
            format: SETS_FORMAT.to_string(),
            schema: SETS_SCHEMA,
            flavor: flavor.as_str().to_string(),
            sets: BTreeMap::new(),
        }
    }

    /// A set by name: the exact name, else the one name that matches ignoring ASCII case.
    pub fn find(&self, name: &str) -> Option<(&str, &AddonSet)> {
        if let Some((key, set)) = self.sets.get_key_value(name) {
            return Some((key.as_str(), set));
        }
        let mut matches = self
            .sets
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name));
        let found = matches.next()?;
        matches
            .next()
            .is_none()
            .then_some((found.0.as_str(), found.1))
    }

    /// The note under a flavor's set list in the voice's terms (task 091: `mrgl set list` and the window's
    /// toolbox share it): how many sets there are and in which file, or that there is none yet with the
    /// command that saves one.
    pub fn summary(&self, path: &Path, folder_name: &str) -> (Kind, String) {
        match self.sets.len() {
            0 => (
                Kind::Nothing,
                format!("no sets of {folder_name} yet: `mrgl set save <name>` saves one"),
            ),
            count => (
                Kind::Success,
                format!(
                    "{} of {folder_name} in {}",
                    count_noun(count, "set"),
                    path.display()
                ),
            ),
        }
    }

    /// The error of a name no set has, listing the saved ones.
    fn not_found(&self, name: &str, flavor: &FlavorId) -> Error {
        let known = if self.sets.is_empty() {
            "no set is saved yet: `mrgl set save <name>` saves one".to_string()
        } else {
            let names: Vec<&str> = self.sets.keys().map(String::as_str).collect();
            format!("the saved sets: {}", names.join(", "))
        };
        Error::SetNotFound {
            name: name.to_string(),
            flavor: flavor_label(flavor),
            known,
        }
    }
}

/// A flavor's sets file under a given home folder: `<home>\<flavor-id>\sets.json`, next to the sidecar.
pub fn sets_path_at(home_dir: &Path, flavor: &FlavorId) -> PathBuf {
    state::FlavorHome::at(home_dir, flavor).dir.join(SETS_FILE)
}

/// A flavor's sets file under this process's home (`MRGL_HOME`, else `LOCALAPPDATA\mrgl`).
pub fn sets_path(flavor: &FlavorId) -> Result<PathBuf> {
    Ok(sets_path_at(&home::home_dir()?, flavor))
}

/// Reads a sets file: a missing one is no set; one that is not a sets file of this flavor and schema,
/// or that names an addon no `AddOns.txt` line could carry, is [`Error::SetsUnreadable`].
pub fn read_sets_at(path: &Path, flavor: &FlavorId) -> Result<SetsFile> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(SetsFile::new(flavor));
        }
        Err(source) => {
            return Err(Error::Io {
                context: "reading the sets file",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let unreadable = |reason: String| Error::SetsUnreadable {
        path: path.to_path_buf(),
        reason,
    };
    let file: SetsFile =
        serde_json::from_str(&text).map_err(|reason| unreadable(reason.to_string()))?;
    if file.format != SETS_FORMAT {
        return Err(unreadable(format!(
            "format {} instead of {SETS_FORMAT}",
            file.format
        )));
    }
    if file.schema != SETS_SCHEMA {
        return Err(unreadable(format!(
            "schema {} instead of {SETS_SCHEMA}",
            file.schema
        )));
    }
    if file.flavor != flavor.as_str() {
        return Err(unreadable(format!(
            "flavor {} instead of {flavor}",
            file.flavor
        )));
    }
    for (name, set) in &file.sets {
        if let Some(addon) = set.addons.keys().find(|addon| !is_addon_name(addon)) {
            return Err(unreadable(format!(
                "set {name} names `{}`, which is not an addon name",
                addon.escape_debug()
            )));
        }
    }
    Ok(file)
}

/// Writes a sets file, creating its folder; through a temporary file next to it, so a crash mid-write
/// leaves the old file whole.
pub fn write_sets_at(path: &Path, sets: &SetsFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            context: "creating the flavor's folder under the mrgl home",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let io = |source| Error::Io {
        context: "writing the sets file",
        path: path.to_path_buf(),
        source,
    };
    let text = serde_json::to_string_pretty(sets).map_err(|source| io(io::Error::other(source)))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text + "\n").map_err(io)?;
    fs::rename(&temporary, path).map_err(io)
}

/// A name `AddOns.txt` can carry on one line: not empty, no colon, no control character, no path
/// separator, no space at either end.
fn is_addon_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '/' | '\\'))
}

/// A set name: trimmed, not empty, at most [`SET_NAME_MAX_CHARS`] characters, no control character.
fn checked_set_name(name: &str) -> Result<String> {
    let refuse = |reason: &str| Error::SetName {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(refuse("empty"));
    }
    if trimmed.chars().count() > SET_NAME_MAX_CHARS {
        return Err(refuse("longer than 64 characters"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(refuse("a control character in it"));
    }
    Ok(trimmed.to_string())
}

fn flavor_label(id: &FlavorId) -> String {
    id.friendly_name().unwrap_or(id.as_str()).to_string()
}

/// Reads a character's `AddOns.txt`; `None` when the character has none yet.
fn read_addons_txt(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            context: "reading AddOns.txt",
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// A set just saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Saved {
    /// The set's name as stored.
    pub name: String,
    pub set: AddonSet,
    /// A set of that name was there before and is replaced.
    pub replaced: bool,
    /// The characters whose `AddOns.txt` was read (a character without one included).
    pub characters: Vec<Character>,
    /// Addons left out because the characters disagree on them, by name.
    pub conflicts: Vec<String>,
    /// Addons of the set with no folder under `Interface\AddOns`, by name: kept in the set all the same
    /// (task 073), a set is the user's data.
    pub not_installed: Vec<String>,
    /// Plain lines about what was passed over.
    pub warnings: Vec<String>,
}

/// Saves a set from the `AddOns.txt` files of the target's characters (every character of the flavor, or
/// the one [`Target::Character`] names, task 027): every addon a file names, with its state; an addon
/// the characters disagree on is left out (an apply then leaves it alone) and named in
/// [`Saved::conflicts`]; an addon that is not installed is kept and named in [`Saved::not_installed`]
/// (task 073). A set of the same name is replaced. Reads `WTF` only, so it goes on while the client runs.
pub fn save_set(flavor: &Flavor, sets_path: &Path, name: &str, target: &Target) -> Result<Saved> {
    save_set_in_account(flavor, sets_path, name, target, None)
}

/// [`save_set`] from the characters of one account folder (`--account`, task 073).
pub fn save_set_in_account(
    flavor: &Flavor,
    sets_path: &Path,
    name: &str,
    target: &Target,
    account: Option<&str>,
) -> Result<Saved> {
    let name = checked_set_name(name)?;
    let mut sets = read_sets_at(sets_path, &flavor.id)?;
    let characters = targets_in_account(flavor, target, account)?;
    let mut states: BTreeMap<String, (String, Option<AddonState>)> = BTreeMap::new();
    let mut warnings = Vec::new();
    for character in &characters {
        let Some(bytes) = read_addons_txt(&character.addons_txt())? else {
            continue;
        };
        for (addon, state) in AddonsTxt::parse(&bytes).entries() {
            if !is_addon_name(&addon) {
                warnings.push(format!(
                    "left out of set {name}: `{}` in {} is not an addon name",
                    addon.escape_debug(),
                    character.addons_txt().display()
                ));
                continue;
            }
            states
                .entry(addon.to_ascii_lowercase())
                .and_modify(|(_, known)| {
                    if *known != Some(state) {
                        *known = None;
                    }
                })
                .or_insert((addon, Some(state)));
        }
    }
    let mut addons = BTreeMap::new();
    let mut conflicts = Vec::new();
    for (addon, state) in states.into_values() {
        match state {
            Some(state) => {
                addons.insert(addon, state);
            }
            None => conflicts.push(addon),
        }
    }
    if addons.is_empty() {
        return Err(Error::NothingToSave {
            flavor: flavor.folder_name(),
        });
    }
    let set = AddonSet {
        saved_at: state::now_rfc3339(),
        addons,
    };
    let not_installed = not_installed(flavor, set.addons.keys());
    let previous = sets.find(&name).map(|(key, _)| key.to_string());
    let replaced = previous.is_some();
    if let Some(previous) = previous {
        sets.sets.remove(&previous);
    }
    sets.sets.insert(name.clone(), set.clone());
    write_sets_at(sets_path, &sets)?;
    Ok(Saved {
        name,
        set,
        replaced,
        characters,
        conflicts,
        not_installed,
        warnings,
    })
}

/// The addons of a list with no folder under the flavor's `Interface\AddOns`, by name.
fn not_installed<'a>(flavor: &Flavor, addons: impl Iterator<Item = &'a String>) -> Vec<String> {
    let addons_dir = scan::addons_dir(&flavor.folder);
    addons
        .filter(|addon| !addons_dir.join(addon).is_dir())
        .cloned()
        .collect()
}

/// What an apply changed in one character's `AddOns.txt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterChange {
    pub character: Character,
    /// Every addon whose line changed or was added, with its new state, in the set's order.
    pub changed: Vec<(String, AddonState)>,
    /// The character had no `AddOns.txt`, and the apply wrote one.
    pub created: bool,
}

/// An apply that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Applied {
    /// The set's name as stored.
    pub name: String,
    pub set: AddonSet,
    /// Every character the set was applied to, the unchanged ones included (with nothing in `changed`).
    pub characters: Vec<CharacterChange>,
    /// Addons of the set with no folder under `Interface\AddOns`: their lines are written all the same.
    pub not_installed: Vec<String>,
    /// Plain lines: a client that started during the apply (task 124).
    pub warnings: Vec<String>,
}

impl Applied {
    /// The lines changed or added over every character.
    pub fn changed(&self) -> usize {
        self.characters
            .iter()
            .map(|character| character.changed.len())
            .sum()
    }

    /// The warning about the addons of the set that have no folder under `Interface\AddOns`, whose lines
    /// an apply writes all the same; `None` when every one is installed (task 091: `mrgl set apply` and the
    /// window's toolbox share it).
    pub fn not_installed_warning(&self, folder_name: &str) -> Option<String> {
        (!self.not_installed.is_empty()).then(|| {
            format!(
                "set {} names addons that are not installed in {folder_name}, their lines are written all the same: {}",
                self.name,
                self.not_installed.join(", ")
            )
        })
    }

    /// The summary of an apply in the voice's terms (task 086: `mrgl set apply` and the window's toolbox
    /// share it): the set, the characters (the one character's `<realm>/<name>` when the target named it,
    /// else their count) and the lines changed; nothing to say when every file matched already.
    pub fn summary(&self, target: &Target, folder_name: &str) -> (Kind, String) {
        let characters = match (target, self.characters.as_slice()) {
            (Target::Character { .. }, [one]) => one.character.label(),
            (_, characters) => count_noun(characters.len(), "character"),
        };
        let name = &self.name;
        match self.changed() {
            0 => {
                let matched = match target {
                    Target::EveryCharacter => {
                        format!("every character of {folder_name} ({characters})")
                    }
                    Target::Character { .. } => format!("{characters} of {folder_name}"),
                };
                (
                    Kind::Nothing,
                    format!("set {name} already matches {matched}, nothing written"),
                )
            }
            count => (
                Kind::Success,
                format!(
                    "applied set {name} to {characters} of {folder_name}: {} changed",
                    count_noun(count, "addon line")
                ),
            ),
        }
    }
}

/// Applies a set to the target's characters: every character of the flavor, or the one
/// [`Target::Character`] names (task 027; every other character's file stays byte for byte). In this
/// order: the set read (an unknown name is [`Error::SetNotFound`]), the characters listed (none is
/// [`Error::NoCharacters`], a name [`find_character`] cannot pin down its error), the guard of task
/// 023 (a running client refuses the apply before anything is written), every new `AddOns.txt` worked
/// out, the undo record of the files that change put in place (task 073, [`undo_apply`]; under
/// [`UNDO_DIR`] next to `sets_path`, replacing the last one; an apply that changes nothing keeps it), then
/// only the files that change written, each through a temporary file. A failed write puts the files
/// already written back and is [`Error::SetApplyFailed`]; the record before it is back too when they are.
/// Once the files are written the running-client check looks again (task 124): a client that started
/// during the apply is a warning in [`Applied::warnings`].
pub fn apply_set(
    flavor: &Flavor,
    sets_path: &Path,
    name: &str,
    target: &Target,
) -> Result<Applied> {
    apply_set_in_account(flavor, sets_path, name, target, None)
}

/// [`apply_set`] to the characters of one account folder (`--account`, task 073).
pub fn apply_set_in_account(
    flavor: &Flavor,
    sets_path: &Path,
    name: &str,
    target: &Target,
    account: Option<&str>,
) -> Result<Applied> {
    apply_set_among(
        &client::SystemProcesses,
        flavor,
        sets_path,
        name,
        target,
        account,
    )
}

/// [`apply_set_in_account`] against the process list it is given, for the guard before the apply and the
/// second look after it.
pub fn apply_set_among(
    processes: &dyn client::ProcessList,
    flavor: &Flavor,
    sets_path: &Path,
    name: &str,
    target: &Target,
    account: Option<&str>,
) -> Result<Applied> {
    let sets = read_sets_at(sets_path, &flavor.id)?;
    let (name, set) = sets
        .find(name)
        .map(|(key, set)| (key.to_string(), set.clone()))
        .ok_or_else(|| sets.not_found(name, &flavor.id))?;
    let characters = targets_in_account(flavor, target, account)?;
    client::ensure_client_not_running_among(processes, flavor)?;

    let mut plans = Vec::new();
    for character in characters {
        let path = character.addons_txt();
        let before = read_addons_txt(&path)?;
        let mut file = AddonsTxt::parse(before.as_deref().unwrap_or_default());
        let mut changed = Vec::new();
        for (addon, state) in &set.addons {
            if file.set_state(addon, *state) {
                changed.push((addon.clone(), *state));
            }
        }
        let after = file.to_bytes();
        plans.push(Plan {
            path,
            before,
            after,
            change: CharacterChange {
                character,
                created: false,
                changed,
            },
        });
    }
    let not_installed = not_installed(flavor, set.addons.keys());
    let applied = |plans: Vec<Plan>| Applied {
        name: name.clone(),
        set: set.clone(),
        characters: plans.into_iter().map(|plan| plan.change).collect(),
        not_installed,
        warnings: Vec::new(),
    };
    if plans.iter().all(|plan| plan.change.changed.is_empty()) {
        // Nothing to write: the record of the apply before stays.
        return Ok(applied(plans));
    }
    let record = UndoSwap::put_in_place(&undo_dir_of(sets_path), &flavor.id, &name, &plans)?;
    if let Err(error) = write_plans(&mut plans, &name) {
        // Files back as they were: the record before this apply is the right one again. Files left
        // half written: this apply's record is the way back.
        if matches!(error, Error::SetApplyFailed { reverted: true, .. }) {
            record.roll_back();
        } else {
            record.keep(); // coverage: os — a put-back failed right after the path was written
        }
        return Err(error);
    }
    record.keep();
    let mut applied = applied(plans);
    applied.warnings.extend(client::started_during_among(
        processes,
        flavor,
        "the set apply",
    ));
    Ok(applied)
}

/// One character's file as an apply means to write it.
struct Plan {
    path: PathBuf,
    /// The bytes before; `None` when there was no file.
    before: Option<Vec<u8>>,
    after: Vec<u8>,
    change: CharacterChange,
}

/// Writes every plan whose bytes change; on a failure, puts back the files written so far (the earlier
/// bytes, or no file where there was none).
fn write_plans(plans: &mut [Plan], set: &str) -> Result<()> {
    let mut written: Vec<usize> = Vec::new();
    for (index, plan) in plans.iter_mut().enumerate() {
        if plan.change.changed.is_empty() {
            continue;
        }
        if let Err(error) = write_file(&plan.path, &plan.after) {
            let reverted = written.iter().all(|&done| {
                let done = &plans[done];
                match &done.before {
                    Some(bytes) => write_file(&done.path, bytes).is_ok(),
                    None => fs::remove_file(&done.path).is_ok(),
                }
            });
            return Err(Error::SetApplyFailed {
                set: set.to_string(),
                reason: format!("{}: {error}", plans[index].path.display()),
                reverted,
            });
        }
        plan.change.created = plan.before.is_none();
        written.push(index);
    }
    Ok(())
}

/// Writes a file through a temporary one next to it, renamed over it.
fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("txt.mrgl-tmp");
    let result = fs::write(&temporary, bytes).and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// The undo record's folder next to a sets file: `<home>\<flavor-id>\set-undo\` (task 073).
pub fn undo_dir_of(sets_path: &Path) -> PathBuf {
    sets_path.with_file_name(UNDO_DIR)
}

/// The undo record of the last apply that changed a file (task 073), `undo.json` in [`UNDO_DIR`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoRecord {
    /// Always [`UNDO_FORMAT`].
    pub format: String,
    /// Always [`UNDO_SCHEMA`].
    pub schema: u32,
    /// The flavor id the apply wrote to.
    pub flavor: String,
    /// The set applied, its name as stored.
    pub set: String,
    /// When it was applied, RFC 3339 in UTC.
    pub applied_at: String,
    /// Every `AddOns.txt` the apply wrote.
    pub files: Vec<UndoFile>,
}

/// One `AddOns.txt` an apply wrote, in an [`UndoRecord`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndoFile {
    /// The account folder's name.
    pub account: String,
    /// The realm folder's name.
    pub realm: String,
    /// The character folder's name.
    pub character: String,
    /// The file in the record's folder with the bytes the apply replaced; `None` when the apply created
    /// the file.
    pub kept: Option<String>,
    /// The file in the record's folder with the bytes the apply wrote, to tell a change since.
    pub written: String,
}

/// An undo record moved into place before an apply writes, with the record before it set aside until the
/// apply is through.
struct UndoSwap {
    dir: PathBuf,
    aside: PathBuf,
}

impl UndoSwap {
    /// Writes the record of the plans that change into a fresh folder next to `dir`, sets the record
    /// there aside and moves the new one in. Nothing under `WTF` is written yet, so a failure here
    /// refuses the apply with every file as it was.
    fn put_in_place(dir: &Path, flavor: &FlavorId, set: &str, plans: &[Plan]) -> Result<UndoSwap> {
        let fresh = dir.with_file_name(format!("{UNDO_DIR}.new"));
        let aside = dir.with_file_name(format!("{UNDO_DIR}.old"));
        let failed = |path: &Path, source: io::Error| Error::Io {
            context: "keeping the undo record of the apply",
            path: path.to_path_buf(),
            source,
        };
        remove_dir_if_any(&fresh).map_err(|source| failed(&fresh, source))?;
        remove_dir_if_any(&aside).map_err(|source| failed(&aside, source))?;
        // coverage: os-begin — a write into the fresh folder the code just made
        if let Err(source) = write_undo_folder(&fresh, flavor, set, plans) {
            let _ = fs::remove_dir_all(&fresh);
            return Err(failed(&fresh, source));
            // coverage: os-end
        }
        let had_one = dir.exists();
        if had_one && let Err(source) = fs::rename(dir, &aside) {
            let _ = fs::remove_dir_all(&fresh);
            return Err(failed(dir, source));
        }
        if let Err(source) = fs::rename(&fresh, dir) {
            // coverage: os-begin — a rename back of the path moved aside a moment ago
            if had_one {
                let _ = fs::rename(&aside, dir);
                // coverage: os-end
            }
            let _ = fs::remove_dir_all(&fresh);
            return Err(failed(dir, source));
        }
        Ok(UndoSwap {
            dir: dir.to_path_buf(),
            aside,
        })
    }

    /// The apply went through: the record before it goes.
    fn keep(self) {
        let _ = fs::remove_dir_all(&self.aside);
    }

    /// The apply wrote nothing in the end: the record before it is put back.
    fn roll_back(self) {
        let _ = fs::remove_dir_all(&self.dir);
        if self.aside.exists() {
            let _ = fs::rename(&self.aside, &self.dir);
        }
    }
}

fn remove_dir_if_any(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn remove_file_if_any(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The record of the plans that change, written into `folder`: per file `<n>-before.txt` (the bytes it
/// replaces, when there were any) and `<n>-after.txt` (the bytes it writes), and `undo.json`.
fn write_undo_folder(
    folder: &Path,
    flavor: &FlavorId,
    set: &str,
    plans: &[Plan],
) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let mut files = Vec::new();
    let changing = plans.iter().filter(|plan| !plan.change.changed.is_empty());
    for (index, plan) in changing.enumerate() {
        let number = index + 1;
        let kept = match &plan.before {
            Some(bytes) => {
                let kept = format!("{number}-before.txt");
                fs::write(folder.join(&kept), bytes)?;
                Some(kept)
            }
            None => None,
        };
        let written = format!("{number}-after.txt");
        fs::write(folder.join(&written), &plan.after)?;
        let character = &plan.change.character;
        files.push(UndoFile {
            account: character.account.clone(),
            realm: character.realm.clone(),
            character: character.name.clone(),
            kept,
            written,
        });
    }
    let record = UndoRecord {
        format: UNDO_FORMAT.to_string(),
        schema: UNDO_SCHEMA,
        flavor: flavor.as_str().to_string(),
        set: set.to_string(),
        applied_at: state::now_rfc3339(),
        files,
    };
    let text = serde_json::to_string_pretty(&record).map_err(io::Error::other)?;
    fs::write(folder.join(UNDO_FILE), text + "\n")
}

/// One file of a read record: its character, the bytes to put back (`None`: remove the file) and the
/// bytes the apply wrote.
struct UndoEntry {
    character: Character,
    then: Option<Vec<u8>>,
    written: Vec<u8>,
}

/// Reads the undo record of a flavor: `None` when there is none; a record that is not one of this flavor
/// and schema, names a path, or misses a kept file is [`Error::SetUndoUnreadable`].
fn read_undo(dir: &Path, flavor: &Flavor) -> Result<Option<(UndoRecord, Vec<UndoEntry>)>> {
    let path = dir.join(UNDO_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::Io {
                context: "reading the undo record",
                path,
                source,
            });
        }
    };
    let unreadable = |reason: String| Error::SetUndoUnreadable {
        path: dir.to_path_buf(),
        reason,
    };
    let record: UndoRecord =
        serde_json::from_str(&text).map_err(|reason| unreadable(reason.to_string()))?;
    if record.format != UNDO_FORMAT || record.schema != UNDO_SCHEMA {
        return Err(unreadable(format!(
            "format {} schema {} instead of {UNDO_FORMAT} schema {UNDO_SCHEMA}",
            record.format, record.schema
        )));
    }
    if record.flavor != flavor.id.as_str() {
        return Err(unreadable(format!(
            "flavor {} instead of {}",
            record.flavor, flavor.id
        )));
    }
    let accounts = flavor.folder.join(WTF_DIR).join(ACCOUNT_DIR);
    let mut entries = Vec::new();
    for file in &record.files {
        let names = [&file.account, &file.realm, &file.character, &file.written];
        if let Some(name) = names
            .into_iter()
            .chain(file.kept.as_ref())
            // A name the record may carry for a folder or a file: one folder name (task 123).
            .find(|name| !zip_entry::is_folder_name(name))
        {
            return Err(unreadable(format!(
                "`{}` is not a plain folder or file name",
                name.escape_debug()
            )));
        }
        let read = |name: &str| {
            fs::read(dir.join(name)).map_err(|source| unreadable(format!("{name}: {source}")))
        };
        entries.push(UndoEntry {
            character: Character {
                account: file.account.clone(),
                realm: file.realm.clone(),
                name: file.character.clone(),
                folder: accounts
                    .join(&file.account)
                    .join(&file.realm)
                    .join(&file.character),
            },
            then: file.kept.as_deref().map(read).transpose()?,
            written: read(&file.written)?,
        });
    }
    Ok(Some((record, entries)))
}

/// An undo that finished (task 073).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Undone {
    /// The set whose apply was undone.
    pub set: String,
    /// When that apply ran, RFC 3339 in UTC.
    pub applied_at: String,
    /// The characters whose `AddOns.txt` is back as it was before the apply.
    pub restored: Vec<Character>,
    /// The characters whose `AddOns.txt` the apply had created, removed again.
    pub removed: Vec<Character>,
    /// Plain lines: a file changed since the apply, a character folder gone, a record that stays.
    pub warnings: Vec<String>,
}

impl Undone {
    /// The summary of an undo in the voice's terms (task 085: `mrgl set undo` and the window's toolbox
    /// share it, as task 086 has the apply's): the set, when it was applied, the files put back and the
    /// ones removed. A warning when the undo has warnings of its own, a success otherwise.
    pub fn summary(&self, folder_name: &str) -> (Kind, String) {
        let mut summary = format!(
            "undid the apply of set {} to {folder_name} ({}): {} put back",
            self.set,
            self.applied_at,
            count_addons_txt(self.restored.len())
        );
        if !self.removed.is_empty() {
            summary.push_str(&format!(
                ", {} the apply created removed",
                count_addons_txt(self.removed.len())
            ));
        }
        let kind = if self.warnings.is_empty() {
            Kind::Success
        } else {
            Kind::Warning
        };
        (kind, summary)
    }
}

/// `1 AddOns.txt`, `2 AddOns.txt files`.
fn count_addons_txt(count: usize) -> String {
    if count == 1 {
        format!("1 {ADDONS_TXT}")
    } else {
        format!("{count} {ADDONS_TXT} files")
    }
}

/// The record of the last apply that changed a file, as [`undo_apply`] would read it, without writing
/// anything (task 085: the window names what an undo puts back before it is pressed). `None` when there is
/// nothing to undo; a record [`undo_apply`] would refuse is its error ([`Error::SetUndoUnreadable`]).
pub fn undo_record(flavor: &Flavor, sets_path: &Path) -> Result<Option<UndoRecord>> {
    Ok(read_undo(&undo_dir_of(sets_path), flavor)?.map(|(record, _)| record))
}

/// Undoes the last apply that changed a file (task 073): every `AddOns.txt` it replaced is put back byte
/// for byte and every one it created is removed, then the record is forgotten, so a second undo has
/// nothing to do ([`Error::NothingToUndo`]). In this order: the record read (every kept file with it), the
/// guard of task 023 (a running client refuses the undo before anything is written), then the files
/// written, each through a temporary file. A file that changed since the apply (the client writes it at
/// logout) is put back all the same, with a warning; a character folder that is gone is passed over with
/// one. A failed write puts the files already written back and is [`Error::SetUndoFailed`]. Once the
/// files are written the running-client check looks again (task 124): a client that started during the
/// undo is a warning among [`Undone::warnings`].
pub fn undo_apply(flavor: &Flavor, sets_path: &Path) -> Result<Undone> {
    undo_apply_among(&client::SystemProcesses, flavor, sets_path)
}

/// [`undo_apply`] against the process list it is given, for the guard before the undo and the second
/// look after it.
pub fn undo_apply_among(
    processes: &dyn client::ProcessList,
    flavor: &Flavor,
    sets_path: &Path,
) -> Result<Undone> {
    let dir = undo_dir_of(sets_path);
    let Some((record, entries)) = read_undo(&dir, flavor)? else {
        return Err(Error::NothingToUndo {
            flavor: flavor.folder_name(),
        });
    };
    client::ensure_client_not_running_among(processes, flavor)?;

    let mut warnings = Vec::new();
    let mut steps = Vec::new();
    for entry in entries {
        let label = entry.character.label();
        if !entry.character.folder.is_dir() {
            warnings.push(format!(
                "{label}: the character folder is gone, nothing to put back"
            ));
            continue;
        }
        let path = entry.character.addons_txt();
        let now = read_addons_txt(&path)?;
        if now.as_deref() != Some(entry.written.as_slice()) {
            warnings.push(format!(
                "{label}: AddOns.txt changed since the apply (the client writes it at logout); put back as it was before the apply all the same"
            ));
        }
        steps.push((entry.character, path, now, entry.then));
    }

    let mut done: Vec<usize> = Vec::new();
    for (index, (_, path, now, then)) in steps.iter().enumerate() {
        if now == then {
            continue;
        }
        let result = match then {
            Some(bytes) => write_file(path, bytes),
            None => remove_file_if_any(path),
        };
        if let Err(error) = result {
            let reverted = done.iter().all(|&earlier| {
                let (_, path, now, _) = &steps[earlier];
                match now {
                    Some(bytes) => write_file(path, bytes).is_ok(),
                    None => remove_file_if_any(path).is_ok(),
                }
            });
            return Err(Error::SetUndoFailed {
                set: record.set,
                reason: format!("{}: {error}", path.display()),
                reverted,
            });
        }
        done.push(index);
    }
    if let Err(error) = fs::remove_dir_all(&dir) {
        warnings.push(format!(
            "the undo record {} could not be removed ({error}); an undo now would put the same files back again",
            dir.display()
        ));
    }
    let mut restored = Vec::new();
    let mut removed = Vec::new();
    for (character, _, _, then) in steps {
        if then.is_some() {
            restored.push(character);
        } else {
            removed.push(character);
        }
    }
    warnings.extend(client::started_during_among(
        processes,
        flavor,
        "the set undo",
    ));
    Ok(Undone {
        set: record.set,
        applied_at: record.applied_at,
        restored,
        removed,
        warnings,
    })
}

/// An edit of one addon in a stored set (task 073).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SetEdit {
    /// The addon gets `enabled` in the set (added when the set does not have it).
    Enable,
    /// The addon gets `disabled` in the set (added when the set does not have it).
    Disable,
    /// The addon leaves the set: an apply then leaves its lines alone.
    Drop,
}

impl SetEdit {
    /// The verb: `enable`, `disable` or `drop`.
    pub const fn as_str(self) -> &'static str {
        match self {
            SetEdit::Enable => "enable",
            SetEdit::Disable => "disable",
            SetEdit::Drop => "drop",
        }
    }
}

/// An edit of a stored set that finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edited {
    /// The set's name as stored.
    pub name: String,
    /// The addon, spelled as the set has it (as given when the edit added it).
    pub addon: String,
    pub edit: SetEdit,
    /// The addon's state in the set before the edit; `None` when the set did not have it.
    pub before: Option<AddonState>,
    /// The addon's state in the set after the edit; `None` once dropped.
    pub after: Option<AddonState>,
    /// Whether `sets.json` was written (an edit to the state the addon has already writes nothing).
    pub changed: bool,
    /// The set after the edit.
    pub set: AddonSet,
    /// Whether the addon has a folder under `Interface\AddOns`.
    pub installed: bool,
}

impl Edited {
    /// The warning about an addon the set keeps that has no folder under `Interface\AddOns`, so a typo
    /// shows; `None` for an installed or a dropped addon (task 085: `mrgl set enable|disable|drop` and the
    /// window share it).
    pub fn not_installed(&self, folder_name: &str) -> Option<String> {
        (self.after.is_some() && !self.installed).then(|| {
            format!(
                "{} is not installed in {folder_name}; the set keeps it all the same",
                self.addon
            )
        })
    }

    /// The summary of an edit in the voice's terms (task 085, shared as [`Edited::not_installed`] is):
    /// what changed in the set and its count after, ending "no AddOns.txt was touched"; nothing to say for
    /// an edit to the state the addon has already.
    pub fn summary(&self, folder_name: &str) -> (Kind, String) {
        let (name, addon, count) = (&self.name, &self.addon, self.set.counted());
        let summary = match (self.before, self.after) {
            _ if !self.changed => {
                let state = self.after.map(AddonState::as_str).unwrap_or("dropped");
                return (
                    Kind::Nothing,
                    format!(
                        "set {name} of {folder_name} has {addon} {state} already, nothing changed"
                    ),
                );
            }
            (None, Some(state)) => {
                format!("added {addon} to set {name} of {folder_name}, {state}: {count}")
            }
            (Some(before), Some(state)) => {
                format!(
                    "{addon} is {state} in set {name} of {folder_name} now, was {before}: {count}"
                )
            }
            (_, None) => format!("dropped {addon} from set {name} of {folder_name}: {count}"),
        };
        (
            Kind::Success,
            format!("{summary}; no {ADDONS_TXT} was touched"),
        )
    }
}

/// Enables, disables or drops one addon of a stored set (task 073), in `sets.json` only: no `AddOns.txt`
/// is touched, so it goes on while the client runs. The addon is matched ignoring ASCII case and keeps the
/// set's spelling; an enable or a disable of an addon the set does not have adds it, a drop of one is
/// [`Error::AddonNotInSet`]. An edit that changes the set stores it again (its `saved_at` moves); one that
/// changes nothing writes nothing.
pub fn edit_set(
    flavor: &Flavor,
    sets_path: &Path,
    name: &str,
    addon: &str,
    edit: SetEdit,
) -> Result<Edited> {
    let addon = addon.trim();
    if !is_addon_name(addon) {
        return Err(Error::AddonName {
            name: addon.to_string(),
        });
    }
    let mut sets = read_sets_at(sets_path, &flavor.id)?;
    let key = sets
        .find(name)
        .map(|(key, _)| key.to_string())
        .ok_or_else(|| sets.not_found(name, &flavor.id))?;
    let set = sets.sets.get_mut(&key).expect("the key was just found");
    let known = set
        .addons
        .keys()
        .find(|known| known.eq_ignore_ascii_case(addon))
        .cloned();
    let before = known
        .as_ref()
        .and_then(|known| set.addons.get(known).copied());
    let stored = known.unwrap_or_else(|| addon.to_string());
    let after = match edit {
        SetEdit::Enable => Some(AddonState::Enabled),
        SetEdit::Disable => Some(AddonState::Disabled),
        SetEdit::Drop => None,
    };
    if edit == SetEdit::Drop && before.is_none() {
        let names: Vec<&str> = set.addons.keys().map(String::as_str).collect();
        return Err(Error::AddonNotInSet {
            set: key,
            addon: addon.to_string(),
            flavor: flavor_label(&flavor.id),
            known: if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            },
        });
    }
    let changed = before != after;
    if changed {
        match after {
            Some(state) => {
                set.addons.insert(stored.clone(), state);
            }
            None => {
                set.addons.remove(&stored);
            }
        }
        set.saved_at = state::now_rfc3339();
    }
    let set = set.clone();
    if changed {
        write_sets_at(sets_path, &sets)?;
    }
    let installed = scan::addons_dir(&flavor.folder).join(&stored).is_dir();
    Ok(Edited {
        name: key,
        addon: stored,
        edit,
        before,
        after,
        changed,
        set,
        installed,
    })
}

/// Removes a set from the flavor's sets file; no `AddOns.txt` is touched. The removed set is returned.
pub fn remove_set(flavor: &FlavorId, sets_path: &Path, name: &str) -> Result<(String, AddonSet)> {
    let mut sets = read_sets_at(sets_path, flavor)?;
    let key = sets
        .find(name)
        .map(|(key, _)| key.to_string())
        .ok_or_else(|| sets.not_found(name, flavor))?;
    let set = sets.sets.remove(&key).expect("the key was just found");
    write_sets_at(sets_path, &sets)?;
    Ok((key, set))
}

/// A set by name from the flavor's sets file, for `set show`.
pub fn find_set(flavor: &FlavorId, sets_path: &Path, name: &str) -> Result<(String, AddonSet)> {
    let sets = read_sets_at(sets_path, flavor)?;
    sets.find(name)
        .map(|(key, set)| (key.to_string(), set.clone()))
        .ok_or_else(|| sets.not_found(name, flavor))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_apply_summary_names_the_one_character_or_counts_them() {
        let character = |realm: &str, name: &str| Character {
            account: "ACC".into(),
            realm: realm.into(),
            name: name.into(),
            folder: PathBuf::from(realm).join(name),
        };
        let change = |realm: &str, name: &str, changed: usize| CharacterChange {
            character: character(realm, name),
            changed: (0..changed)
                .map(|index| (format!("Addon{index}"), AddonState::Enabled))
                .collect(),
            created: false,
        };
        let applied = |characters: Vec<CharacterChange>| Applied {
            name: "raid".into(),
            set: AddonSet {
                saved_at: "2026-10-22T10:00:00Z".into(),
                addons: BTreeMap::new(),
            },
            characters,
            not_installed: Vec::new(),
            warnings: Vec::new(),
        };
        let one = Target::Character {
            name: "grrlwave".into(),
            realm: None,
        };
        assert_eq!(
            applied(vec![change("Second Realm", "Grrlwave", 1)]).summary(&one, "_forever_"),
            (
                Kind::Success,
                "applied set raid to Second Realm/Grrlwave of _forever_: 1 addon line changed"
                    .to_string()
            )
        );
        assert_eq!(
            applied(vec![change("Second Realm", "Grrlwave", 0)]).summary(&one, "_forever_"),
            (
                Kind::Nothing,
                "set raid already matches Second Realm/Grrlwave of _forever_, nothing written"
                    .to_string()
            )
        );
        let every = applied(vec![
            change("Fixture Realm", "Mrgltest", 2),
            change("Second Realm", "Grrlwave", 1),
        ]);
        assert_eq!(
            every.summary(&Target::EveryCharacter, "_forever_").1,
            "applied set raid to 2 characters of _forever_: 3 addon lines changed"
        );
        let matched = applied(vec![change("Fixture Realm", "Mrgltest", 0)]);
        assert_eq!(
            matched.summary(&Target::EveryCharacter, "_forever_"),
            (
                Kind::Nothing,
                "set raid already matches every character of _forever_ (1 character), nothing written"
                    .to_string()
            )
        );
    }

    #[test]
    fn the_set_list_note_counts_the_sets_or_says_how_to_save_one() {
        let path = PathBuf::from("home").join("wowf").join(SETS_FILE);
        let mut file = SetsFile::new(&FlavorId::parse("wowf"));
        assert_eq!(
            file.summary(&path, "_forever_"),
            (
                Kind::Nothing,
                "no sets of _forever_ yet: `mrgl set save <name>` saves one".to_string()
            )
        );
        let set = AddonSet {
            saved_at: "2026-10-22T00:00:00Z".to_string(),
            addons: BTreeMap::new(),
        };
        file.sets.insert("raid".to_string(), set.clone());
        assert_eq!(
            file.summary(&path, "_forever_"),
            (
                Kind::Success,
                format!("1 set of _forever_ in {}", path.display())
            )
        );
        file.sets.insert("solo".to_string(), set);
        assert_eq!(
            file.summary(&path, "_forever_").1,
            format!("2 sets of _forever_ in {}", path.display())
        );
    }

    #[test]
    fn the_not_installed_warning_names_the_missing_addons_of_an_apply() {
        let mut applied = Applied {
            name: "raid".into(),
            set: AddonSet {
                saved_at: "2026-10-22T10:00:00Z".into(),
                addons: BTreeMap::new(),
            },
            characters: Vec::new(),
            not_installed: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(applied.not_installed_warning("_forever_"), None);
        applied.not_installed = vec!["BagSorter".into(), "RaidFrames".into()];
        assert_eq!(
            applied.not_installed_warning("_forever_").as_deref(),
            Some(
                "set raid names addons that are not installed in _forever_, their lines are written all the same: BagSorter, RaidFrames"
            )
        );
    }

    #[test]
    fn the_two_account_hint_names_no_flag() {
        let character = |account: &str, realm: &str| Character {
            account: account.into(),
            realm: realm.into(),
            name: "Mrgltest".into(),
            folder: PathBuf::from(account).join(realm).join("Mrgltest"),
        };
        let flavor = FlavorId::parse("wowf");
        let twins = [
            character("ONE", "Fixture Realm"),
            character("TWO", "Fixture Realm"),
        ];
        let error = find_character(&twins, "mrgltest", None, &flavor).expect_err("two accounts");
        assert!(
            matches!(
                &error,
                Error::AmbiguousCharacter {
                    by: CharacterPick::Account,
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(
            error.to_string().ends_with(
                "Fixture Realm (ONE), Fixture Realm (TWO); the same realm is under more than one account: pick the account too"
            ),
            "{error}"
        );
        assert!(!error.to_string().contains("--"), "{error}");
        let realms = [
            character("ONE", "Fixture Realm"),
            character("ONE", "Second Realm"),
        ];
        let error = find_character(&realms, "mrgltest", None, &flavor).expect_err("two realms");
        assert!(
            error
                .to_string()
                .ends_with("Fixture Realm, Second Realm; name one with --realm"),
            "{error}"
        );
    }

    #[test]
    fn a_file_round_trips_byte_for_byte() {
        for bytes in [
            &b""[..],
            b"A: enabled\r\nB: disabled\r\n",
            b"A: enabled\nB: disabled",
            b"\xEF\xBB\xBFA: enabled\r\n\r\n# a line the client never wrote\r\nB : DISABLED \n",
            b"\xFF\xFE not text\r\nA:enabled",
            b"\r\n\n\r",
        ] {
            assert_eq!(AddonsTxt::parse(bytes).to_bytes(), bytes, "{bytes:?}");
        }
    }

    #[test]
    fn entries_are_the_addon_lines_in_file_order() {
        let file = AddonsTxt::parse(
            b"\xEF\xBB\xBFZeta: enabled\r\nnot an entry\r\nAlpha :  disabled\r\nBad: maybe\r\n: enabled\r\n",
        );
        assert_eq!(
            file.entries(),
            vec![
                ("Zeta".to_string(), AddonState::Enabled),
                ("Alpha".to_string(), AddonState::Disabled),
            ]
        );
        assert_eq!(file.state_of("alpha"), Some(AddonState::Disabled));
        assert_eq!(file.state_of("Missing"), None);
    }

    #[test]
    fn a_state_change_keeps_everything_but_the_word() {
        let mut file = AddonsTxt::parse(b"A: enabled\r\nb :  disabled \r\nC: enabled\r\n");
        assert!(!file.set_state("A", AddonState::Enabled), "no change");
        assert!(file.set_state("B", AddonState::Enabled));
        assert_eq!(
            file.to_bytes(),
            b"A: enabled\r\nb :  enabled \r\nC: enabled\r\n"
        );
        let mut shouting = AddonsTxt::parse(b"A: ENABLED\r\n");
        assert!(
            !shouting.set_state("a", AddonState::Enabled),
            "the same state"
        );
        assert_eq!(shouting.to_bytes(), b"A: ENABLED\r\n");
    }

    #[test]
    fn a_missing_addon_is_appended_with_the_files_ending() {
        let mut file = AddonsTxt::parse(b"A: enabled\nB: disabled");
        assert!(file.set_state("New", AddonState::Disabled));
        assert_eq!(file.to_bytes(), b"A: enabled\nB: disabled\nNew: disabled\n");
        let mut empty = AddonsTxt::parse(b"");
        assert!(empty.set_state("New", AddonState::Enabled));
        assert_eq!(empty.to_bytes(), b"New: enabled\r\n");
    }

    #[test]
    fn addon_and_set_names_are_checked() {
        assert!(is_addon_name("DBM-Core"));
        assert!(is_addon_name("Ölfänger"));
        for bad in ["", " A", "A:B", "A\r\nB: enabled", "..\\A", "a/b"] {
            assert!(!is_addon_name(bad), "{bad:?}");
        }
        assert_eq!(checked_set_name("  raid ").expect("a name"), "raid");
        assert!(checked_set_name(" ").is_err());
        assert!(checked_set_name("a\tb").is_err());
        assert!(checked_set_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn a_set_is_found_by_its_name_or_ignoring_case_when_that_is_unique() {
        let mut file = SetsFile::new(&FlavorId::parse("wowf"));
        let set = AddonSet {
            saved_at: "2026-10-22T00:00:00Z".to_string(),
            addons: BTreeMap::new(),
        };
        file.sets.insert("Raid".to_string(), set.clone());
        assert_eq!(file.find("raid").map(|(name, _)| name), Some("Raid"));
        file.sets.insert("RAID".to_string(), set);
        assert_eq!(file.find("raid"), None, "two match ignoring case");
        assert_eq!(file.find("RAID").map(|(name, _)| name), Some("RAID"));
    }

    #[test]
    fn the_edit_verbs_are_the_words_of_the_command_line() {
        assert_eq!(SetEdit::Enable.as_str(), "enable");
        assert_eq!(SetEdit::Disable.as_str(), "disable");
        assert_eq!(SetEdit::Drop.as_str(), "drop");
    }

    #[test]
    fn the_undo_summary_counts_the_files_and_warns_when_the_undo_did() {
        let character = |name: &str| Character {
            account: "ACC".into(),
            realm: "Realm".into(),
            name: name.into(),
            folder: PathBuf::from("Realm").join(name),
        };
        let mut undone = Undone {
            set: "raid".into(),
            applied_at: "2026-10-22T10:00:00Z".into(),
            restored: vec![character("One")],
            removed: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(
            undone.summary("_forever_"),
            (
                Kind::Success,
                "undid the apply of set raid to _forever_ (2026-10-22T10:00:00Z): 1 AddOns.txt put back"
                    .to_string()
            )
        );
        undone.restored.push(character("Two"));
        undone.removed.push(character("Three"));
        undone
            .warnings
            .push("Realm/Four: the character folder is gone".into());
        assert_eq!(
            undone.summary("_forever_"),
            (
                Kind::Warning,
                "undid the apply of set raid to _forever_ (2026-10-22T10:00:00Z): 2 AddOns.txt files put back, 1 AddOns.txt the apply created removed"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_name_of_the_undo_record_is_one_folder_name() {
        // The record's names follow the one folder-name rule (task 123), whose own cases are tested in
        // `zip_entry`; these are the shapes a record carries.
        assert!(zip_entry::is_folder_name("1-before.txt"));
        assert!(zip_entry::is_folder_name("Fixture Realm"));
        for bad in ["", ".", "..", "a:b", "a/b", "a\\b", "a\u{1}b"] {
            assert!(!zip_entry::is_folder_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_removal_of_what_is_not_there_is_nothing_and_a_file_is_no_folder() {
        let dir = std::env::temp_dir().join(format!("mrgl-sets-removals-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the folder");
        assert!(remove_dir_if_any(&dir.join("missing")).is_ok());
        assert!(remove_file_if_any(&dir.join("missing")).is_ok());
        fs::write(dir.join("a-file"), b"").expect("a file");
        let error = remove_dir_if_any(&dir.join("a-file")).expect_err("a file is no folder");
        assert_ne!(error.kind(), io::ErrorKind::NotFound);
        assert!(dir.join("a-file").exists());
        fs::create_dir_all(dir.join("a-folder")).expect("a folder");
        assert!(remove_dir_if_any(&dir.join("a-folder")).is_ok());
        assert!(!dir.join("a-folder").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
