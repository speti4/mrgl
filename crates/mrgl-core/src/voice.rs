//! The voice table (D-005, `docs/nerglish.md`): every human-readable message is a murloc line and a plain
//! English line. The murloc line is picked by the message's [`Kind`]; the plain line is the caller's and
//! carries every fact. `serious` drops the murloc line. Nothing here prints: [`Voice::say`] returns the
//! text and the caller decides where it goes, so the CLI, the window and the tests share one table.
//! The serious switch has three sources, the first that says wins: the `--serious` flag, `MRGL_SERIOUS`,
//! then the `serious` preference of `config.json` the window's switch writes (task 034, [`crate::home`]).

use std::fmt;

/// What a message is about; it picks the murloc line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `--version` and the greeting of the empty window (`--help` is clap's and stays plain).
    Banner,
    /// Something is running: a scan, a download, an install.
    Progress,
    /// A command finished and did what it said.
    Success,
    /// A command finished and found nothing to change.
    Nothing,
    /// Finished, but with something to look at.
    Warning,
    /// A command failed; the plain line says why.
    Error,
    /// mrgl itself is broken: a panic, a sidecar it cannot rebuild.
    Fatal,
}

impl Kind {
    /// The murloc line of this kind, from the table in `docs/nerglish.md`. A kind always sounds like itself:
    /// an error never sounds like a success.
    pub const fn murloc(self) -> &'static str {
        match self {
            Kind::Banner => "Mrglglgl.",
            Kind::Progress => "Mrrrgl… mrgl mrgl.",
            Kind::Success => "Mrglglglgl!",
            Kind::Nothing => "Mrgl.",
            Kind::Warning => "Mrgl? Mrgl…",
            Kind::Error => "Rwlrwlrwlrwl?!",
            Kind::Fatal => "Aaaaaughibbrgubugbugrguburgle!",
        }
    }

    /// The kind's name in machine-readable output (`--json`), fixed here so a renamed variant cannot change it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Banner => "banner",
            Kind::Progress => "progress",
            Kind::Success => "success",
            Kind::Nothing => "nothing",
            Kind::Warning => "warning",
            Kind::Error => "error",
            Kind::Fatal => "fatal",
        }
    }
}

/// The voice setting of one run: murloc on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Voice {
    serious: bool,
}

/// The environment variable that turns the murloc off; any value but empty and `0` counts.
pub const SERIOUS_ENV: &str = "MRGL_SERIOUS";

impl Voice {
    /// A voice that speaks murloc (`serious == false`) or plain English only.
    pub const fn new(serious: bool) -> Self {
        Self { serious }
    }

    /// The voice of a run: the `--serious` flag wins, then `MRGL_SERIOUS`, then the `serious` preference
    /// of the config file (task 034).
    pub fn from_env(serious_flag: bool) -> Self {
        Self::new(serious_of_process(serious_flag).0)
    }

    /// Whether the murloc line is dropped.
    pub const fn is_serious(self) -> bool {
        self.serious
    }

    /// The message as it is printed: the murloc line, a newline and the plain line, or the plain line alone.
    pub fn say(self, kind: Kind, plain: impl fmt::Display) -> String {
        if self.serious {
            plain.to_string()
        } else {
            format!("{}\n{}", kind.murloc(), plain)
        }
    }
}

/// `1 orphan`, `2 orphans`, `0 orphans`: a count and its noun for a plain line, the noun taking a plain `s`
/// for any count but one (task 086: the summaries the command line and the window share).
pub fn count_noun(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// `MRGL_SERIOUS` set to anything but empty or `0` means serious.
fn serious_env_set(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

/// Which source decided the serious switch (task 034).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriousSource {
    /// The `--serious` flag.
    Flag,
    /// `MRGL_SERIOUS`, set and not empty (`0` says not serious).
    Environment,
    /// The `serious` preference of `config.json`.
    File,
    /// Nothing set: the murloc speaks.
    Default,
}

impl SeriousSource {
    /// The source's name in machine-readable output.
    pub const fn as_str(self) -> &'static str {
        match self {
            SeriousSource::Flag => "flag",
            SeriousSource::Environment => "environment",
            SeriousSource::File => "file",
            SeriousSource::Default => "default",
        }
    }
}

/// The serious switch from its three sources, the first that says wins: the flag (it can only turn the
/// murloc off), a non-empty `MRGL_SERIOUS` (`0` is not serious, anything else is), then the file's
/// preference, read only when needed; nothing set is not serious.
pub fn serious_from(
    flag: bool,
    environment: Option<&std::ffi::OsStr>,
    file: impl FnOnce() -> Option<bool>,
) -> (bool, SeriousSource) {
    if flag {
        return (true, SeriousSource::Flag);
    }
    if let Some(value) = environment.filter(|value| !value.is_empty()) {
        return (serious_env_set(Some(value)), SeriousSource::Environment);
    }
    match file() {
        Some(serious) => (serious, SeriousSource::File),
        None => (false, SeriousSource::Default),
    }
}

/// [`serious_from`] for this process: `MRGL_SERIOUS` and the config file under the mrgl home folder.
pub fn serious_of_process(flag: bool) -> (bool, SeriousSource) {
    serious_from(flag, std::env::var_os(SERIOUS_ENV).as_deref(), || {
        crate::home::read_preferences().serious
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_the_murloc_line_then_the_plain_line() {
        let text = Voice::new(false).say(Kind::Success, "2 addons updated, 12 up to date");
        let expected = format!(
            "{}\n2 addons updated, 12 up to date",
            Kind::Success.murloc()
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn a_count_takes_its_plural_but_for_one() {
        assert_eq!(count_noun(0, "orphan"), "0 orphans");
        assert_eq!(count_noun(1, "orphan"), "1 orphan");
        assert_eq!(count_noun(2, "addon line"), "2 addon lines");
    }

    #[test]
    fn serious_keeps_only_the_plain_line() {
        let text = Voice::new(true).say(Kind::Error, "addon not found: Questie (flavor forever)");
        assert_eq!(text, "addon not found: Questie (flavor forever)");
    }

    #[test]
    fn every_kind_sounds_like_itself() {
        let kinds = [
            Kind::Banner,
            Kind::Progress,
            Kind::Success,
            Kind::Nothing,
            Kind::Warning,
            Kind::Error,
            Kind::Fatal,
        ];
        let lines: std::collections::HashSet<&str> =
            kinds.iter().map(|kind| kind.murloc()).collect();
        assert_eq!(lines.len(), kinds.len(), "two kinds share a murloc line");
        let names: std::collections::HashSet<&str> =
            kinds.iter().map(|kind| kind.as_str()).collect();
        assert_eq!(names.len(), kinds.len(), "two kinds share a JSON name");
        assert!(kinds.iter().all(|kind| !kind.murloc().is_empty()));
    }

    #[test]
    fn the_environment_variable_counts_unless_empty_or_zero() {
        use std::ffi::OsStr;
        assert!(!serious_env_set(None));
        assert!(!serious_env_set(Some(OsStr::new(""))));
        assert!(!serious_env_set(Some(OsStr::new("0"))));
        assert!(serious_env_set(Some(OsStr::new("1"))));
        assert!(serious_env_set(Some(OsStr::new("true"))));
    }

    #[test]
    fn the_flag_wins_then_the_variable_then_the_file() {
        use std::ffi::OsStr;
        let unread = || -> Option<bool> { panic!("the file is read only when needed") };
        assert_eq!(
            serious_from(true, Some(OsStr::new("0")), unread),
            (true, SeriousSource::Flag)
        );
        assert_eq!(
            serious_from(false, Some(OsStr::new("1")), unread),
            (true, SeriousSource::Environment)
        );
        assert_eq!(
            serious_from(false, Some(OsStr::new("0")), unread),
            (false, SeriousSource::Environment),
            "a variable set to 0 says not serious, whatever the file says"
        );
        assert_eq!(
            serious_from(false, Some(OsStr::new("")), || Some(true)),
            (true, SeriousSource::File),
            "an empty variable is not set"
        );
        assert_eq!(
            serious_from(false, None, || Some(false)),
            (false, SeriousSource::File)
        );
        assert_eq!(
            serious_from(false, None, || None),
            (false, SeriousSource::Default)
        );
    }

    #[test]
    fn the_flag_alone_makes_it_serious() {
        // The environment case runs end to end in the CLI tests, where the variable can be set safely.
        assert!(Voice::from_env(true).is_serious());
    }
}
