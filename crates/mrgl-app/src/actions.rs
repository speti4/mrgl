//! The window's actions on a flavor (task 010): the update check, "update all" and a row's update, with a
//! progress event per addon and a cancel between addons, and the row menu's five marks (task 021). They
//! are plain functions over an engine and a source, so `tests/update_actions.rs` runs them on fixture
//! flavors without a window; the commands of [`crate::commands`] hand them the progress channel and the
//! run's cancel flag ([`UpdateRun`]).
//!
//! A flavor folder with the CurseForge app's `.curseclient` marker warns once per run, the first of the
//! answer's warnings, with the core's line of the command line's `update` (task 110 over task 098,
//! `adopt::curse_client_warning`): the check, "update all" and a row's update alike.
//!
//! The cancel rule: the flag is read before each addon of the check and before each install, never during
//! one. An install that has started runs to its end through the engine's own stage, swap and revert (task
//! 004), so a cancel never leaves an addon half-written: it is either updated or as it was.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mrgl_core::adopt;
use mrgl_core::install::{
    Engine, Folded, Mark, PickInstalled, PickStep, SourceInstalled, SourceRequest,
};
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
    /// The check stopped asking a source before its request limit (task 068); sent once per source that
    /// stopped, the other sources still asked (task 101).
    Stopped,
    /// The addon's update starts.
    Updating,
    /// Its asset is being downloaded.
    Downloading,
    /// Its asset goes through the install engine.
    Installing,
    /// The addon is updated.
    Updated,
    /// The one install path found the pick's version installed already and installed nothing (task 099):
    /// under the addon's own entry (another run updated it since the check), or under another entry of the
    /// same source, which the event's warning names; the addon stays as it is.
    InstalledAlready,
    /// An install earlier in the run took every folder of the addon's entry over and dropped the entry
    /// (task 114 over task 108): nothing is left to update and nothing is installed; the event says the
    /// core's line (`Folded::update_line`), which names the addon the entry joined.
    Folded,
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
    // Task 110: a folder the CurseForge app manages too, once per run and first, as `update` warns.
    adopt::curse_client_warning(&engine.flavor)
        .into_iter()
        .for_each(&mut warn);
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
            .unwrap_or_else(|_| State::new(&engine.flavor.id)); // coverage: os — the check read it a moment ago
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
        let mut messages = warning_messages(voice, warnings);
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

    // The addons this run updated, the ones the install path found installed already (task 099), the ones
    // an earlier install of the run folded into its addon (task 114), and the rows that read from the disk
    // again: the updated ones, the folded ones and the ones found under their own entry.
    let mut updated: BTreeSet<String> = BTreeSet::new();
    let mut installed_already: BTreeSet<String> = BTreeSet::new();
    let mut folded: BTreeSet<String> = BTreeSet::new();
    // Task 114 over task 108: the entries the installs of this run dropped, folded into the installed addon.
    let mut dropped: Vec<Folded> = Vec::new();
    let mut reread: BTreeSet<String> = BTreeSet::new();
    let mut left = 0;
    if *plan != Plan::Check {
        // An update of the check always has its request (the check's own pick of a source spec).
        let updates: Vec<(&AddonUpdate, SourceRequest)> = check
            .updates()
            .filter_map(|addon| addon.request().map(|request| (addon, request)))
            .collect();
        let count = updates.len();
        for (position, (addon, request)) in updates.iter().enumerate() {
            if cancelled() {
                left = count - position;
                break;
            }
            let folder = folder_of(&addon.name);
            let event = |phase: Phase, kind: Kind, plain: String| UpdateEvent {
                phase,
                folder: folder.clone(),
                index: Some(position + 1),
                total: Some(count),
                check: None,
                message: say(kind, plain),
            };
            // Task 114: an entry an earlier install of the run dropped has nothing left to update; the
            // command line's answer of task 108, in the core's words.
            if let Some(into) = dropped
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(&addon.name))
            {
                let (kind, line) = into.update_line(&folder_name);
                events(event(Phase::Folded, kind, line));
                folded.insert(addon.name.clone());
                reread.insert(addon.name.clone());
                continue;
            }
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
            let outcome = engine.install_from_source(source, request, &mut |step| {
                send_step(&step, &folder_name, &mut |phase, plain| {
                    events(event(phase, Kind::Progress, plain))
                });
            });
            match outcome {
                Ok(SourceInstalled {
                    outcome: PickInstalled::Installed(installed),
                    ..
                }) => {
                    installed.warnings.iter().cloned().for_each(&mut warn);
                    installed.client_warning.iter().cloned().for_each(&mut warn);
                    dropped.extend(installed.folded.iter().cloned());
                    // Task 079: the checks the command line's install summary reports, a missing
                    // dependency and an interface of another build, as warnings of the answer.
                    engine
                        .check_installed(&installed.folders)
                        .lines(&folder_name)
                        .into_iter()
                        .for_each(&mut warn);
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
                    reread.insert(addon.name.clone());
                }
                Ok(SourceInstalled {
                    outcome: PickInstalled::AlreadyInstalled(current),
                    ..
                }) => {
                    // Task 099: the command line's answer (task 089), in the core's words: the
                    // nothing-to-do line under the addon's own entry, a warning that names the other entry
                    // of the same source otherwise; counted as installed already, not as updated.
                    current.warnings.iter().cloned().for_each(&mut warn);
                    let (kind, line) =
                        current.update_line(&addon.name, &addon.version, &folder_name);
                    events(event(Phase::InstalledAlready, kind, line));
                    installed_already.insert(addon.name.clone());
                    if current.is_entry_of(&addon.name) {
                        reread.insert(addon.name.clone());
                    }
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
            installed_already: installed_already.len(),
            folded: folded.len(),
        },
    };
    let (kind, plain) = check.summary(of, &folder_name);
    let mut messages = warning_messages(voice, warnings);
    messages.push(say(kind, plain));
    // The rows take the check's answer, but an addon updated in this run (or found at the pick's version
    // under its own entry, or folded into another entry, whose entry is gone) reads from the disk and the
    // cache again: it is the release it was updated to now. An addon another entry of its source stood in for keeps the check's answer: it is as it was.
    let online: Vec<AddonUpdate> = check
        .addons
        .into_iter()
        .filter(|addon| !reread.contains(&addon.name))
        .collect();
    answer(&online, messages)
}

/// Sends the progress event of one step of the engine's install path, its phase and its plain line: the
/// download and the install. The run hands the engine the check's own pick, so the engine resolves nothing
/// and the resolving steps send no event.
fn send_step(step: &PickStep<'_>, folder_name: &str, send: &mut dyn FnMut(Phase, String)) {
    match step {
        PickStep::Downloading { pick, .. } => {
            send(Phase::Downloading, format!("downloading {}", pick.asset));
        }
        PickStep::Installing { pick, .. } => send(
            Phase::Installing,
            format!("installing {} into {folder_name}", pick.asset),
        ),
        PickStep::Resolving { .. } | PickStep::Picked { .. } => {}
    }
}

/// The warnings of an answer as its messages, in their order.
fn warning_messages(voice: Voice, warnings: Vec<String>) -> Vec<Message> {
    warnings
        .into_iter()
        .map(|line| Message::new(voice, Kind::Warning, line))
        .collect()
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
    let messages = match engine.apply_mark(folder, mark) {
        Ok(marked) => {
            let (kind, plain) = mark.line(&marked, &engine.flavor.folder_name());
            let mut messages = warning_messages(voice, marked.warnings);
            messages.push(Message::new(voice, kind, plain));
            messages
        }
        Err(error) => vec![Message::new(voice, Kind::Error, error)],
    };
    ActionAnswer {
        flavor: Some(list::flavor_view(engine.flavor.clone(), home, &[])),
        messages,
    }
}

/// The refusal of a run while another one goes.
pub fn busy_line() -> &'static str {
    "an update is running already; wait for it to end or cancel it"
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use mrgl_core::sources::{Pick, SourceSpec, Target};

    fn pick() -> Pick {
        Pick {
            source: "github:Example/One".to_string(),
            release: "v1.1.0".to_string(),
            version: "1.1.0".to_string(),
            published: None,
            prerelease: false,
            asset: "one-folder-1.1.0.zip".to_string(),
            url: "https://example.invalid/one-folder-1.1.0.zip".to_string(),
            size: None,
            md5: None,
            flavor: "forever".to_string(),
            interface: Some(16001),
            guess: false,
            reason: "release.json names forever".to_string(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: mrgl_core::forever::ForeverBuild::Yes,
            stale: None,
        }
    }

    #[test]
    fn the_download_and_the_install_are_events_and_the_resolving_steps_none() {
        let pick = pick();
        let path = Path::new("staging/one-folder-1.1.0.zip");
        let spec = SourceSpec::parse("github:Example/One").expect("a spec");
        let target = Target::new(mrgl_core::flavor::FlavorId::Forever);
        let steps = [
            PickStep::Resolving {
                spec: &spec,
                target: &target,
            },
            PickStep::Picked { pick: &pick },
            PickStep::Downloading {
                pick: &pick,
                to: path,
            },
            PickStep::Installing {
                pick: &pick,
                zip: path,
            },
        ];
        let mut sent = Vec::new();
        for step in &steps {
            send_step(step, "_forever_", &mut |phase, plain| {
                sent.push((phase, plain))
            });
        }
        assert_eq!(
            sent,
            [
                (
                    Phase::Downloading,
                    "downloading one-folder-1.1.0.zip".to_string()
                ),
                (
                    Phase::Installing,
                    "installing one-folder-1.1.0.zip into _forever_".to_string()
                ),
            ]
        );
    }
}
