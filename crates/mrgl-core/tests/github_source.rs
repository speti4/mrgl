//! The GitHub source over the fixture answers under `tests/fixtures/releases/` (task 003): one folder per
//! shape of a repository, served by the folder stub, so no test opens a socket. The cache tests use a
//! temporary folder of their own.

use std::path::{Path, PathBuf};

use mrgl_core::flavor::{BuildVersion, FlavorId};
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::{FolderHttp, Source, SourceSpec, Target};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/releases")
}

fn github() -> GitHub<FolderHttp> {
    GitHub::new(FolderHttp::new(fixtures()), None, None)
}

fn spec(text: &str) -> SourceSpec {
    SourceSpec::parse(text).expect("a spec")
}

fn target(flavor: FlavorId, version: Option<&str>) -> Target {
    Target {
        flavor,
        version: version.and_then(BuildVersion::parse),
    }
}

fn temp_cache(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mrgl-github-cache-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn questie_resolves_every_flavor_from_release_json() {
    let github = github();
    let spec = spec("github:Questie/Questie");
    let forever = github
        .resolve(&spec, &target(FlavorId::Forever, Some("1.60.1.70245")))
        .expect("forever");
    assert_eq!(
        forever.release, "v1.2.3",
        "the draft above it does not count"
    );
    assert_eq!(forever.version, "1.2.3");
    assert_eq!(forever.asset, "Questie-v1.2.3-forever.zip");
    assert_eq!(
        forever.url,
        "https://github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip"
    );
    assert_eq!(forever.flavor, "forever");
    assert_eq!(forever.interface, Some(16001));
    assert_eq!(forever.size, Some(2048));
    assert!(!forever.guess);
    assert!(!forever.prerelease);
    assert_eq!(
        forever.reason,
        "release.json names forever (interface 16001)"
    );
    assert_eq!(
        forever.summary(),
        "github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)"
    );

    let retail = github
        .resolve(&spec, &target(FlavorId::Retail, Some("12.1.5.70448")))
        .expect("retail");
    assert_eq!(
        retail.asset, "Questie-v1.2.3.zip",
        "the library zip, not the nolib one"
    );
    assert_eq!(retail.interface, Some(120105));
    assert!(!retail.guess);

    let era = github
        .resolve(&spec, &target(FlavorId::ClassicEra, Some("1.15.9.70000")))
        .expect("era");
    assert_eq!(era.asset, "Questie-v1.2.3-classic.zip");
    assert_eq!(era.flavor, "classic");

    let mists = github
        .resolve(&spec, &target(FlavorId::Classic, Some("5.5.4.70531")))
        .expect("mists");
    assert_eq!(mists.asset, "Questie-v1.2.3-mists.zip");
    assert!(!mists.guess);

    let classic_unknown = github
        .resolve(&spec, &target(FlavorId::Classic, None))
        .expect("classic without a build");
    assert_eq!(classic_unknown.asset, "Questie-v1.2.3-mists.zip");
    assert!(classic_unknown.guess);

    let cata = github
        .resolve(&spec, &target(FlavorId::Classic, Some("4.4.2.1")))
        .unwrap_err()
        .to_string();
    assert!(
        cata.starts_with("no asset for flavor classic in github:Questie/Questie v1.2.3 (release.json names classic, forever, mainline, mists; assets: "),
        "{cata}"
    );
}

#[test]
fn a_release_json_without_metadata_leaves_the_names() {
    let github = github();
    let spec = spec("github:Example/NoMetadata");
    let retail = github
        .resolve(&spec, &target(FlavorId::Retail, None))
        .expect("retail by name");
    assert_eq!(retail.asset, "NoMetadata-2.0.0.zip");
    assert_eq!(retail.version, "2.0.0");
    assert!(retail.guess);
    assert_eq!(
        retail.reason,
        "no release.json names retail; the asset name has no flavor suffix, which the packager gives the retail zip, a guess"
    );
    let era = github
        .resolve(&spec, &target(FlavorId::ClassicEra, None))
        .expect("era by name");
    assert_eq!(era.asset, "NoMetadata-2.0.0-classic.zip");
    assert!(era.guess);
    let forever = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .unwrap_err()
        .to_string();
    assert_eq!(
        forever,
        "no asset for flavor forever in github:Example/NoMetadata v2.0.0 (release.json names no flavor; assets: NoMetadata-2.0.0.zip, NoMetadata-2.0.0-classic.zip, release.json)"
    );
}

#[test]
fn no_release_json_at_all_is_a_pick_by_name() {
    let github = github();
    let spec = spec("github:Example/NoReleaseJson");
    let forever = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("forever by name");
    assert_eq!(forever.release, "3.0.0");
    assert_eq!(forever.version, "3.0.0");
    assert_eq!(forever.asset, "Thing-3.0.0-forever.zip");
    assert_eq!(forever.interface, None);
    assert!(forever.guess);
    assert_eq!(
        forever.reason,
        "no release.json names forever; the asset name ends in -forever, a guess"
    );
    let cata = github
        .resolve(&spec, &target(FlavorId::Classic, Some("4.4.2.1")))
        .expect("cata by name");
    assert_eq!(cata.asset, "Thing-3.0.0-cata.zip");
    let mists = github
        .resolve(&spec, &target(FlavorId::Classic, Some("5.5.4.1")))
        .unwrap_err()
        .to_string();
    assert_eq!(
        mists,
        "no asset for flavor classic in github:Example/NoReleaseJson 3.0.0 (no release.json; assets: Thing-3.0.0.zip, Thing-3.0.0-forever.zip, Thing-3.0.0-cata.zip)"
    );
}

#[test]
fn an_empty_release_list_and_an_unknown_repository_are_errors() {
    let github = github();
    let empty = github
        .resolve(
            &spec("github:Example/Empty"),
            &target(FlavorId::Forever, None),
        )
        .unwrap_err()
        .to_string();
    assert_eq!(empty, "no release in github:Example/Empty");
    let missing = github
        .resolve(
            &spec("github:Example/Missing"),
            &target(FlavorId::Forever, None),
        )
        .unwrap_err()
        .to_string();
    assert_eq!(
        missing,
        "GitHub answered 404 for https://api.github.com/repos/Example/Missing/releases?per_page=30: no such repository, or it is private"
    );
}

#[test]
fn the_stable_release_wins_over_a_draft_and_a_prerelease() {
    let github = github();
    let pick = github
        .resolve(
            &spec("github:Example/Prerelease"),
            &target(FlavorId::Forever, None),
        )
        .expect("the stable one");
    assert_eq!(pick.release, "v1.0.0");
    assert_eq!(pick.asset, "Pre-v1.0.0-forever.zip");
    assert!(!pick.prerelease);
    let beta = github
        .resolve(
            &spec("github:Example/BetaOnly"),
            &target(FlavorId::Forever, None),
        )
        .expect("a prerelease when there is nothing else");
    assert_eq!(beta.release, "v0.9.0-beta");
    assert_eq!(beta.version, "0.9.0-beta");
    assert!(beta.prerelease);
    assert!(
        beta.summary().contains(", a prerelease)"),
        "{}",
        beta.summary()
    );
}

#[test]
fn a_release_for_other_flavors_only_names_them() {
    let github = github();
    let spec = spec("github:Example/MainlineOnly");
    let error = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .unwrap_err()
        .to_string();
    assert_eq!(
        error,
        "no asset for flavor forever in github:Example/MainlineOnly v4.0.0 (release.json names mainline, standard; assets: Only-v4.0.0.zip, release.json)"
    );
    let retail = github
        .resolve(&spec, &target(FlavorId::Retail, None))
        .expect("retail");
    assert_eq!(retail.asset, "Only-v4.0.0.zip");
    assert!(!retail.guess);
}

#[test]
fn the_release_list_is_cached_with_its_etag() {
    let cache = temp_cache("etag");
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let spec = spec("github:Questie/Questie");
    let target = target(FlavorId::Forever, None);
    let first = github.resolve(&spec, &target).expect("first");
    let file = cache.join("questie").join("questie").join("releases.json");
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("the release list is cached"))
            .expect("the cache is JSON");
    let tag = cached["etag"]
        .as_str()
        .expect("the tag is cached")
        .to_string();
    assert!(tag.starts_with('"') && tag.ends_with('"'), "{tag}");
    assert_eq!(cached["releases"][1]["tag_name"], "v1.2.3");

    let second = github.resolve(&spec, &target).expect("second");
    assert_eq!(first, second);
    let requests = github.http().requests();
    let list_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.url.contains("/repos/Questie/Questie/releases"))
        .collect();
    assert_eq!(list_requests.len(), 2);
    assert_eq!(list_requests[0].header_value("If-None-Match"), None);
    assert_eq!(
        list_requests[1].header_value("If-None-Match"),
        Some(tag.as_str()),
        "the second run asks with the cached tag"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_cache_without_a_tag_or_in_another_shape_is_refreshed() {
    let cache = temp_cache("stale");
    let file = cache.join("questie").join("questie").join("releases.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    for stale in [r#"{"releases":[]}"#, "[]", "{ not json"] {
        std::fs::write(&file, stale).unwrap();
        let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
        let pick = github
            .resolve(
                &spec("github:Questie/Questie"),
                &target(FlavorId::Forever, None),
            )
            .expect("a fresh list: nothing to ask with");
        assert_eq!(pick.release, "v1.2.3", "{stale}");
        assert_eq!(
            github.http().requests()[0].header_value("If-None-Match"),
            None,
            "{stale}"
        );
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains(r#""etag""#), "{stale}: {text}");
    }
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn fetch_writes_the_picked_asset() {
    let stub = temp_cache("fetch-stub");
    let served = stub
        .join("github.com")
        .join("o")
        .join("r")
        .join("releases")
        .join("download")
        .join("v1")
        .join("A-forever.zip");
    std::fs::create_dir_all(served.parent().unwrap()).unwrap();
    std::fs::write(&served, b"not really a zip").unwrap();
    let releases = stub
        .join("api.github.com")
        .join("repos")
        .join("o")
        .join("r")
        .join("releases.json");
    std::fs::create_dir_all(releases.parent().unwrap()).unwrap();
    std::fs::write(
        &releases,
        r#"[{"tag_name":"v1","assets":[{"name":"A-forever.zip","browser_download_url":"https://github.com/o/r/releases/download/v1/A-forever.zip"}]}]"#,
    )
    .unwrap();
    let github = GitHub::new(FolderHttp::new(&stub), None, None);
    let pick = github
        .resolve(&spec("github:o/r"), &target(FlavorId::Forever, None))
        .expect("a pick");
    let to = stub.join("downloads").join(&pick.asset);
    github.fetch(&pick, &to).expect("fetched");
    assert_eq!(std::fs::read(&to).unwrap(), b"not really a zip");
    let _ = std::fs::remove_dir_all(&stub);
}
