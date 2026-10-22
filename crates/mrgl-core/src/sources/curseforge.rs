//! The CurseForge source (task 096): `curse:<id>` through the CurseForge Core API with the user's own key
//! (`MRGL_CURSEFORGE_KEY` or the key store's `mrgl:curse`, read through [`crate::keys`], task 093; D-002: mrgl
//! ships none, and the source is never needed: every other source works without it). Two requests answer a
//! resolve, each with the key in the `x-api-key` header: `GET https://api.curseforge.com/v1/mods/<id>` (the
//! project: its name, its page `links.websiteUrl`, `allowModDistribution`, and `latestFilesIndexes`, the game
//! version types it has files for) and `GET /v1/mods/<id>/files?gameVersionTypeId=<t>&pageSize=50` (its files
//! for the target's game version type, [`game_version_type`]). A file's `releaseType` maps onto the channels
//! ([`file_on`]: 1 a release, 2 a beta, 3 an alpha; the stable channel takes the newest release, else the
//! newest file, the prerelease channel the newest file); the pick's version is the file's `displayName`, its
//! release the file id, its asset the `fileName` behind `downloadUrl`.
//!
//! The download from `edge.forgecdn.net` carries the key too (the CDN asks for it since 2026-07-16). The key
//! goes to `api.curseforge.com` and `edge.forgecdn.net` only ([`keys::may_send`]): every request is sent with
//! the client's own redirects off and a redirect is followed here, with the key only when the next address
//! is on one of the two hosts, so the header never follows a hop to another host. A 401 names the key.
//!
//! The terms of the API allow no saving or caching of its data: nothing of an answer is written, an answer
//! is kept in memory for the run alone (one `adopt` asks the project once for the build and its folders),
//! [`Source::listed_versions`] stays `None`, and the Forever signal `list` shows offline comes from what the
//! sidecar recorded at install ([`crate::state::Entry::forever_build`]), else `unknown`. The window's
//! offline view ([`CurseForge::cached`]) has no key and asks nothing.
//!
//! A project whose author allows no download outside CurseForge (`allowModDistribution: false`, or a file
//! whose `downloadUrl` is `null`) is [`Error::DistributionOff`]: the line names its page and the command that
//! installs the zip the user downloads there with the source recorded, `mrgl add <file.zip> --as curse:<id>`.
//! The download is checked against the file's length and its MD5 (`hashes` with `algo` 2) before it is
//! written; the SHA-1 (`algo` 1) is not read, since no SHA-1 is in the tree and the MD5 and the length catch
//! a damaged or a replaced file as well.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::forever::ForeverBuild;
use crate::keys;
use crate::sources::github::rate_limit_reset;
use crate::sources::wago::interface_of_patch;
use crate::sources::{
    self, BuildOffer, Channel, Http, Offered, Pick, Request, Response, Source, SourceKind,
    SourceSpec, Target, source_gone,
};

/// The Core API, which takes the user's key on every request.
pub const API_URL: &str = "https://api.curseforge.com/v1";
/// The header the key goes in.
pub const KEY_HEADER: &str = "x-api-key";
/// The host's name in a message line.
pub const HOST: &str = "CurseForge";
/// World of Warcraft's game id in the API: a project of another game is no addon.
pub const GAME_ID: u64 = 1;
/// The files one request asks for (the API's largest page).
pub const PAGE_SIZE: u32 = 50;
/// The redirects one request follows before it gives up.
pub const MAX_REDIRECTS: usize = 5;

// The game version type ids of World of Warcraft (`gameVersionTypeId`). They were extracted from
// `GET /v1/games/1/version-types` by another client and are kept here as constants until a key allows
// re-reading them from that endpoint (task 096 waits for the key of task 007 to confirm them).
/// Retail.
pub const RETAIL: u32 = 517;
/// Classic Era.
pub const CLASSIC_ERA: u32 = 67408;
/// World of Warcraft: Forever.
pub const FOREVER: u32 = 88568;
/// The Burning Crusade Classic.
pub const TBC: u32 = 73246;
/// Wrath of the Lich King Classic.
pub const WRATH: u32 = 73713;
/// Titan Reforged (`3.80`).
pub const TITAN: u32 = 81212;
/// Cataclysm Classic.
pub const CATA: u32 = 77522;
/// Mists of Pandaria Classic.
pub const MISTS: u32 = 79434;

/// The game version type of a target and the name a message and the pick's flavor give it, and whether it is
/// a guess: the progressing classic follows the installed build's major (Titan's `3.80` apart from Wrath's
/// `3.4`), as the Wago source's game versions do, and the names are Wago's (`retail`, `classic` for Classic
/// Era, `forever`, `bc`, `wotlk`, `titan`, `cata`, `mop`); without a build it is the newest game type, Mists,
/// a guess. `None` for a flavor id mrgl does not know.
pub fn game_version_type(target: &Target) -> Option<(u32, &'static str, bool)> {
    Some(match &target.flavor {
        FlavorId::Retail => (RETAIL, "retail", false),
        FlavorId::ClassicEra => (CLASSIC_ERA, "classic", false),
        FlavorId::Forever => (FOREVER, "forever", false),
        FlavorId::Classic => match target.version {
            Some(build) if build.major == 5 => (MISTS, "mop", false),
            Some(build) if build.major == 4 => (CATA, "cata", false),
            Some(build) if build.major == 3 && build.minor >= 80 => (TITAN, "titan", false),
            Some(build) if build.major == 3 => (WRATH, "wotlk", false),
            Some(build) if build.major == 2 => (TBC, "bc", false),
            _ => (MISTS, "mop", true),
        },
        FlavorId::Other(_) => return None,
    })
}

/// One entry of a project's `latestFilesIndexes`: a game version type it has a file for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIndex {
    pub file_id: u64,
    pub game_version_type: Option<u32>,
    /// 1 a release, 2 a beta, 3 an alpha.
    pub release_type: u8,
}

/// A project as `GET /v1/mods/<id>` answers it; only the fields the source reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurseMod {
    pub id: Option<u64>,
    pub game_id: Option<u64>,
    pub name: Option<String>,
    /// The project's page (`links.websiteUrl`).
    pub page: Option<String>,
    /// `allowModDistribution`: `Some(false)` when the author allows no download outside CurseForge.
    pub allow_distribution: Option<bool>,
    /// `latestFilesIndexes`, `None` when the answer has no such list.
    pub latest: Option<Vec<FileIndex>>,
}

impl CurseMod {
    /// Whether the project names a file of a game version type in its latest files.
    pub fn lists(&self, type_id: u32) -> Option<bool> {
        let latest = self.latest.as_ref()?;
        Some(
            latest
                .iter()
                .any(|index| index.game_version_type == Some(type_id)),
        )
    }

    /// The Forever signal of the project: `yes` when its latest files name the Forever type, `no` when they
    /// name others only, `unknown` when the answer has no such list.
    pub fn forever_build(&self) -> ForeverBuild {
        match self.lists(FOREVER) {
            Some(true) => ForeverBuild::Yes,
            Some(false) => ForeverBuild::No,
            None => ForeverBuild::Unknown,
        }
    }
}

/// One file as `GET /v1/mods/<id>/files` answers it; only the fields the source reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurseFile {
    pub id: u64,
    /// The version as the author names the file.
    pub display_name: String,
    pub file_name: String,
    /// 1 a release, 2 a beta, 3 an alpha.
    pub release_type: u8,
    /// When it was uploaded, as the API writes it (ISO 8601 in UTC).
    pub file_date: Option<String>,
    pub file_length: Option<u64>,
    /// The MD5 of `hashes` (`algo` 2), lower-case hex.
    pub md5: Option<String>,
    /// `None` when the author allows no download outside CurseForge.
    pub download_url: Option<String>,
    /// `isAvailable` is not `false`.
    pub available: bool,
    /// The game versions it supports, with their game version type (`sortableGameVersions`).
    pub game_versions: Vec<(Option<u32>, String)>,
    /// The addon folders it holds (`modules[].name`).
    pub modules: Vec<String>,
}

impl CurseFile {
    /// Whether the file names a game version type in its game versions; a file that names none is taken at
    /// the request's word (it was asked for that type).
    pub fn fits(&self, type_id: u32) -> bool {
        self.game_versions.iter().all(|(kind, _)| kind.is_none())
            || self
                .game_versions
                .iter()
                .any(|(kind, _)| *kind == Some(type_id))
    }

    /// The interface number of the newest game version of a type it supports (`1.60.1` is 16001).
    pub fn interface(&self, type_id: u32) -> Option<u32> {
        self.game_versions
            .iter()
            .filter(|(kind, _)| *kind == Some(type_id))
            .filter_map(|(_, version)| interface_of_patch(version))
            .max()
    }

    /// Whether the file is a release (`releaseType` 1), not a beta or an alpha.
    pub fn is_release(&self) -> bool {
        self.release_type == 1
    }

    /// `release`, `beta` or `alpha`, for a message.
    pub fn kind(&self) -> &'static str {
        match self.release_type {
            1 => "release",
            2 => "beta",
            _ => "alpha",
        }
    }
}

fn text(value: &serde_json::Value, name: &str) -> Option<String> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn number(value: &serde_json::Value, name: &str) -> Option<u64> {
    let field = value.get(name)?;
    field
        .as_u64()
        .or_else(|| field.as_str().and_then(|text| text.trim().parse().ok()))
}

/// The object under `data`, or the answer itself.
fn data(raw: &serde_json::Value) -> &serde_json::Value {
    match raw.get("data") {
        Some(data) if !data.is_null() => data,
        _ => raw,
    }
}

/// Reads the answer of `GET /v1/mods/<id>`.
pub fn parse_mod(raw: &serde_json::Value) -> std::result::Result<CurseMod, String> {
    let project = data(raw);
    if !project.is_object() {
        return Err("expected an object".to_string());
    }
    let latest = project
        .get("latestFilesIndexes")
        .and_then(serde_json::Value::as_array)
        .map(|indexes| {
            indexes
                .iter()
                .filter_map(|index| {
                    Some(FileIndex {
                        file_id: number(index, "fileId")?,
                        game_version_type: number(index, "gameVersionTypeId")
                            .and_then(|id| u32::try_from(id).ok()),
                        release_type: number(index, "releaseType")
                            .and_then(|kind| u8::try_from(kind).ok())
                            .unwrap_or(1),
                    })
                })
                .collect()
        });
    Ok(CurseMod {
        id: number(project, "id"),
        game_id: number(project, "gameId"),
        name: text(project, "name"),
        page: project
            .get("links")
            .and_then(|links| text(links, "websiteUrl")),
        allow_distribution: project
            .get("allowModDistribution")
            .and_then(serde_json::Value::as_bool),
        latest,
    })
}

/// Reads the answer of `GET /v1/mods/<id>/files`: its `data` list; a file without an id is left out.
pub fn parse_files(raw: &serde_json::Value) -> std::result::Result<Vec<CurseFile>, String> {
    let Some(files) = data(raw).as_array() else {
        return Err("expected a list of files".to_string());
    };
    Ok(files.iter().filter_map(parse_file).collect())
}

fn parse_file(file: &serde_json::Value) -> Option<CurseFile> {
    let md5 = file
        .get("hashes")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|hash| number(hash, "algo") == Some(2))
        .and_then(|hash| text(hash, "value"))
        .filter(|md5| md5.len() == 32 && md5.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|md5| md5.to_ascii_lowercase());
    let game_versions = file
        .get("sortableGameVersions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|version| {
            (
                number(version, "gameVersionTypeId").and_then(|id| u32::try_from(id).ok()),
                text(version, "gameVersion").unwrap_or_default(),
            )
        })
        .collect();
    let modules = file
        .get("modules")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|module| text(module, "name"))
        .collect();
    Some(CurseFile {
        id: number(file, "id")?,
        display_name: text(file, "displayName").unwrap_or_default(),
        file_name: text(file, "fileName").unwrap_or_default(),
        release_type: number(file, "releaseType")
            .and_then(|kind| u8::try_from(kind).ok())
            .unwrap_or(1),
        file_date: text(file, "fileDate"),
        file_length: number(file, "fileLength"),
        md5,
        download_url: text(file, "downloadUrl"),
        available: file.get("isAvailable").and_then(serde_json::Value::as_bool) != Some(false),
        game_versions,
        modules,
    })
}

/// The file a channel takes among the files of a game version type (task 096): on the stable channel the
/// newest release (`releaseType` 1), else the newest file (a beta or an alpha only when nothing else is
/// there, the GitHub rule); on the prerelease channel the newest file. Newest by `fileDate`, then by id; a
/// file that is not available, or that names other game version types only, is left out.
pub fn file_on(files: &[CurseFile], type_id: u32, channel: Channel) -> Option<&CurseFile> {
    let candidates = || {
        files
            .iter()
            .filter(|file| file.available && file.fits(type_id))
    };
    let newest = |file: &&CurseFile| (file.file_date.clone(), file.id);
    let any = candidates().max_by_key(newest);
    match channel {
        Channel::Stable => candidates()
            .filter(|file| file.is_release())
            .max_by_key(newest)
            .or(any),
        Channel::Prerelease => any,
    }
}

/// The page of a project when its answer names none.
pub fn page_of(id: &str) -> String {
    format!("https://www.curseforge.com/projects/{id}")
}

/// The clause of a spec of another source handed to this source itself (the router never does).
const NOT_CURSE: &str = "the CurseForge source answers curse: specs only";

/// The project id of a CurseForge spec; a spec of another source is [`Error::SourceNotConfigured`].
fn project_id(spec: &SourceSpec) -> Result<&str> {
    match spec {
        SourceSpec::CurseForge { id } => Ok(id),
        other => Err(Error::SourceNotConfigured {
            spec: other.to_string(),
            missing: NOT_CURSE.to_string(),
        }),
    }
}

/// The clause of a run without the key.
fn no_key() -> String {
    format!(
        "{} is not set: the CurseForge API needs your own key, `mrgl key set curse` stores it",
        keys::CURSEFORGE_KEY
    )
}

/// The source over any [`Http`]: the real client in the exes, a stub in the tests.
pub struct CurseForge<H: Http> {
    http: H,
    key: Option<String>,
    /// Without a key the source cannot be asked ([`Source::missing`]); the window's offline view
    /// ([`CurseForge::cached`]) has no key and asks nothing either, its resolve failing as a check that could
    /// not ask.
    needs_key: bool,
    /// This run's answers by URL, in memory only: never written (the terms allow no cache).
    answers: Mutex<BTreeMap<String, serde_json::Value>>,
    /// The addon folders of the file this run picked last for a project (`modules`), for adopt's group.
    modules: Mutex<BTreeMap<String, Vec<String>>>,
}

impl<H: Http> CurseForge<H> {
    /// A source with an explicit key.
    pub fn new(http: H, key: Option<String>) -> CurseForge<H> {
        CurseForge {
            http,
            key: key.filter(|key| !key.trim().is_empty()),
            needs_key: true,
            answers: Mutex::new(BTreeMap::new()),
            modules: Mutex::new(BTreeMap::new()),
        }
    }

    /// The source of this process: the key from `MRGL_CURSEFORGE_KEY` or the key store
    /// ([`keys::of_process`]). There is no cache folder.
    pub fn from_env(http: H) -> CurseForge<H> {
        let key = keys::of_process(SourceKind::CurseForge).map(keys::Key::into_value);
        CurseForge::new(http, key)
    }

    /// The source of the window's offline view (task 078, [`super::Sources::cached`]): no key and no cache,
    /// so it asks nothing and every resolve fails as a check that could not ask; `list` reads the Forever
    /// signal from the sidecar instead.
    pub fn cached(http: H) -> CurseForge<H> {
        CurseForge {
            needs_key: false,
            ..CurseForge::new(http, None)
        }
    }

    /// The client behind the source.
    pub fn http(&self) -> &H {
        &self.http
    }

    /// A GET with the key on CurseForge's two hosts only ([`keys::may_send`]), the client's redirects off and
    /// each redirect followed here, the key going along only to a next address on those hosts.
    fn send(&self, url: &str, json: bool) -> Result<Response> {
        let mut url = url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            let mut request = Request::get(url.clone()).no_redirects();
            if json {
                request = request.header("Accept", "application/json");
            }
            if let Some(key) = &self.key
                && keys::may_send(SourceKind::CurseForge, &url)
            {
                request = request.header(KEY_HEADER, key.clone());
            }
            let response = self.http.get(&request)?;
            match response.header("location") {
                Some(location) if matches!(response.status, 301 | 302 | 303 | 307 | 308) => {
                    url = next_url(&url, location.trim());
                }
                _ => return Ok(response),
            }
        }
        Err(Error::Network {
            url,
            reason: format!("more than {MAX_REDIRECTS} redirects"),
        })
    }

    /// An answer of the API as JSON, asked once per run: the key is required, a status other than 200 is
    /// [`Error::SourceStatus`] with its hint.
    fn json(&self, url: &str, spec: &SourceSpec, what: &'static str) -> Result<serde_json::Value> {
        if let Some(held) = self
            .answers
            .lock()
            .ok()
            .and_then(|answers| answers.get(url).cloned())
        {
            return Ok(held);
        }
        if self.key.is_none() {
            return Err(Error::SourceNotConfigured {
                spec: spec.to_string(),
                missing: no_key(),
            });
        }
        let response = self.send(url, true)?;
        if response.status != 200 {
            return Err(status_error(response.status, url, &response, NO_PROJECT));
        }
        let raw: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|reason| Error::SourceDocument {
                what,
                spec: spec.to_string(),
                reason: reason.to_string(),
            })?;
        if let Ok(mut answers) = self.answers.lock() {
            answers.insert(url.to_string(), raw.clone());
        }
        Ok(raw)
    }

    /// The project of a spec (`GET /v1/mods/<id>`); a project of another game is no addon.
    pub fn project(&self, spec: &SourceSpec) -> Result<CurseMod> {
        let id = project_id(spec)?;
        let raw = self.json(&format!("{API_URL}/mods/{id}"), spec, "project")?;
        let project = parse_mod(&raw).map_err(|reason| Error::SourceDocument {
            what: "project",
            spec: spec.to_string(),
            reason,
        })?;
        match project.game_id {
            Some(game) if game != GAME_ID => Err(Error::SourceDocument {
                what: "project",
                spec: spec.to_string(),
                reason: format!("it is no World of Warcraft addon (game id {game})"),
            }),
            _ => Ok(project),
        }
    }

    /// The files of a project for a game version type (`GET /v1/mods/<id>/files?gameVersionTypeId=<t>`,
    /// the first page of [`PAGE_SIZE`], which the API sorts newest first).
    pub fn files(&self, spec: &SourceSpec, type_id: u32) -> Result<Vec<CurseFile>> {
        let id = project_id(spec)?;
        let url =
            format!("{API_URL}/mods/{id}/files?gameVersionTypeId={type_id}&pageSize={PAGE_SIZE}");
        let raw = self.json(&url, spec, "file list")?;
        parse_files(&raw).map_err(|reason| Error::SourceDocument {
            what: "file list",
            spec: spec.to_string(),
            reason,
        })
    }

    /// The game version type of a target, or the error of a flavor CurseForge names no type for.
    fn type_of(spec: &SourceSpec, target: &Target) -> Result<(u32, &'static str, bool)> {
        game_version_type(target).ok_or_else(|| Error::NoAssetForFlavor {
            spec: spec.to_string(),
            release: "-".to_string(),
            flavor: target.flavor_name().to_string(),
            detail: "CurseForge names no game version type for this flavor".to_string(),
        })
    }

    /// The project and the file the target's channel takes, the file's folders held for adopt.
    fn choose(
        &self,
        spec: &SourceSpec,
        target: &Target,
        game: (u32, &'static str, bool),
    ) -> Result<(CurseMod, CurseFile)> {
        let project = self.project(spec)?;
        let files = self.files(spec, game.0)?;
        let file = file_on(&files, game.0, target.channel)
            .cloned()
            .ok_or_else(|| no_file(spec, target, game))?;
        self.hold_modules(spec, &file);
        Ok((project, file))
    }

    fn hold_modules(&self, spec: &SourceSpec, file: &CurseFile) {
        if file.modules.is_empty() {
            return;
        }
        if let Ok(mut modules) = self.modules.lock() {
            modules.insert(spec.to_string(), file.modules.clone());
        }
    }

    /// The pick of a file, or [`Error::DistributionOff`] when its author allows no download outside
    /// CurseForge.
    fn pick_of(
        &self,
        spec: &SourceSpec,
        project: &CurseMod,
        file: &CurseFile,
        (type_id, game, guess): (u32, &'static str, bool),
        why: String,
    ) -> Result<Pick> {
        let id = project_id(spec)?;
        let page = project.page.clone().unwrap_or_else(|| page_of(id));
        let url = match &file.download_url {
            Some(url) if project.allow_distribution != Some(false) => url.clone(),
            _ => {
                return Err(Error::DistributionOff {
                    spec: spec.to_string(),
                    name: project
                        .name
                        .as_ref()
                        .map(|name| format!(" ({name})"))
                        .unwrap_or_default(),
                    page,
                });
            }
        };
        let mut reason = why;
        if guess {
            reason.push_str("; the installed build is unknown, so the game type is a guess");
        }
        let forever_build = if type_id == FOREVER {
            ForeverBuild::Yes
        } else {
            project.forever_build()
        };
        let asset = if file.file_name.is_empty() {
            format!("{}.zip", file.id)
        } else {
            file.file_name.clone()
        };
        let version = if file.display_name.is_empty() {
            file.id.to_string()
        } else {
            file.display_name.clone()
        };
        Ok(Pick {
            source: spec.to_string(),
            release: file.id.to_string(),
            version,
            published: file.file_date.clone(),
            prerelease: !file.is_release(),
            asset,
            url,
            size: file.file_length,
            md5: file.md5.clone(),
            flavor: game.to_string(),
            interface: file.interface(type_id),
            guess,
            reason,
            notes: None,
            page: Some(page),
            nolib: false,
            forever_build,
            stale: None,
        })
    }
}

/// The address a redirect names, against the address that answered it: absolute as given, `//host/…` on
/// `https:`, `/path` on the same host, any other text next to the answering path.
fn next_url(from: &str, location: &str) -> String {
    if location.starts_with("https://") || location.starts_with("http://") {
        return location.to_string();
    }
    if let Some(rest) = location.strip_prefix("//") {
        return format!("https://{rest}");
    }
    let (scheme, rest) = from.split_once("://").unwrap_or(("https", from));
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if location.starts_with('/') {
        return format!("{scheme}://{host}{location}");
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let base = path.rsplit_once('/').map_or(path, |(base, _)| base);
    format!("{scheme}://{base}/{location}")
}

/// The error of a project with no file for the target's game version type.
fn no_file(spec: &SourceSpec, target: &Target, (type_id, game, _): (u32, &str, bool)) -> Error {
    Error::NoAssetForFlavor {
        spec: spec.to_string(),
        release: format!("game version type {type_id} ({game})"),
        flavor: target.flavor_name().to_string(),
        detail: "CurseForge lists no release, beta or alpha file for it".to_string(),
    }
}

/// The hint of a project's 404.
const NO_PROJECT: &str = "no such project, or it was removed";

/// A status the source cannot work with, with the hint the user can act on: a 401 (and a 403, which the CDN
/// answers a request without the key with) names the key, a 429 the rate limit; `not_found` is a 404's.
fn status_error(status: u16, url: &str, response: &Response, not_found: &'static str) -> Error {
    let hint = match status {
        401 | 403 => format!(
            "{} is missing or not accepted: the CurseForge API needs your own key, `mrgl key set curse` stores it",
            keys::CURSEFORGE_KEY
        ),
        429 => match rate_limit_reset(response) {
            Some(reset) => format!(
                "the rate limit is used up until {} (local time)",
                sources::local_time_text(reset)
            ),
            None => "the rate limit is used up; try again after a short wait".to_string(),
        },
        404 => not_found.to_string(),
        500..=599 => "the server has a problem; try again later".to_string(),
        _ => "an unexpected status".to_string(),
    };
    Error::SourceStatus {
        host: HOST,
        status,
        url: url.to_string(),
        hint,
    }
}

impl<H: Http> Source for CurseForge<H> {
    /// The file the target's channel takes among the project's files for its game version type
    /// ([`game_version_type`], [`file_on`]).
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick> {
        let game = Self::type_of(spec, target)?;
        let (project, file) = self.choose(spec, target, game)?;
        let why = if target.channel == Channel::Stable && !file.is_release() {
            format!(
                "CurseForge lists no release file for {}; the newest, the {} {}, is taken",
                game.1,
                file.kind(),
                file.display_name
            )
        } else {
            format!(
                "CurseForge lists the {} {} for {}",
                file.kind(),
                file.display_name,
                game.1
            )
        };
        self.pick_of(spec, &project, &file, game, why)
    }

    /// The file whose `displayName` is the version (with or without a leading `v`), or whose id it is, among
    /// the files of the game version type the first page lists, any release type.
    fn resolve_version(&self, spec: &SourceSpec, target: &Target, version: &str) -> Result<Pick> {
        let game = Self::type_of(spec, target)?;
        let project = self.project(spec)?;
        let files = self.files(spec, game.0)?;
        let file = files
            .iter()
            .filter(|file| file.available && file.fits(game.0))
            .find(|file| {
                file.display_name == version
                    || sources::version_of_tag(&file.display_name) == version
                    || file.id.to_string() == version
            })
            .ok_or_else(|| Error::VersionNotOffered {
                spec: spec.to_string(),
                version: version.to_string(),
            })?;
        self.hold_modules(spec, file);
        let why = format!(
            "CurseForge lists the {} {} for {}",
            file.kind(),
            file.display_name,
            game.1
        );
        self.pick_of(spec, &project, file, game, why)
    }

    /// Downloads the file with the key (on CurseForge's hosts only, a redirect elsewhere without it),
    /// checks its length and its MD5 before it is written; a 404 names the file.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        let response = self.send(&pick.url, false)?;
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
            status => return Err(status_error(status, &pick.url, &response, NO_PROJECT)),
        }
        let got = response.body.len() as u64;
        if let Some(expected) = pick.size
            && got != expected
        {
            return Err(Error::AssetSizeMismatch {
                spec: pick.source.clone(),
                release: pick.release.clone(),
                asset: pick.asset.clone(),
                expected,
                got,
            });
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

    /// A spec of another source is not this source's to answer; without the key the API cannot be asked
    /// (the window's offline view excepted, which asks nothing and fails each resolve instead).
    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        if spec.kind() != SourceKind::CurseForge {
            return Some(NOT_CURSE.to_string());
        }
        (self.needs_key && self.key.is_none()).then(no_key)
    }

    /// Whether the project has a file for the target's game version type (task 097): the project's latest
    /// files name the type or not, and the file the channel takes says yes (its folders held for adopt's
    /// group, [`Source::folders`]); a guess of the game type is `unknown`. A project whose author allows no
    /// download outside CurseForge still has the build, and the reason says how it is installed.
    fn offers(&self, spec: &SourceSpec, target: &Target) -> BuildOffer {
        if let Some(missing) = self.missing(spec) {
            return BuildOffer::new(Offered::Unknown, missing);
        }
        let Some(game) = game_version_type(target) else {
            return BuildOffer::new(
                Offered::Unknown,
                "CurseForge names no game version type for this flavor",
            );
        };
        let project = match self.project(spec) {
            Ok(project) => project,
            Err(error) if source_gone(&error) => {
                return BuildOffer::new(Offered::No, error.to_string());
            }
            Err(error) => return BuildOffer::new(Offered::Unknown, error.to_string()),
        };
        if project.lists(game.0) == Some(false) {
            return BuildOffer::new(
                Offered::No,
                format!("CurseForge lists no file for {}", game.1),
            );
        }
        match self.choose(spec, target, game) {
            Ok((project, file)) => {
                let mut why = format!(
                    "CurseForge lists the {} {} for {}",
                    file.kind(),
                    file.display_name,
                    game.1
                );
                if file.download_url.is_none() || project.allow_distribution == Some(false) {
                    why.push_str(
                        "; its author allows no download outside CurseForge, so it installs from a zip in hand",
                    );
                }
                if game.2 {
                    why.push_str("; the installed build is unknown, so the game type is a guess");
                    BuildOffer::new(Offered::Unknown, why)
                } else {
                    BuildOffer::new(Offered::Yes, why)
                }
            }
            Err(error) if source_gone(&error) => BuildOffer::new(Offered::No, error.to_string()),
            Err(error) => BuildOffer::new(Offered::Unknown, error.to_string()),
        }
    }

    /// The addon folders of the file this run picked for the project (`modules[].name`, task 096), held
    /// from its resolve or its [`Source::offers`]; `None` before either, never a request of its own.
    fn folders(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        self.modules.lock().ok()?.get(&spec.to_string()).cloned()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::flavor::BuildVersion;
    use crate::sources::MapHttp;

    fn file(id: u64, release_type: u8, date: &str) -> CurseFile {
        CurseFile {
            id,
            display_name: format!("1.0.{id}"),
            file_name: format!("A-{id}.zip"),
            release_type,
            file_date: Some(date.to_string()),
            available: true,
            ..CurseFile::default()
        }
    }

    #[test]
    fn the_release_types_map_onto_the_channels() {
        let files = vec![
            file(1, 1, "2026-10-01T10:00:00Z"),
            file(2, 2, "2026-10-10T10:00:00Z"),
            file(3, 3, "2026-10-05T10:00:00Z"),
        ];
        let id = |channel| file_on(&files, FOREVER, channel).map(|file| file.id);
        assert_eq!(id(Channel::Stable), Some(1), "the newest release");
        assert_eq!(id(Channel::Prerelease), Some(2), "the newest file");
        let unstable = vec![
            file(2, 2, "2026-10-10T10:00:00Z"),
            file(3, 3, "2026-10-12T10:00:00Z"),
        ];
        assert_eq!(
            file_on(&unstable, FOREVER, Channel::Stable).map(|file| file.id),
            Some(3),
            "no release: the newest file, as GitHub's prerelease rule"
        );
        // A file that is not available, or names another game version type only, is left out.
        let mut gone = file(4, 1, "2026-10-20T10:00:00Z");
        gone.available = false;
        let mut retail = file(5, 1, "2026-10-21T10:00:00Z");
        retail.game_versions = vec![(Some(RETAIL), "12.1.5".to_string())];
        let mixed = vec![file(1, 1, "2026-10-01T10:00:00Z"), gone, retail];
        assert_eq!(
            file_on(&mixed, FOREVER, Channel::Stable).map(|file| file.id),
            Some(1)
        );
        assert_eq!(
            file_on(&mixed, RETAIL, Channel::Stable).map(|file| file.id),
            Some(5)
        );
        assert_eq!(file_on(&[], FOREVER, Channel::Stable), None);
    }

    #[test]
    fn the_game_version_type_follows_the_flavor_and_the_build() {
        let of = |flavor: FlavorId, version: Option<&str>| {
            game_version_type(&Target {
                version: version.and_then(BuildVersion::parse),
                ..Target::new(flavor)
            })
        };
        assert_eq!(of(FlavorId::Retail, None), Some((517, "retail", false)));
        assert_eq!(
            of(FlavorId::ClassicEra, None),
            Some((67408, "classic", false))
        );
        assert_eq!(of(FlavorId::Forever, None), Some((88568, "forever", false)));
        for (build, expected) in [
            ("5.5.4.70531", (79434, "mop", false)),
            ("4.4.2.1", (77522, "cata", false)),
            ("3.80.2.1", (81212, "titan", false)),
            ("3.4.5.1", (73713, "wotlk", false)),
            ("2.5.6.1", (73246, "bc", false)),
        ] {
            assert_eq!(
                of(FlavorId::Classic, Some(build)),
                Some(expected),
                "{build}"
            );
        }
        assert_eq!(of(FlavorId::Classic, None), Some((79434, "mop", true)));
        assert_eq!(of(FlavorId::Other("wow_x".into()), None), None);
    }

    #[test]
    fn an_answer_reads_the_project_and_its_files() {
        let project = parse_mod(&serde_json::json!({
            "data": {
                "id": 9, "gameId": 1, "name": "A", "allowModDistribution": null,
                "links": { "websiteUrl": "https://www.curseforge.com/wow/addons/a" },
                "latestFilesIndexes": [
                    { "fileId": 1, "gameVersionTypeId": 517, "releaseType": 1 },
                    { "fileId": 2, "gameVersionTypeId": 88568, "releaseType": 2 }
                ]
            }
        }))
        .expect("a project");
        assert_eq!(project.allow_distribution, None);
        assert_eq!(project.forever_build(), ForeverBuild::Yes);
        assert_eq!(project.lists(CLASSIC_ERA), Some(false));
        let no_list = parse_mod(&serde_json::json!({ "data": { "id": 9 } })).expect("a project");
        assert_eq!(no_list.forever_build(), ForeverBuild::Unknown);
        let retail_only = parse_mod(&serde_json::json!({ "data": { "latestFilesIndexes": [
            { "fileId": 1, "gameVersionTypeId": 517 }
        ] } }))
        .expect("a project");
        assert_eq!(retail_only.forever_build(), ForeverBuild::No);
        assert!(parse_mod(&serde_json::json!([])).is_err());

        let files = parse_files(&serde_json::json!({ "data": [
            { "id": 7, "displayName": "1.0.0", "fileName": "A-1.0.0.zip", "releaseType": 1,
              "fileLength": 10, "downloadUrl": null, "isAvailable": true,
              "hashes": [ { "value": "da39a3ee5e6b4b0d3255bfef95601890afd80709", "algo": 1 },
                          { "value": "D41D8CD98F00B204E9800998ECF8427E", "algo": 2 } ],
              "sortableGameVersions": [ { "gameVersion": "1.60.1", "gameVersionTypeId": 88568 } ],
              "modules": [ { "name": "A", "fingerprint": 1 }, { "name": "A_Options" } ] },
            { "displayName": "no id" }
        ] }))
        .expect("files");
        assert_eq!(files.len(), 1, "a file without an id is left out");
        let one = &files[0];
        assert_eq!(one.md5.as_deref(), Some("d41d8cd98f00b204e9800998ecf8427e"));
        assert_eq!(one.download_url, None);
        assert_eq!(one.interface(FOREVER), Some(16001));
        assert!(one.fits(FOREVER) && !one.fits(RETAIL));
        assert_eq!(one.modules, ["A", "A_Options"]);
        assert!(parse_files(&serde_json::json!({ "data": {} })).is_err());
    }

    #[test]
    fn a_redirect_is_followed_with_the_key_on_curseforges_hosts_only() {
        assert_eq!(
            next_url(
                "https://edge.forgecdn.net/files/1/2/a.zip",
                "//cdn.example.test/a.zip"
            ),
            "https://cdn.example.test/a.zip"
        );
        assert_eq!(
            next_url(
                "https://edge.forgecdn.net/files/1/2/a.zip?x=1",
                "/files/3/a.zip"
            ),
            "https://edge.forgecdn.net/files/3/a.zip"
        );
        assert_eq!(
            next_url("https://edge.forgecdn.net/files/1/2/a.zip", "b.zip"),
            "https://edge.forgecdn.net/files/1/2/b.zip"
        );
        let http = MapHttp::new()
            .answer("https://edge.forgecdn.net/files/1/2/a.zip", 302, "")
            .with_header(
                "https://edge.forgecdn.net/files/1/2/a.zip",
                "Location",
                "https://media.example.test/a.zip",
            )
            .answer("https://media.example.test/a.zip", 200, "zip");
        let source = CurseForge::new(http, Some("made-up-key".into()));
        let answer = source
            .send("https://edge.forgecdn.net/files/1/2/a.zip", false)
            .expect("an answer");
        assert_eq!((answer.status, answer.body.as_slice()), (200, &b"zip"[..]));
        let requests = source.http().requests();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| !request.follow_redirects));
        assert_eq!(requests[0].header_value(KEY_HEADER), Some("made-up-key"));
        assert_eq!(requests[1].header_value(KEY_HEADER), None, "another host");
    }
}
