//! Addon sources, in the order D-002 names them: GitHub releases with `release.json` (task 003, [`github`]),
//! then Wago, WoWInterface and CurseForge with the user's own key (M4 Sources), and a zip in hand. A source
//! answers two questions, "which release is current for this addon" and "which asset is the one for this
//! flavor", and never guesses "latest" without saying so: a [`Pick`] carries `guess` and the reason. The
//! network sits behind the [`Http`] trait: [`UreqHttp`] is the real client, [`FolderHttp`] answers from a
//! folder of fixture files, so every test runs offline (`MRGL_HTTP_STUB` points the command line at one).
//!
//! A spec names one of four sources ([`SourceSpec`], task 092): `github:<owner>/<repo>`, `wago:<id>`,
//! `wowi:<id>` and `curse:<id>`, or a page address an id can be read from without a request. The callers
//! never build a source themselves: [`Sources`] (the router of [`router`]) holds the sources this build has
//! and this run can use, and answers for every spec by handing it to its source, or with
//! [`Error::SourceNotConfigured`] naming what is missing.

pub mod github;
pub mod router;
pub mod wowinterface;

pub use router::Sources;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::flavor::{BuildVersion, Flavor, FlavorId};
use crate::forever::ForeverBuild;

/// The environment variable that names a folder the command line answers every request from instead of
/// the network (the end-to-end tests set it); see [`FolderHttp`].
pub const HTTP_STUB_ENV: &str = "MRGL_HTTP_STUB";

/// The hint of a spec that names no source mrgl reads: the four spec forms.
pub const SPEC_FORMS: &str = "expected github:<owner>/<repo>, wago:<id>, wowi:<id> or curse:<id>";

/// The sources a spec can name, in the order of D-002 after GitHub (task 092).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceKind {
    /// GitHub releases, `github:<owner>/<repo>`.
    GitHub,
    /// Wago Addons, `wago:<id>`.
    Wago,
    /// WoWInterface, `wowi:<id>`.
    WowInterface,
    /// CurseForge, `curse:<id>`.
    CurseForge,
}

impl SourceKind {
    /// Every source, in the order above.
    pub const ALL: [SourceKind; 4] = [
        SourceKind::GitHub,
        SourceKind::Wago,
        SourceKind::WowInterface,
        SourceKind::CurseForge,
    ];

    /// The spec prefix without its colon, which is also the source's short name on the command line
    /// (`mrgl key set wago`): `github`, `wago`, `wowi`, `curse`.
    pub fn prefix(self) -> &'static str {
        match self {
            SourceKind::GitHub => "github",
            SourceKind::Wago => "wago",
            SourceKind::WowInterface => "wowi",
            SourceKind::CurseForge => "curse",
        }
    }

    /// The name a message line uses: `GitHub`, `Wago`, `WoWInterface`, `CurseForge`.
    pub fn name(self) -> &'static str {
        match self {
            SourceKind::GitHub => "GitHub",
            SourceKind::Wago => "Wago",
            SourceKind::WowInterface => "WoWInterface",
            SourceKind::CurseForge => "CurseForge",
        }
    }

    /// A source by its prefix or its name, ASCII case-insensitive (`wago`, `wowi`, `WoWInterface`,
    /// `curseforge`).
    pub fn parse(text: &str) -> Option<SourceKind> {
        let text = text.trim();
        SourceKind::ALL.into_iter().find(|kind| {
            text.eq_ignore_ascii_case(kind.prefix()) || text.eq_ignore_ascii_case(kind.name())
        })
    }

    /// Its place in [`SourceKind::ALL`].
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for SourceKind {
    /// The prefix, as a spec and the command line spell it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix())
    }
}

impl Serialize for SourceKind {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.prefix())
    }
}

/// Where an addon comes from, parsed from a spec on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSpec {
    /// `github:<owner>/<repo>`: the repository's releases.
    GitHub { owner: String, repo: String },
    /// `wago:<id>`: a Wago Addons project, by its id of letters and digits (`X-Wago-ID` in a TOC).
    Wago { id: String },
    /// `wowi:<id>`: a WoWInterface project, by its number (`X-WoWI-ID` in a TOC).
    WowInterface { id: String },
    /// `curse:<id>`: a CurseForge project, by its number (`X-Curse-Project-ID` in a TOC).
    CurseForge { id: String },
}

/// The hint of a `github:` spec that is not `<owner>/<repo>`.
const GITHUB_FORM: &str = "expected github:<owner>/<repo>";
/// The hint of a `wago:` spec without an id.
const WAGO_FORM: &str =
    "expected wago:<id>, the addon's id of letters and digits (the X-Wago-ID line of its TOC)";
/// The hint of a `wowi:` spec or a WoWInterface address without an id.
const WOWI_FORM: &str = "expected wowi:<id>, the number of the addon's page address (www.wowinterface.com/downloads/info<id>)";
/// The hint of a `curse:` spec without an id.
const CURSE_FORM: &str = "expected curse:<id>, the addon's project id, a number (the X-Curse-Project-ID line of its TOC)";
/// The hint of a Wago page address: it names the addon by its slug, which is not the id.
const WAGO_PAGE: &str = "a Wago page names the addon by its slug, not its id: use wago:<id>, the X-Wago-ID line of the addon's TOC";
/// The hint of a CurseForge page address with a slug.
const CURSE_PAGE: &str = "a CurseForge page names the addon by its slug, not its id: use curse:<id>, the project id on the page or the X-Curse-Project-ID line of the addon's TOC";

impl SourceSpec {
    /// Parses a spec (task 092): `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>` or `curse:<id>`, or a
    /// page address that carries the id: `https://github.com/<owner>/<repo>` (with or without `.git` and a
    /// trailing slash), `https://www.wowinterface.com/downloads/info<id>[-<name>.html]` and
    /// `https://www.curseforge.com/projects/<id>`. A Wago page (`addons.wago.io/addons/<slug>`) and a
    /// CurseForge addon page (`curseforge.com/wow/addons/<slug>`) name a slug, which is not the id, and are
    /// refused with a hint that says where the id is. Anything else is an error that names the accepted
    /// shape.
    pub fn parse(text: &str) -> Result<SourceSpec> {
        let text = text.trim();
        let error = |hint: &'static str| Error::UnknownSource {
            spec: text.to_string(),
            hint,
        };
        if let Some(path) = text.strip_prefix("github:") {
            return github_path(path).ok_or_else(|| error(GITHUB_FORM));
        }
        if let Some(id) = text.strip_prefix("wago:") {
            return wago_id(id)
                .map(|id| SourceSpec::Wago { id })
                .ok_or_else(|| error(WAGO_FORM));
        }
        if let Some(id) = text.strip_prefix("wowi:") {
            return number_id(id)
                .map(|id| SourceSpec::WowInterface { id })
                .ok_or_else(|| error(WOWI_FORM));
        }
        if let Some(id) = text.strip_prefix("curse:") {
            return number_id(id)
                .map(|id| SourceSpec::CurseForge { id })
                .ok_or_else(|| error(CURSE_FORM));
        }
        let Some(address) = text
            .strip_prefix("https://")
            .or_else(|| text.strip_prefix("http://"))
        else {
            return Err(error(SPEC_FORMS));
        };
        // The query and the fragment never carry the id of a form read here.
        let address = address
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        let (host, path) = address.split_once('/').unwrap_or((address, ""));
        let host = host.to_ascii_lowercase();
        let host = host.strip_prefix("www.").unwrap_or(&host);
        match host {
            "github.com" => github_path(path).ok_or_else(|| error(GITHUB_FORM)),
            "wowinterface.com" => path
                .strip_prefix("downloads/info")
                .map(|rest| {
                    let rest = rest.strip_suffix(".html").unwrap_or(rest);
                    rest.split_once('-').map_or(rest, |(id, _)| id)
                })
                .and_then(number_id)
                .map(|id| SourceSpec::WowInterface { id })
                .ok_or_else(|| error(WOWI_FORM)),
            "curseforge.com" => match path.strip_prefix("projects/") {
                Some(id) => number_id(id)
                    .map(|id| SourceSpec::CurseForge { id })
                    .ok_or_else(|| error(CURSE_FORM)),
                None => Err(error(CURSE_PAGE)),
            },
            "addons.wago.io" => Err(error(WAGO_PAGE)),
            _ => Err(error(SPEC_FORMS)),
        }
    }

    /// The source the spec names.
    pub fn kind(&self) -> SourceKind {
        match self {
            SourceSpec::GitHub { .. } => SourceKind::GitHub,
            SourceSpec::Wago { .. } => SourceKind::Wago,
            SourceSpec::WowInterface { .. } => SourceKind::WowInterface,
            SourceSpec::CurseForge { .. } => SourceKind::CurseForge,
        }
    }

    /// The name the spec suggests for the addon's folder (task 065, `install::name_hints`): the repository
    /// of a GitHub spec; an id of the other sources is no name.
    pub fn name_hint(&self) -> Option<&str> {
        match self {
            SourceSpec::GitHub { repo, .. } => Some(repo),
            SourceSpec::Wago { .. }
            | SourceSpec::WowInterface { .. }
            | SourceSpec::CurseForge { .. } => None,
        }
    }
}

/// `<owner>/<repo>` of a GitHub spec or address, without `.git` and the slashes around it.
fn github_path(path: &str) -> Option<SourceSpec> {
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    let is_name = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    (is_name(owner) && is_name(repo)).then(|| SourceSpec::GitHub {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

/// A Wago id: ASCII letters and digits as written (the ids are case-sensitive, `L6Jb2R6v`), at most 32.
fn wago_id(id: &str) -> Option<String> {
    (!id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| id.to_string())
}

/// A WoWInterface or CurseForge id: a number above zero, written without leading zeros.
fn number_id(id: &str) -> Option<String> {
    if id.is_empty() || id.len() > 12 || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let number: u64 = id.parse().ok()?;
    (number > 0).then(|| number.to_string())
}

impl fmt::Display for SourceSpec {
    /// The canonical spelling: `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>`, `curse:<id>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceSpec::GitHub { owner, repo } => write!(f, "github:{owner}/{repo}"),
            SourceSpec::Wago { id } => write!(f, "wago:{id}"),
            SourceSpec::WowInterface { id } => write!(f, "wowi:{id}"),
            SourceSpec::CurseForge { id } => write!(f, "curse:{id}"),
        }
    }
}

impl Serialize for SourceSpec {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// The release channel an addon follows (task 021): `stable` takes the newest release that is not a
/// prerelease (a prerelease only when nothing else exists), `prerelease` takes the newest release whether
/// it is a prerelease or not, so one addon can follow the beta builds. The GitHub `prerelease` flag
/// decides what a prerelease is. The sidecar records it per addon; absent means `stable`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Prerelease,
}

impl Channel {
    /// The channel's name as the sidecar, the JSON and the command line spell it.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Prerelease => "prerelease",
        }
    }

    /// Reads a channel name, ASCII case-insensitive: `stable`, `prerelease` (also `beta`).
    pub fn parse(text: &str) -> Option<Channel> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("stable") {
            Some(Channel::Stable)
        } else if text.eq_ignore_ascii_case("prerelease") || text.eq_ignore_ascii_case("beta") {
            Some(Channel::Prerelease)
        } else {
            None
        }
    }

    /// Whether this is the default channel (the sidecar leaves it out).
    pub fn is_stable(&self) -> bool {
        *self == Channel::Stable
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a release is picked for: the flavor, the installed build when it is known, and the channel. The
/// build decides between the game types of the progressing classic (Cataclysm, Mists, …), which share one
/// flavor id; the channel decides whether a prerelease can be the current release (task 021).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub flavor: FlavorId,
    pub version: Option<BuildVersion>,
    pub channel: Channel,
}

impl Target {
    /// A target with the flavor alone; the build is unknown, the channel stable.
    pub fn new(flavor: FlavorId) -> Target {
        Target {
            flavor,
            version: None,
            channel: Channel::Stable,
        }
    }

    /// The target of a flavor folder read from disk, on the stable channel.
    pub fn from_flavor(flavor: &Flavor) -> Target {
        Target {
            flavor: flavor.id.clone(),
            version: flavor.version,
            channel: Channel::Stable,
        }
    }

    /// The same target on another channel: an addon's own, from its sidecar entry.
    pub fn with_channel(mut self, channel: Channel) -> Target {
        self.channel = channel;
        self
    }

    /// The major version of the installed build, when known (`5` for Mists of Pandaria Classic).
    pub fn major(&self) -> Option<u32> {
        self.version.map(|version| version.major)
    }

    /// The name the user knows the flavor by: the friendly name, or the id for an unknown flavor.
    pub fn flavor_name(&self) -> &str {
        self.flavor
            .friendly_name()
            .unwrap_or_else(|| self.flavor.as_str())
    }
}

/// The resolved release asset for one flavor: the handover to the install engine (task 004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pick {
    /// The source, canonical (`github:<owner>/<repo>`).
    pub source: String,
    /// The release as the source names it: the tag.
    pub release: String,
    /// The version the release carries: the tag without its leading `v`.
    pub version: String,
    /// When the release was published, as the source writes it (RFC 3339), when it says.
    pub published: Option<String>,
    /// The release is marked a prerelease (alpha, beta); picked on the `prerelease` channel, or on the
    /// stable one when no other release exists.
    pub prerelease: bool,
    /// The asset's file name.
    pub asset: String,
    /// The asset's download URL.
    pub url: String,
    /// The asset's size in bytes, when the source says.
    pub size: Option<u64>,
    /// The asset's MD5 as the source names it, lower-case hex (WoWInterface's `UIMD5`, task 094): the
    /// download is refused before it is written when its own MD5 differs ([`check_md5`]); `None`, and
    /// left out of the JSON, when the source names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    /// The flavor key the pick rests on: the `release.json` flavor (`forever`, `mainline`, `mists`, …) or
    /// the asset-name suffix the heuristics matched.
    pub flavor: String,
    /// The interface number `release.json` names for the asset; `None` for a pick by name.
    pub interface: Option<u32>,
    /// The pick is a guess: no metadata named the flavor, or the installed build is unknown, so the asset
    /// may be the wrong one. The reason says which.
    pub guess: bool,
    /// Why this asset, in plain English, for the message and the sidecar.
    pub reason: String,
    /// The release notes in one line: the first line of the release body with something in it, by
    /// [`notes_line`]; `None` (and left out of the JSON) when the body is missing or says nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// The release's page, where the full notes are (`html_url` on GitHub), when the source names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// The asset is a `nolib` build (`release.json`'s flag, or a `-nolib` name): its libraries are
    /// separate addons. The install records it on the sidecar entry for the dependency check (task 029).
    pub nolib: bool,
    /// Whether the release publishes a `forever` build at all, whatever the target (task 011).
    pub forever_build: ForeverBuild,
    /// The pick comes from the cached release list because the source could not be reached (task 041):
    /// when the cache was last confirmed and the network error; `None` (`null` in the JSON) for a pick the
    /// source answered.
    pub stale: Option<Stale>,
}

/// Why a pick may be stale: it was resolved from the cache, offline (task 041).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stale {
    /// When the source last confirmed the cached release list, RFC 3339 in UTC; `None` when the cache does
    /// not say (written by an earlier build).
    pub checked_at: Option<String>,
    /// The network error that sent the resolve to the cache, as its plain line.
    pub reason: String,
}

impl Pick {
    /// The warning line of a pick resolved from the cache (task 041), `None` for a fresh one: the cache's
    /// date in local time and the network error.
    pub fn stale_warning(&self) -> Option<String> {
        let stale = self.stale.as_ref()?;
        let cached = match stale.checked_at.as_deref().and_then(unix_from_rfc3339) {
            Some(seconds) => format!("cached {}", local_time_text(seconds)),
            None => "cached at an unknown time".to_string(),
        };
        Some(format!(
            "offline: {} {} is from the release list {cached}, the pick may be stale ({})",
            self.source, self.release, stale.reason
        ))
    }

    /// The one-line summary of the pick, for the success or warning line.
    pub fn summary(&self) -> String {
        let interface = self
            .interface
            .map(|interface| format!(", interface {interface}"))
            .unwrap_or_default();
        let prerelease = if self.prerelease {
            ", a prerelease"
        } else {
            ""
        };
        format!(
            "{} {} ({}, flavor {}{interface}{prerelease}): {}",
            self.source, self.release, self.asset, self.flavor, self.reason
        )
    }
}

/// The MD5 of some bytes, lower-case hex (task 094).
pub fn md5_hex(bytes: &[u8]) -> String {
    use md5::{Digest, Md5};
    Md5::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The download of a pick against the MD5 its source names (task 094), before anything writes or opens it:
/// [`Error::AssetChecksumMismatch`] when they differ, nothing to check when the pick names none.
pub fn check_md5(pick: &Pick, body: &[u8]) -> Result<()> {
    let Some(expected) = pick.md5.as_deref() else {
        return Ok(());
    };
    let got = md5_hex(body);
    if got.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    Err(Error::AssetChecksumMismatch {
        spec: pick.source.clone(),
        release: pick.release.clone(),
        asset: pick.asset.clone(),
        expected: expected.to_ascii_lowercase(),
        got,
    })
}

/// The version a release name carries (tasks 003, 094): the name without a leading `v` or `V` when a digit
/// follows (`v1.2.3` is `1.2.3`, `1.2.3` and `version-1` stay).
pub fn version_of_tag(tag: &str) -> &str {
    match tag.strip_prefix('v').or_else(|| tag.strip_prefix('V')) {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => rest,
        _ => tag,
    }
}

/// Why a source says what it says of the Forever build (tasks 011, 094), for the `forever build:` line: the
/// GitHub source reads `release.json`, WoWInterface its compatibility list.
pub fn forever_why(kind: SourceKind, build: ForeverBuild) -> &'static str {
    match (kind, build) {
        (SourceKind::WowInterface, ForeverBuild::Yes) => {
            "WoWInterface lists a Forever compatibility"
        }
        (SourceKind::WowInterface, _) => "WoWInterface lists no Forever compatibility",
        (_, ForeverBuild::Yes) => "release.json names forever",
        (_, ForeverBuild::No) => "release.json names no forever flavor",
        (_, ForeverBuild::Unknown) => "no release.json says",
    }
}

/// A source: resolves a repository and a flavor to a release pick, and fetches the picked asset.
pub trait Source {
    /// The current release's asset for the target, or why there is none.
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick>;

    /// The asset for the target of the release that carries a version (task 057: `import --exact`), on
    /// any channel, or [`Error::VersionNotOffered`] when the source has no such release. A source that lists
    /// its releases looks the version up there; this default takes the current release when it carries
    /// that version, for a source that knows only its current one.
    fn resolve_version(&self, spec: &SourceSpec, target: &Target, version: &str) -> Result<Pick> {
        let pick = self.resolve(spec, target)?;
        if pick.version == version {
            Ok(pick)
        } else {
            Err(Error::VersionNotOffered {
                spec: spec.to_string(),
                version: version.to_string(),
            })
        }
    }

    /// Downloads the picked asset to a path, creating the parent folders.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()>;

    /// The versions the source lists for a spec, newest first, as its last answer had them and without a
    /// request (task 068: the update check's version order); `None` when the source keeps no such list.
    fn listed_versions(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        let _ = spec;
        None
    }

    /// What is left of a request limit this run has to keep within (task 068: the GitHub API without a
    /// token); `None` when the source has no limit to watch.
    fn request_budget(&self) -> Option<RequestBudget> {
        None
    }

    /// The request limit asking about a spec counts against (task 092: the router answers with the budget
    /// of the spec's source); this default is [`Source::request_budget`].
    fn request_budget_for(&self, spec: &SourceSpec) -> Option<RequestBudget> {
        let _ = spec;
        self.request_budget()
    }

    /// What this run lacks to ask about a spec, as the clause of the "source not configured" line
    /// ([`Error::SourceNotConfigured`]): a source this build does not have, a key that is not set; `None`
    /// when the source can be asked (task 092). The default asks anything.
    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        let _ = spec;
        None
    }

    /// Whether the addon publishes a Forever build, from what the source keeps without a request (tasks
    /// 011, 078: `list` and the window stay offline); `unknown` wherever the source does not name Forever,
    /// never inferred (task 092), which is this default.
    fn cached_forever_build(&self, spec: &SourceSpec, channel: Channel) -> ForeverBuild {
        let _ = (spec, channel);
        ForeverBuild::Unknown
    }
}

/// The error of a spec a source cannot ask about: [`Error::SourceNotConfigured`] with the source's
/// [`Source::missing`] clause, `None` when it can.
pub fn not_configured<S: Source + ?Sized>(source: &S, spec: &SourceSpec) -> Option<Error> {
    source
        .missing(spec)
        .map(|missing| Error::SourceNotConfigured {
            spec: spec.to_string(),
            missing,
        })
}

/// A source's request limit as this run sees it (task 068): the requests left before the source refuses,
/// and when the limit resets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestBudget {
    /// The host whose limit it is (`GitHub`).
    pub host: &'static str,
    /// The requests the limit allows in its window (60 an hour for the GitHub API without a token).
    pub limit: u32,
    /// The requests this run has sent against the limit.
    pub sent: u32,
    /// The requests left: the last answer's count less the requests since, or, before any answer said,
    /// the limit less the requests sent.
    pub remaining: u32,
    /// When the limit resets, in Unix seconds, when an answer said.
    pub reset: Option<i64>,
    /// How to get a larger limit, for the message line.
    pub hint: String,
}

/// One HTTP request, as a source issues it: a GET, or a POST with its body ([`Http::post`], task 092).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    /// Header name and value; the names are sent as given.
    pub headers: Vec<(String, String)>,
    /// The body of a POST; `None` for a GET.
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Request {
        Request {
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// A POST with its body (a JSON document for the APIs that take one; the caller sets `Content-Type`).
    pub fn post(url: impl Into<String>, body: impl Into<Vec<u8>>) -> Request {
        Request {
            url: url.into(),
            headers: Vec::new(),
            body: Some(body.into()),
        }
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Request {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    /// A request header's value, ASCII case-insensitive on the name.
    pub fn header_value(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, name)
    }
}

/// The answer to a [`Request`]: the status, the headers and the whole body (an addon zip is a few megabytes
/// at most; the release list and a `release.json` are kilobytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// A response header's value, ASCII case-insensitive on the name.
    pub fn header(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, name)
    }
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// The network behind a source. A GET and a POST, so a stub is a few lines and a test never opens a socket.
pub trait Http {
    /// Sends a GET and returns whatever came back; only a transport failure is an error, a status is the
    /// caller's to read.
    fn get(&self, request: &Request) -> Result<Response>;

    /// Sends a POST with the request's body (task 092: the bulk and match lookups of the M4 sources), and
    /// returns whatever came back like [`Http::get`]. This default sends nothing and fails as the network
    /// does, for a stand-in that only ever answers GETs.
    fn post(&self, request: &Request) -> Result<Response> {
        Err(Error::Network {
            url: request.url.clone(),
            reason: "this client sends no POST".to_string(),
        })
    }
}

/// The real client: `ureq` over rustls, with mrgl's user agent, a connect and a response timeout, and the
/// HTTP status always returned rather than raised. Redirects are followed (an asset download hops to the
/// object store) and an `Authorization` header never follows one.
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> UreqHttp {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .user_agent(format!("mrgl/{}", crate::VERSION))
            .timeout_connect(Some(std::time::Duration::from_secs(20)))
            .timeout_recv_response(Some(std::time::Duration::from_secs(30)))
            .build();
        UreqHttp {
            agent: config.into(),
        }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

/// The error of a request that got no answer, or whose body could not be read.
fn network_error(request: &Request, reason: String) -> Error {
    Error::Network {
        url: request.url.clone(),
        reason,
    }
}

/// The answer `ureq` read, as a [`Response`]: the status, the headers and the whole body.
fn ureq_response(
    request: &Request,
    answer: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Response> {
    let mut response = answer.map_err(|error| network_error(request, error.to_string()))?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .body_mut()
        .with_config()
        // An addon zip is megabytes; a limit keeps a runaway answer from eating the memory.
        .limit(512 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| network_error(request, format!("reading the body: {error}")))?;
    Ok(Response {
        status,
        headers,
        body,
    })
}

impl Http for UreqHttp {
    fn get(&self, request: &Request) -> Result<Response> {
        let mut call = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        ureq_response(request, call.call())
    }

    fn post(&self, request: &Request) -> Result<Response> {
        let mut call = self.agent.post(&request.url);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        let body: &[u8] = request.body.as_deref().unwrap_or_default();
        ureq_response(request, call.send(body))
    }
}

/// The offline stand-in: a folder of files, one per URL. `https://api.github.com/repos/o/r/releases?per_page=30`
/// is answered from `<root>/api.github.com/repos/o/r/releases` or, when that is missing, the same path
/// with `.json`; a path with no file is a 404 with an empty body. The query string picks a file of its own
/// first (task 092): `?game_version=retail&id=7` is answered from `<path>@game_version=retail&id=7` (or with
/// `.json`), the pairs in the order of their names, a `page` left out and a character Windows refuses in a
/// file name (`<>:"/\|?*`) written as `_`, so two answers of one path that differ only in the query come
/// from two files; without such a file the query is ignored, as it always was. A `page` above 1 answers
/// from the same name with `.page-<n>` appended (`releases.page-2.json`, task 042). Every answer carries an
/// `ETag` derived from the file's content, and a request whose `If-None-Match` names that tag gets a 304
/// without a body, so the cache is testable too. A file `<answer>.headers` next to an answer adds its
/// `Name: value` lines to the answer's headers, and its `Status: <code>` line sets the status (a `Link` to
/// the next page, a 403 with the rate-limit headers; task 042). A POST is answered like a GET of its URL,
/// from the same files (task 092). The requests are kept for the tests to read, a POST with its body.
pub struct FolderHttp {
    root: PathBuf,
    requests: Mutex<Vec<Request>>,
}

impl FolderHttp {
    pub fn new(root: impl Into<PathBuf>) -> FolderHttp {
        FolderHttp {
            root: root.into(),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// The folder the answers come from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The requests made so far, oldest first.
    pub fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }

    /// The file a URL maps to: the host and the path under the root, the scheme and the query dropped.
    pub fn path_for(&self, url: &str) -> PathBuf {
        let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
        let without_query = without_scheme
            .split_once('?')
            .map(|(path, _)| path)
            .unwrap_or(without_scheme);
        let mut path = self.root.clone();
        for part in without_query
            .split('/')
            .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        {
            path.push(part);
        }
        path
    }

    /// The name part a URL's query adds to its file (task 092): `@` and the pairs sorted by name, `page`
    /// left out (the pages have their own suffix) and every character Windows refuses in a file name made
    /// `_`; `None` for a URL without a query, or with `page` alone.
    pub fn query_suffix(url: &str) -> Option<String> {
        let (_, query) = url.split_once('?')?;
        let query = query.split('#').next().unwrap_or_default();
        let mut pairs: Vec<&str> = query
            .split('&')
            .filter(|pair| {
                !pair.is_empty() && pair.split_once('=').map_or(*pair, |(key, _)| key) != "page"
            })
            .collect();
        if pairs.is_empty() {
            return None;
        }
        pairs.sort_by_key(|pair| pair.split_once('=').map_or(*pair, |(key, _)| key));
        let joined: String = pairs
            .join("&")
            .chars()
            .map(|c| {
                if c.is_control() || "<>:\"/\\|?*".contains(c) {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        Some(format!("@{joined}"))
    }

    /// The answer file of a URL: the one of its query when it exists ([`FolderHttp::query_suffix`]), else
    /// the one of its path alone; each with the page suffix, the bare name first and then with `.json`.
    /// Nothing found is the path's `.json` name, which reads as a 404.
    fn answer_file(&self, url: &str) -> PathBuf {
        let path = self.path_for(url);
        let page = page_of(url)
            .filter(|page| *page > 1)
            .map(|page| format!(".page-{page}"))
            .unwrap_or_default();
        let named = |query: &str, json: &str| {
            let mut name = path.clone().into_os_string();
            name.push(format!("{query}{page}{json}"));
            PathBuf::from(name)
        };
        let mut candidates = Vec::new();
        if let Some(query) = Self::query_suffix(url) {
            candidates.push(named(&query, ""));
            candidates.push(named(&query, ".json"));
        }
        candidates.push(named("", ""));
        let missing = named("", ".json");
        candidates
            .into_iter()
            .find(|candidate| candidate.is_file())
            .unwrap_or(missing)
    }

    /// The tag of a body: its length and an FNV-1a hash, quoted as an entity tag.
    pub fn etag_of(body: &[u8]) -> String {
        let hash = body.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
        format!("\"{}-{hash:016x}\"", body.len())
    }

    /// The answer to a GET or a POST, from the files; the request is kept first.
    fn answer(&self, request: &Request) -> Result<Response> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        // No folder at all is no network: the offline resolve of task 041 is tested this way.
        if !self.root.is_dir() {
            return Err(Error::Network {
                url: request.url.clone(),
                reason: "offline, the stub folder is missing".to_string(),
            });
        }
        let file = self.answer_file(&request.url);
        let body = match std::fs::read(&file) {
            Ok(body) => body,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Response {
                    status: 404,
                    headers: Vec::new(),
                    body: Vec::new(),
                });
            }
            Err(source) => {
                return Err(Error::Io {
                    context: "reading the stub answer",
                    path: file,
                    source,
                });
            }
        };
        let etag = Self::etag_of(&body);
        let mut status = 200;
        let mut headers = vec![("ETag".to_string(), etag.clone())];
        let mut sidecar = file.into_os_string();
        sidecar.push(".headers");
        if let Ok(text) = std::fs::read_to_string(PathBuf::from(sidecar)) {
            for (name, value) in text.lines().filter_map(|line| line.split_once(':')) {
                let (name, value) = (name.trim(), value.trim());
                if name.eq_ignore_ascii_case("status") {
                    status = value.parse().unwrap_or(status);
                } else if !name.is_empty() {
                    headers.push((name.to_string(), value.to_string()));
                }
            }
        }
        if status == 200 && request.header_value("If-None-Match") == Some(etag.as_str()) {
            return Ok(Response {
                status: 304,
                headers,
                body: Vec::new(),
            });
        }
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

impl Http for FolderHttp {
    fn get(&self, request: &Request) -> Result<Response> {
        self.answer(request)
    }

    fn post(&self, request: &Request) -> Result<Response> {
        self.answer(request)
    }
}

/// The `page` of a URL's query string, when it names one (`?per_page=30&page=2` is 2).
fn page_of(url: &str) -> Option<u32> {
    let (_, query) = url.split_once('?')?;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == "page")
        .and_then(|(_, value)| value.parse().ok())
}

/// The Unix seconds of an RFC 3339 time in UTC as mrgl writes it (`2026-10-21T14:03:05Z`); `None` for any
/// other shape.
pub fn unix_from_rfc3339(text: &str) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut date = date.split('-').map(str::parse::<u16>);
    let mut time = time.split(':').map(str::parse::<u8>);
    let civil = crate::backup::clock::Civil {
        year: date.next()?.ok()?,
        month: u8::try_from(date.next()?.ok()?).ok()?,
        day: u8::try_from(date.next()?.ok()?).ok()?,
        hour: time.next()?.ok()?,
        minute: time.next()?.ok()?,
        second: time.next()?.ok()?,
    };
    let valid = (1..=12).contains(&civil.month)
        && (1..=31).contains(&civil.day)
        && civil.hour < 24
        && civil.minute < 60
        && civil.second < 60
        && date.next().is_none()
        && time.next().is_none();
    valid.then(|| crate::backup::clock::unix_from_civil(&civil))
}

/// Unix seconds as the local time of this machine to the minute, `2026-10-22 14:05`, for a message line
/// (the rate limit's reset, task 042).
pub fn local_time_text(seconds: i64) -> String {
    let time =
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds.max(0).unsigned_abs());
    let civil = crate::backup::clock::local_civil(time);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        civil.year, civil.month, civil.day, civil.hour, civil.minute
    )
}

/// A client picked by the environment: the folder stub when `MRGL_HTTP_STUB` names a folder, else the real
/// one. The command line and the window build their sources through this ([`Sources::from_env`]).
pub fn http_from_env() -> Box<dyn Http> {
    match std::env::var_os(HTTP_STUB_ENV).filter(|value| !value.is_empty()) {
        Some(folder) => Box::new(FolderHttp::new(folder)),
        None => Box::new(UreqHttp::new()),
    }
}

impl<H: Http + ?Sized> Http for Box<H> {
    fn get(&self, request: &Request) -> Result<Response> {
        (**self).get(request)
    }

    fn post(&self, request: &Request) -> Result<Response> {
        (**self).post(request)
    }
}

impl<H: Http + ?Sized> Http for &H {
    fn get(&self, request: &Request) -> Result<Response> {
        (**self).get(request)
    }

    fn post(&self, request: &Request) -> Result<Response> {
        (**self).post(request)
    }
}

/// One client shared by the router's sources (task 092).
impl<H: Http + ?Sized> Http for std::sync::Arc<H> {
    fn get(&self, request: &Request) -> Result<Response> {
        (**self).get(request)
    }

    fn post(&self, request: &Request) -> Result<Response> {
        (**self).post(request)
    }
}

/// The longest notes line a pick carries, in characters; a longer line keeps its first
/// `NOTES_MAX_CHARS - 1` characters and ends with `…`.
pub const NOTES_MAX_CHARS: usize = 120;

/// The release notes in one line (task 030): the first line of a release body that has something in it
/// once its Markdown markers are gone. Leading `#`, `*`, `-`, `+` and `>` (headings, emphasis, list items,
/// quotes) and trailing `#` and `*` are stripped with the whitespace around them, so a rule (`---`) or a
/// bare `##` counts as empty; the line is cut at [`NOTES_MAX_CHARS`] with an ellipsis. `None` when no line
/// is left: an empty body, or one of markers and blank lines only.
pub fn notes_line(body: &str) -> Option<String> {
    let line = body
        .lines()
        .map(|line| {
            line.trim_start_matches(|c: char| c.is_whitespace() || "#*-+>".contains(c))
                .trim_end_matches(|c: char| c.is_whitespace() || "#*".contains(c))
        })
        .find(|line| !line.is_empty())?;
    if line.chars().count() <= NOTES_MAX_CHARS {
        return Some(line.to_string());
    }
    let mut cut: String = line.chars().take(NOTES_MAX_CHARS - 1).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    Some(cut)
}

/// An in-memory stand-in for the unit tests: a URL to its answer, every miss a 404.
#[cfg(test)]
pub(crate) struct MapHttp {
    answers: std::collections::HashMap<String, Response>,
    requests: Mutex<Vec<Request>>,
}

#[cfg(test)]
impl MapHttp {
    pub(crate) fn new() -> MapHttp {
        MapHttp {
            answers: std::collections::HashMap::new(),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn answer(mut self, url: &str, status: u16, body: &str) -> MapHttp {
        self.answers.insert(
            url.to_string(),
            Response {
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            },
        );
        self
    }

    pub(crate) fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
impl Http for MapHttp {
    fn get(&self, request: &Request) -> Result<Response> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        Ok(self.answers.get(&request.url).cloned().unwrap_or(Response {
            status: 404,
            headers: Vec::new(),
            body: Vec::new(),
        }))
    }

    fn post(&self, request: &Request) -> Result<Response> {
        self.get(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_spec_is_owner_and_repo() {
        let expected = SourceSpec::GitHub {
            owner: "Questie".into(),
            repo: "Questie".into(),
        };
        assert_eq!(
            SourceSpec::parse("github:Questie/Questie").unwrap(),
            expected
        );
        assert_eq!(
            SourceSpec::parse("https://github.com/Questie/Questie").unwrap(),
            expected
        );
        assert_eq!(
            SourceSpec::parse("https://github.com/Questie/Questie.git/").unwrap(),
            expected
        );
        assert_eq!(expected.to_string(), "github:Questie/Questie");
    }

    #[test]
    fn a_bad_spec_names_the_accepted_shape() {
        for (spec, hint) in [
            ("Questie/Questie", SPEC_FORMS),
            ("github:Questie", GITHUB_FORM),
            ("github:Questie/Questie/releases", GITHUB_FORM),
            ("github:/Questie", GITHUB_FORM),
            ("github:Que stie/Questie", GITHUB_FORM),
            ("bitbucket:abc", SPEC_FORMS),
            ("", SPEC_FORMS),
            ("wago:", WAGO_FORM),
            ("wago:ab-c", WAGO_FORM),
            ("wowi:abc", WOWI_FORM),
            ("wowi:0", WOWI_FORM),
            ("curse:-1", CURSE_FORM),
            ("curse:1234567890123", CURSE_FORM),
            ("https://addons.wago.io/addons/details", WAGO_PAGE),
            ("https://www.curseforge.com/wow/addons/details", CURSE_PAGE),
            (
                "https://www.wowinterface.com/downloads/fileinfo.php",
                WOWI_FORM,
            ),
            ("https://example.com/addon", SPEC_FORMS),
        ] {
            let error = SourceSpec::parse(spec).unwrap_err().to_string();
            assert_eq!(error, format!("unknown source: {spec} ({hint})"), "{spec}");
        }
        assert_eq!(
            SourceSpec::parse("github:Questie").unwrap_err().to_string(),
            "unknown source: github:Questie (expected github:<owner>/<repo>)"
        );
    }

    #[test]
    fn the_other_sources_read_their_ids_and_print_them_canonically() {
        let wowi = || SourceSpec::WowInterface { id: "5108".into() };
        let curse = || SourceSpec::CurseForge { id: "3358".into() };
        let cases = [
            (
                "wago:L6Jb2R6v",
                SourceSpec::Wago {
                    id: "L6Jb2R6v".into(),
                },
                "wago:L6Jb2R6v",
            ),
            (" wowi:5108 ", wowi(), "wowi:5108"),
            ("wowi:005108", wowi(), "wowi:5108"),
            ("curse:3358", curse(), "curse:3358"),
            (
                "https://www.wowinterface.com/downloads/info5108-Clique.html",
                wowi(),
                "wowi:5108",
            ),
            (
                "http://wowinterface.com/downloads/info5108",
                wowi(),
                "wowi:5108",
            ),
            (
                "https://www.wowinterface.com/downloads/info5108.html#info",
                wowi(),
                "wowi:5108",
            ),
            (
                "https://www.curseforge.com/projects/3358/",
                curse(),
                "curse:3358",
            ),
            (
                "https://WWW.GitHub.com/Questie/Questie?tab=readme",
                SourceSpec::GitHub {
                    owner: "Questie".into(),
                    repo: "Questie".into(),
                },
                "github:Questie/Questie",
            ),
        ];
        for (text, spec, canonical) in cases {
            let parsed = SourceSpec::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"));
            assert_eq!(parsed, spec, "{text}");
            assert_eq!(parsed.to_string(), canonical, "{text}");
            assert_eq!(SourceSpec::parse(canonical).unwrap(), parsed, "{canonical}");
            assert_eq!(
                serde_json::to_value(&parsed).unwrap(),
                serde_json::json!(canonical)
            );
        }
    }

    #[test]
    fn a_spec_knows_its_source_and_its_name_hint() {
        let kinds: Vec<(SourceKind, Option<String>)> =
            ["github:o/Repo", "wago:abc", "wowi:1", "curse:2"]
                .into_iter()
                .map(|text| {
                    let spec = SourceSpec::parse(text).unwrap();
                    (spec.kind(), spec.name_hint().map(str::to_string))
                })
                .collect();
        assert_eq!(
            kinds,
            [
                (SourceKind::GitHub, Some("Repo".to_string())),
                (SourceKind::Wago, None),
                (SourceKind::WowInterface, None),
                (SourceKind::CurseForge, None),
            ]
        );
        for kind in SourceKind::ALL {
            assert_eq!(SourceKind::parse(kind.prefix()), Some(kind));
            assert_eq!(SourceKind::parse(&kind.name().to_uppercase()), Some(kind));
            assert_eq!(kind.to_string(), kind.prefix());
            assert_eq!(SourceKind::ALL[kind.index()], kind);
        }
        assert_eq!(
            SourceKind::parse("curseforge"),
            Some(SourceKind::CurseForge)
        );
        assert_eq!(SourceKind::parse("bitbucket"), None);
    }

    #[test]
    fn the_folder_stub_serves_two_queries_of_one_path_from_two_files() {
        let root = std::env::temp_dir().join(format!("mrgl-stub-query-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("api.example.test").join("addons");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("abc@game_version=forever.json"), b"forever").unwrap();
        std::fs::write(dir.join("abc@game_version=retail.json"), b"retail").unwrap();
        std::fs::write(dir.join("abc.json"), b"any").unwrap();
        std::fs::write(dir.join("get.php@d=7&id=5"), b"zip").unwrap();
        std::fs::write(dir.join("list@kind=a_b.json"), b"[1]").unwrap();
        std::fs::write(dir.join("list@kind=a_b.page-2.json"), b"[2]").unwrap();
        let stub = FolderHttp::new(&root);
        let body = |url: &str| {
            let answer = stub.get(&Request::get(url)).unwrap();
            (answer.status, String::from_utf8(answer.body).unwrap())
        };
        let url = "https://api.example.test/addons/abc?game_version=";
        assert_eq!(body(&format!("{url}forever")), (200, "forever".into()));
        assert_eq!(body(&format!("{url}retail")), (200, "retail".into()));
        // A query without a file of its own falls back to the path alone, as before task 092.
        assert_eq!(body(&format!("{url}mop")), (200, "any".into()));
        assert_eq!(
            body("https://api.example.test/addons/abc"),
            (200, "any".into())
        );
        // The pairs are matched in the order of their names.
        assert_eq!(
            body("https://api.example.test/addons/get.php?id=5&d=7"),
            (200, "zip".into())
        );
        // A character a file name cannot carry is `_`, and a page keeps its own suffix.
        assert_eq!(
            body("https://api.example.test/addons/list?kind=a:b&page=1"),
            (200, "[1]".into())
        );
        assert_eq!(
            body("https://api.example.test/addons/list?page=2&kind=a:b"),
            (200, "[2]".into())
        );
        assert_eq!(
            body("https://api.example.test/addons/nothing?game_version=retail").0,
            404
        );
        assert_eq!(FolderHttp::query_suffix("https://h/x"), None);
        assert_eq!(FolderHttp::query_suffix("https://h/x?page=2"), None);
        assert_eq!(
            FolderHttp::query_suffix("https://h/x?b=2&a=1&page=3"),
            Some("@a=1&b=2".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_folder_stub_answers_a_post_from_the_same_files_and_keeps_its_body() {
        let root = std::env::temp_dir().join(format!("mrgl-stub-post-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("api.example.test").join("addons");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("_recents@game_version=retail.json"), b"[]").unwrap();
        let stub = FolderHttp::new(&root);
        let request = Request::post(
            "https://api.example.test/addons/_recents?game_version=retail",
            b"{\"addons\":[\"abc\"]}".to_vec(),
        )
        .header("Content-Type", "application/json");
        let answer = stub.post(&request).unwrap();
        assert_eq!((answer.status, answer.body.as_slice()), (200, &b"[]"[..]));
        let missing = stub
            .post(&Request::post(
                "https://api.example.test/addons/_match",
                Vec::new(),
            ))
            .unwrap();
        assert_eq!(missing.status, 404);
        let requests = stub.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], request);
        assert_eq!(
            requests[0].body.as_deref(),
            Some(&b"{\"addons\":[\"abc\"]}"[..])
        );
        assert_eq!(Request::get("https://h/x").body, None);
        // A client that answers GETs only refuses a POST as the network does; a shared one forwards it.
        struct GetOnly;
        impl Http for GetOnly {
            fn get(&self, _request: &Request) -> Result<Response> {
                unreachable!("no GET in this test")
            }
        }
        assert_eq!(
            GetOnly.post(&request).unwrap_err().to_string(),
            "cannot reach https://api.example.test/addons/_recents?game_version=retail: this client sends no POST"
        );
        let shared: std::sync::Arc<dyn Http> = std::sync::Arc::new(FolderHttp::new(&root));
        assert_eq!(shared.post(&request).unwrap().status, 200);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_folder_stub_maps_a_url_to_a_file_and_tags_it() {
        let root = std::env::temp_dir().join(format!("mrgl-stub-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = root.join("api.example.test").join("a").join("b.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"{}").unwrap();
        let stub = FolderHttp::new(&root);
        assert_eq!(
            stub.path_for("https://api.example.test/a/b?per_page=30"),
            root.join("api.example.test").join("a").join("b")
        );
        let first = stub
            .get(&Request::get("https://api.example.test/a/b?per_page=30"))
            .unwrap();
        assert_eq!(first.status, 200);
        assert_eq!(first.body, b"{}");
        let etag = first.header("etag").expect("an ETag").to_string();
        let again = stub
            .get(&Request::get("https://api.example.test/a/b").header("If-None-Match", &etag))
            .unwrap();
        assert_eq!(again.status, 304);
        assert!(again.body.is_empty());
        let missing = stub
            .get(&Request::get("https://api.example.test/nothing"))
            .unwrap();
        assert_eq!(missing.status, 404);
        assert_eq!(stub.requests().len(), 3);
        // A path that tries to climb out of the root stays under it.
        assert!(
            stub.path_for("https://host/../../x")
                .starts_with(root.join("host"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_folder_stub_serves_pages_and_a_headers_file() {
        let root = std::env::temp_dir().join(format!("mrgl-stub-pages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("api.example.test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("list.json"), b"[1]").unwrap();
        std::fs::write(
            dir.join("list.json.headers"),
            "Link: <https://api.example.test/list?page=2>; rel=\"next\"\nnot a header line\n",
        )
        .unwrap();
        std::fs::write(dir.join("list.page-2.json"), b"[2]").unwrap();
        std::fs::write(dir.join("limited.json"), b"{}").unwrap();
        std::fs::write(
            dir.join("limited.json.headers"),
            "Status: 403\r\nX-RateLimit-Remaining: 0\r\n",
        )
        .unwrap();
        let stub = FolderHttp::new(&root);
        let first = stub
            .get(&Request::get("https://api.example.test/list?per_page=30"))
            .unwrap();
        assert_eq!((first.status, first.body.as_slice()), (200, &b"[1]"[..]));
        assert_eq!(
            first.header("link"),
            Some("<https://api.example.test/list?page=2>; rel=\"next\"")
        );
        let second = stub
            .get(&Request::get(
                "https://api.example.test/list?per_page=30&page=2",
            ))
            .unwrap();
        assert_eq!((second.status, second.body.as_slice()), (200, &b"[2]"[..]));
        assert_eq!(second.header("link"), None);
        let page_one = stub
            .get(&Request::get("https://api.example.test/list?page=1"))
            .unwrap();
        assert_eq!(page_one.body, b"[1]");
        let missing = stub
            .get(&Request::get("https://api.example.test/list?page=3"))
            .unwrap();
        assert_eq!(missing.status, 404);
        // A status from the headers file is never a 304, whatever the request's tag.
        let limited = stub
            .get(
                &Request::get("https://api.example.test/limited")
                    .header("If-None-Match", FolderHttp::etag_of(b"{}")),
            )
            .unwrap();
        assert_eq!(limited.status, 403);
        assert_eq!(limited.header("x-ratelimit-remaining"), Some("0"));
        assert_eq!(page_of("https://h/x?per_page=30"), None);
        assert_eq!(page_of("https://h/x?per_page=30&page=4"), Some(4));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_stub_folder_is_no_network() {
        let root = std::env::temp_dir().join(format!("mrgl-stub-gone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let error = FolderHttp::new(&root)
            .get(&Request::get("https://api.example.test/a"))
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot reach https://api.example.test/a: offline, the stub folder is missing"
        );
    }

    #[test]
    fn an_rfc3339_time_reads_back_as_unix_seconds() {
        assert_eq!(
            unix_from_rfc3339("2026-10-21T14:03:05Z"),
            Some(1_792_591_385)
        );
        assert_eq!(unix_from_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        for bad in [
            "2026-10-21T14:03:05",
            "2026-10-21 14:03:05Z",
            "2026-13-21T14:03:05Z",
            "2026-10-21T14:03Z",
            "yesterday",
        ] {
            assert_eq!(unix_from_rfc3339(bad), None, "{bad}");
        }
    }

    #[test]
    fn md5_follows_the_rfc_1321_test_suite_and_a_pick_is_checked_against_it() {
        for (text, md5) in [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
        ] {
            assert_eq!(md5_hex(text.as_bytes()), md5, "{text:?}");
        }
        let pick = |md5: Option<&str>| Pick {
            source: "wowi:1".into(),
            release: "v1".into(),
            version: "1".into(),
            published: None,
            prerelease: false,
            asset: "A.zip".into(),
            url: "https://x.test/a".into(),
            size: None,
            md5: md5.map(str::to_string),
            flavor: "mainline".into(),
            interface: None,
            guess: false,
            reason: String::new(),
            notes: None,
            page: None,
            nolib: false,
            forever_build: ForeverBuild::Unknown,
            stale: None,
        };
        assert!(check_md5(&pick(None), b"anything").is_ok());
        assert!(check_md5(&pick(Some("900150983CD24FB0D6963F7D28E17F72")), b"abc").is_ok());
        let error = check_md5(&pick(Some("0cc175b9c0f1b6a831c399e269772661")), b"abc").unwrap_err();
        assert_eq!(
            error.to_string(),
            "refused A.zip of wowi:1 v1: its MD5 is 900150983cd24fb0d6963f7d28e17f72, the source names 0cc175b9c0f1b6a831c399e269772661; the download was damaged or the file changed, try again"
        );
        assert_eq!(version_of_tag("v1.2.3"), "1.2.3");
        assert_eq!(version_of_tag("version-1"), "version-1");
        assert_eq!(
            forever_why(SourceKind::WowInterface, ForeverBuild::Unknown),
            "WoWInterface lists no Forever compatibility"
        );
        assert_eq!(
            forever_why(SourceKind::GitHub, ForeverBuild::Unknown),
            "no release.json says"
        );
    }

    #[test]
    fn a_channel_reads_and_writes_its_name() {
        assert_eq!(Channel::default(), Channel::Stable);
        assert_eq!(Channel::parse("stable"), Some(Channel::Stable));
        assert_eq!(Channel::parse(" PreRelease "), Some(Channel::Prerelease));
        assert_eq!(Channel::parse("beta"), Some(Channel::Prerelease));
        assert_eq!(Channel::parse("nightly"), None);
        assert_eq!(Channel::Prerelease.to_string(), "prerelease");
        assert_eq!(
            serde_json::to_string(&Channel::Prerelease).expect("json"),
            "\"prerelease\""
        );
        assert_eq!(
            serde_json::from_str::<Channel>("\"stable\"").expect("json"),
            Channel::Stable
        );
        assert!(Channel::Stable.is_stable() && !Channel::Prerelease.is_stable());
    }

    #[test]
    fn a_target_knows_its_name_and_major() {
        let target = Target::new(FlavorId::Forever);
        assert_eq!(target.flavor_name(), "forever");
        assert_eq!(target.major(), None);
        let target = Target {
            version: BuildVersion::parse("5.5.4.70531"),
            ..Target::new(FlavorId::Classic)
        };
        assert_eq!(target.major(), Some(5));
        assert_eq!(target.channel, Channel::Stable);
        let beta = target.with_channel(Channel::Prerelease);
        assert_eq!(beta.channel, Channel::Prerelease);
        assert_eq!(beta.major(), Some(5));
        assert_eq!(
            Target::new(FlavorId::Other("wow_x".into())).flavor_name(),
            "wow_x"
        );
    }

    #[test]
    fn the_notes_line_is_the_first_line_with_text_and_its_markers_gone() {
        assert_eq!(
            notes_line("Fixes the quest log\nand more").as_deref(),
            Some("Fixes the quest log")
        );
        assert_eq!(
            notes_line("\r\n  \r\n## What's new ##\r\n- one").as_deref(),
            Some("What's new")
        );
        assert_eq!(
            notes_line("---\n\n* **Bold first item**\n").as_deref(),
            Some("Bold first item")
        );
        assert_eq!(
            notes_line("> quoted line  ").as_deref(),
            Some("quoted line")
        );
    }

    #[test]
    fn an_empty_body_or_one_of_markers_only_has_no_notes() {
        assert_eq!(notes_line(""), None);
        assert_eq!(notes_line("  \r\n\n\t"), None);
        assert_eq!(notes_line("#\n## \n###\n---\n***\n>"), None);
    }

    #[test]
    fn a_body_of_headings_only_gives_the_first_heading() {
        assert_eq!(
            notes_line("# v2.1.0\n## Changes\n### Fixes").as_deref(),
            Some("v2.1.0")
        );
    }

    #[test]
    fn a_long_line_is_cut_with_an_ellipsis() {
        let exact = "a".repeat(NOTES_MAX_CHARS);
        assert_eq!(notes_line(&exact).as_deref(), Some(exact.as_str()));
        let long = format!("{} tail", "é".repeat(NOTES_MAX_CHARS));
        let cut = notes_line(&long).expect("a line");
        assert_eq!(cut.chars().count(), NOTES_MAX_CHARS);
        assert!(cut.ends_with('…'), "{cut}");
        assert!(cut.starts_with("éé"), "{cut}");
        // A cut that lands after a space drops it, so the ellipsis follows a word.
        let spaced = format!("{} {}", "a".repeat(NOTES_MAX_CHARS - 2), "b".repeat(10));
        assert_eq!(
            notes_line(&spaced),
            Some(format!("{}…", "a".repeat(NOTES_MAX_CHARS - 2)))
        );
    }
}
