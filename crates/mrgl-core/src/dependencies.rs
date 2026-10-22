//! The dependency check (task 029): every required dependency of an addon (`## Dependencies`,
//! `## RequiredDeps` and the other `## Dep…` lines of the TOC the flavor loads, [`crate::toc`]) is looked up
//! against a scan, ASCII case-insensitive as the client matches folder names:
//!
//! - **met**: a folder of that name is in the scan, an addon of its own, or the name starts with
//!   [`crate::scan::CLIENT_ADDON_PREFIX`] (`Blizzard_`), an addon the client itself ships, which no
//!   AddOns folder holds ([`is_client_addon`], the rule `savedvars` shares);
//! - **missing**: no addon folder of its own answers it. The client does not count a library embedded in
//!   the addon's own folder (task 076: a required name answered only by `Libs\LibStub` shows `Dependency
//!   missing` in the client and the addon does not load), so such a name is missing too, and the warning
//!   says why;
//! - **embedded**: informative, a subset of *missing*: a folder of that name sits inside the addon's own
//!   folder (`Libs\LibStub`, at most [`EMBED_DEPTH`] levels down), the library a build without `nolib`
//!   carries; a `nolib` build carries none, so for an entry the sidecar marks `nolib` the check does not
//!   look inside.
//!
//! An optional dependency (`## OptionalDeps`) is never reported. The sidecar's `nolib` flag
//! ([`crate::state::Entry::nolib`]) says whether an embedded library is expected; an addon without the
//! flag (unmanaged, a zip in hand) is treated as one that may embed. The check only reports: a missing
//! dependency is a warning, never a refusal.

use std::path::Path;

use serde::Serialize;

use crate::scan::{Addon, Scan, is_client_addon};
use crate::state::State;

/// How many folder levels below an addon folder the check looks for an embedded library: `Libs\LibStub`
/// is two, `Libs\Ace3\AceAddon-3.0` three.
pub const EMBED_DEPTH: usize = 3;

/// The dependencies of one addon folder, as the check found them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AddonDependencies {
    /// The addon folder under `Interface\AddOns`.
    pub folder: String,
    /// The required names of the TOC the flavor loads, in TOC order, each once (ASCII case-insensitive);
    /// empty when the flavor loads no TOC of the folder.
    pub required: Vec<String>,
    /// The required names an addon folder of the scan answers, or the client: a `Blizzard_` name.
    pub met: Vec<String>,
    /// The missing names a folder inside the addon's own folder answers: an embedded library, which the
    /// client does not count (task 076), so each is in `missing` too.
    pub embedded: Vec<String>,
    /// The required names no addon folder of its own answers, the embedded ones included: the client does
    /// not load the addon until they are installed.
    pub missing: Vec<String>,
    /// The sidecar's `nolib` flag of the addon: `Some(true)` a `nolib` build (no embedded library is
    /// expected, none is looked for), `Some(false)` a build with its libraries, `None` unknown.
    pub nolib: Option<bool>,
}

impl AddonDependencies {
    /// Whether a required dependency is missing.
    pub fn has_missing(&self) -> bool {
        !self.missing.is_empty()
    }

    /// The warning line for a missing dependency, plain English; `None` when nothing is missing.
    pub fn warning(&self) -> Option<String> {
        if !self.has_missing() {
            return None;
        }
        let what = match self.missing.len() {
            1 => "dependency",
            _ => "dependencies",
        };
        let mut line = format!(
            "{} needs the missing {what} {}: install {} or the addon does not load",
            self.folder,
            self.missing.join(", "),
            if self.missing.len() == 1 {
                "it"
            } else {
                "them"
            },
        );
        if self.nolib == Some(true) {
            line.push_str(" (a nolib build: its libraries are addons of their own)");
        }
        if !self.embedded.is_empty() {
            line.push_str(&format!(
                " ({} {} only embedded inside {}, and the client does not count a library inside an addon's own folder)",
                self.embedded.join(", "),
                if self.embedded.len() == 1 { "is" } else { "are" },
                self.folder,
            ));
        }
        Some(line)
    }
}

/// Checks every addon of a scan, reading the subfolders of an addon from disk only when a required name
/// is not an addon of the scan. `state` gives the `nolib` flags; `None` treats every addon as unknown.
pub fn check(scan: &Scan, state: Option<&State>) -> Vec<AddonDependencies> {
    check_with(scan, state, |addon| {
        subfolder_names(&addon.path, EMBED_DEPTH)
    })
}

/// [`check`] with the subfolder lookup given: `subfolders` answers the folder names inside an addon's
/// folder (every level the caller wants searched). Pure: the tests pass a closure over a list.
pub fn check_with(
    scan: &Scan,
    state: Option<&State>,
    mut subfolders: impl FnMut(&Addon) -> Vec<String>,
) -> Vec<AddonDependencies> {
    scan.addons
        .iter()
        .map(|addon| {
            let nolib = state
                .and_then(|state| state.entry_for_folder(&addon.folder))
                .and_then(|(_, entry)| entry.nolib);
            check_addon(addon, scan, nolib, &mut subfolders)
        })
        .collect()
}

/// One addon against the scan it belongs to.
fn check_addon(
    addon: &Addon,
    scan: &Scan,
    nolib: Option<bool>,
    subfolders: &mut impl FnMut(&Addon) -> Vec<String>,
) -> AddonDependencies {
    let mut result = AddonDependencies {
        folder: addon.folder.clone(),
        nolib,
        ..AddonDependencies::default()
    };
    let Some(toc) = addon.chosen_toc() else {
        return result;
    };
    for name in &toc.toc.dependencies {
        if !contains(&result.required, name) {
            result.required.push(name.clone());
        }
    }
    let mut inside: Option<Vec<String>> = None;
    for name in &result.required {
        let installed = is_client_addon(name)
            || scan
                .addons
                .iter()
                .any(|other| other.folder.eq_ignore_ascii_case(name));
        if installed {
            result.met.push(name.clone());
        } else if nolib != Some(true)
            && contains(inside.get_or_insert_with(|| subfolders(addon)), name)
        {
            result.embedded.push(name.clone());
            result.missing.push(name.clone());
        } else {
            result.missing.push(name.clone());
        }
    }
    result
}

fn contains(names: &[String], name: &str) -> bool {
    names.iter().any(|known| known.eq_ignore_ascii_case(name))
}

/// The names of the folders below a folder, `depth` levels deep, in no particular order; an unreadable
/// folder adds nothing (the check reports, it never fails on a folder it cannot read).
pub fn subfolder_names(dir: &Path, depth: usize) -> Vec<String> {
    let mut names = Vec::new();
    collect_subfolders(dir, depth, &mut names);
    names
}

fn collect_subfolders(dir: &Path, depth: usize, names: &mut Vec<String>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            names.push(entry.file_name().to_string_lossy().into_owned());
            collect_subfolders(&entry.path(), depth - 1, names);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::flavor::{BuildVersion, Flavor, FlavorId};
    use crate::scan::TocFile;
    use crate::state::Entry;
    use crate::toc;

    fn addon(folder: &str, toc_text: Option<&str>) -> Addon {
        let tocs: Vec<TocFile> = toc_text
            .map(|text| TocFile {
                file: format!("{folder}.toc"),
                suffix: None,
                legacy: false,
                toc: toc::parse_toc(text),
            })
            .into_iter()
            .collect();
        Addon {
            folder: folder.to_string(),
            path: PathBuf::from(folder),
            chosen: tocs.first().map(|file| file.file.clone()),
            tocs,
            other_tocs: Vec::new(),
            linked: false,
        }
    }

    fn scan_of(addons: Vec<Addon>) -> Scan {
        Scan {
            flavor: Flavor {
                id: FlavorId::Forever,
                folder: PathBuf::from("_forever_"),
                version: BuildVersion::parse("1.60.1.70245"),
                version_source: None,
                curse_client: false,
                warnings: Vec::new(),
            },
            addons,
            skipped: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn state_with(folder: &str, nolib: Option<bool>) -> State {
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert(
            folder.to_string(),
            Entry {
                source: "github:someone/something".to_string(),
                version: "1.0.0".to_string(),
                previous_version: None,
                folders: vec![folder.to_string()],
                files: BTreeMap::new(),
                installed_at: "2026-10-21T14:03:05Z".to_string(),
                nolib,
                pinned: None,
                ignored: false,
                channel: crate::sources::Channel::Stable,
                forever_build: None,
            },
        );
        state
    }

    /// The subfolder lookup of the tests: `Needy` embeds `LibEmbedded` under `Libs`.
    fn inside(addon: &Addon) -> Vec<String> {
        match addon.folder.as_str() {
            "Needy" => vec!["Libs".to_string(), "LibEmbedded".to_string()],
            _ => Vec::new(),
        }
    }

    fn needy_scan() -> Scan {
        scan_of(vec![
            addon("LibMet", Some("## Title: LibMet\n")),
            addon(
                "Needy",
                Some(
                    "## Dependencies: LibMet, LibEmbedded, LibMissing\n## RequiredDeps: libmet\n## OptionalDeps: LibOptional\n",
                ),
            ),
        ])
    }

    #[test]
    fn two_embedded_libraries_are_named_together() {
        let scan = scan_of(vec![addon(
            "Needy",
            Some("## Dependencies: LibOne, LibTwo\n"),
        )]);
        let needy = &check_with(&scan, None, |_| {
            vec!["LibOne".to_string(), "LibTwo".to_string()]
        })[0];
        assert_eq!(needy.embedded, ["LibOne", "LibTwo"]);
        assert_eq!(
            needy.warning().as_deref(),
            Some(
                "Needy needs the missing dependencies LibOne, LibTwo: install them or the addon does not load (LibOne, LibTwo are only embedded inside Needy, and the client does not count a library inside an addon's own folder)"
            )
        );
    }

    #[test]
    fn a_required_name_is_met_embedded_or_missing() {
        let checks = check_with(&needy_scan(), None, inside);
        assert_eq!(checks.len(), 2);
        let needy = &checks[1];
        assert_eq!(needy.folder, "Needy");
        assert_eq!(needy.required, ["LibMet", "LibEmbedded", "LibMissing"]);
        assert_eq!(needy.met, ["LibMet"]);
        assert_eq!(needy.embedded, ["LibEmbedded"]);
        assert_eq!(
            needy.missing,
            ["LibEmbedded", "LibMissing"],
            "the client does not count an embedded library (task 076)"
        );
        assert_eq!(needy.nolib, None);
        assert!(needy.has_missing());
        assert_eq!(
            needy.warning().as_deref(),
            Some(
                "Needy needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is only embedded inside Needy, and the client does not count a library inside an addon's own folder)"
            )
        );
        let lib = &checks[0];
        assert!(lib.required.is_empty() && !lib.has_missing());
        assert_eq!(lib.warning(), None);
    }

    #[test]
    fn an_optional_dependency_is_never_reported() {
        let scan = scan_of(vec![addon("Lonely", Some("## OptionalDeps: LibNowhere\n"))]);
        let checks = check_with(&scan, None, inside);
        assert_eq!(
            checks[0],
            AddonDependencies {
                folder: "Lonely".to_string(),
                ..AddonDependencies::default()
            }
        );
    }

    #[test]
    fn the_scan_matches_case_insensitively() {
        let scan = scan_of(vec![
            addon("libmet", Some("## Title: lower case on disk\n")),
            addon("Needy", Some("## Dependencies: LIBMET\n")),
        ]);
        let checks = check_with(&scan, None, inside);
        assert_eq!(checks[1].met, ["LIBMET"]);
    }

    #[test]
    fn a_nolib_build_does_not_count_a_folder_inside_as_embedded() {
        let scan = needy_scan();
        let state = state_with("Needy", Some(true));
        let needy = &check_with(&scan, Some(&state), inside)[1];
        assert_eq!(needy.nolib, Some(true));
        assert!(needy.embedded.is_empty());
        assert_eq!(needy.missing, ["LibEmbedded", "LibMissing"]);
        assert_eq!(
            needy.warning().as_deref(),
            Some(
                "Needy needs the missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (a nolib build: its libraries are addons of their own)"
            )
        );
        let with_libraries = state_with("Needy", Some(false));
        let needy = &check_with(&scan, Some(&with_libraries), inside)[1];
        assert_eq!(needy.nolib, Some(false));
        assert_eq!(needy.embedded, ["LibEmbedded"]);
        assert_eq!(needy.missing, ["LibEmbedded", "LibMissing"]);
    }

    #[test]
    fn an_embedded_only_dependency_is_missing_with_the_reason() {
        let scan = scan_of(vec![addon("Needy", Some("## Dependencies: LibEmbedded\n"))]);
        let needy = &check_with(&scan, None, inside)[0];
        assert_eq!(needy.required, ["LibEmbedded"]);
        assert_eq!(needy.embedded, ["LibEmbedded"]);
        assert_eq!(needy.missing, ["LibEmbedded"]);
        assert_eq!(
            needy.warning().as_deref(),
            Some(
                "Needy needs the missing dependency LibEmbedded: install it or the addon does not load (LibEmbedded is only embedded inside Needy, and the client does not count a library inside an addon's own folder)"
            )
        );
        let optional = scan_of(vec![addon("Needy", Some("## OptionalDeps: LibEmbedded\n"))]);
        let optional = &check_with(&optional, None, |_| panic!("no folder lookup"))[0];
        assert!(
            optional.required.is_empty() && optional.embedded.is_empty() && !optional.has_missing(),
            "an optional dependency, embedded or not, is never reported"
        );
    }

    #[test]
    fn the_subfolders_are_read_only_when_the_scan_does_not_answer() {
        let scan = scan_of(vec![
            addon("LibMet", Some("## Title: LibMet\n")),
            addon("Content", Some("## Dependencies: LibMet\n")),
            addon("Needy", Some("## Dependencies: LibA, LibB\n")),
        ]);
        let mut asked = Vec::new();
        check_with(&scan, None, |addon| {
            asked.push(addon.folder.clone());
            Vec::new()
        });
        assert_eq!(asked, ["Needy"], "once per addon, and only when needed");
    }

    #[test]
    fn a_blizzard_addon_is_met_by_the_client() {
        let scan = scan_of(vec![addon(
            "Needy",
            Some(
                "## Dependencies: Blizzard_Something, LibMissing\n## RequiredDeps: blizzard_lower\n",
            ),
        )]);
        let mut asked = false;
        let needy = &check_with(&scan, None, |_| {
            asked = true;
            Vec::new()
        })[0];
        assert_eq!(needy.met, ["Blizzard_Something", "blizzard_lower"]);
        assert_eq!(
            needy.missing,
            ["LibMissing"],
            "an addon of its own still is"
        );
        assert!(asked, "the folder lookup for LibMissing only");
        let calm = scan_of(vec![addon(
            "Calm",
            Some("## Dependencies: Blizzard_Something\n"),
        )]);
        let calm = &check_with(&calm, None, |_| panic!("no folder lookup for the client"))[0];
        assert_eq!(calm.met, ["Blizzard_Something"]);
        assert!(!calm.has_missing() && calm.warning().is_none());
        assert!(is_client_addon("BLIZZARD_x"));
        assert!(!is_client_addon("Blizzard") && !is_client_addon("MyBlizzard_Thing"));
        assert!(!is_client_addon("Bliz") && !is_client_addon("Blizzárd_x"));
    }

    #[test]
    fn a_folder_the_flavor_loads_no_toc_of_has_nothing_to_check() {
        let mut camelot = addon("Camelot", Some("## Dependencies: LibMissing\n"));
        camelot.chosen = None;
        let checks = check_with(&scan_of(vec![camelot]), None, inside);
        assert!(checks[0].required.is_empty() && checks[0].missing.is_empty());
    }

    #[test]
    fn the_subfolder_walk_stops_at_its_depth() {
        let dir =
            std::env::temp_dir().join(format!("mrgl-dependencies-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Libs").join("LibStub").join("Deep").join("Deeper"))
            .expect("folders");
        std::fs::write(dir.join("core.lua"), "-- a file, not a folder\n").expect("a file");
        let mut names = subfolder_names(&dir, 3);
        names.sort();
        assert_eq!(names, ["Deep", "LibStub", "Libs"]);
        assert!(subfolder_names(&dir.join("nowhere"), 3).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
