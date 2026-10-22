//! The mrgl home folder (D-003): `%LOCALAPPDATA%\mrgl\`, where the state sidecars of task 004 and the
//! `config.json` of the command line live. `MRGL_HOME` overrides it, so a test points it at a temporary
//! folder and never touches the real one. The config remembers the game folder (`mrgl root <folder>`); a
//! missing config is no root, a corrupt one is a warning and no root, never an error the user has to fix.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The environment variable that overrides the home folder.
pub const HOME_ENV: &str = "MRGL_HOME";
/// The config file's name under the home folder.
pub const CONFIG_FILE: &str = "config.json";

/// The home folder from the two environment values: a non-empty `MRGL_HOME`, else `LOCALAPPDATA\mrgl`.
pub fn home_dir_from(mrgl_home: Option<&OsStr>, local_app_data: Option<&OsStr>) -> Option<PathBuf> {
    if let Some(home) = mrgl_home.filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(home));
    }
    local_app_data
        .filter(|value| !value.is_empty())
        .map(|local| Path::new(local).join("mrgl"))
}

/// The home folder of this process.
pub fn home_dir() -> Result<PathBuf> {
    home_dir_from(
        std::env::var_os(HOME_ENV).as_deref(),
        std::env::var_os("LOCALAPPDATA").as_deref(),
    )
    .ok_or(Error::NoHome)
}

/// The config file's path.
pub fn config_path() -> Result<PathBuf> {
    Ok(home_dir()?.join(CONFIG_FILE))
}

/// What the command line remembers between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The World of Warcraft folder, the one with the flavor folders in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
}

/// A config as read: a corrupt file reads as the default with a warning.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: Config,
    pub warning: Option<String>,
}

/// Reads a config file: a missing file is the default, a corrupt one the default with a warning.
pub fn read_config_at(path: &Path) -> Result<LoadedConfig> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LoadedConfig::default());
        }
        Err(source) => {
            return Err(Error::Io {
                context: "reading the config",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    match serde_json::from_str(&text) {
        Ok(config) => Ok(LoadedConfig {
            config,
            warning: None,
        }),
        Err(reason) => Ok(LoadedConfig {
            config: Config::default(),
            warning: Some(format!("unreadable config: {}: {reason}", path.display())),
        }),
    }
}

/// Writes a config file, creating its folder. The file is shared (the remembered root, the window's
/// preferences of task 034): the keys [`Config`] owns are set or removed, every other key of the file is
/// kept as it was.
pub fn write_config_at(path: &Path, config: &Config) -> Result<()> {
    let mut object = read_object_at(path)?;
    let root = serde_json::to_value(&config.root).unwrap_or(serde_json::Value::Null);
    set_or_remove(&mut object, "root", root);
    write_object_at(path, &object)
}

/// The config file as a JSON object: empty for a missing file and for one that is not a JSON object (its
/// reader warned already, and the next write starts it over).
fn read_object_at(path: &Path) -> Result<serde_json::Map<String, serde_json::Value>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(match serde_json::from_str(&text) {
            Ok(serde_json::Value::Object(object)) => object,
            _ => serde_json::Map::new(),
        }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(source) => Err(Error::Io {
            context: "reading the config",
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// A key set to a value, or removed for `null`, so the file names only what is set.
fn set_or_remove(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    if value.is_null() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), value);
    }
}

/// Writes the config object, creating its folder.
fn write_object_at(path: &Path, object: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            context: "creating the home folder",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let text = serde_json::to_string_pretty(object).map_err(|source| Error::Io {
        context: "writing the config",
        path: path.to_path_buf(),
        source: std::io::Error::other(source),
    })?;
    std::fs::write(path, text + "\n").map_err(|source| Error::Io {
        context: "writing the config",
        path: path.to_path_buf(),
        source,
    })
}

/// Reads this process's config.
pub fn read_config() -> Result<LoadedConfig> {
    read_config_at(&config_path()?)
}

/// Writes this process's config.
pub fn write_config(config: &Config) -> Result<()> {
    write_config_at(&config_path()?, config)
}

/// The themes of the window (task 034): `auto` follows the system, `light` and `dark` override it.
pub const THEMES: [&str; 3] = ["auto", "light", "dark"];

/// The window's preferences (task 034), kept in `config.json` next to the remembered root so they outlive
/// the web view's storage and the command line reads the same serious switch: `theme` (one of
/// [`THEMES`]) and `serious`. `None` is a key the file does not set; a value that is not one of these
/// reads as not set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preferences {
    pub theme: Option<String>,
    pub serious: Option<bool>,
}

/// Reads the preferences of a config file; a missing or unreadable file, or a corrupt one, sets none.
pub fn read_preferences_at(path: &Path) -> Preferences {
    let object = read_object_at(path).unwrap_or_default();
    Preferences {
        theme: object
            .get("theme")
            .and_then(serde_json::Value::as_str)
            .filter(|theme| THEMES.contains(theme))
            .map(str::to_string),
        serious: object.get("serious").and_then(serde_json::Value::as_bool),
    }
}

/// Sets the given preferences in a config file (`None` leaves a key as it is), keeping every other key,
/// and returns the preferences as the file has them now. A theme that is not one of [`THEMES`] is refused.
pub fn update_preferences_at(
    path: &Path,
    theme: Option<&str>,
    serious: Option<bool>,
) -> Result<Preferences> {
    if let Some(theme) = theme.filter(|theme| !THEMES.contains(theme)) {
        return Err(Error::Io {
            context: "writing the config",
            path: path.to_path_buf(),
            source: std::io::Error::other(format!("unknown theme {theme}: auto, light or dark")),
        });
    }
    let mut object = read_object_at(path)?;
    if let Some(theme) = theme {
        object.insert("theme".into(), theme.into());
    }
    if let Some(serious) = serious {
        object.insert("serious".into(), serious.into());
    }
    write_object_at(path, &object)?;
    Ok(read_preferences_at(path))
}

/// Reads this process's preferences; no home folder sets none.
pub fn read_preferences() -> Preferences {
    config_path()
        .map(|path| read_preferences_at(&path))
        .unwrap_or_default()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn temp_home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-home-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn mrgl_home_wins_over_localappdata() {
        assert_eq!(
            home_dir_from(Some(OsStr::new("D:\\h")), Some(OsStr::new("C:\\l"))),
            Some(PathBuf::from("D:\\h"))
        );
        assert_eq!(
            home_dir_from(None, Some(OsStr::new("C:\\l"))),
            Some(Path::new("C:\\l").join("mrgl"))
        );
        assert_eq!(home_dir_from(None, None), None);
    }

    #[test]
    fn an_empty_mrgl_home_does_not_count() {
        assert_eq!(
            home_dir_from(Some(OsStr::new("")), Some(OsStr::new("C:\\l"))),
            Some(Path::new("C:\\l").join("mrgl"))
        );
        assert_eq!(
            home_dir_from(Some(OsStr::new("")), Some(OsStr::new(""))),
            None
        );
    }

    #[test]
    fn a_missing_config_is_no_root() {
        let loaded = read_config_at(&temp_home("missing").join(CONFIG_FILE)).expect("reads");
        assert_eq!(loaded, LoadedConfig::default());
        assert_eq!(loaded.config.root, None);
    }

    #[test]
    fn a_corrupt_config_is_a_warning_and_no_root() {
        let home = temp_home("corrupt");
        let path = home.join(CONFIG_FILE);
        std::fs::create_dir_all(&home).expect("home");
        std::fs::write(&path, "{ not json").expect("write");
        let loaded = read_config_at(&path).expect("reads");
        assert_eq!(loaded.config.root, None);
        let warning = loaded.warning.expect("a warning");
        assert!(warning.starts_with("unreadable config: "), "{warning}");
        assert!(warning.contains(CONFIG_FILE), "{warning}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_config_round_trips() {
        let home = temp_home("roundtrip");
        let path = home.join("nested").join(CONFIG_FILE);
        let config = Config {
            root: Some(PathBuf::from("C:\\games\\World of Warcraft")),
        };
        write_config_at(&path, &config).expect("writes, creating the folders");
        let text = std::fs::read_to_string(&path).expect("the file");
        assert!(text.contains("\"root\""), "{text}");
        let loaded = read_config_at(&path).expect("reads");
        assert_eq!(loaded.config, config);
        assert_eq!(loaded.warning, None);
        write_config_at(&path, &Config::default()).expect("writes");
        assert!(
            !std::fs::read_to_string(&path)
                .expect("the file")
                .contains("root")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_preferences_round_trip_next_to_the_root() {
        let home = temp_home("preferences");
        let path = home.join(CONFIG_FILE);
        assert_eq!(read_preferences_at(&path), Preferences::default());
        let config = Config {
            root: Some(PathBuf::from("D:\\Games\\World of Warcraft")),
        };
        write_config_at(&path, &config).expect("the root");
        let set = update_preferences_at(&path, Some("dark"), Some(true)).expect("writes");
        assert_eq!(
            set,
            Preferences {
                theme: Some("dark".into()),
                serious: Some(true),
            }
        );
        assert_eq!(
            read_config_at(&path).expect("reads").config,
            config,
            "the root stays"
        );
        let set = update_preferences_at(&path, None, Some(false)).expect("writes");
        assert_eq!(set.theme.as_deref(), Some("dark"), "a key not given stays");
        assert_eq!(set.serious, Some(false));
        // `mrgl root` writes the root and keeps the preferences.
        write_config_at(&path, &Config::default()).expect("writes");
        assert_eq!(read_preferences_at(&path), set);
        assert_eq!(read_config_at(&path).expect("reads").config.root, None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unknown_theme_is_refused_and_an_odd_value_reads_as_not_set() {
        let home = temp_home("odd-preferences");
        let path = home.join(CONFIG_FILE);
        let error = update_preferences_at(&path, Some("blue"), None).expect_err("refused");
        assert!(error.to_string().contains("unknown theme blue"), "{error}");
        assert!(!path.exists(), "nothing written");
        std::fs::create_dir_all(&home).expect("home");
        std::fs::write(
            &path,
            r#"{ "theme": "blue", "serious": "yes", "other": 1 }"#,
        )
        .expect("write");
        assert_eq!(read_preferences_at(&path), Preferences::default());
        update_preferences_at(&path, Some("light"), None).expect("writes");
        let text = std::fs::read_to_string(&path).expect("the file");
        assert!(
            text.contains("\"other\": 1"),
            "an unknown key stays: {text}"
        );
        std::fs::write(&path, "{ not json").expect("write");
        assert_eq!(read_preferences_at(&path), Preferences::default());
        let _ = std::fs::remove_dir_all(&home);
    }
}
