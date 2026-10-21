//! The GitHub source over the fixture answers under `tests/fixtures/releases/` (task 003): one folder per
//! shape of a repository, served by the folder stub, so no test opens a socket. The cache tests use a
//! temporary folder of their own. Every fixture repository is classified for the Forever build (task 011).
//! The page walk, the rate-limit hint and the per-tag `release.json` cache (task 042) read the paged fixture
//! `Example/Nightly` or a stub folder a test writes for itself.

use std::path::{Path, PathBuf};

use mrgl_core::flavor::{BuildVersion, FlavorId};
use mrgl_core::forever::ForeverBuild;
use mrgl_core::sources::github::{GitHub, MAX_PAGES, current_release, forever_build};
use mrgl_core::sources::{
    Channel, FolderHttp, Pick, Source, SourceSpec, Target, local_time_text, unix_from_rfc3339,
};

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
        version: version.and_then(BuildVersion::parse),
        ..Target::new(flavor)
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
    // The Forever asset lists no size, so any zip a test puts behind it installs (task 046).
    assert_eq!(forever.size, None);
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
fn the_prerelease_channel_takes_the_newest_release_that_is_not_a_draft() {
    let github = github();
    let beta = github
        .resolve(
            &spec("github:Example/Prerelease"),
            &target(FlavorId::Forever, None).with_channel(Channel::Prerelease),
        )
        .expect("the prerelease");
    assert_eq!(
        beta.release, "v1.1.0-beta",
        "the draft above it does not count"
    );
    assert_eq!(beta.version, "1.1.0-beta");
    assert_eq!(beta.asset, "Pre-v1.1.0-beta-forever.zip");
    assert!(beta.prerelease);
    // A repository whose newest release is stable gives that one on either channel.
    let stable = github
        .resolve(
            &spec("github:Questie/Questie"),
            &target(FlavorId::Forever, Some("1.60.1.70245")).with_channel(Channel::Prerelease),
        )
        .expect("the stable one");
    assert_eq!(stable.release, "v1.2.3");
    assert!(!stable.prerelease);
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

#[test]
fn the_release_body_gives_the_notes_line_and_the_page() {
    let github = github();
    let notes = github
        .resolve(
            &spec("github:Example/Notes"),
            &target(FlavorId::Forever, None),
        )
        .expect("a pick");
    assert_eq!(notes.asset, "Notes-v2.1.0-forever.zip");
    assert_eq!(
        notes.notes.as_deref(),
        Some("Quest arrows point the right way again")
    );
    assert_eq!(
        notes.page.as_deref(),
        Some("https://github.com/Example/Notes/releases/tag/v2.1.0")
    );
    let json = serde_json::to_value(&notes).expect("JSON");
    assert_eq!(json["notes"], "Quest arrows point the right way again");

    // An empty body has no notes, and the JSON leaves the field out; the page stays.
    let silent = github
        .resolve(
            &spec("github:Example/NoNotes"),
            &target(FlavorId::Forever, None),
        )
        .expect("a pick");
    assert_eq!(silent.notes, None);
    assert_eq!(
        silent.page.as_deref(),
        Some("https://github.com/Example/NoNotes/releases/tag/v1.0.0")
    );
    let json = serde_json::to_value(&silent).expect("JSON");
    assert!(json.get("notes").is_none(), "{json}");
}

#[test]
fn the_cached_release_list_keeps_the_body() {
    let cache = temp_cache("body");
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let spec = spec("github:Example/Notes");
    let target = target(FlavorId::Forever, None);
    let first = github.resolve(&spec, &target).expect("first");
    let file = cache.join("example").join("notes").join("releases.json");
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("the release list is cached"))
            .expect("the cache is JSON");
    assert!(
        cached["releases"][0]["body"]
            .as_str()
            .is_some_and(|body| body.contains("Quest arrows")),
        "{cached}"
    );
    // The second run reads the list from the cache (a 304): the notes are the same, nothing is fetched twice.
    let second = github.resolve(&spec, &target).expect("second");
    assert_eq!(second.notes, first.notes);
    assert_eq!(second.page, first.page);
    let _ = std::fs::remove_dir_all(&cache);
}

// --- task 011: the derived Forever signal over every fixture repository ---

/// The fixture repositories, `<owner>/<repo>`, as the folder stub serves their release lists.
fn fixture_repositories() -> Vec<String> {
    let repos = fixtures().join("api.github.com").join("repos");
    let mut found = Vec::new();
    for owner in std::fs::read_dir(&repos).expect("the fixture repositories") {
        let owner = owner.unwrap().path();
        for repo in std::fs::read_dir(&owner).unwrap() {
            let repo = repo.unwrap().path();
            found.push(format!(
                "{}/{}",
                owner.file_name().unwrap().to_string_lossy(),
                repo.file_name().unwrap().to_string_lossy()
            ));
        }
    }
    found.sort();
    found
}

#[test]
fn every_fixture_release_is_classified_for_forever() {
    let expected = [
        ("Example/BetaOnly", ForeverBuild::Yes),
        ("Example/Empty", ForeverBuild::Unknown),
        ("Example/MainlineOnly", ForeverBuild::No),
        ("Example/Nightly", ForeverBuild::Yes),
        ("Example/NoMetadata", ForeverBuild::No),
        ("Example/NoNotes", ForeverBuild::Unknown),
        ("Example/NoReleaseJson", ForeverBuild::Unknown),
        ("Example/Notes", ForeverBuild::Yes),
        ("Example/Prerelease", ForeverBuild::Yes),
        ("Questie/Questie", ForeverBuild::Yes),
    ];
    let names: Vec<String> = expected.iter().map(|(name, _)| name.to_string()).collect();
    assert_eq!(
        fixture_repositories(),
        names,
        "a new fixture repository gets its expected Forever build here"
    );
    let github = github();
    for (name, build) in expected {
        let spec = spec(&format!("github:{name}"));
        let SourceSpec::GitHub { owner, repo } = &spec;
        let releases = github.releases(owner, repo).expect("the release list");
        let classified = match current_release(&releases) {
            None => ForeverBuild::Unknown,
            Some(release) => {
                let release_json = github
                    .release_json(&spec, release)
                    .expect("the release.json download")
                    .and_then(Result::ok);
                forever_build(release, release_json.as_ref())
            }
        };
        assert_eq!(classified, build, "{name}");
        // The pick carries the same signal, whatever flavor it is for.
        for flavor in [FlavorId::Forever, FlavorId::Retail] {
            if let Ok(pick) = github.resolve(&spec, &target(flavor.clone(), None)) {
                assert_eq!(pick.forever_build, build, "{name} for {flavor}");
            }
        }
    }
}

#[test]
fn the_forever_build_is_read_back_from_the_cache_alone() {
    let cache = temp_cache("forever");
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let questie = spec("github:Questie/Questie");
    let retail_only = spec("github:Example/MainlineOnly");
    let by_name = spec("github:Example/NoReleaseJson");
    // Nothing cached yet: unknown, and no request.
    assert_eq!(github.cached_forever_build(&questie), ForeverBuild::Unknown);
    assert!(github.http().requests().is_empty());

    github
        .resolve(&questie, &target(FlavorId::Forever, None))
        .expect("questie");
    github
        .resolve(&retail_only, &target(FlavorId::Retail, None))
        .expect("retail");
    github
        .resolve(&by_name, &target(FlavorId::Forever, None))
        .expect("by name");
    let requests = github.http().requests().len();
    assert_eq!(github.cached_forever_build(&questie), ForeverBuild::Yes);
    assert_eq!(github.cached_forever_build(&retail_only), ForeverBuild::No);
    assert_eq!(github.cached_forever_build(&by_name), ForeverBuild::Unknown);
    assert_eq!(
        github.http().requests().len(),
        requests,
        "the cache answers without a request"
    );

    // A cached release.json of another release does not count: the list moved on.
    let cached = cache
        .join("questie")
        .join("questie")
        .join("release-json")
        .join("tag-v1.2.3.json");
    let text = std::fs::read_to_string(&cached).expect("the release.json is cached");
    assert!(text.contains(r#""release":"v1.2.3""#), "{text}");
    std::fs::write(&cached, text.replace("v1.2.3", "v1.2.2")).unwrap();
    assert_eq!(github.cached_forever_build(&questie), ForeverBuild::Unknown);
    std::fs::write(&cached, "{ not json").unwrap();
    assert_eq!(github.cached_forever_build(&questie), ForeverBuild::Unknown);
    let _ = std::fs::remove_dir_all(&cache);
}

// --- task 042: the page walk, the rate-limit reset time, the per-tag release.json cache ---

/// A stub folder of a test's own: each `(path, content)` written under it, the path as the folder stub reads
/// a URL (`api.github.com/repos/o/r/releases.json`, a `.headers` file next to an answer).
fn stub_with(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let stub = temp_cache(name);
    for (path, content) in files {
        let path = stub.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
    }
    stub
}

/// The URLs a source asked for, in order.
fn asked(github: &GitHub<FolderHttp>) -> Vec<String> {
    github
        .http()
        .requests()
        .into_iter()
        .map(|request| request.url)
        .collect()
}

const NIGHTLY_LIST: &str = "https://api.github.com/repos/Example/Nightly/releases?per_page=30";

#[test]
fn a_stable_release_on_page_two_is_found_through_the_link_header() {
    let github = github();
    let spec = spec("github:Example/Nightly");
    let pick = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("the stable one on page two");
    assert_eq!(pick.release, "v1.0.0");
    assert!(!pick.prerelease);
    assert_eq!(pick.asset, "Nightly-v1.0.0-forever.zip");
    assert_eq!(pick.interface, Some(16001));
    assert!(!pick.guess);
    assert_eq!(
        asked(&github),
        [
            NIGHTLY_LIST.to_string(),
            format!("{NIGHTLY_LIST}&page=2"),
            "https://github.com/Example/Nightly/releases/download/v1.0.0/release.json".to_string(),
        ]
    );
    // The second page carries the token like the first: it is the API.
    let token = GitHub::new(FolderHttp::new(fixtures()), Some("secret".into()), None);
    token
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("a pick");
    assert_eq!(
        token.http().requests()[1].header_value("Authorization"),
        Some("Bearer secret")
    );
    let releases = github.releases("Example", "Nightly").expect("the list");
    assert_eq!(releases.len(), 35);

    // The prerelease channel has its pick on page one, but the walk is the list's, not the channel's.
    let beta = github
        .resolve(
            &spec,
            &target(FlavorId::Forever, None).with_channel(Channel::Prerelease),
        )
        .expect("the newest nightly");
    assert_eq!(beta.release, "v1.1.0-nightly.30");
    assert!(beta.guess, "no release.json: a pick by name");
}

#[test]
fn the_page_walk_is_cached_and_a_second_resolve_asks_once() {
    let cache = temp_cache("pages");
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let spec = spec("github:Example/Nightly");
    let first = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("first");
    let file = cache.join("example").join("nightly").join("releases.json");
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("cached")).expect("JSON");
    assert_eq!(
        cached["releases"].as_array().map(Vec::len),
        Some(35),
        "both pages"
    );
    let before = github.http().requests().len();
    let second = github
        .resolve(&spec, &target(FlavorId::Forever, None))
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(
        asked(&github)[before..],
        [NIGHTLY_LIST.to_string()],
        "one conditional request: the list is unchanged and release.json is cached"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn the_page_walk_stops_at_the_bound_and_off_the_api_host() {
    let page = |n: usize, next: &str| {
        let list = format!(
            r#"[{{"tag_name":"v0.0.{n}-nightly","prerelease":true,"assets":[{{"name":"A-forever.zip","browser_download_url":"https://github.test/A-{n}-forever.zip"}}]}}]"#
        );
        let headers = format!("Link: <{next}>; rel=\"next\"\n");
        (list, headers)
    };
    let list_url = "https://api.github.com/repos/o/r/releases?per_page=30";
    let mut files: Vec<(String, String)> = Vec::new();
    for n in 1..=MAX_PAGES + 2 {
        let (list, headers) = page(n, &format!("{list_url}&page={}", n + 1));
        let name = if n == 1 {
            "api.github.com/repos/o/r/releases.json".to_string()
        } else {
            format!("api.github.com/repos/o/r/releases.page-{n}.json")
        };
        files.push((format!("{name}.headers"), headers));
        files.push((name, list));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    let stub = stub_with("bound", &borrowed);
    let github = GitHub::new(FolderHttp::new(&stub), None, None);
    let pick = github
        .resolve(&spec("github:o/r"), &target(FlavorId::Forever, None))
        .expect("the newest prerelease, as without a stable one");
    assert_eq!(pick.release, "v0.0.1-nightly");
    assert_eq!(
        github.http().requests().len(),
        MAX_PAGES,
        "{:?}",
        asked(&github)
    );

    // A next link to another host is not followed: the token would go with it.
    let stub = stub_with(
        "offhost",
        &[
            (
                "api.github.com/repos/o/r/releases.json",
                r#"[{"tag_name":"v2-beta","prerelease":true,"assets":[{"name":"A-forever.zip","browser_download_url":"https://github.test/A-forever.zip"}]}]"#,
            ),
            (
                "api.github.com/repos/o/r/releases.json.headers",
                "Link: <https://elsewhere.test/repos/o/r/releases?page=2>; rel=\"next\"\n",
            ),
        ],
    );
    let github = GitHub::new(FolderHttp::new(&stub), Some("secret".into()), None);
    let pick = github
        .resolve(&spec("github:o/r"), &target(FlavorId::Forever, None))
        .expect("the prerelease");
    assert_eq!(pick.release, "v2-beta");
    assert_eq!(asked(&github), [list_url.to_string()]);
    let _ = std::fs::remove_dir_all(&stub);
}

#[test]
fn the_rate_limit_hint_names_the_reset_time_and_the_token() {
    let list_url = "https://api.github.com/repos/o/r/releases?per_page=30";
    let answer = |name: &str, headers: &str| {
        let stub = stub_with(
            name,
            &[
                (
                    "api.github.com/repos/o/r/releases.json",
                    r#"{"message":"API rate limit exceeded"}"#,
                ),
                ("api.github.com/repos/o/r/releases.json.headers", headers),
            ],
        );
        let error = GitHub::new(FolderHttp::new(&stub), None, None)
            .resolve(&spec("github:o/r"), &target(FlavorId::Forever, None))
            .unwrap_err()
            .to_string();
        let _ = std::fs::remove_dir_all(&stub);
        error
    };
    let reset = 1_792_598_400;
    assert_eq!(
        answer(
            "limit-403",
            "Status: 403\nX-RateLimit-Limit: 60\nX-RateLimit-Remaining: 0\nX-RateLimit-Reset: 1792598400\n"
        ),
        format!(
            "GitHub answered 403 for {list_url}: the rate limit is used up until {} (local time); set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour",
            local_time_text(reset)
        )
    );
    let secondary = answer("limit-429", "Status: 429\nRetry-After: 60\n");
    assert!(
        secondary.starts_with(&format!(
            "GitHub answered 429 for {list_url}: the rate limit is used up until "
        )) && secondary.contains("MRGL_GITHUB_TOKEN"),
        "{secondary}"
    );
    assert_eq!(
        answer("denied", "Status: 403\nX-RateLimit-Remaining: 12\n"),
        format!("GitHub answered 403 for {list_url}: access denied")
    );
}

#[test]
fn a_second_resolve_of_a_tag_reads_release_json_from_the_cache() {
    let cache = temp_cache("per-tag");
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let spec = spec("github:Questie/Questie");
    let target = target(FlavorId::Forever, None);
    let first = github.resolve(&spec, &target).expect("first");
    assert_eq!(
        github.http().requests().len(),
        2,
        "the list and release.json"
    );
    let per_tag = cache
        .join("questie")
        .join("questie")
        .join("release-json")
        .join("tag-v1.2.3.json");
    let text = std::fs::read_to_string(&per_tag).expect("release.json cached for its tag");
    assert!(text.contains(r#""release":"v1.2.3""#), "{text}");

    let second = github.resolve(&spec, &target).expect("second");
    assert_eq!(first, second);
    let requests = github.http().requests();
    assert_eq!(requests.len(), 3, "one request: {requests:?}");
    assert_eq!(
        requests[2].url,
        "https://api.github.com/repos/Questie/Questie/releases?per_page=30"
    );
    assert!(requests[2].header_value("If-None-Match").is_some());

    // A cached file of another asset (re-uploaded: another URL or size) is downloaded again.
    std::fs::write(&per_tag, text.replace("download/v1.2.3", "download/v0")).unwrap();
    let third = github.resolve(&spec, &target).expect("third");
    assert_eq!(first, third);
    let requests = github.http().requests();
    assert_eq!(requests.len(), 5);
    assert!(requests[4].url.ends_with("/v1.2.3/release.json"));
    let _ = std::fs::remove_dir_all(&cache);
}

// --- task 046: the asset download's 404 names the asset, the size is checked ---

#[test]
fn a_missing_asset_names_the_asset_and_the_release() {
    let github = github();
    let pick = github
        .resolve(
            &spec("github:Example/NoReleaseJson"),
            &target(FlavorId::Forever, None),
        )
        .expect("a pick by name");
    let to = temp_cache("missing-asset").join(&pick.asset);
    let error = github.fetch(&pick, &to).unwrap_err();
    assert!(
        matches!(error, mrgl_core::error::Error::AssetMissing { .. }),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "the asset Thing-3.0.0-forever.zip of github:Example/NoReleaseJson 3.0.0 is missing: GitHub answered 404 for https://github.com/Example/NoReleaseJson/releases/download/3.0.0/Thing-3.0.0-forever.zip (removed or renamed after the release was listed)"
    );
    assert!(!to.exists());
    // The release list's own 404 keeps its hint.
    let missing = github
        .resolve(
            &spec("github:Example/Missing"),
            &target(FlavorId::Forever, None),
        )
        .unwrap_err()
        .to_string();
    assert!(
        missing.ends_with(": no such repository, or it is private"),
        "{missing}"
    );
}

#[test]
fn the_downloaded_size_is_checked_against_the_listed_one() {
    let asset = |size: &str| {
        format!(
            r#"[{{"tag_name":"v1","assets":[{{"name":"A-forever.zip",{size}"browser_download_url":"https://github.com/o/r/releases/download/v1/A-forever.zip"}}]}}]"#
        )
    };
    let fetch = |name: &str, size: &str| {
        let list = asset(size);
        let stub = stub_with(
            name,
            &[
                ("api.github.com/repos/o/r/releases.json", list.as_str()),
                (
                    "github.com/o/r/releases/download/v1/A-forever.zip",
                    "nine byte",
                ),
            ],
        );
        let github = GitHub::new(FolderHttp::new(&stub), None, None);
        let pick = github
            .resolve(&spec("github:o/r"), &target(FlavorId::Forever, None))
            .expect("a pick");
        let to = stub.join("downloads").join(&pick.asset);
        let result = github
            .fetch(&pick, &to)
            .map(|()| std::fs::read(&to).expect("written"));
        let written = to.exists();
        let _ = std::fs::remove_dir_all(&stub);
        (result, written)
    };
    let (short, written) = fetch("short-asset", r#""size":2048,"#);
    let error = short.unwrap_err();
    assert!(
        matches!(
            error,
            mrgl_core::error::Error::AssetSizeMismatch {
                expected: 2048,
                got: 9,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "refused A-forever.zip of github:o/r v1: 9 bytes downloaded, the release list names 2048; the download was cut short or the asset changed, try again"
    );
    assert!(!written, "nothing is written for the engine to open");

    let (exact, _) = fetch("exact-asset", r#""size":9,"#);
    assert_eq!(exact.expect("the listed size"), b"nine byte");
    let (no_size, _) = fetch("unsized-asset", "");
    assert_eq!(no_size.expect("no size: as before"), b"nine byte");
    let (null, _) = fetch("null-size-asset", r#""size":null,"#);
    assert_eq!(null.expect("a null size: as before"), b"nine byte");
}

// --- task 041: the offline resolve from the cached release list ---

/// A source whose stub folder does not exist: every request is a network error.
fn offline(cache: &Path) -> GitHub<FolderHttp> {
    GitHub::new(
        FolderHttp::new(cache.join("no-such-stub")),
        None,
        Some(cache.to_path_buf()),
    )
}

#[test]
fn a_network_error_with_a_warm_cache_resolves_stale() {
    let cache = temp_cache("offline-warm");
    let spec = spec("github:Questie/Questie");
    let target = target(FlavorId::Forever, None);
    let online = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    let fresh = online.resolve(&spec, &target).expect("online");
    assert_eq!(fresh.stale, None, "a fresh pick is not stale");
    assert_eq!(fresh.stale_warning(), None);
    let file = cache.join("questie").join("questie").join("releases.json");
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("cached")).expect("JSON");
    let checked_at = cached["checked_at"]
        .as_str()
        .expect("the cache's date")
        .to_string();

    let github = offline(&cache);
    let pick = github.resolve(&spec, &target).expect("from the cache");
    assert_eq!(
        github.http().requests().len(),
        1,
        "the failed list request only: release.json comes from the per-tag cache"
    );
    let stale = pick.stale.clone().expect("a stale pick");
    assert_eq!(stale.checked_at.as_deref(), Some(checked_at.as_str()));
    let reason = "cannot reach https://api.github.com/repos/Questie/Questie/releases?per_page=30: offline, the stub folder is missing";
    assert_eq!(stale.reason, reason);
    assert_eq!(
        Pick {
            stale: None,
            ..pick.clone()
        },
        fresh,
        "the same pick"
    );
    let seconds = unix_from_rfc3339(&checked_at).expect("RFC 3339");
    assert_eq!(
        pick.stale_warning().expect("a warning"),
        format!(
            "offline: github:Questie/Questie v1.2.3 is from the release list cached {}, the pick may be stale ({reason})",
            local_time_text(seconds)
        )
    );
    let json = serde_json::to_value(&pick).expect("JSON");
    assert_eq!(json["stale"]["checked_at"], checked_at.as_str());
    assert_eq!(json["stale"]["reason"], reason);
    assert_eq!(
        serde_json::to_value(&fresh).expect("JSON")["stale"],
        serde_json::Value::Null
    );

    // A cache written before the date was kept says so.
    let mut undated = cached.clone();
    undated.as_object_mut().unwrap().remove("checked_at");
    std::fs::write(&file, undated.to_string()).unwrap();
    let pick = offline(&cache).resolve(&spec, &target).expect("undated");
    assert_eq!(
        pick.stale
            .as_ref()
            .and_then(|stale| stale.checked_at.clone()),
        None
    );
    assert!(
        pick.stale_warning()
            .expect("a warning")
            .contains("is from the release list cached at an unknown time, the pick may be stale"),
        "{:?}",
        pick.stale_warning()
    );
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_network_error_with_a_cold_cache_stays_the_error() {
    let cache = temp_cache("offline-cold");
    let error = offline(&cache)
        .resolve(
            &spec("github:Questie/Questie"),
            &target(FlavorId::Forever, None),
        )
        .unwrap_err();
    assert!(
        matches!(error, mrgl_core::error::Error::Network { .. }),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "cannot reach https://api.github.com/repos/Questie/Questie/releases?per_page=30: offline, the stub folder is missing"
    );
    // Without any cache folder at all, the same.
    let error = GitHub::new(FolderHttp::new(cache.join("no-such-stub")), None, None)
        .resolve(
            &spec("github:Questie/Questie"),
            &target(FlavorId::Forever, None),
        )
        .unwrap_err();
    assert!(
        matches!(error, mrgl_core::error::Error::Network { .. }),
        "{error:?}"
    );
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_conditional_answer_refreshes_the_cache_date() {
    let cache = temp_cache("offline-304");
    let spec = spec("github:Questie/Questie");
    let target = target(FlavorId::Forever, None);
    let github = GitHub::new(FolderHttp::new(fixtures()), None, Some(cache.clone()));
    github.resolve(&spec, &target).expect("first");
    let file = cache.join("questie").join("questie").join("releases.json");
    let mut cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("cached")).expect("JSON");
    cached["checked_at"] = "2000-01-01T00:00:00Z".into();
    std::fs::write(&file, cached.to_string()).unwrap();
    let second = github.resolve(&spec, &target).expect("second, a 304");
    assert_eq!(second.stale, None, "a working network: no warning");
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("cached")).expect("JSON");
    let checked_at = cached["checked_at"].as_str().expect("a date");
    assert_ne!(
        checked_at, "2000-01-01T00:00:00Z",
        "the 304 confirmed the list"
    );
    assert!(cached["etag"].is_string(), "{cached}");
    let _ = std::fs::remove_dir_all(&cache);
}
