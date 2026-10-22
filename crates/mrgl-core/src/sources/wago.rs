//! The Wago source (task 095): `wago:<id>` through the Wago Addons external API with the user's own key
//! (`MRGL_WAGO_KEY` or the key store's `mrgl:wago`, read through [`crate::keys`], task 093; D-002: mrgl ships
//! none). `GET https://addons.wago.io/api/external/addons/<id>?game_version=<v>` with
//! `Authorization: Bearer <key>` answers the addon for one game version ([`game_version`]: `retail`,
//! `classic` for Classic Era, `forever`, and the progressing classic by the installed build, `bc`, `wotlk`,
//! `titan`, `cata`, `mop`) with its three recent releases, `recent_release.{stable,beta,alpha}`, each with its
//! `label` (the version), `download_link`, `created_at`, changelog and the patches it supports. The stable
//! channel takes `stable`, else the newest of the other two; the prerelease channel the newest of the three
//! ([`release_on`]). The download goes with the same key, and the key goes to `addons.wago.io` only
//! ([`keys::may_send`]); a 401 names the key.
//!
//! Every request to Wago's host counts against the limit its answers name (`x-ratelimit-limit`,
//! `x-ratelimit-remaining`; 60 until an answer says, the window unnamed), which the update check reads before
//! each addon ([`Source::request_budget`]). The answers are cached per addon under `cache/wago/<id>.json`, one
//! entry per game version with when it was read, and the offline rule of the GitHub source holds (task 041).
//!
//! The update check asks in bulk (task 106): `POST https://addons.wago.io/api/external/addons/_recents?game_version=<v>`
//! with `{"addons": [<id>, …]}` answers every addon of a game version in one request ([`Wago::recents`],
//! [`parse_recents`]), each answer held for the run and written into the same cache file as a single answer;
//! an addon the bulk answer lacks, or a bulk request that fails, is asked alone. `add` and `update <name>`
//! keep the single request.
//!
//! The Forever signal: `yes` when the addon has a release for `forever`, `no` when Wago answers without one.
//! The keyless public data endpoint `GET https://addons.wago.io/api/data/slugs?game_version=<v>` (the slug and
//! id of every addon with a release for that game version) answers it without a key, and the id of a slug
//! ([`Wago::id_of_slug`]); its lists are read once a day and cached under `cache/wago/slugs-<v>.json`. A Wago
//! page address (`addons.wago.io/addons/<slug>`, task 105) becomes `wago:<id>` through that list of the
//! target's game version ([`Source::canonical_spec`]), so the sidecar records the id.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::forever::ForeverBuild;
use crate::sources::github::{rate_limit_reset, write_cache};
use crate::sources::{
    self, BuildOffer, Channel, Http, Offered, Pick, Request, RequestBudget, Response, Source,
    SourceKind, SourceSpec, Stale, Target, notes_line,
};
use crate::{home, keys, state};

/// The external API, which takes the user's key.
pub const API_URL: &str = "https://addons.wago.io/api/external";
/// The public data endpoints, which take none.
pub const DATA_URL: &str = "https://addons.wago.io/api/data";
/// The host's name in a message line.
pub const HOST: &str = "Wago";
/// The cache folder under the mrgl home folder.
pub const CACHE_SUBDIR: &str = "cache/wago";
/// The requests Wago's answers allow (`x-ratelimit-limit: 60`), until an answer says otherwise.
pub const LIMIT: u32 = 60;
/// How long a cached slug list answers without a request: a day.
pub const SLUGS_MAX_AGE_SECONDS: i64 = 86_400;
/// The stabilities of `recent_release`, the stable one first.
pub const STABILITIES: [&str; 3] = ["stable", "beta", "alpha"];

/// The game version Wago names a target's flavor by, and whether it is a guess: the progressing classic
/// follows the installed build's major (and Titan's `3.80` apart from Wrath's `3.4`), as the GitHub source's
/// flavor keys do; without a build it is the newest game type, `mop`, a guess. `None` for a flavor id mrgl
/// does not know.
pub fn game_version(target: &Target) -> Option<(&'static str, bool)> {
    Some(match &target.flavor {
        FlavorId::Retail => ("retail", false),
        FlavorId::ClassicEra => ("classic", false),
        FlavorId::Forever => ("forever", false),
        FlavorId::Classic => match target.version {
            Some(build) if build.major == 5 => ("mop", false),
            Some(build) if build.major == 4 => ("cata", false),
            Some(build) if build.major == 3 && build.minor >= 80 => ("titan", false),
            Some(build) if build.major == 3 => ("wotlk", false),
            Some(build) if build.major == 2 => ("bc", false),
            _ => ("mop", true),
        },
        FlavorId::Other(_) => return None,
    })
}

/// One recent release of an addon for a game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WagoRelease {
    /// `stable`, `beta` or `alpha`.
    pub stability: &'static str,
    /// The version as the author labels it.
    pub label: String,
    pub download_link: String,
    /// When it was published, as Wago writes it (ISO 8601 in UTC).
    pub created_at: Option<String>,
    /// The game patches it supports for the game version (`1.60.1`).
    pub patches: Vec<String>,
    pub changelog: Option<String>,
}

impl WagoRelease {
    /// The interface number of the newest patch it supports (`1.60.1` is 16001), when it names one.
    pub fn interface(&self) -> Option<u32> {
        self.patches
            .iter()
            .filter_map(|patch| interface_of_patch(patch))
            .max()
    }
}

/// An addon as one answer of the external API has it for a game version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WagoAddon {
    pub id: Option<String>,
    pub slug: Option<String>,
    pub display_name: Option<String>,
    /// The recent releases the answer names, stable, beta, alpha, those that are there.
    pub releases: Vec<WagoRelease>,
}

/// The interface number of a game patch: `12.1.5` is 120105, `1.60.1` is 16001.
pub fn interface_of_patch(patch: &str) -> Option<u32> {
    let mut parts = patch.trim().split('.').map(|part| part.parse::<u32>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    (minor < 100 && patch < 100).then(|| major * 10_000 + minor * 100 + patch)
}

/// Reads one answer of the external API for a game version: the addon (an object, or one under `data`)
/// and its recent releases, each with a label and a download link; a release without either is left out.
pub fn parse_addon(
    raw: &serde_json::Value,
    game_version: &str,
) -> std::result::Result<WagoAddon, String> {
    let addon = match raw.get("data") {
        Some(data) if data.is_object() => data,
        _ => raw,
    };
    if !addon.is_object() {
        return Err("expected an object".to_string());
    }
    let text = |value: &serde_json::Value, name: &str| {
        value
            .get(name)
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string)
    };
    let recent = addon.get("recent_release");
    let releases = STABILITIES
        .iter()
        .filter_map(|stability| {
            let release = recent?.get(*stability)?;
            Some(WagoRelease {
                stability,
                label: text(release, "label")?,
                download_link: text(release, "download_link")?,
                created_at: text(release, "created_at"),
                patches: patches_of(release, game_version),
                changelog: text(release, "changelog"),
            })
        })
        .collect();
    Ok(WagoAddon {
        id: text(addon, "id"),
        slug: text(addon, "slug"),
        display_name: text(addon, "display_name"),
        releases,
    })
}

/// The patches a release supports for a game version: `supported_<v>_patches` (a list or one text),
/// `supported_<v>_patch`, else `supported_patches`.
fn patches_of(release: &serde_json::Value, game_version: &str) -> Vec<String> {
    let field = [
        format!("supported_{game_version}_patches"),
        format!("supported_{game_version}_patch"),
        "supported_patches".to_string(),
    ]
    .into_iter()
    .find_map(|name| release.get(&name).filter(|value| !value.is_null()).cloned());
    match field {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(serde_json::Value::String(text)) => text
            .split(',')
            .map(str::trim)
            .filter(|patch| !patch.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// The release a channel takes (task 095): on the stable channel the `stable` release, else the newest of
/// the others (a prerelease only when nothing else is there, the GitHub rule); on the prerelease channel the
/// newest of the three. Newest by `created_at`; on a tie, or without dates, the stabler one.
pub fn release_on(addon: &WagoAddon, channel: Channel) -> Option<&WagoRelease> {
    let newest = || {
        addon.releases.iter().reduce(|best, release| {
            if release.created_at > best.created_at {
                release
            } else {
                best
            }
        })
    };
    match channel {
        Channel::Stable => addon
            .releases
            .iter()
            .find(|release| release.stability == "stable")
            .or_else(newest),
        Channel::Prerelease => newest(),
    }
}

/// The clause of a spec of another source handed to this source itself (the router never does).
const NOT_WAGO: &str = "the Wago source answers wago: specs only";

/// The addon id of a Wago spec; a spec of another source is [`Error::SourceNotConfigured`].
fn addon_id(spec: &SourceSpec) -> Result<&str> {
    match spec {
        SourceSpec::Wago { id } => Ok(id),
        other => Err(Error::SourceNotConfigured {
            spec: other.to_string(),
            missing: NOT_WAGO.to_string(),
        }),
    }
}

/// The clause of a run without the key.
fn no_key() -> String {
    format!(
        "{} is not set: the Wago API needs your own key, `mrgl key set wago` stores it",
        keys::WAGO_KEY
    )
}

/// The cached answers of one addon: one per game version, as the API sent it, with when it was read.
#[derive(Debug, Default, Serialize, Deserialize)]
struct CachedAddon {
    #[serde(default)]
    answers: BTreeMap<String, CachedAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAnswer {
    #[serde(default)]
    checked_at: Option<String>,
    addon: serde_json::Value,
}

/// Reads the answer of the bulk endpoint (task 106) for the ids asked: an object or a list of addons under
/// `addons` (or under `data`, or the answer itself), each the shape of one addon's answer; an addon of an
/// object is named by its `id`, else by its key. Ids not asked, and entries that are no object, are left
/// out. The answers by id.
pub fn parse_recents(raw: &serde_json::Value, ids: &[&str]) -> BTreeMap<String, serde_json::Value> {
    let root = match raw.get("data") {
        Some(data) if data.is_object() || data.is_array() => data,
        _ => raw,
    };
    let addons = root.get("addons").unwrap_or(root);
    let named: Vec<(Option<&str>, &serde_json::Value)> = match addons {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, addon)| (Some(key.as_str()), addon))
            .collect(),
        serde_json::Value::Array(items) => items.iter().map(|addon| (None, addon)).collect(),
        _ => Vec::new(),
    };
    named
        .into_iter()
        .filter(|(_, addon)| addon.is_object())
        .filter_map(|(key, addon)| {
            let id = addon
                .get("id")
                .and_then(serde_json::Value::as_str)
                .or(key)?;
            ids.contains(&id).then(|| (id.to_string(), addon.clone()))
        })
        .collect()
}

/// The cached slug list of one game version: every addon with a release for it, slug to id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Slugs {
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub addons: BTreeMap<String, String>,
}

impl Slugs {
    /// Whether an addon id is on the list.
    pub fn has_id(&self, id: &str) -> bool {
        self.addons.values().any(|listed| listed == id)
    }
}

/// The requests of one run to Wago's host and what the answers said of the limit.
#[derive(Debug, Default)]
struct BudgetCount {
    sent: u32,
    limit: Option<u32>,
    remaining: Option<(u32, u32)>,
    reset: Option<i64>,
}

/// The source over any [`Http`]: the real client in the exes, a stub in the tests.
pub struct Wago<H: Http> {
    http: H,
    key: Option<String>,
    /// Without a key the source cannot be asked ([`Source::missing`]); the window's offline view
    /// ([`Wago::cached`]) answers from the cache instead.
    needs_key: bool,
    cache_dir: Option<PathBuf>,
    budget: Mutex<BudgetCount>,
    slugs: Mutex<BTreeMap<String, Arc<Slugs>>>,
    /// The answers of this run's bulk lookups (task 106), by game version and id.
    held: Mutex<BTreeMap<(String, String), serde_json::Value>>,
}

impl<H: Http> Wago<H> {
    /// A source with an explicit key and cache folder (`None`: no cache).
    pub fn new(http: H, key: Option<String>, cache_dir: Option<PathBuf>) -> Wago<H> {
        Wago {
            http,
            key: key.filter(|key| !key.trim().is_empty()),
            needs_key: true,
            cache_dir,
            budget: Mutex::new(BudgetCount::default()),
            slugs: Mutex::new(BTreeMap::new()),
            held: Mutex::new(BTreeMap::new()),
        }
    }

    /// The source of this process: the key from `MRGL_WAGO_KEY` or the key store ([`keys::of_process`]),
    /// the cache under the mrgl home folder.
    pub fn from_env(http: H) -> Wago<H> {
        let key = keys::of_process(SourceKind::Wago).map(keys::Key::into_value);
        let cache_dir = home::home_dir().ok().map(|home| home.join(CACHE_SUBDIR));
        Wago::new(http, key, cache_dir)
    }

    /// The source of the window's offline view (task 078, [`super::Sources::cached`]): no key, and every
    /// resolve answers from the cache alone, never a request to the external API.
    pub fn cached(http: H, cache_dir: Option<PathBuf>) -> Wago<H> {
        Wago {
            needs_key: false,
            ..Wago::new(http, None, cache_dir)
        }
    }

    /// The client behind the source.
    pub fn http(&self) -> &H {
        &self.http
    }

    fn cache_path(&self, file: &str) -> Option<PathBuf> {
        Some(self.cache_dir.as_ref()?.join(file))
    }

    fn addon_cache(&self, id: &str) -> Option<PathBuf> {
        self.cache_path(&format!("{}.json", state::file_safe(id)))
    }

    fn cached_answer(&self, id: &str, game_version: &str) -> Option<CachedAnswer> {
        let bytes = std::fs::read(self.addon_cache(id)?).ok()?;
        let cached: CachedAddon = serde_json::from_slice(&bytes).ok()?;
        cached.answers.get(game_version).cloned()
    }

    /// A GET to Wago's host through the client ([`Wago::send_request`]).
    fn send(&self, url: &str, with_key: bool) -> Result<Response> {
        self.send_request(
            Request::get(url).header("Accept", "application/json"),
            with_key,
        )
    }

    /// A request to Wago's host through the client, a POST when it has a body, with the key on the external
    /// API's host only ([`keys::may_send`]), counted against the rate limit with what the answer says of it.
    fn send_request(&self, mut request: Request, with_key: bool) -> Result<Response> {
        let url = request.url.clone();
        let url = url.as_str();
        if with_key
            && let Some(key) = &self.key
            && keys::may_send(SourceKind::Wago, url)
        {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        let response = match request.body {
            Some(_) => self.http.post(&request)?,
            None => self.http.get(&request)?,
        };
        if keys::hosts(SourceKind::Wago)
            .iter()
            .any(|host| url_host(url).eq_ignore_ascii_case(host))
            && let Ok(mut budget) = self.budget.lock()
        {
            budget.sent += 1;
            let number = |name: &str| {
                response
                    .header(name)
                    .and_then(|value| value.trim().parse::<u32>().ok())
            };
            if let Some(limit) = number("x-ratelimit-limit") {
                budget.limit = Some(limit);
            }
            if let Some(remaining) =
                number("x-ratelimit-remaining").or((response.status == 429).then_some(0))
            {
                budget.remaining = Some((remaining, budget.sent));
            }
            if let Some(reset) = rate_limit_reset(&response) {
                budget.reset = Some(reset);
            }
        }
        Ok(response)
    }

    /// The addon for a game version, from the external API with the key, else, when the API cannot be
    /// reached and the answer is cached, from the cache with [`Stale`] (the offline rule of task 041). The
    /// window's offline view without a key reads the cache alone.
    pub fn addon(&self, id: &str, game_version: &str) -> Result<(WagoAddon, Option<Stale>)> {
        let spec = format!("wago:{id}");
        // Task 106: an answer of this run's bulk lookup needs no request of its own.
        if let Some(raw) = self.held_answer(id, game_version) {
            let addon =
                parse_addon(&raw, game_version).map_err(|reason| Error::SourceDocument {
                    what: "addon",
                    spec: spec.clone(),
                    reason,
                })?;
            return Ok((addon, None));
        }
        let url = format!("{API_URL}/addons/{id}?game_version={game_version}");
        let from_cache = |reason: String, error: Error| -> Result<(WagoAddon, Option<Stale>)> {
            let Some(cached) = self.cached_answer(id, game_version) else {
                return Err(error);
            };
            let addon = parse_addon(&cached.addon, game_version).map_err(|reason| {
                Error::SourceDocument {
                    what: "addon",
                    spec: spec.clone(),
                    reason,
                }
            })?;
            Ok((
                addon,
                Some(Stale {
                    checked_at: cached.checked_at,
                    reason,
                }),
            ))
        };
        if self.key.is_none() {
            let error = Error::SourceNotConfigured {
                spec: spec.clone(),
                missing: no_key(),
            };
            return from_cache(format!("{} is not set in this view", keys::WAGO_KEY), error);
        }
        let response = match self.send(&url, true) {
            Ok(response) => response,
            Err(error @ Error::Network { .. }) => return from_cache(error.to_string(), error),
            Err(error) => return Err(error),
        };
        if response.status != 200 {
            return Err(status_error(response.status, &url, &response, NO_ADDON));
        }
        let raw: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|reason| Error::SourceDocument {
                what: "addon",
                spec: spec.clone(),
                reason: reason.to_string(),
            })?;
        let addon = parse_addon(&raw, game_version).map_err(|reason| Error::SourceDocument {
            what: "addon",
            spec: spec.clone(),
            reason,
        })?;
        self.store(id, game_version, raw);
        Ok((addon, None))
    }

    /// Writes an addon's answer for a game version into its cache file, next to the answers of the other
    /// game versions, with when it was read.
    fn store(&self, id: &str, game_version: &str, raw: serde_json::Value) {
        let Some(path) = self.addon_cache(id) else {
            return;
        };
        let mut cached: CachedAddon = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        cached.answers.insert(
            game_version.to_string(),
            CachedAnswer {
                checked_at: Some(state::now_rfc3339()),
                addon: raw,
            },
        );
        // The cache is a convenience: a folder that cannot be written costs the offline answer.
        let _ = write_cache(&path, &cached);
    }

    /// The answer a bulk lookup of this run holds for an addon and a game version.
    fn held_answer(&self, id: &str, game_version: &str) -> Option<serde_json::Value> {
        self.held
            .lock()
            .ok()?
            .get(&(game_version.to_string(), id.to_string()))
            .cloned()
    }

    /// The addons of one game version in one request (task 106): `POST /addons/_recents?game_version=<v>`
    /// with `{"addons": [<id>, …]}` and the key, each answer that reads as an addon held for the run (the
    /// next [`Wago::addon`] of it sends nothing) and written into its cache file as a single answer is. The
    /// ids the answer had; an id it lacks is asked alone later. Without the key nothing is sent.
    pub fn recents(&self, ids: &[&str], game_version: &str) -> Result<Vec<String>> {
        let url = format!("{API_URL}/addons/_recents?game_version={game_version}");
        if self.key.is_none() {
            return Err(Error::SourceNotConfigured {
                spec: format!("wago:{}", ids.join(", wago:")),
                missing: no_key(),
            });
        }
        let body = serde_json::json!({ "addons": ids }).to_string();
        let request = Request::post(url.as_str(), body)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        let response = self.send_request(request, true)?;
        if response.status != 200 {
            return Err(status_error(
                response.status,
                &url,
                &response,
                "no bulk answer there",
            ));
        }
        let raw: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|reason| Error::SourceDocument {
                what: "bulk answer",
                spec: format!("wago: {game_version}"),
                reason: reason.to_string(),
            })?;
        let mut answered = Vec::new();
        for (id, addon) in parse_recents(&raw, ids) {
            if parse_addon(&addon, game_version).is_err() {
                continue;
            }
            self.store(&id, game_version, addon.clone());
            if let Ok(mut held) = self.held.lock() {
                held.insert((game_version.to_string(), id.clone()), addon);
            }
            answered.push(id);
        }
        Ok(answered)
    }

    /// The slug list of a game version (every addon with a release for it), from the keyless data endpoint
    /// without a key: from the cache while it is younger than a day, else from Wago (cached), the cache of any
    /// age standing in when Wago cannot answer. Read once per run.
    pub fn slugs(&self, game_version: &str) -> Result<Arc<Slugs>> {
        if let Some(slugs) = self
            .slugs
            .lock()
            .ok()
            .and_then(|held| held.get(game_version).cloned())
        {
            return Ok(slugs);
        }
        let cache = self.cache_path(&format!("slugs-{}.json", state::file_safe(game_version)));
        let cached: Option<Slugs> = cache
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let fresh = cached.as_ref().is_some_and(|cached| {
            match (
                cached
                    .checked_at
                    .as_deref()
                    .and_then(sources::unix_from_rfc3339),
                sources::unix_from_rfc3339(&state::now_rfc3339()),
            ) {
                (Some(then), Some(now)) => (0..SLUGS_MAX_AGE_SECONDS).contains(&(now - then)),
                _ => false,
            }
        });
        let slugs = match cached {
            Some(cached) if fresh => cached,
            cached => match self.fetch_slugs(game_version) {
                Ok(slugs) => {
                    if let Some(path) = &cache {
                        let _ = write_cache(path, &slugs);
                    }
                    slugs
                }
                Err(error) => cached.ok_or(error)?,
            },
        };
        let slugs = Arc::new(slugs);
        if let Ok(mut held) = self.slugs.lock() {
            held.insert(game_version.to_string(), Arc::clone(&slugs));
        }
        Ok(slugs)
    }

    /// `slugs?game_version=<v>` from the data endpoint, no key: `{"addons": {"<slug>": {"id": …}}}`.
    fn fetch_slugs(&self, game_version: &str) -> Result<Slugs> {
        let url = format!("{DATA_URL}/slugs?game_version={game_version}");
        let response = self.send(&url, false)?;
        if response.status != 200 {
            return Err(status_error(
                response.status,
                &url,
                &response,
                "no slug list there",
            ));
        }
        let raw: serde_json::Value =
            serde_json::from_slice(&response.body).map_err(|reason| Error::SourceDocument {
                what: "slug list",
                spec: format!("wago: {game_version}"),
                reason: reason.to_string(),
            })?;
        let addons = raw
            .get("addons")
            .and_then(serde_json::Value::as_object)
            .map(|addons| {
                addons
                    .iter()
                    .filter_map(|(slug, entry)| {
                        let id = entry.get("id").and_then(serde_json::Value::as_str)?;
                        Some((slug.clone(), id.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Slugs {
            checked_at: Some(state::now_rfc3339()),
            addons,
        })
    }

    /// The id of an addon by its slug (the name of its page, `addons.wago.io/addons/<slug>`), from the slug
    /// list of a game version, without a key; the slug as written, else ASCII case-insensitive; `Ok(None)`
    /// when the list does not have it.
    pub fn id_of_slug(&self, slug: &str, game_version: &str) -> Result<Option<String>> {
        let slugs = self.slugs(game_version)?;
        Ok(slugs.addons.get(slug).cloned().or_else(|| {
            slugs
                .addons
                .iter()
                .find(|(listed, _)| listed.eq_ignore_ascii_case(slug))
                .map(|(_, id)| id.clone())
        }))
    }

    /// The Forever signal from what is cached, never a request: the cached `forever` answer (a release on
    /// the channel: `yes`, none: `no`), else the cached slug list of `forever` of any age, else `unknown`.
    fn forever_from_cache(&self, id: &str, channel: Channel) -> ForeverBuild {
        if let Some(cached) = self.cached_answer(id, "forever")
            && let Ok(addon) = parse_addon(&cached.addon, "forever")
        {
            return yes_or_no(release_on(&addon, channel).is_some());
        }
        if let Some(slugs) = self
            .slugs
            .lock()
            .ok()
            .and_then(|held| held.get("forever").cloned())
        {
            return yes_or_no(slugs.has_id(id));
        }
        self.cache_path("slugs-forever.json")
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Slugs>(&bytes).ok())
            .map_or(ForeverBuild::Unknown, |slugs| yes_or_no(slugs.has_id(id)))
    }

    /// The Forever signal of an addon (task 095): from the cache when it can tell, else from the keyless
    /// slug list of `forever` (one request a day); `unknown` when neither answers.
    pub fn forever_build(&self, id: &str, channel: Channel) -> ForeverBuild {
        match self.forever_from_cache(id, channel) {
            ForeverBuild::Unknown => match self.slugs("forever") {
                Ok(slugs) => yes_or_no(slugs.has_id(id)),
                Err(_) => ForeverBuild::Unknown,
            },
            known => known,
        }
    }

    /// The pick of one release.
    fn pick_of(
        &self,
        spec: &SourceSpec,
        (addon, stale): (&WagoAddon, Option<Stale>),
        release: &WagoRelease,
        target: &Target,
        (game_version, game_guess): (&str, bool),
        why: String,
    ) -> Pick {
        let id = addon_id(spec).unwrap_or_default();
        let name = addon.slug.as_deref().unwrap_or(id);
        let patch = release
            .patches
            .iter()
            .max_by_key(|patch| interface_of_patch(patch))
            .map(|patch| format!(" (patch {patch})"))
            .unwrap_or_default();
        let mut reason = format!("{why}{patch}");
        if game_guess {
            reason.push_str("; the installed build is unknown, so the game type is a guess");
        }
        let forever_build = if game_version == "forever" {
            ForeverBuild::Yes
        } else {
            self.forever_build(id, target.channel)
        };
        Pick {
            source: spec.to_string(),
            release: release.label.clone(),
            version: sources::version_of_tag(&release.label).to_string(),
            published: release.created_at.clone(),
            prerelease: release.stability != "stable",
            asset: format!(
                "{}-{}.zip",
                state::file_safe(name),
                state::file_safe(&release.label)
            ),
            url: release.download_link.clone(),
            size: None,
            md5: None,
            flavor: game_version.to_string(),
            interface: release.interface(),
            guess: game_guess,
            reason,
            notes: release.changelog.as_deref().and_then(notes_line),
            page: addon
                .slug
                .as_ref()
                .map(|slug| format!("https://addons.wago.io/addons/{slug}")),
            nolib: false,
            forever_build,
            stale,
        }
    }

    /// The game version of a target, or the error of a flavor Wago names no game version for.
    fn game_of(spec: &SourceSpec, target: &Target) -> Result<(&'static str, bool)> {
        game_version(target).ok_or_else(|| Error::NoAssetForFlavor {
            spec: spec.to_string(),
            release: "-".to_string(),
            flavor: target.flavor_name().to_string(),
            detail: "Wago names no game version for this flavor".to_string(),
        })
    }
}

/// `yes` or `no`.
fn yes_or_no(yes: bool) -> ForeverBuild {
    if yes {
        ForeverBuild::Yes
    } else {
        ForeverBuild::No
    }
}

/// The host of a URL, without the scheme, the path and the query.
fn url_host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#']).next().unwrap_or_default()
}

/// The error of an addon with no release for the game version.
fn no_release(spec: &SourceSpec, target: &Target, game_version: &str) -> Error {
    Error::NoAssetForFlavor {
        spec: spec.to_string(),
        release: format!("game version {game_version}"),
        flavor: target.flavor_name().to_string(),
        detail: "Wago lists no stable, beta or alpha release for it".to_string(),
    }
}

/// The hint of an addon's 404.
const NO_ADDON: &str = "no such addon, or it was removed";

/// A status the source cannot work with, with the hint the user can act on: a 401 names the key, a 429 (or
/// an answer with no request left) the rate limit; `not_found` is a 404's.
fn status_error(status: u16, url: &str, response: &Response, not_found: &'static str) -> Error {
    let used_up = status == 429
        || response
            .header("x-ratelimit-remaining")
            .is_some_and(|remaining| remaining.trim() == "0");
    let hint = match status {
        401 => format!(
            "{} is missing or not accepted: the Wago API needs your own key, `mrgl key set wago` stores it",
            keys::WAGO_KEY
        ),
        403 | 429 if used_up => match rate_limit_reset(response) {
            Some(reset) => format!(
                "the rate limit is used up until {} (local time)",
                sources::local_time_text(reset)
            ),
            None => "the rate limit is used up; try again after a short wait".to_string(),
        },
        403 => "access denied".to_string(),
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

impl<H: Http> Source for Wago<H> {
    /// The release the target's channel takes for its game version ([`game_version`], [`release_on`]); a
    /// page address is looked up first ([`Source::canonical_spec`]), so the pick names `wago:<id>`.
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick> {
        let spec = &self.canonical_spec(spec, target)?;
        let id = addon_id(spec)?;
        let game = Self::game_of(spec, target)?;
        let (addon, stale) = self.addon(id, game.0)?;
        let release =
            release_on(&addon, target.channel).ok_or_else(|| no_release(spec, target, game.0))?;
        let why = if target.channel == Channel::Stable && release.stability != "stable" {
            format!(
                "Wago lists no stable release for {}; the newest, the {} release, is taken",
                game.0, release.stability
            )
        } else {
            format!("Wago lists a {} release for {}", release.stability, game.0)
        };
        Ok(self.pick_of(spec, (&addon, stale), release, target, game, why))
    }

    /// The recent release whose label is the version (with or without a leading `v`), any stability: Wago
    /// keeps the three recent ones, an older version is not offered.
    fn resolve_version(&self, spec: &SourceSpec, target: &Target, version: &str) -> Result<Pick> {
        let spec = &self.canonical_spec(spec, target)?;
        let id = addon_id(spec)?;
        let game = Self::game_of(spec, target)?;
        let (addon, stale) = self.addon(id, game.0)?;
        let release = addon
            .releases
            .iter()
            .find(|release| {
                release.label == version || sources::version_of_tag(&release.label) == version
            })
            .ok_or_else(|| Error::VersionNotOffered {
                spec: spec.to_string(),
                version: version.to_string(),
            })?;
        let why = format!("Wago lists a {} release for {}", release.stability, game.0);
        Ok(self.pick_of(spec, (&addon, stale), release, target, game, why))
    }

    /// Downloads the release with the key (on Wago's host only, [`keys::may_send`]); a 404 names the file.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        let response = self.send(&pick.url, true)?;
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
            status => return Err(status_error(status, &pick.url, &response, NO_ADDON)),
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

    /// The limit Wago's answers name, less the requests of this run since (task 095).
    fn request_budget(&self) -> Option<RequestBudget> {
        let budget = self.budget.lock().ok()?;
        let limit = budget.limit.unwrap_or(LIMIT);
        let remaining = match budget.remaining {
            Some((remaining, at)) => remaining.saturating_sub(budget.sent.saturating_sub(at)),
            None => limit.saturating_sub(budget.sent),
        };
        Some(RequestBudget {
            host: HOST,
            limit,
            sent: budget.sent,
            remaining,
            reset: budget.reset,
            hint: "run the check again then".to_string(),
            window: "",
            resets: "after a short wait (Wago does not say when)",
        })
    }

    /// A page address's slug looked up in the keyless slug list of the target's game version (task 105):
    /// `wago:<id>`, or [`Error::UnknownWagoSlug`] naming the game version; any other spec is its own.
    fn canonical_spec(&self, spec: &SourceSpec, target: &Target) -> Result<SourceSpec> {
        let SourceSpec::WagoPage { slug } = spec else {
            return Ok(spec.clone());
        };
        let (game_version, _) = Self::game_of(spec, target)?;
        match self.id_of_slug(slug, game_version)? {
            Some(id) => Ok(SourceSpec::Wago { id }),
            None => Err(Error::UnknownWagoSlug {
                slug: slug.clone(),
                game_version: game_version.to_string(),
            }),
        }
    }

    /// The update check's bulk lookup (task 106): the Wago specs of the target's game version, once, with
    /// the key and while the budget has a request left; a failed lookup leaves each addon to be asked
    /// alone. The window's offline view, without a key, sends nothing.
    fn prefetch(&self, specs: &[SourceSpec], target: &Target) {
        let Some((game_version, _)) = game_version(target) else {
            return;
        };
        let mut ids: Vec<&str> = specs
            .iter()
            .filter_map(|spec| addon_id(spec).ok())
            .filter(|id| self.held_answer(id, game_version).is_none())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let budget_left = self
            .request_budget()
            .is_none_or(|budget| budget.remaining > 0);
        if ids.is_empty() || self.key.is_none() || !budget_left {
            return;
        }
        let _ = self.recents(&ids, game_version);
    }

    fn holds(&self, spec: &SourceSpec, target: &Target) -> bool {
        match (addon_id(spec), game_version(target)) {
            (Ok(id), Some((game_version, _))) => self.held_answer(id, game_version).is_some(),
            _ => false,
        }
    }

    /// A spec of another source is not this source's to answer; without the key the Wago API cannot be
    /// asked (the window's offline view excepted, which reads the cache alone).
    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        if spec.kind() != SourceKind::Wago {
            return Some(NOT_WAGO.to_string());
        }
        (self.needs_key && self.key.is_none()).then(no_key)
    }

    /// The Forever signal from the cache alone (task 095): the cached `forever` answer, else the cached slug
    /// list of `forever`; never a request.
    fn cached_forever_build(&self, spec: &SourceSpec, channel: Channel) -> ForeverBuild {
        match addon_id(spec) {
            Ok(id) => self.forever_from_cache(id, channel),
            Err(_) => ForeverBuild::Unknown,
        }
    }

    /// Whether the addon has a release for the target's game version, from the keyless slug list of that
    /// game version (task 097): `yes` when the list names the id, `no` when it does not, `unknown` when the
    /// list cannot be read; with or without the key, one request a day per game version.
    fn offers(&self, spec: &SourceSpec, target: &Target) -> BuildOffer {
        let Ok(id) = addon_id(spec) else {
            return BuildOffer::new(Offered::Unknown, NOT_WAGO);
        };
        let Some((game_version, guess)) = game_version(target) else {
            return BuildOffer::new(
                Offered::Unknown,
                "Wago names no game version for this flavor",
            );
        };
        match self.slugs(game_version) {
            Ok(slugs) if slugs.has_id(id) && guess => BuildOffer::new(
                Offered::Unknown,
                format!(
                    "Wago lists a release for {game_version}; the installed build is unknown, so the game type is a guess"
                ),
            ),
            Ok(slugs) if slugs.has_id(id) => BuildOffer::new(
                Offered::Yes,
                format!("Wago lists a release for {game_version}"),
            ),
            Ok(_) => BuildOffer::new(
                Offered::No,
                format!("Wago lists no release for {game_version}"),
            ),
            Err(error) => BuildOffer::new(Offered::Unknown, error.to_string()),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::flavor::BuildVersion;

    fn release(stability: &'static str, created_at: Option<&str>) -> WagoRelease {
        WagoRelease {
            stability,
            label: format!("1.0.0-{stability}"),
            download_link: format!("https://addons.wago.io/x/{stability}"),
            created_at: created_at.map(str::to_string),
            patches: Vec::new(),
            changelog: None,
        }
    }

    fn addon(releases: Vec<WagoRelease>) -> WagoAddon {
        WagoAddon {
            releases,
            ..WagoAddon::default()
        }
    }

    fn picked(addon: &WagoAddon, channel: Channel) -> Option<&'static str> {
        release_on(addon, channel).map(|release| release.stability)
    }

    #[test]
    fn the_channels_take_the_three_stabilities_in_their_order() {
        // All three, the stable one the oldest: the stable channel keeps it, the prerelease one the newest.
        let three = addon(vec![
            release("stable", Some("2026-10-01T10:00:00Z")),
            release("beta", Some("2026-10-10T10:00:00Z")),
            release("alpha", Some("2026-10-05T10:00:00Z")),
        ]);
        assert_eq!(picked(&three, Channel::Stable), Some("stable"));
        assert_eq!(picked(&three, Channel::Prerelease), Some("beta"));
        // The stable one the newest: both channels take it.
        let newest_stable = addon(vec![
            release("stable", Some("2026-10-20T10:00:00Z")),
            release("beta", Some("2026-10-10T10:00:00Z")),
            release("alpha", Some("2026-10-15T10:00:00Z")),
        ]);
        assert_eq!(picked(&newest_stable, Channel::Prerelease), Some("stable"));
        // No stable release: the stable channel takes the newest of the others, as GitHub's prerelease rule.
        let unstable = addon(vec![
            release("beta", Some("2026-10-10T10:00:00Z")),
            release("alpha", Some("2026-10-12T10:00:00Z")),
        ]);
        assert_eq!(picked(&unstable, Channel::Stable), Some("alpha"));
        assert_eq!(picked(&unstable, Channel::Prerelease), Some("alpha"));
        // A tie, or no dates: the stabler one.
        let undated = addon(vec![release("beta", None), release("alpha", None)]);
        assert_eq!(picked(&undated, Channel::Prerelease), Some("beta"));
        assert_eq!(picked(&addon(Vec::new()), Channel::Stable), None);
    }

    #[test]
    fn the_game_version_follows_the_flavor_and_the_build() {
        let of = |flavor: FlavorId, version: Option<&str>| {
            game_version(&Target {
                version: version.and_then(BuildVersion::parse),
                ..Target::new(flavor)
            })
        };
        assert_eq!(of(FlavorId::Retail, None), Some(("retail", false)));
        assert_eq!(of(FlavorId::ClassicEra, None), Some(("classic", false)));
        assert_eq!(
            of(FlavorId::Forever, Some("1.60.1.70245")),
            Some(("forever", false))
        );
        assert_eq!(
            of(FlavorId::Classic, Some("5.5.4.70531")),
            Some(("mop", false))
        );
        assert_eq!(
            of(FlavorId::Classic, Some("4.4.2.1")),
            Some(("cata", false))
        );
        assert_eq!(
            of(FlavorId::Classic, Some("3.4.5.1")),
            Some(("wotlk", false))
        );
        assert_eq!(
            of(FlavorId::Classic, Some("3.80.2.1")),
            Some(("titan", false))
        );
        assert_eq!(of(FlavorId::Classic, Some("2.5.6.1")), Some(("bc", false)));
        assert_eq!(of(FlavorId::Classic, None), Some(("mop", true)));
        assert_eq!(of(FlavorId::Other("wow_x".into()), None), None);
    }

    #[test]
    fn an_answer_reads_its_releases_and_patches() {
        let raw = serde_json::json!({
            "data": {
                "id": "abc", "slug": "an-addon", "display_name": "An Addon",
                "recent_release": {
                    "stable": { "label": "v1.0.0", "download_link": "https://addons.wago.io/d/1",
                                "created_at": "2026-10-01T10:00:00Z",
                                "supported_forever_patches": ["1.60.0", "1.60.1"] },
                    "beta": { "label": "1.1.0-beta", "download_link": "https://addons.wago.io/d/2",
                              "supported_patches": "1.60.1" },
                    "alpha": { "label": "", "download_link": "https://addons.wago.io/d/3" }
                }
            }
        });
        let addon = parse_addon(&raw, "forever").expect("an addon");
        assert_eq!(addon.slug.as_deref(), Some("an-addon"));
        assert_eq!(
            addon.releases.len(),
            2,
            "a release without a label is left out"
        );
        assert_eq!(addon.releases[0].interface(), Some(16001));
        assert_eq!(addon.releases[1].patches, ["1.60.1"]);
        assert_eq!(
            parse_addon(&serde_json::json!({"recent_release": null}), "retail")
                .expect("an addon")
                .releases,
            Vec::<WagoRelease>::new()
        );
        assert!(parse_addon(&serde_json::json!([]), "retail").is_err());
    }

    #[test]
    fn a_bulk_answer_reads_by_id_in_either_shape_and_keeps_the_ids_asked() {
        let one = serde_json::json!({ "id": "abc", "recent_release": {} });
        let keyed = serde_json::json!({
            "addons": { "abc": one, "def": { "recent_release": {} }, "other": { "id": "zzz" }, "bad": 7 }
        });
        let read = parse_recents(&keyed, &["abc", "def", "missing"]);
        assert_eq!(
            read.keys().map(String::as_str).collect::<Vec<_>>(),
            ["abc", "def"],
            "the key names an addon without an id; an id not asked is left out"
        );
        assert_eq!(read["abc"], one);
        let listed = serde_json::json!({ "data": [one, { "id": "def" }, "text"] });
        assert_eq!(
            parse_recents(&listed, &["abc"])
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["abc"]
        );
        assert!(parse_recents(&serde_json::json!("nothing"), &["abc"]).is_empty());
        assert_eq!(interface_of_patch("12.1.5"), Some(120105));
        assert_eq!(interface_of_patch("3.80.2"), Some(38002));
        assert_eq!(interface_of_patch("x"), None);
    }
}
