//! The toolbox's adopt card (task 109 over tasks 097 and 098) over a copy of the synthetic game folder
//! `mrgl-core/tests/fixtures/adopt/`: the report's rows, lines and counts against the folder stub of the
//! GitHub, WoWInterface and Wago answers (the fixture `releases/` and `sources/` trees copied into one
//! temporary folder), the apply with the sidecar it writes, which the window's update then takes as the
//! command line's does, a second apply that changes nothing, and the guard of a folder the CurseForge app
//! manages with and without the confirmation, and the apply over the answers of the report the card shows,
//! which asks no source again (task 113). No network, no real game folder, no window.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mrgl_app_lib::actions::{self, Phase, Plan, UpdateEvent};
use mrgl_app_lib::toolbox::{self, AdoptAnswer, AdoptRequest, AdoptView};
use mrgl_core::adopt::Answers;
use mrgl_core::flavor;
use mrgl_core::install::Engine;
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::wago::Wago;
use mrgl_core::sources::wowinterface::WowInterface;
use mrgl_core::sources::{FolderHttp, Http, SourceKind, Sources};
use mrgl_core::voice::Voice;

/// The made-up Wago key of these tests; no answer may carry it.
const WAGO_TEST_KEY: &str = "made-up-wago-key-for-the-window-adopt-tests";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the folder");
    for entry in std::fs::read_dir(from).expect("read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// A copy of the adopt fixture's game folder, the stub of every source and a home, all temporary.
struct Game {
    base: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let base =
            std::env::temp_dir().join(format!("mrgl-app-adopt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        copy_tree(&fixtures().join("adopt"), &base.join("game"));
        copy_tree(&fixtures().join("releases"), &base.join("stub"));
        copy_tree(&fixtures().join("sources"), &base.join("stub"));
        Game { base }
    }

    fn root(&self) -> PathBuf {
        self.base.join("game")
    }

    fn home(&self) -> PathBuf {
        self.base.join("home")
    }

    fn state_path(&self, flavor_id: &str) -> PathBuf {
        self.home().join(flavor_id).join("state.json")
    }

    fn sidecar(&self, flavor_id: &str) -> serde_json::Value {
        let text = std::fs::read_to_string(self.state_path(flavor_id)).expect("the sidecar");
        serde_json::from_str(&text).expect("the sidecar parses")
    }

    fn addon(&self, flavor_folder: &str, folder: &str) -> PathBuf {
        self.root()
            .join(flavor_folder)
            .join("Interface/AddOns")
            .join(folder)
    }

    /// The router over the stub, as `Sources::from_env` builds it: GitHub without a token, WoWInterface,
    /// Wago with the made-up key or without one; the caches under the home.
    fn sources(&self, wago_key: bool) -> Sources {
        self.sources_over(Arc::new(FolderHttp::new(self.base.join("stub"))), wago_key)
    }

    /// The folder stub of every source, whose log counts the requests.
    fn stub(&self) -> Arc<FolderHttp> {
        Arc::new(FolderHttp::new(self.base.join("stub")))
    }

    /// [`Game::sources`] over a given stub, a new router each time, as each command of the window builds
    /// its own.
    fn sources_over(&self, http: Arc<dyn Http>, wago_key: bool) -> Sources {
        let cache = self.home().join("cache");
        Sources::none()
            .with(
                SourceKind::GitHub,
                GitHub::new(Arc::clone(&http), None, Some(cache.join("github"))),
            )
            .with(
                SourceKind::Wago,
                Wago::new(
                    Arc::clone(&http),
                    wago_key.then(|| WAGO_TEST_KEY.to_string()),
                    Some(cache.join("wago")),
                ),
            )
            .with(
                SourceKind::WowInterface,
                WowInterface::new(http, Some(cache.join("wowi"))),
            )
    }

    fn report(&self, flavor_id: &str, wago_key: bool) -> AdoptAnswer {
        let home = self.home();
        toolbox::adopt_report(
            &self.root(),
            flavor_id,
            Some(&home),
            &self.sources(wago_key),
            &|kind| kind == SourceKind::Wago && wago_key,
            Voice::new(true),
        )
    }

    fn apply(&self, flavor_id: &str, alongside: bool, wago_key: bool) -> AdoptAnswer {
        self.apply_over(
            flavor_id,
            AdoptRequest {
                alongside,
                answers: None,
            },
            &self.sources(wago_key),
            wago_key,
        )
    }

    fn apply_over(
        &self,
        flavor_id: &str,
        request: AdoptRequest<'_>,
        sources: &Sources,
        wago_key: bool,
    ) -> AdoptAnswer {
        let home = self.home();
        toolbox::adopt_apply(
            &self.root(),
            flavor_id,
            request,
            Some(&home),
            sources,
            &|kind| kind == SourceKind::Wago && wago_key,
            Voice::new(true),
        )
    }

    fn engine(&self, flavor_folder: &str) -> Engine {
        let flavor = flavor::read_flavor(&self.root().join(flavor_folder)).expect("the flavor");
        Engine::with_home(flavor, &self.home())
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn view(answer: &AdoptAnswer) -> &AdoptView {
    answer.adopt.as_ref().expect("the report")
}

fn plains(answer: &AdoptAnswer) -> Vec<(&str, &str)> {
    answer
        .messages
        .iter()
        .map(|message| (message.kind, message.plain.as_str()))
        .collect()
}

/// The rows as `mrgl adopt` prints them: the addon's line, then a line per source and page under it.
fn lines(view: &AdoptView) -> Vec<String> {
    view.rows
        .iter()
        .flat_map(|row| {
            std::iter::once(row.line.clone())
                .chain(row.details.iter().map(|line| format!("  {line}")))
        })
        .collect()
}

#[test]
fn the_report_shows_every_row_its_sources_and_the_counts() {
    let game = Game::new("report");
    let answer = game.report("wowf", false);
    let adopt = view(&answer);
    assert_eq!(
        (adopt.flavor.as_str(), adopt.folder.as_str()),
        ("wowf", "_forever_")
    );
    assert_eq!(
        lines(adopt),
        [
            "MrglCurseOnly 0.4.2: only with CurseForge, needs MRGL_CURSEFORGE_KEY",
            "  curse:4242 from X-Curse-Project-ID: needs MRGL_CURSEFORGE_KEY",
            "MrglElsewhere 0.1 (2 folders: MrglElsewhere, MrglElsewhere_Options): adopt as wowi:1006",
            "  wowi:1006 from X-WoWI-ID: forever build unknown (WoWInterface lists no forever compatibility (it lists 11.2.7))",
            "MrglForever 3.0.0: adopt as wowi:1005",
            "  wowi:1005 from X-WoWI-ID: forever build yes (WoWInterface lists compatibility 1.60.1 (Made-up Forever Patch))",
            "MrglNoForever 4.0.0: no forever build at its sources",
            "  github:Example/MainlineOnly from X-Website: forever build no (no asset for flavor forever in github:Example/MainlineOnly v4.0.0 (release.json names mainline, standard; assets: Only-v4.0.0.zip, release.json; looked at 1 release))",
            "MrglNoHint 2.2: no known source",
            "MrglNoList 1.0.0: adopt as wowi:1002",
            "  wago:MrglWg02 from X-Wago-ID: forever build no (Wago lists no release for forever); needs MRGL_WAGO_KEY",
            "  wowi:1002 from X-WoWI-ID: forever build unknown (WoWInterface lists no compatibility for the project)",
            "  curse:4243 from X-Curse-Project-ID: needs MRGL_CURSEFORGE_KEY",
            "MrglPage 1.0: no known source",
            "  page https://www.example.invalid/mrgl-page from X-Website, not a source mrgl reads",
            "MrglSite (no version): adopt as github:Example/Nightly",
            "  github:Example/Nightly from X-Website: forever build yes (release.json names forever (interface 16001))",
            "MrglWagoOne 1.0.0 (2 folders: MrglWagoOne, MrglWagoOne_Options): needs MRGL_WAGO_KEY",
            "  wago:MrglWg01 from X-Wago-ID: forever build yes (Wago lists a release for forever); needs MRGL_WAGO_KEY",
            "Questie 1.0.0 (2 folders: Questie, QuestieDB): adopt as github:Questie/Questie",
            "  github:Questie/Questie from the GitHub mapping: forever build yes (release.json names forever (interface 16001))",
        ]
    );
    let counts = adopt.counts;
    assert_eq!(
        (
            counts.addons,
            counts.without_curseforge,
            counts.only_curseforge,
            counts.no_source,
            counts.no_build,
            counts.adoptable_now
        ),
        (10, 6, 1, 2, 1, 5)
    );
    const SUMMARY: &str = "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; the Adopt button adopts 5 of them";
    assert_eq!(
        (adopt.summary.kind, adopt.summary.plain.as_str()),
        ("success", SUMMARY)
    );
    assert_eq!(plains(&answer), [("success", SUMMARY)], "the summary, said");
    assert!(!adopt.curse_client && adopt.marker.is_none());
    assert_eq!(answer.flavor, None);
    assert!(answer.adopted.is_empty());

    // The typed row behind the lines, as `mrgl adopt --json` writes it.
    let wago = adopt
        .rows
        .iter()
        .find(|row| row.addon.name == "MrglWagoOne")
        .expect("the Wago row");
    assert_eq!(wago.verdict, "needs MRGL_WAGO_KEY");
    assert_eq!(wago.addon.adopt_as, None);
    assert_eq!(wago.addon.needs, ["MRGL_WAGO_KEY"]);
    assert!(
        !game.state_path("wowf").exists(),
        "the report writes nothing"
    );

    // The Wago key makes the Wago addon adoptable, and stays out of the answer.
    let answer = game.report("wowf", true);
    let adopt = view(&answer);
    assert_eq!(adopt.counts.adoptable_now, 6);
    assert!(
        adopt
            .rows
            .iter()
            .any(|row| row.line == "MrglWagoOne 1.0.0 (2 folders: MrglWagoOne, MrglWagoOne_Options): adopt as wago:MrglWg01")
    );
    let text = serde_json::to_string(&answer).expect("serializes");
    assert!(!text.contains(WAGO_TEST_KEY));
}

#[test]
fn a_folder_the_curseforge_app_manages_shows_its_marker() {
    let game = Game::new("marker");
    let answer = game.report("wow", false);
    let adopt = view(&answer);
    assert!(adopt.curse_client);
    let marker = adopt.marker.as_ref().expect("the marker's line");
    assert_eq!(
        (marker.kind, marker.plain.as_str()),
        (
            "warning",
            "the CurseForge app manages _retail_ (its .curseclient marker is there): a folder belongs to one manager, so the Adopt button writes nothing there without the confirmation to adopt alongside the CurseForge app"
        )
    );
    // The window's words (task 112): no line of the card names a flag of the command line.
    for line in [&marker.plain, &adopt.summary.plain] {
        assert!(!line.contains("--"), "{line}");
    }
    assert_eq!(lines(adopt)[0], "MrglCompat 1.0.0: adopt as wowi:1001");
}

#[test]
fn the_apply_writes_the_sidecar_the_update_takes_and_a_second_apply_nothing() {
    let game = Game::new("apply");
    let toc = game
        .addon("_forever_", "MrglWagoOne")
        .join("MrglWagoOne.toc");
    let before = std::fs::read(&toc).expect("the TOC");
    let answer = game.apply("wowf", false, true);
    assert_eq!(
        answer.adopted,
        [
            "adopted MrglElsewhere 0.1 as wowi:1006 (2 folders: MrglElsewhere, MrglElsewhere_Options)",
            "adopted MrglForever 3.0.0 as wowi:1005 (1 folder: MrglForever)",
            "adopted MrglNoList 1.0.0 as wowi:1002 (1 folder: MrglNoList)",
            "adopted MrglSite unknown as github:Example/Nightly (1 folder: MrglSite)",
            "adopted MrglWagoOne 1.0.0 as wago:MrglWg01 (2 folders: MrglWagoOne, MrglWagoOne_Options)",
            "adopted Questie 1.0.0 as github:Questie/Questie (2 folders: Questie, QuestieDB)",
        ]
    );
    assert_eq!(
        plains(&answer),
        [
            (
                "warning",
                "MrglCurseOnly not adopted: only with CurseForge, needs MRGL_CURSEFORGE_KEY"
            ),
            (
                "success",
                "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; adopted 6 addons into the sidecar, no download and no folder changed: the next update installs each from its source"
            ),
        ]
    );
    let text = serde_json::to_string(&answer).expect("serializes");
    assert!(!text.contains(WAGO_TEST_KEY));

    // The sidecar as the command line's test reads it; no folder changed, nothing downloaded.
    assert_eq!(std::fs::read(&toc).expect("the TOC"), before);
    assert!(!game.home().join("wowf").join("cache").exists());
    let state = game.sidecar("wowf");
    let entry = &state["addons"]["MrglWagoOne"];
    assert_eq!(entry["source"], "wago:MrglWg01");
    assert_eq!(entry["version"], "1.0.0");
    assert_eq!(
        entry["folders"],
        serde_json::json!(["MrglWagoOne", "MrglWagoOne_Options"])
    );
    assert_eq!(entry["files"].as_object().expect("files").len(), 4);
    assert_eq!(state["addons"]["MrglSite"]["version"], "unknown");
    assert!(state["addons"].get("MrglCurseOnly").is_none());

    // The report as it is now: the adopted addons are managed, the rest are its rows.
    let adopt = view(&answer);
    let names: Vec<&str> = adopt
        .rows
        .iter()
        .map(|row| row.addon.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["MrglCurseOnly", "MrglNoForever", "MrglNoHint", "MrglPage"]
    );
    assert_eq!(adopt.counts.adoptable_now, 0);
    // The flavor's rows as they are now: the adopted folders are managed, with their source.
    let flavor = answer.flavor.as_ref().expect("the flavor's rows");
    let row = flavor
        .addons
        .iter()
        .find(|row| row.folder == "MrglWagoOne")
        .expect("the row");
    assert!(row.managed && !row.guessed);
    assert_eq!(
        row.source
            .as_ref()
            .map(|badge| (badge.kind, badge.id.as_str())),
        Some(("wago", "MrglWg01"))
    );
    assert_eq!(
        flavor.addons.iter().filter(|row| row.managed).count(),
        9,
        "nine folders of six addons"
    );

    // The window's update takes the adopted entry and installs the source's build over it.
    let engine = game.engine("_forever_");
    let home = game.home();
    let events = RefCell::new(Vec::new());
    let update = actions::run(
        &engine,
        &game.sources(true),
        &Plan::One("MrglWagoOne".into()),
        &|| false,
        Some(&home),
        Voice::new(true),
        &mut |event: UpdateEvent| events.borrow_mut().push(event),
    );
    let updated: Vec<String> = events
        .borrow()
        .iter()
        .filter(|event| event.phase == Phase::Updated)
        .map(|event| event.message.plain.clone())
        .collect();
    assert_eq!(
        updated,
        ["updated MrglWagoOne from 1.0.0 to 2.0.0 in _forever_"],
        "{:?}",
        update.messages
    );
    let state = game.sidecar("wowf");
    let entry = &state["addons"]["MrglWagoOne"];
    assert_eq!(entry["version"], "2.0.0");
    assert_eq!(entry["previous_version"], "1.0.0");
    assert_eq!(entry["folders"], serde_json::json!(["MrglWagoOne"]));
    assert!(!game.addon("_forever_", "MrglWagoOne_Options").exists());
    assert_ne!(std::fs::read(&toc).expect("the TOC"), before);

    // A second apply changes nothing.
    let first = std::fs::read(game.state_path("wowf")).expect("the sidecar");
    let again = game.apply("wowf", false, true);
    assert!(again.adopted.is_empty());
    assert_eq!(
        plains(&again).last().copied(),
        Some((
            "nothing",
            "4 unmanaged addons in _forever_: 0 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; nothing adopted, the sidecar is unchanged"
        ))
    );
    assert_eq!(
        std::fs::read(game.state_path("wowf")).expect("the sidecar"),
        first
    );
}

#[test]
fn the_marker_guard_holds_without_the_confirmation_and_warns_with_it() {
    let game = Game::new("guard");
    let marker = game.root().join("_retail_/.curseclient");
    let refused = game.apply("wow", false, false);
    assert_eq!(
        plains(&refused),
        [(
            "error",
            "the CurseForge app manages _retail_ (its .curseclient marker is there) and a folder belongs to one manager: the Adopt button wrote nothing; the confirmation to adopt alongside the CurseForge app adopts its addons anyway, and the CurseForge app may still update those folders"
        )]
    );
    assert!(refused.adopt.is_none() && refused.flavor.is_none() && refused.adopted.is_empty());
    assert!(!game.state_path("wow").exists(), "nothing written");

    let confirmed = game.apply("wow", true, false);
    assert_eq!(
        confirmed.adopted,
        ["adopted MrglCompat 1.0.0 as wowi:1001 (1 folder: MrglCompat)"]
    );
    assert_eq!(
        plains(&confirmed),
        [
            (
                "warning",
                "adopting alongside the CurseForge app in _retail_: it may still update or flag the adopted folders, and its .curseclient marker stays where it is"
            ),
            (
                "success",
                "1 unmanaged addon in _retail_: 1 can be adopted without CurseForge, 0 only with CurseForge, 0 with no known source; adopted 1 addon into the sidecar, no download and no folder changed: the next update installs each from its source"
            ),
        ]
    );
    assert_eq!(
        game.sidecar("wow")["addons"]["MrglCompat"]["source"],
        "wowi:1001"
    );
    assert!(marker.is_file(), "the marker stays");
    assert!(view(&confirmed).rows.is_empty());

    // A folder without the marker needs no confirmation, and is not refused without one.
    let forever = game.apply("wowf", false, false);
    assert_eq!(forever.adopted.len(), 5);
    assert!(
        forever
            .messages
            .iter()
            .all(|message| !message.plain.contains(".curseclient"))
    );
}

/// The sidecar without the times of its entries, which differ from one apply to the next.
fn timeless(mut state: serde_json::Value) -> serde_json::Value {
    for entry in state["addons"]
        .as_object_mut()
        .expect("the entries")
        .values_mut()
    {
        entry
            .as_object_mut()
            .expect("an entry")
            .remove("installed_at");
    }
    state
}

#[test]
fn the_apply_writes_from_the_shown_reports_answers_and_asks_no_source() {
    let game = Game::new("answers");
    let stub = game.stub();
    let shown = shown_report(&game, &stub);
    let asked = stub.requests().len();
    assert!(asked > 0, "the report asks the sources once");
    // The answers go to the page and come back as the page sends them.
    let text = serde_json::to_string(&view(&shown).answers).expect("serializes");
    assert!(!text.contains(WAGO_TEST_KEY));
    let answers: Answers = serde_json::from_str(&text).expect("the page's answers parse");
    assert_eq!(answers.flavor, "wowf");
    assert!(answers.offers.contains_key("wowi:1005"));

    let answer = game.apply_over(
        "wowf",
        AdoptRequest {
            alongside: false,
            answers: Some(&answers),
        },
        &game.sources_over(stub.clone(), true),
        true,
    );
    assert_eq!(
        stub.requests().len(),
        asked,
        "no request for the apply and the report after it"
    );
    assert_eq!(answer.adopted.len(), 6, "{:?}", answer.messages);
    let names: Vec<&str> = view(&answer)
        .rows
        .iter()
        .map(|row| row.addon.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["MrglCurseOnly", "MrglNoForever", "MrglNoHint", "MrglPage"]
    );
    assert_eq!(
        view(&answer).answers,
        answers,
        "the same answers, for the next apply"
    );

    // The sidecar equals the one of the apply that asks every source again (task 109).
    let asking = Game::new("answers-asking");
    asking.apply("wowf", false, true);
    assert_eq!(
        timeless(game.sidecar("wowf")),
        timeless(asking.sidecar("wowf"))
    );
}

/// The card's report over a counting stub, with the Wago key.
fn shown_report(game: &Game, stub: &Arc<FolderHttp>) -> AdoptAnswer {
    let home = game.home();
    toolbox::adopt_report(
        &game.root(),
        "wowf",
        Some(&home),
        &game.sources_over(stub.clone(), true),
        &|kind| kind == SourceKind::Wago,
        Voice::new(true),
    )
}

/// An apply over the answers of a report.
fn request(answers: &Answers) -> AdoptRequest<'_> {
    AdoptRequest {
        alongside: false,
        answers: Some(answers),
    }
}

#[test]
fn old_answers_or_another_flavors_are_asked_again() {
    let game = Game::new("old-answers");
    let stub = game.stub();
    let shown = shown_report(&game, &stub);
    let asked = stub.requests().len();
    let mut answers = view(&shown).answers.clone();
    answers.asked_at = "2026-01-01T00:00:00Z".to_string();
    let other = Answers {
        flavor: "wow".to_string(),
        ..view(&shown).answers.clone()
    };
    let answer = game.apply_over(
        "wowf",
        request(&other),
        &game.sources_over(stub.clone(), true),
        true,
    );
    assert!(
        stub.requests().len() > asked,
        "another flavor's answers are asked again"
    );
    assert_eq!(answer.adopted.len(), 6);
    assert_eq!(
        view(&answer).answers.flavor,
        "wowf",
        "the flavor's own answers"
    );

    let game = Game::new("old-answers-2");
    let stub = game.stub();
    let before = stub.requests().len();
    let answer = game.apply_over(
        "wowf",
        request(&answers),
        &game.sources_over(stub.clone(), true),
        true,
    );
    assert!(
        stub.requests().len() > before,
        "answers older than an hour are asked again"
    );
    assert_eq!(answer.adopted.len(), 6);
    assert_ne!(view(&answer).answers.asked_at, answers.asked_at);
}

#[test]
fn without_a_home_folder_nothing_is_read_or_written() {
    let game = Game::new("no-home");
    let sources = game.sources(false);
    let report = toolbox::adopt_report(
        &game.root(),
        "wowf",
        None,
        &sources,
        &|_| false,
        Voice::new(true),
    );
    assert_eq!(report.messages.len(), 1);
    assert_eq!(report.messages[0].kind, "error");
    assert!(report.messages[0].plain.starts_with("no adopt report: "));
    let apply = toolbox::adopt_apply(
        &game.root(),
        "wowf",
        AdoptRequest::default(),
        None,
        &sources,
        &|_| false,
        Voice::new(true),
    );
    assert!(apply.messages[0].plain.starts_with("nothing adopted: "));
    assert!(!game.home().exists());
    let unknown = game.report("wow_mrgl", false);
    assert_eq!(unknown.messages[0].kind, "error");
    assert!(unknown.adopt.is_none());
}

#[test]
fn the_answer_serializes_with_the_keys_of_the_contract() {
    let game = Game::new("keys");
    let answer = game.apply("wowf", false, true);
    let value = serde_json::to_value(&answer).expect("serializes");
    let keys = |value: &serde_json::Value| -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    // The keys of the contract in the header of ui/mock-data.js.
    assert_eq!(keys(&value), ["adopt", "adopted", "flavor", "messages"]);
    let adopt = &value["adopt"];
    assert_eq!(
        keys(adopt),
        [
            "answers",
            "counts",
            "curse_client",
            "flavor",
            "folder",
            "left_out",
            "marker",
            "rows",
            "summary"
        ]
    );
    assert_eq!(
        keys(&adopt["rows"][0]),
        [
            "adopt_as", "class", "details", "folders", "line", "name", "needs", "pages", "sources",
            "verdict", "version"
        ]
    );
    assert_eq!(adopt["rows"][0]["class"], "only-curseforge");
    assert_eq!(
        keys(&adopt["rows"][0]["sources"][0]),
        [
            "build", "from", "missing", "needs", "source", "spec", "taken_by", "why"
        ]
    );
    assert_eq!(
        keys(&adopt["counts"]),
        [
            "addons",
            "adoptable_now",
            "no_build",
            "no_source",
            "only_curseforge",
            "without_curseforge"
        ]
    );
    assert_eq!(keys(&adopt["summary"]), ["kind", "murloc", "plain"]);
    assert_eq!(
        keys(&adopt["answers"]),
        ["asked_at", "flavor", "folders", "offers"]
    );
    assert_eq!(
        keys(&adopt["answers"]["offers"]["wowi:1005"]),
        ["build", "missing", "why"]
    );
}

// --- task 117: the arms the fixture can provoke ---

#[test]
fn a_sidecar_that_cannot_be_read_refuses_the_report_and_the_apply() {
    let game = Game::new("state-folder");
    // A folder where the sidecar file should be: the read fails, and nothing is reported or written.
    std::fs::create_dir_all(game.state_path("wowf")).expect("the folder");
    let report = game.report("wowf", false);
    assert_eq!(report.adopt, None);
    assert_eq!(report.messages.len(), 1);
    assert_eq!(report.messages[0].kind, "error");
    assert!(
        report.messages[0].plain.contains("state.json"),
        "{}",
        report.messages[0].plain
    );
    let apply = game.apply("wowf", false, false);
    assert_eq!((apply.adopt.clone(), apply.flavor.clone()), (None, None));
    assert!(apply.adopted.is_empty());
    assert_eq!(plains(&apply), plains(&report));
}

#[test]
fn a_rebuilt_sidecar_is_the_first_warning_of_the_report() {
    let game = Game::new("state-rebuilt");
    let path = game.state_path("wowf");
    std::fs::create_dir_all(path.parent().expect("the folder of the flavor")).expect("the folder");
    std::fs::write(&path, "{").expect("a sidecar that is not JSON");
    let report = game.report("wowf", false);
    assert!(report.adopt.is_some());
    assert_eq!(report.messages[0].kind, "warning");
    assert!(
        report.messages[0]
            .plain
            .starts_with("unreadable state sidecar, starting over"),
        "{}",
        report.messages[0].plain
    );
    assert_eq!(report.messages.last().expect("the summary").kind, "success");
}

#[test]
fn an_unknown_flavor_refuses_the_apply() {
    let game = Game::new("apply-unknown");
    let apply = game.apply("wow_mrgl", true, false);
    assert_eq!(apply.adopt, None);
    assert_eq!(
        plains(&apply),
        [(
            "error",
            format!(
                "no flavor wow_mrgl: no such flavor folder under {}",
                game.root().display()
            )
            .as_str()
        )]
    );
    assert!(!game.home().exists());
}

#[test]
fn a_sidecar_that_cannot_be_written_keeps_the_report_and_adopts_nothing() {
    let game = Game::new("state-unwritable");
    // A folder where the temporary file of the sidecar goes: the write fails after the sources answered.
    let temporary = game.state_path("wowf").with_extension("json.tmp");
    std::fs::create_dir_all(&temporary).expect("the folder");
    let apply = game.apply("wowf", false, false);
    let adopt = apply
        .adopt
        .as_ref()
        .expect("the report it would have written from");
    assert_eq!(adopt.counts.adoptable_now, 5, "{:?}", lines(adopt));
    assert!(apply.adopted.is_empty() && apply.flavor.is_none());
    assert_eq!(apply.messages.len(), 1);
    assert_eq!(apply.messages[0].kind, "error");
    assert!(
        apply.messages[0].plain.contains("state.json"),
        "{}",
        apply.messages[0].plain
    );
    assert!(!game.state_path("wowf").exists(), "nothing written");
}
