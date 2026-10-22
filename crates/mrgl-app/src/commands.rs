//! The IPC commands the UI calls (`ui/data.js` behind `window.Data`). Each one is a thin wrapper over
//! `mrgl-core` and the plain functions of [`crate::list`]; the text they return is already in the voice's
//! shape, so the UI prints and never composes a murloc line of its own. A command that reads the disk or the
//! network runs on a blocking thread, so the window stays responsive while it works.

use std::path::{Path, PathBuf};

use mrgl_core::VERSION;
use mrgl_core::discover::{self, GameRoot, RootOrigin};
use mrgl_core::doctor;
use mrgl_core::flavor;
use mrgl_core::home;
use mrgl_core::sources::github::GitHub;
use mrgl_core::sources::http_from_env;
use mrgl_core::voice::{self as core_voice, Kind, Voice};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

use crate::list::{self, FlavorsAnswer, ResolveAnswer};

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

/// The preferences of this process.
#[tauri::command]
pub fn preferences() -> PreferencesAnswer {
    preferences_answer(
        std::env::var_os(core_voice::SERIOUS_ENV).as_deref(),
        home::read_preferences(),
    )
}

/// Stores the given preferences in `config.json` (a missing one stays as it is) and answers with them as
/// they apply; the error is the message that says why nothing was stored.
#[tauri::command]
pub fn set_preferences(
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
    Ok(preferences_answer(
        std::env::var_os(core_voice::SERIOUS_ENV).as_deref(),
        stored,
    ))
}

/// "Update all" of a flavor: the update actions are task 010; until then the answer says so, in the voice.
#[tauri::command]
pub fn update_all(flavor: String) -> Vec<Message> {
    let _ = flavor;
    vec![Message::new(
        voice(),
        Kind::Warning,
        mrgl_core::Error::NotImplemented {
            command: "update all in the window",
            task: "010",
        },
    )]
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
    fn update_all_answers_with_its_task() {
        let messages = update_all("wowf".to_string());
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].kind, "warning");
        assert!(
            messages[0].plain.contains("task 010"),
            "{}",
            messages[0].plain
        );
    }
}
