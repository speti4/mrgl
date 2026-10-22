//! The window's actions on a flavor (task 010): the update check, "update all" and a row's update, with a
//! progress event per addon and a cancel between addons, and the row menu's five marks (task 021). They
//! are plain functions over an engine and a source, so `tests/update_actions.rs` runs them on fixture
//! flavors without a window; the commands of [`crate::commands`] hand them the progress channel and the
//! run's cancel flag ([`UpdateRun`]).
//!
//! The cancel rule: the flag is read before each addon of the check and before each install, never during
//! one. An install that has started runs to its end through the engine's own stage, swap and revert (task
//! 004), so a cancel never leaves an addon half-written: it is either updated or as it was.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mrgl_core::install::{Engine, Mark, PickInstalled, PickStep, SourceInstalled};
use mrgl_core::last_check::LastCheck;
use mrgl_core::sources::{Channel, Source};
use mrgl_core::state::{FlavorHome, State};
use mrgl_core::update::{self, AddonUpdate, CheckProgress, SummaryOf};
use mrgl_core::voice::{Kind, Voice};
use serde::Serialize;

use crate::commands::Message;
use crate::list::{self, CheckFact, FlavorView};

/// The one update run the window allows at a time, and its cancel flag: the state the commands share.
#[derive(Debug, Default)]
pub struct UpdateRun {
    running: AtomicBool,
    cancel: AtomicBool,
}

impl UpdateRun {
    /// Starts a run: `None` while another one runs. The guard ends the run, and clears a cancel, when it
    /// is dropped.
    pub fn start(&self) -> Option<RunGuard<'_>> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        self.cancel.store(false, Ordering::Release);
        Some(RunGuard(self))
    }

    /// Asks the run to stop before its next addon; `false` when no run is going.
    pub fn cancel(&self) -> bool {
        if !self.is_running() {
            return false;
        }
        self.cancel.store(true, Ordering::Release);
        true
    }

    /// Whether a run is going.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Whether the run was asked to stop.
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}

/// A run in progress; dropping it ends the run.
#[derive(Debug)]
pub struct RunGuard<'a>(&'a UpdateRun);

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.0.cancel.store(false, Ordering::Release);
        self.0.running.store(false, Ordering::Release);
    }
}

/// What a run does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Asks the sources about every managed addon and installs nothing ("check for updates").
    Check,
    /// The check, then every update it found ("update all").
    All,
    /// The check of the one addon a folder (or a name) names, then its update when it has one.
    One(String),
}

/// Where a progress event stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// The source is about to be asked about the addon.
    Checking,
    /// The addon's check is done; the event carries its [`CheckFact`].
    Checked,
    /// The check stopped before the source's request limit (task 068); sent once.
    Stopped,
    /// The addon's update starts.
    Updating,
    /// Its asset is being downloaded.
    Downloading,
    /// Its asset goes through the install engine.
    Installing,
    /// The addon is updated.
    Updated,
    /// The addon's update failed; the engine left it as it was.
    Failed,
}

/// One progress event of a run, sent to the web view as it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateEvent {
    pub phase: Phase,
    /// The row the event belongs to: the addon's first folder; `None` for the stop.
    pub folder: Option<String>,
    /// The addon's place among the addons of this phase, from 1, and their count.
    pub index: Option<usize>,
    pub total: Option<usize>,
    /// The check's answer for the addon, on [`Phase::Checked`].
    pub check: Option<CheckFact>,
    /// The line of the event in the voice: progress, the stop's warning, or a failed update's error.
    pub message: Message,
}

/// What a run or a mark answers: the flavor as it is now (its rows redrawn), and its messages in order,
/// the warnings first and the summary last.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActionAnswer {
    pub flavor: Option<FlavorView>,
    pub messages: Vec<Message>,
}

impl ActionAnswer {
    /// An answer of one message and no flavor: the run could not start.
    pub fn refused(message: Message) -> ActionAnswer {
        ActionAnswer {
            flavor: None,
            messages: vec![message],
        }
    }
}

/// Runs a plan on the engine's flavor: the check through `update::check_until` with a progress event per
/// addon, then, unless the plan is [`Plan::Check`], every update it found through the one install path
/// (`Engine::install_from_source` with the check's own pick, task 055). `cancelled` is read before each
/// addon of the check and before each install. `home` is the mrgl home folder the answer's rows are read
/// from.
pub fn run<S: Source + ?Sized>(
    engine: &Engine,
    source: &S,
    plan: &Plan,
    cancelled: &dyn Fn() -> bool,
    home: Option<&Path>,
    voice: Voice,
    events: &mut dyn FnMut(UpdateEvent),
) -> ActionAnswer {
    let folder_name = engine.flavor.folder_name();
    let say = |kind: Kind, plain: String| Message::new(voice, kind, plain);
    let answer = |online: &[AddonUpdate], messages: Vec<Message>| ActionAnswer {
        flavor: Some(list::flavor_view(engine.flavor.clone(), home, online)),
        messages,
    };
    if !engine.addons_dir().is_dir() {
        return answer(
            &[],
            vec![say(
                Kind::Nothing,
                list::no_addons_folder_line(&folder_name),
            )],
        );
    }

    // The row of an addon is its first folder, as the list draws it.
    let folders: BTreeMap<String, String> = engine
        .load_state()
        .map(|loaded| {
            loaded
                .state
                .addons
                .iter()
                .map(|(name, entry)| {
                    let folder = entry.folders.first().unwrap_or(name);
                    (name.clone(), folder.clone())
                })
                .collect()
        })
        .unwrap_or_default();
    let folder_of = |name: &str| folders.get(name).cloned();

    let only = match plan {
        Plan::One(folder) => Some(folder.as_str()),
        Plan::Check | Plan::All => None,
    };
    let mut total = 0;
    let check = update::check_until(
        engine,
        source,
        only,
        &mut |step| {
            let event = match step {
                CheckProgress::Checking {
                    index,
                    total,
                    name,
                    source,
                } => UpdateEvent {
                    phase: Phase::Checking,
                    folder: folder_of(name),
                    index: Some(index),
                    total: Some(total),
                    check: None,
                    message: say(
                        Kind::Progress,
                        format!("checking {name} against {source} ({index} of {total})"),
                    ),
                },
                CheckProgress::Stopped { stop } => UpdateEvent {
                    phase: Phase::Stopped,
                    folder: None,
                    index: None,
                    total: None,
                    check: None,
                    message: say(Kind::Warning, stop.line()),
                },
                CheckProgress::Checked {
                    index,
                    total: of,
                    addon,
                } => {
                    total = of;
                    UpdateEvent {
                        phase: Phase::Checked,
                        folder: folder_of(&addon.name),
                        index: Some(index),
                        total: Some(of),
                        check: Some(CheckFact::of(addon)),
                        message: say(Kind::Progress, addon.line()),
                    }
                }
            };
            events(event);
        },
        cancelled,
    );
    let check = match check {
        Ok(check) => check,
        Err(error) => return answer(&[], vec![say(Kind::Error, error.to_string())]),
    };

    let mut warnings: Vec<String> = Vec::new();
    let mut warn = |line: String| {
        if !warnings.contains(&line) {
            warnings.push(line);
        }
    };
    check.warnings.iter().cloned().for_each(&mut warn);
    // Task 041: a pick from the cached release list, offline, is a warning, as on the command line.
    check
        .addons
        .iter()
        .filter_map(|addon| addon.pick.as_ref()?.stale_warning())
        .for_each(&mut warn);

    // Task 078: what the check found that the cache cannot tell again is kept for the next list, the
    // next mark's redraw and the next start, until the next check of the addon replaces it.
    if let Some(home) = home {
        let flavor_home = FlavorHome::at(home, &engine.flavor.id);
        let state = engine
            .load_state()
            .map(|loaded| loaded.state)
            .unwrap_or_else(|_| State::new(&engine.flavor.id));
        let mut last = LastCheck::read(&flavor_home, &engine.flavor.id);
        last.record(&state, &check.addons);
        if let Err(error) = last.write(&flavor_home) {
            warn(format!(
                "the answers of this check are not kept for the next start: {error}"
            ));
        }
    }

    let checked = check.addons.len();
    if cancelled() {
        let what = match plan {
            Plan::Check => "check",
            Plan::All | Plan::One(_) => "update",
        };
        let mut messages: Vec<Message> = warnings
            .into_iter()
            .map(|line| say(Kind::Warning, line))
            .collect();
        let plain = if checked == 0 {
            format!("{what} cancelled before its first addon in {folder_name}; nothing updated")
        } else {
            format!(
                "{what} cancelled after checking {checked} of {total} addons in {folder_name}; nothing updated"
            )
        };
        messages.push(say(Kind::Nothing, plain));
        return answer(&check.addons, messages);
    }

    let mut updated: BTreeSet<String> = BTreeSet::new();
    let mut left = 0;
    if *plan != Plan::Check {
        let updates: Vec<&AddonUpdate> = check.updates().collect();
        let count = updates.len();
        for (position, addon) in updates.iter().enumerate() {
            if cancelled() {
                left = count - position;
                break;
            }
            let Some(request) = addon.request() else {
                continue;
            };
            let folder = folder_of(&addon.name);
            let event = |phase: Phase, kind: Kind, plain: String| UpdateEvent {
                phase,
                folder: folder.clone(),
                index: Some(position + 1),
                total: Some(count),
                check: None,
                message: say(kind, plain),
            };
            events(event(
                Phase::Updating,
                Kind::Progress,
                format!(
                    "updating {} from {} to {} ({} of {count})",
                    addon.name,
                    addon.version,
                    addon.latest.as_deref().unwrap_or_default(),
                    position + 1
                ),
            ));
            let outcome = engine.install_from_source(source, &request, &mut |step| match step {
                PickStep::Downloading { pick, .. } => events(event(
                    Phase::Downloading,
                    Kind::Progress,
                    format!("downloading {}", pick.asset),
                )),
                PickStep::Installing { pick, .. } => events(event(
                    Phase::Installing,
                    Kind::Progress,
                    format!("installing {} into {folder_name}", pick.asset),
                )),
                PickStep::Resolving { .. } | PickStep::Picked { .. } => {}
            });
            match outcome {
                Ok(SourceInstalled {
                    outcome: PickInstalled::Installed(installed),
                    ..
                }) => {
                    installed.warnings.iter().cloned().for_each(&mut warn);
                    installed.client_warning.iter().cloned().for_each(&mut warn);
                    let from = installed
                        .previous_version
                        .as_deref()
                        .unwrap_or(&addon.version);
                    events(event(
                        Phase::Updated,
                        Kind::Progress,
                        format!(
                            "updated {} from {from} to {} in {folder_name}",
                            installed.name, installed.version
                        ),
                    ));
                    updated.insert(addon.name.clone());
                }
                Ok(SourceInstalled {
                    outcome: PickInstalled::AlreadyInstalled(current),
                    ..
                }) => {
                    events(event(
                        Phase::Updated,
                        Kind::Progress,
                        format!(
                            "{} is at {} already in {folder_name}",
                            current.name, current.entry.version
                        ),
                    ));
                    updated.insert(addon.name.clone());
                }
                Err(error) => events(event(
                    Phase::Failed,
                    Kind::Error,
                    format!("{}: {error}", addon.name),
                )),
            }
        }
    }

    let of = match plan {
        Plan::Check => SummaryOf::Check,
        Plan::All | Plan::One(_) => SummaryOf::Update {
            updated: updated.len(),
            left,
        },
    };
    let (kind, plain) = check.summary(of, &folder_name);
    let mut messages: Vec<Message> = warnings
        .into_iter()
        .map(|line| say(Kind::Warning, line))
        .collect();
    messages.push(say(kind, plain));
    // The rows take the check's answer, but an addon updated in this run reads from the disk and the
    // cache again: it is the release it was updated to now.
    let online: Vec<AddonUpdate> = check
        .addons
        .into_iter()
        .filter(|addon| !updated.contains(&addon.name))
        .collect();
    answer(&online, messages)
}

/// The mark of a row menu item: `pin` (at `value`, else the installed version), `unpin`, `ignore`,
/// `unignore`, or `channel` with `stable` or `prerelease` as `value`.
pub fn mark_of(action: &str, value: Option<&str>) -> Result<Mark, String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    match action {
        "pin" => Ok(Mark::Pin(value.map(str::to_string))),
        "unpin" => Ok(Mark::Unpin),
        "ignore" => Ok(Mark::Ignore),
        "unignore" => Ok(Mark::Unignore),
        "channel" => {
            let Some(value) = value else {
                return Err("a channel change needs stable or prerelease".to_string());
            };
            Channel::parse(value)
                .map(Mark::Channel)
                .ok_or_else(|| format!("unknown channel {value}: stable or prerelease"))
        }
        other => Err(format!(
            "unknown row action {other}: pin, unpin, ignore, unignore or channel"
        )),
    }
}

/// One mark on a managed addon of the engine's flavor, by its folder or its name, through the engine's
/// mark calls (task 021); the answer carries the flavor with its rows as they are now and the mark's line.
pub fn mark(
    engine: &Engine,
    folder: &str,
    mark: &Mark,
    home: Option<&Path>,
    voice: Voice,
) -> ActionAnswer {
    let mut messages = Vec::new();
    match engine.apply_mark(folder, mark) {
        Ok(marked) => {
            for warning in &marked.warnings {
                messages.push(Message::new(voice, Kind::Warning, warning));
            }
            let (kind, plain) = mark.line(&marked, &engine.flavor.folder_name());
            messages.push(Message::new(voice, kind, plain));
        }
        Err(error) => messages.push(Message::new(voice, Kind::Error, error)),
    }
    ActionAnswer {
        flavor: Some(list::flavor_view(engine.flavor.clone(), home, &[])),
        messages,
    }
}

/// The refusal of a run while another one goes.
pub fn busy_line() -> &'static str {
    "an update is running already; wait for it to end or cancel it"
}
