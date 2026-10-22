//! The answers of the last online update check that the cache cannot give again (task 078): a source gone
//! (a 404, no release, no asset for the flavor), a check that failed, and an addon the check left before the
//! request limit. The offline check answers every other state from the GitHub source's cache (task 041), so
//! it is kept here only for these three, per addon, in `last-check.json` next to the flavor's sidecar
//! (`<home>\<flavor-id>\last-check.json`), until the next online check of the addon replaces it.
//!
//! The file is a cache like the release list, not the user's data and not the install record: one that is
//! missing, unreadable, of another format or schema, or of another flavor reads as empty, and the next check
//! writes it again. An answer applies while the sidecar entry it was given for still has the same source
//! and version, so an addon updated or reinstalled since (by the command line too) drops it; a mark (pin,
//! ignore, the channel) keeps it, and an ignored addon's row shows the ignore instead, since the check
//! skips it.
//!
//! A `curse:` entry's answer keeps no current release (task 096): the CurseForge API's terms allow no saving
//! of its data, so the file holds the state and the reason of such an answer and never a version CurseForge
//! named.
//!
//! Two writers, one rule ([`record_check`]): the window's check and update (task 078) and the command
//! line's `mrgl update` (task 087), so what one of them found shows in the window either way.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::sources::{SourceKind, SourceSpec};
use crate::state::{self, FlavorHome, State};
use crate::update::{AddonUpdate, UpdateState};

/// The file under the flavor's folder of the mrgl home.
pub const LAST_CHECK_FILE: &str = "last-check.json";
/// The file's `format`.
pub const FORMAT: &str = "mrgl-last-check";
/// The file's `schema`; another number reads as empty.
pub const SCHEMA: u32 = 1;

/// Whether the offline check cannot give a state again, so the last online answer is kept: a source gone,
/// a failed check, an addon not asked before the request limit.
pub fn is_kept(state: UpdateState) -> bool {
    matches!(
        state,
        UpdateState::SourceGone | UpdateState::Failed | UpdateState::RateLimited
    )
}

/// One addon's kept answer: the source and the version it was given for, the state, the current release
/// when the source named one, the reason, and when the check ran (RFC 3339 in UTC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptAnswer {
    pub source: String,
    pub version: String,
    pub state: UpdateState,
    #[serde(default)]
    pub latest: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    pub checked_at: String,
}

/// The kept answers of one flavor, by the sidecar's addon name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastCheck {
    pub format: String,
    pub schema: u32,
    /// The flavor id the answers belong to.
    pub flavor: String,
    #[serde(default)]
    pub addons: BTreeMap<String, KeptAnswer>,
}

impl LastCheck {
    /// No answer kept for a flavor.
    pub fn new(flavor: &FlavorId) -> LastCheck {
        LastCheck {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            flavor: flavor.as_str().to_string(),
            addons: BTreeMap::new(),
        }
    }

    /// The file of a flavor's folder.
    pub fn path(home: &FlavorHome) -> PathBuf {
        home.dir.join(LAST_CHECK_FILE)
    }

    /// The kept answers of a flavor: a missing file, one that cannot be read, or one of another format,
    /// schema or flavor is no answer.
    pub fn read(home: &FlavorHome, flavor: &FlavorId) -> LastCheck {
        std::fs::read(LastCheck::path(home))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<LastCheck>(&bytes).ok())
            .filter(|last| {
                last.format == FORMAT && last.schema == SCHEMA && last.flavor == flavor.as_str()
            })
            .unwrap_or_else(|| LastCheck::new(flavor))
    }

    /// Takes in the addons of an online check: each one's answer replaces the kept one, kept when its state
    /// is one the cache cannot give again ([`is_kept`]) and dropped otherwise; an addon the check did not
    /// cover keeps its answer. Answers the sidecar no longer matches (another version or source, an addon
    /// gone) are dropped.
    pub fn record(&mut self, state: &State, addons: &[AddonUpdate]) {
        self.addons.retain(|name, kept| applies(state, name, kept));
        let checked_at = state::now_rfc3339();
        for addon in addons {
            self.addons
                .retain(|name, _| !name.eq_ignore_ascii_case(&addon.name));
            if is_kept(addon.state) {
                self.addons.insert(
                    addon.name.clone(),
                    KeptAnswer {
                        source: addon.source.clone(),
                        version: addon.version.clone(),
                        state: addon.state,
                        latest: kept_latest(addon),
                        detail: addon.detail.clone(),
                        checked_at: checked_at.clone(),
                    },
                );
            }
        }
    }

    /// Writes the file through a temporary file and a rename, creating the flavor's folder.
    pub fn write(&self, home: &FlavorHome) -> Result<()> {
        let path = LastCheck::path(home);
        write_at(&path, self)
    }

    /// The kept answers that still apply to the sidecar, as the check's addons: the entry's name, channel
    /// and pin, the kept state, release and reason, and no pick. An ignored addon has none: the check skips
    /// it, and its row says so.
    pub fn answers(&self, state: &State) -> Vec<AddonUpdate> {
        self.addons
            .iter()
            .filter(|(name, kept)| applies(state, name, kept))
            .filter_map(|(name, kept)| {
                let (name, entry) = state.entry(name)?;
                (!entry.ignored()).then(|| AddonUpdate {
                    name: name.to_string(),
                    source: entry.source.clone(),
                    version: entry.version.clone(),
                    channel: entry.channel(),
                    pinned: entry.pinned().map(str::to_string),
                    state: kept.state,
                    latest: kept.latest.clone(),
                    detail: kept.detail.clone(),
                    // A kept answer has no current release to order against (task 077).
                    order: None,
                    pick: None,
                })
            })
            .collect()
    }
}

/// Takes an online check's addons into the flavor's kept answers and writes the file: the kept answers as
/// they are, [`LastCheck::record`] over the sidecar `state`, then [`LastCheck::write`]. The window's check
/// and the command line's `update` call it after the check, before any update is installed; a failed write
/// is theirs to report as a warning.
pub fn record_check(
    home: &FlavorHome,
    flavor: &FlavorId,
    state: &State,
    addons: &[AddonUpdate],
) -> Result<()> {
    let mut last = LastCheck::read(home, flavor);
    last.record(state, addons);
    last.write(home)
}

/// The current release a kept answer keeps: none for a `curse:` entry (task 096), whose API's terms allow no
/// saving of its data, so the file holds no version CurseForge named; the source's own word for every other.
/// The state and the reason stay: they are the check's own words, a status and an error line.
fn kept_latest(addon: &AddonUpdate) -> Option<String> {
    let curse = matches!(
        SourceSpec::parse(&addon.source),
        Ok(spec) if spec.kind() == SourceKind::CurseForge
    );
    if curse { None } else { addon.latest.clone() }
}

/// Whether a kept answer still speaks of the addon the sidecar has: the same source and version.
fn applies(state: &State, name: &str, kept: &KeptAnswer) -> bool {
    state
        .entry(name)
        .is_some_and(|(_, entry)| entry.source == kept.source && entry.version == kept.version)
}

fn write_at(path: &Path, last: &LastCheck) -> Result<()> {
    let io = |source| Error::Io {
        context: "writing the last check's answers",
        path: path.to_path_buf(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    let text =
        serde_json::to_string_pretty(last).map_err(|source| io(std::io::Error::other(source)))?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text + "\n").map_err(io)?;
    std::fs::rename(&temporary, path).map_err(io)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::sources::Channel;
    use crate::state::Entry;

    fn entry(source: &str, version: &str) -> Entry {
        Entry {
            source: source.to_string(),
            version: version.to_string(),
            previous_version: None,
            folders: vec!["A".to_string()],
            files: BTreeMap::new(),
            installed_at: "2026-10-22T10:00:00Z".to_string(),
            nolib: None,
            pinned: None,
            ignored: false,
            channel: Channel::Stable,
            forever_build: None,
        }
    }

    fn checked(name: &str, version: &str, state: UpdateState) -> AddonUpdate {
        AddonUpdate {
            name: name.to_string(),
            source: "github:o/r".to_string(),
            version: version.to_string(),
            channel: Channel::Stable,
            pinned: None,
            state,
            latest: None,
            detail: Some("the repository is gone (404)".to_string()),
            order: None,
            pick: None,
        }
    }

    fn temp_home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-last-check-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_answers_the_cache_cannot_give_are_kept_until_the_next_check() {
        let home_dir = temp_home("kept");
        let home = FlavorHome::at(&home_dir, &FlavorId::Forever);
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert("A".into(), entry("github:o/r", "1.0"));
        state.addons.insert("B".into(), entry("github:o/r", "2.0"));
        assert!(LastCheck::read(&home, &FlavorId::Forever).addons.is_empty());

        let mut last = LastCheck::read(&home, &FlavorId::Forever);
        last.record(
            &state,
            &[
                checked("A", "1.0", UpdateState::SourceGone),
                checked("B", "2.0", UpdateState::UpToDate),
            ],
        );
        last.write(&home).expect("written");
        let read = LastCheck::read(&home, &FlavorId::Forever);
        assert_eq!(read.addons.len(), 1, "only the source gone is kept");
        let answers = read.answers(&state);
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].name, "A");
        assert_eq!(answers[0].state, UpdateState::SourceGone);
        assert!(answers[0].line().contains("the repository is gone (404)"));

        // A mark keeps it; an ignored addon shows its ignore instead.
        state.addons.get_mut("A").unwrap().pin(None);
        assert_eq!(read.answers(&state).len(), 1);
        state.addons.get_mut("A").unwrap().ignore();
        assert!(read.answers(&state).is_empty());
        state.addons.get_mut("A").unwrap().unignore();

        // A check of another addon leaves it; the next check of the addon replaces it.
        let mut last = read.clone();
        last.record(&state, &[checked("B", "2.0", UpdateState::Failed)]);
        assert_eq!(last.answers(&state).len(), 2);
        last.record(&state, &[checked("a", "1.0", UpdateState::UpToDate)]);
        assert_eq!(last.addons.keys().collect::<Vec<_>>(), ["B"]);

        // Another installed version drops the answer.
        state.addons.get_mut("B").unwrap().version = "2.1".into();
        assert!(last.answers(&state).is_empty());

        // Another flavor's file, or one that is not JSON, is no answer.
        let _ = std::fs::remove_dir_all(&home_dir);
        let mut other = LastCheck::new(&FlavorId::Retail);
        other.record(&state, &[checked("A", "1.0", UpdateState::Failed)]);
        other.write(&home).expect("written");
        assert!(LastCheck::read(&home, &FlavorId::Forever).addons.is_empty());
        std::fs::write(LastCheck::path(&home), "{ not json").unwrap();
        assert!(LastCheck::read(&home, &FlavorId::Forever).addons.is_empty());
        let _ = std::fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn a_curse_entrys_answer_keeps_no_version() {
        // Task 096: no data of the CurseForge API is saved, so a kept `curse:` answer names no release.
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert("A".into(), entry("curse:42", "1.0"));
        state.addons.insert("B".into(), entry("github:o/r", "2.0"));
        let mut curse = checked("A", "1.0", UpdateState::Failed);
        curse.source = "curse:42".to_string();
        curse.latest = Some("1.1".to_string());
        let mut github = checked("B", "2.0", UpdateState::Failed);
        github.latest = Some("2.1".to_string());
        let mut last = LastCheck::new(&FlavorId::Forever);
        last.record(&state, &[curse, github]);
        assert_eq!(last.addons["A"].latest, None);
        assert_eq!(last.addons["A"].state, UpdateState::Failed);
        assert_eq!(
            last.addons["A"].detail.as_deref(),
            Some("the repository is gone (404)")
        );
        assert_eq!(last.addons["B"].latest.as_deref(), Some("2.1"));
    }

    #[test]
    fn record_check_reads_records_and_writes_in_one_call() {
        // Task 087: the command line's `update` and the window share it.
        let home_dir = temp_home("record");
        let home = FlavorHome::at(&home_dir, &FlavorId::Forever);
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert("A".into(), entry("github:o/r", "1.0"));
        state.addons.insert("B".into(), entry("github:o/r", "2.0"));
        record_check(
            &home,
            &FlavorId::Forever,
            &state,
            &[checked("A", "1.0", UpdateState::SourceGone)],
        )
        .expect("written");
        record_check(
            &home,
            &FlavorId::Forever,
            &state,
            &[checked("B", "2.0", UpdateState::RateLimited)],
        )
        .expect("written");
        let read = LastCheck::read(&home, &FlavorId::Forever);
        assert_eq!(
            read.addons.keys().collect::<Vec<_>>(),
            ["A", "B"],
            "a check of one addon keeps the other's answer"
        );
        record_check(
            &home,
            &FlavorId::Forever,
            &state,
            &[checked("A", "1.0", UpdateState::UpToDate)],
        )
        .expect("written");
        let read = LastCheck::read(&home, &FlavorId::Forever);
        assert_eq!(read.addons.keys().collect::<Vec<_>>(), ["B"]);

        // A folder where the file goes: the write is an error for the caller to report.
        let _ = std::fs::remove_file(LastCheck::path(&home));
        std::fs::create_dir_all(LastCheck::path(&home)).unwrap();
        assert!(record_check(&home, &FlavorId::Forever, &state, &[]).is_err());
        let _ = std::fs::remove_dir_all(&home_dir);
    }
}
