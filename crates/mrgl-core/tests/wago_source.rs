//! The Wago source over the fixture answers under `tests/fixtures/sources/addons.wago.io/` (task 095), served by
//! the folder stub with its query mapping (`addons/<id>@game_version=<v>.json`), so no test opens a socket and
//! no real key is used: the tests pass a made-up one. The addons: `MrglWg01` has a Forever and a retail
//! release (and a zip behind each), `MrglWg02` answers for Forever without a release, `MrglWg03` has the three
//! stabilities for retail and no stable one for Forever, `MrglWg04` answers 401, `MrglWg05` answers with no
//! request left; the keyless slug list of `forever` names `MrglWg01`, `MrglWg03` and `MrglWg05`.

use std::path::{Path, PathBuf};

use mrgl_core::error::Error;
use mrgl_core::flavor::{BuildVersion, FlavorId};
use mrgl_core::forever::ForeverBuild;
use mrgl_core::keys;
use mrgl_core::sources::wago::{self, Wago};
use mrgl_core::sources::{Channel, FolderHttp, Source, SourceKind, SourceSpec, Target};

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
