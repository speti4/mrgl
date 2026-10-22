//! The CurseForge source over the fixture answers under `tests/fixtures/sources/api.curseforge.com/` (task
//! 096), served by the folder stub with its query mapping (`mods/<id>/files@gameVersionTypeId=<t>&pageSize=50.json`),
//! so no test opens a socket and no real key is used: the tests pass a made-up one. The projects: `4242` has a
//! Forever and a retail release and a newer Forever beta (its Forever file on `edge.forgecdn.net`, its retail
//! one behind a redirect of the CDN to `mediafilez.forgecdn.net`), `4243` has retail files only, `4244`'s
//! author allows no download outside CurseForge (its zip in hand is `zips/MrglNoDistribution-2.0.0.zip`), and
//! `4245` answers 401.

use std::path::{Path, PathBuf};

use mrgl_core::error::Error;
use mrgl_core::flavor::{BuildVersion, FlavorId, read_flavor};
use mrgl_core::forever::ForeverBuild;
use mrgl_core::install::{Engine, Metadata, SourceRequest};
use mrgl_core::keys;
use mrgl_core::sources::curseforge::{self, CurseForge};
use mrgl_core::sources::{Channel, FolderHttp, Offered, Source, SourceKind, SourceSpec, Target};
use mrgl_core::state::FlavorHome;
use mrgl_core::update::{self, UpdateState};

/// A made-up key: no test reads or uses a real one.
const KEY: &str = "made-up-curseforge-key-for-tests";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources")
}

fn curse() -> CurseForge<FolderHttp> {
    CurseForge::new(FolderHttp::new(fixtures()), Some(KEY.to_string()))
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
    let dir = std::env::temp_dir().join(format!("mrgl-curse-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_project_answers_forever_and_retail_from_its_files_per_game_version_type() {
    let curse = curse();
    let spec = spec("curse:4242");
    let forever = curse
        .resolve(&spec, &target(FlavorId::Forever, Some("1.60.1.70245")))
        .expect("forever");
    assert_eq!(forever.source, "curse:4242");
    assert_eq!(forever.release, "7001", "the release is the file id");
    assert_eq!(forever.version, "1.0.0", "the version is the displayName");
    assert_eq!(forever.asset, "MrglCurseOnly-1.0.0.zip");
    assert_eq!(
        forever.url,
        "https://edge.forgecdn.net/files/7001/1/MrglCurseOnly-1.0.0.zip"
    );
    assert_eq!(forever.size, Some(559));
    assert_eq!(
        forever.md5.as_deref(),
        Some("dd94b8c56a4dd517c5293ed6bf882263")
    );
    assert_eq!(forever.flavor, "forever");
    assert_eq!(forever.interface, Some(16001));
    assert!(!forever.guess && !forever.prerelease);
    assert_eq!(
        forever.reason,
        "CurseForge lists the release 1.0.0 for forever"
    );
    assert_eq!(
        forever.page.as_deref(),
        Some("https://www.curseforge.com/wow/addons/mrgl-curse-only")
    );
    assert_eq!(
        forever.published.as_deref(),
        Some("2026-10-01T10:00:00.000Z")
    );
    assert_eq!(forever.forever_build, ForeverBuild::Yes);
    let retail = curse
        .resolve(&spec, &target(FlavorId::Retail, Some("12.1.5.70448")))
        .expect("retail");
    assert_eq!(retail.release, "7002");
    assert_eq!(retail.flavor, "retail");
    assert_eq!(retail.interface, Some(120105));
    // The Forever build is classified from the project's latest files, type 88568, without another request.
    assert_eq!(retail.forever_build, ForeverBuild::Yes);
    let requests = curse.http().requests();
    let urls: Vec<&str> = requests
        .iter()
        .map(|request| request.url.as_str())
        .collect();
    assert_eq!(
        urls,
        [
            "https://api.curseforge.com/v1/mods/4242",
            "https://api.curseforge.com/v1/mods/4242/files?gameVersionTypeId=88568&pageSize=50",
            "https://api.curseforge.com/v1/mods/4242/files?gameVersionTypeId=517&pageSize=50",
        ],
        "the project is asked once a run, kept in memory"
    );
    for request in &requests {
        assert_eq!(request.header_value("x-api-key"), Some(KEY));
        assert!(!request.follow_redirects);
    }
    // The type ids as the constants name them, until a key re-reads them.
    assert_eq!(
        [
            curseforge::RETAIL,
            curseforge::CLASSIC_ERA,
            curseforge::FOREVER,
            curseforge::TBC,
            curseforge::WRATH,
            curseforge::TITAN,
            curseforge::CATA,
            curseforge::MISTS,
        ],
        [517, 67408, 88568, 73246, 73713, 81212, 77522, 79434]
    );
}

#[test]
fn the_release_types_map_onto_the_channels_and_a_version_is_found_by_name() {
    let curse = curse();
    let spec = spec("curse:4242");
    let forever = target(FlavorId::Forever, None);
    let stable = curse.resolve(&spec, &forever).expect("stable");
    assert_eq!(
        (stable.version.as_str(), stable.prerelease),
        ("1.0.0", false)
    );
    let beta = curse
        .resolve(&spec, &forever.clone().with_channel(Channel::Prerelease))
        .expect("prerelease");
    assert_eq!(
        (
            beta.version.as_str(),
            beta.release.as_str(),
            beta.prerelease
        ),
        ("1.1.0-beta", "7003", true)
    );
    assert_eq!(
        beta.reason,
        "CurseForge lists the beta 1.1.0-beta for forever"
    );
    let exact = curse
        .resolve_version(&spec, &forever, "1.1.0-beta")
        .expect("listed");
    assert_eq!(exact.release, "7003");
    assert!(matches!(
        curse.resolve_version(&spec, &forever, "0.9.0"),
        Err(Error::VersionNotOffered { .. })
    ));
    // No source keeps a list of versions: nothing of an answer is kept past the run.
    assert_eq!(curse.listed_versions(&spec), None);
    assert_eq!(
        curse.cached_forever_build(&spec, Channel::Stable),
        ForeverBuild::Unknown
    );
    assert_eq!(curse.request_budget(), None);
}

#[test]
fn the_forever_signal_and_the_offer_come_from_the_game_version_types() {
    let curse = curse();
    let forever = target(FlavorId::Forever, None);
    assert_eq!(
        curse
            .project(&spec("curse:4242"))
            .expect("a project")
            .forever_build(),
        ForeverBuild::Yes
    );
    assert_eq!(
        curse
            .project(&spec("curse:4243"))
            .expect("a project")
            .forever_build(),
        ForeverBuild::No
    );
    let offer = curse.offers(&spec("curse:4242"), &forever);
    assert_eq!(offer.build, Offered::Yes);
    assert_eq!(offer.why, "CurseForge lists the release 1.0.0 for forever");
    assert_eq!(
        curse.folders(&spec("curse:4242")),
        Some(vec!["MrglCurseOnly".to_string()]),
        "the file's modules"
    );
    let none = curse.offers(&spec("curse:4243"), &forever);
    assert_eq!(none.build, Offered::No);
    assert_eq!(none.why, "CurseForge lists no file for forever");
    assert_eq!(curse.folders(&spec("curse:4243")), None);
    let off = curse.offers(&spec("curse:4244"), &forever);
    assert_eq!(off.build, Offered::Yes);
    assert_eq!(
        off.why,
        "CurseForge lists the release 2.0.0 for forever; its author allows no download outside CurseForge, so it installs from a zip in hand"
    );
}

#[test]
fn a_401_names_the_key_and_no_key_is_not_configured() {
    let error = curse()
        .resolve(&spec("curse:4245"), &target(FlavorId::Forever, None))
        .expect_err("401");
    assert_eq!(
        error.to_string(),
        "CurseForge answered 401 for https://api.curseforge.com/v1/mods/4245: MRGL_CURSEFORGE_KEY is missing or not accepted: the CurseForge API needs your own key, `mrgl key set curse` stores it"
    );
    assert!(!error.to_string().contains(KEY));
    let without = CurseForge::new(FolderHttp::new(fixtures()), None);
    let missing = "MRGL_CURSEFORGE_KEY is not set: the CurseForge API needs your own key, `mrgl key set curse` stores it";
    assert_eq!(
        without.missing(&spec("curse:4242")).as_deref(),
        Some(missing)
    );
    assert_eq!(
        without
            .resolve(&spec("curse:4242"), &target(FlavorId::Forever, None))
            .expect_err("no key")
            .to_string(),
        format!("source not configured: curse:4242 ({missing})")
    );
    let offer = without.offers(&spec("curse:4242"), &target(FlavorId::Forever, None));
    assert_eq!(
        (offer.build, offer.why.as_str()),
        (Offered::Unknown, missing)
    );
    assert!(
        without.http().requests().is_empty(),
        "nothing asked without a key"
    );
    // The window's offline view has no key and asks nothing either.
    let view = CurseForge::cached(FolderHttp::new(fixtures()));
    assert_eq!(view.missing(&spec("curse:4242")), None);
    assert!(
        view.resolve(&spec("curse:4242"), &target(FlavorId::Forever, None))
            .is_err()
    );
    assert!(view.http().requests().is_empty());
    assert_eq!(
        curse().missing(&spec("wowi:1")).as_deref(),
        Some("the CurseForge source answers curse: specs only")
    );
}

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

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn engine(&self, folder: &str) -> Engine {
        let flavor =
            read_flavor(&self.dir.join("World of Warcraft").join(folder)).expect("the flavor");
        Engine::with_home(flavor, &self.home())
    }

    /// Every file under the folder, for the check that the key is in none.
    fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    files.push((path, bytes));
                }
            }
        }
        let mut files = Vec::new();
        walk(&self.dir, &mut files);
        files
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn the_key_goes_to_the_two_curseforge_hosts_only_and_never_into_a_file() {
    let sandbox = Sandbox::new("key");
    let http = FolderHttp::new(fixtures());
    let curse = CurseForge::new(&http, Some(KEY.to_string()));
    for folder in ["_forever_", "_retail_"] {
        let engine = sandbox.engine(folder);
        let installed = engine
            .install_from_source(
                &curse,
                &SourceRequest::current(spec("curse:4242")),
                &mut |_| {},
            )
            .unwrap_or_else(|error| panic!("{folder}: {error}"));
        assert_eq!(installed.pick.version, "1.0.0");
        let state = engine.load_state().expect("the sidecar").state;
        let (_, entry) = state.entry("MrglCurseOnly").expect("the entry");
        assert_eq!(entry.source, "curse:4242");
        assert_eq!(entry.version, "1.0.0");
        // The Forever signal is recorded at install, since no answer is cached.
        assert_eq!(entry.forever_build, Some(ForeverBuild::Yes), "{folder}");
        let toc = std::fs::read_to_string(
            sandbox
                .dir
                .join("World of Warcraft")
                .join(folder)
                .join("Interface/AddOns/MrglCurseOnly/MrglCurseOnly.toc"),
        )
        .expect("the TOC");
        let interface = if folder == "_forever_" {
            "16001"
        } else {
            "120105"
        };
        assert!(toc.contains(&format!("## Interface: {interface}")), "{toc}");
    }
    let requests = http.requests();
    let hosts: Vec<&str> = requests
        .iter()
        .map(|request| {
            request.url["https://".len()..]
                .split('/')
                .next()
                .unwrap_or_default()
        })
        .collect();
    assert!(hosts.contains(&"api.curseforge.com"));
    assert!(hosts.contains(&"edge.forgecdn.net"), "the downloads");
    assert!(
        hosts.contains(&"mediafilez.forgecdn.net"),
        "the CDN's redirect is followed"
    );
    for request in &requests {
        let sent = request.header_value("x-api-key");
        let ours = keys::may_send(SourceKind::CurseForge, &request.url);
        assert_eq!(sent.is_some(), ours, "{}", request.url);
        assert!(
            sent.is_none()
                || request.url.starts_with("https://api.curseforge.com/")
                || request.url.starts_with("https://edge.forgecdn.net/"),
            "{}",
            request.url
        );
    }
    // Nothing of the key, and no answer of the API, is written under the home folder.
    for (path, bytes) in sandbox.files() {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(KEY), "{}", path.display());
        assert!(!text.contains("latestFilesIndexes"), "{}", path.display());
    }
    assert!(
        !sandbox.home().join("cache").exists(),
        "no cache folder of the source"
    );
    // The sidecar file itself, as JSON: the recorded signal and the schema unchanged.
    let raw: serde_json::Value = serde_json::from_slice(
        &std::fs::read(FlavorHome::at(&sandbox.home(), &FlavorId::Forever).state_path())
            .expect("the sidecar file"),
    )
    .expect("JSON");
    assert_eq!(raw["schema"], 1);
    assert_eq!(raw["addons"]["MrglCurseOnly"]["forever_build"], "yes");
}

#[test]
fn a_project_without_distribution_names_its_page_and_installs_from_a_zip_in_hand() {
    let curse = curse();
    let error = curse
        .resolve(&spec("curse:4244"), &target(FlavorId::Forever, None))
        .expect_err("distribution off");
    assert!(matches!(error, Error::DistributionOff { .. }));
    assert_eq!(
        error.to_string(),
        "the author of curse:4244 (Mrgl No Distribution) allows no download outside CurseForge: download the zip from https://www.curseforge.com/wow/addons/mrgl-no-distribution and install it with `mrgl add <file.zip> --as curse:4244`"
    );
    // The zip in hand installs with the source recorded, and the update check asks that source.
    let sandbox = Sandbox::new("no-distribution");
    let engine = sandbox.engine("_forever_");
    let zip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/zips/MrglNoDistribution-2.0.0.zip");
    let installed = engine
        .install(
            &zip,
            &Metadata {
                source: "curse:4244".to_string(),
                ..Metadata::default()
            },
        )
        .expect("installed");
    assert_eq!(
        (installed.name.as_str(), installed.version.as_str()),
        ("MrglNoDistribution", "2.0.0")
    );
    let state = engine.load_state().expect("the sidecar").state;
    let (_, entry) = state.entry("MrglNoDistribution").expect("the entry");
    assert_eq!(entry.source, "curse:4244");
    assert_eq!(entry.forever_build, None, "a zip in hand says nothing");
    let check = update::check(&engine, &curse, None, &mut |_| {}).expect("checks");
    assert_eq!(check.addons.len(), 1);
    assert_eq!(check.addons[0].state, UpdateState::Failed);
    assert_eq!(
        check.addons[0].detail.as_deref(),
        Some(error.to_string().as_str())
    );
}

#[test]
fn the_update_check_takes_a_curse_entry() {
    let sandbox = Sandbox::new("update");
    let engine = sandbox.engine("_forever_");
    let zips = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zips");
    engine
        .install(
            &zips.join("one-folder-1.0.0.zip"),
            &Metadata {
                source: "curse:4242".to_string(),
                version: Some("0.9.0".to_string()),
                ..Metadata::default()
            },
        )
        .expect("installed");
    let http = FolderHttp::new(fixtures());
    let curse = CurseForge::new(&http, Some(KEY.to_string()));
    let check = update::check(&engine, &curse, None, &mut |_| {}).expect("checks");
    assert_eq!(check.addons.len(), 1);
    let addon = &check.addons[0];
    assert_eq!(addon.state, UpdateState::UpdateAvailable);
    assert_eq!(addon.latest.as_deref(), Some("1.0.0"));
    // Without the key the entry is not checked and says what is missing.
    let without = CurseForge::new(FolderHttp::new(fixtures()), None);
    let check = update::check(&engine, &without, None, &mut |_| {}).expect("checks");
    assert_eq!(check.addons[0].state, UpdateState::NotChecked);
    assert_eq!(
        check.addons[0].detail.as_deref(),
        Some(
            "source not configured: curse:4242 (MRGL_CURSEFORGE_KEY is not set: the CurseForge API needs your own key, `mrgl key set curse` stores it)"
        )
    );
}
