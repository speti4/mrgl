//! The router (task 092): one [`Sources`] value holds a source per [`SourceKind`] and implements [`Source`]
//! by handing every spec to the source it names. The command line and the window build it once per run
//! ([`Sources::from_env`]; the window's offline view over a client that never reaches the network,
//! [`Sources::cached`]) and never build a source themselves, so a new source joins here and nowhere else.
//!
//! A spec of a source this build does not have, or whose source cannot be asked in this run (its key is
//! not set), is [`Error::SourceNotConfigured`] with what is missing; [`Source::missing`] says so before
//! anything is asked, which is how the list import and the update check tell such an entry apart.

use std::path::Path;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::forever::ForeverBuild;
use crate::sources::github::{self, GitHub};
use crate::sources::wago::{self, Wago};
use crate::sources::wowinterface::{self, WowInterface};
use crate::sources::{
    BuildOffer, Channel, Http, Offered, Pick, RequestBudget, Source, SourceKind, SourceSpec,
    Target, http_from_env, not_configured,
};

/// The sources of one run, one slot per [`SourceKind`]; an empty slot is a source this build does not
/// have.
pub struct Sources {
    slots: [Option<Box<dyn Source>>; SourceKind::ALL.len()],
}

impl Sources {
    /// No source at all: every spec reads "source not configured".
    pub fn none() -> Sources {
        Sources {
            slots: [None, None, None, None],
        }
    }

    /// The sources of this process over one client ([`http_from_env`]: the folder stub when
    /// `MRGL_HTTP_STUB` names one, else the network), each with its key and its cache under the mrgl home
    /// folder: GitHub with its token from `MRGL_GITHUB_TOKEN` or the key store ([`GitHub::from_env`],
    /// [`crate::keys`]), WoWInterface without a key (task 094), Wago with `MRGL_WAGO_KEY` or the key store
    /// (task 095; without it a `wago:` spec says the key is missing).
    pub fn from_env() -> Sources {
        let http: Arc<dyn Http> = Arc::from(http_from_env());
        Sources::none()
            .with(SourceKind::GitHub, GitHub::from_env(Arc::clone(&http)))
            .with(SourceKind::Wago, Wago::from_env(Arc::clone(&http)))
            .with(SourceKind::WowInterface, WowInterface::from_env(http))
    }

    /// The sources over a client of the caller's, with their caches under a home folder (`None`: no cache)
    /// and no key: the window's offline view (task 078) passes a client that fails every request, so each
    /// source answers from its own cache by its offline rule; a test passes the folder stub.
    pub fn cached(http: Arc<dyn Http>, home: Option<&Path>) -> Sources {
        Sources::none()
            .with(
                SourceKind::GitHub,
                GitHub::new(
                    Arc::clone(&http),
                    None,
                    home.map(|home| home.join(github::CACHE_SUBDIR)),
                ),
            )
            .with(
                SourceKind::Wago,
                Wago::cached(
                    Arc::clone(&http),
                    home.map(|home| home.join(wago::CACHE_SUBDIR)),
                ),
            )
            .with(
                SourceKind::WowInterface,
                WowInterface::new(http, home.map(|home| home.join(wowinterface::CACHE_SUBDIR))),
            )
    }

    /// The same sources with one slot filled, or replaced: the source that answers every spec of `kind`.
    pub fn with(mut self, kind: SourceKind, source: impl Source + 'static) -> Sources {
        self.slots[kind.index()] = Some(Box::new(source));
        self
    }

    /// Whether this build has a source for `kind` (its key may still be missing: [`Source::missing`]).
    pub fn has(&self, kind: SourceKind) -> bool {
        self.slots[kind.index()].is_some()
    }

    /// The source of a spec's kind, when this build has one.
    fn slot(&self, spec: &SourceSpec) -> Option<&dyn Source> {
        self.slots[spec.kind().index()].as_deref()
    }

    /// The source that answers for a spec, or [`Error::SourceNotConfigured`] naming what is missing.
    fn source_for(&self, spec: &SourceSpec) -> Result<&dyn Source> {
        let Some(source) = self.slot(spec) else {
            return Err(Error::SourceNotConfigured {
                spec: spec.to_string(),
                missing: no_source(spec.kind()),
            });
        };
        match not_configured(source, spec) {
            Some(error) => Err(error),
            None => Ok(source),
        }
    }
}

/// The clause of a source this build does not have.
fn no_source(kind: SourceKind) -> String {
    format!("this build has no {} source", kind.name())
}

impl Source for Sources {
    fn resolve(&self, spec: &SourceSpec, target: &Target) -> Result<Pick> {
        self.source_for(spec)?.resolve(spec, target)
    }

    fn resolve_version(&self, spec: &SourceSpec, target: &Target, version: &str) -> Result<Pick> {
        self.source_for(spec)?
            .resolve_version(spec, target, version)
    }

    /// The download of a pick goes to the source its `source` names.
    fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
        let spec = SourceSpec::parse(&pick.source)?;
        self.source_for(&spec)?.fetch(pick, to)
    }

    fn listed_versions(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        self.slot(spec)?.listed_versions(spec)
    }

    /// The budget nearest its end among the sources that keep one: the update check asks the budget of
    /// the spec at hand ([`Source::request_budget_for`]); this one is for a caller with no spec.
    fn request_budget(&self) -> Option<RequestBudget> {
        self.slots
            .iter()
            .flatten()
            .filter_map(|source| source.request_budget())
            .min_by_key(|budget| budget.remaining)
    }

    fn request_budget_for(&self, spec: &SourceSpec) -> Option<RequestBudget> {
        self.slot(spec)?.request_budget_for(spec)
    }

    /// The source of the spec's kind turns it into the spec it records (task 105: a Wago page address into
    /// `wago:<id>`); a spec of a source this build lacks stays as it is.
    fn canonical_spec(&self, spec: &SourceSpec, target: &Target) -> Result<SourceSpec> {
        match self.slot(spec) {
            Some(source) => source.canonical_spec(spec, target),
            None => Ok(spec.clone()),
        }
    }

    /// Each source gets the specs it answers, in their order, and only those it can ask (task 106).
    fn prefetch(&self, specs: &[SourceSpec], target: &Target) {
        for kind in SourceKind::ALL {
            let Some(source) = self.slots[kind.index()].as_deref() else {
                continue;
            };
            let own: Vec<SourceSpec> = specs
                .iter()
                .filter(|spec| spec.kind() == kind && source.missing(spec).is_none())
                .cloned()
                .collect();
            if !own.is_empty() {
                source.prefetch(&own, target);
            }
        }
    }

    fn holds(&self, spec: &SourceSpec, target: &Target) -> bool {
        self.slot(spec)
            .is_some_and(|source| source.holds(spec, target))
    }

    fn missing(&self, spec: &SourceSpec) -> Option<String> {
        match self.slot(spec) {
            Some(source) => source.missing(spec),
            None => Some(no_source(spec.kind())),
        }
    }

    /// The source's own answer from its cache; `unknown` for a source this build does not have.
    fn cached_forever_build(&self, spec: &SourceSpec, channel: Channel) -> ForeverBuild {
        match self.slot(spec) {
            Some(source) => source.cached_forever_build(spec, channel),
            None => ForeverBuild::Unknown,
        }
    }

    /// The source's own answer (task 097); `unknown` for a source this build does not have, with what is
    /// missing.
    fn offers(&self, spec: &SourceSpec, target: &Target) -> BuildOffer {
        match self.slot(spec) {
            Some(source) => source.offers(spec, target),
            None => BuildOffer::new(Offered::Unknown, no_source(spec.kind())),
        }
    }

    fn folders(&self, spec: &SourceSpec) -> Option<Vec<String>> {
        self.slot(spec)?.folders(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavor::FlavorId;
    use crate::sources::{FolderHttp, Request, Response};

    /// A source that answers every spec with a pick named after it, and says what it lacks when told to.
    struct Named {
        name: &'static str,
        missing: Option<&'static str>,
    }

    impl Source for Named {
        fn resolve(&self, spec: &SourceSpec, _target: &Target) -> Result<Pick> {
            Ok(Pick {
                source: spec.to_string(),
                release: self.name.to_string(),
                version: self.name.to_string(),
                published: None,
                prerelease: false,
                asset: format!("{}.zip", self.name),
                url: format!("https://example.test/{}.zip", self.name),
                size: None,
                md5: None,
                flavor: "forever".to_string(),
                interface: None,
                guess: false,
                reason: "a test pick".to_string(),
                notes: None,
                page: None,
                nolib: false,
                forever_build: ForeverBuild::Yes,
                stale: None,
            })
        }

        fn fetch(&self, pick: &Pick, to: &Path) -> Result<()> {
            std::fs::write(to, pick.release.as_bytes()).map_err(|source| Error::Io {
                context: "writing the test download",
                path: to.to_path_buf(),
                source,
            })
        }

        fn missing(&self, _spec: &SourceSpec) -> Option<String> {
            self.missing.map(str::to_string)
        }

        fn cached_forever_build(&self, _spec: &SourceSpec, _channel: Channel) -> ForeverBuild {
            ForeverBuild::Yes
        }

        fn request_budget(&self) -> Option<RequestBudget> {
            Some(RequestBudget {
                host: self.name,
                limit: 60,
                sent: 1,
                remaining: if self.name == "wago" { 3 } else { 59 },
                reset: None,
                hint: String::new(),
                window: "",
                resets: "soon",
            })
        }
    }

    fn spec(text: &str) -> SourceSpec {
        SourceSpec::parse(text).expect("a spec")
    }

    #[test]
    fn every_spec_goes_to_the_source_it_names() {
        let sources = Sources::none()
            .with(
                SourceKind::GitHub,
                Named {
                    name: "github",
                    missing: None,
                },
            )
            .with(
                SourceKind::Wago,
                Named {
                    name: "wago",
                    missing: None,
                },
            );
        let target = Target::new(FlavorId::Forever);
        assert_eq!(
            sources
                .resolve(&spec("github:o/r"), &target)
                .unwrap()
                .release,
            "github"
        );
        let pick = sources.resolve(&spec("wago:abc"), &target).unwrap();
        assert_eq!(pick.release, "wago");
        assert_eq!(
            sources
                .resolve_version(&spec("wago:abc"), &target, "wago")
                .unwrap()
                .source,
            "wago:abc"
        );
        assert!(sources.has(SourceKind::Wago) && !sources.has(SourceKind::CurseForge));
        assert_eq!(sources.missing(&spec("wago:abc")), None);
        assert_eq!(
            sources.cached_forever_build(&spec("wago:abc"), Channel::Stable),
            ForeverBuild::Yes
        );
        assert_eq!(
            sources
                .request_budget_for(&spec("github:o/r"))
                .map(|budget| budget.host),
            Some("github")
        );
        assert_eq!(
            sources.request_budget().map(|budget| budget.host),
            Some("wago"),
            "the budget nearest its end"
        );
        let dir = std::env::temp_dir().join(format!("mrgl-router-fetch-{}", std::process::id()));
        let to = dir.join("pick.zip");
        std::fs::create_dir_all(&dir).unwrap();
        sources.fetch(&pick, &to).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"wago");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_source_this_build_lacks_or_cannot_ask_names_what_is_missing() {
        let sources = Sources::none().with(
            SourceKind::CurseForge,
            Named {
                name: "curse",
                missing: Some("needs MRGL_CURSEFORGE_KEY"),
            },
        );
        let target = Target::new(FlavorId::Retail);
        assert_eq!(
            sources
                .resolve(&spec("wowi:123"), &target)
                .unwrap_err()
                .to_string(),
            "source not configured: wowi:123 (this build has no WoWInterface source)"
        );
        assert_eq!(
            sources
                .resolve(&spec("curse:42"), &target)
                .unwrap_err()
                .to_string(),
            "source not configured: curse:42 (needs MRGL_CURSEFORGE_KEY)"
        );
        assert_eq!(
            sources.missing(&spec("wago:abc")).as_deref(),
            Some("this build has no Wago source")
        );
        assert_eq!(
            sources.cached_forever_build(&spec("wago:abc"), Channel::Stable),
            ForeverBuild::Unknown
        );
        assert_eq!(sources.listed_versions(&spec("wago:abc")), None);
        assert_eq!(sources.request_budget_for(&spec("wago:abc")), None);
        let error = Sources::none()
            .fetch(
                &Named {
                    name: "x",
                    missing: None,
                }
                .resolve(&spec("github:o/r"), &target)
                .unwrap(),
                Path::new("unused.zip"),
            )
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "source not configured: github:o/r (this build has no GitHub source)"
        );
    }

    /// A client that fails every request as the network does.
    struct Offline;

    impl Http for Offline {
        fn get(&self, request: &Request) -> Result<Response> {
            Err(Error::Network {
                url: request.url.clone(),
                reason: "offline".to_string(),
            })
        }
    }

    #[test]
    fn the_cached_sources_have_github_and_answer_offline_from_the_cache_alone() {
        let sources = Sources::cached(Arc::new(Offline), None);
        assert!(sources.has(SourceKind::GitHub));
        assert!(sources.has(SourceKind::WowInterface), "task 094");
        assert!(sources.has(SourceKind::Wago), "task 095");
        assert!(!sources.has(SourceKind::CurseForge));
        // The offline view asks no key of Wago: it answers from the cache alone.
        assert_eq!(sources.missing(&spec("wago:abc")), None);
        let error = sources
            .resolve(&spec("github:o/r"), &Target::new(FlavorId::Forever))
            .unwrap_err();
        assert!(matches!(error, Error::Network { .. }), "{error}");
        assert_eq!(
            sources.cached_forever_build(&spec("github:o/r"), Channel::Stable),
            ForeverBuild::Unknown
        );
        // The folder stub behind the router answers as it does behind the source.
        let stub = Sources::cached(Arc::new(FolderHttp::new("no-such-folder")), None);
        assert!(matches!(
            stub.resolve(&spec("github:o/r"), &Target::new(FlavorId::Forever)),
            Err(Error::Network { .. })
        ));
    }
}
