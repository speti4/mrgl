//! A game flavor folder's identity (task 002). The flavor is read from the folder itself, never guessed
//! from its name: the folder's `.flavor.info` holds the id (`wow`, `wow_classic`, `wow_classic_era`,
//! `wowf`), and the install root's `.build.info`, one level up, holds the version of every installed
//! product. The id is also the state sidecar's folder name (D-003).
//!
//! `.flavor.info` is two lines, a typed header and the value:
//!
//! ```text
//! Product Flavor!STRING:0
//! wowf
//! ```
//!
//! `.build.info` is pipe-delimited with a typed header row (`Name!TYPE:n` per column) and one row per
//! product; the columns are found by name, never by position. The interface number of a build is derived
//! from its version: `12.1.5` is `120105`, `1.60.1` is `16001`.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::ser::SerializeStruct;

use crate::error::{Error, Result};
use crate::toc::TocSuffix;

/// The flavor ids Blizzard's launcher writes into `.flavor.info`, with the folder they usually live in.
pub const KNOWN_FLAVORS: [(&str, &str); 4] = [
    ("wow", "_retail_"),
    ("wow_classic", "_classic_"),
    ("wow_classic_era", "_classic_era_"),
    ("wowf", "_forever_"),
];

/// The file in a flavor folder that names the flavor.
pub const FLAVOR_INFO: &str = ".flavor.info";
/// The file at the install root that holds every product's version.
pub const BUILD_INFO: &str = ".build.info";
/// The CurseForge app's marker in a flavor folder it manages.
pub const CURSE_CLIENT: &str = ".curseclient";

/// A flavor id as `.flavor.info` spells it; an id the manager does not know is carried as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlavorId {
    /// `wow`: retail, friendly name `retail`.
    Retail,
    /// `wow_classic`: the progressing classic, friendly name `classic`.
    Classic,
    /// `wow_classic_era`: Classic Era, friendly name `era`.
    ClassicEra,
    /// `wowf`: World of Warcraft: Forever, friendly name `forever`.
    Forever,
    /// An id the manager does not know yet; accepted with a warning.
    Other(String),
}

impl FlavorId {
    /// The id as `.flavor.info` spells it; never fails.
    pub fn parse(id: &str) -> FlavorId {
        match id {
            "wow" => FlavorId::Retail,
            "wow_classic" => FlavorId::Classic,
            "wow_classic_era" => FlavorId::ClassicEra,
            "wowf" => FlavorId::Forever,
            other => FlavorId::Other(other.to_string()),
        }
    }

    /// A friendly name (`forever`, `retail`, `classic`, `era`) or an id, ASCII case-insensitive; `None`
    /// for anything else (a folder path, a typo).
    pub fn from_spec(text: &str) -> Option<FlavorId> {
        let text = text.trim();
        [
            FlavorId::Retail,
            FlavorId::Classic,
            FlavorId::ClassicEra,
            FlavorId::Forever,
        ]
        .into_iter()
        .find(|id| {
            id.as_str().eq_ignore_ascii_case(text)
                || id
                    .friendly_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case(text))
        })
    }

    /// The id string.
    pub fn as_str(&self) -> &str {
        match self {
            FlavorId::Retail => "wow",
            FlavorId::Classic => "wow_classic",
            FlavorId::ClassicEra => "wow_classic_era",
            FlavorId::Forever => "wowf",
            FlavorId::Other(id) => id,
        }
    }

    /// The name the command line accepts for `--flavor`.
    pub fn friendly_name(&self) -> Option<&'static str> {
        match self {
            FlavorId::Retail => Some("retail"),
            FlavorId::Classic => Some("classic"),
            FlavorId::ClassicEra => Some("era"),
            FlavorId::Forever => Some("forever"),
            FlavorId::Other(_) => None,
        }
    }

    /// The folder the launcher installs the flavor into, under the game root.
    pub fn folder_name(&self) -> Option<&'static str> {
        match self {
            FlavorId::Retail => Some("_retail_"),
            FlavorId::Classic => Some("_classic_"),
            FlavorId::ClassicEra => Some("_classic_era_"),
            FlavorId::Forever => Some("_forever_"),
            FlavorId::Other(_) => None,
        }
    }

    /// Whether the manager knows the id.
    pub fn is_known(&self) -> bool {
        !matches!(self, FlavorId::Other(_))
    }
}

impl fmt::Display for FlavorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for FlavorId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// A client version as `.build.info` writes it: `12.1.5.70448`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// The build number, the fourth component; `0` when the text had three.
    pub build: u32,
}

impl BuildVersion {
    /// `major.minor.patch.build` or `major.minor.patch`.
    pub fn parse(text: &str) -> Option<BuildVersion> {
        let mut parts = text.trim().split('.').map(|part| part.parse::<u32>().ok());
        let major = parts.next()??;
        let minor = parts.next()??;
        let patch = parts.next()??;
        let build = match parts.next() {
            Some(build) => build?,
            None => 0,
        };
        if parts.next().is_some() {
            return None;
        }
        Some(BuildVersion {
            major,
            minor,
            patch,
            build,
        })
    }

    /// The interface number a TOC names for this version: `major*10000 + minor*100 + patch`.
    pub const fn interface(self) -> u32 {
        self.major * 10000 + self.minor * 100 + self.patch
    }
}

impl fmt::Display for BuildVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

/// A flavor folder, read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flavor {
    pub id: FlavorId,
    /// The flavor folder, absolute.
    pub folder: PathBuf,
    /// The installed build, when `.build.info` had it.
    pub version: Option<BuildVersion>,
    /// The CurseForge app manages this folder's addons: its `.curseclient` marker is in the flavor folder.
    pub curse_client: bool,
    /// Plain lines about what could not be read: an unknown id, a missing `.build.info` or row.
    pub warnings: Vec<String>,
}

impl Flavor {
    /// The folder's own name (`_forever_`), for the summary line.
    pub fn folder_name(&self) -> String {
        self.folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.folder.display().to_string())
    }

    /// The interface number of the installed build.
    pub fn interface(&self) -> Option<u32> {
        self.version.map(BuildVersion::interface)
    }

    /// The TOC suffixes the client tries for this flavor, in order; `None` is the base `<Folder>.toc`.
    pub fn toc_chain(&self) -> Vec<Option<TocSuffix>> {
        toc_chain(&self.id, self.version)
    }
}

impl Serialize for Flavor {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Flavor", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("folder", &self.folder)?;
        state.serialize_field("version", &self.version.map(|v| v.to_string()))?;
        state.serialize_field("interface", &self.interface())?;
        state.serialize_field("build", &self.version.map(|v| v.build))?;
        state.serialize_field("curse_client", &self.curse_client)?;
        state.end()
    }
}

/// The id in a `.flavor.info` text, or the reason it is not one.
pub fn parse_flavor_info(text: &str) -> std::result::Result<String, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err(format!("empty {FLAVOR_INFO}"));
    };
    let id = if first.starts_with("Product Flavor") {
        lines
            .next()
            .ok_or_else(|| format!("no flavor id after the header of {FLAVOR_INFO}"))?
    } else {
        first
    };
    if id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(id.to_string())
    } else {
        Err(format!("unreadable flavor id `{id}` in {FLAVOR_INFO}"))
    }
}

/// The version of one product in a `.build.info` text, or the reason it is not there.
pub fn build_version_of(
    build_info: &str,
    product: &str,
) -> std::result::Result<BuildVersion, String> {
    let build_info = build_info.strip_prefix('\u{feff}').unwrap_or(build_info);
    let mut lines = build_info.lines();
    let header: Vec<&str> = lines
        .next()
        .unwrap_or_default()
        .split('|')
        .map(|column| column.split('!').next().unwrap_or_default().trim())
        .collect();
    let column = |name: &str| header.iter().position(|column| *column == name);
    let (Some(version_at), Some(product_at)) = (column("Version"), column("Product")) else {
        return Err(format!("no Version or Product column in {BUILD_INFO}"));
    };
    for line in lines {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.get(product_at) != Some(&product) {
            continue;
        }
        let text = cells.get(version_at).copied().unwrap_or_default();
        return BuildVersion::parse(text)
            .ok_or_else(|| format!("unreadable version `{text}` for {product} in {BUILD_INFO}"));
    }
    Err(format!("no row for {product} in {BUILD_INFO}"))
}

/// The TOC suffixes the client tries for a flavor, game-type suffix first, then the family, then the base
/// file (`None`). The game-type suffix of the progressing classic follows the build's major version; an
/// unknown version keeps the family and the base; an unknown id has only the base.
pub fn toc_chain(id: &FlavorId, version: Option<BuildVersion>) -> Vec<Option<TocSuffix>> {
    let major = version.map(|v| v.major);
    let mut chain: Vec<Option<TocSuffix>> = match id {
        FlavorId::Retail => {
            let mut chain = Vec::new();
            if major.is_some_and(|major| major >= 12) {
                chain.push(Some(TocSuffix::Standard));
            }
            chain.push(Some(TocSuffix::Mainline));
            chain
        }
        FlavorId::Forever => vec![
            Some(TocSuffix::Forever),
            Some(TocSuffix::Camelot),
            Some(TocSuffix::Mainline),
        ],
        FlavorId::ClassicEra => vec![Some(TocSuffix::Vanilla), Some(TocSuffix::Classic)],
        FlavorId::Classic => {
            let game_type = match major {
                Some(2) => Some(TocSuffix::Tbc),
                Some(3) => Some(TocSuffix::Wrath),
                Some(4) => Some(TocSuffix::Cata),
                Some(5) => Some(TocSuffix::Mists),
                _ => None,
            };
            game_type
                .into_iter()
                .map(Some)
                .chain([Some(TocSuffix::Classic)])
                .collect()
        }
        FlavorId::Other(_) => Vec::new(),
    };
    chain.push(None);
    chain
}

/// Reads a flavor folder: its `.flavor.info` names the flavor (or the folder is not one), the install
/// root's `.build.info` gives the build, and whatever cannot be read about the build is a warning.
pub fn read_flavor(folder: &Path) -> Result<Flavor> {
    let folder = std::path::absolute(folder).map_err(|source| Error::Io {
        context: "resolving the flavor folder",
        path: folder.to_path_buf(),
        source,
    })?;
    let info_path = folder.join(FLAVOR_INFO);
    let info = match std::fs::read_to_string(&info_path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::NotAFlavorFolder {
                path: folder,
                reason: format!("no {FLAVOR_INFO}"),
            });
        }
        Err(source) => {
            return Err(Error::Io {
                context: "reading .flavor.info",
                path: info_path,
                source,
            });
        }
    };
    let id = parse_flavor_info(&info)
        .map(|id| FlavorId::parse(&id))
        .map_err(|reason| Error::NotAFlavorFolder {
            path: folder.clone(),
            reason,
        })?;

    let mut warnings = Vec::new();
    if !id.is_known() {
        warnings.push(format!("unknown flavor id {id} in {}", folder.display()));
    }
    let version = match folder.parent() {
        Some(root) => match std::fs::read_to_string(root.join(BUILD_INFO)) {
            Ok(text) => match build_version_of(&text, id.as_str()) {
                Ok(version) => Some(version),
                Err(reason) => {
                    warnings.push(format!("build unknown: {reason}"));
                    None
                }
            },
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                warnings.push(format!(
                    "build unknown: no {BUILD_INFO} next to {}",
                    folder.display()
                ));
                None
            }
            Err(source) => {
                warnings.push(format!(
                    "build unknown: cannot read {}: {source}",
                    root.join(BUILD_INFO).display()
                ));
                None
            }
        },
        None => {
            warnings.push(format!(
                "build unknown: {} has no parent folder",
                folder.display()
            ));
            None
        }
    };

    let curse_client = folder.join(CURSE_CLIENT).is_file();

    Ok(Flavor {
        id,
        folder,
        version,
        curse_client,
        warnings,
    })
}

/// Every flavor folder directly under a game root (a subfolder with a readable `.flavor.info`), sorted by
/// folder name. A subfolder whose `.flavor.info` is not one is left out; an unreadable root is an error.
pub fn list_flavors(root: &Path) -> Result<Vec<Flavor>> {
    let entries = std::fs::read_dir(root).map_err(|source| Error::Io {
        context: "reading the game folder",
        path: root.to_path_buf(),
        source,
    })?;
    let mut flavors = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: "reading the game folder",
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_dir() || !path.join(FLAVOR_INFO).is_file() {
            continue;
        }
        match read_flavor(&path) {
            Ok(flavor) => flavors.push(flavor),
            Err(Error::NotAFlavorFolder { .. }) => {}
            Err(other) => return Err(other),
        }
    }
    flavors.sort_by_key(Flavor::folder_name);
    Ok(flavors)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_INFO_TEXT: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Tags!STRING:0|Product!STRING:0\n\
eu|1|88d5e8ef05ec5a8446cdec93d142d7c6|12.1.5.70448|Windows x86_64 EU? enUS text?|wow\n\
eu|1|11b890e50f0b1a07bcaad60d5916e65a|5.5.4.70531||wow_classic\n\
us|1|e8c4ef4e67bd1d94d820254984258fa8|1.60.1.70245||wowf\n";

    #[test]
    fn flavor_info_gives_a_known_or_an_other_id() {
        assert_eq!(
            parse_flavor_info("Product Flavor!STRING:0\nwowf\n").as_deref(),
            Ok("wowf")
        );
        assert_eq!(
            parse_flavor_info("\u{feff}Product Flavor!STRING:0\r\nwow_classic\r\n").as_deref(),
            Ok("wow_classic")
        );
        assert_eq!(parse_flavor_info("wow\n").as_deref(), Ok("wow"));
        assert_eq!(FlavorId::parse("wowf"), FlavorId::Forever);
        assert_eq!(
            FlavorId::parse("wow_mrgl"),
            FlavorId::Other("wow_mrgl".to_string())
        );
        assert!(!FlavorId::parse("wow_mrgl").is_known());
        assert_eq!(FlavorId::parse("wow_mrgl").as_str(), "wow_mrgl");
    }

    #[test]
    fn a_bad_flavor_info_has_a_reason() {
        assert_eq!(parse_flavor_info(""), Err("empty .flavor.info".to_string()));
        assert_eq!(
            parse_flavor_info("Product Flavor!STRING:0\n"),
            Err("no flavor id after the header of .flavor.info".to_string())
        );
        assert_eq!(
            parse_flavor_info("Product Flavor!STRING:0\nwow f\n"),
            Err("unreadable flavor id `wow f` in .flavor.info".to_string())
        );
    }

    #[test]
    fn build_info_finds_version_by_column_name_not_position() {
        assert_eq!(
            build_version_of(BUILD_INFO_TEXT, "wow"),
            Ok(BuildVersion {
                major: 12,
                minor: 1,
                patch: 5,
                build: 70448
            })
        );
        assert_eq!(
            build_version_of(BUILD_INFO_TEXT, "wowf").map(|v| v.to_string()),
            Ok("1.60.1.70245".to_string())
        );
        let swapped = "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\n";
        assert_eq!(
            build_version_of(swapped, "wowf").map(|v| v.interface()),
            Ok(16001)
        );
    }

    #[test]
    fn build_info_without_the_row_or_the_column_is_a_reason() {
        assert_eq!(
            build_version_of(BUILD_INFO_TEXT, "wow_classic_era"),
            Err("no row for wow_classic_era in .build.info".to_string())
        );
        assert_eq!(
            build_version_of("Branch!STRING:0|Active!DEC:1\neu|1\n", "wow"),
            Err("no Version or Product column in .build.info".to_string())
        );
        assert_eq!(
            build_version_of("Version!STRING:0|Product!STRING:0\nlatest|wow\n", "wow"),
            Err("unreadable version `latest` for wow in .build.info".to_string())
        );
    }

    #[test]
    fn a_version_gives_its_interface_number() {
        let interface = |text: &str| BuildVersion::parse(text).map(BuildVersion::interface);
        assert_eq!(interface("12.1.5.70448"), Some(120105));
        assert_eq!(interface("5.5.4.70531"), Some(50504));
        assert_eq!(interface("1.60.1.70245"), Some(16001));
        assert_eq!(interface("1.15.9"), Some(11509));
        assert_eq!(BuildVersion::parse("1.15"), None);
        assert_eq!(BuildVersion::parse("1.15.9.1.2"), None);
        assert_eq!(BuildVersion::parse("a.b.c"), None);
    }

    #[test]
    fn the_chain_follows_id_and_major() {
        use TocSuffix::*;
        let version = |major| {
            Some(BuildVersion {
                major,
                minor: 0,
                patch: 0,
                build: 0,
            })
        };
        assert_eq!(
            toc_chain(&FlavorId::Retail, version(12)),
            [Some(Standard), Some(Mainline), None]
        );
        assert_eq!(
            toc_chain(&FlavorId::Retail, version(11)),
            [Some(Mainline), None]
        );
        assert_eq!(toc_chain(&FlavorId::Retail, None), [Some(Mainline), None]);
        assert_eq!(
            toc_chain(&FlavorId::Forever, version(1)),
            [Some(Forever), Some(Camelot), Some(Mainline), None]
        );
        assert_eq!(
            toc_chain(&FlavorId::ClassicEra, version(1)),
            [Some(Vanilla), Some(Classic), None]
        );
        for (major, game_type) in [(2, Tbc), (3, Wrath), (4, Cata), (5, Mists)] {
            assert_eq!(
                toc_chain(&FlavorId::Classic, version(major)),
                [Some(game_type), Some(Classic), None]
            );
        }
        assert_eq!(
            toc_chain(&FlavorId::Classic, version(6)),
            [Some(Classic), None]
        );
        assert_eq!(toc_chain(&FlavorId::Classic, None), [Some(Classic), None]);
        assert_eq!(
            toc_chain(&FlavorId::Other("wow_mrgl".into()), version(1)),
            [None]
        );
    }

    #[test]
    fn a_spec_is_a_friendly_name_or_an_id() {
        assert_eq!(FlavorId::from_spec("forever"), Some(FlavorId::Forever));
        assert_eq!(FlavorId::from_spec("Era"), Some(FlavorId::ClassicEra));
        assert_eq!(FlavorId::from_spec("wow_classic"), Some(FlavorId::Classic));
        assert_eq!(FlavorId::from_spec("WOW"), Some(FlavorId::Retail));
        assert_eq!(FlavorId::from_spec("C:\\games\\_retail_"), None);
        assert_eq!(FlavorId::from_spec("wow_mrgl"), None);
        assert_eq!(FlavorId::Forever.folder_name(), Some("_forever_"));
        assert_eq!(FlavorId::Other("x".into()).folder_name(), None);
        for (id, folder) in KNOWN_FLAVORS {
            assert_eq!(FlavorId::parse(id).folder_name(), Some(folder));
        }
    }
}
