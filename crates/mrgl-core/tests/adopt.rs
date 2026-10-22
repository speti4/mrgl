//! Adopt (tasks 097, 098) over the synthetic game folder `tests/fixtures/adopt/`: the report of every
//! unmanaged addon with its folder group and its sources, asked through the router against the folder stub of
//! the GitHub, WoWInterface and Wago answers (`tests/fixtures/releases/` and `tests/fixtures/sources/`,
//! copied into one temporary folder), with and without the made-up Wago key, and the entries `--apply` puts
//! into a state. No network, no real game folder; the fixture is read in place and never written.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mrgl_core::adopt::{self, Adoptable, Class, Hint, Report};
use mrgl_core::install;
use mrgl_core::scan;
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::wago::Wago;
use mrgl_core::sources::wowinterface::WowInterface;
use mrgl_core::sources::{FolderHttp, Http, Offered, SourceKind, Sources};
use mrgl_core::state::State;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder");
    for entry in std::fs::read_dir(from).expect("a fixture folder") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a copy");
        }
    }
}

/// A temporary folder with the answers of every source, and a home for their caches.
struct Stub {
    base: PathBuf,
}

impl Stub {
    fn new(name: &str) -> Stub {
        let base =
            std::env::temp_dir().join(format!("mrgl-adopt-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        copy_tree(&fixtures().join("releases"), &base.join("stub"));
        copy_tree(&fixtures().join("sources"), &base.join("stub"));
        Stub { base }
    }

    /// The router over the stub: GitHub without a token, WoWInterface, Wago with or without its key.
    fn sources(&self, wago_key: Option<&str>) -> Sources {
        let http: Arc<dyn Http> = Arc::new(FolderHttp::new(self.base.join("stub")));
        let cache = self.base.join("home/cache");
        Sources::none()
            .with(
                SourceKind::GitHub,
                GitHub::new(Arc::clone(&http), None, Some(cache.join("github"))),
            )
            .with(
                SourceKind::Wago,
                Wago::new(
                    Arc::clone(&http),
                    wago_key.map(str::to_string),
                    Some(cache.join("wago")),
                ),
            )
            .with(
                SourceKind::WowInterface,
                WowInterface::new(http, Some(cache.join("wowi"))),
            )
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn report(flavor: &str, sources: &Sources, wago_key: bool) -> Report {
    let scan = scan::scan(&fixtures().join("adopt").join(flavor)).expect("the fixture scans");
    let state = State::new(&scan.flavor.id);
    adopt::report(&scan, &state, sources, &|kind| {
        kind == SourceKind::Wago && wago_key
    })
}

fn row<'a>(report: &'a Report, name: &str) -> &'a Adoptable {
    report
        .addons
        .iter()
        .find(|addon| addon.name == name)
        .unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn every_header_shape_reports_its_sources_without_a_key() {
    let stub = Stub::new("keyless");
    let report = report("_forever_", &stub.sources(None), false);
    let lines: Vec<String> = report
        .addons
        .iter()
        .flat_map(|addon| {
            std::iter::once(addon.line("forever")).chain(addon.detail_lines("forever"))
        })
        .collect();
    assert_eq!(
        lines,
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
    let counts = report.counts;
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
    assert_eq!(
        report.summary().1,
        "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; adopt --apply adopts 5 of them"
    );
    assert!(!report.curse_client && report.marker_line().is_none());

    // The typed answers behind the lines.
    let curse_only = row(&report, "MrglCurseOnly");
    assert_eq!(curse_only.class, Class::OnlyCurseforge);
    assert_eq!(curse_only.needs, ["MRGL_CURSEFORGE_KEY"]);
    assert_eq!(curse_only.sources[0].build, Offered::Unknown);
    assert!(curse_only.adopt_as.is_none());
    let mixed = row(&report, "MrglNoList");
    assert_eq!(
        mixed
            .sources
            .iter()
            .map(|source| source.from)
            .collect::<Vec<_>>(),
        [Hint::WagoId, Hint::WowiId, Hint::CurseProjectId]
    );
    assert_eq!(row(&report, "MrglSite").version, None);
    assert_eq!(row(&report, "MrglNoHint").class, Class::NoSource);
    assert_eq!(row(&report, "MrglPage").class, Class::NoSource);
    assert_eq!(row(&report, "MrglNoForever").class, Class::NoBuild);
    assert_eq!(
        row(&report, "Questie").sources[0].from,
        Hint::Mapping,
        "a folder without a hint finds its source in the mapping"
    );
}

#[test]
fn the_wago_key_makes_the_wago_addon_adoptable() {
    let stub = Stub::new("keyed");
    let report = report("_forever_", &stub.sources(Some("made-up-test-key")), true);
    let wago = row(&report, "MrglWagoOne");
    assert_eq!(wago.adopt_as.as_deref(), Some("wago:MrglWg01"));
    assert!(wago.needs.is_empty());
    assert_eq!(wago.folders, ["MrglWagoOne", "MrglWagoOne_Options"]);
    assert_eq!(
        wago.line("forever"),
        "MrglWagoOne 1.0.0 (2 folders: MrglWagoOne, MrglWagoOne_Options): adopt as wago:MrglWg01"
    );
    assert_eq!(report.counts.adoptable_now, 6);
    // The Wago answer for the flavor comes from the keyless slug list, never from the keyed API.
    let serialized = serde_json::to_string(&report).expect("serializes");
    assert!(!serialized.contains("made-up-test-key"));
}

#[test]
fn a_folder_the_curseforge_app_manages_is_reported() {
    let stub = Stub::new("marked");
    let report = report("_retail_", &stub.sources(None), false);
    assert!(report.curse_client);
    assert_eq!(
        report.marker_line().as_deref(),
        Some(
            "the CurseForge app manages _retail_ (its .curseclient marker is there): a folder belongs to one manager, so adopt --apply writes nothing there without --alongside-curseforge"
        )
    );
    assert_eq!(
        row(&report, "MrglCompat").line("retail"),
        "MrglCompat 1.0.0: adopt as wowi:1001"
    );
}

#[test]
fn a_managed_folder_is_no_row_and_its_source_is_taken() {
    let stub = Stub::new("managed");
    let scan = scan::scan(&fixtures().join("adopt/_forever_")).expect("the fixture scans");
    let mut state = State::new(&scan.flavor.id);
    state.addons.insert(
        "MrglForever".to_string(),
        serde_json::from_value(serde_json::json!({
            "source": "wowi:1005",
            "version": "3.0.0",
            "folders": ["MrglForever"],
            "files": {},
            "installed_at": "2026-10-22T10:00:00Z",
        }))
        .expect("an entry"),
    );
    state.addons.insert(
        "Elsewhere".to_string(),
        serde_json::from_value(serde_json::json!({
            "source": "github:Example/Nightly",
            "version": "1.0.0",
            "folders": ["SomewhereElse"],
            "files": {},
            "installed_at": "2026-10-22T10:00:00Z",
        }))
        .expect("an entry"),
    );
    let report = adopt::report(&scan, &state, &stub.sources(None), &|_| false);
    assert!(
        report
            .addons
            .iter()
            .all(|addon| addon.name != "MrglForever")
    );
    let site = row(&report, "MrglSite");
    assert_eq!(site.sources[0].taken_by.as_deref(), Some("Elsewhere"));
    assert!(site.adopt_as.is_none());
    assert_eq!(
        site.detail_lines("forever")[0],
        "  github:Example/Nightly from X-Website: forever build yes (release.json names forever (interface 16001)); the source of the managed Elsewhere already"
    );
    assert_eq!(
        site.line("forever"),
        "MrglSite (no version): no source this run can write"
    );
}

#[test]
fn apply_writes_an_entry_per_adoptable_addon_and_a_second_run_nothing() {
    let stub = Stub::new("apply");
    let sources = stub.sources(Some("made-up-test-key"));
    let scan = scan::scan(&fixtures().join("adopt/_forever_")).expect("the fixture scans");
    let mut state = State::new(&scan.flavor.id);
    let has_key = |kind: SourceKind| kind == SourceKind::Wago;
    let report = adopt::report(&scan, &state, &sources, &has_key);
    let applied = adopt::apply(&report, &mut state);
    assert_eq!(
        applied
            .adopted
            .iter()
            .map(|adopted| adopted.line())
            .collect::<Vec<_>>(),
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
        applied
            .skipped
            .iter()
            .map(|skipped| skipped.line())
            .collect::<Vec<_>>(),
        ["MrglCurseOnly not adopted: only with CurseForge, needs MRGL_CURSEFORGE_KEY"]
    );
    assert_eq!(
        report.applied_summary(&applied).1,
        "10 unmanaged addons in _forever_: 6 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; adopted 6 addons into the sidecar, no download and no folder changed: the next update installs each from its source"
    );

    // The entry: the spec, the TOC's version, the group and the hash of every file as it is.
    let (_, entry) = state.entry("MrglWagoOne").expect("the Wago entry");
    assert_eq!(entry.source, "wago:MrglWg01");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.folders, ["MrglWagoOne", "MrglWagoOne_Options"]);
    assert_eq!(entry.previous_version, None);
    assert!(entry.pinned.is_none() && !entry.ignored && entry.nolib.is_none());
    let addons = fixtures().join("adopt/_forever_/Interface/AddOns");
    assert_eq!(
        entry.files.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "MrglWagoOne/MrglWagoOne.toc",
            "MrglWagoOne/core.lua",
            "MrglWagoOne_Options/MrglWagoOne_Options.toc",
            "MrglWagoOne_Options/options.lua",
        ]
    );
    assert_eq!(
        entry.files["MrglWagoOne/core.lua"],
        install::sha256_of(&addons.join("MrglWagoOne/core.lua")).expect("hashes")
    );
    // An unsubstituted packager token is no version: the entry says `unknown`.
    assert_eq!(
        state.entry("MrglSite").expect("the site entry").1.version,
        "unknown"
    );
    assert!(state.entry("MrglCurseOnly").is_none());
    // Every adopted folder is managed now.
    assert!(state.entry_for_folder("MrglElsewhere_Options").is_some());

    // A second run over the written state adopts nothing and changes nothing.
    let before = state.clone();
    let again = adopt::report(&scan, &state, &sources, &has_key);
    let applied = adopt::apply(&again, &mut state);
    assert!(applied.adopted.is_empty());
    assert_eq!(state, before);
    assert_eq!(
        again.applied_summary(&applied).1,
        "4 unmanaged addons in _forever_: 0 can be adopted without CurseForge, 1 only with CurseForge, 2 with no known source, 1 with no forever build at their sources; nothing adopted, the sidecar is unchanged"
    );
}

#[test]
fn apply_without_the_wago_key_skips_the_wago_addon_with_the_key_it_needs() {
    let stub = Stub::new("apply-keyless");
    let report = report("_forever_", &stub.sources(None), false);
    let scan = scan::scan(&fixtures().join("adopt/_forever_")).expect("the fixture scans");
    let mut state = State::new(&scan.flavor.id);
    let applied = adopt::apply(&report, &mut state);
    assert_eq!(applied.adopted.len(), 5);
    assert!(
        applied
            .skipped
            .iter()
            .any(|skipped| skipped.line() == "MrglWagoOne not adopted: needs MRGL_WAGO_KEY")
    );
    assert!(state.entry_for_folder("MrglWagoOne").is_none());
}

#[test]
fn the_curseforge_marker_has_its_refusal_and_its_warnings() {
    let scan = scan::scan(&fixtures().join("adopt/_retail_")).expect("the fixture scans");
    assert_eq!(
        adopt::marker_refusal(&scan.flavor),
        "the CurseForge app manages _retail_ (its .curseclient marker is there) and a folder belongs to one manager: adopt --apply wrote nothing; --alongside-curseforge adopts its addons anyway, and the CurseForge app may still update those folders"
    );
    assert_eq!(
        adopt::alongside_warning(&scan.flavor),
        "adopting alongside the CurseForge app in _retail_: it may still update or flag the adopted folders, and its .curseclient marker stays where it is"
    );
    assert_eq!(
        adopt::curse_client_warning(&scan.flavor).as_deref(),
        Some(
            "the CurseForge app manages _retail_ too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager"
        )
    );
    let forever = scan::scan(&fixtures().join("adopt/_forever_")).expect("the fixture scans");
    assert_eq!(adopt::curse_client_warning(&forever.flavor), None);
}
