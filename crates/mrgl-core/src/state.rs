//! The state sidecar (D-003, task 004): `%LOCALAPPDATA%\mrgl\<flavor-id>\state.json` with, per installed
//! addon, its source, version, file list with hashes and install time. The AddOns folder is the truth and
//! the state is a cache: a missing or corrupt file reads as a fresh state with a warning and is rebuilt from
//! the scan, never an error the user has to fix; an addon the scan finds without an entry is "unmanaged".
//!
//! The flavor's folder under the mrgl home also holds the install engine's `staging\` (where a zip is
//! unpacked and checked before anything moves) and `cache\<addon>\<version>.zip` (the current and the
//! previous version's zip, for the rollback of task 022; never required).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::home;
use crate::scan::Scan;
use crate::sources::Channel;

/// The sidecar's file name under the flavor's folder.
pub const STATE_FILE: &str = "state.json";
/// The staging folder under the flavor's folder: a zip is unpacked and checked here before the swap.
pub const STAGING_DIR: &str = "staging";
/// The cache folder under the flavor's folder: `cache\<addon>\<version>.zip`.
pub const CACHE_DIR: &str = "cache";
/// The schema of `state.json`; a file with another number reads as corrupt and is rebuilt.
pub const SCHEMA: u32 = 1;

/// A flavor's folder under the mrgl home, with the sidecar, the staging and the cache in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlavorHome {
    /// `<mrgl home>\<flavor-id>`.
    pub dir: PathBuf,
}

impl FlavorHome {
    /// The flavor's folder under this process's home (`MRGL_HOME`, else `LOCALAPPDATA\mrgl`).
    pub fn new(flavor: &FlavorId) -> Result<FlavorHome> {
        Ok(FlavorHome::at(&home::home_dir()?, flavor))
    }

    /// The flavor's folder under a given home folder.
    pub fn at(home: &Path, flavor: &FlavorId) -> FlavorHome {
        FlavorHome {
            dir: home.join(flavor.as_str()),
        }
    }

    /// `state.json`.
    pub fn state_path(&self) -> PathBuf {
        self.dir.join(STATE_FILE)
    }

    /// The staging folder.
    pub fn staging_dir(&self) -> PathBuf {
        self.dir.join(STAGING_DIR)
    }

    /// The cache folder of every addon.
    pub fn cache_dir(&self) -> PathBuf {
        self.dir.join(CACHE_DIR)
    }

    /// One addon's cache folder: `cache\<addon>`.
    pub fn addon_cache_dir(&self, name: &str) -> PathBuf {
        self.cache_dir().join(file_safe(name))
    }

    /// The cached zip of one version of an addon: `cache\<addon>\<version>.zip`.
    pub fn cached_zip(&self, name: &str, version: &str) -> PathBuf {
        self.addon_cache_dir(name)
            .join(format!("{}.zip", file_safe(version)))
    }

    /// Reads the sidecar: a missing file is a fresh state, a corrupt one a fresh state with a warning.
    pub fn read_state(&self, flavor: &FlavorId) -> Result<LoadedState> {
        read_state_at(&self.state_path(), flavor)
    }

    /// Writes the sidecar, creating the folder.
    pub fn write_state(&self, state: &State) -> Result<()> {
        write_state_at(&self.state_path(), state)
    }
}

/// A name or a version as a file name: anything outside `A-Z a-z 0-9 . _ -` becomes `_`, so a version
/// such as `v1.2/beta` or a name with a space stays one path component.
pub fn file_safe(text: &str) -> String {
    let safe: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = safe.trim_matches('.');
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// One installed addon as the sidecar remembers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The source spec the addon came from, as `mrgl add` takes it: `github:<owner>/<repo>`, later
    /// `wago:…`, `wowi:…`, `curse:…`, or the path of a zip in hand.
    pub source: String,
    /// The installed version: the release's, or the TOC's `## Version`, or `unknown`.
    pub version: String,
    /// The version installed before this one, whose zip the cache may still hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<String>,
    /// The addon's folders under `Interface\AddOns`, the primary one first.
    pub folders: Vec<String>,
    /// Every installed file as `<Folder>/<path>` with `/` separators, and its SHA-256 as hex.
    pub files: BTreeMap<String, String>,
    /// When the install finished, RFC 3339 in UTC (`2026-10-21T14:03:05Z`).
    pub installed_at: String,
    /// The installed asset is a `nolib` build (task 029): `Some(true)` carries no embedded library,
    /// `Some(false)` carries its libraries, `None` (absent in the JSON) is unknown, a zip in hand or an
    /// entry written before the flag existed. The dependency check reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nolib: Option<bool>,
    /// The version the addon is pinned at (task 021): the update check reports a newer release and leaves
    /// the addon alone. `None`, or absent in the file, is unpinned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<String>,
    /// The addon is ignored (task 021): the update check skips it and counts it. Absent means `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ignored: bool,
    /// The release channel the addon follows (task 021); absent means [`Channel::Stable`].
    #[serde(default, skip_serializing_if = "Channel::is_stable")]
    pub channel: Channel,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The per-entry queries the update check calls, and the setters behind `mrgl pin`, `unpin`, `ignore`,
/// `unignore` and `channel` (task 021). Every setter returns whether it changed the entry, so a caller
/// writes the sidecar only when something moved.
impl Entry {
    /// The version the addon is pinned at, when it is.
    pub fn pinned(&self) -> Option<&str> {
        self.pinned.as_deref()
    }

    /// Whether the addon is ignored.
    pub fn ignored(&self) -> bool {
        self.ignored
    }

    /// The release channel the addon follows.
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// Pins the addon at a version, or at the installed one when none (or an empty one) is given.
    pub fn pin(&mut self, version: Option<&str>) -> bool {
        let version = version
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .unwrap_or(&self.version)
            .to_string();
        let changed = self.pinned.as_deref() != Some(version.as_str());
        self.pinned = Some(version);
        changed
    }

    /// Unpins the addon.
    pub fn unpin(&mut self) -> bool {
        self.pinned.take().is_some()
    }

    /// Marks the addon ignored.
    pub fn ignore(&mut self) -> bool {
        !std::mem::replace(&mut self.ignored, true)
    }

    /// Clears the ignored mark.
    pub fn unignore(&mut self) -> bool {
        std::mem::replace(&mut self.ignored, false)
    }

    /// Puts the addon on a release channel.
    pub fn set_channel(&mut self, channel: Channel) -> bool {
        std::mem::replace(&mut self.channel, channel) != channel
    }
}

/// The sidecar of one flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// [`SCHEMA`].
    pub schema: u32,
    /// The flavor id, as `.flavor.info` spells it.
    pub flavor: String,
    /// The managed addons by name, sorted.
    #[serde(default)]
    pub addons: BTreeMap<String, Entry>,
}

impl State {
    /// A fresh state with no managed addon.
    pub fn new(flavor: &FlavorId) -> State {
        State {
            schema: SCHEMA,
            flavor: flavor.as_str().to_string(),
            addons: BTreeMap::new(),
        }
    }

    /// The state rebuilt from a scan: the scan says what is in the folder, and nothing in it carries a
    /// source or a version mrgl installed, so a rebuilt state has no managed addon; every folder is
    /// unmanaged until it is installed through mrgl again.
    pub fn rebuild(flavor: &FlavorId, scan: &Scan) -> State {
        let mut state = State::new(flavor);
        state.reconcile(scan);
        state
    }

    /// The entry of a name, ASCII case-insensitive, with the name as the sidecar spells it.
    pub fn entry(&self, name: &str) -> Option<(&str, &Entry)> {
        self.addons
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(known, entry)| (known.as_str(), entry))
    }

    /// The entry a name on the command line means: the addon by name, else the addon that owns a folder of
    /// that name (a `list` line shows folders), ASCII case-insensitive, with the name as the sidecar
    /// spells it.
    pub fn find(&self, name: &str) -> Option<(&str, &Entry)> {
        self.entry(name).or_else(|| self.entry_for_folder(name))
    }

    /// [`State::find`] for a change: the sidecar's name and the entry to change.
    pub fn find_mut(&mut self, name: &str) -> Option<(String, &mut Entry)> {
        let known = self.find(name)?.0.to_string();
        let entry = self.addons.get_mut(&known)?;
        Some((known, entry))
    }

    /// The entry that owns a folder, ASCII case-insensitive.
    pub fn entry_for_folder(&self, folder: &str) -> Option<(&str, &Entry)> {
        self.addons
            .iter()
            .find(|(_, entry)| {
                entry
                    .folders
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(folder))
            })
            .map(|(name, entry)| (name.as_str(), entry))
    }

    /// Drops every entry none of whose folders is in the folder any more (deleted by hand, which must
    /// never strand the manager); an entry with some folders left stays. Returns the dropped names.
    pub fn reconcile(&mut self, scan: &Scan) -> Vec<String> {
        let present: Vec<&str> = scan
            .addons
            .iter()
            .map(|addon| addon.folder.as_str())
            .chain(scan.skipped.iter().map(String::as_str))
            .collect();
        let dropped: Vec<String> = self
            .addons
            .iter()
            .filter(|(_, entry)| {
                !entry.folders.iter().any(|folder| {
                    present
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(folder))
                })
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in &dropped {
            self.addons.remove(name);
        }
        dropped
    }

    /// Every addon of a scan with its entry when the sidecar knows it; the rest is unmanaged.
    pub fn classify<'a>(&'a self, scan: &'a Scan) -> Vec<Status<'a>> {
        scan.addons
            .iter()
            .map(|addon| {
                let managed = self.entry_for_folder(&addon.folder);
                Status {
                    folder: addon.folder.as_str(),
                    name: managed.map(|(name, _)| name),
                    entry: managed.map(|(_, entry)| entry),
                }
            })
            .collect()
    }
}

/// One scanned addon folder against the sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status<'a> {
    /// The folder under `Interface\AddOns`.
    pub folder: &'a str,
    /// The managed addon's name when the sidecar knows the folder.
    pub name: Option<&'a str>,
    /// Its entry; `None` means unmanaged.
    pub entry: Option<&'a Entry>,
}

impl Status<'_> {
    /// Whether the sidecar knows the folder.
    pub fn is_managed(&self) -> bool {
        self.entry.is_some()
    }
}

/// A state as read: a missing or corrupt file reads as a fresh one, the corrupt one with a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedState {
    pub state: State,
    pub warning: Option<String>,
}

/// Reads a sidecar file: a missing file is a fresh state, a corrupt one or one of another schema or flavor
/// a fresh state with a warning.
pub fn read_state_at(path: &Path, flavor: &FlavorId) -> Result<LoadedState> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LoadedState {
                state: State::new(flavor),
                warning: None,
            });
        }
        Err(source) => {
            return Err(Error::Io {
                context: "reading the state sidecar",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let fresh = |reason: String| LoadedState {
        state: State::new(flavor),
        warning: Some(format!(
            "unreadable state sidecar, starting over (every addon is unmanaged until it is installed through mrgl again): {}: {reason}",
            path.display()
        )),
    };
    match serde_json::from_str::<State>(&text) {
        Ok(state) if state.schema != SCHEMA => Ok(fresh(format!(
            "schema {} instead of {SCHEMA}",
            state.schema
        ))),
        Ok(state) if state.flavor != flavor.as_str() => Ok(fresh(format!(
            "flavor {} instead of {flavor}",
            state.flavor
        ))),
        Ok(state) => Ok(LoadedState {
            state,
            warning: None,
        }),
        Err(reason) => Ok(fresh(reason.to_string())),
    }
}

/// Writes a sidecar file, creating its folder; the write goes through a temporary file next to it, so a
/// crash mid-write leaves the old file whole.
pub fn write_state_at(path: &Path, state: &State) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            context: "creating the flavor's folder under the mrgl home",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let io = |source| Error::Io {
        context: "writing the state sidecar",
        path: path.to_path_buf(),
        source,
    };
    let text =
        serde_json::to_string_pretty(state).map_err(|source| io(std::io::Error::other(source)))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text + "\n").map_err(io)?;
    std::fs::rename(&temporary, path).map_err(io)
}

/// The current time as RFC 3339 in UTC, to the second: `2026-10-21T14:03:05Z`.
pub fn now_rfc3339() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    rfc3339_from_unix(seconds)
}

/// Unix seconds as RFC 3339 in UTC (the civil-from-days algorithm; no calendar crate for one date).
pub fn rfc3339_from_unix(seconds: u64) -> String {
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (hour, minute, second) = (rest / 3_600, rest % 3_600 / 60, rest % 60);
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavor::{BuildVersion, Flavor};
    use crate::scan::Scan;

    fn temp_home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-state-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn entry(folders: &[&str]) -> Entry {
        Entry {
            source: "github:someone/something".to_string(),
            version: "1.0.0".to_string(),
            previous_version: None,
            folders: folders.iter().map(|folder| folder.to_string()).collect(),
            files: BTreeMap::from([(
                format!("{}/{}.toc", folders[0], folders[0]),
                "ab".repeat(32),
            )]),
            installed_at: "2026-10-21T14:03:05Z".to_string(),
            nolib: None,
            pinned: None,
            ignored: false,
            channel: Channel::Stable,
        }
    }

    fn scan_of(folders: &[&str]) -> Scan {
        let flavor = Flavor {
            id: FlavorId::Forever,
            folder: PathBuf::from("_forever_"),
            version: BuildVersion::parse("1.60.1.70245"),
            version_source: None,
            curse_client: false,
            warnings: Vec::new(),
        };
        let chain = flavor.toc_chain();
        let addons = folders
            .iter()
            .map(|folder| crate::scan::Addon {
                folder: folder.to_string(),
                path: PathBuf::from(folder),
                tocs: Vec::new(),
                other_tocs: Vec::new(),
                chosen: chain.first().map(|_| format!("{folder}.toc")),
                linked: false,
            })
            .collect();
        Scan {
            flavor,
            addons,
            skipped: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn the_flavor_home_lays_out_the_sidecar_the_staging_and_the_cache() {
        let home = FlavorHome::at(Path::new("H:\\home"), &FlavorId::Forever);
        assert_eq!(home.dir, Path::new("H:\\home").join("wowf"));
        assert_eq!(home.state_path(), home.dir.join("state.json"));
        assert_eq!(home.staging_dir(), home.dir.join("staging"));
        assert_eq!(
            home.cached_zip("Big Addon", "v1.2/beta"),
            home.dir
                .join("cache")
                .join("Big_Addon")
                .join("v1.2_beta.zip")
        );
        assert_eq!(file_safe("..."), "_");
        assert_eq!(file_safe("..Foo"), "Foo");
    }

    #[test]
    fn a_state_round_trips_and_a_missing_file_is_fresh() {
        let home = temp_home("roundtrip");
        let path = home.join("wowf").join(STATE_FILE);
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert_eq!(loaded.state, State::new(&FlavorId::Forever));
        assert_eq!(loaded.warning, None);

        let mut state = State::new(&FlavorId::Forever);
        state
            .addons
            .insert("Foo".to_string(), entry(&["Foo", "Foo_Options"]));
        write_state_at(&path, &state).expect("writes");
        assert!(!path.with_extension("json.tmp").exists());
        let text = std::fs::read_to_string(&path).expect("the file");
        assert!(text.contains("\"schema\": 1"), "{text}");
        assert!(!text.contains("previous_version"), "{text}");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert_eq!(loaded.state, state);
        assert_eq!(loaded.warning, None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_nolib_flag_is_written_when_known_and_absent_reads_unknown() {
        let mut known = entry(&["Foo"]);
        known.nolib = Some(true);
        let text = serde_json::to_string(&known).expect("serializes");
        assert!(text.contains("\"nolib\":true"), "{text}");
        assert_eq!(
            serde_json::from_str::<Entry>(&text).expect("reads").nolib,
            Some(true)
        );
        let unknown = serde_json::to_string(&entry(&["Foo"])).expect("serializes");
        assert!(!unknown.contains("nolib"), "{unknown}");
        assert_eq!(
            serde_json::from_str::<Entry>(&unknown)
                .expect("reads")
                .nolib,
            None
        );
    }

    #[test]
    fn pin_ignore_and_channel_round_trip_and_stay_out_of_the_file_when_unset() {
        let home = temp_home("holds");
        let path = home.join("wowf").join(STATE_FILE);
        let mut state = State::new(&FlavorId::Forever);
        let mut pinned = entry(&["Pinned"]);
        assert!(pinned.pin(None), "pins the installed version");
        assert_eq!(pinned.pinned(), Some("1.0.0"));
        let mut ignored = entry(&["Ignored"]);
        assert!(ignored.ignore());
        let mut beta = entry(&["Beta"]);
        assert!(beta.set_channel(Channel::Prerelease));
        state.addons.insert("Pinned".to_string(), pinned);
        state.addons.insert("Ignored".to_string(), ignored);
        state.addons.insert("Beta".to_string(), beta);
        state.addons.insert("Plain".to_string(), entry(&["Plain"]));
        write_state_at(&path, &state).expect("writes");
        let text = std::fs::read_to_string(&path).expect("the file");
        assert_eq!(text.matches(r#""pinned": "1.0.0""#).count(), 1, "{text}");
        assert_eq!(text.matches(r#""ignored": true"#).count(), 1, "{text}");
        assert_eq!(
            text.matches(r#""channel": "prerelease""#).count(),
            1,
            "{text}"
        );
        assert!(!text.contains(r#""ignored": false"#), "{text}");
        assert!(!text.contains(r#""stable""#), "{text}");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert_eq!(loaded.state, state);
        assert_eq!(loaded.warning, None);
        let (_, plain) = loaded.state.entry("Plain").expect("plain");
        assert_eq!(
            (plain.pinned(), plain.ignored(), plain.channel()),
            (None, false, Channel::Stable)
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_sidecar_without_the_fields_of_task_021_reads_unchanged() {
        let home = temp_home("older");
        let path = home.join(STATE_FILE);
        std::fs::create_dir_all(&home).expect("home");
        let older = r#"{
  "schema": 1,
  "flavor": "wowf",
  "addons": {
    "Foo": {
      "source": "github:someone/something",
      "version": "1.0.0",
      "folders": ["Foo"],
      "files": {},
      "installed_at": "2026-10-21T14:03:05Z"
    }
  }
}"#;
        std::fs::write(&path, older).expect("write");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert_eq!(loaded.warning, None);
        let (_, foo) = loaded.state.entry("Foo").expect("the entry");
        assert_eq!(foo.pinned(), None);
        assert!(!foo.ignored());
        assert_eq!(foo.channel(), Channel::Stable);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_setters_say_whether_they_changed_the_entry() {
        let mut foo = entry(&["Foo"]);
        assert!(foo.pin(Some(" 0.9.0 ")));
        assert_eq!(foo.pinned(), Some("0.9.0"));
        assert!(!foo.pin(Some("0.9.0")), "the same pin again");
        assert!(foo.pin(Some("")), "an empty version pins the installed one");
        assert_eq!(foo.pinned(), Some("1.0.0"));
        assert!(foo.unpin());
        assert!(!foo.unpin());
        assert!(foo.ignore());
        assert!(!foo.ignore());
        assert!(foo.unignore());
        assert!(!foo.unignore());
        assert!(!foo.set_channel(Channel::Stable));
        assert!(foo.set_channel(Channel::Prerelease));
        assert!(!foo.set_channel(Channel::Prerelease));
    }

    #[test]
    fn find_takes_a_name_or_a_folder() {
        let mut state = State::new(&FlavorId::Forever);
        state
            .addons
            .insert("Foo".to_string(), entry(&["Foo", "Foo_Options"]));
        assert_eq!(state.find("foo").map(|(name, _)| name), Some("Foo"));
        assert_eq!(state.find("FOO_OPTIONS").map(|(name, _)| name), Some("Foo"));
        assert_eq!(state.find("Bar"), None);
        let (name, foo) = state.find_mut("foo_options").expect("by folder");
        assert_eq!(name, "Foo");
        foo.ignore();
        assert!(state.entry("Foo").expect("Foo").1.ignored());
    }

    #[test]
    fn a_corrupt_or_foreign_state_is_fresh_with_a_warning() {
        let home = temp_home("corrupt");
        let path = home.join(STATE_FILE);
        std::fs::create_dir_all(&home).expect("home");
        std::fs::write(&path, "{ not json").expect("write");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert_eq!(loaded.state, State::new(&FlavorId::Forever));
        let warning = loaded.warning.expect("a warning");
        assert!(
            warning.starts_with("unreadable state sidecar, starting over"),
            "{warning}"
        );
        assert!(warning.contains(STATE_FILE), "{warning}");

        std::fs::write(&path, "{\"schema\": 2, \"flavor\": \"wowf\"}").expect("write");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert!(
            loaded
                .warning
                .expect("a warning")
                .contains("schema 2 instead of 1")
        );

        std::fs::write(&path, "{\"schema\": 1, \"flavor\": \"wow\"}").expect("write");
        let loaded = read_state_at(&path, &FlavorId::Forever).expect("reads");
        assert!(
            loaded
                .warning
                .expect("a warning")
                .contains("flavor wow instead of wowf")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn lookups_are_case_insensitive_by_name_and_by_folder() {
        let mut state = State::new(&FlavorId::Forever);
        state
            .addons
            .insert("Foo".to_string(), entry(&["Foo", "Foo_Options"]));
        assert_eq!(state.entry("foo").map(|(name, _)| name), Some("Foo"));
        assert_eq!(state.entry("Bar"), None);
        assert_eq!(
            state.entry_for_folder("FOO_OPTIONS").map(|(name, _)| name),
            Some("Foo")
        );
        assert_eq!(state.entry_for_folder("Foo_Data"), None);
    }

    #[test]
    fn reconcile_drops_what_was_deleted_by_hand_and_classify_names_the_unmanaged() {
        let mut state = State::new(&FlavorId::Forever);
        state
            .addons
            .insert("Foo".to_string(), entry(&["Foo", "Foo_Options"]));
        state.addons.insert("Gone".to_string(), entry(&["Gone"]));
        let scan = scan_of(&["Foo", "ByHand"]);
        assert_eq!(state.reconcile(&scan), ["Gone"]);
        assert!(state.entry("Gone").is_none());
        assert!(
            state.entry("Foo").is_some(),
            "one folder left keeps the entry"
        );
        let statuses = state.classify(&scan);
        assert_eq!(statuses.len(), 2);
        assert!(statuses[0].is_managed() && statuses[0].name == Some("Foo"));
        assert!(!statuses[1].is_managed() && statuses[1].folder == "ByHand");
        let rebuilt = State::rebuild(&FlavorId::Forever, &scan);
        assert!(rebuilt.addons.is_empty(), "a rebuilt state manages nothing");
    }

    #[test]
    fn unix_seconds_format_as_rfc3339_utc() {
        assert_eq!(rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_from_unix(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339_from_unix(1_792_591_385), "2026-10-21T14:03:05Z");
        assert_eq!(rfc3339_from_unix(4_102_444_799), "2099-12-31T23:59:59Z");
        let now = now_rfc3339();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z'));
    }
}
