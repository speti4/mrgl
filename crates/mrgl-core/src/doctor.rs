//! The diagnostic report (task 024): `mrgl doctor` prints it and the window's "copy diagnostics" action
//! (task 009) copies the same text. One [`Report`] holds every fact a bug report needs: the Windows version
//! (task 051), the game root and how it was found, each flavor's `.flavor.info` id and build (with the file
//! the build came from, task 019), whether `Interface\AddOns` is writable, the state sidecar's state, the
//! size of the flavor's zip cache, the run folders under its `staging\` kept for the addon folders a failed
//! swap moved aside (task 088), the WebView2 runtime's version, the GitHub rate limit left, which API keys
//! are set (by name, never a value) and whether the client runs from the flavor folder (task 023).
//! [`Report::lines`] is the plain block a user pastes, the derived `Serialize` is the `--json` document.
//!
//! The report reads and never repairs: the only write is the AddOns probe, an empty file created and
//! removed at once, and only in an `Interface\AddOns` folder that exists. A fact that cannot be read is a
//! value (`unknown`, `missing`) and never an error, so the report always comes out whole. Every path under
//! the user's profile folder is printed with that folder as `~` ([`Redactor`]), so a pasted report carries
//! no home folder path and with it no user name.
//!
//! Behind the report sit the stubs of the modules it reads: the process list of [`crate::client`], the
//! [`Http`] of [`crate::sources`] (one request, `GET /rate_limit`, which GitHub does not count against the
//! limit) and a [`RuntimeLookup`] for the Windows and the WebView2 version, which reads the registry the way
//! the game root discovery does ([`discover::registry_string`], [`discover::registry_dword`]);
//! `MRGL_DISCOVERY` replaces it for one run as it replaces the discovery, so no end-to-end test reads the
//! real registry. [`report_of_process`] is the one call of the command line and the window; [`report`]
//! takes every input explicitly for the tests.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::client::{self, ProcessList, SystemProcesses};
use crate::discover::{
    self, DISCOVERY_ENV, GameRoot, Hive, RootOrigin, WEBVIEW2_ITEM, WINDOWS_ITEM,
};
use crate::flavor::{self, Flavor, FlavorId};
use crate::home;
use crate::scan;
use crate::sources::github::{API_URL, TOKEN_ENV};
use crate::sources::{self, Http, Request};
use crate::staging;
use crate::state::{self, FlavorHome};
use crate::voice::Kind;
use crate::zip_entry;

/// The WebView2 runtime's client id under the EdgeUpdate `Clients` key.
pub const WEBVIEW2_CLIENT_ID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
/// The value under the runtime's key that holds its version.
pub const WEBVIEW2_VALUE: &str = "pv";
/// The registry keys of the WebView2 runtime, in the order they are read: the per-machine install as a
/// 64-bit Windows keeps it, the same on a 32-bit Windows, then the per-user install.
pub const WEBVIEW2_KEYS: [(Hive, &str, Scope); 3] = [
    (
        Hive::LocalMachine,
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        Scope::Machine,
    ),
    (
        Hive::LocalMachine,
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        Scope::Machine,
    ),
    (
        Hive::CurrentUser,
        r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        Scope::User,
    ),
];
/// The key under `HKEY_LOCAL_MACHINE` that holds the Windows version (task 051).
pub const WINDOWS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
/// The first build of Windows 11: `ProductName` says Windows 10 on Windows 11 too, so the release's name
/// comes from the build.
pub const WINDOWS_11_BUILD: u32 = 22000;
/// The first build of Windows 10.
pub const WINDOWS_10_BUILD: u32 = 10240;
/// The API keys the report names when they are set, by their environment variable; the CurseForge key
/// joins with its source (M4).
pub const API_KEYS: [&str; 1] = [TOKEN_ENV];
/// The name of the AddOns probe file, followed by the process id and a counter.
pub const PROBE_PREFIX: &str = ".mrgl-doctor-probe-";

/// Where the WebView2 runtime is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// For every user of the machine (the Windows 11 default).
    Machine,
    /// For the current user only.
    User,
}

impl Scope {
    fn description(self) -> &'static str {
        match self {
            Scope::Machine => "per-machine",
            Scope::User => "per-user",
        }
    }
}

/// The WebView2 runtime the window runs on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WebView2 {
    /// The runtime's version as the registry writes it (`131.0.2903.70`).
    pub version: String,
    pub scope: Scope,
}

/// The Windows version, from the values under [`WINDOWS_KEY`] (task 051): each one `None` when it is not
/// there, so a partial answer still says what it can.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WindowsVersion {
    /// The release: `Windows 11` or `Windows 10` by the build, or the product name as written for another
    /// Windows (a server).
    pub name: Option<String>,
    /// `DisplayVersion`: `24H2`.
    pub display_version: Option<String>,
    /// `CurrentBuildNumber`: `26100`.
    pub build: Option<u32>,
    /// `UBR`, the update build revision: `1234`.
    pub ubr: Option<u32>,
    /// `ProductName` as written (`Windows 10 Pro`, on Windows 11 too).
    pub product_name: Option<String>,
}

impl WindowsVersion {
    /// The version from the registry's values as written: the build is a decimal number, an empty or
    /// unreadable value is `None`, and the name comes from the build and the product name.
    pub fn from_values(
        product_name: Option<&str>,
        display_version: Option<&str>,
        build_number: Option<&str>,
        ubr: Option<u32>,
    ) -> WindowsVersion {
        let text = |value: Option<&str>| {
            value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let product_name = text(product_name);
        let build = build_number.and_then(|build| build.trim().parse::<u32>().ok());
        WindowsVersion {
            name: windows_name(product_name.as_deref(), build),
            display_version: text(display_version),
            build,
            ubr,
            product_name,
        }
    }

    /// The report's value: `Windows 11 24H2, build 26100.1234`; a missing part is left out (`Windows 11,
    /// build 26100`), a missing build is `build unknown`, and no value at all is `unknown`.
    pub fn text(&self) -> String {
        if self.name.is_none() && self.display_version.is_none() && self.build.is_none() {
            return "unknown".to_string();
        }
        let mut release = self.name.clone().unwrap_or_else(|| "Windows".to_string());
        if let Some(display_version) = &self.display_version {
            release.push(' ');
            release.push_str(display_version);
        }
        let build = match (self.build, self.ubr) {
            (Some(build), Some(ubr)) => format!("{build}.{ubr}"),
            (Some(build), None) => build.to_string(),
            (None, _) => "unknown".to_string(),
        };
        format!("{release}, build {build}")
    }
}

/// The release's name: a `Windows 10` or `Windows 11` product name is named by the build (Windows 11 keeps
/// writing `Windows 10` there), any other product name stays as written, and without one the build names
/// it alone.
fn windows_name(product_name: Option<&str>, build: Option<u32>) -> Option<String> {
    let by_build = build.and_then(|build| {
        if build >= WINDOWS_11_BUILD {
            Some("Windows 11")
        } else if build >= WINDOWS_10_BUILD {
            Some("Windows 10")
        } else {
            None
        }
    });
    match product_name {
        Some(name) => {
            let client = ["Windows 10", "Windows 11"]
                .into_iter()
                .find(|client| name.get(..client.len()) == Some(*client));
            match client {
                Some(client) => Some(by_build.unwrap_or(client).to_string()),
                None => Some(name.to_string()),
            }
        }
        None => by_build.map(str::to_string),
    }
}

/// Where the WebView2 and the Windows version come from.
pub trait RuntimeLookup {
    /// The installed runtime, or `None` when no key names a usable version.
    fn webview2(&self) -> Option<WebView2>;
    /// The Windows version (task 051); every part `None` when nothing could be read.
    fn windows(&self) -> WindowsVersion;
}

/// The machine's registry, through [`discover::registry_string`] and [`discover::registry_dword`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRuntime;

impl RuntimeLookup for SystemRuntime {
    fn webview2(&self) -> Option<WebView2> {
        WEBVIEW2_KEYS.iter().find_map(|(hive, key, scope)| {
            let value = discover::registry_string(*hive, key, WEBVIEW2_VALUE)?;
            usable_version(&value.to_string_lossy()).map(|version| WebView2 {
                version,
                scope: *scope,
            })
        })
    }

    fn windows(&self) -> WindowsVersion {
        let string = |value: &str| {
            discover::registry_string(Hive::LocalMachine, WINDOWS_KEY, value)
                .map(|text| text.to_string_lossy().into_owned())
        };
        WindowsVersion::from_values(
            string("ProductName").as_deref(),
            string("DisplayVersion").as_deref(),
            string("CurrentBuildNumber").as_deref(),
            discover::registry_dword(Hive::LocalMachine, WINDOWS_KEY, "UBR"),
        )
    }
}

/// A fixed answer, the stub for tests and for `MRGL_DISCOVERY`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixedRuntime {
    pub webview2: Option<WebView2>,
    pub windows: WindowsVersion,
}

impl RuntimeLookup for FixedRuntime {
    fn webview2(&self) -> Option<WebView2> {
        self.webview2.clone()
    }

    fn windows(&self) -> WindowsVersion {
        self.windows.clone()
    }
}

/// A `pv` value that names an installed runtime: not empty, not `null`, not `0.0.0.0` (what an
/// uninstalled runtime leaves behind).
fn usable_version(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && !value.eq_ignore_ascii_case("null") && value != "0.0.0.0")
        .then(|| value.to_string())
}

/// The runtime lookup `MRGL_DISCOVERY` names: unset or empty is the registry; any other value is a stub,
/// whose `webview2=<version>` item is the per-machine runtime and whose
/// `windows=<display version>,<build>.<ubr>` item is the Windows version (`windows=24H2,26100.1234`; a part
/// may be left out); an item's absence (`off`, a list of folders only) is no runtime, an unknown Windows.
pub fn runtime_from_env(value: Option<&OsStr>) -> Box<dyn RuntimeLookup> {
    match parse_runtime(value) {
        Some(fixed) => Box::new(fixed),
        None => Box::new(SystemRuntime),
    }
}

/// The stub `MRGL_DISCOVERY` describes, or `None` for the registry.
fn parse_runtime(value: Option<&OsStr>) -> Option<FixedRuntime> {
    let value = value.filter(|value| !value.is_empty())?;
    let text = value.to_string_lossy();
    let item = |prefix: &str| {
        text.split(';')
            .map(str::trim)
            .find_map(|item| item.strip_prefix(prefix))
            .map(str::to_string)
    };
    let version = item(WEBVIEW2_ITEM).as_deref().and_then(usable_version);
    let windows = item(WINDOWS_ITEM)
        .map(|item| {
            let (display_version, build) = item.split_once(',').unwrap_or((&item, ""));
            let (build, ubr) = build.split_once('.').unwrap_or((build, ""));
            WindowsVersion::from_values(
                None,
                Some(display_version),
                Some(build),
                ubr.trim().parse().ok(),
            )
        })
        .unwrap_or_default();
    Some(FixedRuntime {
        webview2: version.map(|version| WebView2 {
            version,
            scope: Scope::Machine,
        }),
        windows,
    })
}

/// The GitHub rate limit left, or why it is unknown.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RateLimit {
    /// Requests left in the current window.
    pub remaining: Option<u64>,
    /// Requests per window: 60 without a token, 5000 with one.
    pub limit: Option<u64>,
    /// When the window starts over, RFC 3339 in UTC.
    pub reset: Option<String>,
    /// Why the numbers are unknown (offline, a status, an answer without them); `None` when known.
    pub reason: Option<String>,
}

impl RateLimit {
    fn unknown(reason: impl Into<String>) -> RateLimit {
        RateLimit {
            reason: Some(reason.into()),
            ..RateLimit::default()
        }
    }
}

/// The rate limit, from one `GET /rate_limit` with the token when there is one (GitHub does not count
/// this request). A transport failure, a status other than 200 or an answer without the numbers is
/// `unknown` with the reason, never an error.
pub fn rate_limit(http: &dyn Http, token: Option<&str>) -> RateLimit {
    let mut request = Request::get(format!("{API_URL}/rate_limit"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = token.filter(|token| !token.is_empty()) {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = match http.get(&request) {
        Ok(response) => response,
        Err(error) => return RateLimit::unknown(format!("offline: {error}")),
    };
    if response.status != 200 {
        return RateLimit::unknown(format!("HTTP {}", response.status));
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&response.body) else {
        return RateLimit::unknown("an answer that is not JSON");
    };
    let core = value
        .pointer("/resources/core")
        .or_else(|| value.get("rate"));
    let number = |name: &str| core.and_then(|core| core.get(name)?.as_u64());
    match (number("remaining"), number("limit")) {
        (Some(remaining), Some(limit)) => RateLimit {
            remaining: Some(remaining),
            limit: Some(limit),
            reset: number("reset").map(state::rfc3339_from_unix),
            reason: None,
        },
        _ => RateLimit::unknown("an answer without the numbers"),
    }
}

/// An API key by name, and whether it is set; the value is never read into the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiKey {
    pub name: &'static str,
    pub present: bool,
}

/// The keys of [`API_KEYS`] as an environment reader sees them: present when set and not empty.
pub fn api_keys_from(env: impl Fn(&str) -> Option<OsString>) -> Vec<ApiKey> {
    API_KEYS
        .iter()
        .map(|name| ApiKey {
            name,
            present: env(name).is_some_and(|value| !value.is_empty()),
        })
        .collect()
}

/// Hides the user's profile folder in a path or a line: every occurrence of the folder, in its long and
/// its 8.3 short form, in any case and with `\` or `/` (or a mix) as the separator, becomes `~` when a
/// separator or the end follows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redactor {
    /// The forms of the profile folder, longest first, without a trailing separator.
    forms: Vec<String>,
}

impl Redactor {
    /// A redactor for a profile folder (`USERPROFILE`) and its short form; a folder of fewer than two
    /// names under its root (`C:\`, `C:\Users`) hides nothing, so a broken variable never hides a drive.
    pub fn new(profile: Option<&Path>) -> Redactor {
        let mut forms = Vec::new();
        if let Some(profile) = profile.filter(|profile| {
            profile
                .components()
                .filter(|part| matches!(part, std::path::Component::Normal(_)))
                .count()
                >= 2
        }) {
            forms.push(profile.to_path_buf());
            forms.extend(client::short_form(profile));
        }
        let mut forms: Vec<String> = forms
            .iter()
            .map(|form| {
                form.display()
                    .to_string()
                    .trim_end_matches(['\\', '/'])
                    .to_string()
            })
            .filter(|form| !form.is_empty())
            .collect();
        forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
        forms.dedup_by(|a, b| path_prefix(a, b) == Some(a.len()));
        Redactor { forms }
    }

    /// The redactor of this process: `USERPROFILE`, else `HOME`.
    pub fn of_process() -> Redactor {
        let profile = ["USERPROFILE", "HOME"]
            .iter()
            .find_map(|name| std::env::var_os(name).filter(|value| !value.is_empty()))
            .map(PathBuf::from);
        Redactor::new(profile.as_deref())
    }

    /// A line with the profile folder as `~`.
    pub fn text(&self, text: &str) -> String {
        if self.forms.is_empty() {
            return text.to_string();
        }
        let mut result = String::with_capacity(text.len());
        let mut at = 0;
        'outer: while at < text.len() {
            for form in &self.forms {
                if let Some(length) = path_prefix(&text[at..], form)
                    && text[at + length..].chars().next().is_none_or(is_separator)
                {
                    result.push('~');
                    at += length;
                    continue 'outer;
                }
            }
            let next = text[at..].chars().next().map_or(1, char::len_utf8);
            result.push_str(&text[at..at + next]);
            at += next;
        }
        result
    }

    /// A path as a line shows it, with the profile folder as `~`.
    pub fn path(&self, path: &Path) -> String {
        self.text(&path.display().to_string())
    }
}

/// The length in bytes of the start of `text` that spells the path `form`: character by character in any
/// case, with `\` and `/` as the same separator; `None` when `text` does not start with it.
fn path_prefix(text: &str, form: &str) -> Option<usize> {
    let mut chars = text.char_indices();
    for wanted in form.chars() {
        let (_, found) = chars.next()?;
        let same = found == wanted
            || (is_separator(found) && is_separator(wanted))
            || found.to_lowercase().eq(wanted.to_lowercase());
        if !same {
            return None;
        }
    }
    Some(chars.next().map_or(text.len(), |(index, _)| index))
}

/// A path separator as Windows reads one: `\` or `/`.
fn is_separator(c: char) -> bool {
    matches!(c, '\\' | '/')
}

/// Whether a flavor's `Interface\AddOns` folder takes a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonsStatus {
    /// The probe file was created and removed.
    Writable,
    /// The probe file could not be created; the reason says why.
    NotWritable,
    /// There is no `Interface\AddOns` folder (a client before its first launch) or no flavor folder; no
    /// probe is written.
    Missing,
}

/// The AddOns probe of one flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddonsCheck {
    pub status: AddonsStatus,
    pub reason: Option<String>,
}

/// The state sidecar of one flavor, as a command would find it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarStatus {
    /// `state.json` is there and reads.
    Parsed,
    /// `state.json` is there but corrupt, of another schema or of another flavor: the next command that
    /// reads it starts over from a scan (D-003).
    Rebuilt,
    /// No `state.json` yet: nothing was installed through mrgl.
    Missing,
    /// `state.json` cannot be read at all (a permission); the reason says why.
    Unreadable,
    /// No mrgl home folder, so no sidecar either.
    NoHome,
}

/// The sidecar check of one flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidecarCheck {
    pub status: SidecarStatus,
    /// The managed addons a parsed sidecar lists.
    pub managed: Option<usize>,
    pub reason: Option<String>,
}

/// The zip cache of one flavor (`<home>\<flavor-id>\cache`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheCheck {
    /// The bytes of every file under the folder; `0` when there is none.
    pub bytes: Option<u64>,
    pub files: Option<u64>,
    pub reason: Option<String>,
}

/// What to do with a kept staging folder (task 088): the report names it, the user decides.
pub const KEPT_STAGING_ADVICE: &str = "a failed swap moved the addon folders under previous aside; if one of those addons is missing or broken in Interface\\AddOns, copy its folder back from there, then delete the staging folder (mrgl never removes it)";

/// A run folder under the flavor's `staging\` that the sweep keeps because it holds a `previous\` folder
/// (task 088, [`staging::kept`]): the addon folders an interrupted or failed swap moved aside.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeptStaging {
    /// The run folder, the profile folder as `~`.
    pub path: String,
    /// When it was made, RFC 3339 in UTC; `None` when neither its name nor the file system says.
    pub created: Option<String>,
    /// Its age in whole seconds when the report was made.
    pub age_seconds: Option<u64>,
    /// The bytes and files under it, links and junctions not followed.
    pub bytes: Option<u64>,
    pub files: Option<u64>,
    /// The names under its `previous\` folder.
    pub previous: Vec<String>,
    /// What to do with it: [`KEPT_STAGING_ADVICE`].
    pub advice: &'static str,
}

/// Whether the game client runs from one flavor folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    Running,
    NotRunning,
    /// The process list could not be taken.
    Unknown,
}

/// The running-client check of one flavor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientCheck {
    pub status: ClientStatus,
    pub pid: Option<u32>,
    /// The executable's path, the profile folder as `~`.
    pub exe: Option<String>,
    pub reason: Option<String>,
}

/// One flavor of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlavorReport {
    /// The id `.flavor.info` holds, or the asked one when the folder is missing.
    pub id: String,
    /// The folder's name (`_forever_`).
    pub folder: String,
    /// The folder, the profile folder as `~`.
    pub path: String,
    /// Whether the folder was there with a readable `.flavor.info`.
    pub found: bool,
    /// The installed build (`1.60.1.70245`), and the interface number it gives.
    pub version: Option<String>,
    pub interface: Option<u32>,
    /// Where the build came from: `build-info` or `client-exe` (task 019), and the file.
    pub version_source: Option<&'static str>,
    pub version_file: Option<String>,
    /// The CurseForge app's `.curseclient` marker is in the folder.
    pub curse_client: bool,
    pub addons: AddonsCheck,
    pub sidecar: SidecarCheck,
    pub cache: CacheCheck,
    /// The run folders under `staging\` kept for their `previous\` folder (task 088).
    pub kept_staging: Vec<KeptStaging>,
    pub client: ClientCheck,
    pub warnings: Vec<String>,
}

/// The diagnostic report: what `mrgl doctor` prints and the window copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub mrgl_version: String,
    /// The Windows version (task 051).
    pub windows: WindowsVersion,
    /// The game root, the profile folder as `~`; `None` when nothing names or finds one.
    pub root: Option<String>,
    pub root_origin: Option<RootOrigin>,
    /// The mrgl home folder, the profile folder as `~`.
    pub home: Option<String>,
    pub webview2: Option<WebView2>,
    pub github_rate_limit: RateLimit,
    pub api_keys: Vec<ApiKey>,
    pub flavors: Vec<FlavorReport>,
    pub warnings: Vec<String>,
}

/// Everything the report reads besides the probes, given explicitly.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// The resolved game root.
    pub root: Option<GameRoot>,
    /// The one flavor to report; every flavor of the root when `None`.
    pub only: Option<FlavorId>,
    /// The mrgl home folder.
    pub home: Option<PathBuf>,
    /// The GitHub token, sent with the rate limit request and never printed.
    pub token: Option<String>,
    pub api_keys: Vec<ApiKey>,
    /// Warnings met before the report (an unreadable config).
    pub warnings: Vec<String>,
    pub redactor: Redactor,
}

/// The stubbable parts: the process list, the network and the registry.
pub struct Probes<'a> {
    pub processes: &'a dyn ProcessList,
    pub http: &'a dyn Http,
    pub runtime: &'a dyn RuntimeLookup,
}

/// The report of this process, the call of the command line and the window: the root as every command
/// resolves it (a flag, `MRGL_ROOT`, the remembered one, the discovery), the home folder, the keys and
/// the token from the environment, the system's process list, the network or `MRGL_HTTP_STUB`, and the
/// registry or `MRGL_DISCOVERY`.
pub fn report_of_process(root_flag: Option<PathBuf>, only: Option<FlavorId>) -> Report {
    let mut warnings = Vec::new();
    let root = match discover::resolve_root_of_process(root_flag) {
        Ok(resolution) => {
            warnings.extend(resolution.warnings);
            resolution.root
        }
        Err(error) => {
            warnings.push(error.to_string());
            None
        }
    };
    let token = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|token| !token.is_empty());
    let inputs = Inputs {
        root,
        only,
        home: home::home_dir().ok(),
        token,
        api_keys: api_keys_from(|name| std::env::var_os(name)),
        warnings,
        redactor: Redactor::of_process(),
    };
    let http = sources::http_from_env();
    let runtime = runtime_from_env(std::env::var_os(DISCOVERY_ENV).as_deref());
    report(
        inputs,
        &Probes {
            processes: &SystemProcesses,
            http: http.as_ref(),
            runtime: runtime.as_ref(),
        },
    )
}

/// The report from explicit inputs and probes. Every path and every line that may hold one goes through
/// the redactor.
pub fn report(inputs: Inputs, probes: &Probes<'_>) -> Report {
    let redactor = &inputs.redactor;
    let mut warnings = inputs.warnings.clone();
    let mut flavors = Vec::new();
    if let Some(root) = &inputs.root {
        match flavor::list_flavors(&root.path) {
            Ok(found) => {
                let mut chosen: Vec<Flavor> = found
                    .into_iter()
                    .filter(|flavor| inputs.only.as_ref().is_none_or(|only| *only == flavor.id))
                    .collect();
                if chosen.is_empty()
                    && let Some(only) = &inputs.only
                {
                    flavors.push(missing_flavor(&root.path, only, &inputs, probes));
                }
                for flavor in chosen.drain(..) {
                    flavors.push(flavor_report(&flavor, &inputs, probes));
                }
            }
            Err(error) => warnings.push(error.to_string()),
        }
    }
    Report {
        mrgl_version: crate::VERSION.to_string(),
        windows: probes.runtime.windows(),
        root: inputs.root.as_ref().map(|root| redactor.path(&root.path)),
        root_origin: inputs.root.as_ref().map(|root| root.origin),
        home: inputs.home.as_deref().map(|home| redactor.path(home)),
        webview2: probes.runtime.webview2(),
        github_rate_limit: rate_limit(probes.http, inputs.token.as_deref()),
        api_keys: inputs.api_keys.clone(),
        flavors,
        warnings: warnings.iter().map(|line| redactor.text(line)).collect(),
    }
}

/// A flavor read from its folder.
fn flavor_report(flavor: &Flavor, inputs: &Inputs, probes: &Probes<'_>) -> FlavorReport {
    let redactor = &inputs.redactor;
    FlavorReport {
        id: flavor.id.to_string(),
        folder: flavor.folder_name(),
        path: redactor.path(&flavor.folder),
        found: true,
        version: flavor.version.map(|version| version.to_string()),
        interface: flavor.interface(),
        version_source: flavor.version_source.as_ref().map(|source| source.kind()),
        version_file: flavor
            .version_source
            .as_ref()
            .map(|source| source.file_name().to_string()),
        curse_client: flavor.curse_client,
        addons: addons_check(&flavor.folder, redactor),
        sidecar: sidecar_check(inputs.home.as_deref(), &flavor.id, redactor),
        cache: cache_check(inputs.home.as_deref(), &flavor.id, redactor),
        kept_staging: kept_staging(inputs.home.as_deref(), &flavor.id, redactor),
        client: client_check(probes.processes, &flavor.folder, redactor),
        warnings: flavor
            .warnings
            .iter()
            .map(|line| redactor.text(line))
            .collect(),
    }
}

/// The asked flavor whose folder is not under the root: the home's sidecar and cache still say what
/// mrgl remembers of it; nothing is probed in the game folder.
fn missing_flavor(
    root: &Path,
    id: &FlavorId,
    inputs: &Inputs,
    probes: &Probes<'_>,
) -> FlavorReport {
    let redactor = &inputs.redactor;
    let folder_name = id
        .folder_name()
        .map(str::to_string)
        .unwrap_or_else(|| id.to_string());
    let folder = root.join(&folder_name);
    FlavorReport {
        id: id.to_string(),
        path: redactor.path(&folder),
        found: false,
        version: None,
        interface: None,
        version_source: None,
        version_file: None,
        curse_client: false,
        addons: AddonsCheck {
            status: AddonsStatus::Missing,
            reason: None,
        },
        sidecar: sidecar_check(inputs.home.as_deref(), id, redactor),
        cache: cache_check(inputs.home.as_deref(), id, redactor),
        kept_staging: kept_staging(inputs.home.as_deref(), id, redactor),
        client: client_check(probes.processes, &folder, redactor),
        warnings: vec![format!(
            "no flavor folder {folder_name} with a .flavor.info under the game root"
        )],
        folder: folder_name,
    }
}

/// A counter for the probe's name, so two probes of one process never share a file.
static PROBE: AtomicU64 = AtomicU64::new(0);

/// Creates and removes an empty probe file in the flavor's `Interface\AddOns`; a folder that is not
/// there is `missing` and gets no probe.
fn addons_check(flavor_folder: &Path, redactor: &Redactor) -> AddonsCheck {
    let addons = scan::addons_dir(flavor_folder);
    if !flavor_folder.is_dir() || !addons.is_dir() {
        return AddonsCheck {
            status: AddonsStatus::Missing,
            reason: None,
        };
    }
    let probe = addons.join(format!(
        "{PROBE_PREFIX}{}-{}",
        std::process::id(),
        PROBE.fetch_add(1, Ordering::Relaxed)
    ));
    match std::fs::File::create(&probe) {
        Ok(file) => {
            drop(file);
            let reason = std::fs::remove_file(&probe).err().map(|error| {
                redactor.text(&format!(
                    "the probe file {} could not be removed: {error}",
                    probe.display()
                ))
            });
            AddonsCheck {
                status: AddonsStatus::Writable,
                reason,
            }
        }
        Err(error) => AddonsCheck {
            status: AddonsStatus::NotWritable,
            reason: Some(error.to_string()),
        },
    }
}

/// How the sidecar would read: missing, parsed with its managed addons, or rebuilt; nothing is written.
fn sidecar_check(home: Option<&Path>, id: &FlavorId, redactor: &Redactor) -> SidecarCheck {
    let check = |status, managed, reason: Option<String>| SidecarCheck {
        status,
        managed,
        reason: reason.map(|reason| redactor.text(&reason)),
    };
    let Some(home) = home else {
        return check(SidecarStatus::NoHome, None, None);
    };
    let path = FlavorHome::at(home, id).state_path();
    if !path.exists() {
        return check(SidecarStatus::Missing, None, None);
    }
    match state::read_state_at(&path, id) {
        Ok(loaded) => match loaded.warning {
            Some(warning) => check(SidecarStatus::Rebuilt, None, Some(warning)),
            None => check(SidecarStatus::Parsed, Some(loaded.state.addons.len()), None),
        },
        Err(error) => check(SidecarStatus::Unreadable, None, Some(error.to_string())),
    }
}

/// The bytes and files under the flavor's cache folder, junctions and links not followed; no folder is
/// an empty cache.
fn cache_check(home: Option<&Path>, id: &FlavorId, redactor: &Redactor) -> CacheCheck {
    let Some(home) = home else {
        return CacheCheck {
            bytes: None,
            files: None,
            reason: Some("no mrgl home folder".to_string()),
        };
    };
    let dir = FlavorHome::at(home, id).cache_dir();
    if !dir.exists() {
        return CacheCheck {
            bytes: Some(0),
            files: Some(0),
            reason: None,
        };
    }
    match folder_size(&dir) {
        Ok((bytes, files)) => CacheCheck {
            bytes: Some(bytes),
            files: Some(files),
            reason: None,
        },
        Err(error) => CacheCheck {
            bytes: None,
            files: None,
            reason: Some(redactor.text(&format!("{}: {error}", dir.display()))),
        },
    }
}

/// The run folders under the flavor's `staging\` kept for their `previous\` folder (task 088), with their
/// age and size; none without a home folder.
fn kept_staging(home: Option<&Path>, id: &FlavorId, redactor: &Redactor) -> Vec<KeptStaging> {
    let Some(home) = home else {
        return Vec::new();
    };
    let now = std::time::SystemTime::now();
    staging::kept(&FlavorHome::at(home, id).staging_dir())
        .into_iter()
        .map(|run| {
            let (bytes, files) = folder_size(&run.path)
                .map(|(bytes, files)| (Some(bytes), Some(files)))
                .unwrap_or((None, None));
            KeptStaging {
                path: redactor.path(&run.path),
                created: run.created.map(crate::backup::rfc3339_utc),
                age_seconds: run.created.map(|created| {
                    now.duration_since(created)
                        .map(|age| age.as_secs())
                        .unwrap_or(0)
                }),
                bytes,
                files,
                previous: run.previous,
                advice: KEPT_STAGING_ADVICE,
            }
        })
        .collect()
}

/// An age for a human: `less than a minute`, `5 minutes`, `2 hours`, `3 days`, whole units rounded down.
fn age_text(seconds: u64) -> String {
    let (count, unit) = match seconds {
        0..60 => return "less than a minute".to_string(),
        60..3600 => (seconds / 60, "minute"),
        3600..86400 => (seconds / 3600, "hour"),
        _ => (seconds / 86400, "day"),
    };
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

/// The bytes and the number of files under a folder; a link or a junction counts as nothing.
fn folder_size(dir: &Path) -> std::io::Result<(u64, u64)> {
    let mut bytes = 0;
    let mut files = 0;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let metadata = std::fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                bytes += metadata.len();
                files += 1;
            }
        }
    }
    Ok((bytes, files))
}

/// Whether a client runs from the folder, through the process list it is given.
fn client_check(processes: &dyn ProcessList, folder: &Path, redactor: &Redactor) -> ClientCheck {
    match client::client_running_among(processes, folder) {
        Ok(Some(client)) => ClientCheck {
            status: ClientStatus::Running,
            pid: Some(client.pid),
            exe: Some(redactor.path(&client.exe)),
            reason: None,
        },
        Ok(None) => ClientCheck {
            status: ClientStatus::NotRunning,
            pid: None,
            exe: None,
            reason: None,
        },
        Err(error) => ClientCheck {
            status: ClientStatus::Unknown,
            pid: None,
            exe: None,
            reason: Some(error.to_string()),
        },
    }
}

fn with_reason(text: &str, reason: Option<&String>) -> String {
    match reason {
        Some(reason) => format!("{text} ({reason})"),
        None => text.to_string(),
    }
}

impl FlavorReport {
    /// The flavor's lines: a header, then one indented line per fact.
    fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.found {
            lines.push(format!("flavor: {} ({})", self.folder, self.id));
        } else {
            lines.push(format!(
                "flavor: {} ({}), folder missing",
                self.folder, self.id
            ));
        }
        lines.push(format!("  folder: {}", self.path));
        if self.found {
            let build = match (&self.version, self.interface, &self.version_file) {
                (Some(version), Some(interface), Some(file)) => {
                    format!("{version}, interface {interface}, from {file}")
                }
                (Some(version), _, _) => version.clone(),
                _ => "unknown".to_string(),
            };
            lines.push(format!("  build: {build}"));
            lines.push(format!(
                "  curseforge app: {}",
                if self.curse_client { "yes" } else { "no" }
            ));
        }
        let addons = match self.addons.status {
            AddonsStatus::Writable => "writable",
            AddonsStatus::NotWritable => "not writable",
            AddonsStatus::Missing => "missing",
        };
        lines.push(format!(
            "  addons folder: {}",
            with_reason(addons, self.addons.reason.as_ref())
        ));
        let sidecar = match (self.sidecar.status, self.sidecar.managed) {
            (SidecarStatus::Parsed, Some(1)) => "parsed, 1 managed addon".to_string(),
            (SidecarStatus::Parsed, Some(count)) => format!("parsed, {count} managed addons"),
            (SidecarStatus::Parsed, None) => "parsed".to_string(),
            (SidecarStatus::Rebuilt, _) => "rebuilt".to_string(),
            (SidecarStatus::Missing, _) => "missing".to_string(),
            (SidecarStatus::Unreadable, _) => "unreadable".to_string(),
            (SidecarStatus::NoHome, _) => "unknown, no mrgl home folder".to_string(),
        };
        lines.push(format!(
            "  sidecar: {}",
            with_reason(&sidecar, self.sidecar.reason.as_ref())
        ));
        let cache = match (self.cache.bytes, self.cache.files) {
            (Some(0), _) | (_, Some(0)) => "empty".to_string(),
            (Some(bytes), Some(1)) => format!("{} in 1 file", zip_entry::size_text(bytes)),
            (Some(bytes), Some(files)) => {
                format!("{} in {files} files", zip_entry::size_text(bytes))
            }
            _ => "unknown".to_string(),
        };
        lines.push(format!(
            "  cache: {}",
            with_reason(&cache, self.cache.reason.as_ref())
        ));
        for kept in &self.kept_staging {
            let age = kept
                .age_seconds
                .map(|seconds| format!("{} old", age_text(seconds)))
                .unwrap_or_else(|| "age unknown".to_string());
            let size = match (kept.bytes, kept.files) {
                (Some(bytes), Some(1)) => format!("{} in 1 file", zip_entry::size_text(bytes)),
                (Some(bytes), Some(files)) => {
                    format!("{} in {files} files", zip_entry::size_text(bytes))
                }
                _ => "size unknown".to_string(),
            };
            let previous = if kept.previous.is_empty() {
                "nothing".to_string()
            } else {
                kept.previous.join(", ")
            };
            lines.push(format!(
                "  kept staging: {} ({age}, {size}; previous: {previous})",
                kept.path
            ));
        }
        if !self.kept_staging.is_empty() {
            lines.push(format!("  kept staging advice: {KEPT_STAGING_ADVICE}"));
        }
        let client = match (self.client.status, self.client.pid, &self.client.exe) {
            (ClientStatus::Running, Some(pid), Some(exe)) => {
                let name = Path::new(exe)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| exe.clone());
                format!("running ({name}, pid {pid})")
            }
            (ClientStatus::Running, _, _) => "running".to_string(),
            (ClientStatus::NotRunning, _, _) => "not running".to_string(),
            (ClientStatus::Unknown, _, _) => "unknown".to_string(),
        };
        lines.push(format!(
            "  client: {}",
            with_reason(&client, self.client.reason.as_ref())
        ));
        lines.extend(
            self.warnings
                .iter()
                .map(|warning| format!("  warning: {warning}")),
        );
        lines
    }

    /// What a reader should look at in this flavor.
    fn concerns(&self) -> Vec<String> {
        let mut concerns = Vec::new();
        if !self.found {
            concerns.push(format!("{} is missing", self.folder));
        }
        if self.addons.status == AddonsStatus::NotWritable {
            concerns.push(format!(
                "the AddOns folder of {} is not writable",
                self.folder
            ));
        }
        match self.sidecar.status {
            SidecarStatus::Rebuilt => {
                concerns.push(format!("the sidecar of {} is rebuilt", self.folder))
            }
            SidecarStatus::Unreadable => {
                concerns.push(format!("the sidecar of {} is unreadable", self.folder))
            }
            _ => {}
        }
        match self.kept_staging.len() {
            0 => {}
            1 => concerns.push(format!("a kept staging folder of {}", self.folder)),
            count => concerns.push(format!("{count} kept staging folders of {}", self.folder)),
        }
        if self.client.status == ClientStatus::Unknown {
            concerns.push("the process list is unreadable".to_string());
        }
        if self.found && !self.warnings.is_empty() {
            concerns.push(format!("{} has a warning", self.folder));
        }
        concerns
    }
}

impl Report {
    /// The plain block, one fact per line, as a bug report carries it: the general lines, then one block
    /// per flavor, then the warnings. Every line's meaning is in `docs/development.md`, "The doctor".
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("mrgl: {}", self.mrgl_version),
            format!("windows: {}", self.windows.text()),
        ];
        lines.push(match (&self.root, self.root_origin) {
            (Some(root), Some(origin)) => {
                let how = match origin {
                    RootOrigin::Flag => "from --root",
                    RootOrigin::Environment => "from MRGL_ROOT",
                    RootOrigin::Remembered => "remembered",
                    RootOrigin::Registry => "found in the launcher's registry key",
                    RootOrigin::UninstallEntry => "found in the game's Uninstall registry entry",
                    RootOrigin::CommonFolder => "found in a usual install folder",
                };
                format!("root: {root} ({how})")
            }
            (Some(root), None) => format!("root: {root}"),
            _ => "root: none found".to_string(),
        });
        lines.push(match &self.home {
            Some(home) => format!("home: {home}"),
            None => "home: none (neither MRGL_HOME nor LOCALAPPDATA is set)".to_string(),
        });
        lines.push(match &self.webview2 {
            Some(runtime) => format!(
                "webview2: {} ({})",
                runtime.version,
                runtime.scope.description()
            ),
            None => "webview2: unknown".to_string(),
        });
        let rate = &self.github_rate_limit;
        lines.push(match (rate.remaining, rate.limit) {
            (Some(remaining), Some(limit)) => {
                let reset = rate
                    .reset
                    .as_ref()
                    .map(|reset| format!(", resets {reset}"))
                    .unwrap_or_default();
                format!("github rate limit: {remaining} of {limit} left{reset}")
            }
            _ => format!(
                "github rate limit: {}",
                with_reason("unknown", rate.reason.as_ref())
            ),
        });
        let keys: Vec<String> = self
            .api_keys
            .iter()
            .map(|key| {
                format!(
                    "{} {}",
                    key.name,
                    if key.present { "set" } else { "not set" }
                )
            })
            .collect();
        lines.push(format!("api keys: {}", keys.join(", ")));
        for flavor in &self.flavors {
            lines.extend(flavor.lines());
        }
        lines.extend(
            self.warnings
                .iter()
                .map(|warning| format!("warning: {warning}")),
        );
        lines
    }

    /// The plain block as one text, the lines joined with a line break: what the window copies.
    pub fn text(&self) -> String {
        self.lines().join("\n")
    }

    /// What a reader should look at: no game root, a missing flavor, an AddOns folder that takes no
    /// write, a sidecar that is rebuilt or unreadable, a kept staging folder (task 088), an unreadable
    /// process list, a used-up rate limit and every warning.
    pub fn concerns(&self) -> Vec<String> {
        let mut concerns = Vec::new();
        if self.root.is_none() {
            concerns.push("no game root".to_string());
        }
        for flavor in &self.flavors {
            for concern in flavor.concerns() {
                if !concerns.contains(&concern) {
                    concerns.push(concern);
                }
            }
        }
        if self.github_rate_limit.remaining == Some(0) {
            concerns.push("the GitHub rate limit is used up".to_string());
        }
        if !self.warnings.is_empty() {
            concerns.push(match self.warnings.len() {
                1 => "1 warning".to_string(),
                count => format!("{count} warnings"),
            });
        }
        concerns
    }

    /// The closing message: its kind (success when nothing needs a look, a warning otherwise) and its
    /// plain line.
    pub fn summary(&self) -> (Kind, String) {
        let flavors = match self.flavors.len() {
            0 => "no flavor".to_string(),
            1 => "1 flavor".to_string(),
            count => format!("{count} flavors"),
        };
        let concerns = self.concerns();
        let paste = "paste the lines above into a bug report";
        match concerns.len() {
            0 => (
                Kind::Success,
                format!("diagnostics of {flavors}, nothing to look at; {paste}"),
            ),
            count => (
                Kind::Warning,
                format!(
                    "diagnostics of {flavors}, {} to look at ({}); {paste}",
                    if count == 1 {
                        "1 thing".to_string()
                    } else {
                        format!("{count} things")
                    },
                    concerns.join(", ")
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::FixedProcesses;
    use crate::sources::{MapHttp, Response};

    /// A fresh temporary folder for one test, removed first.
    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-doctor-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the folder");
        dir
    }

    /// A synthetic game root: `_forever_` with an AddOns folder and one addon, `_retail_` with no
    /// `Interface` folder yet, and a `.build.info` with a row for each.
    fn game_root(under: &Path) -> PathBuf {
        let root = under.join("World of Warcraft");
        let addon = root
            .join("_forever_")
            .join("Interface")
            .join("AddOns")
            .join("Made");
        std::fs::create_dir_all(&addon).expect("the addon folder");
        std::fs::write(
            addon.join("Made.toc"),
            "## Interface: 16001\n## Title: Made\n",
        )
        .expect("toc");
        std::fs::create_dir_all(root.join("_retail_")).expect("the retail folder");
        std::fs::write(
            root.join(".build.info"),
            "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\nwow|12.1.5.70448\n",
        )
        .expect(".build.info");
        for (folder, id) in [("_forever_", "wowf"), ("_retail_", "wow")] {
            std::fs::write(
                root.join(folder).join(".flavor.info"),
                format!("Product Flavor!STRING:0\n{id}\n"),
            )
            .expect(".flavor.info");
        }
        root
    }

    /// A process list that cannot be taken.
    struct BrokenProcesses;

    impl ProcessList for BrokenProcesses {
        fn executables(&self) -> std::io::Result<Vec<client::RunningProcess>> {
            Err(std::io::Error::other("no snapshot"))
        }
    }

    /// A network that is not there.
    struct Offline;

    impl Http for Offline {
        fn get(&self, request: &Request) -> crate::error::Result<Response> {
            Err(crate::Error::Network {
                url: request.url.clone(),
                reason: "no route to host".to_string(),
            })
        }
    }

    const RATE_LIMIT: &str = r#"{"resources":{"core":{"limit":5000,"used":3,"remaining":4997,"reset":1792598400}},"rate":{"limit":5000,"remaining":4997,"reset":1792598400}}"#;
    const SECRET: &str = "ghp_madeUpTokenValueForTheDoctorTest";

    #[test]
    fn a_report_on_a_fixture_root_has_every_line_and_no_profile_path() {
        let base = temp_dir("every-line");
        let profile = base.join("Users").join("someone");
        let root = game_root(&profile.join("Games"));
        let home = profile.join("AppData").join("Local").join("mrgl");
        // A parsed sidecar with one addon for Forever, a corrupt one for retail, and a cached zip.
        let forever = FlavorHome::at(&home, &FlavorId::Forever);
        let mut state = state::State::new(&FlavorId::Forever);
        state.addons.insert(
            "Made".to_string(),
            state::Entry {
                source: "github:made/up".to_string(),
                version: "1.0.0".to_string(),
                previous_version: None,
                folders: vec!["Made".to_string()],
                files: Default::default(),
                installed_at: "2026-10-21T12:00:00Z".to_string(),
                nolib: None,
                pinned: None,
                ignored: false,
                channel: crate::sources::Channel::Stable,
            },
        );
        forever.write_state(&state).expect("the sidecar");
        let zip = forever.cached_zip("Made", "1.0.0");
        std::fs::create_dir_all(zip.parent().expect("a parent")).expect("the cache folder");
        std::fs::write(&zip, [0u8; 2048]).expect("the cached zip");
        let retail = FlavorHome::at(&home, &FlavorId::Retail);
        std::fs::create_dir_all(&retail.dir).expect("the retail home");
        std::fs::write(retail.state_path(), "{ not json").expect("a corrupt sidecar");

        let client_exe = root.join("_forever_").join("WowForever.exe");
        let processes = FixedProcesses::new([(4242, client_exe)]);
        let http = MapHttp::new().answer("https://api.github.com/rate_limit", 200, RATE_LIMIT);
        let runtime = FixedRuntime {
            webview2: Some(WebView2 {
                version: "131.0.2903.70".to_string(),
                scope: Scope::Machine,
            }),
            windows: WindowsVersion::from_values(
                Some("Windows 10 Pro"),
                Some("24H2"),
                Some("26100"),
                Some(1234),
            ),
        };
        let inputs = Inputs {
            root: Some(GameRoot {
                path: root.clone(),
                origin: RootOrigin::Registry,
            }),
            only: None,
            home: Some(home.clone()),
            token: Some(SECRET.to_string()),
            api_keys: api_keys_from(|name| (name == TOKEN_ENV).then(|| OsString::from(SECRET))),
            warnings: vec![format!("a warning about {}", profile.join("x").display())],
            redactor: Redactor::new(Some(&profile)),
        };
        let report = report(
            inputs,
            &Probes {
                processes: &processes,
                http: &http,
                runtime: &runtime,
            },
        );
        let text = report.text();
        let json = serde_json::to_string(&report).expect("serializes");

        let expected = [
            format!("mrgl: {}", crate::VERSION),
            "windows: Windows 11 24H2, build 26100.1234".to_string(),
            "root: ~\\Games\\World of Warcraft (found in the launcher's registry key)".to_string(),
            "home: ~\\AppData\\Local\\mrgl".to_string(),
            "webview2: 131.0.2903.70 (per-machine)".to_string(),
            "github rate limit: 4997 of 5000 left, resets 2026-10-21T16:00:00Z".to_string(),
            format!("api keys: {TOKEN_ENV} set"),
            "flavor: _forever_ (wowf)".to_string(),
            "  folder: ~\\Games\\World of Warcraft\\_forever_".to_string(),
            "  build: 1.60.1.70245, interface 16001, from .build.info".to_string(),
            "  curseforge app: no".to_string(),
            "  addons folder: writable".to_string(),
            "  sidecar: parsed, 1 managed addon".to_string(),
            "  cache: 2.0 KB in 1 file".to_string(),
            "  client: running (WowForever.exe, pid 4242)".to_string(),
            "flavor: _retail_ (wow)".to_string(),
            "  folder: ~\\Games\\World of Warcraft\\_retail_".to_string(),
            "  build: 12.1.5.70448, interface 120105, from .build.info".to_string(),
            "  curseforge app: no".to_string(),
            "  addons folder: missing".to_string(),
            "  cache: empty".to_string(),
            "  client: not running".to_string(),
            "warning: a warning about ~\\x".to_string(),
        ];
        if cfg!(windows) {
            let lines = report.lines();
            for line in &expected {
                assert!(lines.contains(line), "missing {line:?} in:\n{text}");
            }
        }
        assert!(
            text.contains("  sidecar: rebuilt (unreadable state sidecar"),
            "{text}"
        );
        // No home folder path and no key value, in the plain block or in the JSON.
        let profile_text = profile.display().to_string();
        for output in [&text, &json] {
            assert!(!output.contains(&profile_text), "{output}");
            assert!(!output.contains(SECRET), "{output}");
        }
        // The token went to the API, and only there.
        let requests = http.requests();
        assert_eq!(requests.len(), 1, "one request");
        assert_eq!(
            requests[0].header_value("Authorization"),
            Some(format!("Bearer {SECRET}").as_str())
        );
        // The probe left nothing behind.
        let left: Vec<_> = std::fs::read_dir(root.join("_forever_/Interface/AddOns"))
            .expect("the AddOns folder")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(left, [OsString::from("Made")]);
        assert!(
            !root.join("_retail_/Interface").exists(),
            "no probe where no folder is"
        );

        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(value["root_origin"], "registry");
        assert_eq!(
            value["windows"],
            serde_json::json!({
                "name": "Windows 11",
                "display_version": "24H2",
                "build": 26100,
                "ubr": 1234,
                "product_name": "Windows 10 Pro",
            })
        );
        assert_eq!(value["webview2"]["version"], "131.0.2903.70");
        assert_eq!(value["webview2"]["scope"], "machine");
        assert_eq!(value["github_rate_limit"]["remaining"], 4997);
        assert_eq!(value["api_keys"][0]["name"], TOKEN_ENV);
        assert_eq!(value["api_keys"][0]["present"], true);
        assert_eq!(value["flavors"][0]["id"], "wowf");
        assert_eq!(value["flavors"][0]["addons"]["status"], "writable");
        assert_eq!(value["flavors"][0]["sidecar"]["status"], "parsed");
        assert_eq!(value["flavors"][0]["sidecar"]["managed"], 1);
        assert_eq!(value["flavors"][0]["cache"]["bytes"], 2048);
        assert_eq!(value["flavors"][0]["client"]["status"], "running");
        assert_eq!(value["flavors"][0]["client"]["pid"], 4242);
        assert_eq!(value["flavors"][1]["addons"]["status"], "missing");
        assert_eq!(value["flavors"][1]["sidecar"]["status"], "rebuilt");
        assert_eq!(value["flavors"][1]["client"]["status"], "not_running");

        let (kind, summary) = report.summary();
        assert_eq!(kind, Kind::Warning);
        assert_eq!(
            summary,
            "diagnostics of 2 flavors, 2 things to look at (the sidecar of _retail_ is rebuilt, 1 warning); paste the lines above into a bug report"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn offline_with_no_root_every_value_is_unknown_and_nothing_fails() {
        let report = report(
            Inputs {
                api_keys: api_keys_from(|_| None),
                ..Inputs::default()
            },
            &Probes {
                processes: &BrokenProcesses,
                http: &Offline,
                runtime: &FixedRuntime::default(),
            },
        );
        let lines = report.lines();
        assert_eq!(
            lines,
            [
                format!("mrgl: {}", crate::VERSION),
                "windows: unknown".to_string(),
                "root: none found".to_string(),
                "home: none (neither MRGL_HOME nor LOCALAPPDATA is set)".to_string(),
                "webview2: unknown".to_string(),
                "github rate limit: unknown (offline: cannot reach https://api.github.com/rate_limit: no route to host)".to_string(),
                format!("api keys: {TOKEN_ENV} not set"),
            ],
            "{lines:#?}"
        );
        assert_eq!(report.flavors, []);
        let json = serde_json::to_value(&report).expect("serializes");
        assert_eq!(
            json["windows"],
            serde_json::json!({
                "name": null,
                "display_version": null,
                "build": null,
                "ubr": null,
                "product_name": null,
            }),
            "one object, every part null"
        );
        let (kind, summary) = report.summary();
        assert_eq!(kind, Kind::Warning);
        assert!(summary.contains("no game root"), "{summary}");
    }

    #[test]
    fn one_flavor_is_asked_for_and_a_missing_one_says_so() {
        let base = temp_dir("only");
        let root = game_root(&base);
        let home = base.join("home");
        let run = |only: FlavorId| {
            report(
                Inputs {
                    root: Some(GameRoot {
                        path: root.clone(),
                        origin: RootOrigin::Flag,
                    }),
                    only: Some(only),
                    home: Some(home.clone()),
                    ..Inputs::default()
                },
                &Probes {
                    processes: &BrokenProcesses,
                    http: &MapHttp::new(),
                    runtime: &FixedRuntime::default(),
                },
            )
        };
        let report = run(FlavorId::Forever);
        assert_eq!(report.flavors.len(), 1);
        assert_eq!(report.flavors[0].folder, "_forever_");
        assert_eq!(report.flavors[0].sidecar.status, SidecarStatus::Missing);
        assert_eq!(report.flavors[0].client.status, ClientStatus::Unknown);
        assert_eq!(report.github_rate_limit.reason.as_deref(), Some("HTTP 404"));
        assert!(
            report
                .lines()
                .contains(&"github rate limit: unknown (HTTP 404)".to_string())
        );

        let report = run(FlavorId::ClassicEra);
        assert_eq!(report.flavors.len(), 1);
        let era = &report.flavors[0];
        assert!(!era.found);
        assert_eq!(era.folder, "_classic_era_");
        assert_eq!(era.addons.status, AddonsStatus::Missing);
        assert!(
            report
                .lines()
                .contains(&"flavor: _classic_era_ (wow_classic_era), folder missing".to_string())
        );
        assert!(!root.join("_classic_era_").exists(), "nothing created");
        let _ = std::fs::remove_dir_all(&base);
    }

    // Task 088: the run folders under `staging\` that the sweep keeps for their `previous\` folder.

    /// A folder under `staging\` as a run two hours ago would have named it, with one file in it.
    fn planted_run(staging: &Path, stem: &str) -> PathBuf {
        let nanos = (std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 60 * 60))
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_nanos();
        let dir = staging.join(format!("{stem}-{nanos}-0"));
        std::fs::create_dir_all(&dir).expect("the run folder");
        std::fs::write(dir.join("left.txt"), "left over\n").expect("a file");
        dir
    }

    #[test]
    fn a_staging_folder_kept_for_its_previous_folders_is_named_with_its_age_size_and_advice() {
        let base = temp_dir("kept-staging");
        let profile = base.join("Users").join("someone");
        let root = game_root(&base);
        let home = profile.join("AppData").join("Local").join("mrgl");
        let staging = FlavorHome::at(&home, &FlavorId::Forever).staging_dir();
        let kept = planted_run(&staging, "Kept.zip");
        let previous = kept.join(staging::PREVIOUS_DIR).join("OneFolder");
        std::fs::create_dir_all(&previous).expect("the previous folder");
        std::fs::write(previous.join("OneFolder.toc"), "## Interface: 16001\n").expect("a TOC");
        let plain = planted_run(&staging, "Plain.zip");
        let run = |only: FlavorId| {
            report(
                Inputs {
                    root: Some(GameRoot {
                        path: root.clone(),
                        origin: RootOrigin::Flag,
                    }),
                    only: Some(only),
                    home: Some(home.clone()),
                    redactor: Redactor::new(Some(&profile)),
                    ..Inputs::default()
                },
                &Probes {
                    processes: &FixedProcesses::new(Vec::<(u32, PathBuf)>::new()),
                    http: &MapHttp::new().answer(
                        "https://api.github.com/rate_limit",
                        200,
                        RATE_LIMIT,
                    ),
                    runtime: &FixedRuntime::default(),
                },
            )
        };
        let report = run(FlavorId::Forever);
        let name = kept.file_name().expect("a name").to_string_lossy();
        let line = format!(
            "  kept staging: ~{sep}AppData{sep}Local{sep}mrgl{sep}wowf{sep}staging{sep}{name} (2 hours old, 30 B in 2 files; previous: OneFolder)",
            sep = std::path::MAIN_SEPARATOR
        );
        let lines = report.lines();
        let at = lines
            .iter()
            .position(|one| *one == line)
            .unwrap_or_else(|| panic!("missing {line:?} in {lines:#?}"));
        assert_eq!(
            lines[at + 1],
            format!("  kept staging advice: {KEPT_STAGING_ADVICE}")
        );
        let plain_name = plain.file_name().expect("a name").to_string_lossy();
        assert!(
            !lines.iter().any(|one| one.contains(plain_name.as_ref())),
            "a folder without previous is not named: {lines:#?}"
        );
        assert_eq!(
            lines
                .iter()
                .filter(|one| one.starts_with("  kept staging"))
                .count(),
            2
        );

        let value = serde_json::to_value(&report).expect("serializes");
        let kept_json = &value["flavors"][0]["kept_staging"];
        assert_eq!(kept_json.as_array().map(Vec::len), Some(1), "{kept_json}");
        assert!(
            kept_json[0]["path"]
                .as_str()
                .is_some_and(|path| path.starts_with('~') && path.ends_with(name.as_ref())),
            "{kept_json}"
        );
        assert_eq!(kept_json[0]["bytes"], 30);
        assert_eq!(kept_json[0]["files"], 2);
        assert_eq!(kept_json[0]["previous"], serde_json::json!(["OneFolder"]));
        assert_eq!(kept_json[0]["advice"], KEPT_STAGING_ADVICE);
        let age = kept_json[0]["age_seconds"].as_u64().expect("an age");
        assert!((7200..7260).contains(&age), "{age}");
        assert!(
            kept_json[0]["created"]
                .as_str()
                .is_some_and(|created| created.ends_with('Z')),
            "{kept_json}"
        );
        let (kind, summary) = report.summary();
        assert_eq!(kind, Kind::Warning);
        assert!(
            summary.contains("a kept staging folder of _forever_"),
            "{summary}"
        );
        assert!(
            !serde_json::to_string(&report)
                .expect("serializes")
                .contains(&profile.display().to_string()),
            "no profile path"
        );

        // Without the previous folder the run folder is the sweep's, and the doctor says nothing of it.
        std::fs::remove_dir_all(kept.join(staging::PREVIOUS_DIR)).expect("removes");
        let report = run(FlavorId::Forever);
        assert_eq!(report.flavors[0].kept_staging, []);
        assert!(
            !report
                .lines()
                .iter()
                .any(|one| one.starts_with("  kept staging")),
        );
        assert!(!report.summary().1.contains("kept staging"));
        let value = serde_json::to_value(&report).expect("serializes");
        assert_eq!(value["flavors"][0]["kept_staging"], serde_json::json!([]));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_age_reads_in_whole_units() {
        for (seconds, text) in [
            (0, "less than a minute"),
            (59, "less than a minute"),
            (60, "1 minute"),
            (3599, "59 minutes"),
            (3600, "1 hour"),
            (7300, "2 hours"),
            (86400, "1 day"),
            (3 * 86400 + 5, "3 days"),
        ] {
            assert_eq!(age_text(seconds), text, "{seconds}");
        }
    }

    #[test]
    fn the_redactor_hides_the_profile_folder_and_nothing_else() {
        let redactor = Redactor::new(Some(Path::new("C:\\Users\\someone")));
        assert_eq!(
            redactor.text("C:\\Users\\someone\\AppData\\Local\\mrgl"),
            "~\\AppData\\Local\\mrgl"
        );
        assert_eq!(redactor.text("c:\\users\\SOMEONE"), "~");
        assert_eq!(
            redactor.text("cannot read C:\\Users\\someone\\x and C:\\Users\\someone/y"),
            "cannot read ~\\x and ~/y"
        );
        assert_eq!(
            redactor.text("C:\\Users\\someone2\\x"),
            "C:\\Users\\someone2\\x",
            "another folder that starts the same"
        );
        assert_eq!(redactor.text("E:\\games\\wow"), "E:\\games\\wow");
        assert_eq!(redactor.text("é C:\\Users\\someone"), "é ~");
        // A profile variable too short to be a profile hides nothing.
        assert_eq!(
            Redactor::new(Some(Path::new("C:\\"))).text("C:\\x"),
            "C:\\x"
        );
        assert_eq!(
            Redactor::new(Some(Path::new("C:\\Users"))).text("C:\\Users\\x"),
            "C:\\Users\\x"
        );
        assert_eq!(Redactor::new(None).text("C:\\Users\\x"), "C:\\Users\\x");
    }

    #[test]
    fn the_redactor_hides_the_profile_folder_with_either_separator_in_any_case() {
        let redactor = Redactor::new(Some(Path::new("C:\\Users\\someone")));
        assert_eq!(
            redactor.text("C:/Users/someone/AppData/Local/mrgl"),
            "~/AppData/Local/mrgl"
        );
        assert_eq!(redactor.text("c:/USERS/SomeOne"), "~");
        assert_eq!(redactor.text("C:/Users\\someone/x"), "~/x", "mixed");
        assert_eq!(
            redactor.text("from C:\\USERS/someone\\x and c:/users/someone"),
            "from ~\\x and ~"
        );
        assert_eq!(
            redactor.text("C:/Users/someone2/x"),
            "C:/Users/someone2/x",
            "another folder that starts the same"
        );
        assert_eq!(
            redactor.text("C:/Users/someone.old"),
            "C:/Users/someone.old"
        );
        // A profile variable written with forward slashes hides the backslash form too.
        let forward = Redactor::new(Some(Path::new("C:/Users/someone/")));
        assert_eq!(forward.text("C:\\Users\\someone\\x"), "~\\x");
        assert_eq!(forward.text("c:/users/someone"), "~");
        // A name beyond ASCII matches in any case too.
        let accented = Redactor::new(Some(Path::new("C:\\Users\\Ébredő")));
        assert_eq!(accented.text("c:/users/ÉBREDŐ/x"), "~/x");
        assert_eq!(accented.text("C:\\Users\\ébredő"), "~");
        assert_eq!(accented.text("C:\\Users\\Ébredő2"), "C:\\Users\\Ébredő2");
    }

    #[cfg(windows)]
    #[test]
    fn the_redactor_hides_the_short_form_too() {
        let base = temp_dir("short-form");
        let profile = base.join("a profile folder name long enough");
        std::fs::create_dir_all(&profile).expect("the folder");
        let redactor = Redactor::new(Some(&profile));
        // With short names off on the volume the short form is the long one; either way both are hidden.
        let short = client::short_form(&profile).expect("a short form");
        assert_eq!(redactor.path(&short.join("x")), "~\\x");
        assert_eq!(redactor.path(&profile.join("x")), "~\\x");
        // Both forms with forward slashes and in upper case too.
        for form in [&short, &profile] {
            let forward = form.join("x").display().to_string().replace('\\', "/");
            assert_eq!(redactor.text(&forward), "~/x", "{forward}");
            assert_eq!(
                redactor.text(&forward.to_ascii_uppercase()),
                "~/X",
                "{forward}"
            );
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_rate_limit_is_unknown_for_anything_but_a_readable_answer() {
        let url = "https://api.github.com/rate_limit";
        let known = rate_limit(&MapHttp::new().answer(url, 200, RATE_LIMIT), None);
        assert_eq!(known.remaining, Some(4997));
        assert_eq!(known.limit, Some(5000));
        assert_eq!(known.reset.as_deref(), Some("2026-10-21T16:00:00Z"));
        assert_eq!(known.reason, None);
        let only_rate = rate_limit(
            &MapHttp::new().answer(url, 200, r#"{"rate":{"limit":60,"remaining":0}}"#),
            None,
        );
        assert_eq!((only_rate.remaining, only_rate.limit), (Some(0), Some(60)));
        for (status, body, reason) in [
            (403, "{}", "HTTP 403"),
            (200, "<html>", "an answer that is not JSON"),
            (200, "{}", "an answer without the numbers"),
        ] {
            let unknown = rate_limit(&MapHttp::new().answer(url, status, body), None);
            assert_eq!(unknown.reason.as_deref(), Some(reason));
            assert_eq!(unknown.remaining, None);
        }
        let http = MapHttp::new().answer(url, 200, RATE_LIMIT);
        rate_limit(&http, Some(""));
        assert_eq!(http.requests()[0].header_value("Authorization"), None);
    }

    #[test]
    fn the_discovery_variable_stubs_the_runtime() {
        assert_eq!(parse_runtime(None), None, "the registry");
        assert_eq!(parse_runtime(Some(OsStr::new(""))), None, "the registry");
        assert_eq!(
            parse_runtime(Some(OsStr::new("off"))),
            Some(FixedRuntime::default())
        );
        assert_eq!(
            parse_runtime(Some(OsStr::new("registry=Q:\\wow; webview2=131.0.2903.70"))),
            Some(FixedRuntime {
                webview2: Some(WebView2 {
                    version: "131.0.2903.70".to_string(),
                    scope: Scope::Machine,
                }),
                windows: WindowsVersion::default(),
            })
        );
        assert_eq!(
            parse_runtime(Some(OsStr::new("webview2=0.0.0.0"))),
            Some(FixedRuntime::default()),
            "an uninstalled runtime"
        );
        let windows = |value: &str| {
            parse_runtime(Some(OsStr::new(value)))
                .expect("a stub")
                .windows
                .text()
        };
        assert_eq!(
            windows("Q:\\wow;windows=24H2,26100.1234"),
            "Windows 11 24H2, build 26100.1234"
        );
        assert_eq!(windows("windows=,19045"), "Windows 10, build 19045");
        assert_eq!(windows("windows=24H2"), "Windows 24H2, build unknown");
        assert_eq!(windows("webview2=131.0.2903.70"), "unknown");
        assert_eq!(usable_version(" null "), None);
        assert_eq!(usable_version(""), None);
        assert_eq!(usable_version("120.0.1").as_deref(), Some("120.0.1"));
    }

    #[test]
    fn the_windows_line_says_what_the_values_hold_and_unknown_for_the_rest() {
        // Every value there: Windows 11 still writes `Windows 10` as its product name.
        let present = WindowsVersion::from_values(
            Some("Windows 10 Pro"),
            Some("24H2"),
            Some("26100"),
            Some(1234),
        );
        assert_eq!(present.name.as_deref(), Some("Windows 11"));
        assert_eq!(present.text(), "Windows 11 24H2, build 26100.1234");
        let windows_10 = WindowsVersion::from_values(
            Some("Windows 10 Home"),
            Some("22H2"),
            Some(" 19045 "),
            Some(5737),
        );
        assert_eq!(windows_10.text(), "Windows 10 22H2, build 19045.5737");
        let server = WindowsVersion::from_values(
            Some("Windows Server 2022 Datacenter"),
            Some("21H2"),
            Some("20348"),
            Some(2700),
        );
        assert_eq!(
            server.text(),
            "Windows Server 2022 Datacenter 21H2, build 20348.2700",
            "another product name stays as written"
        );

        // Partly there: a missing part is left out, a missing build says so.
        assert_eq!(
            WindowsVersion::from_values(None, None, Some("26100"), None).text(),
            "Windows 11, build 26100"
        );
        assert_eq!(
            WindowsVersion::from_values(Some("Windows 10 Pro"), Some(""), None, Some(1234)).text(),
            "Windows 10, build unknown",
            "an empty value is no value, a revision without a build is none"
        );
        assert_eq!(
            WindowsVersion::from_values(None, Some("24H2"), Some("not a number"), None).text(),
            "Windows 24H2, build unknown"
        );

        // Nothing there.
        let absent = WindowsVersion::from_values(None, None, None, None);
        assert_eq!(absent, WindowsVersion::default());
        assert_eq!(absent.text(), "unknown");
        assert_eq!(
            WindowsVersion::from_values(None, None, None, Some(1234)).text(),
            "unknown"
        );
    }

    #[test]
    fn the_keys_are_named_never_read_out() {
        let keys = api_keys_from(|_| Some(OsString::from("")));
        assert_eq!(
            keys,
            [ApiKey {
                name: TOKEN_ENV,
                present: false
            }],
            "an empty value is no key"
        );
        let keys = api_keys_from(|_| Some(OsString::from(SECRET)));
        assert!(keys[0].present);
        let json = serde_json::to_string(&keys).expect("serializes");
        assert!(!json.contains(SECRET), "{json}");
    }

    #[test]
    fn a_byte_count_reads_for_a_human() {
        assert_eq!(zip_entry::size_text(0), "0 B");
        assert_eq!(zip_entry::size_text(1023), "1023 B");
        assert_eq!(zip_entry::size_text(2048), "2.0 KB");
        assert_eq!(zip_entry::size_text(3 * 1024 * 1024 / 2), "1.5 MB");
    }
}
