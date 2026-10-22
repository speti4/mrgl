//! Where the game folder comes from (task 020). A run names it with `--root`, `MRGL_ROOT` or the root
//! that `mrgl root <folder>` remembered in `config.json` (task 002); when none of those does, the
//! discovery looks it up in the launcher's registry key (`HKLM\SOFTWARE\WOW6432Node\Blizzard
//! Entertainment\World of Warcraft`, value `InstallPath`), then in the game's Uninstall entry (task 043:
//! `InstallLocation` under [`UNINSTALL_KEYS`], which an install moved or restored by hand may carry
//! without the launcher's key), and then in the usual install folders (`Program Files (x86)\World of
//! Warcraft`, the same under `Program Files`). A registry value of the type `REG_EXPAND_SZ` is expanded
//! from the environment (`%ProgramFiles(x86)%\World of Warcraft`), a `REG_SZ` value is taken as written
//! ([`registry_path`]). A hit counts only when it is a game root ([`is_game_root`]: a `.build.info` and
//! at least one flavor folder) or when its parent is one (a registry value may name a flavor folder). A
//! discovered root is used for the run and reported with its origin, never written to `config.json`:
//! `mrgl root <folder>` and the window's picker (task 009) are the places that remember it.
//!
//! The lookup sits behind [`RootLookup`]: [`SystemLookup`] reads the registry and the environment,
//! [`FixedLookup`] is the stub for tests, and `MRGL_DISCOVERY` picks a stub for one run of an exe
//! ([`lookup_from_env`]), so an end-to-end test never reads the real registry. [`resolve_root_of_process`]
//! is the one call every command of the command line and the window's picker start from.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::flavor::{self, BUILD_INFO};
use crate::home;

/// The environment variable that names the game folder for one run.
pub const ROOT_ENV: &str = "MRGL_ROOT";
/// The environment variable that replaces the system lookup for one run (see [`lookup_from_env`]).
pub const DISCOVERY_ENV: &str = "MRGL_DISCOVERY";
/// The launcher's key under `HKEY_LOCAL_MACHINE`.
pub const LAUNCHER_KEY: &str = r"SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft";
/// The value under [`LAUNCHER_KEY`] that holds the install folder.
pub const LAUNCHER_VALUE: &str = "InstallPath";
/// The game's Uninstall entry, in the order it is read (task 043): the 32-bit view of
/// `HKEY_LOCAL_MACHINE`, where the installer writes it as it writes the launcher's key, the native view,
/// then the per-user entry.
pub const UNINSTALL_KEYS: [(Hive, &str); 3] = [
    (
        Hive::LocalMachine,
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft",
    ),
    (
        Hive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft",
    ),
    (
        Hive::CurrentUser,
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft",
    ),
];
/// The value under an [`UNINSTALL_KEYS`] key that holds the install folder.
pub const UNINSTALL_VALUE: &str = "InstallLocation";
/// The game folder's name under `Program Files (x86)` and `Program Files` on a standard install.
pub const COMMON_FOLDER: &str = "World of Warcraft";
/// The `MRGL_DISCOVERY` item that stands in for the WebView2 runtime's registry value (task 024).
pub const WEBVIEW2_ITEM: &str = "webview2=";
/// The `MRGL_DISCOVERY` item that stands in for the Windows version's registry values (task 051).
pub const WINDOWS_ITEM: &str = "windows=";

/// A registry hive [`registry_string`] reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    /// `HKEY_LOCAL_MACHINE`: what is installed for every user.
    LocalMachine,
    /// `HKEY_CURRENT_USER`: what is installed for this user.
    CurrentUser,
}

/// One string value of the registry, read the way the launcher's `InstallPath` is (open, query, close):
/// a `REG_SZ` or `REG_EXPAND_SZ` value as written, `None` for a missing key or value, another type or an
/// empty string, and always `None` on a target without a registry. The doctor reads the WebView2
/// runtime's version through it (task 024).
pub fn registry_string(hive: Hive, key: &str, value: &str) -> Option<OsString> {
    registry_text(hive, key, value).map(|value| value.text)
}

/// A string value as the registry holds it: its text as written, and whether it is a `REG_EXPAND_SZ`,
/// whose `%NAME%` references are meant to be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryText {
    pub text: OsString,
    pub expandable: bool,
}

/// One string value of the registry with its type (task 043): `None` as for [`registry_string`].
pub fn registry_text(hive: Hive, key: &str, value: &str) -> Option<RegistryText> {
    system::registry_text(hive, key, value)
}

/// A text with its `%NAME%` references expanded from this process's environment
/// (`ExpandEnvironmentStringsW`); a reference to a variable that is not set stays as written. `None` when
/// the call fails, and always on a target without it.
pub fn expand_environment(text: &OsStr) -> Option<OsString> {
    system::expand_environment(text)
}

/// A registry value as the path the discovery tries (task 043): a `REG_EXPAND_SZ` value expanded through
/// `expand` ([`expand_environment`], a stub in the tests), a `REG_SZ` value as written; `None` when the
/// expansion fails or leaves nothing.
pub fn registry_path(
    value: RegistryText,
    expand: impl Fn(&OsStr) -> Option<OsString>,
) -> Option<PathBuf> {
    let text = if value.expandable {
        expand(&value.text)?
    } else {
        value.text
    };
    (!text.is_empty()).then(|| PathBuf::from(text))
}

/// A registry value of this machine as a path: read, then expanded when it is a `REG_EXPAND_SZ`.
fn system_path(hive: Hive, key: &str, value: &str) -> Option<PathBuf> {
    registry_text(hive, key, value).and_then(|value| registry_path(value, expand_environment))
}

/// One `REG_DWORD` value of the registry, read the same way: `None` for a missing key or value, another
/// type, and always on a target without a registry. The doctor reads the Windows update build revision
/// (`UBR`) through it (task 051).
pub fn registry_dword(hive: Hive, key: &str, value: &str) -> Option<u32> {
    system::registry_dword(hive, key, value)
}

/// The places a game root may be found when nothing names it.
pub trait RootLookup {
    /// The launcher's `InstallPath` value, expanded when it is a `REG_EXPAND_SZ` ([`registry_path`]);
    /// `None` when the key or the value is missing.
    fn launcher_install_path(&self) -> Option<PathBuf>;
    /// The game's Uninstall entry's `InstallLocation` values in the order of [`UNINSTALL_KEYS`], each
    /// expanded the same way (task 043); none unless the lookup reads them.
    fn uninstall_locations(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    /// The usual install folders, in the order they are tried.
    fn common_folders(&self) -> Vec<PathBuf>;
}

/// The machine's registry and environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemLookup;

impl RootLookup for SystemLookup {
    fn launcher_install_path(&self) -> Option<PathBuf> {
        system_path(Hive::LocalMachine, LAUNCHER_KEY, LAUNCHER_VALUE)
    }

    fn uninstall_locations(&self) -> Vec<PathBuf> {
        UNINSTALL_KEYS
            .iter()
            .filter_map(|(hive, key)| system_path(*hive, key, UNINSTALL_VALUE))
            .collect()
    }

    fn common_folders(&self) -> Vec<PathBuf> {
        system::common_folders(&|name| std::env::var_os(name))
    }
}

/// A fixed lookup, the stub for tests: the registry values and the folders it is given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixedLookup {
    pub install_path: Option<PathBuf>,
    pub folders: Vec<PathBuf>,
    /// The Uninstall entry's values (task 043).
    pub uninstall_locations: Vec<PathBuf>,
}

impl FixedLookup {
    /// A lookup that finds nothing: no registry value, no folders.
    pub fn none() -> Self {
        Self::default()
    }
}

impl RootLookup for FixedLookup {
    fn launcher_install_path(&self) -> Option<PathBuf> {
        self.install_path.clone()
    }

    fn uninstall_locations(&self) -> Vec<PathBuf> {
        self.uninstall_locations.clone()
    }

    fn common_folders(&self) -> Vec<PathBuf> {
        self.folders.clone()
    }
}

/// Where a run's game root came from, in the order the places are tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootOrigin {
    /// The `--root` flag.
    Flag,
    /// `MRGL_ROOT`.
    Environment,
    /// The root `mrgl root <folder>` remembered in `config.json`.
    Remembered,
    /// The launcher's registry value.
    Registry,
    /// The game's Uninstall entry in the registry (task 043).
    UninstallEntry,
    /// One of the usual install folders.
    CommonFolder,
}

impl RootOrigin {
    /// The origin's name in machine-readable output (`--json`), fixed here so a renamed variant cannot
    /// change it.
    pub const fn as_str(self) -> &'static str {
        match self {
            RootOrigin::Flag => "flag",
            RootOrigin::Environment => "environment",
            RootOrigin::Remembered => "remembered",
            RootOrigin::Registry => "registry",
            RootOrigin::UninstallEntry => "uninstall_entry",
            RootOrigin::CommonFolder => "common_folder",
        }
    }

    /// The origin as a plain line names it.
    pub const fn description(self) -> &'static str {
        match self {
            RootOrigin::Flag => "--root",
            RootOrigin::Environment => "MRGL_ROOT",
            RootOrigin::Remembered => "remembered",
            RootOrigin::Registry => "the launcher's registry key",
            RootOrigin::UninstallEntry => "the game's Uninstall registry entry",
            RootOrigin::CommonFolder => "a usual install folder",
        }
    }

    /// Whether the discovery found the root, rather than the user naming it.
    pub const fn is_discovered(self) -> bool {
        matches!(
            self,
            RootOrigin::Registry | RootOrigin::UninstallEntry | RootOrigin::CommonFolder
        )
    }
}

impl Serialize for RootOrigin {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// A run's game root with its origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameRoot {
    pub path: PathBuf,
    pub origin: RootOrigin,
}

impl GameRoot {
    /// The plain line that reports a discovered root: where it was found and how to keep it.
    pub fn found_line(&self) -> String {
        format!(
            "game root found in {}: {}; run `mrgl root <folder>` to remember it",
            self.origin.description(),
            self.path.display()
        )
    }
}

/// Whether a folder is a game root: it holds a `.build.info` and at least one flavor folder.
pub fn is_game_root(path: &Path) -> bool {
    path.join(BUILD_INFO).is_file()
        && flavor::list_flavors(path).is_ok_and(|flavors| !flavors.is_empty())
}

/// The game root a candidate names: the candidate when it is a game root, its parent when that is one
/// (a registry value may name a flavor folder), `None` otherwise.
fn game_root_at(candidate: &Path) -> Option<PathBuf> {
    let candidate = std::path::absolute(candidate).unwrap_or_else(|_| candidate.to_path_buf());
    if is_game_root(&candidate) {
        return Some(candidate);
    }
    candidate
        .parent()
        .filter(|parent| is_game_root(parent))
        .map(Path::to_path_buf)
}

/// The first game root the lookup finds: the launcher's registry value, then the Uninstall entry's
/// values, then the usual folders in order; `None` when none of them is a game root.
pub fn discover_root(lookup: &dyn RootLookup) -> Option<GameRoot> {
    if let Some(path) = lookup
        .launcher_install_path()
        .as_deref()
        .and_then(game_root_at)
    {
        return Some(GameRoot {
            path,
            origin: RootOrigin::Registry,
        });
    }
    if let Some(path) = lookup
        .uninstall_locations()
        .iter()
        .find_map(|location| game_root_at(location))
    {
        return Some(GameRoot {
            path,
            origin: RootOrigin::UninstallEntry,
        });
    }
    lookup
        .common_folders()
        .iter()
        .find_map(|folder| game_root_at(folder))
        .map(|path| GameRoot {
            path,
            origin: RootOrigin::CommonFolder,
        })
}

/// The resolution order: the flag, then a non-empty `MRGL_ROOT` value, then the remembered root (read
/// only when needed), then the discovery. A root the user named is taken as given; only a discovered
/// one is checked to be a game root.
pub fn resolve_root(
    flag: Option<PathBuf>,
    environment: Option<&OsStr>,
    remembered: impl FnOnce() -> Option<PathBuf>,
    lookup: &dyn RootLookup,
) -> Option<GameRoot> {
    if let Some(path) = flag {
        return Some(GameRoot {
            path,
            origin: RootOrigin::Flag,
        });
    }
    if let Some(value) = environment.filter(|value| !value.is_empty()) {
        return Some(GameRoot {
            path: PathBuf::from(value),
            origin: RootOrigin::Environment,
        });
    }
    if let Some(path) = remembered() {
        return Some(GameRoot {
            path,
            origin: RootOrigin::Remembered,
        });
    }
    discover_root(lookup)
}

/// The lookup `MRGL_DISCOVERY` names: unset or empty is the system; `off` is a lookup that finds nothing;
/// anything else is a stub, `;`-separated, where `registry=<path>` is the launcher's value,
/// `uninstall=<path>` one of the Uninstall entry's (task 043), `registry-expand=<text>` and
/// `uninstall-expand=<text>` the same as a `REG_EXPAND_SZ` value, expanded from the environment as the
/// system's would be, a `webview2=<version>` or a `windows=<version>` item is the doctor's (tasks 024 and
/// 051, [`crate::doctor`]) and every other item (with or without `folder=`) is one of the usual folders,
/// in order. The stub keeps the end-to-end tests off the real registry and lets a user switch the
/// discovery off.
pub fn lookup_from_env(value: Option<&OsStr>) -> Box<dyn RootLookup> {
    match parse_lookup(value) {
        Some(fixed) => Box::new(fixed),
        None => Box::new(SystemLookup),
    }
}

/// The stub `MRGL_DISCOVERY` describes, or `None` for the system lookup.
fn parse_lookup(value: Option<&OsStr>) -> Option<FixedLookup> {
    let value = value.filter(|value| !value.is_empty())?;
    let text = value.to_string_lossy();
    if text.trim().eq_ignore_ascii_case("off") {
        return Some(FixedLookup::none());
    }
    let mut lookup = FixedLookup::none();
    for item in text
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let expanded = |text: &str| {
            registry_path(
                RegistryText {
                    text: OsString::from(text),
                    expandable: true,
                },
                expand_environment,
            )
        };
        if let Some(path) = item.strip_prefix("registry=") {
            lookup.install_path = Some(PathBuf::from(path));
        } else if let Some(text) = item.strip_prefix("registry-expand=") {
            lookup.install_path = expanded(text);
        } else if let Some(path) = item.strip_prefix("uninstall=") {
            lookup.uninstall_locations.push(PathBuf::from(path));
        } else if let Some(text) = item.strip_prefix("uninstall-expand=") {
            lookup.uninstall_locations.extend(expanded(text));
        } else if item.starts_with(WEBVIEW2_ITEM) || item.starts_with(WINDOWS_ITEM) {
            // The doctor's WebView2 and Windows stubs (tasks 024, 051), not a folder.
            continue;
        } else {
            let path = item.strip_prefix("folder=").unwrap_or(item);
            lookup.folders.push(PathBuf::from(path));
        }
    }
    Some(lookup)
}

/// A run's resolved root, with the warnings met on the way (a corrupt config).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    pub root: Option<GameRoot>,
    pub warnings: Vec<String>,
}

/// The game root of this process, the call every command and the window's picker start from: the flag,
/// `MRGL_ROOT`, the root remembered in the config (a corrupt config is a warning, no home folder is no
/// remembered root), then the discovery through the lookup `MRGL_DISCOVERY` names (the system by
/// default). The error is a config that cannot be read.
pub fn resolve_root_of_process(flag: Option<PathBuf>) -> Result<Resolution> {
    resolve_root_from(flag, &|name| std::env::var_os(name), home::read_config)
}

/// [`resolve_root_of_process`] over an environment reader and a config reader given explicitly:
/// `MRGL_ROOT` and `MRGL_DISCOVERY` come from `env`, the remembered root from `read_config` (read only
/// when the flag and the variable name nothing; [`Error::NoHome`] is no remembered root, its warning is
/// kept, any other error is the result).
pub fn resolve_root_from(
    flag: Option<PathBuf>,
    env: &dyn Fn(&str) -> Option<OsString>,
    read_config: impl FnOnce() -> Result<home::LoadedConfig>,
) -> Result<Resolution> {
    let mut warnings = Vec::new();
    let mut failure = None;
    let lookup = lookup_from_env(env(DISCOVERY_ENV).as_deref());
    let root = resolve_root(
        flag,
        env(ROOT_ENV).as_deref(),
        || match read_config() {
            Ok(loaded) => {
                warnings.extend(loaded.warning);
                loaded.config.root
            }
            Err(Error::NoHome) => None,
            Err(other) => {
                failure = Some(other);
                None
            }
        },
        lookup.as_ref(),
    );
    match failure {
        Some(error) => Err(error),
        None => Ok(Resolution { root, warnings }),
    }
}

#[cfg(windows)]
mod system {
    //! One `RegOpenKeyExW` on a key, one `RegQueryValueExW` for the size and one for the value, and the
    //! key is closed; a missing key or value, or a value of another type, is `None`. An expandable value
    //! goes through `ExpandEnvironmentStringsW`, once for the size and once for the text. The usual
    //! folders come from `ProgramFiles(x86)` and `ProgramFiles`, with the `C:` defaults when the
    //! variables are unset.

    use std::ffi::{OsStr, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_SZ,
        REG_VALUE_TYPE, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    use super::{COMMON_FOLDER, Hive, RegistryText};

    /// A registry key handle closed on drop.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful RegOpenKeyExW and is closed exactly once, here.
            unsafe { RegCloseKey(self.0) };
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        OsStr::new(text).encode_wide().chain([0]).collect()
    }

    fn hkey(hive: Hive) -> HKEY {
        match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        }
    }

    pub(super) fn registry_text(hive: Hive, key: &str, value: &str) -> Option<RegistryText> {
        string_value(hkey(hive), key, value)
    }

    pub(super) fn expand_environment(text: &OsStr) -> Option<OsString> {
        let source: Vec<u16> = text.encode_wide().chain([0]).collect();
        // The variables may change between the two calls; one more round covers a value that grew.
        let mut capacity = 0u32;
        for _ in 0..3 {
            let mut buffer = vec![0u16; capacity as usize];
            let destination = if buffer.is_empty() {
                std::ptr::null_mut()
            } else {
                buffer.as_mut_ptr()
            };
            // SAFETY: the source is NUL-terminated and the destination holds `capacity` UTF-16 units
            // (none with a null pointer, which asks for the size only).
            let needed =
                unsafe { ExpandEnvironmentStringsW(source.as_ptr(), destination, capacity) };
            // coverage: os-begin — the call's own failure
            if needed == 0 {
                return None;
                // coverage: os-end
            }
            if needed <= capacity {
                // `needed` counts the terminating NUL.
                buffer.truncate(needed as usize - 1);
                return Some(OsString::from_wide(&buffer));
            }
            capacity = needed;
        }
        None // coverage: os — a value that grew on every round
    }

    pub(super) fn registry_dword(hive: Hive, key: &str, value: &str) -> Option<u32> {
        let key = open(hkey(hive), key)?;
        let value = wide(value);
        let mut kind: REG_VALUE_TYPE = 0;
        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: the buffer is the four bytes of `data` and `size` says so; a longer value fails with
        // ERROR_MORE_DATA and writes nothing.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                value.as_ptr(),
                std::ptr::null(),
                &mut kind,
                (&raw mut data).cast::<u8>(),
                &mut size,
            )
        };
        // A REG_DWORD that fits the four bytes is one: a size check would be a condition nothing reaches.
        (status == ERROR_SUCCESS && kind == REG_DWORD).then_some(data)
    }

    /// The key opened for reading, closed when the handle drops; `None` when it is not there.
    fn open(hive: HKEY, key: &str) -> Option<Key> {
        let key = wide(key);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: the key name is NUL-terminated and `handle` receives the opened key; the status says
        // whether it did.
        let opened = unsafe { RegOpenKeyExW(hive, key.as_ptr(), 0, KEY_READ, &mut handle) };
        (opened == ERROR_SUCCESS).then_some(Key(handle))
    }

    /// A `REG_SZ` or `REG_EXPAND_SZ` value as written, without its trailing NULs, and whether it is the
    /// expandable type; `None` for a missing key or value, another type, or an empty string.
    fn string_value(hive: HKEY, key: &str, value: &str) -> Option<RegistryText> {
        let key = open(hive, key)?;
        let value = wide(value);
        let mut kind: REG_VALUE_TYPE = 0;
        let mut size: u32 = 0;
        // SAFETY: a null data pointer with a zero size asks for the type and the size in bytes only.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                value.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        // With a null buffer the size query answers ERROR_SUCCESS, never ERROR_MORE_DATA (the API's
        // contract), so success is the one status to go on with.
        if status != ERROR_SUCCESS {
            return None;
        }
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            return None;
        }
        // One UTF-16 unit of slack: the stored value may lack its terminator.
        let mut buffer = vec![0u16; (size as usize).div_ceil(2) + 1];
        let mut size = (buffer.len() * 2) as u32;
        // SAFETY: the buffer is `size` bytes long and `size` is updated to the bytes written.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                value.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buffer.as_mut_ptr().cast::<u8>(),
                &mut size,
            )
        };
        // coverage: os-begin — a read that fails after the size query passed
        if status != ERROR_SUCCESS {
            return None;
            // coverage: os-end
        }
        buffer.truncate((size as usize) / 2);
        while buffer.last() == Some(&0) {
            buffer.pop();
        }
        if buffer.is_empty() {
            return None;
        }
        Some(RegistryText {
            text: OsString::from_wide(&buffer),
            expandable: kind == REG_EXPAND_SZ,
        })
    }

    /// The game folder under `ProgramFiles(x86)` and `ProgramFiles` as `env` reads them, each with its
    /// `C:` default when unset or empty, once when the two are the same folder.
    pub(super) fn common_folders(env: &dyn Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
        let program_files = |name: &str, default: &str| {
            env(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(default))
        };
        let mut folders = vec![program_files(
            "ProgramFiles(x86)",
            r"C:\Program Files (x86)",
        )];
        let x64 = program_files("ProgramFiles", r"C:\Program Files");
        if !folders.contains(&x64) {
            folders.push(x64);
        }
        folders
            .into_iter()
            .map(|folder| folder.join(COMMON_FOLDER))
            .collect()
    }
}

#[cfg(not(windows))]
mod system {
    //! The other targets have no launcher key and no usual folder yet; the macOS preview (M6) brings
    //! `/Applications/World of Warcraft/`.

    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use super::{Hive, RegistryText};

    pub(super) fn registry_text(_hive: Hive, _key: &str, _value: &str) -> Option<RegistryText> {
        None
    }

    pub(super) fn expand_environment(_text: &OsStr) -> Option<OsString> {
        None
    }

    pub(super) fn registry_dword(_hive: Hive, _key: &str, _value: &str) -> Option<u32> {
        None
    }

    pub(super) fn common_folders(_env: &dyn Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
        Vec::new()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    /// The synthetic game folder of the fixtures.
    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flavors")
    }

    /// A fresh temporary folder for one test, removed first.
    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-discover-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the folder");
        dir
    }

    fn lookup(install_path: Option<PathBuf>, folders: &[PathBuf]) -> FixedLookup {
        FixedLookup {
            install_path,
            folders: folders.to_vec(),
            uninstall_locations: Vec::new(),
        }
    }

    #[test]
    fn a_game_root_has_a_build_info_and_a_flavor_folder() {
        assert!(is_game_root(&fixtures()));
        assert!(!is_game_root(&fixtures().join("_retail_")));
        assert!(!is_game_root(&fixtures().join("_no_such_")));

        let build_info_only = temp_dir("build-info-only");
        std::fs::write(build_info_only.join(BUILD_INFO), "Product!STRING:0\n").expect("write");
        assert!(!is_game_root(&build_info_only), "no flavor folder");

        let flavor_only = temp_dir("flavor-only");
        std::fs::create_dir_all(flavor_only.join("_forever_")).expect("the flavor folder");
        std::fs::write(
            flavor_only.join("_forever_").join(flavor::FLAVOR_INFO),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect("write");
        assert!(!is_game_root(&flavor_only), "no .build.info");

        let _ = std::fs::remove_dir_all(&build_info_only);
        let _ = std::fs::remove_dir_all(&flavor_only);
    }

    #[test]
    fn the_registry_value_is_tried_first_and_may_name_a_flavor_folder() {
        let elsewhere = temp_dir("elsewhere");
        let found = discover_root(&lookup(Some(fixtures()), std::slice::from_ref(&elsewhere)))
            .expect("a hit");
        assert_eq!(found.origin, RootOrigin::Registry);
        assert!(is_game_root(&found.path));

        // The launcher's value may point at a flavor folder, with a trailing separator.
        let mut flavor_folder = fixtures().join("_forever_").into_os_string();
        flavor_folder.push(std::path::MAIN_SEPARATOR_STR);
        let found = discover_root(&lookup(Some(PathBuf::from(flavor_folder)), &[]))
            .expect("the parent is the hit");
        assert_eq!(found.origin, RootOrigin::Registry);
        assert_eq!(
            found.path.file_name().and_then(OsStr::to_str),
            Some("flavors")
        );
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn the_usual_folders_follow_in_order_and_a_miss_is_none() {
        let elsewhere = temp_dir("elsewhere-folders");
        let missing = PathBuf::from("Q:\\no\\such\\folder");
        let found = discover_root(&lookup(
            Some(missing.clone()),
            &[elsewhere.clone(), fixtures()],
        ))
        .expect("the second folder");
        assert_eq!(found.origin, RootOrigin::CommonFolder);
        assert!(is_game_root(&found.path));

        assert_eq!(
            discover_root(&lookup(Some(missing), std::slice::from_ref(&elsewhere))),
            None
        );
        assert_eq!(discover_root(&FixedLookup::none()), None);
        // A registry value that names a folder inside a flavor is no hit: neither it nor its parent is a
        // game root.
        assert_eq!(
            discover_root(&lookup(
                Some(fixtures().join("_retail_").join("Interface")),
                &[]
            )),
            None
        );
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn the_order_is_flag_environment_remembered_discovery() {
        let discovery = lookup(Some(fixtures()), &[]);
        let flag = PathBuf::from("Q:\\flag");
        let environment = OsStr::new("Q:\\environment");
        let remembered = || Some(PathBuf::from("Q:\\remembered"));

        let root = resolve_root(
            Some(flag.clone()),
            Some(environment),
            remembered,
            &discovery,
        )
        .expect("the flag");
        assert_eq!((root.path, root.origin), (flag, RootOrigin::Flag));

        let root =
            resolve_root(None, Some(environment), remembered, &discovery).expect("the variable");
        assert_eq!(
            (root.path, root.origin),
            (PathBuf::from(environment), RootOrigin::Environment)
        );

        let root = resolve_root(None, Some(OsStr::new("")), remembered, &discovery)
            .expect("the remembered one; an empty variable does not count");
        assert_eq!(
            (root.path, root.origin),
            (PathBuf::from("Q:\\remembered"), RootOrigin::Remembered)
        );

        let root = resolve_root(None, None, || None, &discovery).expect("the discovery");
        assert_eq!(root.origin, RootOrigin::Registry);
        assert_eq!(
            resolve_root(None, None, || None, &FixedLookup::none()),
            None
        );
    }

    #[test]
    fn the_discovery_variable_describes_a_stub_or_the_system() {
        assert_eq!(parse_lookup(None), None, "the system");
        assert_eq!(parse_lookup(Some(OsStr::new(""))), None, "the system");
        assert_eq!(
            parse_lookup(Some(OsStr::new("off"))),
            Some(FixedLookup::none())
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new("OFF "))),
            Some(FixedLookup::none())
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new(
                "registry=Q:\\reg; folder=Q:\\one;Q:\\two;"
            ))),
            Some(FixedLookup {
                install_path: Some(PathBuf::from("Q:\\reg")),
                folders: vec![PathBuf::from("Q:\\one"), PathBuf::from("Q:\\two")],
                uninstall_locations: Vec::new(),
            })
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new(
                "webview2=131.0.2903.70;windows=24H2,26100.1234;Q:\\only"
            ))),
            Some(FixedLookup {
                install_path: None,
                folders: vec![PathBuf::from("Q:\\only")],
                uninstall_locations: Vec::new(),
            }),
            "the doctor's items are no folders"
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new("Q:\\only"))),
            Some(FixedLookup {
                install_path: None,
                folders: vec![PathBuf::from("Q:\\only")],
                uninstall_locations: Vec::new(),
            })
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new("uninstall=Q:\\one;uninstall=Q:\\two"))),
            Some(FixedLookup {
                uninstall_locations: vec![PathBuf::from("Q:\\one"), PathBuf::from("Q:\\two")],
                ..FixedLookup::none()
            }),
            "the Uninstall entry's values, in order"
        );
    }

    #[test]
    fn an_origin_has_a_fixed_name_a_description_and_a_found_line() {
        let origins = [
            RootOrigin::Flag,
            RootOrigin::Environment,
            RootOrigin::Remembered,
            RootOrigin::Registry,
            RootOrigin::UninstallEntry,
            RootOrigin::CommonFolder,
        ];
        let names: std::collections::HashSet<&str> =
            origins.iter().map(|origin| origin.as_str()).collect();
        assert_eq!(names.len(), origins.len(), "two origins share a JSON name");
        assert!(origins.iter().all(|origin| origin.is_discovered()
            == matches!(
                origin,
                RootOrigin::Registry | RootOrigin::UninstallEntry | RootOrigin::CommonFolder
            )));
        let root = GameRoot {
            path: PathBuf::from("Q:\\wow"),
            origin: RootOrigin::Registry,
        };
        assert_eq!(
            root.found_line(),
            "game root found in the launcher's registry key: Q:\\wow; run `mrgl root <folder>` to remember it"
        );
        let json = serde_json::to_value(&root).expect("serializes");
        assert_eq!(json["origin"], "registry");
        assert_eq!(json["path"], "Q:\\wow");
    }

    #[test]
    fn the_uninstall_entry_follows_the_launchers_value_and_says_so() {
        let elsewhere = temp_dir("elsewhere-uninstall");
        let missing = PathBuf::from("Q:\\no\\such\\folder");
        let with_uninstall = |install_path: Option<PathBuf>, locations: &[PathBuf]| FixedLookup {
            install_path,
            folders: vec![elsewhere.clone()],
            uninstall_locations: locations.to_vec(),
        };

        // The Uninstall entry alone finds the root, and the origin names it.
        let found = discover_root(&with_uninstall(None, &[fixtures()])).expect("a hit");
        assert_eq!(found.origin, RootOrigin::UninstallEntry);
        assert!(is_game_root(&found.path));
        assert!(
            found
                .found_line()
                .starts_with("game root found in the game's Uninstall registry entry: "),
            "{}",
            found.found_line()
        );
        assert_eq!(
            serde_json::to_value(&found).expect("serializes")["origin"],
            "uninstall_entry"
        );

        // A launcher's value that is no game root falls through to it; a later value is tried after a
        // missing one; a value naming a flavor folder counts for its parent.
        let found = discover_root(&with_uninstall(
            Some(missing.clone()),
            &[missing.clone(), fixtures().join("_forever_")],
        ))
        .expect("the parent of the flavor folder");
        assert_eq!(found.origin, RootOrigin::UninstallEntry);
        assert_eq!(
            found.path,
            std::path::absolute(fixtures()).expect("absolute")
        );

        // The launcher's value wins when both name a game root.
        let other = temp_dir("other-root");
        std::fs::write(other.join(BUILD_INFO), "Product!STRING:0\n").expect("write");
        std::fs::create_dir_all(other.join("_forever_")).expect("the flavor folder");
        std::fs::write(
            other.join("_forever_").join(flavor::FLAVOR_INFO),
            "Product Flavor!STRING:0\nwowf\n",
        )
        .expect("write");
        assert!(is_game_root(&other));
        let found = discover_root(&with_uninstall(
            Some(fixtures()),
            std::slice::from_ref(&other),
        ))
        .expect("a hit");
        assert_eq!(found.origin, RootOrigin::Registry);
        assert_eq!(
            found.path,
            std::path::absolute(fixtures()).expect("absolute")
        );

        // The Uninstall entry comes before the usual folders.
        let found = discover_root(&FixedLookup {
            install_path: None,
            folders: vec![fixtures()],
            uninstall_locations: vec![other.clone()],
        })
        .expect("a hit");
        assert_eq!(found.origin, RootOrigin::UninstallEntry);
        assert_eq!(found.path, other);

        let _ = std::fs::remove_dir_all(&elsewhere);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn an_expandable_value_resolves_to_the_root_of_its_literal() {
        // A stub of the environment call: one variable, `%MRGL_TEST_GAMES%`, names the fixtures' parent.
        let parent = fixtures()
            .parent()
            .expect("the fixtures' parent")
            .to_path_buf();
        let environment = |text: &OsStr| {
            Some(OsString::from(
                text.to_string_lossy()
                    .replace("%MRGL_TEST_GAMES%", &parent.to_string_lossy()),
            ))
        };
        let expandable = RegistryText {
            text: OsString::from("%MRGL_TEST_GAMES%\\flavors"),
            expandable: true,
        };
        let literal = RegistryText {
            text: fixtures().into_os_string(),
            expandable: false,
        };
        let expanded = registry_path(expandable.clone(), environment).expect("expanded");
        assert_eq!(expanded, parent.join("flavors"));
        let as_written = registry_path(literal, environment).expect("as written");
        for (launcher, uninstall) in [(Some(expanded.clone()), vec![]), (None, vec![expanded])] {
            let from_expanded = discover_root(&FixedLookup {
                install_path: launcher,
                folders: Vec::new(),
                uninstall_locations: uninstall,
            })
            .expect("the expanded value finds the root");
            let from_literal = discover_root(&lookup(Some(as_written.clone()), &[]))
                .expect("the literal finds the root");
            assert_eq!(from_expanded.path, from_literal.path);
        }

        // A plain string is taken as written, `%` and all; a failed expansion or an empty one is no
        // value.
        let plain = RegistryText {
            expandable: false,
            ..expandable.clone()
        };
        assert_eq!(
            registry_path(plain, environment),
            Some(PathBuf::from("%MRGL_TEST_GAMES%\\flavors"))
        );
        assert_eq!(registry_path(expandable.clone(), |_| None), None);
        assert_eq!(registry_path(expandable, |_| Some(OsString::new())), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_environment_call_expands_a_set_variable_and_keeps_an_unset_one() {
        let system_root = std::env::var_os("SystemRoot").expect("SystemRoot is set on Windows");
        let mut expected = system_root.clone();
        expected.push("\\Fonts");
        assert_eq!(
            expand_environment(OsStr::new("%SystemRoot%\\Fonts")),
            Some(expected)
        );
        assert_eq!(
            expand_environment(OsStr::new("%MRGL_NO_SUCH_VARIABLE_043%\\x")),
            Some(OsString::from("%MRGL_NO_SUCH_VARIABLE_043%\\x"))
        );
        assert_eq!(expand_environment(OsStr::new("")), Some(OsString::new()));
        // The stub item goes through the same call.
        let mut root = system_root;
        root.push("\\World of Warcraft");
        assert_eq!(
            parse_lookup(Some(OsStr::new(
                "registry-expand=%SystemRoot%\\World of Warcraft;uninstall-expand=%SystemRoot%\\World of Warcraft"
            ))),
            Some(FixedLookup {
                install_path: Some(PathBuf::from(&root)),
                folders: Vec::new(),
                uninstall_locations: vec![PathBuf::from(&root)],
            })
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_usual_folders_are_the_game_folder_under_both_program_files() {
        let folders = SystemLookup.common_folders();
        assert!(!folders.is_empty());
        assert!(
            folders
                .iter()
                .all(|folder| folder.file_name().and_then(OsStr::to_str) == Some(COMMON_FOLDER)),
            "{folders:?}"
        );
        assert!(
            folders.iter().any(|folder| folder
                .to_string_lossy()
                .to_lowercase()
                .contains("program files")),
            "{folders:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_usual_folders_come_from_the_variables_with_their_defaults() {
        let env_of = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            }
        };
        assert_eq!(
            system::common_folders(&env_of(&[])),
            [
                PathBuf::from(r"C:\Program Files (x86)\World of Warcraft"),
                PathBuf::from(r"C:\Program Files\World of Warcraft"),
            ],
            "unset: the defaults"
        );
        assert_eq!(
            system::common_folders(&env_of(&[("ProgramFiles(x86)", ""), ("ProgramFiles", "")])),
            system::common_folders(&env_of(&[])),
            "empty: the defaults"
        );
        assert_eq!(
            system::common_folders(&env_of(&[
                ("ProgramFiles(x86)", r"D:\Apps"),
                ("ProgramFiles", r"D:\Apps")
            ])),
            [PathBuf::from(r"D:\Apps\World of Warcraft")],
            "the same folder twice is tried once"
        );
        assert_eq!(
            system::common_folders(&env_of(&[("ProgramFiles", r"D:\Apps")])),
            [
                PathBuf::from(r"C:\Program Files (x86)\World of Warcraft"),
                PathBuf::from(r"D:\Apps\World of Warcraft"),
            ]
        );
    }

    /// A lookup that leaves the trait's default for the Uninstall entry.
    struct Bare;

    impl RootLookup for Bare {
        fn launcher_install_path(&self) -> Option<PathBuf> {
            None
        }

        fn common_folders(&self) -> Vec<PathBuf> {
            Vec::new()
        }
    }

    #[test]
    fn a_lookup_without_an_uninstall_entry_reads_none_by_default() {
        assert!(Bare.uninstall_locations().is_empty());
        assert_eq!(discover_root(&Bare), None);
    }

    #[test]
    fn every_origin_has_a_description() {
        assert_eq!(RootOrigin::Flag.description(), "--root");
        assert_eq!(RootOrigin::Environment.description(), "MRGL_ROOT");
        assert_eq!(RootOrigin::Remembered.description(), "remembered");
        assert_eq!(
            RootOrigin::Registry.description(),
            "the launcher's registry key"
        );
        assert_eq!(
            RootOrigin::UninstallEntry.description(),
            "the game's Uninstall registry entry"
        );
        assert_eq!(
            RootOrigin::CommonFolder.description(),
            "a usual install folder"
        );
    }

    #[test]
    fn the_discovery_variable_picks_the_system_or_a_stub() {
        let system = lookup_from_env(None);
        assert_eq!(system.common_folders(), SystemLookup.common_folders());
        assert_eq!(
            system.launcher_install_path(),
            SystemLookup.launcher_install_path()
        );
        let off = lookup_from_env(Some(OsStr::new("off")));
        assert!(off.common_folders().is_empty());
        assert_eq!(off.launcher_install_path(), None);
        assert!(off.uninstall_locations().is_empty());
        // An empty item between two separators is skipped; an empty candidate is no root.
        assert_eq!(
            parse_lookup(Some(OsStr::new("Q:\\one;;Q:\\two"))).map(|lookup| lookup.folders),
            Some(vec![PathBuf::from("Q:\\one"), PathBuf::from("Q:\\two")])
        );
        assert_eq!(discover_root(&lookup(Some(PathBuf::new()), &[])), None);
    }

    #[test]
    fn the_remembered_root_comes_from_the_config_and_its_failures_are_told() {
        let env_of = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            }
        };
        let off = env_of(&[("MRGL_DISCOVERY", "off")]);
        let remembered = || {
            Ok(home::LoadedConfig {
                config: home::Config {
                    root: Some(PathBuf::from("Q:\\remembered")),
                },
                warning: None,
            })
        };
        let resolution = resolve_root_from(None, &off, remembered).expect("resolves");
        assert_eq!(
            resolution.root,
            Some(GameRoot {
                path: PathBuf::from("Q:\\remembered"),
                origin: RootOrigin::Remembered,
            })
        );
        assert!(resolution.warnings.is_empty());

        // A corrupt config: its warning is kept, and the discovery (off here) answers.
        let corrupt = || {
            Ok(home::LoadedConfig {
                config: home::Config::default(),
                warning: Some("unreadable config: Q:\\config.json: trailing".to_string()),
            })
        };
        let resolution = resolve_root_from(None, &off, corrupt).expect("resolves");
        assert_eq!(resolution.root, None);
        assert_eq!(
            resolution.warnings,
            ["unreadable config: Q:\\config.json: trailing"]
        );

        // No home folder is no remembered root and no warning; the stub finds the fixtures.
        let stub = env_of(&[("MRGL_DISCOVERY", "registry=Q:\\no\\such")]);
        let no_home = || Err(Error::NoHome);
        let resolution = resolve_root_from(None, &stub, no_home).expect("resolves");
        assert_eq!(resolution, Resolution::default());
        let found = |name: &str| (name == DISCOVERY_ENV).then(|| fixtures().into_os_string());
        let resolution = resolve_root_from(None, &found, no_home).expect("resolves");
        assert_eq!(
            resolution.root.map(|root| root.origin),
            Some(RootOrigin::CommonFolder)
        );

        // A config that cannot be read is the error.
        let unreadable = || {
            Err(Error::Io {
                context: "reading the config",
                path: PathBuf::from("Q:\\config.json"),
                source: std::io::Error::other("locked"),
            })
        };
        let error = resolve_root_from(None, &off, unreadable).expect_err("the error");
        assert_eq!(
            error.to_string(),
            "reading the config: Q:\\config.json: locked"
        );

        // The flag and the variable come first: the config is not read.
        let not_read = || panic!("the config is not read");
        let named = env_of(&[("MRGL_ROOT", "Q:\\named"), ("MRGL_DISCOVERY", "off")]);
        let resolution = resolve_root_from(None, &named, not_read).expect("resolves");
        assert_eq!(
            resolution.root.map(|root| (root.path, root.origin)),
            Some((PathBuf::from("Q:\\named"), RootOrigin::Environment))
        );
        let resolution =
            resolve_root_from(Some(PathBuf::from("Q:\\flag")), &off, not_read).expect("resolves");
        assert_eq!(
            resolution.root.map(|root| root.origin),
            Some(RootOrigin::Flag)
        );
    }

    /// The key under `HKEY_LOCAL_MACHINE` with the Windows version: `ProductName` (`REG_SZ`) and
    /// `CurrentMajorVersionNumber` (`REG_DWORD`) are there on every Windows 10 and 11.
    #[cfg(windows)]
    const VERSION_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    /// The system's environment, whose `TEMP` is a `REG_EXPAND_SZ`.
    #[cfg(windows)]
    const ENVIRONMENT_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

    #[cfg(windows)]
    #[test]
    fn the_registry_reads_a_string_an_expandable_string_and_a_dword() {
        let name =
            registry_text(Hive::LocalMachine, VERSION_KEY, "ProductName").expect("ProductName");
        assert!(!name.expandable);
        assert!(
            name.text.to_string_lossy().starts_with("Windows"),
            "{name:?}"
        );
        assert_eq!(
            registry_string(Hive::LocalMachine, VERSION_KEY, "ProductName"),
            Some(name.text)
        );
        let temp = registry_text(Hive::LocalMachine, ENVIRONMENT_KEY, "TEMP").expect("TEMP");
        assert!(temp.expandable, "{temp:?}");
        let expanded = registry_path(temp.clone(), expand_environment).expect("expands");
        assert_ne!(expanded, PathBuf::from(&temp.text));
        assert!(!expanded.to_string_lossy().contains('%'), "{expanded:?}");
        let major = registry_dword(Hive::LocalMachine, VERSION_KEY, "CurrentMajorVersionNumber");
        assert_eq!(major, Some(10), "Windows 10 and 11 both say 10");
        // Asked as the other type, a value is nothing.
        assert_eq!(
            registry_string(Hive::LocalMachine, VERSION_KEY, "CurrentMajorVersionNumber"),
            None,
            "a dword is no string"
        );
        assert_eq!(
            registry_dword(Hive::LocalMachine, VERSION_KEY, "ProductName"),
            None,
            "a string is no dword"
        );
        assert_eq!(
            registry_dword(Hive::LocalMachine, VERSION_KEY, "DigitalProductId"),
            None,
            "a binary is no dword"
        );
        assert_eq!(
            registry_string(Hive::LocalMachine, VERSION_KEY, "DigitalProductId"),
            None,
            "a binary is no string"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_missing_key_or_value_reads_as_none_in_both_hives() {
        for hive in [Hive::LocalMachine, Hive::CurrentUser] {
            assert_eq!(
                registry_string(hive, r"Software\mrgl-no-such-key-020", "x"),
                None
            );
            assert_eq!(
                registry_dword(hive, r"Software\mrgl-no-such-key-020", "x"),
                None
            );
        }
        assert_eq!(
            registry_string(Hive::LocalMachine, VERSION_KEY, "MrglNoSuchValue020"),
            None
        );
        assert_eq!(
            registry_dword(Hive::LocalMachine, VERSION_KEY, "MrglNoSuchValue020"),
            None
        );
    }

    /// A key under `HKEY_CURRENT_USER\Software` written by `reg.exe` for one test (a spawned system
    /// exe, as the client tests spawn `ping.exe`) and deleted on drop.
    #[cfg(windows)]
    struct TestKey {
        reg: PathBuf,
        name: String,
    }

    #[cfg(windows)]
    impl TestKey {
        /// `None` when the system folder has no `reg.exe`; the test is then skipped with a message.
        fn new(suffix: &str) -> Option<TestKey> {
            let reg = PathBuf::from(std::env::var_os("SystemRoot")?)
                .join("System32")
                .join("reg.exe");
            if !reg.is_file() {
                return None;
            }
            let name = format!("Software\\mrgl-test-{}-{suffix}", std::process::id());
            let key = TestKey { reg, name };
            key.run(&["delete", &key.path(), "/f"]);
            Some(key)
        }

        fn path(&self) -> String {
            format!("HKCU\\{}", self.name)
        }

        fn run(&self, args: &[&str]) -> bool {
            std::process::Command::new(&self.reg)
                .args(args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        }

        fn add(&self, value: &str, kind: &str, data: &str) {
            assert!(
                self.run(&[
                    "add",
                    &self.path(),
                    "/v",
                    value,
                    "/t",
                    kind,
                    "/d",
                    data,
                    "/f"
                ]),
                "reg add {value}"
            );
        }
    }

    #[cfg(windows)]
    impl Drop for TestKey {
        fn drop(&mut self) {
            self.run(&["delete", &self.path(), "/f"]);
        }
    }

    #[cfg(windows)]
    #[test]
    fn an_empty_string_value_reads_as_none_and_a_short_string_is_no_dword() {
        let Some(key) = TestKey::new("values") else {
            eprintln!("skipped: no reg.exe in the system folder to write the test key");
            return;
        };
        key.add("Empty", "REG_SZ", "");
        key.add("Short", "REG_SZ", "a");
        key.add("Expand", "REG_EXPAND_SZ", "%SystemRoot%\\mrgl");
        key.add("Number", "REG_DWORD", "42");
        assert_eq!(registry_string(Hive::CurrentUser, &key.name, "Empty"), None);
        assert_eq!(
            registry_string(Hive::CurrentUser, &key.name, "Short"),
            Some(OsString::from("a"))
        );
        // Four bytes with the type of a string: the size fits a dword, the type says no.
        assert_eq!(registry_dword(Hive::CurrentUser, &key.name, "Short"), None);
        assert_eq!(
            registry_text(Hive::CurrentUser, &key.name, "Expand"),
            Some(RegistryText {
                text: OsString::from("%SystemRoot%\\mrgl"),
                expandable: true,
            })
        );
        assert_eq!(
            registry_dword(Hive::CurrentUser, &key.name, "Number"),
            Some(42)
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_launcher_and_the_uninstall_entry_are_read_from_the_registry() {
        // Whether the game is installed on this machine is not the test's to know: a value found is a
        // path as written in the registry, absolute.
        let lookup = SystemLookup;
        if let Some(path) = lookup.launcher_install_path() {
            assert!(path.is_absolute(), "{path:?}");
        }
        for path in lookup.uninstall_locations() {
            assert!(path.is_absolute(), "{path:?}");
        }
        assert_eq!(
            lookup.launcher_install_path(),
            system_path(Hive::LocalMachine, LAUNCHER_KEY, LAUNCHER_VALUE)
        );
    }
}
