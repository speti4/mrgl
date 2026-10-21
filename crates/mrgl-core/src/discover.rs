//! Where the game folder comes from (task 020). A run names it with `--root`, `MRGL_ROOT` or the root
//! that `mrgl root <folder>` remembered in `config.json` (task 002); when none of those does, the
//! discovery looks it up in the launcher's registry key (`HKLM\SOFTWARE\WOW6432Node\Blizzard
//! Entertainment\World of Warcraft`, value `InstallPath`) and then in the usual install folders
//! (`Program Files (x86)\World of Warcraft`, the same under `Program Files`). A hit counts only when it
//! is a game root ([`is_game_root`]: a `.build.info` and at least one flavor folder) or when its parent
//! is one (a launcher value may name a flavor folder). A discovered root is used for the run and reported
//! with its origin, never written to `config.json`: `mrgl root <folder>` and the window's picker (task
//! 009) are the places that remember it.
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
    system::registry_string(hive, key, value)
}

/// One `REG_DWORD` value of the registry, read the same way: `None` for a missing key or value, another
/// type, and always on a target without a registry. The doctor reads the Windows update build revision
/// (`UBR`) through it (task 051).
pub fn registry_dword(hive: Hive, key: &str, value: &str) -> Option<u32> {
    system::registry_dword(hive, key, value)
}

/// The places a game root may be found when nothing names it.
pub trait RootLookup {
    /// The launcher's `InstallPath` value as written; `None` when the key or the value is missing.
    fn launcher_install_path(&self) -> Option<PathBuf>;
    /// The usual install folders, in the order they are tried.
    fn common_folders(&self) -> Vec<PathBuf>;
}

/// The machine's registry and environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemLookup;

impl RootLookup for SystemLookup {
    fn launcher_install_path(&self) -> Option<PathBuf> {
        system::launcher_install_path()
    }

    fn common_folders(&self) -> Vec<PathBuf> {
        system::common_folders()
    }
}

/// A fixed lookup, the stub for tests: the registry value and the folders it is given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixedLookup {
    pub install_path: Option<PathBuf>,
    pub folders: Vec<PathBuf>,
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
            RootOrigin::CommonFolder => "a usual install folder",
        }
    }

    /// Whether the discovery found the root, rather than the user naming it.
    pub const fn is_discovered(self) -> bool {
        matches!(self, RootOrigin::Registry | RootOrigin::CommonFolder)
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
/// (a launcher value may name a flavor folder), `None` otherwise.
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

/// The first game root the lookup finds: the launcher's registry value, then the usual folders in
/// order; `None` when none of them is a game root.
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
/// anything else is a stub, `;`-separated, where `registry=<path>` is the launcher's value, a
/// `webview2=<version>` or a `windows=<version>` item is the doctor's (tasks 024 and 051,
/// [`crate::doctor`]) and every other item (with or without `folder=`) is one of the usual folders, in
/// order. The stub keeps the end-to-end tests off the
/// real registry and lets a user switch the discovery off.
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
        if let Some(path) = item.strip_prefix("registry=") {
            lookup.install_path = Some(PathBuf::from(path));
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
    let mut warnings = Vec::new();
    let mut failure = None;
    let lookup = lookup_from_env(std::env::var_os(DISCOVERY_ENV).as_deref());
    let root = resolve_root(
        flag,
        std::env::var_os(ROOT_ENV).as_deref(),
        || match home::read_config() {
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
    //! One `RegOpenKeyExW` on the launcher's key, one `RegQueryValueExW` for the size and one for the
    //! value, and the key is closed; a missing key or value, or a value that is not a string, is `None`.
    //! The usual folders come from `ProgramFiles(x86)` and `ProgramFiles`, with the `C:` defaults when
    //! the variables are unset.

    use std::ffi::{OsStr, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_SZ,
        REG_VALUE_TYPE, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    use super::{COMMON_FOLDER, Hive, LAUNCHER_KEY, LAUNCHER_VALUE};

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

    pub(super) fn launcher_install_path() -> Option<PathBuf> {
        string_value(HKEY_LOCAL_MACHINE, LAUNCHER_KEY, LAUNCHER_VALUE).map(PathBuf::from)
    }

    pub(super) fn registry_string(hive: Hive, key: &str, value: &str) -> Option<OsString> {
        let hive = match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        };
        string_value(hive, key, value)
    }

    pub(super) fn registry_dword(hive: Hive, key: &str, value: &str) -> Option<u32> {
        let hive = match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        };
        let key = open(hive, key)?;
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
        (status == ERROR_SUCCESS && kind == REG_DWORD && size == 4).then_some(data)
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

    /// A `REG_SZ` or `REG_EXPAND_SZ` value as written (an expandable value is not expanded; the launcher
    /// writes a plain path), without its trailing NULs; `None` for a missing key or value, another type,
    /// or an empty string.
    fn string_value(hive: HKEY, key: &str, value: &str) -> Option<OsString> {
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
        if status != ERROR_SUCCESS && status != ERROR_MORE_DATA {
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
        if status != ERROR_SUCCESS {
            return None;
        }
        buffer.truncate((size as usize) / 2);
        while buffer.last() == Some(&0) {
            buffer.pop();
        }
        if buffer.is_empty() {
            return None;
        }
        Some(OsString::from_wide(&buffer))
    }

    pub(super) fn common_folders() -> Vec<PathBuf> {
        let program_files = |name: &str, default: &str| {
            std::env::var_os(name)
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

    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::Hive;

    pub(super) fn launcher_install_path() -> Option<PathBuf> {
        None
    }

    pub(super) fn registry_string(_hive: Hive, _key: &str, _value: &str) -> Option<OsString> {
        None
    }

    pub(super) fn registry_dword(_hive: Hive, _key: &str, _value: &str) -> Option<u32> {
        None
    }

    pub(super) fn common_folders() -> Vec<PathBuf> {
        Vec::new()
    }
}

#[cfg(test)]
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
            })
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new(
                "webview2=131.0.2903.70;windows=24H2,26100.1234;Q:\\only"
            ))),
            Some(FixedLookup {
                install_path: None,
                folders: vec![PathBuf::from("Q:\\only")],
            }),
            "the doctor's items are no folders"
        );
        assert_eq!(
            parse_lookup(Some(OsStr::new("Q:\\only"))),
            Some(FixedLookup {
                install_path: None,
                folders: vec![PathBuf::from("Q:\\only")],
            })
        );
    }

    #[test]
    fn an_origin_has_a_fixed_name_a_description_and_a_found_line() {
        let origins = [
            RootOrigin::Flag,
            RootOrigin::Environment,
            RootOrigin::Remembered,
            RootOrigin::Registry,
            RootOrigin::CommonFolder,
        ];
        let names: std::collections::HashSet<&str> =
            origins.iter().map(|origin| origin.as_str()).collect();
        assert_eq!(names.len(), origins.len(), "two origins share a JSON name");
        assert!(origins.iter().all(|origin| origin.is_discovered()
            == matches!(origin, RootOrigin::Registry | RootOrigin::CommonFolder)));
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
}
