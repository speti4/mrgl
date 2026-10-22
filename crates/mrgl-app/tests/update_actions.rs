//! The window's update actions and marks (task 010) on a made-up Forever flavor: four addons installed
//! through the engine from the fixture zips, a stub source that answers from those zips, and the run's
//! events, answer and disk checked without a window. Nothing touches the network or a real game folder.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use mrgl_app_lib::actions::{self, ActionAnswer, Phase, Plan, UpdateEvent};
use mrgl_app_lib::list::{self, FlavorView};
use mrgl_core::error::Result;
use mrgl_core::flavor;
use mrgl_core::forever::ForeverBuild;
use mrgl_core::install::{Engine, Mark, Metadata};
use mrgl_core::sources::{Pick, RequestBudget, Source, SourceSpec, Target};
use mrgl_core::update::UpdateState;
use mrgl_core::voice::Voice;

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
    budget: Option<u32>,
    /// A spec whose repository is gone: a 404, as the GitHub source says it.
    gone: Option<&'static str>,
}

impl Stub {
    fn new() -> Stub {
        Stub {
            asked: Cell::new(0),
            budget: None,
            gone: None,
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
            _ => None,
        }
    }
}

impl Source for Stub {
    fn resolve(&self, spec: &SourceSpec, _target: &Target) -> Result<Pick> {
        self.asked.set(self.asked.get() + 1);
        let spec = spec.to_string();
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
            stale: None,
        })
    }

    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("the download folder");
        std::fs::copy(zips().join(&pick.asset), to).expect("the fixture zip");
        Ok(())
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
            "stopped the check before GitHub's rate limit of 60 requests an hour without a token: 2 addons not checked"
        ),
        "{}",
        stops[0].message.plain
    );
    assert!(
        answer
            .messages
            .iter()
            .all(|message| !message.plain.starts_with("stopped the check")),
        "the answer does not repeat the stop"
    );
    let summary = answer.messages.last().expect("a summary");
    assert_eq!(
        (summary.kind, summary.plain.as_str()),
        (
            "warning",
            "checked _forever_: 2 updates available, 2 not checked (rate limit)"
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
