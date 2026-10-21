//! The scan over the synthetic flavors tree under `tests/fixtures/flavors/`: every TOC shape task 002 names,
//! read offline, with no real game folder anywhere.

use std::path::{Path, PathBuf};

use mrgl_core::Error;
use mrgl_core::flavor::{self, FlavorId};
use mrgl_core::scan::{self, Addon, Scan};
use mrgl_core::toc::TocSuffix;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flavors")
}

fn scan_fixture(flavor_folder: &str) -> Scan {
    scan::scan(&fixtures().join(flavor_folder)).expect("the fixture scans")
}

fn addon<'a>(scan: &'a Scan, folder: &str) -> &'a Addon {
    scan.addons
        .iter()
        .find(|addon| addon.folder == folder)
        .unwrap_or_else(|| panic!("{folder} in the scan"))
}

#[test]
fn the_retail_fixture_lists_every_addon_sorted_and_skips_the_bare_folder() {
    let scan = scan_fixture("_retail_");
    let folders: Vec<&str> = scan
        .addons
        .iter()
        .map(|addon| addon.folder.as_str())
        .collect();
    assert_eq!(
        folders,
        [
            "BomCrlf",
            "Camelot",
            "ClassicOnly",
            "Colored",
            "Deps",
            "ForeignStem",
            "Hinted",
            "LoudCase",
            "Midnight",
            "OldSuffixes",
            "PlainToc",
            "Repeated",
            "Spaced",
            "ThreeWays",
        ]
    );
    assert_eq!(scan.skipped, ["NoToc"]);
    assert!(scan.warnings.is_empty(), "{:?}", scan.warnings);
    assert_eq!(scan.flavor.id, FlavorId::Retail);
    assert!(
        scan.flavor.warnings.is_empty(),
        "{:?}",
        scan.flavor.warnings
    );
}

#[test]
fn the_chosen_toc_follows_the_chain() {
    let scan = scan_fixture("_retail_");
    assert_eq!(
        addon(&scan, "PlainToc").chosen.as_deref(),
        Some("PlainToc.toc")
    );
    assert_eq!(
        addon(&scan, "ThreeWays").chosen.as_deref(),
        Some("ThreeWays_Mainline.toc")
    );
    assert_eq!(
        addon(&scan, "Midnight").chosen.as_deref(),
        Some("Midnight_Standard.toc")
    );
    assert_eq!(addon(&scan, "Camelot").chosen, None);
    assert_eq!(addon(&scan, "ClassicOnly").chosen, None);
    assert_eq!(
        addon(&scan, "ThreeWays").version(),
        Some("3.0-mainline"),
        "the version comes from the chosen TOC"
    );
    assert_eq!(
        addon(&scan, "Camelot").version(),
        Some("1.0-camelot"),
        "without a chosen TOC the first non-legacy file (sorted by name) gives the version"
    );
    assert_eq!(
        addon(&scan, "ThreeWays")
            .chosen_toc()
            .map(|file| &file.toc.interface),
        Some(&vec![120105, 50504])
    );
}

#[test]
fn legacy_and_foreign_tocs_are_listed_not_chosen() {
    let scan = scan_fixture("_retail_");
    let old = addon(&scan, "OldSuffixes");
    assert_eq!(old.chosen.as_deref(), Some("OldSuffixes.toc"));
    assert!(old.has_legacy());
    let legacy: Vec<(&str, Option<TocSuffix>)> = old
        .tocs
        .iter()
        .filter(|file| file.legacy)
        .map(|file| (file.file.as_str(), file.suffix))
        .collect();
    assert_eq!(
        legacy,
        [
            ("OldSuffixes-Wrath.toc", Some(TocSuffix::Wrath)),
            ("OldSuffixes_BCC.toc", Some(TocSuffix::Bcc)),
        ]
    );

    let loud = addon(&scan, "LoudCase");
    let cata = loud
        .tocs
        .iter()
        .find(|file| file.file == "LoudCase_CATA.toc")
        .expect("the uppercase suffix file");
    assert_eq!(cata.suffix, Some(TocSuffix::Cata));
    assert!(!cata.legacy);
    assert_eq!(loud.chosen.as_deref(), Some("LoudCase.toc"));

    let foreign = addon(&scan, "ForeignStem");
    assert_eq!(foreign.other_tocs, ["SomethingElse.toc"]);
    assert_eq!(foreign.tocs.len(), 1);
}

#[test]
fn hints_and_the_curse_marker_are_exposed() {
    let scan = scan_fixture("_retail_");
    let hinted = addon(&scan, "Hinted");
    assert!(hinted.curse_client);
    assert!(!hinted.linked);
    let toc = &hinted.primary_toc().expect("a toc").toc;
    assert_eq!(toc.curse_project_id.as_deref(), Some("123456"));
    assert_eq!(toc.wago_id.as_deref(), Some("aB1cD2eF"));
    assert_eq!(toc.wowi_id.as_deref(), Some("7890"));
    assert!(toc.load_on_demand);
    assert!(!addon(&scan, "PlainToc").curse_client);

    let deps = &addon(&scan, "Deps").primary_toc().expect("a toc").toc;
    assert_eq!(
        deps.dependencies,
        ["LibAlpha", "LibBeta", "LibGamma", "LibDelta", "LibEpsilon"]
    );
    assert_eq!(deps.optional_dependencies, ["LibZeta", "LibEta"]);

    let colored = &addon(&scan, "Colored").primary_toc().expect("a toc").toc;
    assert_eq!(colored.title_plain.as_deref(), Some("Colored Addon"));
    assert_eq!(colored.notes_plain.as_deref(), Some("Red notes"));

    let bom = &addon(&scan, "BomCrlf").primary_toc().expect("a toc").toc;
    assert_eq!(bom.title.as_deref(), Some("Bom Crlf"));
    assert_eq!(bom.interface, [120105]);

    let repeated = &addon(&scan, "Repeated").primary_toc().expect("a toc").toc;
    assert_eq!(repeated.version.as_deref(), Some("2"));
    assert_eq!(repeated.title.as_deref(), Some("Repeated"));
    assert_eq!(
        repeated.headers.get("Title-deDE").map(String::as_str),
        Some("Wiederholt")
    );

    let spaced = &addon(&scan, "Spaced").primary_toc().expect("a toc").toc;
    assert_eq!(spaced.interface, [120105, 50504, 11509]);
}

#[test]
fn classic_and_era_pick_their_game_type() {
    let classic = scan_fixture("_classic_");
    assert_eq!(classic.flavor.id, FlavorId::Classic);
    assert_eq!(classic.flavor.interface(), Some(50504));
    assert_eq!(
        addon(&classic, "ClassicPick").chosen.as_deref(),
        Some("ClassicPick_Mists.toc")
    );

    let era = scan_fixture("_classic_era_");
    assert_eq!(era.flavor.id, FlavorId::ClassicEra);
    assert_eq!(era.flavor.interface(), Some(11509));
    assert_eq!(
        addon(&era, "EraPick").chosen.as_deref(),
        Some("EraPick_Vanilla.toc")
    );
}

#[test]
fn a_flavor_without_an_addons_folder_scans_to_nothing() {
    let forever = scan_fixture("_forever_");
    assert_eq!(forever.flavor.id, FlavorId::Forever);
    assert_eq!(forever.flavor.interface(), Some(16001));
    assert!(forever.addons.is_empty());
    assert!(forever.skipped.is_empty());
    assert!(
        forever.flavor.warnings.is_empty(),
        "{:?}",
        forever.flavor.warnings
    );
    assert_eq!(
        forever.flavor.toc_chain(),
        [
            Some(TocSuffix::Forever),
            Some(TocSuffix::Camelot),
            Some(TocSuffix::Mainline),
            None
        ]
    );
}

#[test]
fn an_unknown_id_is_a_warning_not_an_error() {
    let unknown = scan_fixture("_unknown_");
    assert_eq!(unknown.flavor.id, FlavorId::Other("wow_mrgl".to_string()));
    assert_eq!(unknown.flavor.version, None);
    assert_eq!(
        unknown.flavor.warnings.len(),
        2,
        "{:?}",
        unknown.flavor.warnings
    );
    assert!(unknown.flavor.warnings[0].starts_with("unknown flavor id wow_mrgl in "));
    assert_eq!(
        unknown.flavor.warnings[1],
        "build unknown: no row for wow_mrgl in .build.info"
    );
    assert_eq!(addon(&unknown, "One").chosen.as_deref(), Some("One.toc"));
}

#[test]
fn a_folder_without_flavor_info_is_an_error() {
    let error = scan::scan(&fixtures().join("_no_info_")).expect_err("not a flavor folder");
    assert!(
        matches!(&error, Error::NotAFlavorFolder { reason, .. } if reason == "no .flavor.info"),
        "{error}"
    );
    assert!(error.to_string().starts_with("not a game flavor folder: "));
    assert!(error.to_string().ends_with("_no_info_ (no .flavor.info)"));
}

#[test]
fn the_build_version_comes_from_the_root() {
    let retail = flavor::read_flavor(&fixtures().join("_retail_")).expect("a flavor");
    assert_eq!(
        retail.version.map(|v| v.to_string()),
        Some("12.1.5.70448".to_string())
    );
    assert_eq!(retail.interface(), Some(120105));
    assert_eq!(retail.folder_name(), "_retail_");
    assert!(retail.folder.is_absolute());
    let json = serde_json::to_value(&retail).expect("serializes");
    assert_eq!(json["id"], "wow");
    assert_eq!(json["version"], "12.1.5.70448");
    assert_eq!(json["interface"], 120105);
    assert_eq!(json["build"], 70448);
}

#[test]
fn list_flavors_finds_the_fixture_flavors() {
    let flavors = flavor::list_flavors(&fixtures()).expect("the root lists");
    let ids: Vec<String> = flavors.iter().map(|f| f.id.to_string()).collect();
    assert_eq!(
        ids,
        ["wow_classic", "wow_classic_era", "wowf", "wow", "wow_mrgl"]
    );
    let names: Vec<String> = flavors.iter().map(|f| f.folder_name()).collect();
    assert_eq!(
        names,
        [
            "_classic_",
            "_classic_era_",
            "_forever_",
            "_retail_",
            "_unknown_"
        ]
    );
    let empty = flavor::list_flavors(&fixtures().join("_retail_")).expect("lists");
    assert!(empty.is_empty());
}
