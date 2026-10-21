//! The GitHub release source (task 003). The releases come from the REST API
//! (`GET /repos/{owner}/{repo}/releases`, an optional token from `MRGL_GITHUB_TOKEN` against the rate limit),
//! the current release is the newest one that is neither a draft nor a prerelease (on an addon's
//! `prerelease` channel the newest one that is not a draft, task 021), and its asset for the flavor is
//! picked from the release's `release.json`, the file the BigWigs packager uploads next to the
//! zips:
//!
//! ```json
//! { "releases": [ { "filename": "Addon-v1.2.3-forever.zip", "nolib": false,
//!                   "metadata": [ { "flavor": "forever", "interface": 16001 } ] } ] }
//! ```
//!
//! A release without `release.json`, or one whose metadata names other flavors only, falls back to the
//! asset names (`-forever`, `-classic`, `-mists`, …, the retail zip being the one with no suffix) and the
//! pick says it is a guess. The release list is cached with its `ETag` under the mrgl home folder
//! (`cache/github/<owner>/<repo>/`), so a second run costs one conditional request and no rate limit. A
//! first page of 30 releases without a stable one (a repository that publishes nightly prereleases) is
//! followed by the next pages the `Link` header names, up to [`MAX_PAGES`] (task 042).
//!
//! The same release says whether the addon publishes a Forever build at all ([`forever_build`], task 011):
//! the pick carries it, and every release's `release.json` is cached per tag next to the list (task 042), so
//! a repeated resolve of the same release downloads it once and `list` can answer from the cache alone
//! ([`GitHub::cached_forever_build`]), offline.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::forever::ForeverBuild;
use crate::sources::{
    self, Channel, Http, Pick, Request, Response, Source, SourceSpec, Target, notes_line,
};
use crate::{home, state};

/// The environment variable with a personal access token; without one the API allows 60 requests an hour.
pub const TOKEN_ENV: &str = "MRGL_GITHUB_TOKEN";
/// The REST API.
pub const API_URL: &str = "https://api.github.com";
/// The metadata file the packager uploads with the zips.
pub const RELEASE_JSON: &str = "release.json";
/// The cache folder under the mrgl home folder.
pub const CACHE_SUBDIR: &str = "cache/github";
/// The cached release list of one repository, under `<cache>/<owner>/<repo>/`.
pub const CACHE_FILE: &str = "releases.json";
/// The folder of the cached `release.json` files of a repository, one `tag-<tag>.json` per release, next to
/// [`CACHE_FILE`] (tasks 011, 042).
pub const RELEASE_JSON_CACHE_DIR: &str = "release-json";
/// How many releases one page lists; the current one is near the top.
const PER_PAGE: u32 = 30;
/// How many pages of the release list a resolve reads at most while none of them holds a stable release
/// (task 042): 150 releases, a nightly prerelease a day for five months. Past it the newest prerelease is
/// the current release, as for a repository without a stable one.
pub const MAX_PAGES: usize = 5;

/// The source over any [`Http`]: the real client in the exes, a stub in the tests.
pub struct GitHub<H: Http> {
    http: H,
    token: Option<String>,
    cache_dir: Option<PathBuf>,
}

/// One release as the API lists it; only the fields the pick needs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// The release notes as the author wrote them (Markdown); missing or `null` when there are none.
    #[serde(default)]
    pub body: Option<String>,
    /// The release's page on GitHub.
    #[serde(default)]
    pub html_url: Option<String>,
}

impl Release {
    /// The version the tag carries: the tag without a leading `v` (`v1.2.3` is `1.2.3`, `1.2.3` stays).
    pub fn version(&self) -> &str {
        let tag = self.tag_name.as_str();
        match tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')) {
            Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => rest,
            _ => tag,
        }
    }

    /// The release's `release.json` asset, when the packager uploaded one.
    pub fn release_json_asset(&self) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case(RELEASE_JSON))
    }
}

/// One uploaded file of a release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: Option<u64>,
}

/// `release.json` as the packager writes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ReleaseJson {
    #[serde(default)]
    pub releases: Vec<ReleaseJsonEntry>,
}

/// One zip of a `release.json`, with the flavors it was built for.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReleaseJsonEntry {
    pub filename: String,
    #[serde(default)]
    pub nolib: bool,
    #[serde(default)]
    pub metadata: Vec<Metadata>,
}

/// One flavor of a zip: the packager's flavor key and the interface number of its TOC.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Metadata {
    pub flavor: String,
    pub interface: u32,
}

/// The newest release that is neither a draft nor a prerelease; a prerelease only when there is nothing
/// else. The API lists the newest first. This is the stable channel's rule, [`current_release_on`] with
/// [`Channel::Stable`].
pub fn current_release(releases: &[Release]) -> Option<&Release> {
    current_release_on(releases, Channel::Stable)
}

/// The current release on a channel (task 021): on the stable one the newest release that is neither a
/// draft nor a prerelease, a prerelease only when there is nothing else; on the prerelease one the newest
/// release that is not a draft, prerelease or not. The API lists the newest first.
pub fn current_release_on(releases: &[Release], channel: Channel) -> Option<&Release> {
    let mut published = releases.iter().filter(|release| !release.draft);
    match channel {
        Channel::Prerelease => published.next(),
        Channel::Stable => published
            .clone()
            .find(|release| !release.prerelease)
            .or_else(|| published.next()),
    }
}

/// The `release.json` flavor keys (and asset-name suffixes) a target accepts, best first: the packager's
/// own key, then the synonyms the client's TOC suffixes spell. The progressing classic has one flavor id for
/// several game types, so its key follows the installed build's major version; without a build every game
/// type counts, newest first, and the pick is a guess.
pub fn flavor_keys(target: &Target) -> Vec<&'static str> {
    match &target.flavor {
        FlavorId::Forever => vec!["forever", "camelot"],
        FlavorId::Retail => vec!["mainline", "standard"],
        FlavorId::ClassicEra => vec!["classic", "vanilla"],
        FlavorId::Classic => match target.major() {
            Some(5) => vec!["mists"],
            Some(4) => vec!["cata"],
            Some(3) => vec!["wrath", "wotlkc"],
            Some(2) => vec!["bcc", "tbc"],
            _ => vec!["mists", "cata", "wrath", "wotlkc", "bcc", "tbc"],
        },
        FlavorId::Other(_) => Vec::new(),
    }
}

/// Every flavor suffix an asset name can end with; a retail zip ends with none of them.
const FLAVOR_SUFFIXES: [&str; 12] = [
    "forever", "camelot", "mainline", "standard", "classic", "vanilla", "mists", "cata", "wrath",
    "wotlkc", "bcc", "tbc",
];

/// The progressing classic has no single game type: a pick for it without a known build is a guess.
fn game_type_unknown(target: &Target) -> bool {
    target.flavor == FlavorId::Classic && !matches!(target.major(), Some(2..=5))
}

/// An asset chosen for a target, before it becomes a [`Pick`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen<'a> {
    pub asset: &'a Asset,
    pub flavor: String,
    pub interface: Option<u32>,
    pub guess: bool,
    pub reason: String,
    /// The asset is a `nolib` build: its libraries are separate addons, none is embedded (task 029).
    pub nolib: bool,
}

/// The asset a `release.json` names for the target: the first key with an entry, a library-included zip
/// before a `nolib` one, and among several entries of one key the one whose interface is the installed
/// build's major version. `None` when no entry names any of the target's keys.
pub fn pick_by_metadata<'a>(
    release: &'a Release,
    release_json: &ReleaseJson,
    target: &Target,
) -> Option<Chosen<'a>> {
    let asset_of = |filename: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case(filename))
    };
    let major = target.major();
    for key in flavor_keys(target) {
        let mut candidates: Vec<(&Asset, u32, bool)> = release_json
            .releases
            .iter()
            .filter_map(|entry| {
                let metadata = entry
                    .metadata
                    .iter()
                    .find(|metadata| metadata.flavor.eq_ignore_ascii_case(key))?;
                Some((asset_of(&entry.filename)?, metadata.interface, entry.nolib))
            })
            .collect();
        if candidates.is_empty() {
            continue;
        }
        // Library-included first (stable, so the file order breaks the tie), then the build's major.
        candidates.sort_by_key(|(_, _, nolib)| *nolib);
        let (asset, interface, nolib) = candidates
            .iter()
            .find(|(_, interface, _)| major.is_some_and(|major| interface / 10000 == major))
            .or_else(|| candidates.first())
            .copied()?;
        let guess = game_type_unknown(target);
        let reason = if guess {
            format!(
                "release.json names {key} (interface {interface}); the installed build is unknown, so the game type is a guess"
            )
        } else {
            format!("release.json names {key} (interface {interface})")
        };
        return Some(Chosen {
            asset,
            flavor: key.to_string(),
            interface: Some(interface),
            guess,
            reason,
            nolib,
        });
    }
    None
}

/// Whether a release publishes a `forever` build, whatever flavor is being picked for (task 011): `Yes` when
/// its `release.json` names `forever` (or `camelot`) for a zip the release has; without a `release.json`
/// (none, or an unreadable one) `Unknown`; with one that names no Forever zip, `Unknown` when an asset name
/// still ends in `-forever` or `-camelot` (a pick by name, a guess either way) and `No` otherwise.
pub fn forever_build(release: &Release, release_json: Option<&ReleaseJson>) -> ForeverBuild {
    let Some(release_json) = release_json else {
        return ForeverBuild::Unknown;
    };
    let forever = Target::new(FlavorId::Forever);
    let keys = flavor_keys(&forever);
    let has_asset = |filename: &str| {
        release
            .assets
            .iter()
            .any(|asset| asset.name.eq_ignore_ascii_case(filename))
    };
    let named = release_json.releases.iter().any(|entry| {
        has_asset(&entry.filename)
            && entry.metadata.iter().any(|metadata| {
                keys.iter()
                    .any(|key| metadata.flavor.eq_ignore_ascii_case(key))
            })
    });
    if named {
        ForeverBuild::Yes
    } else if pick_by_name(release, &forever).is_some() {
        ForeverBuild::Unknown
    } else {
        ForeverBuild::No
    }
}

/// The asset whose name ends with one of the target's keys (`-forever`, `-mists`, …, before `.zip` and an
/// optional `-nolib`), library-included first; for retail also the zip with no flavor suffix at all, which
/// is how the packager names it. Always a guess: nothing but a file name says what the zip is for.
pub fn pick_by_name<'a>(release: &'a Release, target: &Target) -> Option<Chosen<'a>> {
    let stems: Vec<(&Asset, String, bool)> = release
        .assets
        .iter()
        .filter_map(|asset| {
            let lower = asset.name.to_ascii_lowercase();
            let stem = lower.strip_suffix(".zip")?;
            let (stem, nolib) = match stem.strip_suffix("-nolib") {
                Some(stem) => (stem, true),
                None => (stem, false),
            };
            Some((asset, stem.to_string(), nolib))
        })
        .collect();
    let pick = |asset: &'a Asset, flavor: &str, nolib: bool, reason: String| Chosen {
        asset,
        flavor: flavor.to_string(),
        interface: None,
        guess: true,
        reason,
        nolib,
    };
    for key in flavor_keys(target) {
        let suffix = format!("-{key}");
        let mut hits: Vec<&(&Asset, String, bool)> = stems
            .iter()
            .filter(|(_, stem, _)| stem.ends_with(&suffix))
            .collect();
        hits.sort_by_key(|(_, _, nolib)| *nolib);
        if let Some((asset, _, nolib)) = hits.first() {
            return Some(pick(
                asset,
                key,
                *nolib,
                format!(
                    "no release.json names {}; the asset name ends in {suffix}, a guess",
                    target.flavor_name()
                ),
            ));
        }
    }
    if target.flavor == FlavorId::Retail {
        let mut hits: Vec<&(&Asset, String, bool)> = stems
            .iter()
            .filter(|(_, stem, _)| {
                !FLAVOR_SUFFIXES
                    .iter()
                    .any(|suffix| stem.ends_with(&format!("-{suffix}")))
            })
            .collect();
        hits.sort_by_key(|(_, _, nolib)| *nolib);
        if let Some((asset, _, nolib)) = hits.first() {
            return Some(pick(
                asset,
                "mainline",
                *nolib,
                "no release.json names retail; the asset name has no flavor suffix, which the packager gives the retail zip, a guess".to_string(),
            ));
        }
    }
    None
}

impl<H: Http> GitHub<H> {
    /// A source with an explicit token and cache folder (`None`: no cache).
    pub fn new(http: H, token: Option<String>, cache_dir: Option<PathBuf>) -> GitHub<H> {
        GitHub {
            http,
            token: token.filter(|token| !token.is_empty()),
            cache_dir,
        }
    }

    /// The source of this process: the token from `MRGL_GITHUB_TOKEN`, the cache under the mrgl home
    /// folder (no cache when there is no home folder).
    pub fn from_env(http: H) -> GitHub<H> {
        let token = std::env::var(TOKEN_ENV).ok();
        let cache_dir = home::home_dir().ok().map(|home| home.join(CACHE_SUBDIR));
        GitHub::new(http, token, cache_dir)
    }

    /// The client behind the source.
    pub fn http(&self) -> &H {
        &self.http
    }

    /// The cache file of a repository: the release list with its tag, one file so the two never disagree.
    fn cache_file(&self, owner: &str, repo: &str) -> Option<PathBuf> {
        self.cache_path(owner, repo, CACHE_FILE)
    }

    /// A file of a repository's cache folder, `<cache>/<owner>/<repo>/<file>`, lower case.
    fn cache_path(&self, owner: &str, repo: &str, file: &str) -> Option<PathBuf> {
        Some(
            self.cache_dir
                .as_ref()?
                .join(owner.to_ascii_lowercase())
                .join(repo.to_ascii_lowercase())
                .join(file),
        )
    }

    /// The cached `release.json` of one release: `<cache>/<owner>/<repo>/release-json/tag-<tag>.json`, the
    /// tag made file-safe (the file names the tag again, so two tags that read the same here never mix).
    fn release_json_cache_path(&self, owner: &str, repo: &str, tag: &str) -> Option<PathBuf> {
        self.cache_path(owner, repo, RELEASE_JSON_CACHE_DIR)
            .map(|dir| dir.join(format!("tag-{}.json", state::file_safe(tag))))
    }

    /// The cached `release.json` of a release, when the cache holds the one of this tag and this asset.
    fn cached_release_json(
        &self,
        owner: &str,
        repo: &str,
        release: &Release,
        asset: &Asset,
    ) -> Option<serde_json::Value> {
        let path = self.release_json_cache_path(owner, repo, &release.tag_name)?;
        let cached: CachedReleaseJson = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        (cached.release == release.tag_name
            && cached.url.as_deref() == Some(asset.browser_download_url.as_str())
            && cached.size == asset.size)
            .then_some(cached.release_json)
    }

    /// Whether the repository publishes a `forever` build, from the cache alone (task 011): the cached
    /// release list's current release and the `release.json` cached for its tag, never a request, so
    /// `list` stays offline and fast. `Unknown` when nothing is cached, the cache is unreadable, the
    /// current release has no `release.json`, or none is cached for its tag (the list moved on and no `add`
    /// has fetched the new one yet).
    pub fn cached_forever_build(&self, spec: &SourceSpec) -> ForeverBuild {
        let SourceSpec::GitHub { owner, repo } = spec;
        let Some(cached) = self
            .cache_file(owner, repo)
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<CachedReleases<Vec<Release>>>(&bytes).ok())
        else {
            return ForeverBuild::Unknown;
        };
        let Some(release) = current_release(&cached.releases) else {
            return ForeverBuild::Unknown;
        };
        let release_json = release
            .release_json_asset()
            .and_then(|asset| self.cached_release_json(owner, repo, release, asset))
            .and_then(|raw| serde_json::from_value::<ReleaseJson>(raw).ok());
        match release_json {
            Some(release_json) => forever_build(release, Some(&release_json)),
            None => ForeverBuild::Unknown,
        }
    }

    fn api_request(&self, url: String) -> Request {
        let mut request = Request::get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        request
    }

    /// The releases of a repository, newest first, from the API or, when the API says nothing changed,
    /// from the cache. A first page without a stable release is followed by the next ones (task 042):
    /// the pages the `Link` header names as `next`, on the API host only, until one holds a stable
    /// release or [`MAX_PAGES`] are read; the cache keeps every page read with the first page's tag.
    pub fn releases(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        let url = format!("{API_URL}/repos/{owner}/{repo}/releases?per_page={PER_PAGE}");
        let cache_file = self.cache_file(owner, repo);
        let cached: Option<CachedReleases<serde_json::Value>> = cache_file
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let mut request = self.api_request(url.clone());
        if let Some(etag) = cached.as_ref().and_then(|cached| cached.etag.as_ref()) {
            request = request.header("If-None-Match", etag.clone());
        }
        let response = self.http.get(&request)?;
        let unreadable = |reason: String| Error::SourceDocument {
            what: "release list",
            spec: format!("github:{owner}/{repo}"),
            reason,
        };
        let page = |response: &Response| {
            serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
                .map_err(|reason| unreadable(reason.to_string()))
        };
        let releases: serde_json::Value = match response.status {
            200 => {
                let mut releases = page(&response)?;
                let mut next = next_page(&response);
                let mut pages = 1;
                while pages < MAX_PAGES
                    && !has_stable_release(&releases)
                    && let Some(url) = next.take()
                {
                    let response = self.http.get(&self.api_request(url.clone()))?;
                    if response.status != 200 {
                        return Err(status_error(response.status, &url, &response));
                    }
                    releases.extend(page(&response)?);
                    next = next_page(&response);
                    pages += 1;
                }
                let releases = serde_json::Value::Array(releases);
                if let Some(path) = &cache_file {
                    let cached = CachedReleases {
                        etag: response.header("etag").map(str::to_string),
                        releases: &releases,
                    };
                    // The cache is a convenience: a folder that cannot be written costs a request per
                    // run, never the command.
                    let _ = write_cache(path, &cached);
                }
                releases
            }
            304 => match cached {
                Some(cached) => cached.releases,
                None => return Err(status_error(304, &url, &response)),
            },
            status => return Err(status_error(status, &url, &response)),
        };
        serde_json::from_value(releases).map_err(|reason| unreadable(reason.to_string()))
    }

    /// A release's `release.json`, when it has one: `Ok(None)` without the asset, `Ok(Some(Err(reason)))`
    /// when the file is there but not the packager's shape (the caller falls back and says so).
    pub fn release_json(
        &self,
        spec: &SourceSpec,
        release: &Release,
    ) -> Result<Option<std::result::Result<ReleaseJson, String>>> {
        let Some(asset) = release.release_json_asset() else {
            return Ok(None);
        };
        let SourceSpec::GitHub { owner, repo } = spec;
        // A release's `release.json` does not change once uploaded: the cached one of this tag and this
        // asset answers without a request (task 042).
        if let Some(raw) = self.cached_release_json(owner, repo, release, asset)
            && let Ok(release_json) = ReleaseJson::deserialize(&raw)
        {
            return Ok(Some(Ok(release_json)));
        }
        let response = self.download(&asset.browser_download_url)?;
        let parsed = serde_json::from_slice::<serde_json::Value>(&response.body).and_then(|raw| {
            let release_json = ReleaseJson::deserialize(&raw)?;
            Ok((raw, release_json))
        });
        match parsed {
            Ok((raw, release_json)) => {
                if let Some(path) = self.release_json_cache_path(owner, repo, &release.tag_name) {
                    let cached = CachedReleaseJson {
                        release: release.tag_name.clone(),
                        url: Some(asset.browser_download_url.clone()),
                        size: asset.size,
                        release_json: raw,
                    };
                    // A convenience like the release list: without it, the next resolve downloads it
                    // again and `list` says unknown.
                    let _ = write_cache(&path, &cached);
                }
                Ok(Some(Ok(release_json)))
            }
            Err(reason) => Ok(Some(Err(format!(
                "unreadable {RELEASE_JSON} in {spec} {}: {reason}",
                release.tag_name
            )))),
        }
    }

    /// A release asset's bytes: no token (the download hops to the object store, and the asset is public).
    fn download(&self, url: &str) -> Result<Response> {
        let response = self.http.get(&Request::get(url))?;
        if response.status != 200 {
            return Err(status_error(response.status, url, &response));
        }
        Ok(response)
    }
}

/// The cache file's shape: the release list as the API sent it, with the tag to ask again with.
#[derive(Debug, Serialize, Deserialize)]
struct CachedReleases<T> {
    #[serde(default)]
    etag: Option<String>,
    releases: T,
}

/// The cached `release.json` of one release, with the tag of the release and the asset it came from (a
/// re-uploaded asset has another size or URL, and is downloaded again).
#[derive(Debug, Serialize, Deserialize)]
struct CachedReleaseJson {
    release: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    release_json: serde_json::Value,
}

/// Whether a page of the release list holds a stable release: neither a draft nor a prerelease.
fn has_stable_release(releases: &[serde_json::Value]) -> bool {
    let flag = |release: &serde_json::Value, name: &str| {
        release.get(name).and_then(serde_json::Value::as_bool) == Some(true)
    };
    releases
        .iter()
        .any(|release| !flag(release, "draft") && !flag(release, "prerelease"))
}

/// The next page of a paged answer: the `Link` header's `rel="next"` URL, when it is on the API host (the
/// token goes with it, so a link anywhere else is not followed).
fn next_page(response: &Response) -> Option<String> {
    response.header("link")?.split(',').find_map(|link| {
        let (url, params) = link.trim().split_once(';')?;
        let url = url.trim().strip_prefix('<')?.strip_suffix('>')?;
        let next = params.split(';').any(|param| {
            param
                .trim()
                .strip_prefix("rel=")
                .is_some_and(|rel| rel.trim_matches('"').split(' ').any(|rel| rel == "next"))
        });
        (next && url.starts_with(&format!("{API_URL}/"))).then(|| url.to_string())
    })
}

/// Writes a cache file through a temporary file and a rename, so a reader (another run at the same time)
/// never sees a half-written file.
fn write_cache<T: Serialize>(path: &Path, cached: &T) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("no parent folder"))?;
    std::fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| CACHE_FILE.to_string());
    let temp = parent.join(format!("{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec(cached)?)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// A status the source cannot work with, with the hint the user can act on. A 429, or a 403 with no request
/// left, is the rate limit: the hint names when it resets, in local time, from `x-ratelimit-reset` (else
/// from `retry-after`), and the token variable (task 042).
fn status_error(status: u16, url: &str, response: &Response) -> Error {
    let rate_limited = status == 429
        || response
            .header("x-ratelimit-remaining")
            .is_some_and(|remaining| remaining.trim() == "0");
    let hint = match status {
        304 => "nothing changed, but there is no cached copy to read".to_string(),
        401 => format!("the token in {TOKEN_ENV} is not accepted"),
        403 | 429 if rate_limited => match rate_limit_reset(response) {
            Some(reset) => format!(
                "the rate limit is used up until {} (local time); set {TOKEN_ENV} to a token of your own for 5000 requests an hour",
                sources::local_time_text(reset)
            ),
            None => format!(
                "the rate limit is used up; set {TOKEN_ENV} to a token of your own for 5000 requests an hour"
            ),
        },
        403 => "access denied".to_string(),
        404 => "no such repository, or it is private".to_string(),
        500..=599 => "the server has a problem; try again later".to_string(),
        _ => "an unexpected status".to_string(),
    };
    Error::SourceStatus {
        host: "GitHub",
        status,
        url: url.to_string(),
        hint,
    }
}

/// When the rate limit resets, in Unix seconds: `x-ratelimit-reset`, else now plus `retry-after` (the
/// secondary limit's header).
fn rate_limit_reset(response: &Response) -> Option<i64> {
    let number = |name: &str| {
        response
            .header(name)
            .and_then(|value| value.trim().parse::<i64>().ok())
    };
    number("x-ratelimit-reset").or_else(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        Some(i64::try_from(now).ok()? + number("retry-after")?)
    })
}

impl<H: Http> Source for GitHub<H> {
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick> {
        let SourceSpec::GitHub { owner, repo } = spec;
        let releases = self.releases(owner, repo)?;
        let release =
            current_release_on(&releases, target.channel).ok_or_else(|| Error::NoRelease {
                spec: spec.to_string(),
            })?;
        let (release_json, mut notes) = match self.release_json(spec, release)? {
            Some(Ok(release_json)) => (Some(release_json), None),
            Some(Err(reason)) => (None, Some(reason)),
            None => (None, None),
        };
        let chosen = release_json
            .as_ref()
            .and_then(|release_json| pick_by_metadata(release, release_json, target))
            .or_else(|| pick_by_name(release, target));
        let Some(chosen) = chosen else {
            let mut named: Vec<String> = release_json
                .iter()
                .flat_map(|release_json| &release_json.releases)
                .flat_map(|entry| &entry.metadata)
                .map(|metadata| metadata.flavor.to_ascii_lowercase())
                .collect();
            named.sort();
            named.dedup();
            let assets: Vec<&str> = release
                .assets
                .iter()
                .map(|asset| asset.name.as_str())
                .collect();
            let mut detail = match (&release_json, notes.take()) {
                (Some(_), _) if !named.is_empty() => {
                    format!("{RELEASE_JSON} names {}", named.join(", "))
                }
                (Some(_), _) => format!("{RELEASE_JSON} names no flavor"),
                (None, Some(reason)) => reason,
                (None, None) => format!("no {RELEASE_JSON}"),
            };
            detail.push_str(&format!("; assets: {}", assets.join(", ")));
            return Err(Error::NoAssetForFlavor {
                spec: spec.to_string(),
                release: release.tag_name.clone(),
                flavor: target.flavor_name().to_string(),
                detail,
            });
        };
        let reason = match notes {
            Some(note) => format!("{note}; {}", chosen.reason),
            None => chosen.reason,
        };
        let forever_build = forever_build(release, release_json.as_ref());
        Ok(Pick {
            source: spec.to_string(),
            release: release.tag_name.clone(),
            version: release.version().to_string(),
            published: release.published_at.clone(),
            prerelease: release.prerelease,
            asset: chosen.asset.name.clone(),
            url: chosen.asset.browser_download_url.clone(),
            size: chosen.asset.size,
            flavor: chosen.flavor,
            interface: chosen.interface,
            guess: chosen.guess,
            reason,
            notes: release.body.as_deref().and_then(notes_line),
            page: release.html_url.clone().filter(|page| !page.is_empty()),
            nolib: chosen.nolib,
            forever_build,
        })
    }

    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        let response = self.download(&pick.url)?;
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
mod tests {
    use super::*;
    use crate::flavor::BuildVersion;
    use crate::sources::MapHttp;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.to_string(),
            browser_download_url: format!("https://github.test/download/{name}"),
            size: Some(1),
        }
    }

    fn release(tag: &str, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.to_string(),
            name: None,
            draft: false,
            prerelease: false,
            published_at: None,
            assets: assets.iter().map(|name| asset(name)).collect(),
            body: None,
            html_url: None,
        }
    }

    fn entry(filename: &str, nolib: bool, metadata: &[(&str, u32)]) -> ReleaseJsonEntry {
        ReleaseJsonEntry {
            filename: filename.to_string(),
            nolib,
            metadata: metadata
                .iter()
                .map(|(flavor, interface)| Metadata {
                    flavor: flavor.to_string(),
                    interface: *interface,
                })
                .collect(),
        }
    }

    fn classic(major: u32) -> Target {
        Target {
            version: BuildVersion::parse(&format!("{major}.0.0.1")),
            ..Target::new(FlavorId::Classic)
        }
    }

    #[test]
    fn the_version_drops_a_leading_v() {
        assert_eq!(release("v1.2.3", &[]).version(), "1.2.3");
        assert_eq!(release("1.2.3", &[]).version(), "1.2.3");
        assert_eq!(release("V2", &[]).version(), "2");
        assert_eq!(release("version-1", &[]).version(), "version-1");
    }

    #[test]
    fn the_current_release_skips_drafts_and_prefers_a_stable_one() {
        let mut draft = release("v3", &[]);
        draft.draft = true;
        let mut beta = release("v2-beta", &[]);
        beta.prerelease = true;
        let stable = release("v1", &[]);
        let releases = vec![draft.clone(), beta.clone(), stable.clone()];
        assert_eq!(current_release(&releases), Some(&stable));
        assert_eq!(current_release(&[draft.clone(), beta.clone()]), Some(&beta));
        assert_eq!(current_release(&[draft.clone()]), None);
        assert_eq!(current_release(&[]), None);

        // The prerelease channel takes the newest release that is not a draft, prerelease or not.
        let prerelease = Channel::Prerelease;
        assert_eq!(current_release_on(&releases, prerelease), Some(&beta));
        let newer_stable = release("v4", &[]);
        let releases = vec![newer_stable.clone(), beta.clone(), stable.clone()];
        assert_eq!(
            current_release_on(&releases, prerelease),
            Some(&newer_stable)
        );
        assert_eq!(current_release_on(&[draft.clone()], prerelease), None);
        assert_eq!(current_release_on(&[], prerelease), None);
        assert_eq!(
            current_release_on(&[draft, stable.clone()], Channel::Stable),
            Some(&stable)
        );
    }

    #[test]
    fn the_keys_follow_the_flavor_and_the_build() {
        assert_eq!(
            flavor_keys(&Target::new(FlavorId::Forever)),
            ["forever", "camelot"]
        );
        assert_eq!(flavor_keys(&Target::new(FlavorId::Retail))[0], "mainline");
        assert_eq!(
            flavor_keys(&Target::new(FlavorId::ClassicEra))[0],
            "classic"
        );
        assert_eq!(flavor_keys(&classic(5)), ["mists"]);
        assert_eq!(flavor_keys(&classic(4)), ["cata"]);
        assert_eq!(flavor_keys(&classic(2)), ["bcc", "tbc"]);
        assert_eq!(flavor_keys(&Target::new(FlavorId::Classic))[0], "mists");
        assert_eq!(flavor_keys(&classic(9))[0], "mists");
        assert!(flavor_keys(&Target::new(FlavorId::Other("wow_x".into()))).is_empty());
    }

    #[test]
    fn metadata_picks_the_flavor_key_and_prefers_the_library_zip() {
        let release = release(
            "v1",
            &[
                "A-v1.zip",
                "A-v1-nolib.zip",
                "A-v1-forever.zip",
                "A-v1-forever-nolib.zip",
            ],
        );
        let release_json = ReleaseJson {
            releases: vec![
                entry("A-v1-forever-nolib.zip", true, &[("forever", 16001)]),
                entry("A-v1-nolib.zip", true, &[("mainline", 120105)]),
                entry("A-v1.zip", false, &[("mainline", 120105)]),
                entry("A-v1-forever.zip", false, &[("forever", 16001)]),
            ],
        };
        let chosen = pick_by_metadata(&release, &release_json, &Target::new(FlavorId::Forever))
            .expect("a pick");
        assert_eq!(chosen.asset.name, "A-v1-forever.zip");
        assert_eq!(chosen.flavor, "forever");
        assert_eq!(chosen.interface, Some(16001));
        assert!(!chosen.guess);
        assert_eq!(
            chosen.reason,
            "release.json names forever (interface 16001)"
        );
        assert!(!chosen.nolib, "the library zip");
        let retail = pick_by_metadata(&release, &release_json, &Target::new(FlavorId::Retail))
            .expect("a pick");
        assert_eq!(retail.asset.name, "A-v1.zip");
        assert!(pick_by_metadata(&release, &release_json, &classic(5)).is_none());
    }

    #[test]
    fn the_chosen_asset_carries_its_nolib_flag() {
        let release = release("v1", &["A-v1-forever-nolib.zip", "B-v1-nolib.zip"]);
        let release_json = ReleaseJson {
            releases: vec![entry("A-v1-forever-nolib.zip", true, &[("forever", 16001)])],
        };
        let by_metadata =
            pick_by_metadata(&release, &release_json, &Target::new(FlavorId::Forever))
                .expect("the nolib zip, the only one");
        assert!(by_metadata.nolib);
        let by_name = pick_by_name(&release, &Target::new(FlavorId::Forever)).expect("by name");
        assert_eq!(by_name.asset.name, "A-v1-forever-nolib.zip");
        assert!(by_name.nolib);
        let retail = pick_by_name(&release, &Target::new(FlavorId::Retail)).expect("retail");
        assert_eq!(retail.asset.name, "B-v1-nolib.zip");
        assert!(retail.nolib);
    }

    #[test]
    fn metadata_for_the_progressing_classic_follows_the_build() {
        let release = release("v1", &["A-cata.zip", "A-mists.zip"]);
        let release_json = ReleaseJson {
            releases: vec![
                entry("A-cata.zip", false, &[("cata", 40402)]),
                entry("A-mists.zip", false, &[("mists", 50504)]),
            ],
        };
        let mists = pick_by_metadata(&release, &release_json, &classic(5)).expect("mists");
        assert_eq!(mists.asset.name, "A-mists.zip");
        assert!(!mists.guess);
        let cata = pick_by_metadata(&release, &release_json, &classic(4)).expect("cata");
        assert_eq!(cata.asset.name, "A-cata.zip");
        let unknown = pick_by_metadata(&release, &release_json, &Target::new(FlavorId::Classic))
            .expect("the newest game type");
        assert_eq!(unknown.asset.name, "A-mists.zip");
        assert!(unknown.guess);
        assert!(unknown.reason.contains("the installed build is unknown"));
    }

    #[test]
    fn metadata_ignores_an_entry_without_its_asset() {
        let release = release("v1", &["A-v1.zip"]);
        let release_json = ReleaseJson {
            releases: vec![entry("A-v1-forever.zip", false, &[("forever", 16001)])],
        };
        assert!(
            pick_by_metadata(&release, &release_json, &Target::new(FlavorId::Forever)).is_none()
        );
    }

    #[test]
    fn names_are_a_guess_with_the_retail_zip_the_unsuffixed_one() {
        let release = release(
            "v1",
            &[
                "A-v1-nolib.zip",
                "A-v1.zip",
                "A-v1-classic.zip",
                "A-v1-forever.zip",
                "release.json",
                "A-v1.txt",
            ],
        );
        let forever = pick_by_name(&release, &Target::new(FlavorId::Forever)).expect("forever");
        assert_eq!(forever.asset.name, "A-v1-forever.zip");
        assert_eq!(forever.flavor, "forever");
        assert!(forever.guess);
        assert_eq!(forever.interface, None);
        assert!(forever.reason.contains("-forever"), "{}", forever.reason);
        let retail = pick_by_name(&release, &Target::new(FlavorId::Retail)).expect("retail");
        assert_eq!(retail.asset.name, "A-v1.zip");
        assert_eq!(retail.flavor, "mainline");
        let era = pick_by_name(&release, &Target::new(FlavorId::ClassicEra)).expect("era");
        assert_eq!(era.asset.name, "A-v1-classic.zip");
        assert!(pick_by_name(&release, &classic(5)).is_none());
        let only_suffixed = Release {
            assets: vec![asset("A-classic.zip")],
            ..release.clone()
        };
        assert!(pick_by_name(&only_suffixed, &Target::new(FlavorId::Retail)).is_none());
    }

    #[test]
    fn the_forever_build_is_yes_no_or_unknown() {
        let release = release("v1", &["A-v1.zip", "A-v1-forever.zip", "A-v1-camelot.zip"]);
        let named = |entries: Vec<ReleaseJsonEntry>| ReleaseJson { releases: entries };
        // Without a release.json nothing says, whatever the asset names are.
        assert_eq!(forever_build(&release, None), ForeverBuild::Unknown);
        assert_eq!(
            forever_build(
                &release,
                Some(&named(vec![entry(
                    "A-v1-forever.zip",
                    false,
                    &[("forever", 16001)]
                )]))
            ),
            ForeverBuild::Yes
        );
        assert_eq!(
            forever_build(
                &release,
                Some(&named(vec![entry(
                    "A-v1-camelot.zip",
                    false,
                    &[("Camelot", 16001)]
                )]))
            ),
            ForeverBuild::Yes,
            "the synonym, in any case"
        );
        // A release.json for other flavors with a -forever asset next to it: a pick by name, unknown.
        let mainline = named(vec![entry("A-v1.zip", false, &[("mainline", 120105)])]);
        assert_eq!(
            forever_build(&release, Some(&mainline)),
            ForeverBuild::Unknown
        );
        let plain = self::release("v1", &["A-v1.zip"]);
        assert_eq!(forever_build(&plain, Some(&mainline)), ForeverBuild::No);
        // An entry that names forever for a zip the release does not have does not count.
        let missing = named(vec![entry(
            "A-v1-forever.zip",
            false,
            &[("forever", 16001)],
        )]);
        assert_eq!(forever_build(&plain, Some(&missing)), ForeverBuild::No);
    }

    const RELEASES_URL: &str = "https://api.github.com/repos/o/r/releases?per_page=30";

    fn releases_body(assets: &[&str]) -> String {
        let assets: Vec<String> = assets
            .iter()
            .map(|name| {
                format!(
                    "{{\"name\":\"{name}\",\"browser_download_url\":\"https://dl.test/{name}\",\"size\":3}}"
                )
            })
            .collect();
        format!(
            "[{{\"tag_name\":\"v1.0.0\",\"draft\":false,\"prerelease\":false,\"published_at\":\"2026-10-21T00:00:00Z\",\"assets\":[{}]}}]",
            assets.join(",")
        )
    }

    fn spec() -> SourceSpec {
        SourceSpec::GitHub {
            owner: "o".into(),
            repo: "r".into(),
        }
    }

    #[test]
    fn resolve_sends_the_api_headers_and_the_token() {
        let http =
            MapHttp::new().answer(RELEASES_URL, 200, &releases_body(&["A-v1.0.0-forever.zip"]));
        let github = GitHub::new(http, Some("secret".into()), None);
        let pick = github
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .expect("a pick");
        assert_eq!(pick.source, "github:o/r");
        assert_eq!(pick.release, "v1.0.0");
        assert_eq!(pick.version, "1.0.0");
        assert_eq!(pick.asset, "A-v1.0.0-forever.zip");
        assert_eq!(pick.url, "https://dl.test/A-v1.0.0-forever.zip");
        assert_eq!(pick.size, Some(3));
        assert_eq!(pick.published.as_deref(), Some("2026-10-21T00:00:00Z"));
        assert!(pick.guess);
        let requests = github.http().requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].header_value("Authorization"),
            Some("Bearer secret")
        );
        assert_eq!(
            requests[0].header_value("Accept"),
            Some("application/vnd.github+json")
        );
        assert_eq!(requests[0].header_value("If-None-Match"), None);
    }

    #[test]
    fn an_empty_token_is_no_token() {
        let http = MapHttp::new().answer(RELEASES_URL, 200, "[]");
        let github = GitHub::new(http, Some(String::new()), None);
        let error = github
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .unwrap_err();
        assert_eq!(error.to_string(), "no release in github:o/r");
        assert_eq!(
            github.http().requests()[0].header_value("Authorization"),
            None
        );
    }

    #[test]
    fn the_statuses_have_hints() {
        for (status, needle) in [
            (404, "no such repository"),
            (401, TOKEN_ENV),
            (403, "access denied"),
            (503, "try again later"),
            (418, "an unexpected status"),
        ] {
            let http = MapHttp::new().answer(RELEASES_URL, status, "");
            let error = GitHub::new(http, None, None)
                .resolve(&spec(), &Target::new(FlavorId::Forever))
                .unwrap_err()
                .to_string();
            assert!(
                error.starts_with(&format!("GitHub answered {status} for "))
                    && error.contains(needle),
                "{error}"
            );
        }
        let rate_limited = Response {
            status: 403,
            headers: vec![("X-RateLimit-Remaining".into(), "0".into())],
            body: Vec::new(),
        };
        let error = status_error(403, "u", &rate_limited).to_string();
        assert!(
            error.contains("rate limit") && error.contains(TOKEN_ENV),
            "{error}"
        );
    }

    #[test]
    fn an_unreadable_release_list_is_a_document_error() {
        let http = MapHttp::new().answer(RELEASES_URL, 200, "{ not a list");
        let error = GitHub::new(http, None, None)
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("unreadable release list from github:o/r: "),
            "{error}"
        );
    }

    #[test]
    fn an_unreadable_release_json_falls_back_to_the_names_and_says_so() {
        let http = MapHttp::new()
            .answer(
                RELEASES_URL,
                200,
                &releases_body(&["release.json", "A-forever.zip"]),
            )
            .answer("https://dl.test/release.json", 200, "nope");
        let pick = GitHub::new(http, None, None)
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .expect("a pick by name");
        assert_eq!(pick.asset, "A-forever.zip");
        assert!(pick.guess);
        assert!(
            pick.reason
                .starts_with("unreadable release.json in github:o/r v1.0.0: "),
            "{}",
            pick.reason
        );
        let http = MapHttp::new()
            .answer(
                RELEASES_URL,
                200,
                &releases_body(&["release.json", "A.zip"]),
            )
            .answer("https://dl.test/release.json", 200, "nope");
        let error = GitHub::new(http, None, None)
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with(
                "no asset for flavor forever in github:o/r v1.0.0 (unreadable release.json"
            ),
            "{error}"
        );
        assert!(error.ends_with("; assets: release.json, A.zip)"), "{error}");
    }

    #[test]
    fn a_missing_release_json_download_is_a_status_error() {
        let http = MapHttp::new().answer(RELEASES_URL, 200, &releases_body(&["release.json"]));
        let error = GitHub::new(http, None, None)
            .resolve(&spec(), &Target::new(FlavorId::Forever))
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("GitHub answered 404 for https://dl.test/release.json"),
            "{error}"
        );
    }

    #[test]
    fn fetch_writes_the_asset_and_creates_the_folder() {
        let dir = std::env::temp_dir().join(format!("mrgl-fetch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let http = MapHttp::new().answer("https://dl.test/A.zip", 200, "zip bytes");
        let github = GitHub::new(http, Some("secret".into()), None);
        let pick = Pick {
            source: "github:o/r".into(),
            release: "v1".into(),
            version: "1".into(),
            published: None,
            prerelease: false,
            asset: "A.zip".into(),
            url: "https://dl.test/A.zip".into(),
            size: None,
            flavor: "forever".into(),
            interface: None,
            guess: false,
            reason: String::new(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: ForeverBuild::Unknown,
        };
        let to = dir.join("nested").join("A.zip");
        github.fetch(&pick, &to).expect("fetched");
        assert_eq!(std::fs::read(&to).unwrap(), b"zip bytes");
        // The token stays with the API: a download carries no Authorization header.
        assert_eq!(
            github.http().requests()[0].header_value("Authorization"),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_pick_carries_the_notes_line_and_the_page() {
        let with_body = |body: &str| {
            format!(
                "[{{\"tag_name\":\"v1.0.0\",\"html_url\":\"https://github.test/o/r/releases/tag/v1.0.0\",\"body\":{body},\"assets\":[{{\"name\":\"A-forever.zip\",\"browser_download_url\":\"https://dl.test/A-forever.zip\"}}]}}]"
            )
        };
        let resolve = |list: &str| {
            GitHub::new(MapHttp::new().answer(RELEASES_URL, 200, list), None, None)
                .resolve(&spec(), &Target::new(FlavorId::Forever))
                .expect("a pick")
        };
        let pick = resolve(&with_body(r#""\r\n## Faster scans\r\n\r\n- one\r\n- two""#));
        assert_eq!(pick.notes.as_deref(), Some("Faster scans"));
        assert_eq!(
            pick.page.as_deref(),
            Some("https://github.test/o/r/releases/tag/v1.0.0")
        );
        for empty in [r#""""#, "null", r#""\r\n#\r\n""#] {
            let pick = resolve(&with_body(empty));
            assert_eq!(pick.notes, None, "{empty}");
            assert!(pick.page.is_some(), "{empty}");
        }
        // A release list without the two fields at all, as the older fixtures have it.
        let pick = resolve(&releases_body(&["A-forever.zip"]));
        assert_eq!((pick.notes, pick.page), (None, None));
    }
}
