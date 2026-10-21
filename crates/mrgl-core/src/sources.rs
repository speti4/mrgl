//! Addon sources, in the order D-002 names them: GitHub releases with `release.json` (task 003, [`github`]),
//! then Wago, WoWInterface and CurseForge with the user's own key (M4 Sources), and a zip in hand. A source
//! answers two questions, "which release is current for this addon" and "which asset is the one for this
//! flavor", and never guesses "latest" without saying so: a [`Pick`] carries `guess` and the reason. The
//! network sits behind the [`Http`] trait: [`UreqHttp`] is the real client, [`FolderHttp`] answers from a
//! folder of fixture files, so every test runs offline (`MRGL_HTTP_STUB` points the command line at one).

pub mod github;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::flavor::{BuildVersion, Flavor, FlavorId};

/// The environment variable that names a folder the command line answers every request from instead of
/// the network (the end-to-end tests set it); see [`FolderHttp`].
pub const HTTP_STUB_ENV: &str = "MRGL_HTTP_STUB";

/// Where an addon comes from, parsed from a spec on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSpec {
    /// `github:<owner>/<repo>`: the repository's releases.
    GitHub { owner: String, repo: String },
}

impl SourceSpec {
    /// Parses a spec: `github:<owner>/<repo>` (also a `https://github.com/<owner>/<repo>` URL, with or
    /// without `.git` and a trailing slash). Anything else is an error that names the accepted shape.
    pub fn parse(text: &str) -> Result<SourceSpec> {
        let text = text.trim();
        let unknown = || Error::UnknownSource {
            spec: text.to_string(),
            hint: "expected github:<owner>/<repo>",
        };
        let path = if let Some(path) = text.strip_prefix("github:") {
            path
        } else if let Some(path) = text
            .strip_prefix("https://github.com/")
            .or_else(|| text.strip_prefix("http://github.com/"))
        {
            path
        } else {
            return Err(unknown());
        };
        let path = path.trim_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let mut parts = path.split('/');
        let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(unknown());
        };
        let is_name = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        };
        if !is_name(owner) || !is_name(repo) {
            return Err(unknown());
        }
        Ok(SourceSpec::GitHub {
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }
}

impl fmt::Display for SourceSpec {
    /// The canonical spelling, `github:<owner>/<repo>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceSpec::GitHub { owner, repo } => write!(f, "github:{owner}/{repo}"),
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

/// What a release is picked for: the flavor, and the installed build when it is known. The build decides
/// between the game types of the progressing classic (Cataclysm, Mists, …), which share one flavor id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub flavor: FlavorId,
    pub version: Option<BuildVersion>,
}

impl Target {
    /// A target with the flavor alone; the build is unknown.
    pub fn new(flavor: FlavorId) -> Target {
        Target {
            flavor,
            version: None,
        }
    }

    /// The target of a flavor folder read from disk.
    pub fn from_flavor(flavor: &Flavor) -> Target {
        Target {
            flavor: flavor.id.clone(),
            version: flavor.version,
        }
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
    /// The release is marked a prerelease (alpha, beta); picked only when no other release exists.
    pub prerelease: bool,
    /// The asset's file name.
    pub asset: String,
    /// The asset's download URL.
    pub url: String,
    /// The asset's size in bytes, when the source says.
    pub size: Option<u64>,
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
}

impl Pick {
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

/// A source: resolves a repository and a flavor to a release pick, and fetches the picked asset.
pub trait Source {
    /// The current release's asset for the target, or why there is none.
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick>;

    /// Downloads the picked asset to a path, creating the parent folders.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()>;
}

/// One HTTP GET, as a source issues it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    /// Header name and value; the names are sent as given.
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Request {
        Request {
            url: url.into(),
            headers: Vec::new(),
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

/// The network behind a source. One method, so a stub is a few lines and a test never opens a socket.
pub trait Http {
    /// Sends a GET and returns whatever came back; only a transport failure is an error, a status is the
    /// caller's to read.
    fn get(&self, request: &Request) -> Result<Response>;
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

impl Http for UreqHttp {
    fn get(&self, request: &Request) -> Result<Response> {
        let network = |reason: String| Error::Network {
            url: request.url.clone(),
            reason,
        };
        let mut call = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        let mut response = call.call().map_err(|error| network(error.to_string()))?;
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
            .map_err(|error| network(format!("reading the body: {error}")))?;
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// The offline stand-in: a folder of files, one per URL. `https://api.github.com/repos/o/r/releases?per_page=30`
/// is answered from `<root>/api.github.com/repos/o/r/releases` or, when that is missing, the same path
/// with `.json`; the query string is ignored; a path with no file is a 404 with an empty body. Every answer
/// carries an `ETag` derived from the file's content, and a request whose `If-None-Match` names that tag
/// gets a 304 without a body, so the cache is testable too. The requests are kept for the tests to read.
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

    /// The tag of a body: its length and an FNV-1a hash, quoted as an entity tag.
    pub fn etag_of(body: &[u8]) -> String {
        let hash = body.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
        format!("\"{}-{hash:016x}\"", body.len())
    }
}

impl Http for FolderHttp {
    fn get(&self, request: &Request) -> Result<Response> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        let path = self.path_for(&request.url);
        let file = if path.is_file() {
            path
        } else {
            let mut with_json = path.clone().into_os_string();
            with_json.push(".json");
            PathBuf::from(with_json)
        };
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
        let headers = vec![("ETag".to_string(), etag.clone())];
        if request.header_value("If-None-Match") == Some(etag.as_str()) {
            return Ok(Response {
                status: 304,
                headers,
                body: Vec::new(),
            });
        }
        Ok(Response {
            status: 200,
            headers,
            body,
        })
    }
}

/// A client picked by the environment: the folder stub when `MRGL_HTTP_STUB` names a folder, else the real
/// one. The command line and the window build their sources through this.
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
}

impl<H: Http + ?Sized> Http for &H {
    fn get(&self, request: &Request) -> Result<Response> {
        (**self).get(request)
    }
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
        for spec in [
            "Questie/Questie",
            "github:Questie",
            "github:Questie/Questie/releases",
            "github:/Questie",
            "github:Que stie/Questie",
            "wago:abc",
            "",
        ] {
            let error = SourceSpec::parse(spec).unwrap_err().to_string();
            assert!(
                error.starts_with("unknown source: ")
                    && error.ends_with("(expected github:<owner>/<repo>)"),
                "{spec}: {error}"
            );
        }
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
    fn a_target_knows_its_name_and_major() {
        let target = Target::new(FlavorId::Forever);
        assert_eq!(target.flavor_name(), "forever");
        assert_eq!(target.major(), None);
        let target = Target {
            flavor: FlavorId::Classic,
            version: BuildVersion::parse("5.5.4.70531"),
        };
        assert_eq!(target.major(), Some(5));
        assert_eq!(
            Target::new(FlavorId::Other("wow_x".into())).flavor_name(),
            "wow_x"
        );
    }
}
