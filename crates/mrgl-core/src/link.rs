//! Dev addon linking (task 005): the addons one develops are loaded from their repo, not copied. `link`
//! makes a directory junction under `Interface\AddOns` for every subfolder of a development folder that
//! has a TOC named after it (the same rule as the scan, [`crate::scan::scan_addons`]), and records each as
//! a sidecar entry with the source `link:<development folder>` ([`source_of`]). A folder that is itself an
//! addon (a TOC named after it, task 062) is linked as that one addon, with the source `link:<that
//! folder>`; the addon shape wins over its subfolders, which are its own. `unlink` removes exactly
//! those junctions and their entries, never a real folder and never a junction mrgl did not make. A
//! junction needs no admin rights, unlike a symbolic link, and is made here, as a mount-point reparse
//! point through the Windows API, never by running `mklink`.
//!
//! The install engine refuses to install over a linked addon or over a junction it did not make
//! ([`Engine::refuse_links`]), and `remove` refuses a linked addon: `unlink` is the way out. A name that
//! already exists under `AddOns` as a real folder, or as a junction somewhere else (a dangling one too), is
//! skipped with the reason, never replaced and never followed. On a target without junctions the two
//! commands answer [`Error::JunctionsWindowsOnly`].

use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::client::strip_verbatim;
use crate::error::{Error, Result};
use crate::flavor::FlavorId;
use crate::install::Engine;
use crate::scan;
use crate::state::{self, Entry, State};

/// The prefix of a linked addon's source in the sidecar: `link:<development folder>`.
pub const SOURCE_PREFIX: &str = "link:";

/// One addon linked from the development folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkedAddon {
    /// The folder name, under `AddOns` and in the development folder; the sidecar's key.
    pub name: String,
    /// The TOC's `## Version`, or `unknown`.
    pub version: String,
    /// The folder the junction points to.
    pub target: PathBuf,
    /// The junction was there already, pointing to the same folder (an earlier `link`, or one made by
    /// hand to the same target, which is taken over).
    pub already_linked: bool,
}

/// An addon of the development folder that was not linked, or a link that `unlink` left in place, with
/// the plain reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// What `link` was given (task 062).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// A folder whose subfolders are addons, each with a TOC named after it.
    DevelopmentFolder,
    /// A folder that is itself an addon: a TOC named after it, the multi-TOC suffixes included.
    AddonFolder,
}

/// The shape of the folder `link` is given: an addon folder when it has a TOC named after it (the flavor's
/// chain picks nothing here, any base or suffix TOC counts, as in the scan), a development folder
/// otherwise. A folder that cannot be read is a development folder, so `link` reports the reason.
pub fn shape_of(folder: &Path) -> Shape {
    let addon = dev_folder(folder)
        .ok()
        .and_then(|folder| scan::read_addon_folder(&folder, &[]));
    match addon {
        Some(_) => Shape::AddonFolder,
        None => Shape::DevelopmentFolder,
    }
}

/// What `link` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Linked {
    /// The development folder or the addon folder, absolute.
    pub folder: PathBuf,
    /// Whether `folder` is a development folder or one addon folder.
    pub shape: Shape,
    /// The source recorded for every linked addon: `link:<folder>`.
    pub source: String,
    /// The addons linked, by name.
    pub linked: Vec<LinkedAddon>,
    /// The addons not linked: a real folder or another junction in the way, a name managed already.
    pub skipped: Vec<Skipped>,
    /// Subfolders of the development folder without a TOC named after them, by name; empty for an addon
    /// folder.
    pub without_toc: Vec<String>,
    /// Plain lines about what went on around the link: a rebuilt sidecar, entries dropped.
    pub warnings: Vec<String>,
}

/// What `unlink` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unlinked {
    /// The development folder or the addon folder, absolute.
    pub folder: PathBuf,
    /// The source whose entries were looked for: `link:<folder>`.
    pub source: String,
    /// The addons whose junctions were removed (or were gone already), by name.
    pub unlinked: Vec<String>,
    /// The entries whose folder is not a junction into the development folder any more: left in place.
    pub kept: Vec<Skipped>,
    /// Plain lines about what went on around the unlink.
    pub warnings: Vec<String>,
}

/// `Ok(())` where junctions exist (Windows), [`Error::JunctionsWindowsOnly`] elsewhere. `link` and
/// `unlink` call it first, so the answer comes before anything else is read.
pub fn ensure_supported() -> Result<()> {
    if cfg!(windows) {
        Ok(())
    } else {
        Err(Error::JunctionsWindowsOnly)
    }
}

/// The sidecar source of the addons linked from a development folder: `link:<folder>`.
pub fn source_of(folder: &Path) -> String {
    format!("{SOURCE_PREFIX}{}", folder.display())
}

/// The development folder of a `link:` source; `None` for every other source.
pub fn linked_folder(source: &str) -> Option<&str> {
    source.strip_prefix(SOURCE_PREFIX)
}

/// Whether a path is a junction or a symbolic link (the reparse points the scan reports as linked),
/// without following it.
pub fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// Where a junction or a symbolic link points, as the link records it, without the `\\?\` prefix.
pub fn link_target(path: &Path) -> io::Result<PathBuf> {
    std::fs::read_link(path).map(|target| strip_verbatim(&target))
}

/// Makes a directory junction at `link` (which must not exist) pointing to `target` (an absolute folder
/// on a local drive). Windows only; elsewhere an `Unsupported` error.
pub fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    system::create_junction(link, target)
}

/// The development folder as the sidecar spells it: canonical, without the `\\?\` prefix. It has to
/// exist (`link`); [`dev_folder_lenient`] takes one that is gone (`unlink`).
pub fn dev_folder(folder: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(folder).map_err(|source| Error::Io {
        context: "reading the development folder",
        path: folder.to_path_buf(),
        source,
    })?;
    Ok(strip_verbatim(&canonical))
}

/// The development folder as [`dev_folder`] spells it, also when it is gone (deleted after the link): the
/// nearest folder above it that still exists is canonicalized and the missing names are appended, so an
/// 8.3 short name or another spelling of a parent (`C:\Users\RUNNER~1\…`) still matches the recorded
/// source.
pub fn dev_folder_lenient(folder: &Path) -> PathBuf {
    let absolute = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    let mut missing = Vec::new();
    let mut current = absolute.as_path();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(current) {
            let mut path = strip_verbatim(&canonical);
            path.extend(missing.iter().rev());
            return path;
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                current = parent;
            }
            _ => return strip_verbatim(&absolute),
        }
    }
}

/// Whether two paths name the same folder, component by component, case-insensitively on Windows.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let a: Vec<Component<'_>> = a.components().collect();
    let b: Vec<Component<'_>> = b.components().collect();
    a.len() == b.len()
        && a.iter()
            .zip(&b)
            .all(|(a, b)| same_component(a.as_os_str(), b.as_os_str()))
}

/// Whether `path` lies under `folder` (strictly), component by component, case-insensitively on Windows.
fn is_under(path: &Path, folder: &Path) -> bool {
    let mut path = path.components();
    for wanted in folder.components() {
        match path.next() {
            Some(found) if same_component(found.as_os_str(), wanted.as_os_str()) => {}
            _ => return false,
        }
    }
    path.next().is_some()
}

fn same_component(a: &std::ffi::OsStr, b: &std::ffi::OsStr) -> bool {
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Whether a sidecar entry was linked from this development folder.
fn linked_from(entry: &Entry, folder: &Path) -> bool {
    linked_folder(&entry.source).is_some_and(|known| same_path(Path::new(known), folder))
}

/// Whether the link at `path` points to `target`: both resolved when they can be, as written otherwise
/// (a dangling link).
fn points_to(path: &Path, target: &Path) -> bool {
    if let (Ok(resolved), Ok(wanted)) = (std::fs::canonicalize(path), std::fs::canonicalize(target))
    {
        return resolved == wanted;
    }
    link_target(path).is_ok_and(|found| same_path(&found, target))
}

/// A link as a message names it: where it points, and whether that is gone (a dangling junction is
/// reported, never followed).
fn describe_link(path: &Path) -> String {
    match link_target(path) {
        Ok(target) if target.exists() => format!("a junction to {}", target.display()),
        Ok(target) => format!(
            "a dangling junction to {} (its target is gone)",
            target.display()
        ),
        Err(error) => format!("a junction whose target cannot be read ({error})"),
    }
}

/// The flavor as a message names it: the friendly name, else the id.
fn flavor_label(id: &FlavorId) -> String {
    id.friendly_name().unwrap_or(id.as_str()).to_string()
}

impl Engine {
    /// The install engine's guard (task 005): an install whose folders (the staged ones and the ones it
    /// drops, task 067) include an addon `link` made, or a junction or a symbolic link that mrgl did not
    /// make, is refused before anything moves. A folder two entries list (a dropped folder that a link
    /// entry names too) is the linked addon when one of them is.
    pub fn refuse_links(&self, state: &State, folders: &[String]) -> Result<()> {
        let flavor = flavor_label(&self.flavor.id);
        for folder in folders {
            let linked = state.addons.iter().find_map(|(name, entry)| {
                let listed = entry
                    .folders
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(folder));
                listed
                    .then(|| linked_folder(&entry.source))
                    .flatten()
                    .map(|dev| (name, dev))
            });
            if let Some((name, dev)) = linked {
                return Err(Error::LinkedAddon {
                    action: "install over",
                    name: name.to_string(),
                    flavor,
                    folder: dev.to_string(),
                });
            }
            let path = self.addons_dir().join(folder);
            if is_link(&path) {
                return Err(Error::ForeignJunction {
                    folder: folder.clone(),
                    flavor,
                    target: link_target(&path)
                        .map(|target| target.display().to_string())
                        .unwrap_or_else(|error| format!("an unreadable target ({error})")),
                });
            }
        }
        Ok(())
    }

    /// `remove`'s guard (task 005): a linked addon is unlinked, never removed, so its development folder
    /// is never reached.
    pub fn refuse_remove_linked(&self, name: &str, entry: &Entry) -> Result<()> {
        match linked_folder(&entry.source) {
            Some(dev) => Err(Error::LinkedAddon {
                action: "remove",
                name: name.to_string(),
                flavor: flavor_label(&self.flavor.id),
                folder: dev.to_string(),
            }),
            None => Ok(()),
        }
    }
}

/// Links every addon of a development folder into the engine's flavor: a junction under
/// `Interface\AddOns` per subfolder with a TOC named after it, and a sidecar entry with the source
/// `link:<folder>`, the TOC's version (or `unknown`) and no file list (the files are the developer's and
/// change all the time). A folder that is itself an addon (task 062, [`shape_of`]) is linked as that one
/// addon, under its own name and with the source `link:<folder>`. A name in the way (a real folder,
/// another junction, an addon managed from another source) is skipped with its reason; a junction already
/// pointing to the same folder is kept and recorded. Running it again links what is new and changes
/// nothing else.
pub fn link(engine: &Engine, folder: &Path) -> Result<Linked> {
    ensure_supported()?;
    let dev = dev_folder(folder)?;
    let source = source_of(&dev);
    let chain = engine.flavor.toc_chain();
    let single = if dev.is_dir() {
        scan::read_addon_folder(&dev, &chain)
    } else {
        None
    };
    let shape = if single.is_some() {
        Shape::AddonFolder
    } else {
        Shape::DevelopmentFolder
    };
    let found = match single {
        Some(addon) => scan::Addons {
            addons: vec![addon],
            ..scan::Addons::default()
        },
        None if dev.is_dir() => scan::scan_addons(&dev, &chain)?,
        None => scan::Addons::default(),
    };
    if found.addons.is_empty() {
        return Err(Error::NoAddonInDevFolder { path: dev });
    }

    let loaded = engine.load_state()?;
    let mut state = loaded.state;
    let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
    let current = scan::scan_flavor(&engine.flavor)?;
    let dropped = state.reconcile(&current);
    warnings.extend(dropped.iter().map(|name| {
        format!("{name} was deleted by hand since its install and is no longer managed")
    }));
    warnings.extend(found.warnings.iter().cloned());

    let addons_dir = engine.addons_dir();
    std::fs::create_dir_all(&addons_dir).map_err(|source| Error::Io {
        context: "creating the AddOns folder",
        path: addons_dir.clone(),
        source,
    })?;

    let mut linked = Vec::new();
    let mut skipped = Vec::new();
    let mut forgotten = false;
    for addon in &found.addons {
        let name = addon.folder.clone();
        let target = addon.path.clone();
        let path = addons_dir.join(&name);
        let known = state
            .entry(&name)
            .or_else(|| state.entry_for_folder(&name))
            .map(|(known, entry)| (known.to_string(), entry.clone()));
        if let Some((known, entry)) = &known
            && !linked_from(entry, &dev)
        {
            skipped.push(Skipped {
                name,
                reason: format!("{known} is managed already, from {}", entry.source),
            });
            continue;
        }
        let already_linked = match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                if !points_to(&path, &target) {
                    skipped.push(Skipped {
                        reason: format!("{} is in the way and stays", describe_link(&path)),
                        name,
                    });
                    continue;
                }
                true
            }
            Ok(_) => {
                // An entry of this folder whose junction was replaced by a real folder by hand is
                // forgotten: the folder is not linked any more, and an install may take it over.
                let mut reason =
                    "a folder of that name is in AddOns already and stays (not a junction)"
                        .to_string();
                if let Some((known, _)) = &known {
                    state.addons.remove(known);
                    forgotten = true;
                    reason.push_str("; its link entry is forgotten");
                }
                skipped.push(Skipped { name, reason });
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Err(error) = create_junction(&path, &target) {
                    skipped.push(Skipped {
                        name,
                        reason: format!("the junction could not be made: {error}"),
                    });
                    continue;
                }
                false
            }
            Err(error) => {
                skipped.push(Skipped {
                    name,
                    reason: format!("{}: {error}", path.display()),
                });
                continue;
            }
        };
        let version = addon
            .version()
            .map(str::trim)
            .filter(|version| !version.is_empty())
            .unwrap_or("unknown")
            .to_string();
        let (key, installed_at) = match known {
            Some((known, entry)) => (known, entry.installed_at),
            None => (name.clone(), state::now_rfc3339()),
        };
        state.addons.insert(
            key,
            Entry {
                source: source.clone(),
                version: version.clone(),
                previous_version: None,
                folders: vec![name.clone()],
                files: BTreeMap::new(),
                installed_at,
                nolib: None,
                pinned: None,
                ignored: false,
                channel: crate::sources::Channel::Stable,
            },
        );
        linked.push(LinkedAddon {
            name,
            version,
            target,
            already_linked,
        });
    }
    if !linked.is_empty() || !dropped.is_empty() || forgotten {
        engine.home.write_state(&state)?;
    }
    Ok(Linked {
        folder: dev,
        shape,
        source,
        linked,
        skipped,
        without_toc: found.skipped,
        warnings,
    })
}

/// Removes the junctions `link` made from a development folder or an addon folder and their sidecar
/// entries: an entry's folder that is a junction into the development folder (or to the addon folder
/// itself, task 062) is removed (the junction itself, nothing it points to), one that is gone already is
/// forgotten, a real folder is left in place and forgotten (it is
/// not linked any more), and a junction pointing elsewhere is left in place with its entry. The
/// development folder itself may be gone.
pub fn unlink(engine: &Engine, folder: &Path) -> Result<Unlinked> {
    ensure_supported()?;
    let dev = dev_folder_lenient(folder);
    let source = source_of(&dev);
    let loaded = engine.load_state()?;
    let mut state = loaded.state;
    let mut warnings: Vec<String> = loaded.warning.into_iter().collect();
    let names: Vec<(String, Entry)> = state
        .addons
        .iter()
        .filter(|(_, entry)| linked_from(entry, &dev))
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .collect();

    let addons_dir = engine.addons_dir();
    let mut unlinked = Vec::new();
    let mut kept = Vec::new();
    for (name, entry) in names {
        let mut keep_entry = false;
        let mut kept_reason = None;
        for folder in &entry.folders {
            let path = addons_dir.join(folder);
            match std::fs::symlink_metadata(&path) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    warnings.push(format!("{folder} was gone from AddOns already"));
                }
                Err(error) => {
                    keep_entry = true;
                    kept_reason = Some(format!("{}: {error}", path.display()));
                }
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    let ours = link_target(&path)
                        .is_ok_and(|target| is_under(&target, &dev) || same_path(&target, &dev));
                    if !ours {
                        keep_entry = true;
                        kept_reason = Some(format!(
                            "{folder} is {} now, not into {}; left in place",
                            describe_link(&path),
                            dev.display()
                        ));
                        continue;
                    }
                    if let Err(error) = std::fs::remove_dir(&path) {
                        keep_entry = true;
                        kept_reason = Some(format!("the junction could not be removed: {error}"));
                    }
                }
                Ok(_) => {
                    kept_reason = Some(format!(
                        "{folder} is a real folder now, not a junction; left in place and no longer managed"
                    ));
                }
            }
        }
        match kept_reason {
            Some(reason) => {
                if !keep_entry {
                    state.addons.remove(&name);
                }
                kept.push(Skipped { name, reason });
            }
            None => {
                state.addons.remove(&name);
                unlinked.push(name);
            }
        }
    }
    if !unlinked.is_empty() || !kept.is_empty() {
        engine.home.write_state(&state)?;
    }
    Ok(Unlinked {
        folder: dev,
        source,
        unlinked,
        kept,
        warnings,
    })
}

#[cfg(windows)]
mod system {
    //! A junction is a directory with a mount-point reparse point: an empty folder is created, opened with
    //! `FILE_FLAG_OPEN_REPARSE_POINT`, and one `DeviceIoControl` with `FSCTL_SET_REPARSE_POINT` writes the
    //! reparse data (the NT path `\??\C:\…` as the substitute name, the Win32 path as the print name),
    //! as `mklink /J` does. No admin rights are needed. The folder is removed again when the call fails.

    use std::fs::OpenOptions;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT;

    /// `IO_REPARSE_TAG_MOUNT_POINT` (winnt.h); written out so the large `SystemServices` bindings stay
    /// out of the build for one constant.
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    /// The mount-point header after the tag, the length and the reserved field: four `u16` offsets and
    /// lengths of the two names.
    const MOUNT_POINT_HEADER: usize = 8;

    /// The `REPARSE_DATA_BUFFER` of a mount point to `target`, as bytes.
    pub(super) fn reparse_buffer(target: &Path) -> io::Result<Vec<u8>> {
        let print: Vec<u16> = target.as_os_str().encode_wide().collect();
        let substitute: Vec<u16> = "\\??\\"
            .encode_utf16()
            .chain(print.iter().copied())
            .collect();
        let names = (substitute.len() + 1 + print.len() + 1) * 2;
        let data_length = MOUNT_POINT_HEADER + names;
        if 8 + data_length > MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the target path is too long for a junction",
            ));
        }
        let unit = |value: usize| (value as u16).to_le_bytes();
        let mut buffer = Vec::with_capacity(8 + data_length);
        buffer.extend(IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buffer.extend(unit(data_length));
        buffer.extend(0u16.to_le_bytes());
        buffer.extend(unit(0));
        buffer.extend(unit(substitute.len() * 2));
        buffer.extend(unit((substitute.len() + 1) * 2));
        buffer.extend(unit(print.len() * 2));
        for code in substitute
            .iter()
            .chain([&0])
            .chain(print.iter())
            .chain([&0])
        {
            buffer.extend(code.to_le_bytes());
        }
        Ok(buffer)
    }

    pub(super) fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
        if !target.is_absolute() || target.as_os_str().to_string_lossy().starts_with("\\\\") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a junction points to a folder on a local drive, by its full path",
            ));
        }
        let buffer = reparse_buffer(target)?;
        std::fs::create_dir(link)?;
        let set = || -> io::Result<()> {
            let folder = OpenOptions::new()
                .write(true)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(link)?;
            let mut returned = 0u32;
            // SAFETY: the handle is an open directory handle with write access, the input buffer is
            // `buffer.len()` bytes of a well-formed mount-point REPARSE_DATA_BUFFER, there is no output
            // buffer, and the call is synchronous (no OVERLAPPED).
            let ok = unsafe {
                DeviceIoControl(
                    folder.as_raw_handle(),
                    FSCTL_SET_REPARSE_POINT,
                    buffer.as_ptr().cast(),
                    buffer.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        };
        let result = set();
        if result.is_err() {
            let _ = std::fs::remove_dir(link);
        }
        result
    }
}

#[cfg(not(windows))]
mod system {
    //! No junctions outside Windows: `link` and `unlink` answer [`crate::Error::JunctionsWindowsOnly`]
    //! before they get here.

    use std::io;
    use std::path::Path;

    pub(super) fn create_junction(_link: &Path, _target: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "junctions are Windows-only",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_source_names_its_folder() {
        let source = source_of(Path::new("C:\\dev\\addons"));
        assert_eq!(source, "link:C:\\dev\\addons");
        assert_eq!(linked_folder(&source), Some("C:\\dev\\addons"));
        assert_eq!(linked_folder("github:someone/something"), None);
    }

    #[test]
    fn paths_compare_by_component() {
        assert!(same_path(
            Path::new("/dev/addons"),
            Path::new("/dev/addons/")
        ));
        assert!(!same_path(
            Path::new("/dev/addons"),
            Path::new("/dev/addon")
        ));
        assert!(is_under(
            Path::new("/dev/addons/Foo"),
            Path::new("/dev/addons")
        ));
        assert!(!is_under(
            Path::new("/dev/addons"),
            Path::new("/dev/addons")
        ));
        assert!(!is_under(
            Path::new("/dev/addonsX/Foo"),
            Path::new("/dev/addons")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_compare_case_insensitively() {
        assert!(same_path(
            Path::new("C:\\Dev\\Addons"),
            Path::new("c:\\dev\\addons")
        ));
        assert!(is_under(
            Path::new("c:\\DEV\\addons\\Foo"),
            Path::new("C:\\dev\\Addons")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn the_reparse_buffer_is_a_mount_point_with_both_names() {
        let buffer = system::reparse_buffer(Path::new("C:\\dev\\Foo")).expect("a buffer");
        let word = |at: usize| u16::from_le_bytes([buffer[at], buffer[at + 1]]) as usize;
        assert_eq!(
            u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]),
            0xA000_0003
        );
        // `\??\C:\dev\Foo` is 14 units, `C:\dev\Foo` 10, each with its NUL.
        assert_eq!(word(4), 8 + (15 + 11) * 2, "the data length");
        assert_eq!((word(8), word(10)), (0, 28), "the substitute name");
        assert_eq!((word(12), word(14)), (30, 20), "the print name");
        assert_eq!(buffer.len(), 8 + word(4));
        let names: Vec<u16> = buffer[16..]
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(
            String::from_utf16_lossy(&names),
            "\\??\\C:\\dev\\Foo\0C:\\dev\\Foo\0"
        );
    }
}
