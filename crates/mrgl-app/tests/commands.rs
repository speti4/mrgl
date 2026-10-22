//! The IPC commands of the window (`commands`, task 117) on Tauri's mock runtime (`tauri::test`): the app
//! with the two run states, a mock window for the theme, a channel that collects the progress events, and
//! the async commands under `block_on`. The busy and refused arms hold a run's guard before the call. A
//! command that reads the process environment (`MRGL_HOME`, `MRGL_ROOT`, `MRGL_DISCOVERY`,
//! `MRGL_HTTP_STUB`, the key store) runs in a child: the test binary re-executes itself for that one test
//! with the environment set on the child, as the command line's tests set it on `mrgl.exe`, and no test
//! sets a variable in-process. Nothing touches the network or a real game folder.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use mrgl_app_lib::actions::{self, UpdateEvent, UpdateRun};
use mrgl_app_lib::commands;
use mrgl_app_lib::toolbox::{self, ToolboxRun};
use mrgl_core::VERSION;
use mrgl_core::flavor::FlavorId;
use mrgl_core::sets::{self, AddonSet, AddonState, SetsFile};
use mrgl_core::state::{Entry, FlavorHome, State};
use tauri::Manager;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::test::MockRuntime;

const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Tags!STRING:0|Product!STRING:0\n\
eu|1|e8c4ef4e67bd1d94d820254984258fa8|1.60.1.70245||wowf\n";

/// The marker the parent sets on the child: the test body runs there.
const CHILD_ENV: &str = "MRGL_APP_TEST_CHILD";
/// The parent's process id, set on the child, so both name the same temporary folders.
const PARENT_ENV: &str = "MRGL_APP_TEST_PARENT";

/// The process id the temporary folders are named by: the parent's, in a child run too.
fn test_id() -> String {
    std::env::var(PARENT_ENV).unwrap_or_else(|_| std::process::id().to_string())
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../mrgl-core/tests/fixtures")
}

/// The app of the commands: the mock runtime with the update run and the toolbox run managed, as `run()`
/// manages them.
fn app() -> tauri::App<MockRuntime> {
    tauri::test::mock_builder()
        .manage(UpdateRun::default())
        .manage(ToolboxRun::default())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("the mock app")
}

/// The update run the app holds, for as long as the app.
fn update_run(app: &tauri::App<MockRuntime>) -> &UpdateRun {
    app.state::<UpdateRun>().inner()
}

/// The toolbox run the app holds, for as long as the app.
fn toolbox_run(app: &tauri::App<MockRuntime>) -> &ToolboxRun {
    app.state::<ToolboxRun>().inner()
}

fn window(app: &tauri::App<MockRuntime>) -> tauri::WebviewWindow<MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .expect("the mock window")
}

/// A progress channel and the events it received, as the web view would, in order.
fn channel() -> (Channel<UpdateEvent>, Arc<Mutex<Vec<serde_json::Value>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let received = Arc::clone(&events);
    let channel = Channel::new(move |body| {
        let text = match body {
            InvokeResponseBody::Json(text) => text,
            InvokeResponseBody::Raw(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        };
        received
            .lock()
            .expect("the events")
            .push(serde_json::from_str(&text).expect("an event as JSON"));
        Ok(())
    });
    (channel, events)
}

fn phases(events: &Mutex<Vec<serde_json::Value>>) -> Vec<String> {
    events
        .lock()
        .expect("the events")
        .iter()
        .map(|event| event["phase"].as_str().expect("a phase").to_string())
        .collect()
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tauri::async_runtime::block_on(future)
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

/// A temporary game root with a Forever flavor, a mrgl home folder and a folder for the HTTP stub,
/// removed at the end.
struct Game {
    dir: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let dir = std::env::temp_dir().join(format!("mrgl-app-commands-{}-{name}", test_id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = Game { dir };
        let flavor = game.root().join("_forever_");
        std::fs::create_dir_all(flavor.join("Interface/AddOns")).expect("the flavor folder");
        std::fs::write(game.root().join(".build.info"), BUILD_INFO).expect(".build.info");
        std::fs::write(
            flavor.join(".flavor.info"),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect(".flavor.info");
        std::fs::create_dir_all(game.home()).expect("the home folder");
        game
    }

    fn root(&self) -> PathBuf {
        self.dir.join("World of Warcraft")
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn addons(&self) -> PathBuf {
        self.root().join("_forever_/Interface/AddOns")
    }

    /// `Questie` 1.2.2 installed by hand and managed from `github:Questie/Questie` in the sidecar; the
    /// fixture answers of the GitHub source have 1.2.3 for it.
    fn with_questie(self) -> Game {
        let folder = self.addons().join("Questie");
        std::fs::create_dir_all(&folder).expect("the addon folder");
        std::fs::write(
            folder.join("Questie.toc"),
            "## Interface: 16001\n## Title: Questie\n## Version: 1.2.2\n",
        )
        .expect("the TOC");
        let mut state = State::new(&FlavorId::Forever);
        state.addons.insert(
            "Questie".into(),
            Entry {
                source: "github:Questie/Questie".to_string(),
                version: "1.2.2".to_string(),
                previous_version: None,
                folders: vec!["Questie".to_string()],
                files: Default::default(),
                installed_at: "2026-10-22T10:00:00Z".to_string(),
                nolib: None,
                pinned: None,
                ignored: false,
                channel: mrgl_core::sources::Channel::Stable,
                forever_build: None,
            },
        );
        FlavorHome::at(&self.home(), &FlavorId::Forever)
            .write_state(&state)
            .expect("the sidecar");
        self
    }

    /// The `WTF` tree of the addon-sets fixture (two characters), `RaidFrames` installed, and the set
    /// `raid` in the home folder.
    fn with_sets(self) -> Game {
        copy_tree(
            &fixtures().join("addon-sets/WTF"),
            &self.root().join("_forever_/WTF"),
        );
        let folder = self.addons().join("RaidFrames");
        std::fs::create_dir_all(&folder).expect("the addon folder");
        std::fs::write(
            folder.join("RaidFrames.toc"),
            "## Interface: 11508\n## Title: RaidFrames\n## Version: 1.0.0\n",
        )
        .expect("the TOC");
        let mut file = SetsFile::new(&FlavorId::Forever);
        file.sets.insert(
            "raid".to_string(),
            AddonSet {
                saved_at: "2026-10-22T10:00:00Z".to_string(),
                addons: [
                    ("RaidFrames".to_string(), AddonState::Enabled),
                    ("Missing".to_string(), AddonState::Disabled),
                ]
                .into_iter()
                .collect(),
            },
        );
        sets::write_sets_at(&sets::sets_path_at(&self.home(), &FlavorId::Forever), &file)
            .expect("the sets file");
        self
    }

    /// The game folder of the adopt fixture in place of the bare flavor, and the stub of every source
    /// (the `releases/` and `sources/` fixture trees in one folder) for `MRGL_HTTP_STUB`.
    fn with_adopt_fixture(self) -> Game {
        let _ = std::fs::remove_dir_all(self.root());
        copy_tree(&fixtures().join("adopt"), &self.root());
        copy_tree(&fixtures().join("releases"), &self.stub());
        copy_tree(&fixtures().join("sources"), &self.stub());
        self
    }

    fn stub(&self) -> PathBuf {
        self.dir.join("stub")
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The environment of a child run: the variables set and the ones removed, over the defaults.
#[derive(Default)]
struct ChildEnv {
    set: Vec<(&'static str, PathBuf)>,
    remove: Vec<&'static str>,
}

/// Runs `body` in a child process when this is the parent, and `body` itself when this is the child. The
/// child is this test binary with the one test, `MRGL_KEY_STORE=off`, the fixture answers as
/// `MRGL_HTTP_STUB`, the discovery off, every other `MRGL_*` variable removed, and `env` on top; the
/// parent asserts that the child passed. Under cargo-llvm-cov the child writes its own profile, as the
/// command line's exe does.
fn in_child(test: &str, env: ChildEnv, body: impl FnOnce()) {
    if std::env::var_os(CHILD_ENV).is_some() {
        body();
        return;
    }
    let mut command = Command::new(std::env::current_exe().expect("the test exe"));
    command.args([test, "--exact", "--nocapture", "--test-threads=1"]);
    for name in [
        "MRGL_SERIOUS",
        "MRGL_ROOT",
        "MRGL_HOME",
        "MRGL_GITHUB_TOKEN",
        "MRGL_WAGO_KEY",
        "MRGL_CURSEFORGE_KEY",
    ] {
        command.env_remove(name);
    }
    command.env(CHILD_ENV, "1");
    command.env(PARENT_ENV, test_id());
    command.env("MRGL_KEY_STORE", "off");
    command.env("MRGL_HTTP_STUB", fixtures().join("releases"));
    command.env("MRGL_DISCOVERY", "off");
    for name in &env.remove {
        command.env_remove(name);
    }
    for (name, value) in &env.set {
        command.env(name, value);
    }
    let output = command.output().expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the child failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("test result: ok. 1 passed"),
        "the child ran the one test:\n{stdout}"
    );
}

fn env(set: &[(&'static str, &Path)]) -> ChildEnv {
    ChildEnv {
        set: set
            .iter()
            .map(|(name, value)| (*name, value.to_path_buf()))
            .collect(),
        remove: Vec::new(),
    }
}

fn last(messages: &[commands::Message]) -> (&str, &str) {
    let message = messages.last().expect("a message");
    (message.kind, message.plain.as_str())
}

// --- in-process: the version, the busy arms, the cancel, the theme ---

#[test]
fn the_version_is_the_workspace_one() {
    assert_eq!(commands::app_version(), VERSION);
}

#[test]
fn an_update_is_refused_while_another_update_or_a_toolbox_write_runs() {
    let app = app();
    let handle = app.handle().clone();
    let (progress, events) = channel();
    let guard = toolbox_run(&app).start().expect("a write starts");
    let answer = block_on(commands::check_updates(
        handle.clone(),
        "wowf".into(),
        progress.clone(),
    ));
    assert_eq!(answer.flavor, None);
    assert_eq!(
        last(&answer.messages),
        ("warning", toolbox::toolbox_busy_line())
    );
    drop(guard);

    let guard = update_run(&app).start().expect("a run starts");
    let answer = block_on(commands::update_all(
        handle.clone(),
        "wowf".into(),
        progress.clone(),
    ));
    assert_eq!(last(&answer.messages), ("warning", actions::busy_line()));
    let answer = block_on(commands::update_one(
        handle,
        "wowf".into(),
        "Questie".into(),
        progress,
    ));
    assert_eq!(last(&answer.messages), ("warning", actions::busy_line()));
    drop(guard);
    assert!(phases(&events).is_empty(), "a refused run sends no event");
}

#[test]
fn a_mark_waits_for_an_update_and_for_a_toolbox_write_and_refuses_a_bad_action() {
    let app = app();
    let handle = app.handle().clone();
    let mark = |action: &str, value: Option<&str>| {
        block_on(commands::mark(
            handle.clone(),
            "wowf".into(),
            "Questie".into(),
            action.into(),
            value.map(str::to_string),
        ))
    };
    let guard = update_run(&app).start().expect("a run starts");
    assert_eq!(
        last(&mark("pin", None).messages),
        (
            "warning",
            "an update is running; pin, ignore and the channel wait until it ends"
        )
    );
    drop(guard);
    let guard = toolbox_run(&app).start().expect("a write starts");
    assert_eq!(
        last(&mark("ignore", None).messages),
        ("warning", toolbox::marks_wait_line())
    );
    drop(guard);
    // The action is read before the engine, so no game root is needed for a bad one.
    assert_eq!(
        last(&mark("rename", None).messages),
        (
            "error",
            "unknown row action rename: pin, unpin, ignore, unignore or channel"
        )
    );
    assert_eq!(
        last(&mark("channel", Some(" ")).messages),
        ("error", "a channel change needs stable or prerelease")
    );
}

#[test]
fn a_toolbox_write_is_refused_while_another_one_runs() {
    let app = app();
    let handle = app.handle().clone();
    let guard = toolbox_run(&app).start().expect("a write starts");
    let answers = [
        block_on(commands::apply_set(
            handle.clone(),
            "wowf".into(),
            "raid".into(),
            None,
            None,
            None,
        )),
        block_on(commands::undo_set(handle.clone(), "wowf".into())),
        block_on(commands::edit_set(
            handle.clone(),
            "wowf".into(),
            "raid".into(),
            "RaidFrames".into(),
            "disable".into(),
        )),
        block_on(commands::clean_saved_variables(
            handle.clone(),
            "wowf".into(),
        )),
    ];
    for answer in &answers {
        assert_eq!(answer.toolbox, None);
        assert_eq!(last(&answer.messages), ("warning", toolbox::busy_line()));
    }
    let adopt = block_on(commands::adopt_apply(
        handle.clone(),
        "wowf".into(),
        false,
        None,
    ));
    assert_eq!(adopt.adopt, None);
    assert_eq!(last(&adopt.messages), ("warning", toolbox::busy_line()));
    drop(guard);

    // The cleanup and the adopt wait for an update too: both touch what an install swaps or writes.
    let guard = update_run(&app).start().expect("a run starts");
    let answer = block_on(commands::clean_saved_variables(
        handle.clone(),
        "wowf".into(),
    ));
    assert_eq!(
        last(&answer.messages),
        ("warning", toolbox::updating_line())
    );
    let adopt = block_on(commands::adopt_apply(handle, "wowf".into(), true, None));
    assert_eq!(
        last(&adopt.messages),
        ("warning", toolbox::adopt_updating_line())
    );
    drop(guard);
}

#[test]
fn a_cancel_reaches_the_run_the_app_holds() {
    let app = app();
    let message = commands::cancel_update(app.state::<UpdateRun>());
    assert_eq!(
        (message.kind, message.plain.as_str()),
        ("nothing", "no update is running, nothing to cancel")
    );
    let guard = update_run(&app).start().expect("a run starts");
    let message = commands::cancel_update(app.state::<UpdateRun>());
    assert_eq!(message.kind, "progress");
    assert!(app.state::<UpdateRun>().cancelled());
    drop(guard);
}

#[test]
fn the_mock_window_takes_every_theme() {
    let app = app();
    let window = window(&app);
    for theme in ["light", "dark", "auto", "sepia"] {
        commands::apply_native_theme(&window, theme);
    }
    assert_eq!(window.label(), "main");
}

// --- in a child: the commands over the process's root, home and sources ---

#[test]
fn the_flavors_and_the_resolve_read_the_root_of_the_process() {
    let game = Game::new("flavors").with_questie();
    in_child(
        "the_flavors_and_the_resolve_read_the_root_of_the_process",
        env(&[("MRGL_ROOT", &game.root()), ("MRGL_HOME", &game.home())]),
        || {
            let answer = block_on(commands::flavors());
            assert_eq!(
                answer.root.as_deref(),
                Some(game.root().to_str().expect("utf-8"))
            );
            assert_eq!(answer.message.kind, "banner");
            assert_eq!(
                answer.message.plain,
                format!("1 flavor under {}, 1 addon", game.root().display())
            );
            let row = &answer.flavors[0].addons[0];
            assert!(row.managed && row.folder == "Questie");

            let answer = block_on(commands::resolve("wowf".into(), "Questie".into()));
            let pick = answer.pick.expect("the pick");
            assert_eq!(
                (pick.source.as_str(), pick.version.as_str()),
                ("github:Questie/Questie", "1.2.3")
            );
            assert_eq!(answer.message.kind, "success");

            // A flavor the root does not have: a known folder that is missing, and an unknown id.
            let answer = block_on(commands::resolve("wow_classic".into(), "Questie".into()));
            assert_eq!(answer.pick, None);
            assert_eq!(answer.message.kind, "error");
            assert!(
                answer.message.plain.contains("_classic_"),
                "{}",
                answer.message.plain
            );
            let answer = block_on(commands::resolve("wow_mrgl".into(), "Questie".into()));
            assert_eq!(
                answer.message.plain,
                "no flavor wow_mrgl: no game root, or no such flavor folder under it"
            );
        },
    );
}

#[test]
fn the_diagnostics_are_the_doctors_report_of_the_process() {
    let game = Game::new("diagnostics").with_questie();
    in_child(
        "the_diagnostics_are_the_doctors_report_of_the_process",
        env(&[("MRGL_ROOT", &game.root()), ("MRGL_HOME", &game.home())]),
        || {
            let diagnostics = block_on(commands::diagnostics());
            assert!(
                diagnostics.text.contains("flavor: _forever_"),
                "{}",
                diagnostics.text
            );
            assert!(
                diagnostics
                    .copied
                    .plain
                    .starts_with("diagnostics of 1 flavor copied, "),
                "{}",
                diagnostics.copied.plain
            );
            assert_eq!(diagnostics.failed.kind, "error");
        },
    );
}

#[test]
fn the_update_commands_run_on_the_engine_of_the_process() {
    let game = Game::new("updates").with_questie();
    in_child(
        "the_update_commands_run_on_the_engine_of_the_process",
        env(&[("MRGL_ROOT", &game.root()), ("MRGL_HOME", &game.home())]),
        || {
            let app = app();
            let handle = app.handle().clone();
            let (progress, events) = channel();
            let answer = block_on(commands::check_updates(
                handle.clone(),
                "wowf".into(),
                progress.clone(),
            ));
            assert_eq!(
                last(&answer.messages),
                ("success", "checked _forever_: 1 update available")
            );
            assert_eq!(phases(&events), ["checking", "checked"]);
            assert!(!app.state::<UpdateRun>().is_running(), "the run ended");

            let (progress, events) = channel();
            let answer = block_on(commands::update_all(
                handle.clone(),
                "wowf".into(),
                progress,
            ));
            assert_eq!(last(&answer.messages), ("success", "1 addon updated"));
            assert_eq!(
                phases(&events),
                [
                    "checking",
                    "checked",
                    "updating",
                    "downloading",
                    "installing",
                    "updated"
                ]
            );
            let flavor = answer.flavor.expect("the flavor");
            assert_eq!(flavor.addons[0].version.as_deref(), Some("1.2.3"));

            let (progress, events) = channel();
            let answer = block_on(commands::update_one(
                handle.clone(),
                "wowf".into(),
                "Questie".into(),
                progress,
            ));
            assert_eq!(
                last(&answer.messages),
                ("nothing", "nothing updated: 1 up to date")
            );
            assert_eq!(phases(&events), ["checking", "checked"]);

            let answer = block_on(commands::mark(
                handle.clone(),
                "wowf".into(),
                "Questie".into(),
                "ignore".into(),
                None,
            ));
            assert_eq!(last(&answer.messages).0, "success");
            assert!(answer.flavor.expect("the flavor").addons[0].ignored);

            // The engine of a flavor the root does not have.
            let (progress, _) = channel();
            let answer = block_on(commands::update_one(
                handle.clone(),
                "wow_classic".into(),
                "Questie".into(),
                progress,
            ));
            assert_eq!(answer.flavor, None);
            assert!(
                answer.messages[0].plain.contains("_classic_"),
                "{}",
                answer.messages[0].plain
            );
            let answer = block_on(commands::mark(
                handle,
                "wow_mrgl".into(),
                "Questie".into(),
                "pin".into(),
                None,
            ));
            assert_eq!(
                last(&answer.messages),
                (
                    "error",
                    "no flavor wow_mrgl: no game root, or no such flavor folder under it"
                )
            );
        },
    );
}

#[test]
fn the_toolbox_commands_run_on_the_root_of_the_process() {
    let game = Game::new("toolbox").with_sets();
    in_child(
        "the_toolbox_commands_run_on_the_root_of_the_process",
        env(&[("MRGL_ROOT", &game.root()), ("MRGL_HOME", &game.home())]),
        || {
            let app = app();
            let handle = app.handle().clone();
            let answer = block_on(commands::toolbox("wowf".into()));
            let toolbox = answer.toolbox.expect("the toolbox");
            assert_eq!(toolbox.sets.len(), 1);
            assert_eq!(toolbox.characters.len(), 2);

            let answer = block_on(commands::apply_set(
                handle.clone(),
                "wowf".into(),
                "raid".into(),
                Some("Mrgltest".into()),
                Some("Fixture Realm".into()),
                None,
            ));
            assert_eq!(last(&answer.messages).0, "success");
            assert_eq!(answer.changes.len(), 1);
            assert!(!app.state::<ToolboxRun>().is_running(), "the write ended");

            let answer = block_on(commands::undo_set(handle.clone(), "wowf".into()));
            assert_eq!(last(&answer.messages).0, "success");
            assert!(answer.undone.is_some());

            let answer = block_on(commands::edit_set(
                handle.clone(),
                "wowf".into(),
                "raid".into(),
                "Missing".into(),
                "drop".into(),
            ));
            assert_eq!(last(&answer.messages).0, "success");
            assert_eq!(answer.toolbox.expect("the toolbox").sets[0].addons.len(), 1);

            let answer = block_on(commands::clean_saved_variables(handle, "wowf".into()));
            assert_eq!(answer.zip, None, "no orphan to move");
            assert_eq!(last(&answer.messages).0, "nothing");
        },
    );
}

#[test]
fn the_adopt_commands_run_on_the_root_of_the_process() {
    let game = Game::new("adopt").with_adopt_fixture();
    in_child(
        "the_adopt_commands_run_on_the_root_of_the_process",
        env(&[
            ("MRGL_ROOT", &game.root()),
            ("MRGL_HOME", &game.home()),
            ("MRGL_HTTP_STUB", &game.stub()),
        ]),
        || {
            let app = app();
            let handle = app.handle().clone();
            let report = block_on(commands::adopt("wowf".into()));
            let view = report.adopt.expect("the report");
            assert!(!view.rows.is_empty());
            assert!(
                view.rows
                    .iter()
                    .any(|row| row.verdict == "needs MRGL_WAGO_KEY"),
                "no key in this process: {:?}",
                view.rows.iter().map(|row| &row.verdict).collect::<Vec<_>>()
            );

            let answer = block_on(commands::adopt_apply(
                handle,
                "wowf".into(),
                false,
                Some(view.answers),
            ));
            // Five without a Wago key; `MrglElsewhere` and `Questie` have two folders each.
            assert_eq!(answer.adopted.len(), 5, "{:?}", answer.messages);
            assert_eq!(last(&answer.messages).0, "success");
            let managed = answer
                .flavor
                .expect("the flavor")
                .addons
                .iter()
                .filter(|row| row.managed)
                .count();
            assert_eq!(managed, 7, "every adopted folder is a managed row now");
            assert!(!app.state::<ToolboxRun>().is_running(), "the write ended");
        },
    );
}

#[test]
fn the_preferences_are_stored_in_the_home_of_the_process() {
    let game = Game::new("preferences");
    in_child(
        "the_preferences_are_stored_in_the_home_of_the_process",
        env(&[("MRGL_HOME", &game.home())]),
        || {
            let app = app();
            let window = window(&app);
            let before = commands::preferences();
            assert_eq!(
                (before.theme.as_str(), before.serious_source),
                ("auto", "default")
            );
            let answer = commands::set_preferences(window.clone(), Some("dark".into()), Some(true))
                .expect("stored");
            assert_eq!(
                (answer.theme.as_str(), answer.serious, answer.serious_source),
                ("dark", true, "file")
            );
            assert_eq!(answer.stored.theme.as_deref(), Some("dark"));
            let after = commands::preferences();
            assert_eq!(after, answer);
            let error = commands::set_preferences(window, Some("sepia".into()), None)
                .expect_err("an unknown theme is refused");
            assert_eq!(error.kind, "error");
            assert!(
                error.plain.starts_with("preferences not saved: "),
                "{}",
                error.plain
            );
            assert_eq!(commands::preferences(), after, "nothing changed");
        },
    );
}

#[test]
fn a_config_that_cannot_be_read_refuses_every_command_that_needs_the_root() {
    let game = Game::new("config-folder").with_questie();
    // A folder where the config file should be: the read fails, and so does every root resolution.
    std::fs::create_dir_all(game.home().join("config.json")).expect("the folder");
    in_child(
        "a_config_that_cannot_be_read_refuses_every_command_that_needs_the_root",
        env(&[("MRGL_HOME", &game.home())]),
        || {
            let app = app();
            let handle = app.handle().clone();
            let reason = |plain: &str| plain.contains("config.json");
            let answer = block_on(commands::flavors());
            assert_eq!((answer.root, answer.flavors.len()), (None, 0));
            assert_eq!(answer.message.kind, "error");
            assert!(reason(&answer.message.plain), "{}", answer.message.plain);
            let answer = block_on(commands::resolve("wowf".into(), "Questie".into()));
            assert!(answer.pick.is_none() && reason(&answer.message.plain));
            let (progress, _) = channel();
            let answer = block_on(commands::check_updates(
                handle.clone(),
                "wowf".into(),
                progress,
            ));
            assert!(answer.flavor.is_none() && reason(&answer.messages[0].plain));
            let answer = block_on(commands::toolbox("wowf".into()));
            assert!(answer.toolbox.is_none() && reason(&answer.messages[0].plain));
            let answer = block_on(commands::adopt("wowf".into()));
            assert!(answer.adopt.is_none() && reason(&answer.messages[0].plain));
            let answer = block_on(commands::undo_set(handle, "wowf".into()));
            assert!(answer.toolbox.is_none() && reason(&answer.messages[0].plain));
        },
    );
}

#[test]
fn without_a_game_root_every_command_that_needs_one_says_so() {
    let game = Game::new("no-root");
    in_child(
        "without_a_game_root_every_command_that_needs_one_says_so",
        env(&[("MRGL_HOME", &game.home())]),
        || {
            let app = app();
            let handle = app.handle().clone();
            let no_flavor = "no flavor wowf: no game root, or no such flavor folder under it";
            let no_root = "no flavor wowf: no game root; choose the game folder";
            let answer = block_on(commands::flavors());
            assert_eq!(answer.message.kind, "nothing");
            let answer = block_on(commands::resolve("wowf".into(), "Questie".into()));
            assert_eq!(
                last(std::slice::from_ref(&answer.message)),
                ("error", no_flavor)
            );
            let (progress, _) = channel();
            let answer = block_on(commands::update_one(
                handle.clone(),
                "wowf".into(),
                "Questie".into(),
                progress,
            ));
            assert_eq!(last(&answer.messages), ("error", no_flavor));
            let answer = block_on(commands::toolbox("wowf".into()));
            assert_eq!(last(&answer.messages), ("error", no_root));
            let answer = block_on(commands::apply_set(
                handle.clone(),
                "wowf".into(),
                "raid".into(),
                None,
                None,
                None,
            ));
            assert_eq!(last(&answer.messages), ("error", no_root));
            let answer = block_on(commands::edit_set(
                handle.clone(),
                "wowf".into(),
                "raid".into(),
                "RaidFrames".into(),
                "enable".into(),
            ));
            assert_eq!(last(&answer.messages), ("error", no_root));
            let answer = block_on(commands::clean_saved_variables(
                handle.clone(),
                "wowf".into(),
            ));
            assert_eq!(last(&answer.messages), ("error", no_root));
            let answer = block_on(commands::adopt("wowf".into()));
            assert_eq!(last(&answer.messages), ("error", no_root));
            let answer = block_on(commands::adopt_apply(handle, "wowf".into(), true, None));
            assert_eq!(last(&answer.messages), ("error", no_root));
        },
    );
}

#[test]
fn without_a_home_folder_the_engine_and_the_preferences_say_so() {
    let game = Game::new("no-home").with_questie();
    let mut env = env(&[("MRGL_ROOT", &game.root())]);
    // Neither `MRGL_HOME` nor `LOCALAPPDATA`: no mrgl home folder at all.
    env.remove.push("LOCALAPPDATA");
    in_child(
        "without_a_home_folder_the_engine_and_the_preferences_say_so",
        env,
        || {
            let app = app();
            let handle = app.handle().clone();
            let window = window(&app);
            let answer = block_on(commands::flavors());
            assert_eq!(answer.message.kind, "banner");
            assert!(
                answer.flavors[0]
                    .warnings
                    .iter()
                    .any(|warning| warning.starts_with("no mrgl home folder")),
                "{:?}",
                answer.flavors[0].warnings
            );
            let (progress, _) = channel();
            let answer = block_on(commands::check_updates(handle, "wowf".into(), progress));
            assert_eq!(answer.flavor, None);
            assert_eq!(answer.messages[0].kind, "error");
            assert_eq!(
                answer.messages[0].plain,
                mrgl_core::Error::NoHome.to_string()
            );
            let error = commands::set_preferences(window, Some("dark".into()), None)
                .expect_err("nowhere to store");
            assert_eq!(
                error.plain,
                format!("preferences not saved: {}", mrgl_core::Error::NoHome)
            );
            let preferences = commands::preferences();
            assert_eq!(preferences.serious_source, "default");
        },
    );
}

#[test]
fn a_child_run_is_the_one_test_with_the_environment_given() {
    // The harness itself: the marker tells the child apart, and the child's own variables are there.
    let game = Game::new("harness");
    in_child(
        "a_child_run_is_the_one_test_with_the_environment_given",
        env(&[("MRGL_HOME", &game.home())]),
        || {
            assert_eq!(
                std::env::var_os("MRGL_HOME").as_deref(),
                Some(game.home().as_os_str())
            );
            assert_eq!(
                std::env::var_os("MRGL_KEY_STORE").as_deref(),
                Some(OsStr::new("off"))
            );
            assert_eq!(
                std::env::var_os("MRGL_DISCOVERY").as_deref(),
                Some(OsStr::new("off"))
            );
            assert_eq!(std::env::var_os("MRGL_ROOT"), None);
        },
    );
}
