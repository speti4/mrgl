//! The Wago source over the fixture answers under `tests/fixtures/sources/addons.wago.io/` (task 095), served by
//! the folder stub with its query mapping (`addons/<id>@game_version=<v>.json`), so no test opens a socket and
//! no real key is used: the tests pass a made-up one. The addons: `MrglWg01` has a Forever and a retail
//! release (and a zip behind each), `MrglWg02` answers for Forever without a release, `MrglWg03` has the three
//! stabilities for retail and no stable one for Forever, `MrglWg04` answers 401, `MrglWg05` answers with no
//! request left; the keyless slug list of `forever` names `MrglWg01`, `MrglWg03` and `MrglWg05`. The bulk
//! answers of task 106 (`addons/_recents@game_version=<v>.json`) hold `MrglWg01` and `MrglWg03` for Forever
//! and the first three for retail.

use std::path::{Path, PathBuf};

use mrgl_core::error::Error;
use mrgl_core::flavor::{BuildVersion, FlavorId, read_flavor};
use mrgl_core::forever::ForeverBuild;
use mrgl_core::install::{Engine, Metadata};
use mrgl_core::keys;
use mrgl_core::sources::wago::{self, Wago};
use mrgl_core::sources::{Channel, FolderHttp, Source, SourceKind, SourceSpec, Target};
use mrgl_core::update::{self, UpdateState};

/// A made-up key: no test reads or uses a real one.
const KEY: &str = "made-up-wago-key-for-tests";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources")
}

fn wago() -> Wago<FolderHttp> {
    Wago::new(FolderHttp::new(fixtures()), Some(KEY.to_string()), None)
}

fn spec(text: &str) -> SourceSpec {
    SourceSpec::parse(text).expect("a spec")
}

fn target(flavor: FlavorId, version: Option<&str>) -> Target {
    Target {
        version: version.and_then(BuildVersion::parse),
        ..Target::new(flavor)
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mrgl-wago-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn one_url_answers_forever_and_retail_from_two_files() {
    let wago = wago();
    let spec = spec("wago:MrglWg01");
    let forever = wago
        .resolve(&spec, &target(FlavorId::Forever, Some("1.60.1.70245")))
        .expect("forever");
    assert_eq!(forever.source, "wago:MrglWg01");
    assert_eq!(forever.version, "2.0.0");
    assert_eq!(forever.asset, "mrgl-wago-one-2.0.0.zip");
    assert_eq!(
        forever.url,
        "https://addons.wago.io/downloads/MrglWg01/2.0.0-forever.zip"
    );
    assert_eq!(forever.flavor, "forever");
    assert_eq!(forever.interface, Some(16001));
    assert!(!forever.guess);
    assert!(!forever.prerelease);
    assert_eq!(
        forever.reason,
        "Wago lists a stable release for forever (patch 1.60.1)"
    );
    assert_eq!(forever.notes.as_deref(), Some("2.0.0"));
    assert_eq!(
        forever.page.as_deref(),
        Some("https://addons.wago.io/addons/mrgl-wago-one")
    );
    assert_eq!(forever.forever_build, ForeverBuild::Yes);
    let retail = wago
        .resolve(&spec, &target(FlavorId::Retail, Some("12.1.5.70448")))
        .expect("retail");
    assert_eq!(
        retail.url,
        "https://addons.wago.io/downloads/MrglWg01/2.0.0-retail.zip"
    );
    assert_eq!(retail.interface, Some(120105), "the newest patch it names");
    assert_eq!(retail.flavor, "retail");
    // A retail resolve asks the keyless slug list for the Forever signal, without the key.
    assert_eq!(retail.forever_build, ForeverBuild::Yes);
    let requests = wago.http().requests();
    let urls: Vec<&str> = requests
        .iter()
        .map(|request| request.url.as_str())
        .collect();
    assert_eq!(
        urls,
        [
            "https://addons.wago.io/api/external/addons/MrglWg01?game_version=forever",
            "https://addons.wago.io/api/external/addons/MrglWg01?game_version=retail",
            "https://addons.wago.io/api/data/slugs?game_version=forever",
        ]
    );
    let bearer = format!("Bearer {KEY}");
    assert_eq!(
        requests[0].header_value("Authorization"),
        Some(bearer.as_str())
    );
    assert_eq!(requests[2].header_value("Authorization"), None, "keyless");
}

#[test]
fn the_three_stabilities_map_onto_the_channels() {
    let wago = wago();
    let spec = spec("wago:MrglWg03");
    let retail = target(FlavorId::Retail, None);
    let stable = wago.resolve(&spec, &retail).expect("stable");
    assert_eq!(
        (stable.version.as_str(), stable.prerelease),
        ("3.0.0", false)
    );
    let beta = wago
        .resolve(&spec, &retail.clone().with_channel(Channel::Prerelease))
        .expect("prerelease");
    assert_eq!(
        (beta.version.as_str(), beta.prerelease),
        ("3.2.0-alpha", true),
        "the newest of the three"
    );
    // No stable release for Forever: the stable channel takes the newest of the others and says so.
    let forever = wago
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("forever");
    assert_eq!(forever.version, "3.2.0-alpha");
    assert!(forever.prerelease);
    assert_eq!(
        forever.reason,
        "Wago lists no stable release for forever; the newest, the alpha release, is taken (patch 1.60.1)"
    );
    // A version asked for by name is any of the three recent ones.
    let exact = wago
        .resolve_version(&spec, &retail, "3.1.0-beta")
        .expect("listed");
    assert!(exact.prerelease);
    assert!(matches!(
        wago.resolve_version(&spec, &retail, "2.0.0"),
        Err(Error::VersionNotOffered { .. })
    ));
    // The progressing classic follows the build; without one its newest game type is a guess.
    let mists = wago
        .resolve(&spec, &target(FlavorId::Classic, Some("5.5.4.70531")))
        .expect("mists");
    assert_eq!((mists.flavor.as_str(), mists.guess), ("mop", false));
    assert_eq!(mists.interface, Some(50504));
    let unknown = wago
        .resolve(&spec, &target(FlavorId::Classic, None))
        .expect("a guess");
    assert!(unknown.guess);
    assert_eq!(
        unknown.reason,
        "Wago lists a stable release for mop (patch 5.5.4); the installed build is unknown, so the game type is a guess"
    );
    // A game version the addon has no answer for is Wago's 404.
    assert!(matches!(
        wago.resolve(&spec, &target(FlavorId::Classic, Some("4.4.2.1"))),
        Err(Error::SourceStatus { status: 404, .. })
    ));
}

#[test]
fn the_forever_signal_is_yes_with_a_release_and_no_without_one() {
    let cache = temp_dir("signal");
    let wago = Wago::new(
        FolderHttp::new(fixtures()),
        Some(KEY.into()),
        Some(cache.clone()),
    );
    let forever = target(FlavorId::Forever, None);
    wago.resolve(&spec("wago:MrglWg01"), &forever)
        .expect("a release");
    let error = wago
        .resolve(&spec("wago:MrglWg02"), &forever)
        .expect_err("no release for forever");
    assert_eq!(
        error.to_string(),
        "no asset for flavor forever in wago:MrglWg02 game version forever (Wago lists no stable, beta or alpha release for it)"
    );
    // From the cached answers alone, no request.
    let cached = Wago::new(
        FolderHttp::new(cache.join("no-network")),
        None,
        Some(cache.clone()),
    );
    assert_eq!(
        cached.cached_forever_build(&spec("wago:MrglWg01"), Channel::Stable),
        ForeverBuild::Yes
    );
    assert_eq!(
        cached.cached_forever_build(&spec("wago:MrglWg02"), Channel::Stable),
        ForeverBuild::No
    );
    assert_eq!(
        cached.cached_forever_build(&spec("wago:MrglWg05"), Channel::Stable),
        ForeverBuild::Unknown,
        "nothing cached for it"
    );
    assert!(cached.http().requests().is_empty());
    // The keyless slug list answers without a key, and classifies both ways.
    let keyless = Wago::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    assert_eq!(
        keyless.forever_build("MrglWg05", Channel::Stable),
        ForeverBuild::Yes
    );
    assert_eq!(
        keyless.forever_build("MrglWg09", Channel::Stable),
        ForeverBuild::No
    );
    assert_eq!(
        keyless
            .id_of_slug("mrgl-wago-three", "forever")
            .expect("the list"),
        Some("MrglWg03".to_string())
    );
    assert!(
        keyless
            .http()
            .requests()
            .iter()
            .all(|request| request.header_value("Authorization").is_none())
    );
    assert!(cache.join("slugs-forever.json").is_file());
    // Once the slug list is cached, the signal needs no request either.
    let offline = Wago::new(
        FolderHttp::new(cache.join("no-network")),
        None,
        Some(cache.clone()),
    );
    assert_eq!(
        offline.cached_forever_build(&spec("wago:MrglWg05"), Channel::Stable),
        ForeverBuild::Yes
    );
    // No cache file carries the key.
    for entry in std::fs::read_dir(&cache).expect("the cache") {
        let text = std::fs::read_to_string(entry.expect("an entry").path()).expect("text");
        assert!(!text.contains(KEY), "{text}");
    }
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_401_names_the_key_and_no_key_is_not_configured() {
    let error = wago()
        .resolve(&spec("wago:MrglWg04"), &target(FlavorId::Forever, None))
        .expect_err("401");
    assert_eq!(
        error.to_string(),
        "Wago answered 401 for https://addons.wago.io/api/external/addons/MrglWg04?game_version=forever: MRGL_WAGO_KEY is missing or not accepted: the Wago API needs your own key, `mrgl key set wago` stores it"
    );
    assert!(!error.to_string().contains(KEY));
    let without = Wago::new(FolderHttp::new(fixtures()), None, None);
    let missing =
        "MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it";
    assert_eq!(
        without.missing(&spec("wago:MrglWg01")).as_deref(),
        Some(missing)
    );
    assert_eq!(
        without
            .resolve(&spec("wago:MrglWg01"), &target(FlavorId::Forever, None))
            .expect_err("no key")
            .to_string(),
        format!("source not configured: wago:MrglWg01 ({missing})")
    );
    assert!(
        without.http().requests().is_empty(),
        "nothing asked without a key"
    );
    assert_eq!(
        wago().missing(&spec("github:o/r")).as_deref(),
        Some("the Wago source answers wago: specs only")
    );
}

#[test]
fn the_key_goes_to_wagos_host_only_and_with_the_download() {
    let dir = temp_dir("fetch");
    let wago = wago();
    let pick = wago
        .resolve(&spec("wago:MrglWg01"), &target(FlavorId::Forever, None))
        .expect("a pick");
    let to = dir.join(&pick.asset);
    wago.fetch(&pick, &to).expect("fetched");
    assert!(to.is_file());
    let bearer = format!("Bearer {KEY}");
    let download = wago
        .http()
        .requests()
        .into_iter()
        .find(|request| request.url == pick.url)
        .expect("the download");
    assert_eq!(
        download.header_value("Authorization"),
        Some(bearer.as_str())
    );
    // A download link anywhere else gets no key.
    let mut elsewhere = pick.clone();
    elsewhere.url = "https://cdn.example.test/mrgl/MrglWg01.zip".to_string();
    assert!(matches!(
        wago.fetch(&elsewhere, &dir.join("elsewhere.zip")),
        Err(Error::AssetMissing { .. })
    ));
    for request in wago.http().requests() {
        let sent = request.header_value("Authorization").is_some();
        assert_eq!(
            sent,
            keys::may_send(SourceKind::Wago, &request.url) && !request.url.contains("/api/data/"),
            "{}",
            request.url
        );
        assert!(!sent || request.url.starts_with("https://addons.wago.io/"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_rate_limit_headers_feed_the_budget() {
    let wago = wago();
    let budget = wago.request_budget().expect("a budget");
    assert_eq!(
        (budget.limit, budget.sent, budget.remaining),
        (wago::LIMIT, 0, 60)
    );
    wago.resolve(&spec("wago:MrglWg05"), &target(FlavorId::Forever, None))
        .expect("a pick");
    let budget = wago.request_budget().expect("a budget");
    assert_eq!((budget.host, budget.sent, budget.remaining), ("Wago", 1, 0));
    let error = wago
        .resolve(&spec("wago:MrglWg04"), &target(FlavorId::Forever, None))
        .expect_err("401");
    assert!(matches!(error, Error::SourceStatus { status: 401, .. }));
    let budget = wago.request_budget().expect("a budget");
    assert_eq!(
        (budget.sent, budget.remaining),
        (2, 58),
        "the last answer's count"
    );
}

#[test]
fn offline_the_cached_answer_stands() {
    let cache = temp_dir("offline");
    let online = Wago::new(
        FolderHttp::new(fixtures()),
        Some(KEY.into()),
        Some(cache.clone()),
    );
    let forever = target(FlavorId::Forever, None);
    let fresh = online
        .resolve(&spec("wago:MrglWg01"), &forever)
        .expect("a pick");
    let offline = Wago::new(
        FolderHttp::new(cache.join("no-network")),
        Some(KEY.into()),
        Some(cache.clone()),
    );
    let pick = offline
        .resolve(&spec("wago:MrglWg01"), &forever)
        .expect("from the cache");
    assert_eq!(pick.version, fresh.version);
    let stale = pick.stale.expect("stale");
    assert_eq!(
        stale.reason,
        "cannot reach https://addons.wago.io/api/external/addons/MrglWg01?game_version=forever: offline, the stub folder is missing"
    );
    // The window's offline view has no key and reads the cache alone.
    let view = Wago::cached(FolderHttp::new(fixtures()), Some(cache.clone()));
    assert_eq!(view.missing(&spec("wago:MrglWg01")), None);
    let pick = view
        .resolve(&spec("wago:MrglWg01"), &forever)
        .expect("from the cache");
    assert_eq!(
        pick.stale.map(|stale| stale.reason).as_deref(),
        Some("MRGL_WAGO_KEY is not set in this view")
    );
    assert!(view.http().requests().is_empty(), "never a request");
    let _ = std::fs::remove_dir_all(&cache);
}

// --- task 106: the update check asks Wago in bulk, one request per game version ---

/// A temporary game root with a Forever and a retail flavor and a mrgl home folder, removed at the end.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let dir = temp_dir(name);
        let root = dir.join("World of Warcraft");
        std::fs::create_dir_all(&root).expect("the game root");
        std::fs::write(
            root.join(".build.info"),
            "Version!STRING:0|Product!STRING:0\n1.60.1.70245|wowf\n12.1.5.70448|wow\n",
        )
        .expect(".build.info");
        for (folder, product) in [("_forever_", "wowf"), ("_retail_", "wow")] {
            std::fs::create_dir_all(root.join(folder).join("Interface/AddOns"))
                .expect("the AddOns folder");
            std::fs::write(
                root.join(folder).join(".flavor.info"),
                format!("Product Flavor!STRING:0\n{product}\n"),
            )
            .expect(".flavor.info");
        }
        Sandbox { dir }
    }

    /// The engine of a flavor folder with its addons installed from the fixture zips under `wago:` specs.
    fn engine(&self, folder: &str, addons: &[(&str, &str, &str)]) -> Engine {
        let flavor =
            read_flavor(&self.dir.join("World of Warcraft").join(folder)).expect("the flavor");
        let engine = Engine::with_home(flavor, &self.dir.join("home"));
        let zips = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zips");
        for (zip, source, version) in addons {
            engine
                .install(
                    &zips.join(zip),
                    &Metadata {
                        source: source.to_string(),
                        version: Some(version.to_string()),
                        name: None,
                        nolib: None,
                    },
                )
                .unwrap_or_else(|error| panic!("installing {zip}: {error}"));
        }
        engine
    }

    fn cache(&self) -> PathBuf {
        self.dir.join("home").join(wago::CACHE_SUBDIR)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The ids a bulk request asked about, from its body.
fn bulk_ids(request: &mrgl_core::sources::Request) -> Vec<String> {
    let body: serde_json::Value =
        serde_json::from_slice(request.body.as_deref().expect("a POST")).expect("a JSON body");
    body["addons"]
        .as_array()
        .expect("the ids")
        .iter()
        .map(|id| id.as_str().expect("an id").to_string())
        .collect()
}

#[test]
fn the_update_check_asks_wago_once_per_game_version_and_caches_each_answer() {
    let sandbox = Sandbox::new("bulk");
    let addons = [
        ("one-folder-1.0.0.zip", "wago:MrglWg01", "1.0.0"),
        ("several-folders-2.0.0.zip", "wago:MrglWg02", "0.9.0"),
        ("wrapped-4.0.0.zip", "wago:MrglWg03", "3.0.0"),
    ];
    let asked = ["MrglWg01", "MrglWg02", "MrglWg03"];
    for (folder, game_version, held) in [
        ("_forever_", "forever", ["MrglWg01", "MrglWg03"].as_slice()),
        (
            "_retail_",
            "retail",
            ["MrglWg01", "MrglWg02", "MrglWg03"].as_slice(),
        ),
    ] {
        let engine = sandbox.engine(folder, &addons);
        let http = FolderHttp::new(fixtures());
        let wago = Wago::new(&http, Some(KEY.to_string()), Some(sandbox.cache()));
        let check = update::check(&engine, &wago, None, &mut |_| {}).expect("checks");
        let requests = http.requests();
        let posts: Vec<_> = requests
            .iter()
            .filter(|request| request.body.is_some())
            .collect();
        assert_eq!(posts.len(), 1, "one POST for {game_version}");
        assert_eq!(
            posts[0].url,
            format!(
                "https://addons.wago.io/api/external/addons/_recents?game_version={game_version}"
            )
        );
        assert_eq!(bulk_ids(posts[0]), asked);
        assert_eq!(
            posts[0].header_value("Authorization"),
            Some(format!("Bearer {KEY}").as_str())
        );
        assert_eq!(
            posts[0].header_value("Content-Type"),
            Some("application/json")
        );
        // An addon the bulk answer lacks (MrglWg02 has no Forever release) is asked alone; the others are not.
        let singles: Vec<&str> = requests
            .iter()
            .filter(|request| request.body.is_none())
            .filter_map(|request| {
                request
                    .url
                    .strip_prefix("https://addons.wago.io/api/external/addons/")
            })
            .collect();
        let alone: Vec<String> = asked
            .iter()
            .filter(|id| !held.contains(id))
            .map(|id| format!("{id}?game_version={game_version}"))
            .collect();
        assert_eq!(singles, alone, "{game_version}");
        let states: Vec<(String, UpdateState)> = check
            .addons
            .iter()
            .map(|addon| (addon.source.clone(), addon.state))
            .collect();
        // Forever: MrglWg02 has no release, MrglWg03 only a beta and an alpha; retail: stable releases.
        let (two, three) = if game_version == "forever" {
            (UpdateState::SourceGone, UpdateState::UpdateAvailable)
        } else {
            (UpdateState::UpdateAvailable, UpdateState::UpToDate)
        };
        assert_eq!(
            states,
            [
                ("wago:MrglWg01".to_string(), UpdateState::UpdateAvailable),
                ("wago:MrglWg02".to_string(), two),
                ("wago:MrglWg03".to_string(), three),
            ],
            "{game_version}"
        );
        // The bulk answers land in the same cache files as a single answer.
        for id in held {
            let cached: serde_json::Value = serde_json::from_slice(
                &std::fs::read(sandbox.cache().join(format!("{id}.json"))).expect("the cache file"),
            )
            .expect("JSON");
            assert_eq!(
                cached["answers"][game_version]["addon"]["id"], *id,
                "{id} {game_version}"
            );
            assert!(cached["answers"][game_version]["checked_at"].is_string());
        }
    }
    // Both game versions sit side by side in one addon's cache file.
    let cached: serde_json::Value = serde_json::from_slice(
        &std::fs::read(sandbox.cache().join("MrglWg01.json")).expect("the cache file"),
    )
    .expect("JSON");
    assert!(cached["answers"]["forever"].is_object() && cached["answers"]["retail"].is_object());
}

#[test]
fn one_addons_check_keeps_the_single_request_and_a_used_up_budget_sends_no_bulk() {
    let sandbox = Sandbox::new("bulk-single");
    let engine = sandbox.engine(
        "_forever_",
        &[
            ("one-folder-1.0.0.zip", "wago:MrglWg01", "1.0.0"),
            ("wrapped-4.0.0.zip", "wago:MrglWg03", "3.0.0"),
        ],
    );
    let http = FolderHttp::new(fixtures());
    let wago = Wago::new(&http, Some(KEY.to_string()), None);
    let check = update::check(&engine, &wago, Some("OneFolder"), &mut |_| {}).expect("checks");
    assert_eq!(check.addons.len(), 1);
    let urls: Vec<String> = http
        .requests()
        .iter()
        .map(|request| request.url.clone())
        .collect();
    assert_eq!(
        urls,
        ["https://addons.wago.io/api/external/addons/MrglWg01?game_version=forever"],
        "`update <name>` asks the one addon"
    );

    // With no request left the bulk lookup is not sent, and the check stops before the limit.
    let http = FolderHttp::new(fixtures());
    let wago = Wago::new(&http, Some(KEY.to_string()), None);
    let forever = target(FlavorId::Forever, None);
    wago.resolve(&spec("wago:MrglWg05"), &forever)
        .expect("no request left after it");
    let check = update::check(&engine, &wago, None, &mut |_| {}).expect("checks");
    assert!(http.requests().iter().all(|request| request.body.is_none()));
    assert_eq!(http.requests().len(), 1, "nothing after the used-up budget");
    assert_eq!(check.count(UpdateState::RateLimited), 2);
    assert_eq!(check.stopped.len(), 1);
    assert_eq!(check.stopped[0].source, SourceKind::Wago);
    assert!(!wago.holds(&spec("wago:MrglWg01"), &forever));
}

#[test]
fn a_bulk_answer_is_held_for_the_run_and_answers_without_a_request() {
    let http = FolderHttp::new(fixtures());
    let wago = Wago::new(&http, Some(KEY.to_string()), None);
    let forever = target(FlavorId::Forever, None);
    let answered = wago
        .recents(&["MrglWg01", "MrglWg03", "MrglWg05"], "forever")
        .expect("the bulk answer");
    assert_eq!(answered, ["MrglWg01", "MrglWg03"]);
    assert!(wago.holds(&spec("wago:MrglWg01"), &forever));
    assert!(!wago.holds(&spec("wago:MrglWg05"), &forever));
    assert!(
        !wago.holds(&spec("wago:MrglWg01"), &target(FlavorId::Retail, None)),
        "held for its game version only"
    );
    let pick = wago
        .resolve(&spec("wago:MrglWg03"), &forever)
        .expect("from the bulk answer");
    assert_eq!(pick.version, "3.2.0-alpha");
    assert_eq!(http.requests().len(), 1, "the resolve sent nothing");
    let budget = wago.request_budget().expect("a budget");
    assert_eq!((budget.sent, budget.remaining), (1, 59), "the POST counts");
    // Without the key nothing is sent.
    let keyless = Wago::new(FolderHttp::new(fixtures()), None, None);
    assert!(matches!(
        keyless.recents(&["MrglWg01"], "forever"),
        Err(Error::SourceNotConfigured { .. })
    ));
    assert!(keyless.http().requests().is_empty());
}

// --- task 105: a Wago page address resolves to its `wago:<id>` ---

#[test]
fn a_page_address_turns_into_its_id_through_the_keyless_slug_list() {
    let forever = target(FlavorId::Forever, None);
    let page = spec("https://addons.wago.io/addons/mrgl-wago-one");
    assert_eq!(page.kind(), SourceKind::Wago);
    // The lookup needs no key; the source without one still says the key is missing for the install.
    let keyless = Wago::new(FolderHttp::new(fixtures()), None, None);
    assert_eq!(
        keyless.canonical_spec(&page, &forever).expect("the id"),
        spec("wago:MrglWg01")
    );
    assert!(keyless.missing(&page).is_some());
    assert!(
        keyless
            .http()
            .requests()
            .iter()
            .all(|request| request.header_value("Authorization").is_none()),
        "the slug list goes without the key"
    );
    assert_eq!(
        keyless
            .canonical_spec(
                &spec("https://addons.wago.io/addons/MRGL-Wago-Three"),
                &forever
            )
            .expect("the id"),
        spec("wago:MrglWg03"),
        "a slug in other letter case"
    );
    let error = keyless
        .canonical_spec(
            &spec("https://addons.wago.io/addons/no-such-addon"),
            &forever,
        )
        .expect_err("not on the list");
    assert_eq!(
        error.to_string(),
        "no Wago addon no-such-addon in the slug list of game version forever: it has no release for forever, or the address names no addon"
    );
    // Every other spec is its own, and a resolve of the page picks for the id.
    assert_eq!(
        keyless
            .canonical_spec(&spec("wago:abc"), &forever)
            .expect("itself"),
        spec("wago:abc")
    );
    let pick = wago().resolve(&page, &forever).expect("a pick");
    assert_eq!(pick.source, "wago:MrglWg01");
}
