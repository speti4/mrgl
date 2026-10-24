//! The scan over the synthetic flavors tree under `tests/fixtures/flavors/`: every TOC shape task 002 names,
//! read offline, with no real game folder anywhere.

use std::path::{Path, PathBuf};

use mrgl_core::Error;
use mrgl_core::flavor::{self, FlavorId, VersionSource};
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
    assert!(
        scan.flavor.curse_client,
        "the retail fixture carries the CurseForge app's marker"
    );
    assert!(!scan_fixture("_classic_").flavor.curse_client);
    let hinted = addon(&scan, "Hinted");
    assert!(!hinted.linked);
    let toc = &hinted.primary_toc().expect("a toc").toc;
    assert_eq!(toc.curse_project_id.as_deref(), Some("123456"));
    assert_eq!(toc.wago_id.as_deref(), Some("aB1cD2eF"));
    assert_eq!(toc.wowi_id.as_deref(), Some("7890"));
    assert!(toc.load_on_demand);

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

/// The synthetic game folder whose `.build.info` has a row for retail only, with client exe stubs
/// (`client-exe/`, task 019).
fn client_exe_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/client-exe")
}

#[test]
fn a_missing_build_info_row_falls_back_to_the_client_exe() {
    let era = flavor::read_flavor(&client_exe_fixtures().join("_classic_era_")).expect("a flavor");
    assert_eq!(era.id, FlavorId::ClassicEra);
    assert_eq!(
        era.version.map(|v| v.to_string()),
        Some("1.15.7.70100".to_string())
    );
    assert_eq!(era.interface(), Some(11507));
    assert_eq!(
        era.version_source,
        Some(VersionSource::ClientExe("WowClassic.exe".to_string()))
    );
    assert!(era.warnings.is_empty(), "{:?}", era.warnings);
    let json = serde_json::to_value(&era).expect("serializes");
    assert_eq!(json["version"], "1.15.7.70100");
    assert_eq!(json["version_source"], "client-exe");
    assert_eq!(json["version_file"], "WowClassic.exe");
    let scan = scan::scan(&client_exe_fixtures().join("_classic_era_")).expect("scans");
    assert!(scan.addons.is_empty());
    assert_eq!(
        scan.flavor.toc_chain(),
        [Some(TocSuffix::Vanilla), Some(TocSuffix::Classic), None]
    );
}

#[test]
fn build_info_stays_the_first_source() {
    let retail = flavor::read_flavor(&client_exe_fixtures().join("_retail_")).expect("a flavor");
    assert_eq!(
        retail.version.map(|v| v.to_string()),
        Some("12.1.5.70448".to_string()),
        "the row, not the exe's 12.1.0.70000"
    );
    assert_eq!(retail.version_source, Some(VersionSource::BuildInfo));
    assert!(retail.warnings.is_empty(), "{:?}", retail.warnings);
    let json = serde_json::to_value(&retail).expect("serializes");
    assert_eq!(json["version_source"], "build-info");
    assert_eq!(json["version_file"], ".build.info");
}

#[test]
fn neither_source_keeps_the_warning() {
    let forever = flavor::read_flavor(&client_exe_fixtures().join("_forever_")).expect("a flavor");
    assert_eq!(forever.version, None);
    assert_eq!(forever.version_source, None);
    assert_eq!(
        forever.warnings,
        ["build unknown: no row for wowf in .build.info"],
        "no exe in the folder: the warning as before"
    );
    let json = serde_json::to_value(&forever).expect("serializes");
    assert!(json["version_source"].is_null());
    assert!(json["version_file"].is_null());

    let classic = flavor::read_flavor(&client_exe_fixtures().join("_classic_")).expect("a flavor");
    assert_eq!(classic.version, None);
    assert_eq!(
        classic.warnings,
        [
            "build unknown: no row for wow_classic in .build.info; WowClassic.exe has no readable version (no version resource (RT_VERSION))"
        ],
        "an exe without a version resource is named in the warning"
    );
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

/// A temporary flavor folder with its `.flavor.info`, removed when the guard goes.
struct TempFlavor {
    root: PathBuf,
}

impl TempFlavor {
    fn new(name: &str) -> TempFlavor {
        let root =
            std::env::temp_dir().join(format!("mrgl-scan-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the flavor folder");
        std::fs::write(root.join(".flavor.info"), "Product Flavor!STRING:0\nwowf\n")
            .expect(".flavor.info");
        TempFlavor { root }
    }

    fn addons(&self) -> PathBuf {
        scan::addons_dir(&self.root)
    }
}

impl Drop for TempFlavor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn an_addons_folder_that_is_a_file_is_an_error() {
    let flavor = TempFlavor::new("addons-is-a-file");
    std::fs::create_dir_all(flavor.root.join("Interface")).expect("Interface");
    std::fs::write(flavor.addons(), "not a folder").expect("a file where AddOns goes");
    let error = scan::scan(&flavor.root).expect_err("the AddOns folder cannot be read");
    assert!(
        matches!(&error, Error::Io { context, .. } if *context == "reading the AddOns folder"),
        "{error}"
    );
}

#[test]
fn the_primary_toc_falls_back_to_a_legacy_one() {
    let legacy = |file: &str, version: &str| mrgl_core::scan::TocFile {
        file: file.to_string(),
        suffix: Some(TocSuffix::Bcc),
        legacy: true,
        toc: mrgl_core::toc::parse_toc(&format!("## Version: {version}\n")),
    };
    let addon = Addon {
        folder: "Old".to_string(),
        path: PathBuf::from("Old"),
        tocs: vec![
            legacy("Old_BCC.toc", "2.4"),
            legacy("Old_WOTLKC.toc", "3.3"),
        ],
        other_tocs: Vec::new(),
        chosen: None,
        linked: false,
    };
    assert_eq!(addon.chosen_toc(), None);
    assert_eq!(
        addon.primary_toc().map(|file| file.file.as_str()),
        Some("Old_BCC.toc")
    );
    assert_eq!(addon.version(), Some("2.4"));
    assert!(addon.has_legacy());
}

#[cfg(windows)]
#[test]
fn an_addon_whose_toc_cannot_be_read_is_skipped_with_a_warning() {
    let flavor = TempFlavor::new("locked-toc");
    let locked = flavor.addons().join("Locked");
    std::fs::create_dir_all(&locked).expect("the addon folder");
    std::fs::write(locked.join("Locked.toc"), "## Interface: 16001\n").expect("its TOC");
    let open = mrgl_test_support::locked::file(&locked.join("Locked.toc"));
    let scan = scan::scan(&flavor.root).expect("scans");
    assert!(scan.addons.is_empty());
    assert_eq!(scan.skipped, ["Locked"]);
    assert_eq!(scan.warnings.len(), 1, "{:?}", scan.warnings);
    assert!(
        scan.warnings[0].starts_with(&format!(
            "skipped {}: {}: ",
            locked.display(),
            locked.join("Locked.toc").display()
        )),
        "{}",
        scan.warnings[0]
    );
    drop(open);
    assert_eq!(scan::scan(&flavor.root).expect("scans").addons.len(), 1);
}

#[cfg(windows)]
#[test]
fn a_folder_only_a_verbatim_path_reaches_is_left_out() {
    // A folder whose name ends in a dot exists only through the `\\?\` prefix: the listing names it, and
    // its metadata by the plain path is a missing file, so the scan passes it over.
    let flavor = TempFlavor::new("trailing-dot");
    std::fs::create_dir_all(flavor.addons()).expect("AddOns");
    let verbatim = PathBuf::from(format!(
        "\\\\?\\{}",
        flavor.addons().join("Dotted.").display()
    ));
    std::fs::create_dir(&verbatim).expect("the folder with a trailing dot");
    std::fs::create_dir_all(flavor.addons().join("Plain")).expect("Plain");
    std::fs::write(
        flavor.addons().join("Plain").join("Plain.toc"),
        "## Interface: 16001\n",
    )
    .expect("its TOC");
    let scan = scan::scan(&flavor.root).expect("scans");
    assert_eq!(
        scan.addons
            .iter()
            .map(|addon| addon.folder.as_str())
            .collect::<Vec<_>>(),
        ["Plain"]
    );
    assert!(scan.skipped.is_empty(), "{:?}", scan.skipped);
    assert!(scan.warnings.is_empty(), "{:?}", scan.warnings);
    let _ = std::fs::remove_dir(&verbatim);
}

#[test]
fn a_flavor_info_that_cannot_be_read_or_understood_is_an_error() {
    let flavor = TempFlavor::new("bad-flavor-info");
    std::fs::write(
        flavor.root.join(".flavor.info"),
        "Product Flavor!STRING:0\nwow f\n",
    )
    .expect("a bad id");
    let error = scan::scan(&flavor.root).expect_err("not a flavor folder");
    assert!(
        matches!(&error, Error::NotAFlavorFolder { reason, .. } if reason == "unreadable flavor id `wow f` in .flavor.info"),
        "{error}"
    );
    std::fs::remove_file(flavor.root.join(".flavor.info")).expect("removed");
    std::fs::create_dir(flavor.root.join(".flavor.info")).expect("a folder in the file's place");
    let error = scan::scan(&flavor.root).expect_err("cannot be read");
    assert!(
        matches!(&error, Error::Io { context, .. } if *context == "reading .flavor.info"),
        "{error}"
    );
}

#[test]
fn a_missing_or_unreadable_build_info_is_named_in_the_warning() {
    let flavor = TempFlavor::new("no-build-info");
    let read = flavor::read_flavor(&flavor.root).expect("a flavor");
    assert_eq!(read.version, None);
    assert_eq!(
        read.warnings,
        [format!(
            "build unknown: no .build.info next to {}",
            read.folder.display()
        )]
    );
    // A game folder with the flavor in it, whose `.build.info` is a folder: it cannot be read.
    let game = flavor.root.join("game");
    let forever = game.join("_forever_");
    std::fs::create_dir_all(game.join(".build.info")).expect("a folder in the file's place");
    std::fs::create_dir_all(&forever).expect("the flavor folder");
    std::fs::copy(
        flavor.root.join(".flavor.info"),
        forever.join(".flavor.info"),
    )
    .expect("copied");
    let read = flavor::read_flavor(&forever).expect("a flavor");
    assert_eq!(read.warnings.len(), 1, "{:?}", read.warnings);
    assert!(
        read.warnings[0].starts_with(&format!(
            "build unknown: cannot read {}: ",
            read.folder
                .parent()
                .expect("the game folder")
                .join(".build.info")
                .display()
        )),
        "{}",
        read.warnings[0]
    );
}

#[test]
fn list_flavors_leaves_out_a_folder_that_is_no_flavor_and_fails_on_a_root_it_cannot_read() {
    let flavor = TempFlavor::new("list-flavors");
    let error = flavor::list_flavors(&flavor.root.join("missing")).expect_err("no root");
    assert!(
        matches!(&error, Error::Io { context, .. } if *context == "reading the game folder"),
        "{error}"
    );
    let game = flavor.root.join("game");
    for (folder, info) in [
        ("_forever_", "Product Flavor!STRING:0\nwowf\n"),
        ("_odd_", "Product Flavor!STRING:0\nwow f\n"),
    ] {
        std::fs::create_dir_all(game.join(folder)).expect("the folder");
        std::fs::write(game.join(folder).join(".flavor.info"), info).expect(".flavor.info");
    }
    let flavors = flavor::list_flavors(&game).expect("lists");
    assert_eq!(
        flavors.iter().map(|f| f.folder_name()).collect::<Vec<_>>(),
        ["_forever_"],
        "a folder whose .flavor.info names no flavor is left out"
    );
}

#[cfg(windows)]
#[test]
fn list_flavors_fails_on_a_flavor_info_it_cannot_read() {
    use mrgl_test_support::locked;
    let flavor = TempFlavor::new("list-flavors-locked");
    let game = flavor.root.join("game");
    let info = game.join("_forever_").join(".flavor.info");
    std::fs::create_dir_all(game.join("_forever_")).expect("the folder");
    std::fs::write(&info, "Product Flavor!STRING:0\nwowf\n").expect(".flavor.info");
    let open = locked::file(&info);
    let error = flavor::list_flavors(&game).expect_err("the file cannot be read");
    assert!(
        matches!(&error, Error::Io { context, .. } if *context == "reading .flavor.info"),
        "{error}"
    );
    drop(open);
    assert_eq!(flavor::list_flavors(&game).expect("lists").len(), 1);
}
