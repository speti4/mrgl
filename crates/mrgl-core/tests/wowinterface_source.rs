//! The WoWInterface source over the fixture answers under `tests/fixtures/sources/` (task 094), served by the
//! folder stub, so no test opens a socket: `api.mmoui.com/v3/game/WOW/filedetails/<id>.json` per project, a
//! trimmed `filelist.json` with the compatibility lists, and the stored zips behind the `UIDownload` addresses
//! under `cdn.wowinterface.com/downloads/` (written by `zips/make-zips.py`). The projects: `1001` lists retail,
//! Classic Era and Cataclysm Classic, `1002` no compatibility at all, `1003` waits for approval, `1004` names an
//! MD5 its file does not have, `1005` lists a (made-up) Forever version. The cache tests use a temporary
//! folder of their own.

use std::path::{Path, PathBuf};

use mrgl_core::error::Error;
use mrgl_core::flavor::{BuildVersion, FlavorId};
use mrgl_core::forever::ForeverBuild;
use mrgl_core::sources::wowinterface::{self, WowInterface};
use mrgl_core::sources::{Channel, FolderHttp, Source, SourceSpec, Target, md5_hex};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources")
}

fn wowi() -> WowInterface<FolderHttp> {
    WowInterface::new(FolderHttp::new(fixtures()), None)
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
    let dir = std::env::temp_dir().join(format!("mrgl-wowi-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_compatibility_list_of_the_flavors_game_picks_by_metadata() {
    let wowi = wowi();
    let spec = spec("wowi:1001");
    let retail = wowi
        .resolve(&spec, &target(FlavorId::Retail, Some("12.1.5.70448")))
        .expect("retail");
    assert_eq!(retail.source, "wowi:1001");
    assert_eq!(retail.release, "v2.0.0");
    assert_eq!(
        retail.version, "2.0.0",
        "the leading v dropped, as for a tag"
    );
    assert_eq!(retail.asset, "MrglCompat-2.0.0.zip");
    assert_eq!(
        retail.url,
        "https://cdn.wowinterface.com/downloads/getfile.php?id=1001&d=1785000000&minion"
    );
    assert_eq!(
        retail.md5.as_deref(),
        Some("feae3d188db602d6902af44c109f9454")
    );
    assert_eq!(retail.flavor, "mainline");
    assert!(!retail.guess, "{}", retail.reason);
    assert_eq!(
        retail.reason,
        "WoWInterface lists compatibility 12.0.7 (Made-up Patch)"
    );
    assert_eq!(retail.interface, None, "the list names no interface");
    assert_eq!(retail.published.as_deref(), Some("2026-07-25T17:20:00Z"));
    assert_eq!(retail.notes.as_deref(), Some("2.0.0"));
    assert_eq!(
        retail.page.as_deref(),
        Some("https://www.wowinterface.com/downloads/info1001-MrglCompat.html")
    );
    assert_eq!(retail.forever_build, ForeverBuild::Unknown);
    assert_eq!(retail.stale, None);

    let era = wowi
        .resolve(&spec, &target(FlavorId::ClassicEra, None))
        .expect("era");
    assert!(!era.guess);
    assert_eq!(era.flavor, "classic");
    assert_eq!(
        era.reason,
        "WoWInterface lists compatibility 1.15.7 (Classic)"
    );
    let cata = wowi
        .resolve(&spec, &target(FlavorId::Classic, Some("4.4.2.60000")))
        .expect("cata");
    assert!(!cata.guess);
    assert_eq!(cata.flavor, "cata");
    // Mists Classic is not on the list: the one file is a guess there.
    let mists = wowi
        .resolve(&spec, &target(FlavorId::Classic, Some("5.5.4.70531")))
        .expect("mists");
    assert!(mists.guess);
    assert_eq!(
        mists.reason,
        "WoWInterface lists no classic compatibility (it lists 12.0.7, 1.15.7, 4.4.2); the project's one file is taken, a guess"
    );
    // The prerelease channel takes the same one file.
    let beta = wowi
        .resolve(
            &spec,
            &target(FlavorId::Retail, None).with_channel(Channel::Prerelease),
        )
        .expect("beta");
    assert_eq!(beta.version, "2.0.0");
    assert!(!beta.prerelease);
}

#[test]
fn without_a_list_and_for_forever_the_one_file_is_a_guess() {
    let wowi = wowi();
    let none = wowi
        .resolve(&spec("wowi:1002"), &target(FlavorId::Retail, None))
        .expect("a pick");
    assert!(none.guess);
    assert_eq!(
        none.reason,
        "WoWInterface lists no compatibility for the project; the project's one file is taken, a guess"
    );
    assert_eq!(none.version, "1.0.0");
    let forever = wowi
        .resolve(
            &spec("wowi:1001"),
            &target(FlavorId::Forever, Some("1.60.1.70245")),
        )
        .expect("a pick");
    assert!(forever.guess);
    assert_eq!(forever.flavor, "forever");
    assert_eq!(
        forever.reason,
        "WoWInterface lists no forever compatibility (it lists 12.0.7, 1.15.7, 4.4.2); the project's one file is taken, a guess"
    );
    // The fixture answer classifies the Forever build as unknown: nothing names Forever, nothing inferred.
    assert_eq!(forever.forever_build, ForeverBuild::Unknown);
}

#[test]
fn a_forever_compatibility_picks_by_metadata_and_says_yes() {
    let pick = wowi()
        .resolve(
            &spec("wowi:1005"),
            &target(FlavorId::Forever, Some("1.60.1.70245")),
        )
        .expect("a pick");
    assert!(!pick.guess, "{}", pick.reason);
    assert_eq!(
        pick.reason,
        "WoWInterface lists compatibility 1.60.1 (Made-up Forever Patch)"
    );
    assert_eq!(pick.forever_build, ForeverBuild::Yes);
}

#[test]
fn a_pending_file_is_reported_and_never_picked() {
    let wowi = wowi();
    let error = wowi
        .resolve(&spec("wowi:1003"), &target(FlavorId::Retail, None))
        .expect_err("pending");
    assert!(matches!(error, Error::ReleasePending { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        "the newest file of wowi:1003 waits for approval on WoWInterface: nothing is installed until it is approved"
    );
    assert!(
        wowi.http()
            .requests()
            .iter()
            .all(|request| !request.url.contains("cdn.wowinterface.com")),
        "nothing downloaded"
    );
}

#[test]
fn the_download_is_checked_against_the_md5_before_it_is_written() {
    let dir = temp_dir("fetch");
    let wowi = wowi();
    let pick = wowi
        .resolve(&spec("wowi:1001"), &target(FlavorId::Retail, None))
        .expect("a pick");
    let to = dir.join("nested").join(&pick.asset);
    wowi.fetch(&pick, &to).expect("fetched");
    let bytes = std::fs::read(&to).expect("written");
    assert_eq!(md5_hex(&bytes), "feae3d188db602d6902af44c109f9454");
    // No key and no header goes with a WoWInterface request.
    assert!(
        wowi.http()
            .requests()
            .iter()
            .all(|request| request.headers.is_empty())
    );

    let bad = wowi
        .resolve(&spec("wowi:1004"), &target(FlavorId::Retail, None))
        .expect("a pick");
    let to = dir.join(&bad.asset);
    let error = wowi.fetch(&bad, &to).expect_err("a wrong MD5");
    assert!(
        matches!(error, Error::AssetChecksumMismatch { .. }),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "refused MrglBadSum-1.0.0.zip of wowi:1004 1.0.0: its MD5 is bb4291ce1f730854f286d0707adf9a9d, the source names 0123456789abcdef0123456789abcdef; the download was damaged or the file changed, try again"
    );
    assert!(!to.exists(), "nothing written for the engine to open");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_project_is_a_404_with_a_hint() {
    let error = wowi()
        .resolve(&spec("wowi:9999"), &target(FlavorId::Retail, None))
        .expect_err("no such project");
    assert_eq!(
        error.to_string(),
        "WoWInterface answered 404 for https://api.mmoui.com/v3/game/WOW/filedetails/9999.json: no such project, or it was removed"
    );
}

#[test]
fn the_file_list_is_read_once_a_run_and_once_a_day_from_the_cache() {
    let cache = temp_dir("list");
    let wowi = WowInterface::new(FolderHttp::new(fixtures()), Some(cache.clone()));
    let retail = target(FlavorId::Retail, None);
    for id in ["wowi:1001", "wowi:1002", "wowi:1005"] {
        wowi.resolve(&spec(id), &retail).expect("a pick");
    }
    let lists = |wowi: &WowInterface<FolderHttp>| {
        wowi.http()
            .requests()
            .iter()
            .filter(|request| request.url.ends_with("/filelist.json"))
            .count()
    };
    assert_eq!(lists(&wowi), 1, "once a run");
    let cached: serde_json::Value = serde_json::from_slice(
        &std::fs::read(cache.join(wowinterface::FILE_LIST_CACHE)).expect("the cached list"),
    )
    .expect("JSON");
    assert!(cached["checked_at"].is_string());
    assert_eq!(
        cached["projects"]["1006"]["dirs"],
        serde_json::json!(["MrglElsewhere", "MrglElsewhere_Options"]),
        "the list keeps every project's folders"
    );
    assert_eq!(
        cached["projects"]["1002"]["compatibility"],
        serde_json::json!([])
    );
    // A second run within the day reads the cached list and asks for the project alone.
    let again = WowInterface::new(FolderHttp::new(fixtures()), Some(cache.clone()));
    again.resolve(&spec("wowi:1001"), &retail).expect("a pick");
    assert_eq!(lists(&again), 0, "the cached list is a day fresh");
    assert_eq!(
        again
            .listing("1006")
            .expect("the list")
            .expect("the project")
            .dirs,
        ["MrglElsewhere", "MrglElsewhere_Options"]
    );
    // A list older than a day is asked for again.
    let mut old = cached.clone();
    old["checked_at"] = serde_json::json!("2020-01-01T00:00:00Z");
    std::fs::write(
        cache.join(wowinterface::FILE_LIST_CACHE),
        serde_json::to_vec(&old).expect("JSON"),
    )
    .expect("written");
    let stale = WowInterface::new(FolderHttp::new(fixtures()), Some(cache.clone()));
    stale.resolve(&spec("wowi:1001"), &retail).expect("a pick");
    assert_eq!(lists(&stale), 1);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn offline_the_cached_answer_stands_and_the_forever_signal_needs_no_request() {
    let cache = temp_dir("offline");
    let online = WowInterface::new(FolderHttp::new(fixtures()), Some(cache.clone()));
    let forever = target(FlavorId::Forever, None);
    let fresh = online
        .resolve(&spec("wowi:1005"), &forever)
        .expect("a pick");
    online
        .resolve(&spec("wowi:1001"), &forever)
        .expect("a pick");
    assert!(cache.join("1005.json").is_file());

    // A stub folder that does not exist is no network at all.
    let offline = WowInterface::new(
        FolderHttp::new(cache.join("no-network")),
        Some(cache.clone()),
    );
    let pick = offline
        .resolve(&spec("wowi:1005"), &forever)
        .expect("from the cache");
    let stale = pick.stale.clone().expect("a stale pick");
    assert!(stale.checked_at.is_some());
    assert_eq!(
        stale.reason,
        "cannot reach https://api.mmoui.com/v3/game/WOW/filedetails/1005.json: offline, the stub folder is missing"
    );
    assert_eq!(pick.version, fresh.version);
    assert!(!pick.guess, "the cached list still names Forever");
    assert!(
        offline
            .resolve(&spec("wowi:1002"), &forever)
            .is_err_and(|error| matches!(error, Error::Network { .. })),
        "nothing cached for it: the network error stands"
    );

    // The Forever signal of `list`, from the cache alone.
    let cached = WowInterface::new(
        FolderHttp::new(cache.join("no-network")),
        Some(cache.clone()),
    );
    assert_eq!(
        cached.cached_forever_build(&spec("wowi:1005"), Channel::Stable),
        ForeverBuild::Yes
    );
    assert_eq!(
        cached.cached_forever_build(&spec("wowi:1001"), Channel::Stable),
        ForeverBuild::Unknown
    );
    assert!(cached.http().requests().is_empty(), "never a request");
    assert_eq!(
        wowi().cached_forever_build(&spec("wowi:1005"), Channel::Stable),
        ForeverBuild::Unknown,
        "nothing cached"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn another_sources_spec_is_not_this_sources_to_answer() {
    let wowi = wowi();
    let github = spec("github:Questie/Questie");
    assert_eq!(
        wowi.missing(&github).as_deref(),
        Some("the WoWInterface source answers wowi: specs only")
    );
    assert_eq!(wowi.missing(&spec("wowi:1001")), None);
    assert!(matches!(
        wowi.resolve(&github, &target(FlavorId::Retail, None)),
        Err(Error::SourceNotConfigured { .. })
    ));
}
