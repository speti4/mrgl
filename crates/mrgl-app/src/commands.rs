//! The IPC commands the UI calls (`ui/data.js` behind `window.Data`). Each one is a thin wrapper over
//! `mrgl-core` and the plain functions of [`crate::list`], [`crate::actions`] and [`crate::toolbox`]; the text they return is already in the voice's
//! shape, so the UI prints and never composes a murloc line of its own. A command that reads the disk or the
//! network runs on a blocking thread, so the window stays responsive while it works.

use std::path::{Path, PathBuf};

use mrgl_core::VERSION;
use mrgl_core::discover::{self, GameRoot, RootOrigin};
use mrgl_core::doctor;
use mrgl_core::flavor;
use mrgl_core::home;
use mrgl_core::install::Engine;
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::http_from_env;
use mrgl_core::voice::{self as core_voice, Kind, Voice};
use serde::Serialize;
use tauri::Manager;
use tauri::ipc::Channel;
use tauri_plugin_dialog::DialogExt;

use crate::actions::{self, ActionAnswer, Plan, UpdateEvent, UpdateRun};
use crate::list::{self, FlavorsAnswer, ResolveAnswer};
use crate::toolbox::{self, ToolboxAnswer, ToolboxRun};

/// A message as the UI shows it: the kind (it picks the place, `ui/voice.js`), the murloc line (empty when
/// serious) and the plain line.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Message {
    pub kind: &'static str,
    pub murloc: String,
    pub plain: String,
}

impl Message {
    /// Splits the voice's text into its two parts; a serious voice gives an empty murloc line.
    pub fn new(voice: Voice, kind: Kind, plain: impl std::fmt::Display) -> Self {
        let plain = plain.to_string();
        let murloc = if voice.is_serious() {
            String::new()
        } else {
            kind.murloc().to_string()
        };
        Self {
            kind: kind.as_str(),
            murloc,
            plain,
        }
    }
}

/// The voice of the window: `MRGL_SERIOUS` drops the murloc line as it does in the CLI.
fn voice() -> Voice {
    Voice::from_env(false)
}

/// The mrgl home folder of this process, when there is one.
fn home_dir() -> Option<PathBuf> {
    home::home_dir().ok()
}

/// Runs blocking work off the async runtime's threads; a panic in it becomes a fatal message.
async fn blocking<T, F>(work: F, failed: impl FnOnce(Message) -> T) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(work).await {
        Ok(value) => value,
        Err(error) => failed(Message::new(
            voice(),
            Kind::Fatal,
            format!("the window's worker stopped: {error}"),
        )),
    }
}

/// The version the status bar shows, from the workspace `Cargo.toml`.
#[tauri::command]
pub fn app_version() -> &'static str {
    VERSION
}

/// The game root as every command resolves it (`MRGL_ROOT`, the remembered root, then the discovery), with
/// its flavors and their rows.
#[tauri::command]
pub async fn flavors() -> FlavorsAnswer {
    blocking(load_flavors, failed_answer).await
}

fn load_flavors() -> FlavorsAnswer {
    match discover::resolve_root_of_process(None) {
        Ok(resolution) => list::load(
            resolution.root,
            resolution.warnings,
            home_dir().as_deref(),
            voice(),
        ),
        Err(error) => failed_answer(Message::new(voice(), Kind::Error, error)),
    }
}

fn failed_answer(message: Message) -> FlavorsAnswer {
    FlavorsAnswer {
        root: None,
        root_origin: None,
        flavors: Vec::new(),
        message,
    }
}

/// What choosing the game folder answers: nothing when the dialog was cancelled, the flavors of the saved
/// root, or the message that says why the folder is not one.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RootChoice {
    pub flavors: Option<FlavorsAnswer>,
    pub message: Option<Message>,
}

/// The folder dialog, then the chosen game root remembered in `config.json` as `mrgl root <folder>` does.
#[tauri::command]
pub async fn choose_root(app: tauri::AppHandle) -> RootChoice {
    blocking(
        move || {
            let picked = app
                .dialog()
                .file()
                .set_title("Choose the World of Warcraft folder")
                .blocking_pick_folder();
            let Some(picked) = picked.and_then(|path| path.into_path().ok()) else {
                return RootChoice {
                    flavors: None,
                    message: None,
                };
            };
            let home = home_dir();
            let config = home.as_ref().map(|home| home.join(home::CONFIG_FILE));
            choose(&picked, config.as_deref(), home.as_deref(), voice())
        },
        |message| RootChoice {
            flavors: None,
            message: Some(message),
        },
    )
    .await
}

/// The game root a chosen folder names: the folder when it holds flavor folders, its parent when the
/// folder is a flavor folder itself; `None` otherwise.
pub fn game_root_of(chosen: &Path) -> Option<PathBuf> {
    let chosen = std::path::absolute(chosen).unwrap_or_else(|_| chosen.to_path_buf());
    let has_flavors =
        |path: &Path| flavor::list_flavors(path).is_ok_and(|flavors| !flavors.is_empty());
    if has_flavors(&chosen) {
        return Some(chosen);
    }
    chosen
        .parent()
        .filter(|parent| has_flavors(parent))
        .map(Path::to_path_buf)
}

/// Takes a chosen folder: remembers its game root in the config (`None`: no home folder, nothing is
/// remembered) and answers with its flavors, or says why the folder is not a game folder.
pub fn choose(
    picked: &Path,
    config: Option<&Path>,
    home: Option<&Path>,
    voice: Voice,
) -> RootChoice {
    let refused = |plain: String| RootChoice {
        flavors: None,
        message: Some(Message::new(voice, Kind::Error, plain)),
    };
    let Some(root) = game_root_of(picked) else {
        return refused(
            mrgl_core::Error::NotAGameRoot {
                path: picked.to_path_buf(),
            }
            .to_string(),
        );
    };
    let saved = match config {
        Some(config) => {
            let mut loaded = match home::read_config_at(config) {
                Ok(loaded) => loaded.config,
                Err(error) => return refused(error.to_string()),
            };
            loaded.root = Some(root.clone());
            if let Err(error) = home::write_config_at(config, &loaded) {
                return refused(error.to_string());
            }
            true
        }
        None => false,
    };
    let mut answer = list::load(
        Some(GameRoot {
            path: root.clone(),
            origin: RootOrigin::Remembered,
        }),
        Vec::new(),
        home,
        voice,
    );
    if answer.message.kind == Kind::Banner.as_str() {
        let count: usize = answer
            .flavors
            .iter()
            .map(|flavor| flavor.addons.len())
            .sum();
        let names: Vec<&str> = answer
            .flavors
            .iter()
            .map(|flavor| flavor.name.as_str())
            .collect();
        let plain = if saved {
            format!(
                "game root saved: {} (flavors: {}), {}",
                root.display(),
                names.join(", "),
                list::count_noun(count, "addon")
            )
        } else {
            format!(
                "game root chosen for this run, not remembered (no mrgl home folder): {} (flavors: {}), {}",
                root.display(),
                names.join(", "),
                list::count_noun(count, "addon")
            )
        };
        answer.message = Message::new(
            voice,
            if saved { Kind::Success } else { Kind::Warning },
            plain,
        );
    }
    RootChoice {
        flavors: Some(answer),
        message: None,
    }
}

/// The diagnostic report for "copy diagnostics": the text `mrgl doctor` prints (task 024), the message
/// once it is on the clipboard, and the message when the clipboard refuses it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostics {
    pub text: String,
    pub copied: Message,
    pub failed: Message,
}

/// The doctor's report of this process (one request for the GitHub rate limit, the AddOns write probe).
#[tauri::command]
pub async fn diagnostics() -> Diagnostics {
    blocking(
        || diagnostics_of(&doctor::report_of_process(None, None), voice()),
        |message| Diagnostics {
            text: String::new(),
            copied: message.clone(),
            failed: message,
        },
    )
    .await
}

/// The text and the two messages of a report.
pub fn diagnostics_of(report: &doctor::Report, voice: Voice) -> Diagnostics {
    let concerns = report.concerns();
    let flavors = match report.flavors.len() {
        0 => "no flavor".to_string(),
        1 => "1 flavor".to_string(),
        count => format!("{count} flavors"),
    };
    let (kind, look) = match concerns.len() {
        0 => (Kind::Success, "nothing to look at".to_string()),
        1 => (
            Kind::Warning,
            format!("1 thing to look at ({})", concerns[0]),
        ),
        count => (
            Kind::Warning,
            format!("{count} things to look at ({})", concerns.join(", ")),
        ),
    };
    Diagnostics {
        text: report.text(),
        copied: Message::new(
            voice,
            kind,
            format!("diagnostics of {flavors} copied, {look}; paste them into a bug report"),
        ),
        failed: Message::new(
            voice,
            Kind::Error,
            "the clipboard took no text; run `mrgl doctor` and copy its lines instead",
        ),
    }
}

/// Resolves a managed addon through its source (the network, or `MRGL_HTTP_STUB`): the source's own `Pick`,
/// the handover to the update actions of task 010.
#[tauri::command]
pub async fn resolve(flavor: String, folder: String) -> ResolveAnswer {
    blocking(
        move || {
            let root = match discover::resolve_root_of_process(None) {
                Ok(resolution) => resolution.root,
                Err(error) => {
                    return ResolveAnswer {
                        pick: None,
                        message: Message::new(voice(), Kind::Error, error),
                    };
                }
            };
            let Some(flavor_folder) =
                root.and_then(|root| list::flavor_folder(&root.path, &flavor))
            else {
                return ResolveAnswer {
                    pick: None,
                    message: Message::new(
                        voice(),
                        Kind::Error,
                        format!(
                            "no flavor {flavor}: no game root, or no such flavor folder under it"
                        ),
                    ),
                };
            };
            let github = GitHub::from_env(http_from_env());
            list::resolve(
                &github,
                &flavor_folder,
                &folder,
                home_dir().as_deref(),
                voice(),
            )
        },
        |message| ResolveAnswer {
            pick: None,
            message,
        },
    )
    .await
}

/// The window's preferences (task 034): the theme and the serious switch as they apply, which source
/// decided the switch (`environment` means `MRGL_SERIOUS`, which the window's switch cannot override),
/// and what `config.json` stores, so the view can carry an old web view value over once.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PreferencesAnswer {
    pub theme: String,
    pub serious: bool,
    pub serious_source: &'static str,
    pub stored: StoredPreferences,
}

/// The preferences as the config file has them; `None` is a key it does not set.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StoredPreferences {
    pub theme: Option<String>,
    pub serious: Option<bool>,
}

/// The answer from `MRGL_SERIOUS` and the stored preferences; the window has no `--serious` flag.
pub fn preferences_answer(
    environment: Option<&std::ffi::OsStr>,
    stored: home::Preferences,
) -> PreferencesAnswer {
    let (serious, source) = core_voice::serious_from(false, environment, || stored.serious);
    PreferencesAnswer {
        theme: stored.theme.clone().unwrap_or_else(|| "auto".to_string()),
        serious,
        serious_source: source.as_str(),
        stored: StoredPreferences {
            theme: stored.theme,
            serious: stored.serious,
        },
    }
}

/// The theme the native window takes for a theme preference (task 079), so the title bar matches the
/// page: `light` and `dark` set it, `auto` (and anything else) leaves the system's, `None`.
pub fn native_theme(theme: &str) -> Option<tauri::Theme> {
    match theme {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    }
}

/// Gives the window the native theme of a theme preference, at start and on every change of it. A theme
/// the platform cannot set is no error: the title bar keeps the system's, the page is drawn all the same.
pub fn apply_native_theme(window: &tauri::WebviewWindow, theme: &str) {
    let _ = window.set_theme(native_theme(theme));
}

/// The preferences of this process.
#[tauri::command]
pub fn preferences() -> PreferencesAnswer {
    preferences_answer(
        std::env::var_os(core_voice::SERIOUS_ENV).as_deref(),
        home::read_preferences(),
    )
}

/// Stores the given preferences in `config.json` (a missing one stays as it is) and answers with them as
/// they apply; the error is the message that says why nothing was stored. The window takes the theme as
/// it applies ([`apply_native_theme`], task 079).
#[tauri::command]
pub fn set_preferences(
    window: tauri::WebviewWindow,
    theme: Option<String>,
    serious: Option<bool>,
) -> Result<PreferencesAnswer, Message> {
    let stored = home::config_path()
        .and_then(|path| home::update_preferences_at(&path, theme.as_deref(), serious))
        .map_err(|error| {
            Message::new(
                voice(),
                Kind::Error,
                format!("preferences not saved: {error}"),
            )
        })?;
    let answer = preferences_answer(std::env::var_os(core_voice::SERIOUS_ENV).as_deref(), stored);
    apply_native_theme(&window, &answer.theme);
    Ok(answer)
}

/// The engine of a flavor of this process's game root, or the message that says why there is none.
fn engine_of(flavor: &str) -> Result<Engine, Message> {
    let refused = |plain: String| Message::new(voice(), Kind::Error, plain);
    let root = discover::resolve_root_of_process(None)
        .map_err(|error| refused(error.to_string()))?
        .root;
    let folder = root
        .and_then(|root| list::flavor_folder(&root.path, flavor))
        .ok_or_else(|| {
            refused(format!(
                "no flavor {flavor}: no game root, or no such flavor folder under it"
            ))
        })?;
    let flavor = flavor::read_flavor(&folder).map_err(|error| refused(error.to_string()))?;
    Engine::new(flavor).map_err(|error| refused(error.to_string()))
}

/// A run of the update actions (task 010) on a blocking thread: one at a time ([`UpdateRun`]), each
/// progress event sent on the channel as it happens, the answer at the end.
async fn update_run(
    app: tauri::AppHandle,
    flavor: String,
    plan: Plan,
    progress: Channel<UpdateEvent>,
) -> ActionAnswer {
    blocking(
        move || {
            // A cleanup reads the AddOns folders an install swaps; it is quick, so the run waits for it.
            if app.state::<ToolboxRun>().is_running() {
                return ActionAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    toolbox::toolbox_busy_line(),
                ));
            }
            let run = app.state::<UpdateRun>();
            let Some(_guard) = run.start() else {
                return ActionAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    actions::busy_line(),
                ));
            };
            let engine = match engine_of(&flavor) {
                Ok(engine) => engine,
                Err(message) => return ActionAnswer::refused(message),
            };
            let github = GitHub::from_env(http_from_env());
            actions::run(
                &engine,
                &github,
                &plan,
                &|| run.cancelled(),
                home_dir().as_deref(),
                voice(),
                &mut |event| {
                    // A closed web view takes no more events; the run goes on to its end.
                    let _ = progress.send(event);
                },
            )
        },
        ActionAnswer::refused,
    )
    .await
}

/// "Check for updates" of a flavor (task 010): every managed addon against its source, nothing installed.
#[tauri::command]
pub async fn check_updates(
    app: tauri::AppHandle,
    flavor: String,
    progress: Channel<UpdateEvent>,
) -> ActionAnswer {
    update_run(app, flavor, Plan::Check, progress).await
}

/// "Update all" of a flavor (task 010): the check, then every update it found.
#[tauri::command]
pub async fn update_all(
    app: tauri::AppHandle,
    flavor: String,
    progress: Channel<UpdateEvent>,
) -> ActionAnswer {
    update_run(app, flavor, Plan::All, progress).await
}

/// A row's "Update" (task 010): the check of that addon, then its update when it has one.
#[tauri::command]
pub async fn update_one(
    app: tauri::AppHandle,
    flavor: String,
    folder: String,
    progress: Channel<UpdateEvent>,
) -> ActionAnswer {
    update_run(app, flavor, Plan::One(folder), progress).await
}

/// Cancels the running update before its next addon; the addon in flight finishes first.
#[tauri::command]
pub fn cancel_update(run: tauri::State<'_, UpdateRun>) -> Message {
    cancel_message(run.cancel(), voice())
}

/// The answer of a cancel: whether a run was there to stop.
pub fn cancel_message(running: bool, voice: Voice) -> Message {
    if running {
        Message::new(
            voice,
            Kind::Progress,
            "cancelling: the addon in flight finishes, no other one starts",
        )
    } else {
        Message::new(
            voice,
            Kind::Nothing,
            "no update is running, nothing to cancel",
        )
    }
}

/// A row menu's mark (task 010 over task 021): `pin`, `unpin`, `ignore`, `unignore` or `channel`, with
/// the version to pin at (none: the installed one) or the channel as `value`. Refused while an update
/// runs, since both write the flavor's sidecar.
#[tauri::command]
pub async fn mark(
    app: tauri::AppHandle,
    flavor: String,
    folder: String,
    action: String,
    value: Option<String>,
) -> ActionAnswer {
    blocking(
        move || {
            if app.state::<UpdateRun>().is_running() {
                return ActionAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    "an update is running; pin, ignore and the channel wait until it ends",
                ));
            }
            let mark = match actions::mark_of(&action, value.as_deref()) {
                Ok(mark) => mark,
                Err(plain) => {
                    return ActionAnswer::refused(Message::new(voice(), Kind::Error, plain));
                }
            };
            match engine_of(&flavor) {
                Ok(engine) => {
                    actions::mark(&engine, &folder, &mark, home_dir().as_deref(), voice())
                }
                Err(message) => ActionAnswer::refused(message),
            }
        },
        ActionAnswer::refused,
    )
    .await
}

/// The game root of this process, or the message that says why there is none.
fn root_of_process(flavor: &str) -> Result<PathBuf, Message> {
    let refused = |plain: String| Message::new(voice(), Kind::Error, plain);
    discover::resolve_root_of_process(None)
        .map_err(|error| refused(error.to_string()))?
        .root
        .map(|root| root.path)
        .ok_or_else(|| {
            refused(format!(
                "no flavor {flavor}: no game root; choose the game folder"
            ))
        })
}

/// The toolbox of a flavor (task 074): its sets with their addons, its characters, its SavedVariables with
/// their sizes and orphans, and whether its client runs. Reads only.
#[tauri::command]
pub async fn toolbox(flavor: String) -> ToolboxAnswer {
    blocking(
        move || match root_of_process(&flavor) {
            Ok(root) => toolbox::load(&root, &flavor, home_dir().as_deref(), voice()),
            Err(message) => ToolboxAnswer::refused(message),
        },
        ToolboxAnswer::refused,
    )
    .await
}

/// Applies a stored set to every character of a flavor, or to the one `character` (and `realm`) names
/// (task 074 over tasks 014 and 027); refused while the client runs (task 023) and while another toolbox
/// write goes.
#[tauri::command]
pub async fn apply_set(
    app: tauri::AppHandle,
    flavor: String,
    set: String,
    character: Option<String>,
    realm: Option<String>,
) -> ToolboxAnswer {
    blocking(
        move || {
            let run = app.state::<ToolboxRun>();
            let Some(_guard) = run.start() else {
                return ToolboxAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    toolbox::busy_line(),
                ));
            };
            match root_of_process(&flavor) {
                Ok(root) => toolbox::apply(
                    &root,
                    &flavor,
                    &set,
                    character.as_deref(),
                    realm.as_deref(),
                    home_dir().as_deref(),
                    voice(),
                ),
                Err(message) => ToolboxAnswer::refused(message),
            }
        },
        ToolboxAnswer::refused,
    )
    .await
}

/// Moves a flavor's orphan SavedVariables into the dated zip of its backups folder (task 074 over task
/// 025); refused while the client runs (task 023), while an update runs (an install swaps addon folders,
/// which the orphan rule reads) and while another toolbox write goes.
#[tauri::command]
pub async fn clean_saved_variables(app: tauri::AppHandle, flavor: String) -> ToolboxAnswer {
    blocking(
        move || {
            if app.state::<UpdateRun>().is_running() {
                return ToolboxAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    toolbox::updating_line(),
                ));
            }
            let run = app.state::<ToolboxRun>();
            let Some(_guard) = run.start() else {
                return ToolboxAnswer::refused(Message::new(
                    voice(),
                    Kind::Warning,
                    toolbox::busy_line(),
                ));
            };
            match root_of_process(&flavor) {
                Ok(root) => toolbox::clean(&root, &flavor, home_dir().as_deref(), voice()),
                Err(message) => ToolboxAnswer::refused(message),
            }
        },
        ToolboxAnswer::refused,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_carries_its_kind_and_both_lines() {
        let message = Message::new(Voice::new(false), Kind::Success, "2 addons updated");
        assert_eq!(message.kind, "success");
        assert_eq!(message.murloc, Kind::Success.murloc());
        assert_eq!(message.plain, "2 addons updated");
    }

    #[test]
    fn a_serious_message_has_an_empty_murloc_line() {
        let message = Message::new(Voice::new(true), Kind::Error, "addon not found");
        assert_eq!(message.kind, "error");
        assert_eq!(message.murloc, "");
        assert_eq!(message.plain, "addon not found");
    }

    #[test]
    fn a_message_serializes_in_the_contract_shape() {
        let message = Message::new(Voice::new(false), Kind::Banner, "4 flavors");
        let value = serde_json::to_value(&message).expect("serializes");
        assert_eq!(
            value,
            serde_json::json!({ "kind": "banner", "murloc": "Mrglglgl.", "plain": "4 flavors" })
        );
    }

    #[test]
    fn the_stored_preferences_apply_unless_the_variable_says() {
        use std::ffi::OsStr;
        let stored = home::Preferences {
            theme: Some("dark".into()),
            serious: Some(true),
        };
        // A cleared web view asks again and gets the file's values.
        let answer = preferences_answer(None, stored.clone());
        assert_eq!(answer.theme, "dark");
        assert!(answer.serious);
        assert_eq!(answer.serious_source, "file");
        assert_eq!(answer.stored.serious, Some(true));
        let answer = preferences_answer(Some(OsStr::new("0")), stored);
        assert!(!answer.serious, "MRGL_SERIOUS=0 wins over the file");
        assert_eq!(answer.serious_source, "environment");
        let answer = preferences_answer(None, home::Preferences::default());
        assert_eq!(answer.theme, "auto");
        assert!(!answer.serious);
        assert_eq!(answer.serious_source, "default");
        assert_eq!(answer.stored.theme, None);
    }

    #[test]
    fn the_window_takes_the_theme_the_preference_applies() {
        // Task 079: the native title bar follows the page's theme, `auto` leaves the system's.
        let theme_of = |stored: Option<&str>| {
            let answer = preferences_answer(
                None,
                home::Preferences {
                    theme: stored.map(str::to_string),
                    serious: None,
                },
            );
            native_theme(&answer.theme)
        };
        assert_eq!(theme_of(Some("light")), Some(tauri::Theme::Light));
        assert_eq!(theme_of(Some("dark")), Some(tauri::Theme::Dark));
        assert_eq!(theme_of(Some("auto")), None);
        assert_eq!(theme_of(None), None, "no stored theme is auto");
        assert_eq!(native_theme("sepia"), None, "an unknown theme is auto");
    }

    #[test]
    fn a_cancel_says_whether_a_run_was_there() {
        let run = UpdateRun::default();
        assert!(!run.cancel(), "nothing runs");
        let message = cancel_message(run.cancel(), Voice::new(true));
        assert_eq!(
            (message.kind, message.plain.as_str()),
            ("nothing", "no update is running, nothing to cancel")
        );
        let guard = run.start().expect("the first run starts");
        assert!(run.start().is_none(), "one run at a time");
        assert!(run.cancel() && run.cancelled());
        let message = cancel_message(true, Voice::new(true));
        assert_eq!(message.kind, "progress");
        drop(guard);
        assert!(
            !run.is_running() && !run.cancelled(),
            "the end of a run clears the cancel"
        );
        assert!(run.start().is_some(), "the next run starts");
    }
}
