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

/// Writes a config file, creating its folder.
pub fn write_config_at(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            context: "creating the home folder",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|source| Error::Io {
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

#[cfg(test)]
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
}
