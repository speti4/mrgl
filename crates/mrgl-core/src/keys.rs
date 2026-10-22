//! API keys (task 093): one place that names the keys of the sources, reads them, stores them and says
//! where each one comes from, never what it is. Only the user's own keys (D-002): mrgl ships none.
//!
//! A key is looked up in the environment first (`MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY`,
//! `MRGL_CURSEFORGE_KEY`; an empty variable is not set), then in the key store, the Windows Credential
//! Manager's generic credential `mrgl:<source>` (`mrgl:github`, `mrgl:wago`, `mrgl:curse`), which
//! `mrgl key set <source>` writes from the standard input and `mrgl key clear <source>` removes. Outside
//! Windows the store is not available (a key store there is M6); `MRGL_KEY_STORE=off` turns it off for a
//! run, which is how the end-to-end tests stay away from the real Credential Manager, and the unit tests
//! hand every function a [`MemoryStore`].
//!
//! A key's value lives in a [`Key`], whose `Debug` hides it, and goes to its source's hosts only
//! ([`may_send`]); a listing ([`KeyState`]: `mrgl key list`, the doctor) names the key and its [`Origin`]
//! and nothing else.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::BufRead;
use std::sync::Mutex;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::sources::SourceKind;

/// The GitHub personal access token; without one the API allows 60 requests an hour.
pub const GITHUB_TOKEN: &str = "MRGL_GITHUB_TOKEN";
/// The Wago Addons API key.
pub const WAGO_KEY: &str = "MRGL_WAGO_KEY";
/// The CurseForge API key.
pub const CURSEFORGE_KEY: &str = "MRGL_CURSEFORGE_KEY";
/// The sources that take a key, in the order every listing names them; WoWInterface needs none.
pub const KEYED: [SourceKind; 3] = [SourceKind::GitHub, SourceKind::Wago, SourceKind::CurseForge];
/// The variable that turns the key store off for a run when it says `off`.
pub const STORE_ENV: &str = "MRGL_KEY_STORE";
/// The prefix of a key's entry in the store, before the source's prefix: `mrgl:wago`.
pub const TARGET_PREFIX: &str = "mrgl:";
/// The longest key the store takes, in bytes (the Credential Manager's limit for a credential's blob).
pub const MAX_KEY_BYTES: usize = 2560;

/// The variable of a source's key; `None` for a source without one.
pub fn name_of(kind: SourceKind) -> Option<&'static str> {
    match kind {
        SourceKind::GitHub => Some(GITHUB_TOKEN),
        SourceKind::Wago => Some(WAGO_KEY),
        SourceKind::CurseForge => Some(CURSEFORGE_KEY),
        SourceKind::WowInterface => None,
    }
}

/// The source of a key name or a source name on the command line: `wago`, `Wago`, `curseforge`, or the
/// variable itself (`MRGL_WAGO_KEY`); an error that names the accepted ones for anything else, and one
/// that says so for a source without a key.
pub fn keyed_source(text: &str) -> Result<SourceKind> {
    let text = text.trim();
    let kind = SourceKind::parse(text)
        .or_else(|| {
            KEYED
                .into_iter()
                .find(|kind| name_of(*kind).is_some_and(|name| name.eq_ignore_ascii_case(text)))
        })
        .ok_or_else(|| Error::NoKeySource {
            name: text.to_string(),
        })?;
    if name_of(kind).is_none() {
        return Err(Error::KeyNotNeeded {
            source_name: kind.name(),
        });
    }
    Ok(kind)
}

/// The store entry of a source's key: `mrgl:<prefix>`.
pub fn target_name(kind: SourceKind) -> String {
    format!("{TARGET_PREFIX}{}", kind.prefix())
}

/// The hosts a source's key may be sent to, over HTTPS: the API's, and the download host that asks for the
/// same key (CurseForge's CDN, since 2026-07-16). The GitHub token goes to the API only: an asset download
/// hops to an object store that does not need it.
pub fn hosts(kind: SourceKind) -> &'static [&'static str] {
    match kind {
        SourceKind::GitHub => &["api.github.com"],
        SourceKind::Wago => &["addons.wago.io"],
        SourceKind::CurseForge => &["api.curseforge.com", "edge.forgecdn.net"],
        SourceKind::WowInterface => &[],
    }
}

/// Whether a source's key may go with a request to a URL: an `https://` URL on one of its [`hosts`],
/// exactly (no user part, any port refused, ASCII case-insensitive).
pub fn may_send(kind: SourceKind, url: &str) -> bool {
    let Some(scheme) = url.get(..8) else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("https://") {
        return false;
    }
    let authority = url[8..].split(['/', '?', '#']).next().unwrap_or_default();
    hosts(kind)
        .iter()
        .any(|host| authority.eq_ignore_ascii_case(host))
}

/// Where a key comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Its environment variable is set and not empty.
    Environment,
    /// The key store holds it, and the environment does not.
    CredentialStore,
    /// Neither.
    NotSet,
}

impl Origin {
    /// The words of a listing: `set (environment)`, `set (credential store)`, `not set`.
    pub fn text(self) -> &'static str {
        match self {
            Origin::Environment => "set (environment)",
            Origin::CredentialStore => "set (credential store)",
            Origin::NotSet => "not set",
        }
    }

    /// Whether the key is set.
    pub fn is_set(self) -> bool {
        self != Origin::NotSet
    }
}

/// A key with its value, for the source that sends it. `Debug` names the key and its origin only.
#[derive(Clone, PartialEq, Eq)]
pub struct Key {
    pub source: SourceKind,
    pub name: &'static str,
    pub origin: Origin,
    value: String,
}

impl Key {
    /// The value, for the request header of the source's own hosts ([`may_send`]).
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The value, taken.
    pub fn into_value(self) -> String {
        self.value
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Key")
            .field("source", &self.source)
            .field("name", &self.name)
            .field("origin", &self.origin)
            .field("value", &"<hidden>")
            .finish()
    }
}

/// A key as a listing names it: the source, the variable and where it comes from, never the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyState {
    pub source: SourceKind,
    pub name: &'static str,
    pub origin: Origin,
}

impl KeyState {
    /// `MRGL_WAGO_KEY set (credential store)`.
    pub fn text(&self) -> String {
        format!("{} {}", self.name, self.origin.text())
    }
}

/// Where keys are kept between runs: the Windows Credential Manager ([`CredentialStore`]), a store that
/// is off or missing ([`NoStore`]), or the unit tests' [`MemoryStore`]. A method never puts the value in
/// an error.
pub trait KeyStore {
    /// The store's name for a message line: `the Windows Credential Manager`.
    fn name(&self) -> &'static str;

    /// `Ok` when the store can be read and written, else why not ([`Error::KeyStoreUnavailable`]).
    fn available(&self) -> Result<()>;

    /// The key of a source, `None` when the store has none.
    fn read(&self, kind: SourceKind) -> Result<Option<String>>;

    /// Stores a source's key, replacing the one there.
    fn write(&self, kind: SourceKind, value: &str) -> Result<()>;

    /// Removes a source's key; whether there was one.
    fn delete(&self, kind: SourceKind) -> Result<bool>;
}

/// The key store of this process: the Windows Credential Manager, or none with `MRGL_KEY_STORE=off`.
pub fn store_of_process() -> Box<dyn KeyStore> {
    let off = std::env::var_os(STORE_ENV).is_some_and(|value| value.eq_ignore_ascii_case("off"));
    if off {
        Box::new(NoStore {
            reason: "MRGL_KEY_STORE=off",
        })
    } else {
        Box::new(CredentialStore)
    }
}

/// A source's key: its environment variable when set and not empty, else the store's entry; `None` for
/// a source without a key, or when neither has one. A store that cannot be read reads as no key there.
pub fn lookup(
    kind: SourceKind,
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn KeyStore,
) -> Option<Key> {
    let name = name_of(kind)?;
    let key = |origin, value: String| Key {
        source: kind,
        name,
        origin,
        value,
    };
    if let Some(value) = env(name)
        .map(|value| value.to_string_lossy().trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(key(Origin::Environment, value));
    }
    store
        .read(kind)
        .ok()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| key(Origin::CredentialStore, value))
}

/// Every key with its origin, in the order of [`KEYED`].
pub fn list(env: &dyn Fn(&str) -> Option<OsString>, store: &dyn KeyStore) -> Vec<KeyState> {
    KEYED
        .into_iter()
        .filter_map(|kind| {
            let name = name_of(kind)?;
            Some(KeyState {
                source: kind,
                name,
                origin: lookup(kind, env, store).map_or(Origin::NotSet, |key| key.origin),
            })
        })
        .collect()
}

/// [`lookup`] in this process: its environment and [`store_of_process`].
pub fn of_process(kind: SourceKind) -> Option<Key> {
    lookup(kind, &|name| std::env::var_os(name), &*store_of_process())
}

/// [`list`] in this process.
pub fn list_of_process() -> Vec<KeyState> {
    list(&|name| std::env::var_os(name), &*store_of_process())
}

/// Reads a key from an input: one line from a terminal (Enter ends it), else all of it (a pipe), trimmed.
/// Refused when nothing is there, when it holds a space or a second line, or when it is longer than the
/// store takes; the error never carries the value.
pub fn read_key(input: &mut dyn BufRead, one_line: bool) -> Result<String> {
    let mut text = String::new();
    let read = if one_line {
        input.read_line(&mut text)
    } else {
        input.read_to_string(&mut text)
    };
    read.map_err(|error| Error::KeyInput {
        reason: format!("the standard input cannot be read: {error}"),
    })?;
    let key = text.trim();
    if key.is_empty() {
        return Err(Error::KeyInput {
            reason: "nothing on the standard input; pipe the key in, never as an argument"
                .to_string(),
        });
    }
    if key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Error::KeyInput {
            reason: "the standard input holds more than one word or line, not one key".to_string(),
        });
    }
    if key.len() > MAX_KEY_BYTES {
        return Err(Error::KeyInput {
            reason: format!("the key is longer than {MAX_KEY_BYTES} bytes"),
        });
    }
    Ok(key.to_string())
}

/// Stores a source's key once the store says it is available.
pub fn set(store: &dyn KeyStore, kind: SourceKind, value: &str) -> Result<()> {
    store.available()?;
    store.write(kind, value)
}

/// Removes a source's key from the store; whether there was one.
pub fn clear(store: &dyn KeyStore, kind: SourceKind) -> Result<bool> {
    store.available()?;
    store.delete(kind)
}

/// No store: turned off for the run, or a system without one; reads find nothing, writes are refused.
pub struct NoStore {
    /// Why there is none, for the refusal.
    pub reason: &'static str,
}

impl KeyStore for NoStore {
    fn name(&self) -> &'static str {
        "the key store"
    }

    fn available(&self) -> Result<()> {
        Err(Error::KeyStoreUnavailable {
            reason: self.reason.to_string(),
        })
    }

    fn read(&self, _kind: SourceKind) -> Result<Option<String>> {
        Ok(None)
    }

    fn write(&self, _kind: SourceKind, _value: &str) -> Result<()> {
        self.available()
    }

    fn delete(&self, _kind: SourceKind) -> Result<bool> {
        self.available().map(|()| false)
    }
}

/// A store in memory, for the tests: the same contract as the Credential Manager without touching it.
#[derive(Debug, Default)]
pub struct MemoryStore {
    keys: Mutex<BTreeMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore::default()
    }
}

impl KeyStore for MemoryStore {
    fn name(&self) -> &'static str {
        "the test store"
    }

    fn available(&self) -> Result<()> {
        Ok(())
    }

    fn read(&self, kind: SourceKind) -> Result<Option<String>> {
        Ok(self
            .keys
            .lock()
            .ok()
            .and_then(|keys| keys.get(&target_name(kind)).cloned()))
    }

    fn write(&self, kind: SourceKind, value: &str) -> Result<()> {
        if let Ok(mut keys) = self.keys.lock() {
            keys.insert(target_name(kind), value.to_string());
        }
        Ok(())
    }

    fn delete(&self, kind: SourceKind) -> Result<bool> {
        Ok(self
            .keys
            .lock()
            .ok()
            .and_then(|mut keys| keys.remove(&target_name(kind)))
            .is_some())
    }
}

/// The Windows Credential Manager: one generic credential per source, `mrgl:<source>`, kept for this
/// user on this machine (`CRED_PERSIST_LOCAL_MACHINE`), the key as its UTF-8 blob; visible and removable
/// under Control Panel, Credential Manager, Windows Credentials. Outside Windows it is not available.
pub struct CredentialStore;

/// The store's name in a line.
const CREDENTIAL_MANAGER: &str = "the Windows Credential Manager";

// coverage: off — the user's real Credential Manager; the unit tests run the same contract against the
// memory store, the end-to-end tests run with the store off
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod credential_manager {
    use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
    use windows_sys::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    };

    use super::{CREDENTIAL_MANAGER, CredentialStore, KeyStore, target_name};
    use crate::error::{Error, Result};
    use crate::sources::SourceKind;

    /// A string as a null-terminated wide string.
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The error of a call that failed, with the system's words for its code.
    fn failed(action: &'static str, code: u32) -> Error {
        Error::KeyStore {
            action,
            store: CREDENTIAL_MANAGER,
            reason: std::io::Error::from_raw_os_error(code as i32).to_string(),
        }
    }

    impl KeyStore for CredentialStore {
        fn name(&self) -> &'static str {
            CREDENTIAL_MANAGER
        }

        fn available(&self) -> Result<()> {
            Ok(())
        }

        fn read(&self, kind: SourceKind) -> Result<Option<String>> {
            let target = wide(&target_name(kind));
            let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
            // SAFETY: the target is a null-terminated wide string that outlives the call, and the
            // credential pointer is written by the call only on success, then freed with CredFree.
            let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
            if ok == 0 {
                // SAFETY: reads the calling thread's last error, right after the failed call.
                let code = unsafe { GetLastError() };
                return if code == ERROR_NOT_FOUND {
                    Ok(None)
                } else {
                    Err(failed("read", code))
                };
            }
            // SAFETY: on success the pointer is a valid CREDENTIALW whose blob holds `CredentialBlobSize`
            // bytes (a null blob with size 0 is an empty key); the bytes are copied before CredFree.
            let bytes = unsafe {
                let found = &*credential;
                let bytes = if found.CredentialBlob.is_null() || found.CredentialBlobSize == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(
                        found.CredentialBlob,
                        found.CredentialBlobSize as usize,
                    )
                    .to_vec()
                };
                CredFree(credential.cast());
                bytes
            };
            Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
        }

        fn write(&self, kind: SourceKind, value: &str) -> Result<()> {
            let mut target = wide(&target_name(kind));
            let mut user = wide("mrgl");
            let mut blob = value.as_bytes().to_vec();
            let credential = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: target.as_mut_ptr(),
                CredentialBlobSize: blob.len() as u32,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                UserName: user.as_mut_ptr(),
                ..Default::default()
            };
            // SAFETY: every pointer in the credential points into a buffer of this frame that outlives
            // the call; the call copies what it keeps.
            let ok = unsafe { CredWriteW(&credential, 0) };
            blob.fill(0);
            if ok == 0 {
                // SAFETY: as in `read`.
                return Err(failed("store", unsafe { GetLastError() }));
            }
            Ok(())
        }

        fn delete(&self, kind: SourceKind) -> Result<bool> {
            let target = wide(&target_name(kind));
            // SAFETY: the target is a null-terminated wide string that outlives the call.
            let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
            if ok == 0 {
                // SAFETY: as in `read`.
                let code = unsafe { GetLastError() };
                return if code == ERROR_NOT_FOUND {
                    Ok(false)
                } else {
                    Err(failed("remove", code))
                };
            }
            Ok(true)
        }
    }
}

#[cfg(not(windows))]
impl KeyStore for CredentialStore {
    fn name(&self) -> &'static str {
        CREDENTIAL_MANAGER
    }

    fn available(&self) -> Result<()> {
        Err(Error::KeyStoreUnavailable {
            reason: "the Windows Credential Manager exists on Windows only".to_string(),
        })
    }

    fn read(&self, _kind: SourceKind) -> Result<Option<String>> {
        Ok(None)
    }

    fn write(&self, _kind: SourceKind, _value: &str) -> Result<()> {
        self.available()
    }

    fn delete(&self, _kind: SourceKind) -> Result<bool> {
        self.available().map(|()| false)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const SECRET: &str = "ghp_not-a-real-token-0123456789";

    fn env_with(
        pairs: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn the_names_and_the_sources_go_together() {
        assert_eq!(
            KEYED.map(|kind| name_of(kind).unwrap()),
            [GITHUB_TOKEN, WAGO_KEY, CURSEFORGE_KEY]
        );
        assert_eq!(name_of(SourceKind::WowInterface), None);
        assert_eq!(target_name(SourceKind::CurseForge), "mrgl:curse");
        for (text, kind) in [
            ("wago", SourceKind::Wago),
            ("CurseForge", SourceKind::CurseForge),
            ("curse", SourceKind::CurseForge),
            ("github", SourceKind::GitHub),
            ("mrgl_wago_key", SourceKind::Wago),
        ] {
            assert_eq!(keyed_source(text).unwrap(), kind, "{text}");
        }
        assert_eq!(
            keyed_source("wowi").unwrap_err().to_string(),
            "no key for WoWInterface: its API needs none"
        );
        assert_eq!(
            keyed_source("bitbucket").unwrap_err().to_string(),
            "no key source: bitbucket (github, wago or curse)"
        );
    }

    #[test]
    fn the_environment_comes_first_then_the_store() {
        let store = MemoryStore::new();
        let none = env_with(&[]);
        assert_eq!(lookup(SourceKind::Wago, &none, &store), None);
        store.write(SourceKind::Wago, " stored-key \n").unwrap();
        let key = lookup(SourceKind::Wago, &none, &store).expect("the stored key");
        assert_eq!(
            (key.name, key.origin, key.value()),
            (WAGO_KEY, Origin::CredentialStore, "stored-key")
        );
        let env = env_with(&[("MRGL_WAGO_KEY", "from-env")]);
        let key = lookup(SourceKind::Wago, &env, &store).expect("the variable");
        assert_eq!((key.origin, key.value()), (Origin::Environment, "from-env"));
        // An empty variable is not set, so the store answers.
        let empty = env_with(&[("MRGL_WAGO_KEY", "  ")]);
        assert_eq!(
            lookup(SourceKind::Wago, &empty, &store).map(|key| key.origin),
            Some(Origin::CredentialStore)
        );
        assert_eq!(lookup(SourceKind::WowInterface, &env, &store), None);
        let states = list(&env_with(&[("MRGL_GITHUB_TOKEN", SECRET)]), &store);
        assert_eq!(
            states.iter().map(KeyState::text).collect::<Vec<_>>(),
            [
                "MRGL_GITHUB_TOKEN set (environment)",
                "MRGL_WAGO_KEY set (credential store)",
                "MRGL_CURSEFORGE_KEY not set",
            ]
        );
        let json = serde_json::to_string(&states).unwrap();
        assert_eq!(
            json,
            r#"[{"source":"github","name":"MRGL_GITHUB_TOKEN","origin":"environment"},{"source":"wago","name":"MRGL_WAGO_KEY","origin":"credential_store"},{"source":"curse","name":"MRGL_CURSEFORGE_KEY","origin":"not_set"}]"#
        );
        assert!(!json.contains(SECRET) && !json.contains("stored-key"));
    }

    #[test]
    fn the_test_store_sets_reads_and_clears_a_key() {
        let store = MemoryStore::new();
        set(&store, SourceKind::CurseForge, SECRET).unwrap();
        assert_eq!(
            store.read(SourceKind::CurseForge).unwrap().as_deref(),
            Some(SECRET)
        );
        assert_eq!(store.read(SourceKind::Wago).unwrap(), None);
        set(&store, SourceKind::CurseForge, "replaced").unwrap();
        assert_eq!(
            store.read(SourceKind::CurseForge).unwrap().as_deref(),
            Some("replaced")
        );
        assert!(clear(&store, SourceKind::CurseForge).unwrap());
        assert!(!clear(&store, SourceKind::CurseForge).unwrap(), "gone");
        assert_eq!(store.read(SourceKind::CurseForge).unwrap(), None);
    }

    #[test]
    fn a_store_that_is_off_finds_nothing_and_refuses_a_write() {
        let store = NoStore {
            reason: "MRGL_KEY_STORE=off",
        };
        assert_eq!(store.read(SourceKind::Wago).unwrap(), None);
        let error = set(&store, SourceKind::Wago, SECRET)
            .unwrap_err()
            .to_string();
        assert_eq!(error, "the key store is not available: MRGL_KEY_STORE=off");
        assert!(!error.contains(SECRET));
        assert!(clear(&store, SourceKind::Wago).is_err());
        assert_eq!(
            lookup(SourceKind::Wago, &env_with(&[]), &store),
            None,
            "no key"
        );
    }

    #[test]
    fn a_key_is_read_from_the_input_as_one_word() {
        let read = |text: &str, one_line: bool| {
            read_key(
                &mut std::io::Cursor::new(text.as_bytes().to_vec()),
                one_line,
            )
        };
        assert_eq!(read(&format!("{SECRET}\r\n"), false).unwrap(), SECRET);
        assert_eq!(read(&format!("  {SECRET}  "), false).unwrap(), SECRET);
        assert_eq!(
            read(&format!("{SECRET}\nsecond line\n"), true).unwrap(),
            SECRET,
            "a terminal gives one line"
        );
        for (text, reason) in [
            (
                "",
                "nothing on the standard input; pipe the key in, never as an argument",
            ),
            (
                "\r\n",
                "nothing on the standard input; pipe the key in, never as an argument",
            ),
            (
                "two words",
                "the standard input holds more than one word or line, not one key",
            ),
            (
                "one\ntwo",
                "the standard input holds more than one word or line, not one key",
            ),
        ] {
            let error = read(text, false).unwrap_err().to_string();
            assert_eq!(
                error,
                format!("no key read from the standard input: {reason}"),
                "{text:?}"
            );
        }
        let long = "k".repeat(MAX_KEY_BYTES + 1);
        let error = read(&long, false).unwrap_err().to_string();
        assert!(
            error.ends_with("the key is longer than 2560 bytes"),
            "{error}"
        );
        assert!(!error.contains(&long));
    }

    #[test]
    fn a_key_goes_to_its_own_hosts_over_https_only() {
        assert!(may_send(
            SourceKind::GitHub,
            "https://api.github.com/repos/o/r/releases?per_page=30"
        ));
        assert!(may_send(
            SourceKind::CurseForge,
            "https://edge.forgecdn.net/files/1/2/a.zip"
        ));
        assert!(may_send(
            SourceKind::CurseForge,
            "HTTPS://API.CURSEFORGE.COM/v1/mods/1"
        ));
        assert!(may_send(
            SourceKind::Wago,
            "https://addons.wago.io/api/external/addons/x"
        ));
        for (kind, url) in [
            (
                SourceKind::GitHub,
                "https://github.com/o/r/releases/download/v1/a.zip",
            ),
            (
                SourceKind::GitHub,
                "http://api.github.com/repos/o/r/releases",
            ),
            (SourceKind::GitHub, "https://api.github.com.example.test/x"),
            (SourceKind::GitHub, "https://user@api.github.com/x"),
            (SourceKind::GitHub, "https://api.github.com:8443/x"),
            (SourceKind::Wago, "https://api.curseforge.com/v1/mods/1"),
            (SourceKind::CurseForge, "https://addons.wago.io/x"),
            (
                SourceKind::WowInterface,
                "https://api.mmoui.com/v3/game/WOW/filelist.json",
            ),
            (SourceKind::GitHub, "api.github.com/x"),
        ] {
            assert!(!may_send(kind, url), "{kind} {url}");
        }
    }

    #[test]
    fn a_key_hides_its_value_from_debug() {
        let key = lookup(
            SourceKind::GitHub,
            &env_with(&[("MRGL_GITHUB_TOKEN", SECRET)]),
            &MemoryStore::new(),
        )
        .expect("a key");
        let debug = format!("{key:?}");
        assert!(!debug.contains(SECRET), "{debug}");
        assert!(debug.contains("MRGL_GITHUB_TOKEN") && debug.contains("<hidden>"));
        assert_eq!(key.into_value(), SECRET);
    }

    #[cfg(not(windows))]
    #[test]
    fn the_credential_store_is_not_available_outside_windows() {
        let store = CredentialStore;
        assert_eq!(store.read(SourceKind::Wago).unwrap(), None);
        assert_eq!(
            set(&store, SourceKind::Wago, SECRET)
                .unwrap_err()
                .to_string(),
            "the key store is not available: the Windows Credential Manager exists on Windows only"
        );
    }
}
