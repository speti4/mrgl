//! The window's update actions and marks (task 010) on a made-up Forever flavor: four addons installed
//! through the engine from the fixture zips, a stub source that answers from those zips, and the run's
//! events, answer and disk checked without a window. Nothing touches the network or a real game folder.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use mrgl_app_lib::actions::{self, ActionAnswer, Phase, Plan, UpdateEvent};
use mrgl_app_lib::commands::Message;
use mrgl_app_lib::list::{self, FlavorView};
use mrgl_core::error::Result;
use mrgl_core::flavor;
use mrgl_core::forever::ForeverBuild;
use mrgl_core::install::{Engine, Mark, Metadata};
use mrgl_core::sources::{Pick, RequestBudget, Source, SourceKind, SourceSpec, Stale, Target};
use mrgl_core::update::UpdateState;
use mrgl_core::voice::{Kind, Voice};

const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Tags!STRING:0|Product!STRING:0\n\
eu|1|e8c4ef4e67bd1d94d820254984258fa8|1.60.1.70245||wowf\n";

fn zips() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/zips")
}

/// A temporary game root with a Forever flavor and a mrgl home folder, removed at the end.
struct Game {
    dir: PathBuf,
}

impl Game {
    /// The flavor with four managed addons: `OneFolder` 1.0.0, `RootToc` 3.0.0, `SeveralFolders` 2.0.0
    /// (three folders) and `Wrapped` 4.0.0, each from its own `github:` source.
    fn new(name: &str) -> Game {
        let game = Game::bare(name);
        std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
        let engine = game.engine();
        for (zip, source, version) in [
            ("one-folder-1.0.0.zip", "github:Example/One", "1.0.0"),
            ("root-toc-3.0.0.zip", "github:Example/Broken", "3.0.0"),
            (
                "several-folders-2.0.0.zip",
                "github:Example/Several",
                "2.0.0",
            ),
            ("wrapped-4.0.0.zip", "github:Example/Wrapped", "4.0.0"),
        ] {
            engine
                .install(
                    &zips().join(zip),
                    &Metadata {
                        source: source.to_string(),
                        version: Some(version.to_string()),
                        name: None,
                        nolib: None,
                    },
                )
                .unwrap_or_else(|error| panic!("installing {zip}: {error}"));
        }
        game
    }

    /// The flavor folder alone: no `Interface\AddOns` yet, as before the client's first start.
    fn bare(name: &str) -> Game {
        let dir =
            std::env::temp_dir().join(format!("mrgl-app-actions-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("World of Warcraft");
        std::fs::create_dir_all(root.join("_forever_")).expect("the flavor folder");
        std::fs::write(root.join(".build.info"), BUILD_INFO).expect(".build.info");
        std::fs::write(
            root.join("_forever_/.flavor.info"),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect(".flavor.info");
        Game { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn flavor_folder(&self) -> PathBuf {
        self.dir.join("World of Warcraft/_forever_")
    }

    fn addons(&self) -> PathBuf {
        self.flavor_folder().join("Interface/AddOns")
    }

    fn engine(&self) -> Engine {
        let flavor = flavor::read_flavor(&self.flavor_folder()).expect("the flavor");
        Engine::with_home(flavor, &self.home())
    }

    fn toc_version(&self, folder: &str) -> String {
        let text =
            std::fs::read_to_string(self.addons().join(folder).join(format!("{folder}.toc")))
                .unwrap_or_else(|error| panic!("{folder}.toc: {error}"));
        text.lines()
            .find_map(|line| line.strip_prefix("## Version: "))
            .expect("a version line")
            .trim()
            .to_string()
    }

    fn sidecar_version(&self, name: &str) -> String {
        let state = self.engine().load_state().expect("the sidecar").state;
        state.addons[name].version.clone()
    }

    fn staging_is_empty(&self) -> bool {
        let staging = self.home().join("wowf/staging");
        std::fs::read_dir(&staging).map_or(true, |mut entries| entries.next().is_none())
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A source over the fixture zips: each spec has a current release and the zip behind it. `github:Example/
/// Broken` serves a zip with no addon, so its update fails in the engine's stage. After `budget` asks the
/// source has no requests left, as the GitHub API without a token (task 068).
struct Stub {
    asked: Cell<u32>,
    /// The specs the source was asked about, in order (task 116).
    specs: RefCell<Vec<String>>,
    budget: Option<u32>,
    /// A spec whose repository is gone: a 404, as the GitHub source says it.
    gone: Option<&'static str>,
    /// Every pick comes from the cache, offline (task 041): each carries the stale warning.
    stale: bool,
}

impl Stub {
    fn new() -> Stub {
        Stub {
            asked: Cell::new(0),
            specs: RefCell::new(Vec::new()),
            budget: None,
            gone: None,
            stale: false,
        }
    }

    fn release(spec: &str) -> Option<(&'static str, &'static str)> {
        match spec {
            "github:Example/One" => Some(("1.1.0", "one-folder-1.1.0.zip")),
            "github:Example/Broken" => Some(("3.1.0", "no-addon.zip")),
            "github:Example/Several" => Some(("2.1.0", "several-folders-2.1.0.zip")),
            "github:Example/Wrapped" => Some(("4.0.0", "wrapped-4.0.0.zip")),
            "github:Example/Needy" => Some(("1.0.0", "needs-library-1.0.0.zip")),
            "github:Example/Stale" => Some(("1.0.0", "stale-interface-1.0.0.zip")),
            "github:Example/Twice" => Some(("1.1.0", "one-folder-1.1.0.zip")),
            "github:Example/Folded" => Some(("2.1.0", "several-folders-2.1.0.zip")),
            "wago:MrglStale" => Some(("1.0.0", "stale-interface-1.0.0.zip")),
            _ => None,
        }
    }
}

impl Source for Stub {
    fn resolve(&self, spec: &SourceSpec, _target: &Target) -> Result<Pick> {
        self.asked.set(self.asked.get() + 1);
        let spec = spec.to_string();
        self.specs.borrow_mut().push(spec.clone());
        // Task 115: a project whose author allows no download outside CurseForge.
        if spec == "curse:4244" {
            return Err(mrgl_core::Error::DistributionOff {
                spec,
                name: " (Mrgl No Distribution)".to_string(),
                page: "https://www.curseforge.com/wow/addons/mrgl-no-distribution".to_string(),
            });
        }
        if self.gone == Some(spec.as_str()) {
            return Err(mrgl_core::Error::SourceStatus {
                host: "GitHub",
                status: 404,
                url: format!("https://api.github.com/repos/{}/releases", &spec[7..]),
                hint: "no such repository, or it is private".to_string(),
            });
        }
        let (version, zip) = Stub::release(&spec)
            .ok_or_else(|| mrgl_core::Error::NoRelease { spec: spec.clone() })?;
        Ok(Pick {
            source: spec,
            release: format!("v{version}"),
            version: version.to_string(),
            published: None,
            prerelease: false,
            asset: zip.to_string(),
            url: format!("https://example.invalid/{zip}"),
            size: None,
            md5: None,
            flavor: "forever".to_string(),
            interface: Some(16001),
            guess: false,
            reason: "release.json names forever".to_string(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: ForeverBuild::Yes,
            stale: self.stale.then(|| Stale {
                checked_at: None,
                reason: "the network is off".to_string(),
            }),
        })
    }

    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("the download folder");
        std::fs::copy(zips().join(&pick.asset), to).expect("the fixture zip");
        Ok(())
    }

    /// The budget is GitHub's: a spec of another source has none (task 101).
    fn request_budget_for(&self, spec: &SourceSpec) -> Option<RequestBudget> {
        if spec.kind() != SourceKind::GitHub {
            return None;
        }
        self.request_budget()
    }

    fn request_budget(&self) -> Option<RequestBudget> {
        let limit = self.budget?;
        Some(RequestBudget {
            host: "GitHub",
            limit: 60,
            sent: self.asked.get(),
            remaining: limit.saturating_sub(self.asked.get()),
            reset: None,
            hint: "set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour"
                .to_string(),
            window: " an hour without a token",
            resets: "within the hour",
        })
    }
}

/// A run with the events it sent; `cancel_at` sets the cancel flag on the first event it matches.
fn run_with(
    game: &Game,
    source: &Stub,
    plan: Plan,
    cancel_at: Option<&dyn Fn(&UpdateEvent) -> bool>,
) -> (ActionAnswer, Vec<UpdateEvent>) {
    let cancelled = Cell::new(false);
    let events = RefCell::new(Vec::new());
    let engine = game.engine();
    let home = game.home();
    let answer = actions::run(
        &engine,
        source,
        &plan,
        &|| cancelled.get(),
        Some(&home),
        Voice::new(true),
        &mut |event| {
            if cancel_at.is_some_and(|at| at(&event)) {
                cancelled.set(true);
            }
            events.borrow_mut().push(event);
        },
    );
    (answer, events.into_inner())
}

fn phases(events: &[UpdateEvent]) -> Vec<(Phase, Option<&str>)> {
    events
        .iter()
        .map(|event| (event.phase, event.folder.as_deref()))
        .collect()
}

fn last_plain(answer: &ActionAnswer) -> &str {
    &answer.messages.last().expect("a summary").plain
}

#[test]
fn update_all_installs_every_update_reports_each_addon_and_sums_up() {
    let game = Game::new("all");
    let (answer, events) = run_with(&game, &Stub::new(), Plan::All, None);
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("OneFolder")),
            (Phase::Checked, Some("OneFolder")),
            (Phase::Checking, Some("RootToc")),
            (Phase::Checked, Some("RootToc")),
            (Phase::Checking, Some("SeveralFolders")),
            (Phase::Checked, Some("SeveralFolders")),
            (Phase::Checking, Some("Wrapped")),
            (Phase::Checked, Some("Wrapped")),
            (Phase::Updating, Some("OneFolder")),
            (Phase::Downloading, Some("OneFolder")),
            (Phase::Installing, Some("OneFolder")),
            (Phase::Updated, Some("OneFolder")),
            (Phase::Updating, Some("RootToc")),
            (Phase::Downloading, Some("RootToc")),
            (Phase::Installing, Some("RootToc")),
            (Phase::Failed, Some("RootToc")),
            (Phase::Updating, Some("SeveralFolders")),
            (Phase::Downloading, Some("SeveralFolders")),
            (Phase::Installing, Some("SeveralFolders")),
            (Phase::Updated, Some("SeveralFolders")),
        ]
    );
    let plain: Vec<&str> = events
        .iter()
        .map(|event| event.message.plain.as_str())
        .collect();
    assert_eq!(
        plain[0],
        "checking OneFolder against github:Example/One (1 of 4)"
    );
    assert_eq!(plain[1], "OneFolder  1.0.0 -> 1.1.0  update available");
    assert_eq!(plain[7], "Wrapped  4.0.0  up to date");
    assert_eq!(plain[8], "updating OneFolder from 1.0.0 to 1.1.0 (1 of 3)");
    assert_eq!(plain[9], "downloading one-folder-1.1.0.zip");
    assert_eq!(plain[10], "installing one-folder-1.1.0.zip into _forever_");
    assert_eq!(
        plain[11],
        "updated OneFolder from 1.0.0 to 1.1.0 in _forever_"
    );
    assert!(
        plain[15].starts_with("RootToc: ") && events[15].message.kind == "error",
        "{}",
        plain[15]
    );
    let checked = events[1].check.as_ref().expect("the check's answer");
    assert_eq!(
        (checked.label, checked.latest.as_deref()),
        ("update available", Some("1.1.0"))
    );
    for event in &events {
        assert_eq!(event.message.murloc, "", "a serious voice");
        if event.phase != Phase::Failed {
            assert_eq!(event.message.kind, "progress", "{}", event.message.plain);
        }
    }

    let summary = answer.messages.last().expect("a summary");
    assert_eq!(summary.kind, "warning", "one update failed");
    assert_eq!(summary.plain, "2 addons updated, 1 failed, 1 up to date");

    assert_eq!(game.toc_version("OneFolder"), "1.1.0");
    assert_eq!(game.toc_version("SeveralFolders"), "2.1.0");
    assert!(
        !game.addons().join("SeveralFolders_Data").exists(),
        "the folder 2.1.0 no longer ships is gone"
    );
    assert_eq!(
        game.toc_version("RootToc"),
        "3.0.0",
        "the failed update left it as it was"
    );
    assert_eq!(game.sidecar_version("OneFolder"), "1.1.0");
    assert_eq!(game.sidecar_version("RootToc"), "3.0.0");
    assert!(game.staging_is_empty());

    // The answer's rows: the updated addons as they are now, the failed one with the check's update.
    let flavor = answer.flavor.as_ref().expect("the flavor");
    let row = |folder: &str| {
        flavor
            .addons
            .iter()
            .find(|row| row.folder == folder)
            .unwrap_or_else(|| panic!("no row {folder}"))
    };
    assert_eq!(row("OneFolder").version.as_deref(), Some("1.1.0"));
    assert_eq!(row("OneFolder").update, None);
    assert_eq!(
        row("RootToc")
            .update
            .as_ref()
            .map(|update| update.version.as_str()),
        Some("3.1.0")
    );
    assert_eq!(
        row("Wrapped").check.as_ref().map(|check| check.label),
        Some("up to date")
    );
}

#[test]
fn a_check_installs_nothing_and_says_what_is_available() {
    let game = Game::new("check");
    let (answer, events) = run_with(&game, &Stub::new(), Plan::Check, None);
    assert!(
        events
            .iter()
            .all(|event| matches!(event.phase, Phase::Checking | Phase::Checked)),
        "{:?}",
        phases(&events)
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        (
            "success",
            "checked _forever_: 3 updates available, 1 up to date"
        )
    );
    assert_eq!(game.toc_version("OneFolder"), "1.0.0");
    assert_eq!(game.sidecar_version("SeveralFolders"), "2.0.0");
    let flavor = answer.flavor.expect("the flavor");
    let updates: Vec<&str> = flavor
        .addons
        .iter()
        .filter(|row| row.update.is_some())
        .map(|row| row.folder.as_str())
        .collect();
    assert_eq!(updates, ["OneFolder", "RootToc", "SeveralFolders"]);
}

#[test]
fn a_cancel_during_an_install_lets_that_addon_finish_and_starts_no_other() {
    let game = Game::new("cancel-install");
    let at_download: &dyn Fn(&UpdateEvent) -> bool = &|event| event.phase == Phase::Downloading;
    let (answer, events) = run_with(&game, &Stub::new(), Plan::All, Some(at_download));
    let after_check: Vec<(Phase, Option<&str>)> = phases(&events).into_iter().skip(8).collect();
    assert_eq!(
        after_check,
        [
            (Phase::Updating, Some("OneFolder")),
            (Phase::Downloading, Some("OneFolder")),
            (Phase::Installing, Some("OneFolder")),
            (Phase::Updated, Some("OneFolder")),
        ],
        "the addon in flight finishes, no other one starts"
    );
    assert_eq!(
        last_plain(&answer),
        "1 addon updated, 2 not updated (cancelled), 1 up to date"
    );
    assert_eq!(answer.messages.last().expect("a summary").kind, "warning");
    assert_eq!(
        game.toc_version("OneFolder"),
        "1.1.0",
        "the addon in flight is whole"
    );
    assert_eq!(game.sidecar_version("OneFolder"), "1.1.0");
    assert_eq!(
        game.toc_version("SeveralFolders"),
        "2.0.0",
        "the next one never started"
    );
    assert!(game.addons().join("SeveralFolders_Data").exists());
    assert_eq!(game.sidecar_version("SeveralFolders"), "2.0.0");
    assert!(game.staging_is_empty(), "no half-written download is left");
}

#[test]
fn a_cancel_during_the_check_stops_it_and_installs_nothing() {
    let game = Game::new("cancel-check");
    let at_first: &dyn Fn(&UpdateEvent) -> bool = &|event| event.phase == Phase::Checked;
    let (answer, events) = run_with(&game, &Stub::new(), Plan::All, Some(at_first));
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("OneFolder")),
            (Phase::Checked, Some("OneFolder"))
        ]
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        (
            "nothing",
            "update cancelled after checking 1 of 4 addons in _forever_; nothing updated"
        )
    );
    assert_eq!(game.toc_version("OneFolder"), "1.0.0");

    // A cancel that comes before the first addon: nothing is asked.
    let source = Stub::new();
    let engine = game.engine();
    let answer = actions::run(
        &engine,
        &source,
        &Plan::Check,
        &|| true,
        Some(&game.home()),
        Voice::new(true),
        &mut |_| panic!("no event before the first addon"),
    );
    assert_eq!(source.asked.get(), 0);
    assert_eq!(
        last_plain(&answer),
        "check cancelled before its first addon in _forever_; nothing updated"
    );
}

#[test]
fn a_row_update_checks_and_updates_that_addon_alone() {
    let game = Game::new("one");
    let source = Stub::new();
    let (answer, events) = run_with(&game, &source, Plan::One("SeveralFolders".into()), None);
    assert_eq!(source.asked.get(), 1, "one addon asked about");
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("SeveralFolders")),
            (Phase::Checked, Some("SeveralFolders")),
            (Phase::Updating, Some("SeveralFolders")),
            (Phase::Downloading, Some("SeveralFolders")),
            (Phase::Installing, Some("SeveralFolders")),
            (Phase::Updated, Some("SeveralFolders")),
        ]
    );
    assert_eq!(last_plain(&answer), "1 addon updated");
    assert_eq!(game.toc_version("SeveralFolders"), "2.1.0");
    assert_eq!(game.toc_version("OneFolder"), "1.0.0");

    let (answer, _) = run_with(&game, &source, Plan::One("Wrapped".into()), None);
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        ("nothing", "nothing updated: 1 up to date")
    );
}

#[test]
fn the_budget_stop_is_one_event_and_the_summary_counts_the_rest() {
    let game = Game::new("budget");
    let source = Stub {
        budget: Some(2),
        ..Stub::new()
    };
    let (answer, events) = run_with(&game, &source, Plan::Check, None);
    let stops: Vec<&UpdateEvent> = events
        .iter()
        .filter(|event| event.phase == Phase::Stopped)
        .collect();
    assert_eq!(stops.len(), 1, "the stop is said once");
    assert_eq!(stops[0].message.kind, "warning");
    assert!(
        stops[0].message.plain.starts_with(
            "stopped asking GitHub before its rate limit of 60 requests an hour without a token: 2 addons not checked"
        ),
        "{}",
        stops[0].message.plain
    );
    assert!(
        answer
            .messages
            .iter()
            .all(|message| !message.plain.starts_with("stopped asking")),
        "the answer does not repeat the stop"
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        (
            "warning",
            "checked _forever_: 2 updates available, 2 not checked (GitHub's rate limit)"
        )
    );

    // The addons the stop left keep their answer for the next list (task 078).
    let flavor = flavor::read_flavor(&game.flavor_folder()).expect("the flavor");
    let view = list::flavor_view(flavor, Some(&game.home()), &[]);
    let left = view
        .addons
        .iter()
        .filter_map(|row| row.check.as_ref())
        .filter(|check| check.state == UpdateState::RateLimited)
        .count();
    assert_eq!(left, 2);
}

#[test]
fn the_budget_stop_of_one_source_leaves_the_other_source_asked() {
    let game = Game::new("budget-per-source");
    game.engine()
        .install(
            &zips().join("stale-interface-1.0.0.zip"),
            &Metadata {
                source: "wago:MrglStale".to_string(),
                version: Some("1.0.0".to_string()),
                name: None,
                nolib: None,
            },
        )
        .expect("the Wago addon");
    let source = Stub {
        budget: Some(1),
        ..Stub::new()
    };
    let (answer, events) = run_with(&game, &source, Plan::Check, None);
    let stops: Vec<&str> = events
        .iter()
        .filter(|event| event.phase == Phase::Stopped)
        .map(|event| event.message.plain.as_str())
        .collect();
    assert_eq!(stops.len(), 1, "{stops:?}");
    assert!(
        stops[0].starts_with(
            "stopped asking GitHub before its rate limit of 60 requests an hour without a token: 3 addons not checked"
        ),
        "{}",
        stops[0]
    );
    let checked: Vec<(&str, UpdateState)> = events
        .iter()
        .filter_map(|event| Some((event.folder.as_deref()?, event.check.as_ref()?.state)))
        .collect();
    assert_eq!(
        checked,
        [
            ("OneFolder", UpdateState::UpdateAvailable),
            ("RootToc", UpdateState::RateLimited),
            ("SeveralFolders", UpdateState::RateLimited),
            ("StaleInterface", UpdateState::UpToDate),
            ("Wrapped", UpdateState::RateLimited),
        ],
        "the Wago addon after the stop is asked"
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        (
            "warning",
            "checked _forever_: 1 update available, 1 up to date, 3 not checked (GitHub's rate limit)"
        )
    );
}

#[test]
fn a_flavor_without_an_addons_folder_says_the_client_was_never_started() {
    let game = Game::bare("never-started");
    let source = Stub::new();
    let (answer, events) = run_with(&game, &source, Plan::All, None);
    assert!(events.is_empty());
    assert_eq!(source.asked.get(), 0);
    let summary = answer.messages.last().expect("a message");
    assert_eq!(summary.kind, "nothing");
    assert!(
        summary
            .plain
            .starts_with("no Interface\\AddOns folder in _forever_: the client has not been started for this flavor yet"),
        "{}",
        summary.plain
    );
    let flavor = answer.flavor.expect("the flavor");
    assert!(!flavor.addons_folder && flavor.addons.is_empty());
}

#[test]
fn the_row_menu_marks_go_through_the_engine() {
    let game = Game::new("marks");
    let engine = game.engine();
    let home = game.home();
    let mark = |folder: &str, action: &str, value: Option<&str>| {
        let mark = actions::mark_of(action, value).expect("a mark");
        actions::mark(&engine, folder, &mark, Some(&home), Voice::new(true))
    };
    let row_of = |answer: &ActionAnswer, folder: &str| {
        answer
            .flavor
            .as_ref()
            .expect("the flavor")
            .addons
            .iter()
            .find(|row| row.folder == folder)
            .cloned()
            .expect("the row")
    };

    let answer = mark("SeveralFolders", "pin", None);
    assert_eq!(
        (answer.messages[0].kind, answer.messages[0].plain.as_str()),
        ("success", "pinned SeveralFolders at 2.0.0 in _forever_")
    );
    assert_eq!(
        row_of(&answer, "SeveralFolders").pinned.as_deref(),
        Some("2.0.0")
    );
    let answer = mark("SeveralFolders", "pin", None);
    assert_eq!(answer.messages[0].kind, "nothing", "pinned already");
    let answer = mark("SeveralFolders", "unpin", None);
    assert_eq!(
        answer.messages[0].plain,
        "unpinned SeveralFolders in _forever_"
    );
    assert_eq!(row_of(&answer, "SeveralFolders").pinned, None);

    let answer = mark("OneFolder", "ignore", None);
    assert_eq!(answer.messages[0].plain, "ignoring OneFolder in _forever_");
    assert!(row_of(&answer, "OneFolder").ignored);
    let answer = mark("OneFolder", "unignore", None);
    assert_eq!(
        answer.messages[0].plain,
        "OneFolder is no longer ignored in _forever_"
    );

    let answer = mark("Wrapped", "channel", Some("prerelease"));
    assert_eq!(
        answer.messages[0].plain,
        "Wrapped follows the prerelease channel in _forever_"
    );
    assert_eq!(
        row_of(&answer, "Wrapped")
            .channel
            .map(|channel| channel.as_str()),
        Some("prerelease")
    );
    let answer = mark("Wrapped", "channel", Some("stable"));
    assert_eq!(
        answer.messages[0].plain,
        "Wrapped follows the stable channel in _forever_"
    );

    let answer = mark("Nope", "ignore", None);
    assert_eq!(answer.messages[0].kind, "error");
    assert_eq!(
        actions::mark_of("channel", Some("nightly")),
        Err("unknown channel nightly: stable or prerelease".to_string())
    );
    assert!(actions::mark_of("delete", None).is_err());
    assert_eq!(
        actions::mark_of("pin", Some("1.2.0")),
        Ok(Mark::Pin(Some("1.2.0".into())))
    );

    // A pinned addon is checked and left alone by "update all".
    mark("OneFolder", "pin", None);
    let (answer, _) = run_with(&game, &Stub::new(), Plan::All, None);
    assert_eq!(
        last_plain(&answer),
        "1 addon updated, 1 failed, 1 up to date, 1 pinned"
    );
    assert_eq!(game.toc_version("OneFolder"), "1.0.0");
}

#[test]
fn a_source_gone_stays_on_its_row_across_a_mark_and_a_restart() {
    // Task 078: the window keeps what its last online check found until the next check replaces it.
    let game = Game::new("kept");
    let engine = game.engine();
    let home = game.home();
    let gone_of = |flavor: &FlavorView| {
        flavor
            .addons
            .iter()
            .find(|row| row.folder == "Wrapped")
            .and_then(|row| row.check.as_ref())
            .map(|check| (check.state, check.label))
    };
    let source = Stub {
        gone: Some("github:Example/Wrapped"),
        ..Stub::new()
    };
    let (answer, _) = run_with(&game, &source, Plan::Check, None);
    let gone = Some((UpdateState::SourceGone, "source gone"));
    assert_eq!(gone_of(answer.flavor.as_ref().expect("the flavor")), gone);

    // A mark redraws the rows from the disk: the source is still gone, on the marked row too.
    for (folder, action, value) in [
        ("OneFolder", "pin", None),
        ("Wrapped", "channel", Some("prerelease")),
        ("Wrapped", "pin", None),
    ] {
        let mark = actions::mark_of(action, value).expect("a mark");
        let answer = actions::mark(&engine, folder, &mark, Some(&home), Voice::new(true));
        assert_eq!(
            gone_of(answer.flavor.as_ref().expect("the flavor")),
            gone,
            "after {action} on {folder}"
        );
    }
    // A restart: the list read again, with no check run.
    let flavor = flavor::read_flavor(&game.flavor_folder()).expect("the flavor");
    let view = list::flavor_view(flavor.clone(), Some(&home), &[]);
    assert_eq!(gone_of(&view), gone);
    let wrapped = view
        .addons
        .iter()
        .find(|row| row.folder == "Wrapped")
        .expect("the row");
    let line = &wrapped.check.as_ref().expect("the kept answer").line;
    assert!(
        line.contains("no such repository, or it is private"),
        "{line}"
    );

    // The next check of the addon replaces it: the repository is back.
    let unpin = actions::mark_of("unpin", None).expect("a mark");
    actions::mark(&engine, "Wrapped", &unpin, Some(&home), Voice::new(true));
    let (answer, _) = run_with(&game, &Stub::new(), Plan::One("Wrapped".into()), None);
    let back = gone_of(answer.flavor.as_ref().expect("the flavor"));
    assert_eq!(back, Some((UpdateState::UpToDate, "up to date")));
    let view = list::flavor_view(flavor, Some(&home), &[]);
    assert_ne!(gone_of(&view), gone, "nothing kept once the source answers");
}

#[test]
fn an_update_reports_the_dependency_and_interface_checks() {
    // Task 079: the checks the command line's install summary reports, after the window's update.
    let game = Game::new("checks");
    let engine = game.engine();
    for (zip, source) in [
        ("needs-library-1.0.0.zip", "github:Example/Needy"),
        ("stale-interface-1.0.0.zip", "github:Example/Stale"),
    ] {
        engine
            .install(
                &zips().join(zip),
                &Metadata {
                    source: source.to_string(),
                    version: Some("0.9.0".to_string()),
                    name: None,
                    nolib: None,
                },
            )
            .unwrap_or_else(|error| panic!("installing {zip}: {error}"));
    }
    let (answer, _) = run_with(&game, &Stub::new(), Plan::One("NeedsLibrary".into()), None);
    let warnings: Vec<&str> = answer
        .messages
        .iter()
        .filter(|message| message.kind == "warning")
        .map(|message| message.plain.as_str())
        .collect();
    assert_eq!(
        warnings,
        [
            // The embedded-only library counts as missing since task 076.
            "NeedsLibrary needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside NeedsLibrary, and the client does not count a library inside an addon's own folder)"
        ],
        "{:?}",
        answer.messages
    );
    assert_eq!(last_plain(&answer), "1 addon updated");
    let needy = answer
        .flavor
        .as_ref()
        .expect("the flavor")
        .addons
        .iter()
        .find(|row| row.folder == "NeedsLibrary")
        .expect("the row");
    assert_eq!(needy.version.as_deref(), Some("1.0.0"));
    assert_eq!(needy.missing, ["LibEmbedded", "LibMissing"]);

    let (answer, _) = run_with(
        &game,
        &Stub::new(),
        Plan::One("StaleInterface".into()),
        None,
    );
    let warning = answer
        .messages
        .iter()
        .find(|message| message.kind == "warning")
        .expect("the interface warning");
    assert_eq!(
        warning.plain,
        "StaleInterface: interface 110207 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor"
    );
}

/// A flavor whose sidecar has two entries of `github:Example/Twice` (tasks 099 and 100): `OldOneFolder`,
/// an older name sorting first, at the source's current release 1.1.0 (its own folder, made by hand), and
/// `OneFolder` at 1.0.0, installed from the fixture zip.
fn two_entries_of_one_source(name: &str) -> Game {
    let game = Game::bare(name);
    std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
    let engine = game.engine();
    engine
        .install(
            &zips().join("one-folder-1.0.0.zip"),
            &Metadata {
                source: "github:Example/Twice".to_string(),
                version: Some("1.0.0".to_string()),
                name: None,
                nolib: None,
            },
        )
        .expect("installing OneFolder");
    let old = game.addons().join("OldOneFolder");
    std::fs::create_dir_all(&old).expect("the old folder");
    std::fs::write(
        old.join("OldOneFolder.toc"),
        "## Interface: 16001\n## Title: OldOneFolder\n## Version: 1.1.0\n",
    )
    .expect("the old TOC");
    let mut state = engine.load_state().expect("the sidecar").state;
    let mut entry = state.addons["OneFolder"].clone();
    entry.version = "1.1.0".to_string();
    entry.folders = vec!["OldOneFolder".to_string()];
    entry.files.clear();
    state.addons.insert("OldOneFolder".to_string(), entry);
    engine
        .home
        .write_state(&state)
        .expect("the sidecar written");
    game
}

#[test]
fn a_row_update_works_on_the_named_entry_of_two_from_one_source() {
    // Task 100: the one install path looks the addon up by its source and its name, so the row update of
    // `OneFolder` updates it and leaves `OldOneFolder` alone; before, it found `OldOneFolder` by the source
    // alone and answered "installed already" (task 099).
    let game = two_entries_of_one_source("named-entry");
    let old = game
        .engine()
        .load_state()
        .expect("the sidecar")
        .state
        .addons["OldOneFolder"]
        .clone();
    let (answer, events) = run_with(&game, &Stub::new(), Plan::One("OneFolder".into()), None);
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("OneFolder")),
            (Phase::Checked, Some("OneFolder")),
            (Phase::Updating, Some("OneFolder")),
            (Phase::Downloading, Some("OneFolder")),
            (Phase::Installing, Some("OneFolder")),
            (Phase::Updated, Some("OneFolder")),
        ]
    );
    assert_eq!(last_plain(&answer), "1 addon updated");
    assert_eq!(game.toc_version("OneFolder"), "1.1.0");
    assert_eq!(game.sidecar_version("OneFolder"), "1.1.0");
    let state = game.engine().load_state().expect("the sidecar").state;
    assert_eq!(
        state.addons["OldOneFolder"], old,
        "the other entry left alone"
    );
    assert!(
        game.addons().join("OldOneFolder").is_dir(),
        "its folder too"
    );
}

#[test]
fn an_update_another_run_made_since_the_check_is_installed_already() {
    // Task 099: the one install path's "installed already" answer is its own phase in the core's words,
    // counted apart from the updated addons. Here another run (the command line, say) updates `OneFolder`
    // between the check and the install, so the install path finds it at the pick's version.
    let game = Game::new("installed-already");
    let engine = game.engine();
    let home = game.home();
    let events = RefCell::new(Vec::new());
    let answer = actions::run(
        &engine,
        &Stub::new(),
        &Plan::One("OneFolder".into()),
        &|| false,
        Some(&home),
        Voice::new(true),
        &mut |event| {
            if event.phase == Phase::Updating {
                game.engine()
                    .install(
                        &zips().join("one-folder-1.1.0.zip"),
                        &Metadata {
                            source: "github:Example/One".to_string(),
                            version: Some("1.1.0".to_string()),
                            name: None,
                            nolib: None,
                        },
                    )
                    .expect("the other run's update");
            }
            events.borrow_mut().push(event);
        },
    );
    let events = events.into_inner();
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("OneFolder")),
            (Phase::Checked, Some("OneFolder")),
            (Phase::Updating, Some("OneFolder")),
            (Phase::InstalledAlready, Some("OneFolder")),
        ]
    );
    let last = &events.last().expect("an event").message;
    assert_eq!(
        (last.kind, last.plain.as_str()),
        (
            "nothing",
            "OneFolder 1.1.0 is installed already in _forever_ from github:Example/One, nothing to do"
        )
    );
    // The command line's summary of the same answer (task 089).
    assert_eq!(last_plain(&answer), "nothing updated: 1 installed already");
    // The row reads from the disk again: the release the other run installed.
    let row = answer
        .flavor
        .as_ref()
        .expect("the flavor")
        .addons
        .iter()
        .find(|row| row.folder == "OneFolder")
        .expect("the row");
    assert_eq!(row.version.as_deref(), Some("1.1.0"));
}

/// Task 110: the core's warning of an install path in a folder the CurseForge app manages (task 098).
const CURSE_CLIENT_WARNING: &str = "the CurseForge app manages _forever_ too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager";

/// The three plans of the window's update actions: the check, "update all" and a row's update.
fn every_plan() -> [Plan; 3] {
    [Plan::Check, Plan::All, Plan::One("SeveralFolders".into())]
}

#[test]
fn a_folder_the_curseforge_app_manages_warns_once_per_run() {
    let game = Game::new("curse-client");
    let marker = game.flavor_folder().join(".curseclient");
    std::fs::write(&marker, "").expect("the marker");
    for plan in every_plan() {
        let (answer, events) = run_with(&game, &Stub::new(), plan.clone(), None);
        let warned: Vec<(&str, &str)> = answer
            .messages
            .iter()
            .filter(|message| message.plain == CURSE_CLIENT_WARNING)
            .map(|message| (message.kind, message.murloc.as_str()))
            .collect();
        assert_eq!(warned, [("warning", "")], "once, a warning: {plan:?}");
        assert_eq!(
            answer.messages[0].plain, CURSE_CLIENT_WARNING,
            "the first of the answer's messages: {plan:?}"
        );
        assert!(
            events
                .iter()
                .all(|event| event.message.plain != CURSE_CLIENT_WARNING),
            "no progress event repeats it: {plan:?}"
        );
    }
    // "Update all" installed as ever, and the marker stays where the CurseForge app put it.
    assert_eq!(game.toc_version("OneFolder"), "1.1.0");
    assert_eq!(game.toc_version("SeveralFolders"), "2.1.0");
    assert!(marker.is_file());
}

// --- task 117: the arms the fixture can provoke ---

#[test]
fn a_refused_answer_is_the_one_line_and_no_flavor() {
    let answer = ActionAnswer::refused(Message::new(
        Voice::new(true),
        Kind::Warning,
        actions::busy_line(),
    ));
    assert_eq!(answer.flavor, None);
    assert_eq!(answer.messages.len(), 1);
    assert_eq!(
        (answer.messages[0].kind, answer.messages[0].plain.as_str()),
        (
            "warning",
            "an update is running already; wait for it to end or cancel it"
        )
    );
}

#[test]
fn a_sidecar_that_cannot_be_read_fails_the_check() {
    let game = Game::new("state-folder");
    let state = game.home().join("wowf").join("state.json");
    std::fs::remove_file(&state).expect("the sidecar removed");
    std::fs::create_dir_all(&state).expect("a folder in its place");
    let source = Stub::new();
    let (answer, events) = run_with(&game, &source, Plan::All, None);
    assert!(
        events.is_empty() && source.asked.get() == 0,
        "nothing asked"
    );
    assert_eq!(answer.messages.len(), 1);
    assert_eq!(answer.messages[0].kind, "error");
    assert!(
        answer.messages[0].plain.contains("state.json"),
        "{}",
        answer.messages[0].plain
    );
    let flavor = answer.flavor.expect("the flavor, drawn all the same");
    assert!(flavor.addons.iter().all(|row| !row.managed), "nothing read");
    assert_eq!(game.toc_version("OneFolder"), "1.0.0");
}

#[test]
fn the_answers_of_a_check_that_cannot_be_kept_are_a_warning() {
    let game = Game::new("last-check-folder");
    let last_check = game
        .home()
        .join("wowf")
        .join(mrgl_core::last_check::LAST_CHECK_FILE);
    std::fs::create_dir_all(&last_check).expect("a folder where the file goes");
    let (answer, _) = run_with(&game, &Stub::new(), Plan::Check, None);
    let kinds: Vec<&str> = answer.messages.iter().map(|message| message.kind).collect();
    assert_eq!(kinds, ["warning", "success"]);
    assert!(
        answer.messages[0]
            .plain
            .starts_with("the answers of this check are not kept for the next start: "),
        "{}",
        answer.messages[0].plain
    );
    assert!(last_check.is_dir(), "left as it was");
}

#[test]
fn a_cancel_keeps_the_warnings_met_before_it() {
    let game = Game::new("cancel-warned");
    std::fs::write(game.flavor_folder().join(".curseclient"), "").expect("the marker");
    let at_first: &dyn Fn(&UpdateEvent) -> bool = &|event| event.phase == Phase::Checked;
    let (answer, _) = run_with(&game, &Stub::new(), Plan::All, Some(at_first));
    let plains: Vec<(&str, &str)> = answer
        .messages
        .iter()
        .map(|message| (message.kind, message.plain.as_str()))
        .collect();
    assert_eq!(
        plains,
        [
            ("warning", CURSE_CLIENT_WARNING),
            (
                "nothing",
                "update cancelled after checking 1 of 4 addons in _forever_; nothing updated"
            )
        ]
    );
}

#[test]
fn a_stale_pick_of_one_source_warns_once_for_its_two_entries() {
    // Task 041: offline, both entries of `github:Example/Twice` resolve from the same cached list, and
    // the answer carries the one line once.
    let game = two_entries_of_one_source("stale-twice");
    let source = Stub {
        stale: true,
        ..Stub::new()
    };
    let (answer, _) = run_with(&game, &source, Plan::Check, None);
    let stale: Vec<&str> = answer
        .messages
        .iter()
        .filter(|message| {
            message
                .plain
                .starts_with("offline: github:Example/Twice v1.1.0 ")
        })
        .map(|message| message.kind)
        .collect();
    assert_eq!(stale, ["warning"], "{:?}", answer.messages);
    assert!(
        answer.messages[0]
            .plain
            .ends_with(", the pick may be stale (the network is off)"),
        "{}",
        answer.messages[0].plain
    );
}

#[test]
fn without_a_home_folder_the_answers_of_a_check_are_not_kept_and_every_row_is_unmanaged() {
    let game = Game::new("no-home");
    let engine = game.engine();
    let answer = actions::run(
        &engine,
        &Stub::new(),
        &Plan::Check,
        &|| false,
        None,
        Voice::new(true),
        &mut |_| {},
    );
    assert_eq!(
        last_plain(&answer),
        "checked _forever_: 3 updates available, 1 up to date"
    );
    let flavor = answer.flavor.expect("the flavor");
    assert!(
        flavor
            .warnings
            .iter()
            .any(|warning| warning.starts_with("no mrgl home folder: ")),
        "{:?}",
        flavor.warnings
    );
    assert!(
        flavor
            .addons
            .iter()
            .all(|row| !row.managed && row.check.is_none())
    );
    assert!(
        !game
            .home()
            .join("wowf")
            .join(mrgl_core::last_check::LAST_CHECK_FILE)
            .exists(),
        "nothing kept"
    );
}

#[test]
fn an_addon_deleted_since_the_check_is_not_updated_when_a_second_entry_has_the_pick() {
    // Task 099: the install path finds the pick's version under another entry of the same source once the
    // addon's own entry is gone (its folder deleted by hand between the check and the install): a warning
    // that names the other entry, the addon counted as installed already, and no row read again.
    let game = two_entries_of_one_source("deleted-since");
    let engine = game.engine();
    let home = game.home();
    let events = RefCell::new(Vec::new());
    let answer = actions::run(
        &engine,
        &Stub::new(),
        &Plan::One("OneFolder".into()),
        &|| false,
        Some(&home),
        Voice::new(true),
        &mut |event| {
            if event.phase == Phase::Updating {
                std::fs::remove_dir_all(game.addons().join("OneFolder")).expect("deleted by hand");
            }
            events.borrow_mut().push(event);
        },
    );
    let events = events.into_inner();
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("OneFolder")),
            (Phase::Checked, Some("OneFolder")),
            (Phase::Updating, Some("OneFolder")),
            (Phase::InstalledAlready, Some("OneFolder")),
        ]
    );
    let last = &events.last().expect("an event").message;
    assert_eq!(last.kind, "warning");
    assert!(
        last.plain
            .starts_with("OneFolder not updated: OldOneFolder, a second entry of github:Example/Twice, has 1.1.0 installed already in _forever_"),
        "{}",
        last.plain
    );
    assert_eq!(last_plain(&answer), "nothing updated: 1 installed already");
    let flavor = answer.flavor.expect("the flavor");
    assert!(
        flavor.addons.iter().all(|row| row.folder != "OneFolder"),
        "the deleted folder is no row"
    );
    assert!(
        game.addons().join("OldOneFolder").is_dir(),
        "the other entry's folder stays"
    );
}

#[test]
fn a_channel_change_needs_a_value() {
    assert_eq!(
        actions::mark_of("channel", None),
        Err("a channel change needs stable or prerelease".to_string())
    );
    assert_eq!(
        actions::mark_of("channel", Some("  ")),
        Err("a channel change needs stable or prerelease".to_string())
    );
    assert!(actions::mark_of("channel", Some("prerelease")).is_ok());
}

#[test]
fn a_folder_without_the_curseforge_marker_has_no_such_warning() {
    let game = Game::new("no-curse-client");
    for plan in every_plan() {
        let (answer, _) = run_with(&game, &Stub::new(), plan.clone(), None);
        assert!(
            answer
                .messages
                .iter()
                .all(|message| !message.plain.contains(".curseclient")),
            "{plan:?}: {:?}",
            answer.messages
        );
    }
}

/// Task 114 over task 108: a flavor whose sidecar has two entries of `github:Example/Folded`,
/// `SeveralFolders` and `SeveralFolders_Options` at 2.0.0, each with its own folder made by hand; the
/// source's current release 2.1.0 is `several-folders-2.1.0.zip`, which carries both folders.
fn two_entries_one_zip(name: &str) -> Game {
    let game = Game::bare(name);
    let mut entries = Vec::new();
    for folder in ["SeveralFolders", "SeveralFolders_Options"] {
        let dir = game.addons().join(folder);
        std::fs::create_dir_all(&dir).expect("the folder");
        std::fs::write(
            dir.join(format!("{folder}.toc")),
            format!("## Interface: 16001\n## Title: {folder}\n## Version: 2.0.0\n"),
        )
        .expect("the TOC");
        entries.push(format!(
            r#""{folder}": {{ "source": "github:Example/Folded", "version": "2.0.0", "folders": ["{folder}"], "files": {{}}, "installed_at": "2026-10-21T14:03:05Z" }}"#
        ));
    }
    let flavor_home = game.home().join("wowf");
    std::fs::create_dir_all(&flavor_home).expect("the flavor home");
    std::fs::write(
        flavor_home.join("state.json"),
        format!(
            r#"{{ "schema": 1, "flavor": "wowf", "addons": {{ {} }} }}"#,
            entries.join(", ")
        ),
    )
    .expect("the sidecar");
    game
}

#[test]
fn update_all_says_an_entry_an_earlier_install_folded_into_its_addon() {
    // Task 114: the install of `SeveralFolders` takes the folder of `SeveralFolders_Options` over and drops
    // its entry (task 108); the update of `SeveralFolders_Options` after it installs nothing, says the
    // core's line as its own phase and is counted as folded, not as installed already or failed.
    let game = two_entries_one_zip("folded");
    let source = Stub::new();
    let (answer, events) = run_with(&game, &source, Plan::All, None);
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("SeveralFolders")),
            (Phase::Checked, Some("SeveralFolders")),
            (Phase::Checking, Some("SeveralFolders_Options")),
            (Phase::Checked, Some("SeveralFolders_Options")),
            (Phase::Updating, Some("SeveralFolders")),
            (Phase::Downloading, Some("SeveralFolders")),
            (Phase::Installing, Some("SeveralFolders")),
            (Phase::Updated, Some("SeveralFolders")),
            (Phase::Folded, Some("SeveralFolders_Options")),
        ]
    );
    let last = events.last().expect("an event");
    assert_eq!((last.index, last.total), (Some(2), Some(2)));
    assert_eq!(
        (last.message.kind, last.message.plain.as_str()),
        (
            "success",
            "SeveralFolders_Options folded into SeveralFolders: SeveralFolders 2.1.0 ships its folder SeveralFolders_Options in _forever_, SeveralFolders_Options has no entry of its own now"
        )
    );
    assert_eq!(source.asked.get(), 2, "the folded entry asks nothing more");
    assert_eq!(
        last_plain(&answer),
        "1 addon updated, 1 folded into another entry"
    );
    assert_eq!(answer.messages.last().expect("a summary").kind, "success");
    let plain: Vec<&str> = answer
        .messages
        .iter()
        .map(|message| message.plain.as_str())
        .collect();
    assert!(
        plain.contains(&"SeveralFolders_Options is no longer managed: SeveralFolders replaced every folder of it"),
        "{plain:?}"
    );
    // The sidecar holds the one entry, with both folders.
    let state = game.engine().load_state().expect("the sidecar").state;
    assert_eq!(
        state.addons.keys().collect::<Vec<_>>(),
        ["SeveralFolders"],
        "one entry after"
    );
    assert_eq!(state.addons["SeveralFolders"].version, "2.1.0");
    assert_eq!(
        state.addons["SeveralFolders"].folders,
        ["SeveralFolders", "SeveralFolders_Options"]
    );
    assert_eq!(game.toc_version("SeveralFolders_Options"), "2.1.0");
    // The rows read from the disk again: no row keeps the dropped entry's check answer.
    let flavor = answer.flavor.as_ref().expect("the flavor");
    for row in &flavor.addons {
        assert!(row.managed, "{row:?}");
        assert_eq!(row.update, None, "{row:?}");
    }
}

/// The line of the distribution-off project `curse:4244` (task 115), as the CurseForge source says it.
const DISTRIBUTION_OFF: &str = "the author of curse:4244 (Mrgl No Distribution) allows no download outside CurseForge: download the zip from https://www.curseforge.com/wow/addons/mrgl-no-distribution and install it with `mrgl add <file.zip> --as curse:4244`";

#[test]
fn a_project_without_distribution_is_a_state_of_its_own_on_its_row_and_after_a_restart() {
    // Task 115: the zip in hand of `curse:4244`, recorded with its source; the check asks the source, whose
    // answer is the state `distribution-off`, counted apart and no failure.
    let game = Game::bare("distribution-off");
    std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
    game.engine()
        .install(
            &zips().join("MrglNoDistribution-2.0.0.zip"),
            &Metadata {
                source: "curse:4244".to_string(),
                version: None,
                name: None,
                nolib: None,
            },
        )
        .expect("the zip in hand");
    let (answer, events) = run_with(&game, &Stub::new(), Plan::All, None);
    assert_eq!(
        phases(&events),
        [
            (Phase::Checking, Some("MrglNoDistribution")),
            (Phase::Checked, Some("MrglNoDistribution")),
        ]
    );
    let check = events[1].check.as_ref().expect("the check's answer");
    assert_eq!(check.state, UpdateState::DistributionOff);
    assert_eq!(check.label, "only on CurseForge");
    assert_eq!(check.detail.as_deref(), Some(DISTRIBUTION_OFF));
    assert_eq!(
        events[1].message.plain,
        format!("MrglNoDistribution  2.0.0  only on CurseForge: {DISTRIBUTION_OFF}")
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        ("nothing", "nothing updated: 1 only on CurseForge")
    );
    let row_state = |flavor: &FlavorView| {
        let row = flavor
            .addons
            .iter()
            .find(|row| row.folder == "MrglNoDistribution")
            .expect("the row");
        assert!(row.update.is_none(), "nothing to install");
        row.check.as_ref().map(|check| check.state)
    };
    assert_eq!(
        row_state(answer.flavor.as_ref().expect("the flavor")),
        Some(UpdateState::DistributionOff)
    );
    // A restart lists offline, where CurseForge is not asked: the kept answer of the check stands.
    let home = game.home();
    let restarted = list::flavor_view(game.engine().flavor.clone(), Some(&home), &[]);
    assert_eq!(row_state(&restarted), Some(UpdateState::DistributionOff));
}

// --- task 116: the add form's zip in hand, with the source it records ---

/// The add of a zip through the window's action, with the serious voice.
fn add(game: &Game, zip: &Path, spec: Option<&str>) -> ActionAnswer {
    let home = game.home();
    actions::add_zip(&game.engine(), zip, spec, Some(&home), Voice::new(true))
}

fn row<'a>(answer: &'a ActionAnswer, folder: &str) -> &'a list::Row {
    answer
        .flavor
        .as_ref()
        .expect("the flavor")
        .addons
        .iter()
        .find(|row| row.folder == folder)
        .unwrap_or_else(|| panic!("no row {folder}"))
}

#[test]
fn the_add_form_installs_a_zip_in_hand_with_its_source_and_the_next_check_asks_it() {
    // Task 116: the zip downloaded from the page of a project without distribution, recorded as
    // `curse:4244` (the form's text trimmed), its version the TOC's; the row carries the source like any
    // other, and the next check asks that source.
    let game = Game::bare("add-zip-curse");
    std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
    let answer = add(
        &game,
        &zips().join("MrglNoDistribution-2.0.0.zip"),
        Some(" curse:4244 "),
    );
    let plain: Vec<(&str, &str)> = answer
        .messages
        .iter()
        .map(|message| (message.kind, message.plain.as_str()))
        .collect();
    assert_eq!(
        plain,
        [(
            "success",
            "installed MrglNoDistribution 2.0.0 (1 folder: MrglNoDistribution) into _forever_"
        )]
    );
    let added = row(&answer, "MrglNoDistribution");
    assert!(added.managed);
    assert_eq!(added.version.as_deref(), Some("2.0.0"));
    let badge = serde_json::to_value(&added.source).expect("serializes");
    assert_eq!(
        badge,
        serde_json::json!({ "kind": "curseforge", "id": "4244" })
    );
    let state = game.engine().load_state().expect("the sidecar").state;
    assert_eq!(state.addons["MrglNoDistribution"].source, "curse:4244");
    assert_eq!(state.addons["MrglNoDistribution"].version, "2.0.0");

    let source = Stub::new();
    let (answer, _) = run_with(&game, &source, Plan::Check, None);
    assert_eq!(*source.specs.borrow(), ["curse:4244"]);
    assert_eq!(
        last_plain(&answer),
        "checked _forever_: 1 only on CurseForge"
    );
}

#[test]
fn the_add_form_records_the_path_without_a_source_and_refuses_a_page_or_a_bad_spec() {
    let game = Game::bare("add-zip-path");
    std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
    let zip = zips().join("one-folder-1.0.0.zip");
    // A page address is no spec the form takes, and a spec mrgl does not read is the core's error; nothing
    // is installed either way.
    let answer = add(
        &game,
        &zip,
        Some("https://addons.wago.io/addons/mrgl-wago-one"),
    );
    assert_eq!(
        (answer.messages.len(), last_plain(&answer)),
        (
            1,
            "the source of a zip is a spec, not a page address: https://addons.wago.io/addons/mrgl-wago-one (use wago:<id>, the X-Wago-ID line of the addon's TOC)"
        )
    );
    assert_eq!(answer.messages[0].kind, "error");
    let answer = add(&game, &zip, Some("nowhere:1"));
    assert_eq!(answer.messages[0].kind, "error");
    assert!(
        !game.addons().join("OneFolder").exists(),
        "nothing installed"
    );
    // An empty field records the zip's path, as `mrgl add <file.zip>` does.
    let answer = add(&game, &zip, Some("  "));
    assert_eq!(
        last_plain(&answer),
        "installed OneFolder 1.0.0 (1 folder: OneFolder) into _forever_"
    );
    let badge = serde_json::to_value(&row(&answer, "OneFolder").source).expect("serializes");
    assert_eq!(
        badge,
        serde_json::json!({ "kind": "zip", "id": "one-folder-1.0.0.zip" })
    );
    let state = game.engine().load_state().expect("the sidecar").state;
    assert_eq!(state.addons["OneFolder"].source, zip.display().to_string());
    // The same zip again is a reinstall, in the command line's words.
    let answer = add(&game, &zip, None);
    assert_eq!(
        last_plain(&answer),
        "reinstalled OneFolder 1.0.0 (1 folder: OneFolder) in _forever_"
    );
}

#[test]
fn the_add_form_warns_as_an_install_path_does_and_says_a_failed_install() {
    let game = Game::bare("add-zip-warnings");
    // Before the client's first start there is no AddOns folder to add to.
    let answer = add(&game, &zips().join("one-folder-1.0.0.zip"), None);
    let message = answer.messages.last().expect("a message");
    assert_eq!(message.kind, "nothing");
    assert!(
        message.plain.starts_with("no Interface")
            && message
                .plain
                .ends_with("folder in _forever_: the client has not been started for this flavor yet; start it once, then add the zip"),
        "{}",
        message.plain
    );
    assert!(answer.flavor.is_some());
    std::fs::create_dir_all(game.addons()).expect("the AddOns folder");
    std::fs::write(game.flavor_folder().join(".curseclient"), "").expect("the marker");
    // Task 110: the marker's warning first, then the install's interface check, then the summary.
    let answer = add(
        &game,
        &zips().join("stale-interface-1.0.0.zip"),
        Some("wago:MrglStale"),
    );
    let plain: Vec<(&str, &str)> = answer
        .messages
        .iter()
        .map(|message| (message.kind, message.plain.as_str()))
        .collect();
    assert_eq!(plain[0], ("warning", CURSE_CLIENT_WARNING));
    assert_eq!(
        plain[1],
        (
            "warning",
            "StaleInterface: interface 110207 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor"
        )
    );
    assert_eq!(
        plain.last().copied(),
        Some((
            "success",
            "installed StaleInterface 1.0.0 (1 folder: StaleInterface) into _forever_"
        ))
    );
    // A zip that is not there is the engine's error, and the folder stays as it was.
    let answer = add(&game, &game.dir.join("gone.zip"), None);
    assert_eq!(answer.messages.last().expect("a message").kind, "error");
    assert_eq!(answer.messages[0].plain, CURSE_CLIENT_WARNING);
}
