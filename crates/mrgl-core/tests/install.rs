//! The install engine over the fixture zips under `tests/fixtures/zips/`, in a temporary game folder with
//! a temporary mrgl home: install, update (a second version over the first), the folders a version drops,
//! the escaping path, the failed swap that leaves the folder as it was, remove, the sidecar round trip, the
//! running client as a warning (task 038, through a stub process list), the install of a GitHub pick
//! (task 040) through the folder stub over `tests/fixtures/releases/`, the marks of task 021 and the
//! rollback from the kept zip (task 022). No real game folder and no network anywhere.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mrgl_core::Error;
use mrgl_core::client::{FixedProcesses, ProcessList, RunningProcess};
use mrgl_core::flavor::{self, FlavorId};
use mrgl_core::install::{Engine, Metadata, PickInstalled, PickStep};
use mrgl_core::sources::Channel;
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::{FolderHttp, Source, SourceSpec, Target};
use mrgl_core::state::{self, STATE_FILE, State};

fn zips() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zips")
}

fn zip(name: &str) -> PathBuf {
    zips().join(name)
}

/// A temporary game folder with one `_forever_` flavor and a temporary mrgl home next to it.
struct Sandbox {
    root: PathBuf,
    engine: Engine,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root =
            std::env::temp_dir().join(format!("mrgl-install-test-{}-{name}", std::process::id()));
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
        let flavor = flavor::read_flavor(&flavor_dir).expect("a flavor");
        let engine = Engine::with_home(flavor, &root.join("home"));
        Sandbox { root, engine }
    }

    fn addons(&self) -> PathBuf {
        self.engine.addons_dir()
    }

    fn addon(&self, folder: &str) -> PathBuf {
        self.addons().join(folder)
    }

    fn install(&self, zip_name: &str) -> mrgl_core::install::Installed {
        self.engine
            .install(&zip(zip_name), &metadata(zip_name))
            .unwrap_or_else(|error| panic!("{zip_name} installs: {error}"))
    }

    fn state(&self) -> State {
        let loaded = self.engine.load_state().expect("the state reads");
        assert_eq!(loaded.warning, None);
        loaded.state
    }

    fn staging_entries(&self) -> Vec<String> {
        match std::fs::read_dir(self.engine.home.staging_dir()) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    fn cache_entries(&self, name: &str) -> Vec<String> {
        let mut names: Vec<String> = match std::fs::read_dir(self.engine.home.addon_cache_dir(name))
        {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        };
        names.sort();
        names
    }

    fn folders(&self) -> Vec<String> {
        let mut names: Vec<String> = match std::fs::read_dir(self.addons()) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(_) => Vec::new(),
        };
        names.sort();
        names
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn metadata(zip_name: &str) -> Metadata {
    Metadata {
        source: zip(zip_name).display().to_string(),
        version: None,
        name: None,
        nolib: None,
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn one_folder_installs_with_its_files_hashed_and_its_zip_cached() {
    let sandbox = Sandbox::new("one-folder");
    let installed = sandbox.install("one-folder-1.0.0.zip");
    assert_eq!(installed.name, "OneFolder");
    assert_eq!(installed.version, "1.0.0");
    assert_eq!(installed.previous_version, None);
    assert_eq!(installed.folders, ["OneFolder"]);
    assert!(installed.removed_folders.is_empty());
    assert!(installed.warnings.is_empty(), "{:?}", installed.warnings);

    assert_eq!(sandbox.folders(), ["OneFolder"]);
    assert_eq!(
        read(&sandbox.addon("OneFolder/core.lua")),
        "-- OneFolder 1.0.0\n"
    );
    assert!(sandbox.addon("OneFolder/locale/enUS.lua").is_file());
    assert!(sandbox.addon("OneFolder/old.lua").is_file());

    let state = sandbox.state();
    assert_eq!(state.flavor, "wowf");
    let (name, entry) = state.entry("onefolder").expect("the entry");
    assert_eq!(name, "OneFolder");
    assert_eq!(
        entry.source,
        zip("one-folder-1.0.0.zip").display().to_string()
    );
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.folders, ["OneFolder"]);
    let files: Vec<&str> = entry.files.keys().map(String::as_str).collect();
    assert_eq!(
        files,
        [
            "OneFolder/OneFolder.toc",
            "OneFolder/core.lua",
            "OneFolder/locale/enUS.lua",
            "OneFolder/old.lua",
        ]
    );
    let on_disk =
        mrgl_core::install::sha256_of(&sandbox.addon("OneFolder/core.lua")).expect("hashes");
    assert_eq!(entry.files["OneFolder/core.lua"], on_disk);
    assert!(entry.files.values().all(|hash| hash.len() == 64));
    assert_eq!(entry.installed_at.len(), 20, "{}", entry.installed_at);
    assert!(entry.installed_at.ends_with('Z'));

    assert_eq!(sandbox.cache_entries("OneFolder"), ["1.0.0.zip"]);
    assert_eq!(
        std::fs::read(sandbox.engine.home.cached_zip("OneFolder", "1.0.0"))
            .expect("the cached zip"),
        std::fs::read(zip("one-folder-1.0.0.zip")).expect("the fixture")
    );
    assert!(
        sandbox.staging_entries().is_empty(),
        "{:?}",
        sandbox.staging_entries()
    );
}

#[test]
fn a_second_version_replaces_the_first_and_keeps_one_previous_zip() {
    let sandbox = Sandbox::new("update");
    sandbox.install("one-folder-1.0.0.zip");
    let updated = sandbox.install("one-folder-1.1.0.zip");
    assert_eq!(updated.version, "1.1.0");
    assert_eq!(updated.previous_version.as_deref(), Some("1.0.0"));
    assert_eq!(
        read(&sandbox.addon("OneFolder/core.lua")),
        "-- OneFolder 1.1.0\n"
    );
    assert!(sandbox.addon("OneFolder/new.lua").is_file());
    assert!(
        !sandbox.addon("OneFolder/old.lua").exists(),
        "the old file is gone with the old folder"
    );
    let state = sandbox.state();
    let (_, entry) = state.entry("OneFolder").expect("the entry");
    assert_eq!(entry.version, "1.1.0");
    assert_eq!(entry.previous_version.as_deref(), Some("1.0.0"));
    assert!(!entry.files.contains_key("OneFolder/old.lua"));
    assert!(entry.files.contains_key("OneFolder/new.lua"));
    assert_eq!(
        sandbox.cache_entries("OneFolder"),
        ["1.0.0.zip", "1.1.0.zip"]
    );

    let third = sandbox.install("one-folder-1.2.0.zip");
    assert_eq!(third.previous_version.as_deref(), Some("1.1.0"));
    assert_eq!(
        sandbox.cache_entries("OneFolder"),
        ["1.1.0.zip", "1.2.0.zip"],
        "one previous version per addon"
    );
    assert!(sandbox.staging_entries().is_empty());
}

#[test]
fn reinstalling_the_same_version_keeps_the_previous_one() {
    let sandbox = Sandbox::new("same-version");
    sandbox.install("one-folder-1.0.0.zip");
    sandbox.install("one-folder-1.1.0.zip");
    let again = sandbox.install("one-folder-1.1.0.zip");
    assert_eq!(again.previous_version.as_deref(), Some("1.0.0"));
    assert_eq!(
        sandbox.cache_entries("OneFolder"),
        ["1.0.0.zip", "1.1.0.zip"]
    );
}

#[test]
fn several_folders_install_as_one_addon_and_a_later_version_drops_a_folder() {
    let sandbox = Sandbox::new("several");
    let installed = sandbox.install("several-folders-2.0.0.zip");
    assert_eq!(installed.name, "SeveralFolders");
    assert_eq!(installed.version, "2.0.0");
    assert_eq!(
        installed.folders,
        [
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ]
    );
    assert_eq!(
        sandbox.folders(),
        [
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ],
        "README.md and Docs/ stay out of AddOns"
    );
    let state = sandbox.state();
    assert_eq!(state.addons.len(), 1);
    assert_eq!(
        state
            .entry_for_folder("severalfolders_options")
            .map(|(name, _)| name),
        Some("SeveralFolders")
    );

    let updated = sandbox.install("several-folders-2.1.0.zip");
    assert_eq!(updated.version, "2.1.0");
    assert_eq!(
        updated.folders,
        ["SeveralFolders", "SeveralFolders_Options"]
    );
    assert_eq!(updated.removed_folders, ["SeveralFolders_Data"]);
    assert_eq!(
        sandbox.folders(),
        ["SeveralFolders", "SeveralFolders_Options"]
    );
    assert_eq!(
        read(&sandbox.addon("SeveralFolders_Options/core.lua")),
        "-- options 2.1.0\n"
    );
    let (_, entry) = sandbox
        .state()
        .entry("SeveralFolders")
        .map(|(n, e)| (n.to_string(), e.clone()))
        .expect("the entry");
    assert_eq!(entry.folders, ["SeveralFolders", "SeveralFolders_Options"]);
    assert!(
        entry
            .files
            .keys()
            .all(|file| !file.starts_with("SeveralFolders_Data/"))
    );
}

#[test]
fn a_toc_at_the_root_is_one_addon_named_after_it() {
    let sandbox = Sandbox::new("root-toc");
    let installed = sandbox.install("root-toc-3.0.0.zip");
    assert_eq!(installed.name, "RootToc");
    assert_eq!(installed.version, "3.0.0");
    assert_eq!(installed.folders, ["RootToc"]);
    assert!(sandbox.addon("RootToc/RootToc.toc").is_file());
    assert!(sandbox.addon("RootToc/RootToc_Vanilla.toc").is_file());
    assert!(sandbox.addon("RootToc/core.lua").is_file());
    let files: Vec<String> = sandbox
        .state()
        .entry("RootToc")
        .expect("the entry")
        .1
        .files
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        files,
        [
            "RootToc/RootToc.toc",
            "RootToc/RootToc_Vanilla.toc",
            "RootToc/core.lua"
        ]
    );
}

#[test]
fn a_wrapper_folder_is_looked_into_one_level_down() {
    let sandbox = Sandbox::new("wrapped");
    let installed = sandbox.install("wrapped-4.0.0.zip");
    assert_eq!(installed.name, "Wrapped");
    assert_eq!(installed.version, "4.0.0");
    assert_eq!(sandbox.folders(), ["Wrapped"]);
    assert!(sandbox.addon("Wrapped/Wrapped.toc").is_file());
    let files: Vec<String> = sandbox
        .state()
        .entry("Wrapped")
        .expect("the entry")
        .1
        .files
        .keys()
        .cloned()
        .collect();
    assert_eq!(files, ["Wrapped/Wrapped.toc", "Wrapped/core.lua"]);
}

#[test]
fn the_metadata_names_the_source_the_version_and_the_name() {
    let sandbox = Sandbox::new("metadata");
    let given = Metadata {
        source: "github:someone/one-folder".to_string(),
        version: Some("v1.0.0-release".to_string()),
        name: Some("One".to_string()),
        nolib: Some(false),
    };
    let installed = sandbox
        .engine
        .install(&zip("one-folder-1.0.0.zip"), &given)
        .expect("installs");
    assert_eq!(installed.name, "One");
    assert_eq!(installed.version, "v1.0.0-release");
    assert_eq!(installed.folders, ["OneFolder"]);
    let state = sandbox.state();
    let (_, entry) = state.entry("One").expect("the entry");
    assert_eq!(entry.source, "github:someone/one-folder");
    assert_eq!(entry.nolib, Some(false), "the pick's nolib flag (task 029)");
    assert_eq!(sandbox.cache_entries("One"), ["v1.0.0-release.zip"]);

    let no_version = sandbox.install("no-version.zip");
    assert_eq!(no_version.version, "unknown");
    let state = sandbox.state();
    let (_, entry) = state.entry("NoVersion").expect("the entry");
    assert_eq!(entry.nolib, None, "a zip in hand does not say");
    assert_eq!(sandbox.cache_entries("NoVersion"), ["unknown.zip"]);
}

#[test]
fn an_escaping_entry_refuses_the_zip_and_writes_nothing() {
    let sandbox = Sandbox::new("escaping");
    let error = sandbox
        .engine
        .install(&zip("escaping-path.zip"), &metadata("escaping-path.zip"))
        .expect_err("refused");
    match &error {
        Error::ZipEntryEscapes { entry, .. } => assert_eq!(entry, "Escape/../../Outside.lua"),
        other => panic!("{other}"),
    }
    assert!(error.to_string().starts_with("refused "), "{error}");
    assert!(sandbox.folders().is_empty());
    assert!(
        !sandbox
            .root
            .join("home")
            .join("wowf")
            .join("Outside.lua")
            .exists()
    );
    assert!(!sandbox.root.join("home").join("Outside.lua").exists());
    assert!(
        sandbox.staging_entries().is_empty(),
        "the staging folder is cleaned"
    );
    assert!(
        !sandbox.engine.home.state_path().exists(),
        "nothing was recorded"
    );
}

#[test]
fn a_zip_without_an_addon_and_a_file_that_is_no_zip_are_errors() {
    let sandbox = Sandbox::new("no-addon");
    let error = sandbox
        .engine
        .install(&zip("no-addon.zip"), &metadata("no-addon.zip"))
        .expect_err("refused");
    assert!(matches!(error, Error::NoAddonInZip { .. }), "{error}");
    assert!(error.to_string().starts_with("no addon in "), "{error}");

    let not_a_zip = sandbox.root.join("not-a-zip.zip");
    std::fs::write(&not_a_zip, "plain text").expect("write");
    let error = sandbox
        .engine
        .install(&not_a_zip, &Metadata::default())
        .expect_err("refused");
    assert!(matches!(error, Error::ZipUnreadable { .. }), "{error}");
    assert!(
        error.to_string().starts_with("not a readable zip: "),
        "{error}"
    );

    let error = sandbox
        .engine
        .install(&sandbox.root.join("missing.zip"), &Metadata::default())
        .expect_err("refused");
    assert!(matches!(error, Error::ZipUnreadable { .. }), "{error}");
    assert!(sandbox.staging_entries().is_empty());
}

#[test]
fn a_failed_swap_puts_every_folder_back_as_it_was() {
    let sandbox = Sandbox::new("failed-swap");
    sandbox.install("several-folders-2.0.0.zip");
    std::fs::write(
        sandbox.addon("SeveralFolders/edited.lua"),
        "-- the user's edit\n",
    )
    .expect("edit");
    let state_before = read(&sandbox.engine.home.state_path());

    let mut state = sandbox.state();
    let staged = sandbox
        .engine
        .stage(
            &zip("several-folders-2.1.0.zip"),
            &metadata("several-folders-2.1.0.zip"),
        )
        .expect("stages");
    assert_eq!(
        staged.folders(),
        ["SeveralFolders", "SeveralFolders_Options"]
    );
    // The second folder vanishes from the staging before the swap, so its move fails after the first
    // folder was swapped already.
    std::fs::remove_dir_all(&staged.addons[1].path).expect("sabotage");
    let error = sandbox
        .engine
        .commit(staged, &mut state)
        .expect_err("the swap fails");
    match &error {
        Error::SwapFailed { folder, .. } => assert_eq!(folder, "SeveralFolders_Options"),
        other => panic!("{other}"),
    }
    assert!(
        error.to_string().starts_with(
            "installing SeveralFolders_Options failed and the AddOns folder is as it was: "
        ),
        "{error}"
    );

    assert_eq!(
        sandbox.folders(),
        [
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ]
    );
    assert_eq!(
        read(&sandbox.addon("SeveralFolders/core.lua")),
        "-- SeveralFolders 2.0.0\n"
    );
    assert_eq!(
        read(&sandbox.addon("SeveralFolders/edited.lua")),
        "-- the user's edit\n"
    );
    assert_eq!(
        read(&sandbox.addon("SeveralFolders_Options/core.lua")),
        "-- options 2.0.0\n"
    );
    assert_eq!(
        read(&sandbox.engine.home.state_path()),
        state_before,
        "the sidecar is untouched"
    );
    assert!(
        sandbox.staging_entries().is_empty(),
        "{:?}",
        sandbox.staging_entries()
    );
}

#[test]
fn a_failed_first_swap_leaves_a_fresh_folder_absent() {
    let sandbox = Sandbox::new("failed-first");
    let mut state = sandbox.state();
    let staged = sandbox
        .engine
        .stage(
            &zip("one-folder-1.0.0.zip"),
            &metadata("one-folder-1.0.0.zip"),
        )
        .expect("stages");
    std::fs::remove_dir_all(&staged.addons[0].path).expect("sabotage");
    let error = sandbox
        .engine
        .commit(staged, &mut state)
        .expect_err("the swap fails");
    assert!(matches!(error, Error::SwapFailed { .. }), "{error}");
    assert!(sandbox.folders().is_empty());
    assert!(!sandbox.engine.home.state_path().exists());
}

#[cfg(windows)]
#[test]
fn a_locked_file_in_the_old_folder_fails_the_swap_and_changes_nothing() {
    use std::os::windows::fs::OpenOptionsExt;
    let sandbox = Sandbox::new("locked");
    sandbox.install("one-folder-1.0.0.zip");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(sandbox.addon("OneFolder/core.lua"))
        .expect("a handle with no sharing");
    let error = sandbox
        .engine
        .install(
            &zip("one-folder-1.1.0.zip"),
            &metadata("one-folder-1.1.0.zip"),
        )
        .expect_err("the folder cannot move while a file in it is open");
    assert!(matches!(error, Error::SwapFailed { .. }), "{error}");
    drop(lock);
    assert_eq!(
        read(&sandbox.addon("OneFolder/core.lua")),
        "-- OneFolder 1.0.0\n"
    );
    assert!(sandbox.addon("OneFolder/old.lua").is_file());
    assert_eq!(
        sandbox
            .state()
            .entry("OneFolder")
            .expect("the entry")
            .1
            .version,
        "1.0.0"
    );
    assert!(sandbox.staging_entries().is_empty());
}

#[test]
fn remove_deletes_the_folders_the_entry_and_the_cache() {
    let sandbox = Sandbox::new("remove");
    sandbox.install("several-folders-2.0.0.zip");
    sandbox.install("one-folder-1.0.0.zip");
    let removed = sandbox.engine.remove("severalfolders").expect("removes");
    assert_eq!(removed.name, "SeveralFolders");
    assert_eq!(removed.version, "2.0.0");
    assert_eq!(
        removed.folders,
        [
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ]
    );
    assert!(removed.warnings.is_empty(), "{:?}", removed.warnings);
    assert_eq!(sandbox.folders(), ["OneFolder"]);
    let state = sandbox.state();
    assert!(state.entry("SeveralFolders").is_none());
    assert!(state.entry("OneFolder").is_some());
    assert!(sandbox.cache_entries("SeveralFolders").is_empty());
    assert!(
        !sandbox
            .engine
            .home
            .addon_cache_dir("SeveralFolders")
            .exists()
    );

    let error = sandbox.engine.remove("SeveralFolders").expect_err("gone");
    assert_eq!(
        error.to_string(),
        "addon not found: SeveralFolders (flavor forever)"
    );

    std::fs::create_dir_all(sandbox.addon("ByHand")).expect("a folder by hand");
    std::fs::write(sandbox.addon("ByHand/ByHand.toc"), "## Title: By Hand\n").expect("toc");
    let error = sandbox.engine.remove("ByHand").expect_err("not managed");
    assert!(matches!(error, Error::NotManaged { .. }), "{error}");
    assert!(
        error
            .to_string()
            .starts_with("addon not managed by mrgl: ByHand (flavor forever); "),
        "{error}"
    );
    assert!(
        sandbox.addon("ByHand").is_dir(),
        "a folder by hand is never deleted"
    );
}

#[test]
fn a_folder_deleted_by_hand_leaves_the_sidecar_on_the_next_install() {
    let sandbox = Sandbox::new("by-hand");
    sandbox.install("one-folder-1.0.0.zip");
    std::fs::remove_dir_all(sandbox.addon("OneFolder")).expect("deleted by hand");
    let installed = sandbox.install("root-toc-3.0.0.zip");
    assert_eq!(
        installed.warnings,
        ["OneFolder was deleted by hand since its install and is no longer managed"]
    );
    let state = sandbox.state();
    assert!(state.entry("OneFolder").is_none());
    assert!(state.entry("RootToc").is_some());
}

#[test]
fn a_corrupt_sidecar_is_rebuilt_with_a_warning_and_the_install_goes_on() {
    let sandbox = Sandbox::new("corrupt");
    sandbox.install("one-folder-1.0.0.zip");
    std::fs::write(sandbox.engine.home.state_path(), "{ not json").expect("corrupt it");
    let installed = sandbox.install("root-toc-3.0.0.zip");
    assert_eq!(installed.warnings.len(), 1, "{:?}", installed.warnings);
    assert!(
        installed.warnings[0].starts_with("unreadable state sidecar, starting over"),
        "{}",
        installed.warnings[0]
    );
    let state = sandbox.state();
    assert!(state.entry("RootToc").is_some());
    assert!(
        state.entry("OneFolder").is_none(),
        "the rebuilt state manages only what mrgl installed since"
    );
    assert!(
        sandbox.addon("OneFolder").is_dir(),
        "the folder itself is untouched"
    );
    let scan = mrgl_core::scan::scan_flavor(&sandbox.engine.flavor).expect("scans");
    let statuses = state.classify(&scan);
    let unmanaged: Vec<&str> = statuses
        .iter()
        .filter(|status| !status.is_managed())
        .map(|status| status.folder)
        .collect();
    assert_eq!(unmanaged, ["OneFolder"]);
}

#[test]
fn a_folder_taken_over_from_another_managed_addon_is_a_warning() {
    let sandbox = Sandbox::new("take-over");
    sandbox.install("several-folders-2.0.0.zip");
    let given = Metadata {
        source: "zip in hand".to_string(),
        version: Some("9".to_string()),
        name: Some("Options".to_string()),
        nolib: None,
    };
    // A zip that ships only the Options folder under another name takes that folder over.
    let only_options = sandbox.root.join("only-options.zip");
    let mut entries = BTreeMap::new();
    entries.insert(
        "SeveralFolders_Options/SeveralFolders_Options.toc",
        "## Version: 9\n",
    );
    entries.insert("SeveralFolders_Options/core.lua", "-- 9\n");
    write_zip(&only_options, &entries);
    let installed = sandbox
        .engine
        .install(&only_options, &given)
        .expect("installs");
    assert_eq!(
        installed.warnings,
        ["SeveralFolders lost 1 folder to Options"]
    );
    let state = sandbox.state();
    assert_eq!(
        state
            .entry("SeveralFolders")
            .expect("still there")
            .1
            .folders,
        ["SeveralFolders", "SeveralFolders_Data"]
    );
    assert_eq!(
        state.entry("Options").expect("new").1.folders,
        ["SeveralFolders_Options"]
    );
    assert_eq!(
        read(&sandbox.addon("SeveralFolders_Options/core.lua")),
        "-- 9\n"
    );
}

#[test]
fn the_sidecar_round_trips_through_the_engine_and_the_state_module() {
    let sandbox = Sandbox::new("round-trip");
    sandbox.install("several-folders-2.0.0.zip");
    let path = sandbox.engine.home.state_path();
    assert_eq!(
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .as_deref(),
        Some(STATE_FILE)
    );
    assert_eq!(
        path.parent().and_then(Path::parent),
        Some(sandbox.root.join("home").as_path())
    );
    let loaded = state::read_state_at(&path, &FlavorId::Forever).expect("reads");
    assert_eq!(loaded.warning, None);
    let copy = sandbox.root.join("copy.json");
    state::write_state_at(&copy, &loaded.state).expect("writes");
    assert_eq!(read(&copy), read(&path));
    let value: serde_json::Value = serde_json::from_str(&read(&path)).expect("json");
    assert_eq!(value["schema"], 1);
    assert_eq!(value["flavor"], "wowf");
    assert_eq!(value["addons"]["SeveralFolders"]["version"], "2.0.0");
    assert_eq!(
        value["addons"]["SeveralFolders"]["folders"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert!(value["addons"]["SeveralFolders"]["files"]["SeveralFolders/core.lua"].is_string());
}

// --- task 038: a running client is a warning on the install and the removal, never a refusal ---

/// The stand-in client of task 023 as the engine's process list sees it: a process whose executable
/// lives in the flavor folder, next to an unrelated one; no real process is started.
fn stand_in_running(sandbox: &Sandbox) -> FixedProcesses {
    FixedProcesses::new([
        (4, PathBuf::from("Q:\\Windows\\System32\\svchost.exe")),
        (
            4242,
            sandbox.engine.flavor.folder.join("client-stand-in.exe"),
        ),
    ])
}

/// A process list that cannot be taken at all.
struct BrokenProcesses;

impl ProcessList for BrokenProcesses {
    fn executables(&self) -> std::io::Result<Vec<RunningProcess>> {
        Err(std::io::Error::other("no snapshot"))
    }
}

#[test]
fn an_install_and_a_removal_with_the_client_running_warn_and_still_write() {
    let mut sandbox = Sandbox::new("client-running");
    let processes = stand_in_running(&sandbox);
    sandbox.engine = sandbox.engine.clone().with_processes(processes);

    let installed = sandbox.install("one-folder-1.0.0.zip");
    assert_eq!(
        installed.client_warning.as_deref(),
        Some(
            "the client is running in _forever_ (client-stand-in.exe, pid 4242): the installed addons load at /reload or the next login"
        )
    );
    assert!(installed.warnings.is_empty(), "{:?}", installed.warnings);
    assert_eq!(
        sandbox.folders(),
        ["OneFolder"],
        "the install still happens"
    );
    assert!(sandbox.state().entry("OneFolder").is_some());

    let updated = sandbox.install("one-folder-1.1.0.zip");
    assert!(updated.client_warning.is_some());
    assert!(sandbox.addon("OneFolder/new.lua").is_file());

    let removed = sandbox.engine.remove("OneFolder").expect("removes");
    assert_eq!(
        removed.client_warning.as_deref(),
        Some(
            "the client is running in _forever_ (client-stand-in.exe, pid 4242): the removed addons stay loaded until /reload or the next login"
        )
    );
    assert!(removed.warnings.is_empty(), "{:?}", removed.warnings);
    assert!(sandbox.folders().is_empty(), "the removal still happens");
}

#[test]
fn without_the_client_nothing_is_reported() {
    let mut sandbox = Sandbox::new("client-not-running");
    // A client of another flavor and one in a sibling folder with the same prefix are not this flavor's.
    let game = sandbox
        .engine
        .flavor
        .folder
        .parent()
        .expect("the game folder");
    let processes = FixedProcesses::new([
        (7, game.join("_retail_").join("Wow.exe")),
        (8, game.join("_forever_2").join("Wow.exe")),
    ]);
    sandbox.engine = sandbox.engine.clone().with_processes(processes);
    let installed = sandbox.install("one-folder-1.0.0.zip");
    assert_eq!(installed.client_warning, None);
    let removed = sandbox.engine.remove("OneFolder").expect("removes");
    assert_eq!(removed.client_warning, None);

    // The default engine asks the system's list, where no process runs from the temporary folder.
    let sandbox = Sandbox::new("client-system-list");
    let installed = sandbox.install("one-folder-1.0.0.zip");
    assert_eq!(installed.client_warning, None);
}

#[test]
fn a_process_list_that_cannot_be_taken_is_a_warning_and_the_install_goes_on() {
    let mut sandbox = Sandbox::new("client-unknown");
    sandbox.engine = sandbox.engine.clone().with_processes(BrokenProcesses);
    let installed = sandbox.install("one-folder-1.0.0.zip");
    assert_eq!(
        installed.client_warning.as_deref(),
        Some("cannot list the running processes: no snapshot")
    );
    assert_eq!(sandbox.folders(), ["OneFolder"]);
    let removed = sandbox.engine.remove("OneFolder").expect("removes");
    assert!(removed.client_warning.is_some());
    assert!(sandbox.folders().is_empty());
}

// --- task 021: pin, ignore and the channel, through the engine ---

#[test]
fn pin_ignore_and_the_channel_round_trip_through_the_sidecar() {
    let sandbox = Sandbox::new("marks");
    sandbox.install("one-folder-1.0.0.zip");
    sandbox.install("several-folders-2.0.0.zip");
    let before = std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar");

    let pinned = sandbox.engine.pin("onefolder", None).expect("pins");
    assert_eq!(pinned.name, "OneFolder");
    assert_eq!(
        pinned.pinned.as_deref(),
        Some("1.0.0"),
        "the installed version"
    );
    assert!(pinned.changed);
    let again = sandbox
        .engine
        .pin("OneFolder", Some("1.0.0"))
        .expect("pins");
    assert!(!again.changed, "the same pin writes nothing");
    let other = sandbox
        .engine
        .pin("OneFolder", Some("0.9.0"))
        .expect("pins");
    assert_eq!(other.pinned.as_deref(), Some("0.9.0"));
    assert!(other.changed);

    // A folder of a several-folder addon names the addon.
    let ignored = sandbox
        .engine
        .ignore("SeveralFolders_Options")
        .expect("ignores");
    assert_eq!(ignored.name, "SeveralFolders");
    assert!(ignored.ignored && ignored.changed);
    let beta = sandbox
        .engine
        .set_channel("SeveralFolders", Channel::Prerelease)
        .expect("a channel");
    assert_eq!(beta.channel, Channel::Prerelease);

    let state = sandbox.state();
    let (_, one) = state.entry("OneFolder").expect("OneFolder");
    assert_eq!(one.pinned(), Some("0.9.0"));
    assert!(!one.ignored());
    let (_, several) = state.entry("SeveralFolders").expect("SeveralFolders");
    assert_eq!(several.pinned(), None);
    assert!(several.ignored());
    assert_eq!(several.channel(), Channel::Prerelease);

    assert!(sandbox.engine.unpin("OneFolder").expect("unpins").changed);
    assert!(!sandbox.engine.unpin("OneFolder").expect("unpins").changed);
    assert!(
        sandbox
            .engine
            .unignore("SeveralFolders")
            .expect("unignores")
            .changed
    );
    sandbox
        .engine
        .set_channel("SeveralFolders", Channel::Stable)
        .expect("a channel");
    let after = std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar");
    assert_eq!(
        after, before,
        "every mark undone leaves the sidecar as it was"
    );
}

#[test]
fn a_reinstall_keeps_the_marks_and_warns_when_the_pin_differs() {
    let sandbox = Sandbox::new("marks-kept");
    sandbox.install("one-folder-1.0.0.zip");
    sandbox.engine.pin("OneFolder", None).expect("pins");
    sandbox.engine.ignore("OneFolder").expect("ignores");
    sandbox
        .engine
        .set_channel("OneFolder", Channel::Prerelease)
        .expect("a channel");
    let updated = sandbox.install("one-folder-1.1.0.zip");
    assert_eq!(
        updated.warnings,
        [
            "OneFolder is pinned at 1.0.0 and 1.1.0 is installed now; the pin stays until `mrgl unpin OneFolder`"
        ]
    );
    let state = sandbox.state();
    let (_, entry) = state.entry("OneFolder").expect("the entry");
    assert_eq!(entry.version, "1.1.0");
    assert_eq!(entry.pinned(), Some("1.0.0"));
    assert!(entry.ignored());
    assert_eq!(entry.channel(), Channel::Prerelease);
}

#[test]
fn a_mark_on_an_unknown_or_unmanaged_addon_is_an_error_and_writes_nothing() {
    let sandbox = Sandbox::new("marks-unmanaged");
    let error = sandbox.engine.pin("Nope", None).expect_err("unknown");
    assert_eq!(error.to_string(), "addon not found: Nope (flavor forever)");
    std::fs::create_dir_all(sandbox.addon("ByHand")).expect("a folder by hand");
    let error = sandbox.engine.ignore("ByHand").expect_err("not managed");
    assert!(matches!(error, Error::NotManagedToMark { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        "addon not managed by mrgl: ByHand (flavor forever); it was installed by hand or by another manager, and only an addon mrgl installed can be ignored"
    );
    assert!(!sandbox.engine.home.state_path().exists());
}

#[test]
fn a_pick_installs_through_the_source_and_then_is_installed_already() {
    let sandbox = Sandbox::new("pick");
    let releases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/releases");
    let github = GitHub::new(FolderHttp::new(releases), None, None);
    let spec = SourceSpec::parse("github:Questie/Questie").expect("a spec");
    let pick = github
        .resolve(&spec, &Target::from_flavor(&sandbox.engine.flavor))
        .expect("a pick");
    let staging = sandbox.engine.home.staging_dir();
    let mut steps = Vec::new();
    let mut record = |step: PickStep<'_>| match step {
        PickStep::Downloading { pick, to } => {
            assert!(to.starts_with(&staging), "{}", to.display());
            assert!(!to.exists());
            steps.push(format!("downloading {}", pick.asset));
        }
        PickStep::Installing { pick, zip } => {
            assert!(zip.is_file(), "downloaded: {}", zip.display());
            steps.push(format!("installing {}", pick.version));
        }
    };
    let outcome = sandbox
        .engine
        .install_pick(&github, &pick, &mut record)
        .expect("installs");
    assert_eq!(
        steps,
        ["downloading Questie-v1.2.3-forever.zip", "installing 1.2.3"]
    );
    let PickInstalled::Installed(installed) = outcome else {
        panic!("installed, not {outcome:?}");
    };
    assert_eq!(installed.name, "Questie");
    assert_eq!(
        installed.version, "1.2.3",
        "the pick's, not the TOC's v1.2.3"
    );
    assert_eq!(installed.folders, ["Questie"]);
    let state = sandbox.state();
    let (_, entry) = state.entry("Questie").expect("in the sidecar");
    assert_eq!(entry.source, "github:Questie/Questie");
    assert_eq!(entry.version, "1.2.3");
    assert_eq!(sandbox.staging_entries(), Vec::<String>::new());
    assert_eq!(sandbox.cache_entries("Questie"), ["1.2.3.zip"]);

    // The same pick again, and the source spelled in another case: installed already, nothing fetched.
    let downloads = |github: &GitHub<FolderHttp>| {
        github
            .http()
            .requests()
            .iter()
            .filter(|request| request.url == pick.url)
            .count()
    };
    let mut lower = pick.clone();
    lower.source = "github:questie/questie".to_string();
    for again in [&pick, &lower] {
        let outcome = sandbox
            .engine
            .install_pick(&github, again, &mut |step| panic!("no step: {step:?}"))
            .expect("checks");
        let PickInstalled::AlreadyInstalled(current) = outcome else {
            panic!("installed already, not {outcome:?}");
        };
        assert_eq!(current.name, "Questie");
        assert_eq!(current.entry.version, "1.2.3");
        assert!(current.warnings.is_empty());
    }
    assert_eq!(downloads(&github), 1);

    // Its folder deleted by hand: not installed any more, so the pick is downloaded and installed again.
    std::fs::remove_dir_all(sandbox.addon("Questie")).expect("deleted by hand");
    let outcome = sandbox
        .engine
        .install_pick(&github, &pick, &mut |_| {})
        .expect("installs again");
    let PickInstalled::Installed(installed) = outcome else {
        panic!("installed, not {outcome:?}");
    };
    assert_eq!(
        installed.warnings,
        ["Questie was deleted by hand since its install and is no longer managed"]
    );
    assert!(sandbox.addon("Questie").join("Questie.toc").is_file());
    assert_eq!(downloads(&github), 2);
}

#[test]
fn another_version_from_the_same_source_is_updated_under_its_name() {
    let sandbox = Sandbox::new("pick-update");
    // The addon installed from a zip in hand under another name, then recorded as from the source.
    sandbox.install("one-folder-1.0.0.zip");
    let mut state = sandbox.state();
    let entry = state.addons.get_mut("OneFolder").expect("installed");
    entry.source = "github:Questie/Questie".to_string();
    sandbox.engine.home.write_state(&state).expect("writes");
    let releases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/releases");
    let github = GitHub::new(FolderHttp::new(releases), None, None);
    let spec = SourceSpec::parse("github:Questie/Questie").expect("a spec");
    let pick = github
        .resolve(&spec, &Target::from_flavor(&sandbox.engine.flavor))
        .expect("a pick");
    let outcome = sandbox
        .engine
        .install_pick(&github, &pick, &mut |_| {})
        .expect("updates");
    let PickInstalled::Installed(installed) = outcome else {
        panic!("installed, not {outcome:?}");
    };
    // The sidecar's name for the source stays the key, and the folders the new zip lacks go.
    assert_eq!(installed.name, "OneFolder");
    assert_eq!(installed.previous_version.as_deref(), Some("1.0.0"));
    assert_eq!(installed.version, "1.2.3");
    assert_eq!(installed.removed_folders, ["OneFolder"]);
    assert!(!sandbox.addon("OneFolder").exists());
    assert!(sandbox.addon("Questie").is_dir());
    let state = sandbox.state();
    assert_eq!(state.addons.len(), 1);
    assert_eq!(state.addons["OneFolder"].folders, ["Questie"]);
}

/// A zip written by the test itself, stored entries only.
fn write_zip(path: &Path, entries: &BTreeMap<&str, &str>) {
    use std::io::Write;
    let file = std::fs::File::create(path).expect("the zip file");
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in entries {
        writer.start_file(*name, options).expect("an entry");
        writer.write_all(content.as_bytes()).expect("its bytes");
    }
    writer.finish().expect("the central directory");
}

// --- task 022: the rollback to the previous version from the kept zip ---

/// Every file under a folder, relative with `/` separators, and its SHA-256.
fn tree_hashes(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, into: &mut BTreeMap<String, String>) {
        let entries =
            std::fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        for entry in entries {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(base, &path, into);
            } else {
                let relative = path
                    .strip_prefix(base)
                    .expect("under the base")
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let hash = mrgl_core::install::sha256_of(&path).expect("hashes");
                into.insert(relative, hash);
            }
        }
    }
    let mut hashes = BTreeMap::new();
    walk(dir, dir, &mut hashes);
    hashes
}

/// A fresh unpack of a fixture zip into a folder of the sandbox, independent of the engine.
fn fresh_unpack(sandbox: &Sandbox, zip_name: &str) -> PathBuf {
    let into = sandbox.root.join("fresh").join(zip_name);
    let _ = std::fs::remove_dir_all(&into);
    let file = std::fs::File::open(zip(zip_name)).expect("the fixture");
    zip::ZipArchive::new(file)
        .expect("a zip")
        .extract(&into)
        .expect("unpacks");
    into
}

#[test]
fn a_rollback_puts_the_previous_version_back_byte_for_byte_and_pins_it() {
    let sandbox = Sandbox::new("rollback");
    sandbox.install("one-folder-1.0.0.zip");
    sandbox.install("one-folder-1.1.0.zip");
    let source = sandbox
        .state()
        .entry("OneFolder")
        .expect("the entry")
        .1
        .source
        .clone();

    let rolled = sandbox.engine.rollback("onefolder").expect("rolls back");
    assert_eq!(rolled.name, "OneFolder");
    assert_eq!(rolled.version, "1.0.0");
    assert_eq!(rolled.previous_version, "1.1.0");
    assert_eq!(rolled.pinned, "1.0.0");
    assert_eq!(rolled.source, source, "the source is kept");
    assert_eq!(rolled.folders, ["OneFolder"]);
    assert!(rolled.removed_folders.is_empty());
    assert!(rolled.warnings.is_empty(), "{:?}", rolled.warnings);

    // The files are 1.0.0's, byte for byte: old.lua is back, new.lua is gone.
    let first = tree_hashes(&fresh_unpack(&sandbox, "one-folder-1.0.0.zip").join("OneFolder"));
    assert_eq!(tree_hashes(&sandbox.addon("OneFolder")), first);
    assert!(sandbox.addon("OneFolder/old.lua").is_file());
    assert!(!sandbox.addon("OneFolder/new.lua").exists());

    let state = sandbox.state();
    let (_, entry) = state.entry("OneFolder").expect("the entry");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.previous_version.as_deref(), Some("1.1.0"));
    assert_eq!(entry.pinned(), Some("1.0.0"));
    assert_eq!(entry.source, source);
    let recorded: BTreeMap<String, String> = entry
        .files
        .iter()
        .map(|(path, hash)| {
            (
                path.trim_start_matches("OneFolder/").to_string(),
                hash.clone(),
            )
        })
        .collect();
    assert_eq!(recorded, first, "the sidecar records 1.0.0's hashes");

    // The cache holds both zips, unchanged; the staging folder is empty.
    assert_eq!(
        sandbox.cache_entries("OneFolder"),
        ["1.0.0.zip", "1.1.0.zip"]
    );
    for version in ["1.0.0", "1.1.0"] {
        assert_eq!(
            std::fs::read(sandbox.engine.home.cached_zip("OneFolder", version)).expect("cached"),
            std::fs::read(zip(&format!("one-folder-{version}.zip"))).expect("the fixture"),
            "{version}"
        );
    }
    assert!(sandbox.staging_entries().is_empty());

    // A second rollback goes forward again, and pins there.
    let forward = sandbox.engine.rollback("OneFolder").expect("rolls forward");
    assert_eq!(forward.version, "1.1.0");
    assert_eq!(forward.previous_version, "1.0.0");
    assert_eq!(forward.pinned, "1.1.0");
    let second = tree_hashes(&fresh_unpack(&sandbox, "one-folder-1.1.0.zip").join("OneFolder"));
    assert_eq!(tree_hashes(&sandbox.addon("OneFolder")), second);
    let state = sandbox.state();
    let (_, entry) = state.entry("OneFolder").expect("the entry");
    assert_eq!(entry.pinned(), Some("1.1.0"));
    assert_eq!(entry.previous_version.as_deref(), Some("1.0.0"));
    assert_eq!(
        sandbox.cache_entries("OneFolder"),
        ["1.0.0.zip", "1.1.0.zip"]
    );
}

#[test]
fn a_rollback_brings_back_a_dropped_folder_and_keeps_the_other_marks() {
    let sandbox = Sandbox::new("rollback-folders");
    sandbox.install("several-folders-2.0.0.zip");
    let updated = sandbox.install("several-folders-2.1.0.zip");
    assert_eq!(updated.removed_folders, ["SeveralFolders_Data"]);
    sandbox
        .engine
        .pin("SeveralFolders", None)
        .expect("pins at 2.1.0");
    sandbox.engine.ignore("SeveralFolders").expect("ignores");
    sandbox
        .engine
        .set_channel("SeveralFolders", Channel::Prerelease)
        .expect("a channel");

    let rolled = sandbox
        .engine
        .rollback("SeveralFolders_Options")
        .expect("a folder names the addon");
    assert_eq!(rolled.name, "SeveralFolders");
    assert_eq!(rolled.version, "2.0.0");
    assert_eq!(
        rolled.folders,
        [
            "SeveralFolders",
            "SeveralFolders_Data",
            "SeveralFolders_Options"
        ]
    );
    assert!(rolled.warnings.is_empty(), "{:?}", rolled.warnings);
    let fresh = fresh_unpack(&sandbox, "several-folders-2.0.0.zip");
    for folder in &rolled.folders {
        assert_eq!(
            tree_hashes(&sandbox.addon(folder)),
            tree_hashes(&fresh.join(folder)),
            "{folder}"
        );
    }
    let state = sandbox.state();
    let (_, entry) = state.entry("SeveralFolders").expect("the entry");
    assert_eq!(
        entry.pinned(),
        Some("2.0.0"),
        "the pin moves to the rolled-back version"
    );
    assert!(entry.ignored());
    assert_eq!(entry.channel(), Channel::Prerelease);

    let forward = sandbox
        .engine
        .rollback("SeveralFolders")
        .expect("rolls forward");
    assert_eq!(forward.version, "2.1.0");
    assert_eq!(forward.removed_folders, ["SeveralFolders_Data"]);
    assert!(!sandbox.addon("SeveralFolders_Data").exists());
}

#[test]
fn a_rollback_without_a_previous_version_or_its_zip_is_an_error_and_changes_nothing() {
    let sandbox = Sandbox::new("rollback-errors");
    let error = sandbox.engine.rollback("Nope").expect_err("unknown");
    assert_eq!(error.to_string(), "addon not found: Nope (flavor forever)");
    std::fs::create_dir_all(sandbox.addon("ByHand")).expect("a folder by hand");
    let error = sandbox.engine.rollback("ByHand").expect_err("not managed");
    assert_eq!(
        error.to_string(),
        "addon not managed by mrgl: ByHand (flavor forever); it was installed by hand or by another manager, and only an addon mrgl installed can be rolled back"
    );
    assert!(!sandbox.engine.home.state_path().exists());

    sandbox.install("one-folder-1.0.0.zip");
    let before = std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar");
    let error = sandbox
        .engine
        .rollback("OneFolder")
        .expect_err("no previous");
    assert!(matches!(error, Error::NoPreviousVersion { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        "nothing to roll back to: OneFolder 1.0.0 has no previous version (flavor forever); mrgl keeps the version an update replaces"
    );
    assert_eq!(
        std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar"),
        before
    );

    sandbox.install("one-folder-1.1.0.zip");
    let kept = sandbox.engine.home.cached_zip("OneFolder", "1.0.0");
    std::fs::remove_file(&kept).expect("the kept zip deleted by hand");
    let before = std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar");
    let addon_before = tree_hashes(&sandbox.addon("OneFolder"));
    let error = sandbox
        .engine
        .rollback("OneFolder")
        .expect_err("no kept zip");
    assert!(matches!(error, Error::RollbackZipMissing { .. }), "{error}");
    assert_eq!(
        error.to_string(),
        format!(
            "cannot roll back OneFolder to 1.0.0: its zip is not in the cache ({}); install that version again with `mrgl add <zip>`",
            kept.display()
        )
    );
    assert_eq!(
        std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar"),
        before,
        "the sidecar is untouched"
    );
    assert_eq!(tree_hashes(&sandbox.addon("OneFolder")), addon_before);
    assert!(sandbox.staging_entries().is_empty());
}

// --- task 039: the bound on the unpacked size of a zip ---

use mrgl_core::zip_entry::Excess;

/// The uncompressed sizes a fixture zip's files declare, summed.
fn declared_size(zip_name: &str) -> u64 {
    let file = std::fs::File::open(zip(zip_name)).expect("the fixture");
    let mut archive = zip::ZipArchive::new(file).expect("a zip");
    (0..archive.len())
        .map(|index| archive.by_index(index).expect("an entry").size())
        .sum()
}

#[test]
fn a_zip_declaring_more_than_the_bound_is_refused_before_anything_is_written() {
    let sandbox = Sandbox::new("declares-too-much");
    let name = "declares-too-much-1.0.0.zip";
    let error = sandbox
        .engine
        .install(&zip(name), &metadata(name))
        .expect_err("refused");
    assert!(
        matches!(
            &error,
            Error::ZipTooLarge {
                excess: Excess::Declared { .. },
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "refused {}: it declares 3.0 GB unpacked, more than the bound of 2.0 GB; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you trust",
            zip(name).display()
        )
    );
    assert!(sandbox.folders().is_empty());
    assert!(sandbox.staging_entries().is_empty());
    assert!(!sandbox.engine.home.state_path().exists());

    // A higher bound for a zip the user trusts: the declared size passes, the real bytes are a few.
    let installed = sandbox
        .engine
        .clone()
        .with_max_unpacked(4 << 30)
        .install(&zip(name), &metadata(name))
        .expect("installed under 4 GiB");
    assert_eq!(installed.folders, ["DeclaresTooMuch"]);
    assert_eq!(
        read(&sandbox.addon("DeclaresTooMuch").join("core.lua")),
        "-- DeclaresTooMuch 1.0.0\n"
    );
    assert!(sandbox.staging_entries().is_empty());
}

#[test]
fn a_zip_that_lies_about_its_size_is_refused_while_writing_and_staging_stays_empty() {
    let sandbox = Sandbox::new("lies-about-its-size");
    let name = "lies-about-its-size-1.0.0.zip";
    let error = sandbox
        .engine
        .install(&zip(name), &metadata(name))
        .expect_err("refused");
    match &error {
        Error::ZipTooLarge {
            excess: Excess::Entry {
                entry, declared, ..
            },
            ..
        } => {
            assert_eq!(entry, "LiesAboutItsSize/core.lua");
            assert_eq!(*declared, 16);
        }
        other => panic!("{other}"),
    }
    assert!(
        error.to_string().ends_with(
            "the entry `LiesAboutItsSize/core.lua` unpacks to more than the 16 B it declares, so the bound of 2.0 GB cannot hold (109 B written, then removed)"
        ),
        "{error}"
    );
    assert!(sandbox.folders().is_empty());
    assert!(
        sandbox.staging_entries().is_empty(),
        "what was written is removed with the staging folder"
    );
    assert!(!sandbox.engine.home.state_path().exists());
}

#[test]
fn a_normal_zip_installs_up_to_a_bound_of_exactly_its_size() {
    let sandbox = Sandbox::new("bound-exact");
    let name = "one-folder-1.0.0.zip";
    let size = declared_size(name);
    let error = sandbox
        .engine
        .clone()
        .with_max_unpacked(size - 1)
        .install(&zip(name), &metadata(name))
        .expect_err("one byte short");
    assert!(
        matches!(
            &error,
            Error::ZipTooLarge {
                excess: Excess::Declared { declared, limit },
                ..
            } if *declared == size && *limit == size - 1
        ),
        "{error}"
    );
    assert!(sandbox.staging_entries().is_empty());
    let installed = sandbox
        .engine
        .clone()
        .with_max_unpacked(size)
        .install(&zip(name), &metadata(name))
        .expect("at the bound");
    assert_eq!(installed.version, "1.0.0");
    // The default bound, as every other test here installs.
    sandbox.install("one-folder-1.1.0.zip");
}

#[test]
fn a_rollback_past_the_bound_is_refused_and_changes_nothing() {
    let sandbox = Sandbox::new("rollback-bound");
    sandbox.install("one-folder-1.0.0.zip");
    sandbox.install("one-folder-1.1.0.zip");
    let before = std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar");
    let addon_before = tree_hashes(&sandbox.addon("OneFolder"));
    let kept = sandbox.engine.home.cached_zip("OneFolder", "1.0.0");
    let error = sandbox
        .engine
        .clone()
        .with_max_unpacked(100)
        .rollback("OneFolder")
        .expect_err("past a bound of 100 bytes");
    assert!(
        matches!(&error, Error::ZipTooLarge { path, .. } if path == &kept),
        "{error}"
    );
    assert_eq!(
        std::fs::read(sandbox.engine.home.state_path()).expect("the sidecar"),
        before
    );
    assert_eq!(tree_hashes(&sandbox.addon("OneFolder")), addon_before);
    assert!(sandbox.staging_entries().is_empty());
}
