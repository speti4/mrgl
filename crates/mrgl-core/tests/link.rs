//! Dev addon linking (task 005) over the synthetic development folder `tests/fixtures/dev/` (two addons
//! with a TOC, `OneFolder` and `DevAddon`, and `Media` without one), copied into a temporary folder so a
//! test can never touch the fixture, against a temporary game folder and mrgl home: link, the scan lists
//! the junctions as linked, an install or a removal over a linked addon is refused, a real folder or a
//! foreign junction in the way is skipped, unlink removes exactly the junctions and leaves the
//! development folder byte for byte as it was. A single addon folder (`tests/fixtures/single-addon/`, task
//! 062) links and unlinks as that one addon. The junction tests are Windows-only; elsewhere the two calls
//! answer that junctions are Windows-only.

// The helpers serve the Windows tests; elsewhere only the Windows-only answer is tested.
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_core::Error;
use mrgl_core::flavor;
use mrgl_core::install::{Engine, Metadata};
use mrgl_core::link;

/// A temporary game folder with one `_forever_` flavor, a temporary mrgl home and a copy of the fixture
/// development folder, all under one folder that the drop removes.
struct Sandbox {
    root: PathBuf,
    dev: PathBuf,
    engine: Engine,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root =
            std::env::temp_dir().join(format!("mrgl-link-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("game");
        let flavor_dir = game.join("_forever_");
        std::fs::create_dir_all(&flavor_dir).expect("the flavor folder");
        std::fs::write(
            game.join(".build.info"),
            "Version!STRING:0|Product!STRING:0\n1.60.1.70245|wowf\n",
        )
        .expect(".build.info");
        std::fs::write(
            flavor_dir.join(".flavor.info"),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect(".flavor.info");
        let dev = root.join("dev");
        copy_tree(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev"),
            &dev,
        );
        let flavor = flavor::read_flavor(&flavor_dir).expect("a flavor");
        let engine = Engine::with_home(flavor, &root.join("home"));
        Sandbox { root, dev, engine }
    }

    fn addon(&self, folder: &str) -> PathBuf {
        self.engine.addons_dir().join(folder)
    }

    /// The names in the AddOns folder, sorted.
    fn addons_listing(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.engine.addons_dir())
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the folder");
    for entry in std::fs::read_dir(from).expect("read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// A tree as relative paths to the bytes of every file, folders as names with a trailing `/`.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("read") {
            let entry = entry.expect("entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().expect("type").is_dir() {
                into.insert(format!("{prefix}{name}/"), Vec::new());
                walk(&entry.path(), &format!("{prefix}{name}/"), into);
            } else {
                into.insert(
                    format!("{prefix}{name}"),
                    std::fs::read(entry.path()).expect("bytes"),
                );
            }
        }
    }
    let mut into = BTreeMap::new();
    walk(dir, "", &mut into);
    into
}

/// A copy of the synthetic addon folder `tests/fixtures/single-addon/SoloAddon` (a TOC with the Forever
/// suffix only, and an embedded `LibSolo` folder with a TOC of its own) under the sandbox, as a
/// repository that is the addon folder itself (task 062).
fn solo_copy(sandbox: &Sandbox) -> PathBuf {
    let solo = sandbox.root.join("repos").join("SoloAddon");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/single-addon/SoloAddon"),
        &solo,
    );
    solo
}

fn zip(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/zips")
        .join(name)
}

fn metadata_for(zip: &Path) -> Metadata {
    Metadata {
        source: zip.display().to_string(),
        version: None,
        name: None,
        nolib: None,
    }
}

#[cfg(not(windows))]
#[test]
fn link_and_unlink_are_windows_only_elsewhere() {
    let sandbox = Sandbox::new("elsewhere");
    for result in [
        link::link(&sandbox.engine, &sandbox.dev).map(|_| ()),
        link::unlink(&sandbox.engine, &sandbox.dev).map(|_| ()),
    ] {
        let error = result.expect_err("refused");
        assert!(matches!(error, Error::JunctionsWindowsOnly), "{error}");
        assert_eq!(
            error.to_string(),
            "junctions are Windows-only: link and unlink work on Windows"
        );
    }
    assert!(sandbox.addons_listing().is_empty());
}

#[cfg(windows)]
#[test]
fn link_scan_refuse_and_unlink_leave_the_development_folder_untouched() {
    let sandbox = Sandbox::new("round");
    let before = snapshot(&sandbox.dev);

    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("links");
    let dev = link::dev_folder(&sandbox.dev).expect("the folder");
    assert_eq!(linked.folder, dev);
    assert_eq!(linked.source, format!("link:{}", dev.display()));
    let names: Vec<(&str, &str, bool)> = linked
        .linked
        .iter()
        .map(|addon| {
            (
                addon.name.as_str(),
                addon.version.as_str(),
                addon.already_linked,
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("DevAddon", "unknown", false),
            ("OneFolder", "9.9.9-dev", false)
        ]
    );
    assert!(linked.skipped.is_empty(), "{:?}", linked.skipped);
    assert_eq!(linked.without_toc, ["Media"]);
    assert_eq!(sandbox.addons_listing(), ["DevAddon", "OneFolder"]);
    assert!(link::is_link(&sandbox.addon("OneFolder")));
    assert!(
        link::same_path(
            &link::link_target(&sandbox.addon("OneFolder")).expect("a target"),
            &dev.join("OneFolder")
        ),
        "the junction points into the development folder"
    );

    // The scan reads through the junctions and flags them as linked (the `linked` flag of task 002).
    let scan = mrgl_core::scan::scan_flavor(&sandbox.engine.flavor).expect("scans");
    let flags: Vec<(&str, bool, Option<&str>)> = scan
        .addons
        .iter()
        .map(|addon| (addon.folder.as_str(), addon.linked, addon.version()))
        .collect();
    assert_eq!(
        flags,
        [
            ("DevAddon", true, None),
            ("OneFolder", true, Some("9.9.9-dev"))
        ]
    );

    // The sidecar knows them by the `link:` source, with no file list.
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    let (_, entry) = state.entry("OneFolder").expect("an entry");
    assert_eq!(entry.source, linked.source);
    assert_eq!(entry.version, "9.9.9-dev");
    assert_eq!(entry.folders, ["OneFolder"]);
    assert!(entry.files.is_empty());

    // A second link changes nothing and says so.
    let again = link::link(&sandbox.engine, &sandbox.dev).expect("links again");
    assert!(again.linked.iter().all(|addon| addon.already_linked));
    assert_eq!(again.linked.len(), 2);

    // An install over a linked addon is refused before anything moves, and so is a removal.
    let zip = zip("one-folder-1.0.0.zip");
    let error = sandbox
        .engine
        .install(&zip, &metadata_for(&zip))
        .expect_err("refused");
    assert!(
        matches!(&error, Error::LinkedAddon { action: "install over", name, .. } if name == "OneFolder"),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "refused to install over OneFolder (flavor forever): it is linked from {}; `mrgl unlink` with that folder removes the link",
            dev.display()
        )
    );
    let error = sandbox.engine.remove("onefolder").expect_err("refused");
    assert!(
        matches!(
            &error,
            Error::LinkedAddon {
                action: "remove",
                ..
            }
        ),
        "{error}"
    );
    assert!(link::is_link(&sandbox.addon("OneFolder")));
    assert_eq!(snapshot(&sandbox.dev), before);

    // Unlink removes exactly the two junctions and their entries.
    let unlinked = link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");
    assert_eq!(unlinked.unlinked, ["DevAddon", "OneFolder"]);
    assert!(unlinked.kept.is_empty(), "{:?}", unlinked.kept);
    assert!(sandbox.addons_listing().is_empty());
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    assert!(state.addons.is_empty(), "{state:?}");
    assert_eq!(snapshot(&sandbox.dev), before, "the development folder");

    // A second unlink finds nothing; the install that was refused now goes through.
    let again = link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");
    assert!(again.unlinked.is_empty() && again.kept.is_empty());
    sandbox
        .engine
        .install(&zip, &metadata_for(&zip))
        .expect("installs");
    assert!(!link::is_link(&sandbox.addon("OneFolder")));
    assert_eq!(snapshot(&sandbox.dev), before);
}

#[cfg(windows)]
#[test]
fn a_real_folder_or_a_foreign_junction_in_the_way_is_skipped_and_kept() {
    let sandbox = Sandbox::new("in-the-way");
    std::fs::create_dir_all(sandbox.addon("DevAddon")).expect("a real folder");
    std::fs::write(sandbox.addon("DevAddon").join("mine.lua"), "-- mine\n").expect("a file");
    let elsewhere = sandbox.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("another folder");
    let elsewhere = link::dev_folder(&elsewhere).expect("canonical");
    link::create_junction(&sandbox.addon("OneFolder"), &elsewhere).expect("a foreign junction");

    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("runs");
    assert!(linked.linked.is_empty(), "{:?}", linked.linked);
    let reasons: Vec<(&str, &str)> = linked
        .skipped
        .iter()
        .map(|skipped| (skipped.name.as_str(), skipped.reason.as_str()))
        .collect();
    assert_eq!(reasons.len(), 2);
    assert_eq!(
        reasons[0],
        (
            "DevAddon",
            "a folder of that name is in AddOns already and stays (not a junction)"
        )
    );
    assert_eq!(reasons[1].0, "OneFolder");
    assert_eq!(
        reasons[1].1,
        format!(
            "a junction to {} is in the way and stays",
            elsewhere.display()
        )
    );
    assert_eq!(
        std::fs::read(sandbox.addon("DevAddon").join("mine.lua")).expect("kept"),
        b"-- mine\n"
    );
    assert!(link::is_link(&sandbox.addon("OneFolder")));
    assert!(
        sandbox
            .engine
            .load_state()
            .expect("the sidecar")
            .state
            .addons
            .is_empty()
    );

    // An install over a junction mrgl did not make is refused too.
    let zip = zip("one-folder-1.0.0.zip");
    let error = sandbox
        .engine
        .install(&zip, &metadata_for(&zip))
        .expect_err("refused");
    assert!(
        matches!(&error, Error::ForeignJunction { folder, .. } if folder == "OneFolder"),
        "{error}"
    );

    // A dangling junction is reported, not followed.
    std::fs::remove_dir(&elsewhere).expect("the target goes");
    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("runs");
    let one = linked
        .skipped
        .iter()
        .find(|skipped| skipped.name == "OneFolder")
        .expect("skipped");
    assert_eq!(
        one.reason,
        format!(
            "a dangling junction to {} (its target is gone) is in the way and stays",
            elsewhere.display()
        )
    );
    let scan = mrgl_core::scan::scan_flavor(&sandbox.engine.flavor).expect("scans");
    assert!(scan.skipped.iter().any(|name| name == "OneFolder"));
    assert!(
        scan.warnings
            .iter()
            .any(|warning| warning.contains("OneFolder")),
        "{:?}",
        scan.warnings
    );
}

#[cfg(windows)]
#[test]
fn a_junction_made_by_hand_to_the_same_folder_is_taken_over() {
    let sandbox = Sandbox::new("by-hand");
    let dev = link::dev_folder(&sandbox.dev).expect("the folder");
    std::fs::create_dir_all(sandbox.engine.addons_dir()).expect("AddOns");
    link::create_junction(&sandbox.addon("OneFolder"), &dev.join("OneFolder"))
        .expect("a junction by hand");
    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("links");
    let flags: Vec<(&str, bool)> = linked
        .linked
        .iter()
        .map(|addon| (addon.name.as_str(), addon.already_linked))
        .collect();
    assert_eq!(flags, [("DevAddon", false), ("OneFolder", true)]);
    let unlinked = link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");
    assert_eq!(unlinked.unlinked.len(), 2);
    assert!(sandbox.addons_listing().is_empty());
}

#[cfg(windows)]
#[test]
fn a_single_addon_folder_links_and_unlinks_on_its_own() {
    let sandbox = Sandbox::new("single");
    let solo = solo_copy(&sandbox);
    let before = snapshot(&solo);
    assert_eq!(link::shape_of(&solo), link::Shape::AddonFolder);
    assert_eq!(link::shape_of(&sandbox.dev), link::Shape::DevelopmentFolder);

    let linked = link::link(&sandbox.engine, &solo).expect("links");
    let folder = link::dev_folder(&solo).expect("the folder");
    assert_eq!(linked.shape, link::Shape::AddonFolder);
    assert_eq!(linked.folder, folder);
    assert_eq!(linked.source, format!("link:{}", folder.display()));
    let names: Vec<(&str, &str, bool)> = linked
        .linked
        .iter()
        .map(|addon| {
            (
                addon.name.as_str(),
                addon.version.as_str(),
                addon.already_linked,
            )
        })
        .collect();
    assert_eq!(names, [("SoloAddon", "0.4.0-dev", false)]);
    assert!(link::same_path(&linked.linked[0].target, &folder));
    assert!(linked.skipped.is_empty(), "{:?}", linked.skipped);
    assert!(linked.without_toc.is_empty(), "{:?}", linked.without_toc);
    // The addon shape wins: the embedded LibSolo is part of the addon, never linked on its own.
    assert_eq!(sandbox.addons_listing(), ["SoloAddon"]);
    assert!(link::is_link(&sandbox.addon("SoloAddon")));
    assert!(link::same_path(
        &link::link_target(&sandbox.addon("SoloAddon")).expect("a target"),
        &folder
    ));
    assert!(
        sandbox
            .addon("SoloAddon")
            .join("LibSolo/LibSolo.toc")
            .is_file()
    );

    // The scan reads it through the junction, the Forever suffix TOC chosen.
    let scan = mrgl_core::scan::scan_flavor(&sandbox.engine.flavor).expect("scans");
    let solo_scan = scan
        .addons
        .iter()
        .find(|addon| addon.folder == "SoloAddon")
        .expect("SoloAddon in the scan");
    assert!(solo_scan.linked);
    assert_eq!(solo_scan.chosen.as_deref(), Some("SoloAddon_Forever.toc"));
    assert_eq!(solo_scan.version(), Some("0.4.0-dev"));
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    let (_, entry) = state.entry("SoloAddon").expect("an entry");
    assert_eq!(entry.source, linked.source);
    assert_eq!(entry.folders, ["SoloAddon"]);
    assert!(entry.files.is_empty());

    // A second link changes nothing; a development folder still links its subfolders next to it.
    let again = link::link(&sandbox.engine, &solo).expect("links again");
    assert_eq!(again.linked.len(), 1);
    assert!(again.linked[0].already_linked);
    let dev = link::link(&sandbox.engine, &sandbox.dev).expect("links the development folder");
    assert_eq!(dev.shape, link::Shape::DevelopmentFolder);
    let dev_names: Vec<&str> = dev.linked.iter().map(|addon| addon.name.as_str()).collect();
    assert_eq!(dev_names, ["DevAddon", "OneFolder"]);
    assert_eq!(
        sandbox.addons_listing(),
        ["DevAddon", "OneFolder", "SoloAddon"]
    );

    // Unlinking the addon folder removes its one junction and entry; the development folder's stay.
    let unlinked = link::unlink(&sandbox.engine, &solo).expect("unlinks");
    assert_eq!(unlinked.folder, folder);
    assert_eq!(unlinked.unlinked, ["SoloAddon"]);
    assert!(unlinked.kept.is_empty(), "{:?}", unlinked.kept);
    assert_eq!(sandbox.addons_listing(), ["DevAddon", "OneFolder"]);
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    let left: Vec<&str> = state.addons.keys().map(String::as_str).collect();
    assert_eq!(left, ["DevAddon", "OneFolder"]);
    assert_eq!(snapshot(&solo), before, "the addon folder");

    // Linked again and then deleted: the dangling junction to the addon folder is still ours to remove.
    link::link(&sandbox.engine, &solo).expect("links once more");
    std::fs::remove_dir_all(&solo).expect("the addon folder goes");
    let unlinked = link::unlink(&sandbox.engine, &solo).expect("unlinks");
    assert_eq!(unlinked.unlinked, ["SoloAddon"]);
    assert_eq!(sandbox.addons_listing(), ["DevAddon", "OneFolder"]);
    let unlinked = link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");
    assert_eq!(unlinked.unlinked, ["DevAddon", "OneFolder"]);
    assert!(sandbox.addons_listing().is_empty());
}

#[cfg(windows)]
#[test]
fn unlink_leaves_a_real_folder_and_takes_a_dangling_junction_of_its_own() {
    let sandbox = Sandbox::new("unlink-cases");
    link::link(&sandbox.engine, &sandbox.dev).expect("links");

    // DevAddon's junction is replaced by a real folder by hand: unlink leaves it and forgets it.
    std::fs::remove_dir(sandbox.addon("DevAddon")).expect("the junction goes");
    std::fs::create_dir(sandbox.addon("DevAddon")).expect("a real folder");
    std::fs::write(sandbox.addon("DevAddon").join("copy.lua"), "-- a copy\n").expect("a file");
    // The development folder is gone: OneFolder's junction dangles, and is still ours to remove. Its
    // name is still spelled as the link recorded it (canonical parents, even when the temp folder's
    // path carries an 8.3 short name).
    let recorded = link::dev_folder(&sandbox.dev).expect("the folder");
    std::fs::remove_dir_all(&sandbox.dev).expect("the development folder goes");
    assert_eq!(link::dev_folder_lenient(&sandbox.dev), recorded);
    assert_eq!(
        link::dev_folder_lenient(&sandbox.dev.join("deeper")),
        recorded.join("deeper")
    );

    let unlinked = link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");
    assert_eq!(unlinked.unlinked, ["OneFolder"]);
    assert_eq!(unlinked.kept.len(), 1);
    assert_eq!(unlinked.kept[0].name, "DevAddon");
    assert_eq!(
        unlinked.kept[0].reason,
        "DevAddon is a real folder now, not a junction; left in place and no longer managed"
    );
    assert_eq!(sandbox.addons_listing(), ["DevAddon"]);
    assert!(sandbox.addon("DevAddon").join("copy.lua").is_file());
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    assert!(state.addons.is_empty(), "{state:?}");
}

#[cfg(windows)]
#[test]
fn a_folder_without_addons_and_a_missing_folder_are_errors() {
    let sandbox = Sandbox::new("no-addons");
    let error = link::link(&sandbox.engine, &sandbox.dev.join("Media")).expect_err("no addon");
    assert!(matches!(error, Error::NoAddonInDevFolder { .. }), "{error}");
    assert!(
        error
            .to_string()
            .ends_with("Media: no TOC named after it and no subfolder with one"),
        "{error}"
    );
    let error = link::link(&sandbox.engine, &sandbox.root.join("missing")).expect_err("missing");
    assert!(
        error
            .to_string()
            .starts_with("reading the development folder: "),
        "{error}"
    );
    assert!(sandbox.addons_listing().is_empty());
}

#[cfg(windows)]
#[test]
fn a_link_replaced_by_a_real_folder_is_forgotten_on_the_next_link() {
    let sandbox = Sandbox::new("relink");
    link::link(&sandbox.engine, &sandbox.dev).expect("links");
    std::fs::remove_dir(sandbox.addon("DevAddon")).expect("the junction goes");
    std::fs::create_dir(sandbox.addon("DevAddon")).expect("a real folder");
    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("links again");
    assert_eq!(linked.linked.len(), 1);
    assert!(linked.linked[0].already_linked);
    assert_eq!(linked.skipped.len(), 1);
    assert_eq!(
        linked.skipped[0].reason,
        "a folder of that name is in AddOns already and stays (not a junction); its link entry is forgotten"
    );
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    assert!(state.entry("DevAddon").is_none());
    assert!(state.entry("OneFolder").is_some());
}

// --- task 055: the link guard sits in the commit, so every path that stages and commits refuses ---

/// A source without the network: every spec resolves to one pick, served from one fixture zip.
struct OneZip {
    zip: PathBuf,
    version: &'static str,
}

impl mrgl_core::sources::Source for OneZip {
    fn resolve(
        &self,
        spec: &mrgl_core::sources::SourceSpec,
        _target: &mrgl_core::sources::Target,
    ) -> mrgl_core::error::Result<mrgl_core::sources::Pick> {
        Ok(mrgl_core::sources::Pick {
            source: spec.to_string(),
            release: format!("v{}", self.version),
            version: self.version.to_string(),
            published: None,
            prerelease: false,
            asset: "One-forever.zip".to_string(),
            url: "https://github.test/One-forever.zip".to_string(),
            size: None,
            flavor: "forever".to_string(),
            interface: Some(16001),
            guess: false,
            reason: "release.json names forever (interface 16001)".to_string(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: mrgl_core::forever::ForeverBuild::Unknown,
            stale: None,
        })
    }

    fn fetch(&self, _pick: &mrgl_core::sources::Pick, to: &Path) -> mrgl_core::error::Result<()> {
        std::fs::copy(&self.zip, to).expect("the download");
        Ok(())
    }
}

/// An import of one `github:` entry the flavor does not have, so it reaches the install path.
#[cfg(windows)]
fn import_one(sandbox: &Sandbox, source: &OneZip) -> mrgl_core::list_file::Imported {
    use mrgl_core::list_file::{self, FORMAT, ListEntry, ListFile, SCHEMA};
    let list = ListFile {
        format: FORMAT.to_string(),
        schema: SCHEMA,
        flavor: "wowf".to_string(),
        exported_at: None,
        addons: vec![ListEntry {
            name: "Fresh".to_string(),
            source: "github:Example/Fresh".to_string(),
            version: source.version.to_string(),
            folders: Vec::new(),
        }],
    };
    let report = list_file::import(&sandbox.engine, source, &list, false, false).expect("imports");
    report.items.into_iter().next().expect("the entry")
}

#[cfg(windows)]
#[test]
fn every_install_path_refuses_a_linked_addon_and_a_foreign_junction() {
    use mrgl_core::list_file::Outcome;
    let sandbox = Sandbox::new("every-path");
    let before = snapshot(&sandbox.dev);
    let source = OneZip {
        zip: zip("one-folder-1.2.0.zip"),
        version: "1.2.0",
    };
    let spec = mrgl_core::sources::SourceSpec::parse("github:Example/One").expect("a spec");
    let from_source = |sandbox: &Sandbox| {
        sandbox.engine.install_from_source(
            &source,
            &mrgl_core::install::SourceRequest::current(spec.clone()),
            &mut |_| {},
        )
    };

    // A linked addon: the source path and the import refuse it like the zip path does.
    link::link(&sandbox.engine, &sandbox.dev).expect("links");
    let error = from_source(&sandbox).expect_err("refused");
    assert!(
        matches!(&error, Error::LinkedAddon { action: "install over", name, .. } if name == "OneFolder"),
        "{error}"
    );
    let imported = import_one(&sandbox, &source);
    assert_eq!(imported.outcome, Outcome::Failed);
    assert!(
        imported
            .detail
            .starts_with("refused to install over OneFolder (flavor forever): it is linked from "),
        "{}",
        imported.detail
    );
    assert!(link::is_link(&sandbox.addon("OneFolder")));
    link::unlink(&sandbox.engine, &sandbox.dev).expect("unlinks");

    // A junction mrgl did not make over a managed addon with a previous version: the zip, the source,
    // the import and the rollback all refuse it, and nothing moves.
    for name in ["one-folder-1.0.0.zip", "one-folder-1.1.0.zip"] {
        sandbox
            .engine
            .install(&zip(name), &metadata_for(&zip(name)))
            .expect("installs");
    }
    std::fs::remove_dir_all(sandbox.addon("OneFolder")).expect("the folder goes");
    let dev = link::dev_folder(&sandbox.dev).expect("the folder");
    link::create_junction(&sandbox.addon("OneFolder"), &dev.join("OneFolder"))
        .expect("a junction by hand");
    let foreign = |error: &Error| matches!(error, Error::ForeignJunction { folder, .. } if folder == "OneFolder");
    let zip_path = zip("one-folder-1.2.0.zip");
    let error = sandbox
        .engine
        .install(&zip_path, &metadata_for(&zip_path))
        .expect_err("refused");
    assert!(foreign(&error), "{error}");
    let error = from_source(&sandbox).expect_err("refused");
    assert!(foreign(&error), "{error}");
    let imported = import_one(&sandbox, &source);
    assert_eq!(imported.outcome, Outcome::Failed);
    assert!(
        imported
            .detail
            .starts_with("refused to install over OneFolder (flavor forever): it is a junction"),
        "{}",
        imported.detail
    );
    let error = sandbox.engine.rollback("OneFolder").expect_err("refused");
    assert!(foreign(&error), "{error}");

    assert!(link::is_link(&sandbox.addon("OneFolder")));
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    let (_, entry) = state.entry("OneFolder").expect("still managed");
    assert_eq!(entry.version, "1.1.0", "the rollback changed nothing");
    assert!(entry.pinned.is_none());
    assert!(state.entry("Fresh").is_none());
    assert_eq!(snapshot(&sandbox.dev), before, "the development folder");
}

// --- task 006: the update goes through the same commit, so it refuses a foreign junction too ---

#[cfg(windows)]
#[test]
fn an_update_over_a_foreign_junction_is_refused() {
    use mrgl_core::update::{self, UpdateState};
    let sandbox = Sandbox::new("update-path");
    let one = zip("one-folder-1.1.0.zip");
    sandbox
        .engine
        .install(
            &one,
            &Metadata {
                source: "github:Example/One".to_string(),
                version: Some("1.1.0".to_string()),
                name: None,
                nolib: None,
            },
        )
        .expect("installs");
    std::fs::remove_dir_all(sandbox.addon("OneFolder")).expect("the folder goes");
    let dev = link::dev_folder(&sandbox.dev).expect("the folder");
    link::create_junction(&sandbox.addon("OneFolder"), &dev.join("OneFolder"))
        .expect("a junction by hand");
    let source = OneZip {
        zip: zip("one-folder-1.2.0.zip"),
        version: "1.2.0",
    };
    let check = update::check(&sandbox.engine, &source, None, &mut |_| {}).expect("checks");
    let addon = &check.addons[0];
    assert_eq!(addon.state, UpdateState::UpdateAvailable);
    let request = addon.request().expect("an update");
    let error = sandbox
        .engine
        .install_from_source(&source, &request, &mut |_| {})
        .expect_err("refused");
    assert!(
        matches!(&error, Error::ForeignJunction { folder, .. } if folder == "OneFolder"),
        "{error}"
    );
    assert!(link::is_link(&sandbox.addon("OneFolder")));
    let state = sandbox.engine.load_state().expect("the sidecar").state;
    assert_eq!(state.addons["OneFolder"].version, "1.1.0");
}

// --- task 067: the guard covers the folders a new version drops ---

#[cfg(windows)]
#[test]
fn an_update_that_drops_a_linked_folder_is_refused() {
    let sandbox = Sandbox::new("dropped-folder");
    // The development folder holds the folder that version 2.1.0 drops.
    let data = sandbox.dev.join("SeveralFolders_Data");
    std::fs::create_dir_all(&data).expect("the folder");
    std::fs::write(
        data.join("SeveralFolders_Data.toc"),
        "## Interface: 11508\n## Title: Data in development\n## Version: 9.9.9\n\ncore.lua\n",
    )
    .expect("a TOC");
    std::fs::write(data.join("core.lua"), "-- work in progress\n").expect("a file");
    let older = zip("several-folders-2.0.0.zip");
    let newer = zip("several-folders-2.1.0.zip");
    sandbox
        .engine
        .install(&older, &metadata_for(&older))
        .expect("installs");
    let dev = link::dev_folder(&sandbox.dev).expect("the folder");
    let refused_update = |sandbox: &Sandbox| {
        let before = snapshot(&sandbox.dev);
        let listing = sandbox.addons_listing();
        let error = sandbox
            .engine
            .install(&newer, &metadata_for(&newer))
            .expect_err("refused");
        assert!(link::is_link(&sandbox.addon("SeveralFolders_Data")));
        assert_eq!(snapshot(&sandbox.dev), before, "the development folder");
        assert_eq!(sandbox.addons_listing(), listing, "the AddOns folder");
        let state = sandbox.engine.load_state().expect("the sidecar").state;
        assert_eq!(state.addons["SeveralFolders"].version, "2.0.0");
        assert!(
            snapshot(&sandbox.addon("SeveralFolders")).contains_key("SeveralFolders.toc"),
            "the previous version stays"
        );
        error
    };

    // A junction made by hand where the dropped folder was: not moved aside, not deleted.
    std::fs::remove_dir_all(sandbox.addon("SeveralFolders_Data")).expect("the folder goes");
    link::create_junction(
        &sandbox.addon("SeveralFolders_Data"),
        &dev.join("SeveralFolders_Data"),
    )
    .expect("a junction by hand");
    let error = refused_update(&sandbox);
    assert!(
        matches!(&error, Error::ForeignJunction { folder, .. } if folder == "SeveralFolders_Data"),
        "{error}"
    );

    // The same folder linked by `mrgl link`, with a sidecar that still lists it under the managed addon
    // (`link` skips a folder another entry lists, so only a sidecar written otherwise holds both).
    std::fs::remove_dir(sandbox.addon("SeveralFolders_Data")).expect("the junction goes");
    let mut state = sandbox.engine.load_state().expect("the sidecar").state;
    let folders = &mut state
        .addons
        .get_mut("SeveralFolders")
        .expect("managed")
        .folders;
    folders.retain(|folder| folder != "SeveralFolders_Data");
    sandbox.engine.home.write_state(&state).expect("writes");
    let linked = link::link(&sandbox.engine, &sandbox.dev).expect("links");
    assert!(
        linked
            .linked
            .iter()
            .any(|addon| addon.name == "SeveralFolders_Data"),
        "{linked:?}"
    );
    let mut state = sandbox.engine.load_state().expect("the sidecar").state;
    state
        .addons
        .get_mut("SeveralFolders")
        .expect("managed")
        .folders
        .push("SeveralFolders_Data".to_string());
    sandbox.engine.home.write_state(&state).expect("writes");
    let error = refused_update(&sandbox);
    assert!(
        matches!(&error, Error::LinkedAddon { action: "install over", name, .. } if name == "SeveralFolders_Data"),
        "{error}"
    );
}
