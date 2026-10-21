//! The TOC interface against the flavor's build (task 015): an addon whose primary TOC names no interface
//! number of the installed build's major version was most likely made for another flavor or an older
//! expansion, the first sign of a wrong flavor pick. The check is a warning, never a refusal: the client
//! decides what it loads.
//!
//! The rule: the major version of an interface number is the number divided by 10000 (`120105` is 12,
//! `16001` is 1, `50504` is 5, the same arithmetic as [`BuildVersion::interface`]), and an addon passes when
//! one number of its `## Interface` list has the build's major. Only the TOC the flavor loads counts (the
//! primary TOC of an addon the flavor loads at all; one it loads nothing from carries its own flag in a
//! listing). A TOC without an `## Interface` line and a flavor whose build is unknown give no warning:
//! there is nothing to compare.

use serde::Serialize;

use crate::flavor::BuildVersion;
use crate::scan::{Addon, Scan};

/// An addon whose loaded TOC names no interface of the build's major version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceMismatch {
    /// The addon folder under `Interface\AddOns`.
    pub folder: String,
    /// The TOC file the numbers come from.
    pub toc: String,
    /// The TOC's `## Interface` numbers.
    pub interfaces: Vec<u32>,
    /// The flavor's build, `12.1.5.70448`.
    pub build: String,
    /// The build's major version, the one an interface number has to carry.
    pub major: u32,
}

impl InterfaceMismatch {
    /// The interface numbers as the TOC lists them, comma-separated: `110207` or `11509,50504`.
    pub fn interface_list(&self) -> String {
        self.interfaces
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The plain line: `StaleAddon: interface 110207 is not for the build 12.1.5.70448`.
    pub fn line(&self) -> String {
        format!(
            "{}: interface {} is not for the build {}",
            self.folder,
            self.interface_list(),
            self.build
        )
    }
}

/// The major version an interface number stands for: `120105` is 12, `16001` is 1.
pub const fn interface_major(interface: u32) -> u32 {
    interface / 10000
}

/// Whether one of the numbers has the build's major version; an empty list passes (nothing to compare).
pub fn matches_build(interfaces: &[u32], build: BuildVersion) -> bool {
    interfaces.is_empty()
        || interfaces
            .iter()
            .any(|interface| interface_major(*interface) == build.major)
}

/// Checks one scanned addon against a build; `None` when it passes, the build is unknown or the flavor
/// loads nothing from the folder.
pub fn check_addon(addon: &Addon, build: Option<BuildVersion>) -> Option<InterfaceMismatch> {
    let build = build?;
    addon.chosen.as_ref()?;
    let toc = addon.primary_toc()?;
    if matches_build(&toc.toc.interface, build) {
        return None;
    }
    Some(InterfaceMismatch {
        folder: addon.folder.clone(),
        toc: toc.file.clone(),
        interfaces: toc.toc.interface.clone(),
        build: build.to_string(),
        major: build.major,
    })
}

/// Every addon of a scan that fails the check, in the scan's order.
pub fn check_scan(scan: &Scan) -> Vec<InterfaceMismatch> {
    scan.addons
        .iter()
        .filter_map(|addon| check_addon(addon, scan.flavor.version))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::TocFile;
    use crate::toc::parse_toc;
    use std::path::{Path, PathBuf};

    fn addon(folder: &str, interface: &str, chosen: bool) -> Addon {
        let file = format!("{folder}.toc");
        Addon {
            folder: folder.to_string(),
            path: PathBuf::from(folder),
            tocs: vec![TocFile {
                file: file.clone(),
                suffix: None,
                legacy: false,
                toc: parse_toc(&format!("## Interface: {interface}\n## Version: 1.0.0\n")),
            }],
            other_tocs: Vec::new(),
            chosen: chosen.then_some(file),
            linked: false,
        }
    }

    fn build(text: &str) -> Option<BuildVersion> {
        BuildVersion::parse(text)
    }

    #[test]
    fn an_interface_of_another_major_is_a_mismatch_with_a_plain_line() {
        let mismatch = check_addon(&addon("Stale", "110207", true), build("12.1.5.70448"))
            .expect("a mismatch");
        assert_eq!(mismatch.folder, "Stale");
        assert_eq!(mismatch.toc, "Stale.toc");
        assert_eq!(mismatch.interfaces, [110207]);
        assert_eq!(mismatch.major, 12);
        assert_eq!(
            mismatch.line(),
            "Stale: interface 110207 is not for the build 12.1.5.70448"
        );
        let retail_in_forever =
            check_addon(&addon("Retail", "120105", true), build("1.60.1.70245"))
                .expect("a mismatch");
        assert_eq!(
            retail_in_forever.line(),
            "Retail: interface 120105 is not for the build 1.60.1.70245"
        );
    }

    #[test]
    fn the_same_major_passes_and_one_match_in_a_list_is_enough() {
        let forever = build("1.60.1.70245");
        assert_eq!(check_addon(&addon("Fits", "16001", true), forever), None);
        assert_eq!(
            check_addon(&addon("Era", "11509", true), forever),
            None,
            "the major decides, not the minor"
        );
        assert_eq!(
            check_addon(&addon("Multi", "120105, 50504, 11509", true), forever),
            None
        );
        let mismatch =
            check_addon(&addon("Two", "120105,50504", true), forever).expect("a mismatch");
        assert_eq!(mismatch.interface_list(), "120105,50504");
    }

    #[test]
    fn an_unknown_build_no_interface_or_an_unloaded_folder_give_no_warning() {
        assert_eq!(check_addon(&addon("Stale", "110207", true), None), None);
        assert_eq!(
            check_addon(&addon("Unloaded", "110207", false), build("12.1.5.70448")),
            None
        );
        let mut empty = addon("NoInterface", "", true);
        empty.tocs[0].toc = parse_toc("## Title: No Interface\n");
        assert_eq!(check_addon(&empty, build("12.1.5.70448")), None);
        assert_eq!(interface_major(120105), 12);
        assert_eq!(interface_major(16001), 1);
        assert_eq!(interface_major(90207), 9);
    }

    #[test]
    fn the_fixture_flavors_have_no_mismatch() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flavors");
        for flavor in ["_retail_", "_classic_", "_classic_era_", "_unknown_"] {
            let scan = crate::scan::scan(&fixtures.join(flavor)).expect("the fixture scans");
            assert!(!scan.addons.is_empty(), "{flavor}");
            assert_eq!(check_scan(&scan), [], "{flavor}");
        }
    }
}
