//! The update check (task 006): every managed addon of a flavor's state sidecar against its source's current
//! release on the addon's own channel (task 021), one [`UpdateState`] per addon. An ignored addon is
//! skipped without a request; a pinned one is checked and left alone; a zip in hand and a linked folder
//! have no source to ask. An addon with an update carries the pick the check made, and
//! [`AddonUpdate::request`] hands it to the one install path of the engine
//! ([`Engine::install_from_source`], task 055) without a second request but the download. The check reads
//! and never writes (the GitHub source may write its own release list cache).
//!
//! Task 068: an installed version newer than the current release (a deleted release, a channel switch) is
//! [`UpdateState::InstalledNewer`], not an update; the order is the source's release list where it lists
//! both versions, else [`lenient_order`]. The check reports every addon through a [`CheckProgress`]
//! callback, the one the command line and the window share, and stops before a source's request limit
//! ([`Source::request_budget`]: the GitHub API's 60 requests an hour without a token): the addons it leaves
//! are [`UpdateState::RateLimited`] and [`UpdateCheck::stopped`] says when the limit resets.

use std::cmp::Ordering;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::install::{Engine, SourceRequest, Wanted};
use crate::list_file::EntrySource;
use crate::scan;
use crate::sources::{Channel, Pick, RequestBudget, Source, SourceSpec, Target};
use crate::state::Entry;

/// Where one managed addon stands against its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateState {
    /// The source's current release is the installed version.
    UpToDate,
    /// The installed version is newer than the source's current release on the addon's channel (a
    /// deleted release, a switch from the prerelease channel to the stable one): not an update, left alone.
    InstalledNewer,
    /// The source's current release is another version, picked from metadata: the update installs it.
    UpdateAvailable,
    /// The source's current release is another version, but its asset for the flavor is a guess by name
    /// (no `release.json` names the flavor): reported, not installed; `mrgl add <source>` installs it.
    GuessOnly,
    /// Pinned (task 021): checked, reported with the current release, left alone.
    Pinned,
    /// Ignored (task 021): skipped without a request.
    Ignored,
    /// The source has no release or no asset for the flavor any more, or the repository is gone (404).
    SourceGone,
    /// No source to ask: a zip in hand, a linked development folder, a source not handled yet.
    NotChecked,
    /// The source could not be asked: offline, a rate limit, a server error.
    Failed,
    /// Not asked: the check stopped before the source's request limit ([`UpdateCheck::stopped`]).
    RateLimited,
}

impl UpdateState {
    /// Every state, in the order a summary counts them.
    pub const ALL: [UpdateState; 10] = [
        UpdateState::UpdateAvailable,
        UpdateState::UpToDate,
        UpdateState::InstalledNewer,
        UpdateState::Pinned,
        UpdateState::Ignored,
        UpdateState::GuessOnly,
        UpdateState::SourceGone,
        UpdateState::NotChecked,
        UpdateState::RateLimited,
        UpdateState::Failed,
    ];

    /// The state in plain words, as a summary counts it (`12 up to date`).
    pub const fn label(self) -> &'static str {
        match self {
            UpdateState::UpToDate => "up to date",
            UpdateState::InstalledNewer => "newer than the release",
            UpdateState::UpdateAvailable => "update available",
            UpdateState::GuessOnly => "guess only",
            UpdateState::Pinned => "pinned",
            UpdateState::Ignored => "ignored",
            UpdateState::SourceGone => "source gone",
            UpdateState::NotChecked => "not checked",
            UpdateState::Failed => "check failed",
            UpdateState::RateLimited => "not checked (rate limit)",
        }
    }
}

/// One managed addon and where it stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddonUpdate {
    /// The sidecar's name.
    pub name: String,
    /// The sidecar's source spec.
    pub source: String,
    /// The installed version.
    pub version: String,
    /// The channel it was checked on.
    pub channel: Channel,
    /// The version it is pinned at.
    pub pinned: Option<String>,
    pub state: UpdateState,
    /// The version of the source's current release on the addon's channel, when the source answered.
    pub latest: Option<String>,
    /// The plain reason where the state needs one: the pick's reason for a guess, the error for a source
    /// gone or a failed check, why an addon is not checked, why an installed version is newer when the
    /// release list says.
    pub detail: Option<String>,
    /// The pick the check made, when the source answered.
    pub pick: Option<Pick>,
}

impl AddonUpdate {
    /// The request that installs this update through the one install path, with the pick the check
    /// made; `None` unless the state is [`UpdateState::UpdateAvailable`].
    pub fn request(&self) -> Option<SourceRequest> {
        if self.state != UpdateState::UpdateAvailable {
            return None;
        }
        let pick = self.pick.as_ref()?;
        let EntrySource::GitHub(spec) = EntrySource::of(&self.source) else {
            return None;
        };
        Some(SourceRequest {
            spec,
            wanted: Wanted::Pick(Box::new(pick.clone())),
            name: Some(self.name.clone()),
        })
    }

    /// The addon as a plain data line: the name, the version (with the current release after an arrow
    /// for an update), and the state with its reason.
    pub fn line(&self) -> String {
        let versions = match (&self.latest, self.state) {
            (Some(latest), UpdateState::UpdateAvailable | UpdateState::GuessOnly) => {
                format!("{} -> {latest}", self.version)
            }
            _ => self.version.clone(),
        };
        let detail = self.detail.as_deref().unwrap_or_default();
        let text = match self.state {
            UpdateState::Pinned => {
                let pinned = self.pinned.as_deref().unwrap_or_default();
                match &self.latest {
                    Some(latest) if *latest != self.version => {
                        format!("pinned at {pinned}, {latest} is out, left alone")
                    }
                    Some(_) => format!("pinned at {pinned}, up to date"),
                    None => format!("pinned at {pinned}, not checked: {detail}"),
                }
            }
            UpdateState::Ignored => "ignored, not checked".to_string(),
            UpdateState::InstalledNewer => {
                let latest = self.latest.as_deref().unwrap_or_default();
                match &self.detail {
                    Some(detail) => {
                        format!("newer than the current release {latest} ({detail}), left alone")
                    }
                    None => format!("newer than the current release {latest}, left alone"),
                }
            }
            UpdateState::GuessOnly => format!(
                "guess only: {detail}; `mrgl add {}` installs it anyway",
                self.source
            ),
            UpdateState::SourceGone
            | UpdateState::NotChecked
            | UpdateState::Failed
            | UpdateState::RateLimited => {
                format!("{}: {detail}", self.state.label())
            }
            UpdateState::UpToDate | UpdateState::UpdateAvailable => self.state.label().to_string(),
        };
        format!("{}  {versions}  {text}", self.name)
    }
}

/// The check of a flavor: every managed addon by name, and the plain warnings of reading the sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub addons: Vec<AddonUpdate>,
    pub warnings: Vec<String>,
    /// The check stopped before the source's request limit (task 068); `None` when it asked about every
    /// addon it had to.
    pub stopped: Option<BudgetStop>,
}

/// Why the check stopped before asking about every addon (task 068): the source's request limit was
/// reached in this run, so the addons left are not asked about rather than refused one by one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetStop {
    /// The host whose limit it is (`GitHub`).
    pub host: &'static str,
    /// The requests the limit allows an hour.
    pub limit: u32,
    /// The requests this run sent against it.
    pub sent: u32,
    /// When the limit resets, in Unix seconds, when the source said.
    pub reset: Option<i64>,
    /// How many addons the check left: [`UpdateState::RateLimited`].
    pub not_checked: usize,
    /// How to get a larger limit.
    pub hint: String,
}

impl BudgetStop {
    fn of(budget: RequestBudget, not_checked: usize) -> BudgetStop {
        BudgetStop {
            host: budget.host,
            limit: budget.limit,
            sent: budget.sent,
            reset: budget.reset,
            not_checked,
            hint: budget.hint,
        }
    }

    /// The one line of the stop: the limit, the addons left and when the limit resets, in local time.
    pub fn line(&self) -> String {
        let addons = match self.not_checked {
            1 => "1 addon".to_string(),
            count => format!("{count} addons"),
        };
        let reset = match self.reset {
            Some(reset) => format!(
                "the limit resets at {} (local time)",
                crate::sources::local_time_text(reset)
            ),
            None => "the limit resets within the hour".to_string(),
        };
        format!(
            "stopped the check before {}'s rate limit of {} requests an hour without a token: {addons} not checked, {reset}; {}",
            self.host, self.limit, self.hint
        )
    }
}

/// One step of [`check`], for the caller's progress line (task 068); the command line prints it, the
/// window of task 010 shares it. `index` counts from 1 to `total`, the addons of the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckProgress<'a> {
    /// The source is about to be asked about an addon (not sent for an ignored addon, one with no source
    /// to ask, or one the check leaves for the request limit).
    Checking {
        index: usize,
        total: usize,
        name: &'a str,
        source: &'a str,
    },
    /// The check stops before the source's request limit; every addon left that needs a request follows
    /// as [`UpdateState::RateLimited`].
    Stopped { stop: &'a BudgetStop },
    /// An addon's check is done, whatever its state; sent for every addon, in the sidecar's order.
    Checked {
        index: usize,
        total: usize,
        addon: &'a AddonUpdate,
    },
}

impl UpdateCheck {
    /// How many addons are in a state.
    pub fn count(&self, state: UpdateState) -> usize {
        self.addons
            .iter()
            .filter(|addon| addon.state == state)
            .count()
    }

    /// The addons with an update to install, in the sidecar's order.
    pub fn updates(&self) -> impl Iterator<Item = &AddonUpdate> {
        self.addons
            .iter()
            .filter(|addon| addon.state == UpdateState::UpdateAvailable)
    }
}

/// Checks the managed addons of the engine's flavor against their sources, or the one addon `only` names
/// (by name, else by one of its folders), reporting each through `progress`. An addon whose folders were
/// deleted by hand is left out with a warning; a name the sidecar does not know is an error that says
/// whether the folder exists. Before each request the source's request budget is read: with nothing left,
/// the check stops asking and the addons left are [`UpdateState::RateLimited`]. Nothing is written.
pub fn check<S: Source + ?Sized>(
    engine: &Engine,
    source: &S,
    only: Option<&str>,
    progress: &mut dyn FnMut(CheckProgress<'_>),
) -> Result<UpdateCheck> {
    let loaded = engine.load_state()?;
    let mut state = loaded.state;
    let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
    let scan = scan::scan_flavor(&engine.flavor)?;
    for name in state.reconcile(&scan) {
        warnings.push(format!(
            "{name} was deleted by hand since its install and is left out of the check"
        ));
    }
    let entries: Vec<(&String, &Entry)> = match only {
        Some(name) => {
            let Some((known, _)) = state.find(name) else {
                let flavor = engine
                    .flavor
                    .id
                    .friendly_name()
                    .unwrap_or(engine.flavor.id.as_str())
                    .to_string();
                let name = name.to_string();
                return Err(if engine.addons_dir().join(&name).is_dir() {
                    Error::NotManagedToMark {
                        name,
                        flavor,
                        action: "updated",
                    }
                } else {
                    Error::AddonNotFound { name, flavor }
                });
            };
            state
                .addons
                .iter()
                .filter(|(name, _)| *name == known)
                .collect()
        }
        None => state.addons.iter().collect(),
    };
    let total = entries.len();
    let mut addons = Vec::with_capacity(total);
    let mut stopped: Option<BudgetStop> = None;
    for (position, (name, entry)) in entries.iter().enumerate() {
        let index = position + 1;
        let asks = asks_source(entry);
        if asks
            && stopped.is_none()
            && let Some(budget) = source.request_budget()
            && budget.remaining == 0
        {
            let not_checked = entries[position..]
                .iter()
                .filter(|(_, entry)| asks_source(entry))
                .count();
            let stop = BudgetStop::of(budget, not_checked);
            progress(CheckProgress::Stopped { stop: &stop });
            stopped = Some(stop);
        }
        let addon = match &stopped {
            Some(stop) if asks => {
                let mut addon = unchecked(name, entry, UpdateState::RateLimited);
                addon.detail = Some(format!(
                    "the check stopped before {}'s rate limit",
                    stop.host
                ));
                addon
            }
            _ => {
                if asks {
                    progress(CheckProgress::Checking {
                        index,
                        total,
                        name,
                        source: &entry.source,
                    });
                }
                check_entry(engine, source, name, entry)
            }
        };
        progress(CheckProgress::Checked {
            index,
            total,
            addon: &addon,
        });
        addons.push(addon);
    }
    Ok(UpdateCheck {
        addons,
        warnings,
        stopped,
    })
}

/// Whether the check asks a source about an entry: not an ignored one, and only one of a source it
/// handles.
fn asks_source(entry: &Entry) -> bool {
    !entry.ignored && matches!(EntrySource::of(&entry.source), EntrySource::GitHub(_))
}

/// An entry as the check reports it before it asks anything: the sidecar's facts and a state.
fn unchecked(name: &str, entry: &Entry, state: UpdateState) -> AddonUpdate {
    AddonUpdate {
        name: name.to_string(),
        source: entry.source.clone(),
        version: entry.version.clone(),
        channel: entry.channel,
        pinned: entry.pinned.clone(),
        state,
        latest: None,
        detail: None,
        pick: None,
    }
}

/// One entry against its source, on its own channel.
fn check_entry<S: Source + ?Sized>(
    engine: &Engine,
    source: &S,
    name: &str,
    entry: &Entry,
) -> AddonUpdate {
    let mut addon = unchecked(name, entry, UpdateState::Ignored);
    if entry.ignored {
        return addon;
    }
    let spec = match EntrySource::of(&entry.source) {
        EntrySource::GitHub(spec) => spec,
        other => {
            addon.state = UpdateState::NotChecked;
            addon.detail = Some(
                match other {
                    EntrySource::Zip => "a zip in hand; a newer zip goes in with mrgl add <zip>",
                    EntrySource::Linked => "a linked development folder",
                    _ => "a source the update check does not handle yet",
                }
                .to_string(),
            );
            return addon;
        }
    };
    let target = Target::from_flavor(&engine.flavor).with_channel(entry.channel);
    match source.resolve(&spec, &target) {
        Ok(pick) => {
            addon.latest = Some(pick.version.clone());
            let order = if pick.version == entry.version {
                Some((Ordering::Equal, false))
            } else {
                installed_order(source, &spec, &entry.version, &pick.version)
            };
            addon.state = match order {
                _ if entry.pinned.is_some() => UpdateState::Pinned,
                Some((Ordering::Equal, _)) => UpdateState::UpToDate,
                Some((Ordering::Greater, delisted)) => {
                    if delisted {
                        addon.detail = Some(format!("{} is no longer listed", entry.version));
                    }
                    UpdateState::InstalledNewer
                }
                _ if pick.guess => {
                    addon.detail = Some(pick.reason.clone());
                    UpdateState::GuessOnly
                }
                _ => UpdateState::UpdateAvailable,
            };
            addon.pick = Some(pick);
        }
        Err(error) => {
            addon.state = if entry.pinned.is_some() {
                UpdateState::Pinned
            } else if source_gone(&error) {
                UpdateState::SourceGone
            } else {
                UpdateState::Failed
            };
            addon.detail = Some(error.to_string());
        }
    }
    addon
}

/// How the installed version stands to the current release's (task 068), and whether the source's release
/// list lacks the installed one while it lists releases: the list's order (newest first) where it lists
/// both, else [`lenient_order`]; `None` when neither can tell, and another version then counts as an
/// update.
fn installed_order<S: Source + ?Sized>(
    source: &S,
    spec: &SourceSpec,
    installed: &str,
    current: &str,
) -> Option<(Ordering, bool)> {
    let listed = source.listed_versions(spec);
    let position = |version: &str| listed.as_ref()?.iter().position(|listed| listed == version);
    match (position(installed), position(current)) {
        (Some(installed), Some(current)) => Some((current.cmp(&installed), false)),
        (None, _) => lenient_order(installed, current).map(|order| (order, listed.is_some())),
        (Some(_), None) => lenient_order(installed, current).map(|order| (order, false)),
    }
}

/// A lenient order of two version strings (task 068), for versions a release list does not order: a
/// leading `v` dropped, the part before the first `-` or `+` compared as its numbers (`1.2.10` after
/// `1.2.9`, `1.2` the same as `1.2.0`), and on equal numbers a version without a suffix after one with it
/// (`2.0.0` after `2.0.0-beta`), two suffixes compared by their words and numbers (`beta10` after `beta9`,
/// `rc1` after `beta2`). `None` when either has no number to compare.
pub fn lenient_order(a: &str, b: &str) -> Option<Ordering> {
    let ((a_core, a_suffix), (b_core, b_suffix)) = (split_suffix(a), split_suffix(b));
    let (mut a_numbers, mut b_numbers) = (numbers_of(a_core), numbers_of(b_core));
    if a_numbers.is_empty() || b_numbers.is_empty() {
        return None;
    }
    let length = a_numbers.len().max(b_numbers.len());
    a_numbers.resize(length, 0);
    b_numbers.resize(length, 0);
    let order = a_numbers.cmp(&b_numbers);
    if order != Ordering::Equal {
        return Some(order);
    }
    Some(match (a_suffix, b_suffix) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => suffix_tokens(a).cmp(&suffix_tokens(b)),
    })
}

/// A version without its leading `v`, split at its first `-` or `+` into the numbers and the suffix.
fn split_suffix(version: &str) -> (&str, Option<&str>) {
    let version = match version.strip_prefix(['v', 'V']) {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => rest,
        _ => version,
    };
    match version.split_once(['-', '+']) {
        Some((core, suffix)) if !suffix.is_empty() => (core, Some(suffix)),
        Some((core, _)) => (core, None),
        None => (version, None),
    }
}

/// The runs of digits of a version's numbers part, as numbers.
fn numbers_of(core: &str) -> Vec<u64> {
    core.split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(u64::MAX))
        .collect()
}

/// One piece of a version's suffix: a number sorts before a word, as in semantic versioning.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SuffixToken {
    Number(u64),
    Word(String),
}

/// A suffix as its runs of digits and of letters, the separators dropped (`beta.10` is `beta`, `10`).
fn suffix_tokens(suffix: &str) -> Vec<SuffixToken> {
    let mut tokens = Vec::new();
    let mut rest = suffix;
    while let Some(start) = rest.find(|c: char| c.is_ascii_alphanumeric()) {
        rest = &rest[start..];
        let digits = rest.starts_with(|c: char| c.is_ascii_digit());
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() || c.is_ascii_digit() != digits)
            .unwrap_or(rest.len());
        let (token, after) = rest.split_at(end);
        tokens.push(if digits {
            SuffixToken::Number(token.parse().unwrap_or(u64::MAX))
        } else {
            SuffixToken::Word(token.to_ascii_lowercase())
        });
        rest = after;
    }
    tokens
}

/// Whether an error says the source has nothing for the addon any more, rather than that it could not be
/// asked: no release, no asset for the flavor, or no such repository.
fn source_gone(error: &Error) -> bool {
    matches!(
        error,
        Error::NoRelease { .. }
            | Error::NoAssetForFlavor { .. }
            | Error::SourceStatus {
                status: 404 | 410,
                ..
            }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addon(state: UpdateState, latest: Option<&str>, detail: Option<&str>) -> AddonUpdate {
        AddonUpdate {
            name: "Thing".to_string(),
            source: "github:Example/Thing".to_string(),
            version: "1.0.0".to_string(),
            channel: Channel::Stable,
            pinned: (state == UpdateState::Pinned).then(|| "1.0.0".to_string()),
            state,
            latest: latest.map(str::to_string),
            detail: detail.map(str::to_string),
            pick: None,
        }
    }

    #[test]
    fn every_state_has_its_line() {
        let lines: Vec<String> = [
            addon(UpdateState::UpToDate, Some("1.0.0"), None),
            addon(UpdateState::InstalledNewer, Some("0.9.0"), None),
            addon(
                UpdateState::InstalledNewer,
                Some("0.9.0"),
                Some("1.0.0 is no longer listed"),
            ),
            addon(UpdateState::UpdateAvailable, Some("1.1.0"), None),
            addon(UpdateState::GuessOnly, Some("2.0.0"), Some("a guess")),
            addon(UpdateState::Pinned, Some("1.1.0"), None),
            addon(UpdateState::Pinned, Some("1.0.0"), None),
            addon(UpdateState::Pinned, None, Some("offline")),
            addon(UpdateState::Ignored, None, None),
            addon(UpdateState::SourceGone, None, Some("no release in x")),
            addon(UpdateState::NotChecked, None, Some("a zip in hand")),
            addon(UpdateState::Failed, None, Some("cannot reach y")),
            addon(
                UpdateState::RateLimited,
                None,
                Some("the check stopped before GitHub's rate limit"),
            ),
        ]
        .iter()
        .map(AddonUpdate::line)
        .collect();
        assert_eq!(
            lines,
            [
                "Thing  1.0.0  up to date",
                "Thing  1.0.0  newer than the current release 0.9.0, left alone",
                "Thing  1.0.0  newer than the current release 0.9.0 (1.0.0 is no longer listed), left alone",
                "Thing  1.0.0 -> 1.1.0  update available",
                "Thing  1.0.0 -> 2.0.0  guess only: a guess; `mrgl add github:Example/Thing` installs it anyway",
                "Thing  1.0.0  pinned at 1.0.0, 1.1.0 is out, left alone",
                "Thing  1.0.0  pinned at 1.0.0, up to date",
                "Thing  1.0.0  pinned at 1.0.0, not checked: offline",
                "Thing  1.0.0  ignored, not checked",
                "Thing  1.0.0  source gone: no release in x",
                "Thing  1.0.0  not checked: a zip in hand",
                "Thing  1.0.0  check failed: cannot reach y",
                "Thing  1.0.0  not checked (rate limit): the check stopped before GitHub's rate limit",
            ]
        );
    }

    #[test]
    fn the_lenient_order_reads_numbers_and_puts_a_suffix_first() {
        use Ordering::{Equal, Greater, Less};
        for (a, b, order) in [
            ("1.2.10", "1.2.9", Some(Greater)),
            ("v1.2.3", "1.2.4", Some(Less)),
            ("1.2", "1.2.0", Some(Equal)),
            ("2.0.0", "2.0.0-beta", Some(Greater)),
            ("2.0.0-beta2", "2.0.0", Some(Less)),
            ("2.0.0-beta10", "2.0.0-beta9", Some(Greater)),
            ("2.0.0-rc1", "2.0.0-beta2", Some(Greater)),
            ("2.0.0-Beta.1", "2.0.0-beta.1", Some(Equal)),
            ("1.1.0-beta", "1.0.0", Some(Greater)),
            ("r124", "r123", Some(Greater)),
            ("20261022", "20261021", Some(Greater)),
            ("nightly", "1.0.0", None),
            ("1.0.0", "", None),
        ] {
            assert_eq!(lenient_order(a, b), order, "{a} against {b}");
        }
    }

    #[test]
    fn the_stop_line_names_the_limit_the_addons_left_and_the_reset() {
        let mut stop = BudgetStop {
            host: "GitHub",
            limit: 60,
            sent: 60,
            reset: None,
            not_checked: 1,
            hint: "set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour".into(),
        };
        assert_eq!(
            stop.line(),
            "stopped the check before GitHub's rate limit of 60 requests an hour without a token: 1 addon not checked, the limit resets within the hour; set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour"
        );
        stop.reset = Some(1_792_598_400);
        stop.not_checked = 12;
        assert!(
            stop.line().contains(&format!(
                ": 12 addons not checked, the limit resets at {} (local time); ",
                crate::sources::local_time_text(1_792_598_400)
            )),
            "{}",
            stop.line()
        );
    }

    #[test]
    fn only_an_available_update_has_a_request() {
        assert!(
            addon(UpdateState::UpdateAvailable, Some("1.1.0"), None)
                .request()
                .is_none(),
            "no pick, no request"
        );
        assert!(
            addon(UpdateState::GuessOnly, Some("2.0.0"), None)
                .request()
                .is_none()
        );
        assert!(
            source_gone(&Error::NoRelease {
                spec: "github:o/r".into()
            }) && !source_gone(&Error::Network {
                url: "u".into(),
                reason: "offline".into()
            })
        );
    }
}
