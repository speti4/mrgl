//! The window's flavor list (task 009) on a made-up game folder: the commands are plain functions, so the
//! rows are checked without a window. Each test writes its own small game root, mrgl home folder (the
//! sidecars) and release cache under a temporary folder; the cache is filled by the GitHub source itself
//! from the fixture answers under `crates/mrgl-core/tests/fixtures/releases`, so nothing touches the
//! network or a real game folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_app_lib::commands;
use mrgl_app_lib::list::{self, FlavorView, Row};
use mrgl_core::discover::{GameRoot, RootOrigin};
use mrgl_core::flavor::FlavorId;
use mrgl_core::home;
use mrgl_core::sources::github::{self, GitHub};
use mrgl_core::sources::{Channel, FolderHttp, SourceSpec};
use mrgl_core::state::{Entry, FlavorHome, State};
use mrgl_core::update::VersionOrder;
use mrgl_core::voice::Voice;

const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Tags!STRING:0|Product!STRING:0\n\
eu|1|88d5e8ef05ec5a8446cdec93d142d7c6|12.1.5.70448||wow\n\
eu|1|e8c4ef4e67bd1d94d820254984258fa8|1.60.1.70245||wowf\n";

/// A temporary folder with a game root (`root`) and a mrgl home folder (`home`), removed at the end.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("mrgl-app-list-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp folder");
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.join("World of Warcraft")
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.dir.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("folders");
        std::fs::write(path, text).expect("write");
    }

    fn addon(&self, flavor: &str, folder: &str, toc: &str) {
        self.write(
            &format!("World of Warcraft/{flavor}/Interface/AddOns/{folder}/{folder}.toc"),
            toc,
        );
    }

    /// Retail (the CurseForge marker, four addons) and Forever (four addons), with their sidecars and the
    /// cached release list of `Questie/Questie`.
    fn game() -> Fixture {
        Fixture::game_named("game")
    }

    fn game_named(name: &str) -> Fixture {
        let fixture = Fixture::new(name);
        fixture.write("World of Warcraft/.build.info", BUILD_INFO);
        fixture.write(
            "World of Warcraft/_retail_/.flavor.info",
            "Product Flavor!STRING:0\nwow\n",
        );
        fixture.write("World of Warcraft/_retail_/.curseclient", "{}");
        fixture.write(
            "World of Warcraft/_forever_/.flavor.info",
            "Product Flavor!STRING:0\nwowf\n",
        );
        fixture.addon(
            "_retail_",
            "Questie",
            "## Interface: 120105\n## Title: |cff00ff00Questie|r\n## Version: 1.2.2\n",
        );
        fixture.addon(
            "_retail_",
            "Hinted",
            "## Interface: 120105\n## Title: Hinted\n## Version: 2.0.0\n## X-Curse-Project-ID: 123456\n## X-Wago-ID: aB1cD2eF\n",
        );
        fixture.addon(
            "_retail_",
            "Needy",
            "## Interface: 120105\n## Version: 0.1\n## Dependencies: LibGone, Plain, Blizzard_Something, LibInside\n",
        );
        // A library only embedded inside Needy: the client does not count it (task 076).
        fixture.write(
            "World of Warcraft/_retail_/Interface/AddOns/Needy/Libs/LibInside/LibInside.lua",
            "-- embedded\n",
        );
        fixture.addon("_retail_", "Plain", "## Interface: 120105\n## Version: 3\n");
        fixture.addon(
            "_forever_",
            "Questie",
            "## Interface: 16001\n## Title: Questie\n## Version: v1.2.3\n",
        );
        fixture.addon(
            "_forever_",
            "DevAddon",
            "## Interface: 16001\n## Version: 9.9.9-dev\n",
        );
        fixture.addon(
            "_forever_",
            "ZipOne",
            "## Interface: 16001\n## Version: 1.0\n",
        );
        fixture.addon(
            "_forever_",
            "Loose",
            "## Interface: 16001\n## X-Curse-Project-ID: 42\n",
        );

        let home = fixture.home();
        let mut retail = State::new(&FlavorId::Retail);
        retail.addons.insert(
            "Questie".into(),
            entry("github:Questie/Questie", "1.2.2", &["Questie"]),
        );
        FlavorHome::at(&home, &FlavorId::Retail)
            .write_state(&retail)
            .expect("retail sidecar");
        let mut forever = State::new(&FlavorId::Forever);
        forever.addons.insert(
            "Questie".into(),
            entry("github:Questie/Questie", "1.2.3", &["Questie"]),
        );
        forever.addons.insert(
            "DevAddon".into(),
            entry("link:D:\\dev\\addons", "9.9.9-dev", &["DevAddon"]),
        );
        let mut zip = entry("C:\\downloads\\ZipOne-1.0.zip", "1.0", &["ZipOne"]);
        zip.pinned = Some("1.0".into());
        zip.ignored = true;
        forever.addons.insert("ZipOne".into(), zip);
        FlavorHome::at(&home, &FlavorId::Forever)
            .write_state(&forever)
            .expect("forever sidecar");

        // The cache an `add` leaves: the release list and the current release's `release.json`.
        let source = fixture_source(&home);
        let spec = SourceSpec::parse("github:Questie/Questie").expect("spec");
        let releases = source
            .releases("Questie", "Questie")
            .expect("the fixture list");
        let current = github::current_release(&releases).expect("a current release");
        source
            .release_json(&spec, current)
            .expect("the fixture release.json");
        fixture
    }

    fn game_root(&self) -> GameRoot {
        GameRoot {
            path: self.root(),
            origin: RootOrigin::Remembered,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn entry(source: &str, version: &str, folders: &[&str]) -> Entry {
    Entry {
        source: source.to_string(),
        version: version.to_string(),
        previous_version: None,
        folders: folders.iter().map(|folder| folder.to_string()).collect(),
        files: BTreeMap::new(),
        installed_at: "2026-10-22T10:00:00Z".to_string(),
        nolib: None,
        pinned: None,
        ignored: false,
        channel: Channel::Stable,
    }
}

/// The GitHub source over the fixture answers, with its cache under the home folder.
fn fixture_source(home: &Path) -> GitHub<FolderHttp> {
    let answers =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures/releases");
    GitHub::new(
        FolderHttp::new(answers),
        None,
        Some(home.join(github::CACHE_SUBDIR)),
    )
}

fn flavor<'a>(flavors: &'a [FlavorView], id: &str) -> &'a FlavorView {
    flavors
        .iter()
        .find(|flavor| flavor.id == id)
        .unwrap_or_else(|| panic!("no flavor {id}"))
}

fn row<'a>(flavor: &'a FlavorView, folder: &str) -> &'a Row {
    flavor
        .addons
        .iter()
        .find(|row| row.folder == folder)
        .unwrap_or_else(|| panic!("no row {folder} in {}", flavor.id))
}

#[test]
fn the_flavors_of_a_root_come_with_a_banner() {
    let fixture = Fixture::game();
    let home = fixture.home();
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
    assert_eq!(answer.message.kind, "banner");
    assert_eq!(
        answer.message.plain,
        format!("2 flavors under {}, 8 addons", fixture.root().display())
    );
    assert_eq!(
        answer.root.as_deref(),
        Some(fixture.root().to_str().expect("utf-8"))
    );
    let ids: Vec<&str> = answer
        .flavors
        .iter()
        .map(|flavor| flavor.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["wowf", "wow"],
        "the flavor folders sorted by folder name"
    );
    let retail = flavor(&answer.flavors, "wow");
    assert_eq!(retail.name, "retail");
    assert_eq!(retail.folder, "_retail_");
    assert_eq!(retail.version.as_deref(), Some("12.1.5.70448"));
    assert_eq!(retail.interface, Some(120105));
    assert_eq!(retail.version_source, Some("build-info"));
    assert_eq!(retail.version_file.as_deref(), Some(".build.info"));
    assert!(retail.curse_client);
    assert!(retail.warnings.is_empty(), "{:?}", retail.warnings);
    assert!(!flavor(&answer.flavors, "wowf").curse_client);
}

#[test]
fn the_retail_rows_carry_their_states_facts() {
    let fixture = Fixture::game_named("retail");
    let home = fixture.home();
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(true),
    );
    assert_eq!(answer.message.murloc, "", "a serious voice");
    let retail = flavor(&answer.flavors, "wow");

    let questie = row(retail, "Questie");
    assert_eq!(
        questie.name, "Questie",
        "the title without its color escapes"
    );
    assert!(questie.managed && !questie.guessed && !questie.linked);
    assert_eq!(
        questie.version.as_deref(),
        Some("1.2.2"),
        "the sidecar's version"
    );
    let source = questie.source.as_ref().expect("a source");
    assert_eq!(
        (source.kind, source.id.as_str()),
        ("github", "Questie/Questie")
    );
    let update = questie
        .update
        .as_ref()
        .expect("the cached release is newer");
    assert_eq!(update.version, "1.2.3");
    assert_eq!(update.release, "v1.2.3");
    assert!(!update.prerelease);
    assert_eq!(questie.forever, None, "no Forever notes outside Forever");

    let hinted = row(retail, "Hinted");
    assert!(!hinted.managed && hinted.guessed);
    let source = hinted.source.as_ref().expect("a hinted source");
    assert_eq!(
        (source.kind, source.id.as_str()),
        ("wago", "aB1cD2eF"),
        "Wago before CurseForge (D-002)"
    );
    assert_eq!(
        hinted.version.as_deref(),
        Some("2.0.0"),
        "the TOC's version"
    );

    let needy = row(retail, "Needy");
    assert_eq!(
        needy.missing,
        ["LibGone", "LibInside"],
        "Plain is an addon of its own, Blizzard_Something the client's (task 064), LibInside only embedded (task 076)"
    );
    assert_eq!(needy.name, "Needy", "no title: the folder");

    let plain = row(retail, "Plain");
    assert!(!plain.managed && !plain.guessed && plain.source.is_none() && plain.update.is_none());
}

#[test]
fn every_forever_row_carries_its_forever_status() {
    let fixture = Fixture::game_named("forever");
    let home = fixture.home();
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
    let forever = flavor(&answer.flavors, "wowf");
    assert!(forever.addons.iter().all(|row| row.forever.is_some()));

    let questie = row(forever, "Questie");
    assert_eq!(questie.update, None, "installed at the current release");
    let status = questie.forever.as_ref().expect("a status");
    assert_eq!(
        status.build.as_str(),
        "yes",
        "the cached release.json names forever"
    );

    let dev = row(forever, "DevAddon");
    assert!(dev.linked && dev.managed);
    let source = dev.source.as_ref().expect("the link's source");
    assert_eq!(
        (source.kind, source.id.as_str()),
        ("linked", "D:\\dev\\addons")
    );

    let zip = row(forever, "ZipOne");
    let source = zip.source.as_ref().expect("the zip's source");
    assert_eq!((source.kind, source.id.as_str()), ("zip", "ZipOne-1.0.zip"));
    assert_eq!(zip.pinned.as_deref(), Some("1.0"));
    assert!(zip.ignored);
    assert_eq!(
        zip.forever.as_ref().expect("a status").build.as_str(),
        "unknown"
    );

    let loose = row(forever, "Loose");
    assert!(loose.guessed && !loose.managed);
    assert_eq!(loose.source.as_ref().expect("a hint").kind, "curseforge");
    assert_eq!(loose.version, None);
}

#[test]
fn a_row_and_a_flavor_serialize_with_the_keys_of_the_contract() {
    let fixture = Fixture::game_named("keys");
    let home = fixture.home();
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
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
    // The contract of ui/mock-data.js (ROW_KEYS and FLAVOR_KEYS of tests/ui/mock-data.test.js).
    let mut row_keys = vec![
        "folder", "name", "version", "linked", "source", "managed", "guessed", "update", "check",
        "channel", "pinned", "ignored", "forever", "missing",
    ];
    row_keys.sort_unstable();
    let mut flavor_keys = vec![
        "id",
        "name",
        "folder",
        "version",
        "interface",
        "version_source",
        "version_file",
        "curse_client",
        "addons_folder",
        "warnings",
        "addons",
    ];
    flavor_keys.sort_unstable();
    assert_eq!(keys(&value), ["flavors", "message", "root", "root_origin"]);
    assert_eq!(value["root_origin"], "remembered");
    for flavor in value["flavors"].as_array().expect("flavors") {
        assert_eq!(keys(flavor), flavor_keys);
        for row in flavor["addons"].as_array().expect("addons") {
            assert_eq!(keys(row), row_keys);
        }
    }
    let forever = &value["flavors"][0]["addons"][0]["forever"];
    for key in ["build", "note", "verified", "source", "stale"] {
        assert!(forever.get(key).is_some(), "forever.{key}: {forever}");
    }
}

#[test]
fn without_a_cached_list_no_update_is_known() {
    let fixture = Fixture::game_named("nocache");
    let home = fixture.home();
    std::fs::remove_dir_all(home.join("cache")).expect("the cache");
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
    let retail = flavor(&answer.flavors, "wow");
    assert_eq!(row(retail, "Questie").update, None);
    let forever = flavor(&answer.flavors, "wowf");
    let status = row(forever, "Questie").forever.clone().expect("a status");
    assert_eq!(status.build.as_str(), "unknown");
}

#[test]
fn without_a_home_folder_every_addon_is_unmanaged() {
    let fixture = Fixture::game_named("nohome");
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        None,
        Voice::new(false),
    );
    let retail = flavor(&answer.flavors, "wow");
    assert!(retail.addons.iter().all(|row| !row.managed));
    assert!(
        retail
            .warnings
            .iter()
            .any(|warning| warning.contains("no mrgl home folder")),
        "{:?}",
        retail.warnings
    );
}

#[test]
fn no_root_is_nothing_and_a_folder_without_flavors_is_an_error() {
    let answer = list::load(
        None,
        vec!["unreadable config: x".into()],
        None,
        Voice::new(false),
    );
    assert_eq!(answer.message.kind, "nothing");
    assert!(
        answer.message.plain.starts_with("no game root: "),
        "{}",
        answer.message.plain
    );
    assert!(
        answer.message.plain.ends_with("; unreadable config: x"),
        "{}",
        answer.message.plain
    );
    assert!(answer.flavors.is_empty());

    let fixture = Fixture::new("empty-root");
    let answer = list::load(
        Some(GameRoot {
            path: fixture.dir.clone(),
            origin: RootOrigin::Environment,
        }),
        Vec::new(),
        None,
        Voice::new(false),
    );
    assert_eq!(answer.message.kind, "error");
    assert_eq!(answer.root_origin, Some(RootOrigin::Environment));
    assert!(answer.flavors.is_empty());
}

#[test]
fn a_discovered_root_says_where_it_was_found() {
    let fixture = Fixture::game_named("discovered");
    let answer = list::load(
        Some(GameRoot {
            path: fixture.root(),
            origin: RootOrigin::Registry,
        }),
        Vec::new(),
        None,
        Voice::new(false),
    );
    assert!(
        answer.message.plain.contains(
            "(found in the launcher's registry key; choose the game folder to remember it)"
        ),
        "{}",
        answer.message.plain
    );
}

#[test]
fn a_chosen_flavor_folder_remembers_its_game_root() {
    let fixture = Fixture::game_named("choose");
    let home = fixture.home();
    let config = home.join(home::CONFIG_FILE);
    let choice = commands::choose(
        &fixture.root().join("_forever_"),
        Some(&config),
        Some(&home),
        Voice::new(false),
    );
    let answer = choice.flavors.expect("the flavors of the chosen root");
    assert_eq!(answer.message.kind, "success");
    assert!(
        answer.message.plain.starts_with("game root saved: ")
            && answer
                .message
                .plain
                .ends_with("(flavors: forever, retail), 8 addons"),
        "{}",
        answer.message.plain
    );
    let saved = home::read_config_at(&config).expect("the config").config;
    assert_eq!(saved.root.as_deref(), Some(fixture.root().as_path()));
}

#[test]
fn a_folder_that_is_no_game_root_is_refused_and_nothing_is_saved() {
    let fixture = Fixture::new("choose-wrong");
    let home = fixture.home();
    let config = home.join(home::CONFIG_FILE);
    let choice = commands::choose(&fixture.dir, Some(&config), Some(&home), Voice::new(false));
    assert!(choice.flavors.is_none());
    assert_eq!(choice.message.expect("why").kind, "error");
    assert!(!config.exists());
}

#[test]
fn a_resolve_hands_over_the_sources_pick() {
    let fixture = Fixture::game_named("resolve");
    let home = fixture.home();
    let source = fixture_source(&home);
    let answer = list::resolve(
        &source,
        &fixture.root().join("_retail_"),
        "Questie",
        Some(&home),
        Voice::new(false),
    );
    let pick = answer.pick.expect("a pick");
    assert_eq!(pick.source, "github:Questie/Questie");
    assert_eq!(pick.version, "1.2.3");
    assert_eq!(pick.flavor, "mainline");
    assert_eq!(answer.message.kind, "success");
    assert!(
        answer.message.plain.starts_with("Questie 1.2.2 → 1.2.3: "),
        "{}",
        answer.message.plain
    );

    let unmanaged = list::resolve(
        &source,
        &fixture.root().join("_retail_"),
        "Plain",
        Some(&home),
        Voice::new(false),
    );
    assert!(unmanaged.pick.is_none());
    assert_eq!(unmanaged.message.kind, "error");
    assert_eq!(
        unmanaged.message.plain,
        "Plain is not managed by mrgl in _retail_"
    );
}

#[test]
fn the_rows_take_the_offline_checks_answer_in_its_version_order() {
    let fixture = Fixture::game_named("order");
    let home = fixture.home();
    // A Forever addon installed at a prerelease while it follows the stable channel: the cached list puts
    // 1.1.0-beta above the current stable 1.0.0, so it is newer than the release, not an update (task 068).
    fixture.addon(
        "_forever_",
        "Switched",
        "## Interface: 16001\n## Version: 1.1.0-beta\n",
    );
    let flavor_home = FlavorHome::at(&home, &FlavorId::Forever);
    let mut forever = flavor_home
        .read_state(&FlavorId::Forever)
        .expect("sidecar")
        .state;
    forever.addons.insert(
        "Switched".into(),
        entry("github:Example/Prerelease", "1.1.0-beta", &["Switched"]),
    );
    // Task 077: the same prerelease, pinned: the pin is read against the same order, so no release is out.
    fixture.addon(
        "_forever_",
        "Held",
        "## Interface: 16001\n## Version: 1.1.0-beta\n",
    );
    let mut held = entry("github:Example/Prerelease", "1.1.0-beta", &["Held"]);
    held.pin(None);
    forever.addons.insert("Held".into(), held);
    flavor_home.write_state(&forever).expect("forever sidecar");
    let source = fixture_source(&home);
    let spec = SourceSpec::parse("github:Example/Prerelease").expect("spec");
    let releases = source
        .releases("Example", "Prerelease")
        .expect("the fixture list");
    // The `release.json` of each release the fixture has one for (a draft's is not served).
    for release in &releases {
        let _ = source.release_json(&spec, release);
    }
    let cache = home.join("cache/github/Questie/Questie/releases.json");
    let cached = std::fs::read(&cache).expect("the cached list");

    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
    let forever = flavor(&answer.flavors, "wowf");
    let switched = row(forever, "Switched");
    assert_eq!(
        switched.update, None,
        "a newer installed version is no update"
    );
    let check = switched.check.as_ref().expect("the offline check");
    assert_eq!(check.label, "newer than the release");
    assert_eq!(check.latest.as_deref(), Some("1.0.0"));
    assert_eq!(
        serde_json::to_value(check).expect("serializes")["state"],
        "installed-newer"
    );
    assert_eq!(
        switched.channel.map(|channel| channel.as_str()),
        Some("stable")
    );
    let held = row(forever, "Held");
    let check = held.check.as_ref().expect("the offline check of a pin");
    assert_eq!(
        (check.label, check.latest.as_deref(), check.order),
        ("pinned", Some("1.0.0"), Some(VersionOrder::Newer))
    );
    assert_eq!(
        check.line,
        "Held  1.1.0-beta  pinned at 1.1.0-beta, newer than the current release 1.0.0, left alone"
    );
    assert_eq!(
        serde_json::to_value(check).expect("serializes")["order"],
        "newer"
    );
    let questie = row(forever, "Questie");
    assert_eq!(
        questie.check.as_ref().map(|check| check.label),
        Some("up to date")
    );
    assert!(forever.addons_folder);

    let retail = flavor(&answer.flavors, "wow");
    let questie = row(retail, "Questie");
    assert_eq!(
        questie.check.as_ref().map(|check| check.label),
        Some("update available")
    );
    assert_eq!(
        row(retail, "Plain").check,
        None,
        "an unmanaged addon has no check"
    );
    assert_eq!(row(retail, "Plain").channel, None);

    assert_eq!(
        std::fs::read(&cache).expect("the cached list"),
        cached,
        "the list reads the cache and never stamps it as confirmed"
    );
}

#[test]
fn a_flavor_without_an_addons_folder_says_so() {
    let fixture = Fixture::game_named("no-addons");
    std::fs::remove_dir_all(fixture.root().join("_forever_/Interface")).expect("the folder");
    let home = fixture.home();
    let answer = list::load(
        Some(fixture.game_root()),
        Vec::new(),
        Some(&home),
        Voice::new(false),
    );
    let forever = flavor(&answer.flavors, "wowf");
    assert!(!forever.addons_folder && forever.addons.is_empty());
    assert!(flavor(&answer.flavors, "wow").addons_folder);
    assert!(
        list::no_addons_folder_line("_forever_").starts_with(
            "no Interface\\AddOns folder in _forever_: the client has not been started"
        )
    );
}
