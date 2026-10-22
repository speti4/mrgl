//! The WoWInterface source (task 094): `wowi:<id>` through the MMOUI API, which takes no key. One project
//! has one file: `GET https://api.mmoui.com/v3/game/WOW/filedetails/<id>.json` answers an array of one
//! project with its version (`UIVersion`), the file's name and download address (`UIFileName`,
//! `UIDownload`), its MD5 (`UIMD5`), its date in milliseconds (`UIDate`) and whether its newest file waits
//! for the site's approval (`UIPending`: `"1"`). A pending file is reported and never installed
//! ([`Error::ReleasePending`]), and the download is checked against the MD5 before it is written, so the
//! engine never opens a damaged or replaced zip ([`sources::check_md5`]).
//!
//! The file names no flavor. The project's compatibility list (`UICompatibility`, the author's "works with"
//! game versions) is in the all-projects answer, `filelist.json` (some 5.6 MB for every project), which this
//! source reads at most once a day and keeps trimmed to what it needs under the mrgl home folder
//! (`cache/wowi/filelist.json`: each project's compatibility, its folders `UIDir` and its page). A
//! compatibility entry of the flavor's game ([`game_of`]: `1.13`–`1.15` Classic Era, `2`–`5` the progressing
//! classic by its major, `6` and up retail, `1.60` and up Forever) makes the pick one by metadata; without a
//! list, without an entry of the flavor's game, and for Forever, which no project lists today, the project's
//! one file is a guess, and the interface check after the install is the safety net. The Forever signal is
//! `yes` when the list names a Forever version and `unknown` otherwise, never inferred from a classic or
//! retail entry.
//!
//! The project's answer is cached under `cache/wowi/<id>.json` with when it was read (the API allows an
//! hour, `Cache-Control: max-age=3600`): every resolve asks again, and when the API cannot be reached the
//! cached answer stands with [`Stale`], the offline rule of the GitHub source (task 041).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::forever::ForeverBuild;
use crate::sources::github::{self, write_cache};
use crate::sources::{
    self, BuildOffer, Channel, Http, Offered, Pick, Request, Source, SourceSpec, Stale, Target,
    notes_line,
};
use crate::{home, state};

/// The MMOUI API of World of Warcraft.
pub const API_URL: &str = "https://api.mmoui.com/v3/game/WOW";
/// The host's name in a message line.
pub const HOST: &str = "WoWInterface";
/// The cache folder under the mrgl home folder.
pub const CACHE_SUBDIR: &str = "cache/wowi";
/// The trimmed file list in the cache folder.
pub const FILE_LIST_CACHE: &str = "filelist.json";
/// How long the cached file list answers without a request: a day, since the list is megabytes.
pub const FILE_LIST_MAX_AGE_SECONDS: i64 = 86_400;

/// The source over any [`Http`]: the real client in the exes, a stub in the tests.
pub struct WowInterface<H: Http> {
    http: H,
    cache_dir: Option<PathBuf>,
    /// The file list once this run has read it, so an update check over many projects reads it once.
    file_list: Mutex<Option<Arc<FileList>>>,
}

/// One project as `filedetails/<id>.json` answers it; only the fields the pick needs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileDetails {
    #[serde(rename = "UID", deserialize_with = "text_or_number")]
    pub id: String,
    /// The version of the project's file, as the author writes it (`v5.0.14-release`).
    #[serde(rename = "UIVersion", default)]
    pub version: String,
    /// When the file was uploaded, in Unix milliseconds.
    #[serde(rename = "UIDate", default, deserialize_with = "optional_number")]
    pub date: Option<i64>,
    /// The file's MD5, hex.
    #[serde(rename = "UIMD5", default)]
    pub md5: Option<String>,
    #[serde(rename = "UIFileName", default)]
    pub file_name: String,
    #[serde(rename = "UIDownload", default)]
    pub download: String,
    /// `"1"` when the newest file waits for the site's approval.
    #[serde(rename = "UIPending", default, deserialize_with = "optional_text")]
    pub pending: Option<String>,
    #[serde(rename = "UIName", default)]
    pub name: Option<String>,
    #[serde(rename = "UIChangeLog", default)]
    pub changelog: Option<String>,
}

impl FileDetails {
    /// Whether the newest file waits for the site's approval (`UIPending` is `1`).
    pub fn is_pending(&self) -> bool {
        self.pending.as_deref().map(str::trim) == Some("1")
    }

    /// The MD5 the API names, lower-case, when it is one (32 hex digits).
    pub fn md5_hex(&self) -> Option<String> {
        let md5 = self.md5.as_deref()?.trim();
        (md5.len() == 32 && md5.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| md5.to_ascii_lowercase())
    }
}

/// One entry of a project's compatibility list: a game version and the patch's name, as the author picked
/// them (`12.0.7`, `Revelations`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compatibility {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub name: String,
}

/// What the file list says of one project: its compatibility list, the addon folders its file holds and
/// its page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listing {
    #[serde(default)]
    pub compatibility: Vec<Compatibility>,
    /// The addon folders of the project's file (`UIDir`), as the author declared them.
    #[serde(default)]
    pub dirs: Vec<String>,
    /// The project's page (`UIFileInfoURL`).
    #[serde(default)]
    pub page: Option<String>,
}

/// The file list as the cache keeps it: every project's [`Listing`] by its id, and when the API sent it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileList {
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub projects: BTreeMap<String, Listing>,
}

/// One project of `filelist.json`, the fields the [`Listing`] keeps.
#[derive(Debug, Deserialize)]
struct RawListEntry {
    #[serde(rename = "UID", deserialize_with = "text_or_number")]
    id: String,
    #[serde(rename = "UICompatibility", default)]
    compatibility: Option<Vec<Compatibility>>,
    #[serde(rename = "UIDir", default)]
    dirs: Option<Vec<String>>,
    #[serde(rename = "UIFileInfoURL", default)]
    page: Option<String>,
}

/// The game a compatibility version belongs to ([`game_of`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    /// `6` and up: retail.
    Retail,
    /// `1.13` to `1.59`: Classic Era.
    ClassicEra,
    /// `1.60` and up: World of Warcraft: Forever (patch `1.60.1` at its start).
    Forever,
    /// `2` to `5`: the progressing classic, by its major and minor version (`3.80` is Titan, `3.4` Wrath).
    Classic { major: u32, minor: u32 },
}

/// The game of a compatibility version (`12.0.7` retail, `1.15.7` Classic Era, `4.4.2` the progressing
/// classic, `1.60.1` Forever); `None` for a text that does not start with a major and a minor number.
pub fn game_of(version: &str) -> Option<Game> {
    let mut parts = version.trim().split('.');
    let major: u32 = parts.next()?.trim().parse().ok()?;
    let minor: u32 = parts.next().map_or(Some(0), |minor| {
        minor
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    })?;
    Some(match major {
        0 => return None,
        1 if minor >= 60 => Game::Forever,
        1 => Game::ClassicEra,
        2..=5 => Game::Classic { major, minor },
        _ => Game::Retail,
    })
}

/// Whether a compatibility entry's game is the target's: the same game, and for the progressing classic
/// with a known build the same major (and on the third, Titan's `3.80` apart from Wrath's `3.4`). The second
/// value says the game type is a guess: the progressing classic with an unknown build.
fn fits(game: Game, target: &Target) -> Option<bool> {
    match (&target.flavor, game) {
        (FlavorId::Retail, Game::Retail)
        | (FlavorId::ClassicEra, Game::ClassicEra)
        | (FlavorId::Forever, Game::Forever) => Some(false),
        (FlavorId::Classic, Game::Classic { major, minor }) => match target.version {
            Some(build) if (2..=5).contains(&build.major) => (build.major == major
                && (major != 3 || (build.minor >= 80) == (minor >= 80)))
                .then_some(false),
            _ => Some(true),
        },
        _ => None,
    }
}

/// The flavor key of a pick for a game: the `release.json` key the GitHub source uses for it, so the two
/// sources write a flavor the same way (`mainline`, `classic`, `forever`, `cata`, …).
fn flavor_key(target: &Target, game: Option<Game>) -> String {
    let key = match game {
        Some(Game::Classic { major: 5, .. }) => Some("mists"),
        Some(Game::Classic { major: 4, .. }) => Some("cata"),
        Some(Game::Classic { major: 3, .. }) => Some("wrath"),
        Some(Game::Classic { major: 2, .. }) => Some("bcc"),
        _ => github::flavor_keys(target).first().copied(),
    };
    key.map_or_else(|| target.flavor.as_str().to_string(), str::to_string)
}

/// The entry of a compatibility list that makes a pick one by metadata for the target ([`fits`]): the
/// newest of the flavor's game, by its version numbers; with whether the game type is a guess.
pub fn compatibility_for<'a>(
    listing: &'a [Compatibility],
    target: &Target,
) -> Option<(&'a Compatibility, bool)> {
    let numbers = |entry: &Compatibility| -> Vec<u32> {
        entry
            .version
            .split('.')
            .map(|part| part.trim().parse().unwrap_or(0))
            .collect()
    };
    listing
        .iter()
        .filter_map(|entry| Some((entry, fits(game_of(&entry.version)?, target)?)))
        .max_by(|(a, _), (b, _)| numbers(a).cmp(&numbers(b)))
}

/// Whether the project publishes a Forever build, from its listing (task 094): `yes` when its compatibility
/// list names a Forever version, `unknown` otherwise; never `no` and never inferred from another game,
/// since the list is the author's "works with" and may be incomplete.
pub fn forever_build_of(listing: Option<&Listing>) -> ForeverBuild {
    let forever = listing.is_some_and(|listing| {
        listing
            .compatibility
            .iter()
            .any(|entry| game_of(&entry.version) == Some(Game::Forever))
    });
    if forever {
        ForeverBuild::Yes
    } else {
        ForeverBuild::Unknown
    }
}

/// The page of a project: `www.wowinterface.com/downloads/info<id>`.
pub fn page_of(id: &str) -> String {
    format!("https://www.wowinterface.com/downloads/info{id}")
}

/// The flavor key, whether the pick is a guess, and why: by the compatibility list when it has an entry of
/// the flavor's game, a guess otherwise (task 094).
fn choose(
    listing: std::result::Result<Option<&Listing>, &str>,
    target: &Target,
) -> (String, bool, String) {
    let flavor = target.flavor_name();
    const TAKEN: &str = "the project's one file is taken, a guess";
    let listing = match listing {
        Ok(listing) => listing,
        Err(reason) => {
            return (
                flavor_key(target, None),
                true,
                format!("the file list of WoWInterface could not be read ({reason}); {TAKEN}"),
            );
        }
    };
    let entries = listing
        .map(|listing| listing.compatibility.as_slice())
        .unwrap_or_default();
    if let Some((entry, game_guess)) = compatibility_for(entries, target) {
        let named = format!(
            "WoWInterface lists compatibility {} ({})",
            entry.version,
            entry.name.trim()
        );
        let reason = if game_guess {
            format!("{named}; the installed build is unknown, so the game type is a guess")
        } else {
            named
        };
        return (
            flavor_key(target, game_of(&entry.version)),
            game_guess,
            reason,
        );
    }
    let reason = if entries.is_empty() {
        format!("WoWInterface lists no compatibility for the project; {TAKEN}")
    } else {
        let mut versions: Vec<&str> = entries.iter().map(|entry| entry.version.trim()).collect();
        versions.dedup();
        format!(
            "WoWInterface lists no {flavor} compatibility (it lists {}); {TAKEN}",
            versions.join(", ")
        )
    };
    (flavor_key(target, None), true, reason)
}

/// The clause of a spec of another source handed to this source itself (the router never does).
const NOT_WOWI: &str = "the WoWInterface source answers wowi: specs only";

/// The project id of a WoWInterface spec; a spec of another source is [`Error::SourceNotConfigured`].
fn project(spec: &SourceSpec) -> Result<&str> {
    match spec {
        SourceSpec::WowInterface { id } => Ok(id),
        other => Err(Error::SourceNotConfigured {
            spec: other.to_string(),
            missing: NOT_WOWI.to_string(),
        }),
    }
}

/// The cached answer of one project: the API's array as it came, and when the API sent it.
#[derive(Debug, Serialize, Deserialize)]
struct CachedDetails {
    #[serde(default)]
    checked_at: Option<String>,
    details: serde_json::Value,
}

impl<H: Http> WowInterface<H> {
    /// A source with an explicit cache folder (`None`: no cache).
    pub fn new(http: H, cache_dir: Option<PathBuf>) -> WowInterface<H> {
        WowInterface {
            http,
            cache_dir,
            file_list: Mutex::new(None),
        }
    }

    /// The source of this process: the cache under the mrgl home folder (no cache without one). The API
    /// takes no key.
    pub fn from_env(http: H) -> WowInterface<H> {
        let cache_dir = home::home_dir().ok().map(|home| home.join(CACHE_SUBDIR));
        WowInterface::new(http, cache_dir)
    }

    /// The client behind the source.
    pub fn http(&self) -> &H {
        &self.http
    }

    fn cache_path(&self, file: &str) -> Option<PathBuf> {
        Some(self.cache_dir.as_ref()?.join(file))
    }

    /// The project's answer: from the API, else, when the API cannot be reached and the answer is cached,
    /// from the cache with [`Stale`] (the offline rule of task 041). A status other than 200 is
    /// [`Error::SourceStatus`] (a 404: no such project), an empty array [`Error::NoRelease`].
    pub fn file_details(&self, id: &str) -> Result<(FileDetails, Option<Stale>)> {
        let spec = format!("wowi:{id}");
        let url = format!("{API_URL}/filedetails/{id}.json");
        let cache = self.cache_path(&format!("{}.json", state::file_safe(id)));
        let (raw, stale) = match self.http.get(&Request::get(url.clone())) {
            Ok(response) if response.status == 200 => {
                let raw: serde_json::Value = serde_json::from_slice(&response.body)
                    .map_err(|reason| unreadable("file details", &spec, reason))?;
                if let Some(path) = &cache {
                    // The cache is a convenience: a folder that cannot be written costs the offline answer.
                    let _ = write_cache(
                        path,
                        &CachedDetails {
                            checked_at: Some(state::now_rfc3339()),
                            details: raw.clone(),
                        },
                    );
                }
                (raw, None)
            }
            Ok(response) => return Err(status_error(response.status, &url, NO_PROJECT)),
            Err(error @ Error::Network { .. }) => {
                let Some(cached) = cache
                    .as_ref()
                    .and_then(|path| std::fs::read(path).ok())
                    .and_then(|bytes| serde_json::from_slice::<CachedDetails>(&bytes).ok())
                else {
                    return Err(error);
                };
                let stale = Stale {
                    checked_at: cached.checked_at,
                    reason: error.to_string(),
                };
                (cached.details, Some(stale))
            }
            Err(error) => return Err(error),
        };
        let mut projects: Vec<FileDetails> = serde_json::from_value(raw)
            .map_err(|reason| unreadable("file details", &spec, reason))?;
        if projects.is_empty() {
            return Err(Error::NoRelease { spec });
        }
        Ok((projects.remove(0), stale))
    }

    /// The file list, from the cache while it is younger than a day, else from the API (cached trimmed);
    /// when the API cannot answer, the cache of any age stands. Read once per run.
    pub fn file_list(&self) -> Result<Arc<FileList>> {
        if let Some(list) = self.file_list.lock().ok().and_then(|list| list.clone()) {
            return Ok(list);
        }
        let cache = self.cache_path(FILE_LIST_CACHE);
        let cached: Option<FileList> = cache
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let fresh = cached.as_ref().is_some_and(|cached| {
            let now = state::now_rfc3339();
            match (
                cached
                    .checked_at
                    .as_deref()
                    .and_then(sources::unix_from_rfc3339),
                sources::unix_from_rfc3339(&now),
            ) {
                (Some(then), Some(now)) => (0..FILE_LIST_MAX_AGE_SECONDS).contains(&(now - then)),
                _ => false,
            }
        });
        let list = match cached {
            Some(cached) if fresh => cached,
            cached => match self.fetch_file_list() {
                Ok(list) => {
                    if let Some(path) = &cache {
                        let _ = write_cache(path, &list);
                    }
                    list
                }
                Err(error) => cached.ok_or(error)?,
            },
        };
        let list = Arc::new(list);
        // A poisoned lock (a panic while the list was being kept) still holds the slot.
        *self
            .file_list
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&list));
        Ok(list)
    }

    /// `filelist.json` from the API, trimmed to the [`Listing`] of every project.
    fn fetch_file_list(&self) -> Result<FileList> {
        let url = format!("{API_URL}/filelist.json");
        let response = self.http.get(&Request::get(url.clone()))?;
        if response.status != 200 {
            return Err(status_error(response.status, &url, "no file list there"));
        }
        let raw: Vec<RawListEntry> = serde_json::from_slice(&response.body)
            .map_err(|reason| unreadable("file list", "wowi:", reason))?;
        let projects = raw
            .into_iter()
            .map(|entry| {
                (
                    entry.id,
                    Listing {
                        compatibility: entry.compatibility.unwrap_or_default(),
                        dirs: entry.dirs.unwrap_or_default(),
                        page: entry.page.filter(|page| !page.is_empty()),
                    },
                )
            })
            .collect();
        Ok(FileList {
            checked_at: Some(state::now_rfc3339()),
            projects,
        })
    }

    /// The listing of one project from the file list ([`WowInterface::file_list`]); `Ok(None)` for a
    /// project the list does not have.
    pub fn listing(&self, id: &str) -> Result<Option<Listing>> {
        Ok(self.file_list()?.projects.get(id).cloned())
    }

    /// The listing of one project from the cache alone, never a request (task 078: `list` and the window
    /// stay offline); `None` when nothing is cached or the project is not in it.
    pub fn cached_listing(&self, id: &str) -> Option<Listing> {
        if let Some(list) = self.file_list.lock().ok().and_then(|list| list.clone()) {
            return list.projects.get(id).cloned();
        }
        let bytes = std::fs::read(self.cache_path(FILE_LIST_CACHE)?).ok()?;
        let list: FileList = serde_json::from_slice(&bytes).ok()?;
        list.projects.get(id).cloned()
    }
}

/// The error of an answer that is not the document expected.
fn unreadable(what: &'static str, spec: &str, reason: serde_json::Error) -> Error {
    Error::SourceDocument {
        what,
        spec: spec.to_string(),
        reason: reason.to_string(),
    }
}

/// The hint of a project's 404.
const NO_PROJECT: &str = "no such project, or it was removed";

/// A status the source cannot work with, with the hint the user can act on; `not_found` is a 404's.
fn status_error(status: u16, url: &str, not_found: &'static str) -> Error {
    let hint = match status {
        404 => not_found,
        500..=599 => "the server has a problem; try again later",
        _ => "an unexpected status",
    };
    Error::SourceStatus {
        host: HOST,
        status,
        url: url.to_string(),
        hint: hint.to_string(),
    }
}

/// A text field the API writes as a string or a number (`"5108"`, `5108`).
fn text_or_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(text) => Ok(text),
        serde_json::Value::Number(number) => Ok(number.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "expected a string or a number, found {other}"
        ))),
    }
}

/// An optional text field the API writes as a string or a number.
fn optional_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(text) => Some(text),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

/// An optional number the API writes as a number or a string of digits.
fn optional_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<i64>, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(text) => text.trim().parse().ok(),
        _ => None,
    })
}

impl<H: Http> Source for WowInterface<H> {
    /// The project's one file for the target (task 094): refused while it waits for approval, picked by
    /// the compatibility list when it names the flavor's game, a guess otherwise. Both channels take the
    /// same file: WoWInterface keeps one per project.
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick> {
        let id = project(spec)?;
        let (details, stale) = self.file_details(id)?;
        if details.is_pending() {
            return Err(Error::ReleasePending {
                spec: spec.to_string(),
                host: HOST,
            });
        }
        if details.download.is_empty() || details.file_name.is_empty() {
            return Err(Error::NoRelease {
                spec: spec.to_string(),
            });
        }
        let listing = self.listing(id).map_err(|error| error.to_string());
        let (flavor, guess, reason) = choose(
            listing.as_ref().map(Option::as_ref).map_err(String::as_str),
            target,
        );
        let listing = listing.ok().flatten();
        let page = listing
            .as_ref()
            .and_then(|listing| listing.page.clone())
            .unwrap_or_else(|| page_of(id));
        Ok(Pick {
            source: spec.to_string(),
            release: details.version.clone(),
            version: sources::version_of_tag(&details.version).to_string(),
            published: details
                .date
                .and_then(|millis| u64::try_from(millis / 1000).ok())
                .map(state::rfc3339_from_unix),
            prerelease: false,
            asset: details.file_name.clone(),
            url: details.download.clone(),
            size: None,
            md5: details.md5_hex(),
            flavor,
            interface: None,
            guess,
            reason,
            notes: details.changelog.as_deref().and_then(notes_line),
            page: Some(page),
            nolib: false,
            forever_build: forever_build_of(listing.as_ref()),
            stale,
        })
    }

    /// A spec of another source is not this source's to answer.
    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        project(spec).err().map(|_| NOT_WOWI.to_string())
    }

    /// The Forever signal from the cached file list alone (task 094): `yes` when it names a Forever
    /// version for the project, `unknown` otherwise; never a request.
    fn cached_forever_build(&self, spec: &SourceSpec, _channel: Channel) -> ForeverBuild {
        match project(spec) {
            Ok(id) => forever_build_of(self.cached_listing(id).as_ref()),
            Err(_) => ForeverBuild::Unknown,
        }
    }

    /// Whether the project has a build for the target, from its compatibility list in the file list alone
    /// (task 097): no request per project. `yes` for an entry of the flavor's game, Forever included once
    /// the list names a Forever version; `unknown` otherwise, never `no`: the list is the author's "works
    /// with" and may be incomplete, and the project's one file may still run.
    fn offers(&self, spec: &SourceSpec, target: &Target) -> BuildOffer {
        let id = match project(spec) {
            Ok(id) => id,
            Err(_) => return BuildOffer::new(Offered::Unknown, NOT_WOWI),
        };
        let listing = match self.listing(id) {
            Ok(Some(listing)) => listing,
            Ok(None) => {
                return BuildOffer::new(
                    Offered::Unknown,
                    "the file list of WoWInterface does not have the project",
                );
            }
            Err(error) => return BuildOffer::new(Offered::Unknown, error.to_string()),
        };
        match compatibility_for(&listing.compatibility, target) {
            Some((entry, false)) => BuildOffer::new(
                Offered::Yes,
                format!(
                    "WoWInterface lists compatibility {} ({})",
                    entry.version.trim(),
                    entry.name.trim()
                ),
            ),
            Some((entry, true)) => BuildOffer::new(
                Offered::Unknown,
                format!(
                    "WoWInterface lists compatibility {}; the installed build is unknown, so the game type is a guess",
                    entry.version.trim()
                ),
            ),
            None if listing.compatibility.is_empty() => BuildOffer::new(
                Offered::Unknown,
                "WoWInterface lists no compatibility for the project",
            ),
            None => {
                let mut versions: Vec<&str> = listing
                    .compatibility
                    .iter()
                    .map(|entry| entry.version.trim())
                    .collect();
                versions.dedup();
                BuildOffer::new(
                    Offered::Unknown,
                    format!(
                        "WoWInterface lists no {} compatibility (it lists {})",
                        target.flavor_name(),
                        versions.join(", ")
                    ),
                )
            }
        }
    }

    /// The project's folders as its author declared them (`UIDir`), from the file list (task 097).
    fn folders(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        let dirs = self.listing(project(spec).ok()?).ok()??.dirs;
        (!dirs.is_empty()).then_some(dirs)
    }

    /// Downloads the project's file (no key, the CDN serves it), checks it against the MD5 the API named
    /// before it is written (task 094), and writes it. A 404 names the file and the project.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        let response = self.http.get(&Request::get(pick.url.clone()))?;
        match response.status {
            200 => {}
            404 => {
                return Err(Error::AssetMissing {
                    host: HOST,
                    spec: pick.source.clone(),
                    release: pick.release.clone(),
                    asset: pick.asset.clone(),
                    url: pick.url.clone(),
                });
            }
            status => return Err(status_error(status, &pick.url, NO_PROJECT)),
        }
        sources::check_md5(pick, &response.body)?;
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                context: "creating the download folder",
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(to, &response.body).map_err(|source| Error::Io {
            context: "writing the download",
            path: to.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::flavor::BuildVersion;
    use crate::sources::MapHttp;

    fn target(flavor: FlavorId, version: Option<&str>) -> Target {
        Target {
            version: version.and_then(BuildVersion::parse),
            ..Target::new(flavor)
        }
    }

    fn entries(versions: &[&str]) -> Vec<Compatibility> {
        versions
            .iter()
            .map(|version| Compatibility {
                version: version.to_string(),
                name: format!("Patch {version}"),
            })
            .collect()
    }

    #[test]
    fn a_compatibility_version_names_its_game() {
        assert_eq!(game_of("12.0.7"), Some(Game::Retail));
        assert_eq!(game_of("6.0.2"), Some(Game::Retail));
        assert_eq!(game_of("1.15.7"), Some(Game::ClassicEra));
        assert_eq!(game_of("1.13"), Some(Game::ClassicEra));
        assert_eq!(game_of("1.60.1"), Some(Game::Forever));
        assert_eq!(game_of("4.4.2"), Some(Game::Classic { major: 4, minor: 4 }));
        assert_eq!(
            game_of("3.80.2"),
            Some(Game::Classic {
                major: 3,
                minor: 80
            })
        );
        assert_eq!(game_of("6.1"), Some(Game::Retail));
        assert_eq!(game_of("Classic"), None);
        assert_eq!(game_of(""), None);
        assert_eq!(game_of("0.5.3"), None);
    }

    #[test]
    fn the_flavors_game_picks_the_newest_entry_and_the_rest_is_none() {
        let list = entries(&["11.2.7", "12.0.7", "1.15.7", "4.4.2", "2.5.4", "3.4.3"]);
        let pick = |flavor, version| {
            compatibility_for(&list, &target(flavor, version))
                .map(|(entry, guess)| (entry.version.as_str(), guess))
        };
        assert_eq!(
            pick(FlavorId::Retail, Some("12.1.5.70448")),
            Some(("12.0.7", false))
        );
        assert_eq!(pick(FlavorId::Retail, None), Some(("12.0.7", false)));
        assert_eq!(pick(FlavorId::ClassicEra, None), Some(("1.15.7", false)));
        assert_eq!(
            pick(FlavorId::Classic, Some("4.4.2.60000")),
            Some(("4.4.2", false))
        );
        assert_eq!(
            pick(FlavorId::Classic, Some("3.4.5.1")),
            Some(("3.4.3", false))
        );
        // Titan's 3.80 is not Wrath's 3.4, and Mists is listed nowhere.
        assert_eq!(pick(FlavorId::Classic, Some("3.80.2.1")), None);
        assert_eq!(pick(FlavorId::Classic, Some("5.5.4.70531")), None);
        // Without a build every classic entry fits, the newest first, and the game type is a guess; so
        // with a build of no game type the list can name.
        assert_eq!(pick(FlavorId::Classic, None), Some(("4.4.2", true)));
        assert_eq!(
            pick(FlavorId::Classic, Some("9.0.0.1")),
            Some(("4.4.2", true))
        );
        // No project lists Forever today: no entry, a guess.
        assert_eq!(pick(FlavorId::Forever, Some("1.60.1.70245")), None);
        assert_eq!(pick(FlavorId::Other("wow_x".into()), None), None);
        let forever = entries(&["1.60.1"]);
        assert_eq!(
            compatibility_for(&forever, &target(FlavorId::Forever, None))
                .map(|(e, g)| (e.version.as_str(), g)),
            Some(("1.60.1", false))
        );
    }

    #[test]
    fn the_forever_signal_is_yes_for_a_forever_entry_and_unknown_otherwise() {
        let listing = |versions: &[&str]| Listing {
            compatibility: entries(versions),
            ..Listing::default()
        };
        assert_eq!(forever_build_of(None), ForeverBuild::Unknown);
        assert_eq!(forever_build_of(Some(&listing(&[]))), ForeverBuild::Unknown);
        assert_eq!(
            forever_build_of(Some(&listing(&["12.0.7", "1.15.7"]))),
            ForeverBuild::Unknown,
            "never inferred from a classic or retail entry"
        );
        assert_eq!(
            forever_build_of(Some(&listing(&["1.60.1"]))),
            ForeverBuild::Yes
        );
    }

    #[test]
    fn the_reason_says_what_the_list_names() {
        let listing = Listing {
            compatibility: entries(&["1.15.7", "12.0.7"]),
            ..Listing::default()
        };
        let retail = target(FlavorId::Retail, None);
        assert_eq!(
            choose(Ok(Some(&listing)), &retail),
            (
                "mainline".to_string(),
                false,
                "WoWInterface lists compatibility 12.0.7 (Patch 12.0.7)".to_string()
            )
        );
        let forever = target(FlavorId::Forever, None);
        assert_eq!(
            choose(Ok(Some(&listing)), &forever),
            (
                "forever".to_string(),
                true,
                "WoWInterface lists no forever compatibility (it lists 1.15.7, 12.0.7); the project's one file is taken, a guess".to_string()
            )
        );
        assert_eq!(
            choose(Ok(None), &retail).2,
            "WoWInterface lists no compatibility for the project; the project's one file is taken, a guess"
        );
        assert_eq!(
            choose(Err("offline"), &retail).2,
            "the file list of WoWInterface could not be read (offline); the project's one file is taken, a guess"
        );
        let classic = target(FlavorId::Classic, None);
        let cata = Listing {
            compatibility: entries(&["4.4.2"]),
            ..Listing::default()
        };
        assert_eq!(
            choose(Ok(Some(&cata)), &classic),
            (
                "cata".to_string(),
                true,
                "WoWInterface lists compatibility 4.4.2 (Patch 4.4.2); the installed build is unknown, so the game type is a guess".to_string()
            )
        );
    }

    #[test]
    fn the_details_read_the_md5_and_the_pending_flag() {
        let details: Vec<FileDetails> = serde_json::from_str(
            r#"[{"UID":5108,"UIVersion":"v1","UIDate":"1785517959000","UIMD5":"ABCDEF0123456789abcdef0123456789","UIFileName":"A.zip","UIDownload":"https://x.test/a","UIPending":1}]"#,
        )
        .expect("details");
        let details = &details[0];
        assert_eq!(details.id, "5108");
        assert_eq!(details.date, Some(1_785_517_959_000));
        assert!(details.is_pending());
        assert_eq!(
            details.md5_hex().as_deref(),
            Some("abcdef0123456789abcdef0123456789")
        );
        let short: FileDetails =
            serde_json::from_str(r#"{"UID":"1","UIMD5":"abc","UIPending":"0"}"#).expect("details");
        assert_eq!(short.md5_hex(), None);
        assert!(!short.is_pending());
        // A field of another type than a string or a number reads as none; the id must be one.
        let odd: FileDetails =
            serde_json::from_str(r#"{"UID":1,"UIPending":true,"UIDate":true}"#).expect("details");
        assert_eq!((odd.pending, odd.date), (None, None));
        let error = serde_json::from_str::<FileDetails>(r#"{"UID":true}"#).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("expected a string or a number, found true"),
            "{error}"
        );
    }

    #[test]
    fn the_flavor_key_of_a_classic_entry_is_the_packagers() {
        for (version, build, key) in [
            ("5.5.0", "5.5.4.70531", "mists"),
            ("4.4.2", "4.4.2.60000", "cata"),
            ("3.4.3", "3.4.5.1", "wrath"),
            ("2.5.4", "2.5.6.1", "bcc"),
        ] {
            let listing = Listing {
                compatibility: entries(&[version]),
                ..Listing::default()
            };
            let (flavor, guess, _) =
                choose(Ok(Some(&listing)), &target(FlavorId::Classic, Some(build)));
            assert_eq!((flavor.as_str(), guess), (key, false), "{version}");
        }
    }

    const DETAILS_URL: &str = "https://api.mmoui.com/v3/game/WOW/filedetails/1.json";
    const FILE_LIST_URL: &str = "https://api.mmoui.com/v3/game/WOW/filelist.json";
    const FILE_LIST: &str =
        r#"[{"UID":"1","UICompatibility":[{"version":"4.4.2","name":"Cata"}],"UIDir":["A"]}]"#;

    fn spec() -> SourceSpec {
        SourceSpec::WowInterface { id: "1".into() }
    }

    fn retail() -> Target {
        target(FlavorId::Retail, None)
    }

    /// A temporary cache folder of one test, removed before use and when the guard drops.
    struct TempCache(PathBuf);

    impl TempCache {
        fn new(name: &str) -> TempCache {
            let dir =
                std::env::temp_dir().join(format!("mrgl-wowi-unit-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempCache(dir)
        }

        fn plant(&self, file: &str, body: &str) {
            std::fs::write(self.0.join(file), body).unwrap();
        }
    }

    impl Drop for TempCache {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A pick of `wowi:1` for a download test.
    fn test_pick(url: &str) -> Pick {
        Pick {
            source: "wowi:1".into(),
            release: "v1".into(),
            version: "1".into(),
            published: None,
            prerelease: false,
            asset: "A-1.zip".into(),
            url: url.into(),
            size: None,
            md5: None,
            flavor: "mainline".into(),
            interface: None,
            guess: true,
            reason: String::new(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: ForeverBuild::Unknown,
            stale: None,
        }
    }

    #[test]
    fn an_answer_that_is_no_project_list_or_no_json_and_a_status_are_errors() {
        let resolve = |http: MapHttp| {
            WowInterface::new(http, None)
                .resolve(&spec(), &retail())
                .unwrap_err()
        };
        let error = resolve(MapHttp::new().answer(DETAILS_URL, 200, "{ not")).to_string();
        assert!(
            error.starts_with("unreadable file details from wowi:1: "),
            "{error}"
        );
        let error =
            resolve(MapHttp::new().answer(DETAILS_URL, 200, "[{\"UID\":true}]")).to_string();
        assert!(
            error.starts_with(
                "unreadable file details from wowi:1: expected a string or a number, found true"
            ),
            "{error}"
        );
        let error = resolve(MapHttp::new().answer(DETAILS_URL, 200, "[]"));
        assert!(matches!(error, Error::NoRelease { .. }), "{error}");
        // A project whose answer names no file.
        let error = resolve(MapHttp::new().answer(
            DETAILS_URL,
            200,
            "[{\"UID\":\"1\",\"UIVersion\":\"v1\"}]",
        ));
        assert!(matches!(error, Error::NoRelease { .. }), "{error}");
        // A download without a file name is no file either.
        let error = resolve(MapHttp::new().answer(
            DETAILS_URL,
            200,
            "[{\"UID\":\"1\",\"UIVersion\":\"v1\",\"UIDownload\":\"https://x.test/a\"}]",
        ));
        assert!(matches!(error, Error::NoRelease { .. }), "{error}");
        for (status, hint) in [
            (503, "the server has a problem; try again later"),
            (418, "an unexpected status"),
        ] {
            let error = resolve(MapHttp::new().answer(DETAILS_URL, status, "")).to_string();
            assert_eq!(
                error,
                format!("WoWInterface answered {status} for {DETAILS_URL}: {hint}")
            );
        }
        // An error of the client that is not the network's is never answered from the cache.
        let error = resolve(MapHttp::new().broken(DETAILS_URL));
        assert!(matches!(error, Error::Io { .. }), "{error}");
    }

    #[test]
    fn a_file_list_cached_without_a_date_is_read_again_and_stands_in_offline() {
        let cache = TempCache::new("list-date");
        let stale = r#"{"checked_at":null,"projects":{"9":{"compatibility":[],"dirs":["Old"],"page":null}}}"#;
        cache.plant(FILE_LIST_CACHE, stale);
        let http = MapHttp::new().answer(FILE_LIST_URL, 200, FILE_LIST);
        let wowi = WowInterface::new(http, Some(cache.0.clone()));
        let list = wowi.file_list().expect("the list");
        assert!(list.projects.contains_key("1") && !list.projects.contains_key("9"));
        assert_eq!(wowi.http().requests().len(), 1);
        // The list held for the run answers the offline listing without reading the cache.
        assert_eq!(
            wowi.cached_listing("1").map(|listing| listing.dirs),
            Some(vec!["A".to_string()])
        );
        assert_eq!(wowi.cached_listing("9"), None);
        // Offline, the cached list of any age stands.
        cache.plant(FILE_LIST_CACHE, stale);
        let offline = WowInterface::new(
            MapHttp::new().offline(FILE_LIST_URL, "no network"),
            Some(cache.0.clone()),
        );
        assert_eq!(
            offline
                .listing("9")
                .expect("the stale list")
                .map(|listing| listing.dirs),
            Some(vec!["Old".to_string()])
        );
        // Offline without a cached list the error stands.
        let bare = WowInterface::new(MapHttp::new().offline(FILE_LIST_URL, "no network"), None);
        let error = bare.file_list().unwrap_err();
        assert!(matches!(error, Error::Network { .. }), "{error}");
    }

    #[test]
    fn a_file_list_that_is_no_json_or_a_status_is_an_error() {
        let error = WowInterface::new(MapHttp::new().answer(FILE_LIST_URL, 404, ""), None)
            .file_list()
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            format!("WoWInterface answered 404 for {FILE_LIST_URL}: no file list there")
        );
        let error = WowInterface::new(MapHttp::new().answer(FILE_LIST_URL, 200, "{ not"), None)
            .file_list()
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("unreadable file list from wowi:: "),
            "{error}"
        );
    }

    #[test]
    fn the_offer_says_what_the_list_cannot_tell() {
        let wowi = WowInterface::new(MapHttp::new().answer(FILE_LIST_URL, 200, FILE_LIST), None);
        let wago = SourceSpec::parse("wago:abc").unwrap();
        let offer = wowi.offers(&wago, &retail());
        assert_eq!(
            (offer.build, offer.why.as_str()),
            (Offered::Unknown, NOT_WOWI)
        );
        assert_eq!(
            wowi.cached_forever_build(&wago, Channel::Stable),
            ForeverBuild::Unknown
        );
        let offer = wowi.offers(&SourceSpec::WowInterface { id: "9".into() }, &retail());
        assert_eq!(
            (offer.build, offer.why.as_str()),
            (
                Offered::Unknown,
                "the file list of WoWInterface does not have the project"
            )
        );
        // The progressing classic without a build: a classic entry, the game type a guess.
        let offer = wowi.offers(&spec(), &target(FlavorId::Classic, None));
        assert_eq!(
            (offer.build, offer.why.as_str()),
            (
                Offered::Unknown,
                "WoWInterface lists compatibility 4.4.2; the installed build is unknown, so the game type is a guess"
            )
        );
        // A list that cannot be read.
        let offline = WowInterface::new(MapHttp::new().offline(FILE_LIST_URL, "no network"), None);
        let offer = offline.offers(&spec(), &retail());
        assert_eq!(
            (offer.build, offer.why.as_str()),
            (
                Offered::Unknown,
                "cannot reach https://api.mmoui.com/v3/game/WOW/filelist.json: no network"
            )
        );
    }

    #[test]
    fn a_download_that_is_missing_or_refused_or_cannot_be_written_is_an_error() {
        let url = "https://cdn.wowinterface.com/downloads/getfile.php?id=1";
        let fetch =
            |http: MapHttp, to: &Path| WowInterface::new(http, None).fetch(&test_pick(url), to);
        let cache = TempCache::new("download");
        let to = cache.0.join("nested").join("A-1.zip");
        let error = fetch(MapHttp::new(), &to).unwrap_err();
        assert!(matches!(error, Error::AssetMissing { .. }), "{error}");
        assert_eq!(
            error.to_string(),
            format!(
                "the asset A-1.zip of wowi:1 v1 is missing: WoWInterface answered 404 for {url} (removed or renamed after the release was listed)"
            )
        );
        let error = fetch(MapHttp::new().answer(url, 500, ""), &to)
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            format!(
                "WoWInterface answered 500 for {url}: the server has a problem; try again later"
            )
        );
        fetch(MapHttp::new().answer(url, 200, "zip"), &to).expect("fetched");
        assert_eq!(std::fs::read(&to).unwrap(), b"zip");
        // A file where the download folder would be, and an empty path that cannot be written.
        let file = cache.0.join("a-file");
        std::fs::write(&file, b"").unwrap();
        let error = fetch(
            MapHttp::new().answer(url, 200, "zip"),
            &file.join("A-1.zip"),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.starts_with("creating the download folder: "),
            "{error}"
        );
        let error = fetch(MapHttp::new().answer(url, 200, "zip"), Path::new(""))
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("writing the download: "), "{error}");
    }
}
