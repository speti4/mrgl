//! The dependency check (task 029) over the synthetic game folder `fixtures/dependencies/`: `Needy`
//! requires `LibMet` (an addon of its own, met), `LibEmbedded` (a folder inside `Needy`, embedded) and
//! `LibMissing` (nowhere, missing); `NolibBuild` carries a `LibEmbedded` folder too, which counts only
//! while the sidecar does not mark it a `nolib` build. Offline, read-only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_core::dependencies::{self, AddonDependencies};
use mrgl_core::flavor::FlavorId;
use mrgl_core::scan;
use mrgl_core::state::{Entry, State};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dependencies/_forever_")
}

fn of<'a>(checks: &'a [AddonDependencies], folder: &str) -> &'a AddonDependencies {
    checks
        .iter()
        .find(|check| check.folder == folder)
        .unwrap_or_else(|| panic!("no check for {folder}"))
}

#[test]
fn met_embedded_and_missing_over_the_fixture() {
    let scan = scan::scan(&fixture()).expect("the fixture scans");
    let checks = dependencies::check(&scan, None);
    assert_eq!(checks.len(), 3, "one per addon: {checks:?}");

    let needy = of(&checks, "Needy");
    assert_eq!(needy.required, ["LibMet", "LibEmbedded", "LibMissing"]);
    assert_eq!(needy.met, ["LibMet"]);
    assert_eq!(needy.embedded, ["LibEmbedded"]);
    assert_eq!(needy.missing, ["LibMissing"]);
    assert_eq!(needy.nolib, None);

    let lib = of(&checks, "LibMet");
    assert!(lib.required.is_empty() && !lib.has_missing());

    let unknown = of(&checks, "NolibBuild");
    assert_eq!(unknown.met, ["LibMet"]);
    assert_eq!(
        unknown.embedded,
        ["LibEmbedded"],
        "without the flag an embedded library is possible"
    );
    assert!(!unknown.has_missing());
}

#[test]
fn the_sidecars_nolib_flag_turns_the_folder_inside_into_a_missing_dependency() {
    let scan = scan::scan(&fixture()).expect("the fixture scans");
    let mut state = State::new(&FlavorId::Forever);
    state.addons.insert(
        "NolibBuild".to_string(),
        Entry {
            source: "github:someone/nolib-build".to_string(),
            version: "1.0.0".to_string(),
            previous_version: None,
            folders: vec!["NolibBuild".to_string()],
            files: BTreeMap::new(),
            installed_at: "2026-10-21T14:03:05Z".to_string(),
            nolib: Some(true),
        },
    );
    let checks = dependencies::check(&scan, Some(&state));
    let nolib = of(&checks, "NolibBuild");
    assert_eq!(nolib.nolib, Some(true));
    assert!(nolib.embedded.is_empty());
    assert_eq!(nolib.missing, ["LibEmbedded"]);
    assert_eq!(
        nolib.warning().as_deref(),
        Some(
            "NolibBuild needs the missing dependency LibEmbedded: install it or the addon does not load (a nolib build: its libraries are addons of their own)"
        )
    );
    assert_eq!(
        of(&checks, "Needy").embedded,
        ["LibEmbedded"],
        "an addon the sidecar does not know stays unknown"
    );
}
