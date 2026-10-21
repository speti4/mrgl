//! The AddOns scan (task 002): every folder under `Interface\AddOns` with a TOC becomes an installed addon,
//! with its TOC files ([`crate::toc`]), the one the flavor loads, its source hints (`X-Curse-Project-ID`,
//! `X-Wago-ID`, `X-WoWI-ID`) and whether it is a junction. The scan never writes. Whether the state
//! sidecar knows an addon is task 004's layer on top of this.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::flavor::{self, Flavor};
use crate::toc::{self, Toc, TocName, TocSuffix};

/// One `.toc` file of an addon folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TocFile {
    /// The file name as on disk.
    pub file: String,
    /// The suffix the name carries; `None` for the base `<Folder>.toc`.
    pub suffix: Option<TocSuffix>,
    /// A legacy suffix or a hyphen separator: listed, never loaded by the client.
    pub legacy: bool,
    /// The parsed headers.
    pub toc: Toc,
}

/// An addon folder under `Interface\AddOns`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Addon {
    /// The folder name, which is the addon's name to the client.
    pub folder: String,
    /// The folder's path.
    pub path: PathBuf,
    /// The base, suffix and legacy TOC files, sorted by file name.
    pub tocs: Vec<TocFile>,
    /// `.toc` files whose stem is neither the folder nor the folder plus a known suffix, by name.
    pub other_tocs: Vec<String>,
    /// The file name of the TOC the flavor loads: the first hit of the flavor's chain among the
    /// non-legacy files, or `None` when the flavor loads nothing from this folder.
    pub chosen: Option<String>,
    /// The folder is a junction or a symbolic link (the dev linking of task 005).
    pub linked: bool,
}

impl Addon {
    /// The TOC the flavor loads.
    pub fn chosen_toc(&self) -> Option<&TocFile> {
        let chosen = self.chosen.as_deref()?;
        self.tocs.iter().find(|file| file.file == chosen)
    }

    /// The TOC to read the version and the hints from: the chosen one, else the first non-legacy one,
    /// else the first legacy one.
    pub fn primary_toc(&self) -> Option<&TocFile> {
        self.chosen_toc()
            .or_else(|| self.tocs.iter().find(|file| !file.legacy))
            .or_else(|| self.tocs.first())
    }

    /// The `## Version` of the primary TOC.
    pub fn version(&self) -> Option<&str> {
        self.primary_toc()?.toc.version.as_deref()
    }

    /// Whether any TOC file is a legacy one.
    pub fn has_legacy(&self) -> bool {
        self.tocs.iter().any(|file| file.legacy)
    }
}

/// The result of scanning one AddOns folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Addons {
    /// The addons, sorted by folder name (ASCII case-insensitive).
    pub addons: Vec<Addon>,
    /// Folders with no `.toc` at all, or ones the scan could not read, by name.
    pub skipped: Vec<String>,
    /// Plain lines about folders the scan could not read.
    pub warnings: Vec<String>,
}

/// The result of scanning a flavor folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scan {
    pub flavor: Flavor,
    pub addons: Vec<Addon>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
}

/// The AddOns folder of a flavor folder.
pub fn addons_dir(flavor_folder: &Path) -> PathBuf {
    flavor_folder.join("Interface").join("AddOns")
}

/// Scans an AddOns folder against a flavor's TOC chain ([`flavor::toc_chain`]). A missing folder scans
/// to nothing; files in it are ignored; an addon folder the scan cannot read is skipped with a warning.
pub fn scan_addons(dir: &Path, chain: &[Option<TocSuffix>]) -> Result<Addons> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Addons::default());
        }
        Err(source) => {
            return Err(Error::Io {
                context: "reading the AddOns folder",
                path: dir.to_path_buf(),
                source,
            });
        }
    };

    let mut result = Addons::default();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            context: "reading the AddOns folder",
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let folder = entry.file_name().to_string_lossy().into_owned();
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let linked = metadata.file_type().is_symlink();
        if !linked && !metadata.is_dir() {
            continue;
        }
        match read_addon(&path, folder.clone(), linked, chain) {
            Ok(Some(addon)) => result.addons.push(addon),
            Ok(None) => result.skipped.push(folder),
            Err(reason) => {
                result
                    .warnings
                    .push(format!("skipped {}: {reason}", path.display()));
                result.skipped.push(folder);
            }
        }
    }
    result
        .addons
        .sort_by_key(|addon| addon.folder.to_ascii_lowercase());
    result.skipped.sort_by_key(|name| name.to_ascii_lowercase());
    Ok(result)
}

/// One addon folder; `Ok(None)` when it holds no TOC of its own; `Err` with the plain reason when it
/// cannot be read (a dangling junction, a permission problem).
fn read_addon(
    path: &Path,
    folder: String,
    linked: bool,
    chain: &[Option<TocSuffix>],
) -> std::result::Result<Option<Addon>, String> {
    let entries = std::fs::read_dir(path).map_err(|source| source.to_string())?;
    let mut tocs = Vec::new();
    let mut other_tocs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| source.to_string())?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = toc::classify_toc_name(&folder, &file_name) else {
            continue;
        };
        let (suffix, legacy) = match name {
            TocName::Base => (None, false),
            TocName::Suffix(suffix) => (Some(suffix), false),
            TocName::Legacy(suffix) => (Some(suffix), true),
            TocName::Other => {
                other_tocs.push(file_name);
                continue;
            }
        };
        let bytes = std::fs::read(entry.path())
            .map_err(|source| format!("{}: {source}", entry.path().display()))?;
        tocs.push(TocFile {
            file: file_name,
            suffix,
            legacy,
            toc: toc::parse_toc(&String::from_utf8_lossy(&bytes)),
        });
    }
    if tocs.is_empty() {
        return Ok(None);
    }
    tocs.sort_by_key(|file| file.file.to_ascii_lowercase());
    other_tocs.sort_by_key(|name| name.to_ascii_lowercase());
    let chosen = chain.iter().find_map(|wanted| {
        tocs.iter()
            .find(|file| !file.legacy && file.suffix == *wanted)
            .map(|file| file.file.clone())
    });
    Ok(Some(Addon {
        folder,
        path: path.to_path_buf(),
        tocs,
        other_tocs,
        chosen,
        linked,
    }))
}

/// Scans a flavor's `Interface\AddOns` folder.
pub fn scan_flavor(flavor: &Flavor) -> Result<Scan> {
    let addons = scan_addons(&addons_dir(&flavor.folder), &flavor.toc_chain())?;
    Ok(Scan {
        flavor: flavor.clone(),
        addons: addons.addons,
        skipped: addons.skipped,
        warnings: addons.warnings,
    })
}

/// Reads a flavor folder and scans its addons.
pub fn scan(folder: &Path) -> Result<Scan> {
    let flavor = flavor::read_flavor(folder)?;
    scan_flavor(&flavor)
}
