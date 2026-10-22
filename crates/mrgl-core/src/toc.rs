//! TOC parsing (task 002): the `## Key: value` header lines of an addon's `.toc` files, the multi-TOC file
//! names (the game-type suffixes `_Standard`, `_Forever`, `_Camelot`, `_Mists`, `_Cata`, `_Wrath`, `_TBC`,
//! `_Vanilla`, the family suffixes `_Mainline` and `_Classic`, the legacy `_BCC` and `_WOTLKC`), the
//! comma-separated `## Interface:` lists, and the title, version, dependency and source-hint fields. Pure
//! functions over text, tested against the fixtures.
//!
//! The client reads a TOC like this (warcraft.wiki.gg, "TOC format"): a line that starts with `##` is a
//! directive, `#` alone starts a comment, anything else names a file; the value after the first colon is
//! trimmed; only the first 1024 characters of a line count; a repeated directive keeps its last value,
//! except the dependency lists, which append.

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

/// How many bytes of a line the client reads; the rest is ignored.
pub const MAX_LINE_BYTES: usize = 1024;

/// The flavor-specific TOC file name suffixes the client knows, canonical spelling in [`TocSuffix::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TocSuffix {
    /// Retail from 12.0 ("Midnight"), ahead of `_Mainline`.
    Standard,
    /// World of Warcraft: Forever.
    Forever,
    /// Forever's codename; the packager still writes it.
    Camelot,
    /// Mists of Pandaria Classic.
    Mists,
    /// Cataclysm Classic.
    Cata,
    /// Wrath of the Lich King Classic.
    Wrath,
    /// Burning Crusade Classic.
    Tbc,
    /// Classic Era.
    Vanilla,
    /// The retail family: retail and Forever, after their game-type suffixes.
    Mainline,
    /// The classic family: every classic flavor, after its game-type suffix.
    Classic,
    /// Legacy Burning Crusade Classic; the client no longer loads it.
    Bcc,
    /// Legacy Wrath Classic; the client no longer loads it.
    Wotlkc,
}

impl TocSuffix {
    /// Every suffix, game-type ones first, then the families, then the legacy ones.
    pub const ALL: [TocSuffix; 12] = [
        TocSuffix::Standard,
        TocSuffix::Forever,
        TocSuffix::Camelot,
        TocSuffix::Mists,
        TocSuffix::Cata,
        TocSuffix::Wrath,
        TocSuffix::Tbc,
        TocSuffix::Vanilla,
        TocSuffix::Mainline,
        TocSuffix::Classic,
        TocSuffix::Bcc,
        TocSuffix::Wotlkc,
    ];

    /// The suffix a file name part names, ASCII case-insensitive (`Vanilla`, `CATA`, `wrath`), without
    /// the separator.
    pub fn parse(text: &str) -> Option<TocSuffix> {
        TocSuffix::ALL
            .into_iter()
            .find(|suffix| suffix.name().eq_ignore_ascii_case(text))
    }

    const fn name(self) -> &'static str {
        match self {
            TocSuffix::Standard => "Standard",
            TocSuffix::Forever => "Forever",
            TocSuffix::Camelot => "Camelot",
            TocSuffix::Mists => "Mists",
            TocSuffix::Cata => "Cata",
            TocSuffix::Wrath => "Wrath",
            TocSuffix::Tbc => "TBC",
            TocSuffix::Vanilla => "Vanilla",
            TocSuffix::Mainline => "Mainline",
            TocSuffix::Classic => "Classic",
            TocSuffix::Bcc => "BCC",
            TocSuffix::Wotlkc => "WOTLKC",
        }
    }

    /// The canonical suffix with its underscore, as the client spells it: `_Vanilla`.
    pub const fn as_str(self) -> &'static str {
        match self {
            TocSuffix::Standard => "_Standard",
            TocSuffix::Forever => "_Forever",
            TocSuffix::Camelot => "_Camelot",
            TocSuffix::Mists => "_Mists",
            TocSuffix::Cata => "_Cata",
            TocSuffix::Wrath => "_Wrath",
            TocSuffix::Tbc => "_TBC",
            TocSuffix::Vanilla => "_Vanilla",
            TocSuffix::Mainline => "_Mainline",
            TocSuffix::Classic => "_Classic",
            TocSuffix::Bcc => "_BCC",
            TocSuffix::Wotlkc => "_WOTLKC",
        }
    }

    /// A suffix the client no longer loads.
    pub const fn is_legacy(self) -> bool {
        matches!(self, TocSuffix::Bcc | TocSuffix::Wotlkc)
    }
}

impl fmt::Display for TocSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for TocSuffix {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What a `.toc` file name inside an addon folder means, from its stem against the folder name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TocName {
    /// `<Folder>.toc`, the file every flavor falls back to.
    Base,
    /// `<Folder>_<Suffix>.toc` with a suffix the client loads.
    Suffix(TocSuffix),
    /// A hyphen separator (`<Folder>-Wrath.toc`, old packager output) or a legacy suffix: listed, never
    /// loaded.
    Legacy(TocSuffix),
    /// A `.toc` whose stem is neither the folder nor the folder plus a known suffix.
    Other,
}

/// Classifies a file name of an addon folder; `None` when it is not a `.toc` at all. The comparison is
/// ASCII case-insensitive, as the file system is.
pub fn classify_toc_name(folder: &str, file_name: &str) -> Option<TocName> {
    let stem_end = file_name.len().checked_sub(4)?;
    let (stem, extension) = (file_name.get(..stem_end)?, file_name.get(stem_end..)?);
    if !extension.eq_ignore_ascii_case(".toc") {
        return None;
    }
    if stem.eq_ignore_ascii_case(folder) {
        return Some(TocName::Base);
    }
    let Some(rest) = stem
        .get(..folder.len())
        .filter(|head| head.eq_ignore_ascii_case(folder))
        .and_then(|_| stem.get(folder.len()..))
    else {
        return Some(TocName::Other);
    };
    let Some(separator) = rest.chars().next() else {
        return Some(TocName::Other);
    };
    if separator != '_' && separator != '-' {
        return Some(TocName::Other);
    }
    match TocSuffix::parse(&rest[separator.len_utf8()..]) {
        Some(suffix) if separator == '-' || suffix.is_legacy() => Some(TocName::Legacy(suffix)),
        Some(suffix) => Some(TocName::Suffix(suffix)),
        None => Some(TocName::Other),
    }
}

/// The headers of one `.toc` file: the typed fields the manager uses, and every header as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Toc {
    /// `## Title`, as written, color escapes included.
    pub title: Option<String>,
    /// The title without its color escapes.
    pub title_plain: Option<String>,
    /// `## Notes`, as written.
    pub notes: Option<String>,
    /// The notes without their color escapes.
    pub notes_plain: Option<String>,
    /// `## Version`.
    pub version: Option<String>,
    /// `## Author`.
    pub author: Option<String>,
    /// `## Interface`, every number of the comma-separated list.
    pub interface: Vec<u32>,
    /// `## Dependencies`, `## RequiredDeps` and any `## Dep…` line, appended in file order.
    pub dependencies: Vec<String>,
    /// `## OptionalDeps`, appended in file order.
    pub optional_dependencies: Vec<String>,
    /// `## LoadOnDemand: 1`.
    pub load_on_demand: bool,
    /// `## X-Curse-Project-ID`.
    pub curse_project_id: Option<String>,
    /// `## X-Wago-ID`.
    pub wago_id: Option<String>,
    /// `## X-WoWI-ID`.
    pub wowi_id: Option<String>,
    /// `## X-Website`, the addon's page (task 097): a GitHub repository's address names a source
    /// ([`github_repo_of_website`]), any other page is only a page.
    pub website: Option<String>,
    /// Every `## Key: value` header: the key as first spelled, the value of its last line.
    pub headers: BTreeMap<String, String>,
}

impl Toc {
    fn set(&mut self, key: &str, value: &str) {
        match self
            .headers
            .keys()
            .find(|known| known.eq_ignore_ascii_case(key))
            .cloned()
        {
            Some(known) => {
                self.headers.insert(known, value.to_string());
            }
            None => {
                self.headers.insert(key.to_string(), value.to_string());
            }
        }

        let owned = || Some(value.to_string());
        if key.eq_ignore_ascii_case("Title") {
            self.title = owned();
            self.title_plain = Some(strip_color_escapes(value));
        } else if key.eq_ignore_ascii_case("Notes") {
            self.notes = owned();
            self.notes_plain = Some(strip_color_escapes(value));
        } else if key.eq_ignore_ascii_case("Version") {
            // An unsubstituted packager token (`@project-version@`, a TOC from the repository rather than a
            // packaged release) is no version (task 097); the raw headers keep it.
            self.version = (!is_packager_token(value)).then(|| value.to_string());
        } else if key.eq_ignore_ascii_case("Author") {
            self.author = owned();
        } else if key.eq_ignore_ascii_case("Interface") {
            self.interface = parse_interface_list(value);
        } else if key.eq_ignore_ascii_case("RequiredDeps")
            || starts_with_ignore_ascii_case(key, "Dep")
        {
            self.dependencies.extend(split_list(value));
        } else if key.eq_ignore_ascii_case("OptionalDeps") {
            self.optional_dependencies.extend(split_list(value));
        } else if key.eq_ignore_ascii_case("LoadOnDemand") {
            self.load_on_demand = value.trim() == "1";
        } else if key.eq_ignore_ascii_case("X-Curse-Project-ID") {
            self.curse_project_id = owned();
        } else if key.eq_ignore_ascii_case("X-Wago-ID") {
            self.wago_id = owned();
        } else if key.eq_ignore_ascii_case("X-WoWI-ID") {
            self.wowi_id = owned();
        } else if key.eq_ignore_ascii_case("X-Website") {
            self.website = owned();
        }
    }

    /// The `## X-Wago-ID` when it has the shape of a Wago id ([`is_wago_id`]).
    pub fn wago_id_checked(&self) -> Option<&str> {
        self.wago_id
            .as_deref()
            .map(str::trim)
            .filter(|id| is_wago_id(id))
    }

    /// The `## X-WoWI-ID` when it is a project number ([`is_number_id`]).
    pub fn wowi_id_checked(&self) -> Option<&str> {
        self.wowi_id
            .as_deref()
            .map(str::trim)
            .filter(|id| is_number_id(id))
    }

    /// The `## X-Curse-Project-ID` when it is a project number ([`is_number_id`]).
    pub fn curse_project_id_checked(&self) -> Option<&str> {
        self.curse_project_id
            .as_deref()
            .map(str::trim)
            .filter(|id| is_number_id(id))
    }
}

/// Whether a header value is a packager token left as written (`@project-version@`, `@file-revision@`): an
/// `@`, a name of letters, digits and `-` or `_`, and an `@`.
pub fn is_packager_token(value: &str) -> bool {
    let value = value.trim();
    value.len() > 2
        && value.starts_with('@')
        && value.ends_with('@')
        && value[1..value.len() - 1]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether an `## X-Wago-ID` has the shape of a Wago id: eight ASCII letters and digits (`L6Jb2R6v`).
pub fn is_wago_id(id: &str) -> bool {
    id.len() == 8 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Whether an `## X-WoWI-ID` or `## X-Curse-Project-ID` is a project number: ASCII digits, not all zero, at
/// most twelve.
pub fn is_number_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 12
        && id.chars().all(|c| c.is_ascii_digit())
        && id.chars().any(|c| c != '0')
}

/// The `<owner>/<repo>` of an `## X-Website` on github.com (`https://github.com/<owner>/<repo>`, with or
/// without `www.`, `.git`, a trailing slash or a deeper path such as `/wiki`); `None` for any other page.
pub fn github_repo_of_website(website: &str) -> Option<String> {
    let website = website.trim();
    let address = website
        .strip_prefix("https://")
        .or_else(|| website.strip_prefix("http://"))
        .unwrap_or(website);
    let address = address.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = address.split_once('/')?;
    let host = host.to_ascii_lowercase();
    if host != "github.com" && host != "www.github.com" {
        return None;
    }
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    let is_name = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    (is_name(owner) && is_name(repo)).then(|| format!("{owner}/{repo}"))
}

fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// Parses the text of a `.toc` file: a BOM is dropped, CRLF and LF both end a line, every line is cut at
/// [`MAX_LINE_BYTES`] (on a character boundary), and only the `## Key: value` lines count.
pub fn parse_toc(text: &str) -> Toc {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut toc = Toc::default();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some((key, value)) = header_line(cut(line, MAX_LINE_BYTES)) {
            toc.set(key, value);
        }
    }
    toc
}

/// A `## Key: value` line as its trimmed key and value; `None` for a comment, a file name or a directive
/// without a colon. The `##` must start the line: whitespace before it makes the line a file name.
pub fn header_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("##")?;
    let (key, value) = rest.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value.trim()))
}

/// A comma-separated value as its trimmed, non-empty items.
pub fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// The numbers of a comma-separated `## Interface` value; an item that is not a number is dropped (it is
/// still in the raw headers).
pub fn parse_interface_list(value: &str) -> Vec<u32> {
    split_list(value)
        .iter()
        .filter_map(|item| item.parse().ok())
        .collect()
}

/// Removes the UI color escapes (`|cAARRGGBB` and `|r`) a title or notes line may carry.
pub fn strip_color_escapes(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find('|') {
        plain.push_str(&rest[..position]);
        let after = &rest[position + 1..];
        let skipped = if let Some(tail) = after.strip_prefix(['c', 'C']) {
            tail.get(..8)
                .filter(|code| code.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|_| &tail[8..])
        } else {
            after.strip_prefix(['r', 'R'])
        };
        match skipped {
            Some(tail) => rest = tail,
            None => {
                plain.push('|');
                rest = after;
            }
        }
    }
    plain.push_str(rest);
    plain
}

fn cut(line: &str, max: usize) -> &str {
    if line.len() <= max {
        return line;
    }
    let mut end = max;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_line_is_a_trimmed_key_and_value() {
        assert_eq!(
            header_line("## Interface:  120105 "),
            Some(("Interface", "120105"))
        );
        assert_eq!(header_line("##Title: X"), Some(("Title", "X")));
        assert_eq!(
            header_line("## X-Website: https://example.test/a"),
            Some(("X-Website", "https://example.test/a"))
        );
    }

    #[test]
    fn comment_and_file_lines_are_not_headers() {
        assert_eq!(header_line("# a comment"), None);
        assert_eq!(header_line("core/init.lua"), None);
        assert_eq!(header_line(" ## Title: indented is a file name"), None);
        assert_eq!(header_line("## no colon"), None);
        assert_eq!(header_line("## : no key"), None);
    }

    #[test]
    fn keys_match_case_insensitively_and_keep_the_first_spelling() {
        let toc = parse_toc("## title: One\n## Title: Two\n");
        assert_eq!(toc.title.as_deref(), Some("Two"));
        assert_eq!(toc.headers.get("title").map(String::as_str), Some("Two"));
        assert_eq!(toc.headers.len(), 1);
    }

    #[test]
    fn a_repeated_key_keeps_the_last_value() {
        let toc = parse_toc("## Version: 1.0\n## Version: 1.1\n");
        assert_eq!(toc.version.as_deref(), Some("1.1"));
    }

    #[test]
    fn dependency_keys_append_and_split_on_commas() {
        let toc = parse_toc(
            "## RequiredDeps: A, B\n## Dependencies: C\n## Deps: D,E\n## OptionalDeps: F, , G\n",
        );
        assert_eq!(toc.dependencies, ["A", "B", "C", "D", "E"]);
        assert_eq!(toc.optional_dependencies, ["F", "G"]);
    }

    #[test]
    fn the_interface_list_parses_with_and_without_spaces() {
        assert_eq!(parse_interface_list("120105,50504"), [120105, 50504]);
        assert_eq!(
            parse_interface_list("120105, 50504, 11509"),
            [120105, 50504, 11509]
        );
        assert_eq!(parse_interface_list("120105, next"), [120105]);
        let toc = parse_toc("## Interface: 11509\n## Interface: 120105, 50504\n");
        assert_eq!(toc.interface, [120105, 50504]);
    }

    #[test]
    fn color_escapes_leave_the_plain_title() {
        assert_eq!(
            strip_color_escapes("|cff00ff00Green|r Addon"),
            "Green Addon"
        );
        assert_eq!(strip_color_escapes("|CFFFF0000Red|R"), "Red");
        assert_eq!(strip_color_escapes("a |c short"), "a |c short");
        assert_eq!(strip_color_escapes("no escapes"), "no escapes");
        let toc = parse_toc("## Title: |cff00ff00Green|r Addon\n## Notes: |cffff0000hot|r\n");
        assert_eq!(toc.title.as_deref(), Some("|cff00ff00Green|r Addon"));
        assert_eq!(toc.title_plain.as_deref(), Some("Green Addon"));
        assert_eq!(toc.notes_plain.as_deref(), Some("hot"));
    }

    #[test]
    fn a_bom_and_crlf_are_removed() {
        let toc = parse_toc("\u{feff}## Title: Bom\r\n## Version: 2\r\n");
        assert_eq!(toc.title.as_deref(), Some("Bom"));
        assert_eq!(toc.version.as_deref(), Some("2"));
    }

    #[test]
    fn a_line_counts_only_to_1024_bytes_on_a_char_boundary() {
        let long = format!("## Notes: {}", "é".repeat(600));
        let toc = parse_toc(&long);
        let notes = toc.notes.expect("notes");
        assert!(notes.len() <= MAX_LINE_BYTES);
        assert!(notes.chars().all(|c| c == 'é'));
        assert_eq!(cut("abc", 2), "ab");
        assert_eq!(cut("éé", 3), "é");
    }

    #[test]
    fn a_localized_title_stays_in_the_raw_map() {
        let toc = parse_toc("## Title: Plain\n## Title-deDE: Deutsch\n");
        assert_eq!(toc.title.as_deref(), Some("Plain"));
        assert_eq!(
            toc.headers.get("Title-deDE").map(String::as_str),
            Some("Deutsch")
        );
    }

    #[test]
    fn the_source_hints_and_load_on_demand_are_typed() {
        let toc = parse_toc(
            "## X-Curse-Project-ID: 12345\n## X-Wago-ID: abcDEF12\n## X-WoWI-ID: 678\n## LoadOnDemand: 1\n## Author: Someone\n",
        );
        assert_eq!(toc.curse_project_id.as_deref(), Some("12345"));
        assert_eq!(toc.wago_id.as_deref(), Some("abcDEF12"));
        assert_eq!(toc.wowi_id.as_deref(), Some("678"));
        assert!(toc.load_on_demand);
        assert_eq!(toc.author.as_deref(), Some("Someone"));
        assert!(!parse_toc("## LoadOnDemand: 0\n").load_on_demand);
    }

    #[test]
    fn the_id_headers_are_checked_for_their_shape() {
        let toc =
            parse_toc("## X-Curse-Project-ID: 12345\n## X-Wago-ID: L6Jb2R6v\n## X-WoWI-ID: 0678\n");
        assert_eq!(toc.curse_project_id_checked(), Some("12345"));
        assert_eq!(toc.wago_id_checked(), Some("L6Jb2R6v"));
        assert_eq!(toc.wowi_id_checked(), Some("0678"));
        let odd =
            parse_toc("## X-Curse-Project-ID: @curse-id@\n## X-Wago-ID: short\n## X-WoWI-ID: 0\n");
        assert_eq!(odd.curse_project_id_checked(), None);
        assert_eq!(odd.wago_id_checked(), None);
        assert_eq!(odd.wowi_id_checked(), None);
        // The raw values stay typed and in the headers, as `list` shows them.
        assert_eq!(odd.wago_id.as_deref(), Some("short"));
        assert!(!is_wago_id("L6Jb2R6v9") && !is_wago_id("L6Jb-R6v"));
        assert!(is_number_id("1") && !is_number_id("12a") && !is_number_id("1234567890123"));
    }

    #[test]
    fn an_unsubstituted_packager_token_is_no_version() {
        let toc = parse_toc("## Version: @project-version@\n");
        assert_eq!(toc.version, None);
        assert_eq!(
            toc.headers.get("Version").map(String::as_str),
            Some("@project-version@")
        );
        assert_eq!(
            parse_toc("## Version: v1.2.3\n").version.as_deref(),
            Some("v1.2.3")
        );
        assert!(is_packager_token(" @file-revision@ "));
        assert!(!is_packager_token("@@") && !is_packager_token("1.0 @beta@ x"));
    }

    #[test]
    fn a_github_website_names_its_repository_and_another_page_none() {
        let toc = parse_toc("## X-Website: https://github.com/Owner/Repo\n");
        assert_eq!(
            toc.website.as_deref(),
            Some("https://github.com/Owner/Repo")
        );
        for (website, repo) in [
            ("https://github.com/Owner/Repo", Some("Owner/Repo")),
            ("http://www.GitHub.com/Owner/Repo.git/", Some("Owner/Repo")),
            (
                "https://github.com/Owner/Repo/wiki/Setup",
                Some("Owner/Repo"),
            ),
            ("github.com/Owner/Repo?tab=readme", Some("Owner/Repo")),
            ("https://github.com/Owner", None),
            ("https://github.com", None),
            ("https://owner.github.io/Repo", None),
            ("https://www.example.test/addon", None),
        ] {
            assert_eq!(
                github_repo_of_website(website).as_deref(),
                repo,
                "{website}"
            );
        }
    }

    #[test]
    fn a_toc_name_is_base_suffix_legacy_or_other() {
        assert_eq!(classify_toc_name("Foo", "Foo.toc"), Some(TocName::Base));
        assert_eq!(classify_toc_name("Foo", "foo.TOC"), Some(TocName::Base));
        assert_eq!(
            classify_toc_name("Foo", "Foo_Vanilla.toc"),
            Some(TocName::Suffix(TocSuffix::Vanilla))
        );
        assert_eq!(
            classify_toc_name("Foo", "Foo_CATA.toc"),
            Some(TocName::Suffix(TocSuffix::Cata))
        );
        assert_eq!(
            classify_toc_name("Foo", "Foo-Wrath.toc"),
            Some(TocName::Legacy(TocSuffix::Wrath))
        );
        assert_eq!(
            classify_toc_name("Foo", "Foo_BCC.toc"),
            Some(TocName::Legacy(TocSuffix::Bcc))
        );
        assert_eq!(
            classify_toc_name("Foo", "Foo_Options.toc"),
            Some(TocName::Other)
        );
        assert_eq!(classify_toc_name("Foo", "Bar.toc"), Some(TocName::Other));
        assert_eq!(classify_toc_name("Foo", "FooBar.toc"), Some(TocName::Other));
        assert_eq!(classify_toc_name("Foo", "Foo.lua"), None);
        assert_eq!(classify_toc_name("Foo", "toc"), None);
    }

    #[test]
    fn a_suffix_parses_in_any_case_and_names_its_legacy_ones() {
        assert_eq!(TocSuffix::parse("mainline"), Some(TocSuffix::Mainline));
        assert_eq!(TocSuffix::parse("TBC"), Some(TocSuffix::Tbc));
        assert_eq!(TocSuffix::parse("Plunderstorm"), None);
        assert!(TocSuffix::Bcc.is_legacy() && TocSuffix::Wotlkc.is_legacy());
        assert!(!TocSuffix::Classic.is_legacy());
        assert_eq!(TocSuffix::Tbc.as_str(), "_TBC");
        for suffix in TocSuffix::ALL {
            assert_eq!(TocSuffix::parse(&suffix.as_str()[1..]), Some(suffix));
        }
    }
}
