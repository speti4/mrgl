//! Forever notes (task 011): two signals on an addon for the Forever flavor, derived first and curated
//! second. The derived one, [`ForeverBuild`], says whether the addon's source publishes a `forever` build,
//! read from the release the source already fetches (`sources::github::forever_build`). The curated one is
//! `data/forever-notes.json` in the repo, compiled into the core with `include_str!`: the crippled cases
//! only, an addon that has a Forever build but half works under Forever's combat restrictions, one note per
//! addon keyed by its source spec, with a one-line reason, the date it was verified against its source and
//! the source. A note verified more than [`STALE_AFTER_DAYS`] days ago is marked stale in the output.
//! [`ForeverStatus`] joins the two in the shape the window's data contract names (`forever: { build, note
//! }`), with the date, the source and the stale flag next to them.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::backup::clock;
use crate::sources::SourceSpec;

/// The curated file as it is compiled in.
pub const CURATED_JSON: &str = include_str!("../../../data/forever-notes.json");
/// The schema of the curated file; the schema check test holds the file to it.
pub const SCHEMA: u32 = 1;
/// A note verified longer ago than this is stale: Forever's restrictions and the addons move with every
/// patch, so a note older than a season is shown with a warning until someone verifies it again.
pub const STALE_AFTER_DAYS: i64 = 90;
/// How many days a `verified` date may lie after today in UTC: a note dated by the local day east of UTC
/// is a day ahead of UTC between local and UTC midnight, and the schema check still takes it.
pub const VERIFIED_SLACK_DAYS: i64 = 1;

/// Whether an addon's source publishes a `forever` build, as its current release says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForeverBuild {
    /// The release's `release.json` names a `forever` (or `camelot`) flavor for one of its zips.
    Yes,
    /// The release's `release.json` names flavors, none of them Forever, and no asset name says forever.
    No,
    /// Nothing says: no `release.json`, an unreadable one, a pick by asset name, or nothing cached.
    #[default]
    Unknown,
}

impl ForeverBuild {
    /// The value's name, as the JSON writes it.
    pub const fn as_str(self) -> &'static str {
        match self {
            ForeverBuild::Yes => "yes",
            ForeverBuild::No => "no",
            ForeverBuild::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ForeverBuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The curated file: the restrictions (`kinds`) and the notes (`addons`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedNotes {
    /// [`SCHEMA`].
    pub schema: u32,
    /// What the file is, for a reader of the raw JSON.
    #[serde(default)]
    pub about: Option<String>,
    /// Forever's restrictions that cripple an addon, by a kebab-case id.
    pub kinds: BTreeMap<String, Restriction>,
    /// One note per addon, keyed by its source spec (`github:<owner>/<repo>`).
    pub addons: BTreeMap<String, Note>,
}

/// One of Forever's restrictions, described once with its source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Restriction {
    /// One line, lower case, no trailing full stop.
    pub reason: String,
    /// The date the reason was checked against the source, `YYYY-MM-DD`.
    pub verified: String,
    /// Where the reason comes from, an `https://` URL.
    pub source: String,
}

/// The note on one addon: which restriction cripples it and how.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    /// The restriction, an id of `kinds`.
    pub kind: String,
    /// One line, lower case, no trailing full stop: what does not work on Forever.
    pub reason: String,
    /// The date the note was checked against the source, `YYYY-MM-DD`.
    pub verified: String,
    /// Where the note comes from, an `https://` URL.
    pub source: String,
}

impl CuratedNotes {
    /// Parses a curated file.
    pub fn parse(text: &str) -> Result<CuratedNotes, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    /// The note on a source spec: the spec is read in its canonical spelling when it parses, and the key
    /// compares ASCII case-insensitively (GitHub's owners and repositories are case-insensitive).
    pub fn note(&self, spec: &str) -> Option<&Note> {
        let canonical = SourceSpec::parse(spec)
            .map(|spec| spec.to_string())
            .unwrap_or_else(|_| spec.trim().to_string());
        self.addons
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(&canonical))
            .map(|(_, note)| note)
    }

    /// Everything wrong with the file, one line each; empty when the file holds to the schema: the schema
    /// number, every key a canonical source spec, every kind known, every reason one line, every date a real
    /// day no later than [`VERIFIED_SLACK_DAYS`] after `today` (UTC), every source an `https://` URL.
    pub fn problems(&self, today: i64) -> Vec<String> {
        let mut problems = Vec::new();
        if self.schema != SCHEMA {
            problems.push(format!("schema {} (expected {SCHEMA})", self.schema));
        }
        let check =
            |problems: &mut Vec<String>, what: &str, reason: &str, verified: &str, source: &str| {
                let reason_ok = !reason.trim().is_empty()
                    && !reason.contains('\n')
                    && !reason.ends_with('.')
                    && reason.trim() == reason;
                if !reason_ok {
                    problems.push(format!(
                        "{what}: the reason is not one trimmed line without a full stop"
                    ));
                }
                match day_number(verified) {
                    Some(day) if day <= today + VERIFIED_SLACK_DAYS => {}
                    Some(_) => problems.push(format!(
                        "{what}: verified {verified} is in the future, more than \
                         {VERIFIED_SLACK_DAYS} day after today in UTC"
                    )),
                    None => problems.push(format!(
                        "{what}: verified {verified} is not a YYYY-MM-DD date"
                    )),
                }
                if !source.starts_with("https://") || source.contains(char::is_whitespace) {
                    problems.push(format!("{what}: the source {source} is not an https URL"));
                }
            };
        for (id, kind) in &self.kinds {
            let kebab = !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !kebab {
                problems.push(format!("kind {id}: the id is not kebab-case"));
            }
            check(
                &mut problems,
                &format!("kind {id}"),
                &kind.reason,
                &kind.verified,
                &kind.source,
            );
        }
        for (key, note) in &self.addons {
            match SourceSpec::parse(key) {
                Ok(spec) if spec.to_string() == *key => {}
                Ok(spec) => {
                    problems.push(format!("addon {key}: the key is not canonical ({spec})"))
                }
                Err(error) => problems.push(format!("addon {key}: {error}")),
            }
            if !self.kinds.contains_key(&note.kind) {
                problems.push(format!("addon {key}: unknown kind {}", note.kind));
            }
            check(
                &mut problems,
                &format!("addon {key}"),
                &note.reason,
                &note.verified,
                &note.source,
            );
        }
        problems
    }
}

/// The curated file compiled into this build. The schema check test parses the same text, so an empty
/// set here (an unreadable file) cannot reach a release.
pub fn curated() -> &'static CuratedNotes {
    static NOTES: OnceLock<CuratedNotes> = OnceLock::new();
    NOTES.get_or_init(|| CuratedNotes::parse(CURATED_JSON).unwrap_or_default())
}

/// The Forever signals of one addon: the derived build and the curated note, in the window's shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeverStatus {
    /// Whether the source publishes a `forever` build.
    pub build: ForeverBuild,
    /// The curated note's reason, or `None` when the addon has no note.
    pub note: Option<String>,
    /// The restriction the note names.
    pub kind: Option<String>,
    /// The date the note was verified.
    pub verified: Option<String>,
    /// The note's source.
    pub source: Option<String>,
    /// The note was verified more than [`STALE_AFTER_DAYS`] days before `today`.
    pub stale: bool,
}

impl ForeverStatus {
    /// The status of a source spec with its derived build, the note read from `notes`.
    pub fn of(notes: &CuratedNotes, spec: &str, build: ForeverBuild, today: i64) -> ForeverStatus {
        let note = notes.note(spec);
        ForeverStatus {
            build,
            note: note.map(|note| note.reason.clone()),
            kind: note.map(|note| note.kind.clone()),
            verified: note.map(|note| note.verified.clone()),
            source: note.map(|note| note.source.clone()),
            stale: note.is_some_and(|note| is_stale(&note.verified, today)),
        }
    }

    /// The status of a source spec against the compiled-in notes, today.
    pub fn for_source(spec: &str, build: ForeverBuild) -> ForeverStatus {
        ForeverStatus::of(curated(), spec, build, today())
    }
}

/// Whether a `verified` date is more than [`STALE_AFTER_DAYS`] before `today`; a date that does not
/// parse counts as stale, since nobody can say when it was checked.
pub fn is_stale(verified: &str, today: i64) -> bool {
    day_number(verified).is_none_or(|day| today - day > STALE_AFTER_DAYS)
}

/// Today in UTC, as days since 1970-01-01.
pub fn today() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    seconds.div_euclid(86_400)
}

/// A `YYYY-MM-DD` date as days since 1970-01-01; `None` for anything else or a day the calendar does
/// not have (`2026-02-30`).
pub fn day_number(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let civil = clock::Civil {
        year: year.parse().ok()?,
        month: month.parse().ok()?,
        day: day.parse().ok()?,
        hour: 0,
        minute: 0,
        second: 0,
    };
    if !(1..=12).contains(&civil.month) || civil.day == 0 {
        return None;
    }
    let seconds = clock::unix_from_civil(&civil);
    // A day past the month's end rolls over into the next month: the round trip catches it.
    (clock::civil_from_unix(seconds) == civil).then_some(seconds.div_euclid(86_400))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const DAY_2026_10_21: i64 = 20_747;

    #[test]
    fn dates_are_days_since_the_epoch() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2026-10-21"), Some(DAY_2026_10_21));
        assert_eq!(day_number("2024-02-29"), Some(19_782));
        for bad in [
            "2026-02-30",
            "2026-13-01",
            "2026-00-10",
            "2026-1-01",
            "21-10-2026",
            "2026-10-21T00:00:00Z",
            "",
        ] {
            assert_eq!(day_number(bad), None, "{bad}");
        }
        assert!(today() > DAY_2026_10_21 - 1);
    }

    #[test]
    fn the_compiled_in_file_holds_to_the_schema() {
        let notes = CuratedNotes::parse(CURATED_JSON).expect("data/forever-notes.json parses");
        assert_eq!(&notes, curated());
        assert_eq!(notes.problems(today()), Vec::<String>::new());
        // The cases the research describes in kind: the hidden combat values and the combat log events.
        assert!(notes.kinds.contains_key("secret-combat-values"));
        assert!(notes.kinds.contains_key("combat-log-events"));
    }

    const SYNTHETIC: &str = r#"{
        "schema": 1,
        "kinds": {
            "combat-log-events": { "reason": "no combat log events", "verified": "2026-10-01",
                                   "source": "https://example.test/restrictions" }
        },
        "addons": {
            "github:Example/Fresh": { "kind": "combat-log-events", "reason": "the meter stays empty",
                                      "verified": "2026-10-01", "source": "https://example.test/fresh" },
            "github:Example/Old": { "kind": "combat-log-events", "reason": "threat colors are off",
                                    "verified": "2026-06-01", "source": "https://example.test/old" }
        }
    }"#;

    #[test]
    fn a_note_older_than_the_age_is_stale() {
        let notes = CuratedNotes::parse(SYNTHETIC).expect("parses");
        assert_eq!(notes.problems(DAY_2026_10_21), Vec::<String>::new());
        let fresh = ForeverStatus::of(
            &notes,
            "github:Example/Fresh",
            ForeverBuild::Yes,
            DAY_2026_10_21,
        );
        assert_eq!(fresh.note.as_deref(), Some("the meter stays empty"));
        assert_eq!(fresh.kind.as_deref(), Some("combat-log-events"));
        assert!(!fresh.stale);
        // 2026-06-01 is 142 days before 2026-10-21.
        let old = ForeverStatus::of(
            &notes,
            "https://github.com/example/old",
            ForeverBuild::Unknown,
            DAY_2026_10_21,
        );
        assert_eq!(old.note.as_deref(), Some("threat colors are off"));
        assert!(old.stale);
        assert_eq!(
            serde_json::to_value(&old).unwrap(),
            serde_json::json!({
                "build": "unknown",
                "note": "threat colors are off",
                "kind": "combat-log-events",
                "verified": "2026-06-01",
                "source": "https://example.test/old",
                "stale": true,
            })
        );
        // The boundary: exactly the age is not stale yet, a day more is.
        let day = day_number("2026-10-01").unwrap();
        assert!(!is_stale("2026-10-01", day + STALE_AFTER_DAYS));
        assert!(is_stale("2026-10-01", day + STALE_AFTER_DAYS + 1));
        assert!(is_stale("someday", day));
        let none = ForeverStatus::of(&notes, "github:o/r", ForeverBuild::No, DAY_2026_10_21);
        assert_eq!(none.note, None);
        assert!(!none.stale);
    }

    #[test]
    fn the_schema_check_names_every_problem() {
        let bad = r#"{
            "schema": 2,
            "kinds": { "Bad Id": { "reason": "ends with a stop.", "verified": "2026-10-21",
                                   "source": "http://example.test" } },
            "addons": {
                "github:o/r/x": { "kind": "nope", "reason": "", "verified": "2099-01-01",
                                   "source": "https://example.test" },
                "https://github.com/o/r": { "kind": "Bad Id", "reason": "fine", "verified": "2026-02-30",
                                             "source": "https://example.test" }
            }
        }"#;
        let problems = CuratedNotes::parse(bad)
            .expect("parses")
            .problems(DAY_2026_10_21);
        for needle in [
            "schema 2 (expected 1)",
            "kind Bad Id: the id is not kebab-case",
            "kind Bad Id: the reason is not one trimmed line without a full stop",
            "kind Bad Id: the source http://example.test is not an https URL",
            "addon github:o/r/x: unknown source",
            "addon github:o/r/x: unknown kind nope",
            "addon github:o/r/x: the reason is not one trimmed line",
            "addon github:o/r/x: verified 2099-01-01 is in the future",
            "addon https://github.com/o/r: the key is not canonical (github:o/r)",
            "addon https://github.com/o/r: verified 2026-02-30 is not a YYYY-MM-DD date",
        ] {
            assert!(
                problems.iter().any(|problem| problem.starts_with(needle)),
                "{needle}: {problems:#?}"
            );
        }
        let unknown_field = r#"{ "schema": 1, "kinds": {}, "addons": {}, "extra": true }"#;
        assert!(CuratedNotes::parse(unknown_field).is_err());
    }

    #[test]
    fn a_verified_date_may_be_one_day_after_today_in_utc() {
        let dated = |verified: &str| {
            SYNTHETIC
                .replace("2026-10-01", verified)
                .replace("2026-06-01", verified)
        };
        // Today, and tomorrow in UTC (the local day east of UTC before UTC midnight), pass.
        for verified in ["2026-10-21", "2026-10-22"] {
            let notes = CuratedNotes::parse(&dated(verified)).expect("parses");
            assert_eq!(
                notes.problems(DAY_2026_10_21),
                Vec::<String>::new(),
                "{verified}"
            );
        }
        // Two days ahead is past the slack, for a kind and for a note alike.
        let problems = CuratedNotes::parse(&dated("2026-10-23"))
            .expect("parses")
            .problems(DAY_2026_10_21);
        let late = "verified 2026-10-23 is in the future, more than 1 day after today in UTC";
        assert_eq!(
            problems,
            vec![
                format!("kind combat-log-events: {late}"),
                format!("addon github:Example/Fresh: {late}"),
                format!("addon github:Example/Old: {late}"),
            ]
        );
    }

    #[test]
    fn the_build_writes_lower_case() {
        assert_eq!(
            serde_json::to_value([ForeverBuild::Yes, ForeverBuild::No, ForeverBuild::Unknown])
                .unwrap(),
            serde_json::json!(["yes", "no", "unknown"])
        );
        assert_eq!(ForeverBuild::default().to_string(), "unknown");
    }
}
