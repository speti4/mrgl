//! The update check (task 006): every managed addon of a flavor's state sidecar against its source's current
//! release on the addon's own channel (task 021), one [`UpdateState`] per addon. An ignored addon is
//! skipped without a request; a pinned one is checked and left alone; a zip in hand and a linked folder
//! have no source to ask. An addon with an update carries the pick the check made, and
//! [`AddonUpdate::request`] hands it to the one install path of the engine
//! ([`Engine::install_from_source`], task 055) without a second request but the download. The check reads
//! and never writes (the GitHub source may write its own release list cache).

use serde::Serialize;

use crate::error::{Error, Result};
use crate::install::{Engine, SourceRequest, Wanted};
use crate::list_file::EntrySource;
use crate::scan;
use crate::sources::{Channel, Pick, Source, Target};
use crate::state::Entry;

/// Where one managed addon stands against its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateState {
    /// The source's current release is the installed version.
    UpToDate,
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
}

impl UpdateState {
    /// Every state, in the order a summary counts them.
    pub const ALL: [UpdateState; 8] = [
        UpdateState::UpdateAvailable,
        UpdateState::UpToDate,
        UpdateState::Pinned,
        UpdateState::Ignored,
        UpdateState::GuessOnly,
        UpdateState::SourceGone,
        UpdateState::NotChecked,
        UpdateState::Failed,
    ];

    /// The state in plain words, as a summary counts it (`12 up to date`).
    pub const fn label(self) -> &'static str {
        match self {
            UpdateState::UpToDate => "up to date",
            UpdateState::UpdateAvailable => "update available",
            UpdateState::GuessOnly => "guess only",
            UpdateState::Pinned => "pinned",
            UpdateState::Ignored => "ignored",
            UpdateState::SourceGone => "source gone",
            UpdateState::NotChecked => "not checked",
            UpdateState::Failed => "check failed",
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
    /// gone or a failed check, why an addon is not checked.
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
            UpdateState::GuessOnly => format!(
                "guess only: {detail}; `mrgl add {}` installs it anyway",
                self.source
            ),
            UpdateState::SourceGone | UpdateState::NotChecked | UpdateState::Failed => {
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
/// (by name, else by one of its folders). An addon whose folders were deleted by hand is left out with a
/// warning; a name the sidecar does not know is an error that says whether the folder exists. Nothing is
/// written.
pub fn check<S: Source + ?Sized>(
    engine: &Engine,
    source: &S,
    only: Option<&str>,
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
    let addons = entries
        .into_iter()
        .map(|(name, entry)| check_entry(engine, source, name, entry))
        .collect();
    Ok(UpdateCheck { addons, warnings })
}

/// One entry against its source, on its own channel.
fn check_entry<S: Source + ?Sized>(
    engine: &Engine,
    source: &S,
    name: &str,
    entry: &Entry,
) -> AddonUpdate {
    let mut addon = AddonUpdate {
        name: name.to_string(),
        source: entry.source.clone(),
        version: entry.version.clone(),
        channel: entry.channel,
        pinned: entry.pinned.clone(),
        state: UpdateState::Ignored,
        latest: None,
        detail: None,
        pick: None,
    };
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
            addon.state = if entry.pinned.is_some() {
                UpdateState::Pinned
            } else if pick.version == entry.version {
                UpdateState::UpToDate
            } else if pick.guess {
                addon.detail = Some(pick.reason.clone());
                UpdateState::GuessOnly
            } else {
                UpdateState::UpdateAvailable
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
            addon(UpdateState::UpdateAvailable, Some("1.1.0"), None),
            addon(UpdateState::GuessOnly, Some("2.0.0"), Some("a guess")),
            addon(UpdateState::Pinned, Some("1.1.0"), None),
            addon(UpdateState::Pinned, Some("1.0.0"), None),
            addon(UpdateState::Pinned, None, Some("offline")),
            addon(UpdateState::Ignored, None, None),
            addon(UpdateState::SourceGone, None, Some("no release in x")),
            addon(UpdateState::NotChecked, None, Some("a zip in hand")),
            addon(UpdateState::Failed, None, Some("cannot reach y")),
        ]
        .iter()
        .map(AddonUpdate::line)
        .collect();
        assert_eq!(
            lines,
            [
                "Thing  1.0.0  up to date",
                "Thing  1.0.0 -> 1.1.0  update available",
                "Thing  1.0.0 -> 2.0.0  guess only: a guess; `mrgl add github:Example/Thing` installs it anyway",
                "Thing  1.0.0  pinned at 1.0.0, 1.1.0 is out, left alone",
                "Thing  1.0.0  pinned at 1.0.0, up to date",
                "Thing  1.0.0  pinned at 1.0.0, not checked: offline",
                "Thing  1.0.0  ignored, not checked",
                "Thing  1.0.0  source gone: no release in x",
                "Thing  1.0.0  not checked: a zip in hand",
                "Thing  1.0.0  check failed: cannot reach y",
            ]
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
