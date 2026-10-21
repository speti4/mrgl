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

/// The sets file's name under the flavor's folder of the mrgl home.
pub const SETS_FILE: &str = "sets.json";
/// The `format` value of a sets file.
pub const SETS_FORMAT: &str = "mrgl-sets";
/// The schema of `sets.json`; a file with another number is refused, never rewritten.
pub const SETS_SCHEMA: u32 = 1;
/// The client's per-character addon state file.
pub const ADDONS_TXT: &str = "AddOns.txt";
/// The accounts folder under `WTF`.
pub const ACCOUNT_DIR: &str = "Account";
/// The account-wide SavedVariables folder, next to the realm folders of an account: never a realm.
pub const SAVED_VARIABLES_DIR: &str = "SavedVariables";
/// The longest set name, in characters.
pub const SET_NAME_MAX_CHARS: usize = 64;

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

    /// `<account>/<realm>/<character>`, for the plain lines.
    pub fn label(&self) -> String {
        format!("{}/{}/{}", self.account, self.realm, self.name)
    }
}

/// The characters of a flavor's `WTF` folder.
pub fn characters(flavor: &Flavor) -> Result<Vec<Character>> {
    characters_in(&flavor.folder.join(WTF_DIR))
}

/// The characters under a `WTF` folder, sorted by account, realm and name: every folder two levels under
/// an account folder (`<realm>\<character>`), the account's `SavedVariables` never a realm. Files, links
/// and junctions are passed over; a missing `WTF` or `Account` folder is no character, never an error.
pub fn characters_in(wtf: &Path) -> Result<Vec<Character>> {
    let mut characters = Vec::new();
    for (account, account_dir) in subfolders(&wtf.join(ACCOUNT_DIR))? {
        for (realm, realm_dir) in subfolders(&account_dir)? {
            if realm.eq_ignore_ascii_case(SAVED_VARIABLES_DIR) {
                continue;
            }
            for (name, folder) in subfolders(&realm_dir)? {
                characters.push(Character {
                    account: account.clone(),
                    realm: realm.clone(),
                    name,
                    folder,
                });
            }
        }
    }
    characters.sort();
    Ok(characters)
}

/// The real subfolders of a folder (no link, no junction) with their names; a missing folder has none.
fn subfolders(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let io = |source| Error::Io {
        context: "reading WTF",
        path: dir.to_path_buf(),
        source,
    };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io(source)),
    };
    let mut folders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io)?;
        if entry.file_type().map_err(io)?.is_dir() {
            folders.push((
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            ));
        }
    }
    folders.sort();
    Ok(folders)
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
    /// The characters whose `AddOns.txt` was read.
    pub characters: usize,
    /// Addons left out because the characters disagree on them, by name.
    pub conflicts: Vec<String>,
    /// Plain lines about what was passed over.
    pub warnings: Vec<String>,
}

/// Saves a set from the flavor's `AddOns.txt` files: every addon a file names, with its state; an addon
/// the characters disagree on is left out (an apply then leaves it alone) and named in
/// [`Saved::conflicts`]. A set of the same name is replaced. Reads `WTF` only, so it goes on while the
/// client runs.
pub fn save_set(flavor: &Flavor, sets_path: &Path, name: &str) -> Result<Saved> {
    let name = checked_set_name(name)?;
    let mut sets = read_sets_at(sets_path, &flavor.id)?;
    let characters = characters(flavor)?;
    if characters.is_empty() {
        return Err(Error::NoCharacters {
            path: flavor.folder.join(WTF_DIR),
        });
    }
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
        characters: characters.len(),
        conflicts,
        warnings,
    })
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
}

impl Applied {
    /// The lines changed or added over every character.
    pub fn changed(&self) -> usize {
        self.characters
            .iter()
            .map(|character| character.changed.len())
            .sum()
    }
}

/// Applies a set to every character of the flavor. In this order: the set read (an unknown name is
/// [`Error::SetNotFound`]), the characters listed (none is [`Error::NoCharacters`]), the guard of task
/// 023 (a running client refuses the apply before anything is written), every new `AddOns.txt` worked
/// out, then only the files that change written, each through a temporary file. A failed write puts the
/// files already written back and is [`Error::SetApplyFailed`].
pub fn apply_set(flavor: &Flavor, sets_path: &Path, name: &str) -> Result<Applied> {
    let sets = read_sets_at(sets_path, &flavor.id)?;
    let (name, set) = sets
        .find(name)
        .map(|(key, set)| (key.to_string(), set.clone()))
        .ok_or_else(|| sets.not_found(name, &flavor.id))?;
    let characters = characters(flavor)?;
    if characters.is_empty() {
        return Err(Error::NoCharacters {
            path: flavor.folder.join(WTF_DIR),
        });
    }
    client::ensure_client_not_running(flavor)?;

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
    write_plans(&mut plans, &name)?;

    let addons_dir = scan::addons_dir(&flavor.folder);
    let not_installed = set
        .addons
        .keys()
        .filter(|addon| !addons_dir.join(addon).is_dir())
        .cloned()
        .collect();
    Ok(Applied {
        name,
        set,
        characters: plans.into_iter().map(|plan| plan.change).collect(),
        not_installed,
    })
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
mod tests {
    use super::*;

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
}
