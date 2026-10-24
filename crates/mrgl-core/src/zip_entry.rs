//! The one check every zip entry passes before mrgl writes it (task 044): the install engine's unpack into
//! its staging folder ([`crate::install`], task 004) and the `WTF` restore ([`crate::backup`], task 013) both
//! call [`checked_entries`] and write only what it hands back, so the rule that keeps a zip from writing
//! outside its folder lives here and nowhere else.
//!
//! An entry passes when it can be opened for reading, its name (read as UTF-8, else as CP437) is a relative
//! path under the target folder and it is not a symbolic link. A name fails on an absolute path, a drive
//! letter, a `..` component, a component with a character Windows refuses (a control character,
//! `:<>"|?*`), a component ending in a dot or a space (Windows drops it, task 124) or a reserved device
//! name (`CON`, `NUL`, `COM1` … `LPT9`, with or without an extension); `\` counts as a separator, `.` and
//! empty components are dropped, and a name with no component left (`./`) is skipped. A zip of more than
//! [`MAX_ENTRIES`] entries is refused before any entry is read (task 124). A zip that fails refuses as a whole: [`Error::ZipEntryEscapes`] for a name or a link,
//! [`Error::ZipUnreadable`] for a zip or an entry that cannot be read, each naming the zip by its
//! [`ZipRole`].
//!
//! The engine's `safe_relative_path` and the restore's `safe_entry_path` were the same name rule, word for
//! word, and both refused a symbolic link. Where their callers differed, the stricter rule won:
//!
//! - **When the entries are checked.** The engine checked each entry as it unpacked it, so a zip refused at
//!   its fifth entry had already written four into staging; the restore checked every entry before
//!   anything moved. Now every entry is checked before the caller writes the first one.
//! - **How an entry is opened.** The engine opened every entry for reading, so an encrypted entry or a
//!   compression method this build does not read made the zip unreadable; the restore only read the raw
//!   entry, so such an entry passed the check and failed half way through the unpack, after `WTF` had
//!   moved aside. Now the check opens every entry for reading: an entry mrgl cannot unpack makes the zip
//!   unreadable before anything moves.
//!
//! The check also bounds what a zip unpacks to (task 039), so a zip bomb from a compromised source cannot
//! fill the disk from the staging folder or from `WTF`. The bound is [`MAX_UNPACKED`] (2 GiB) unless the
//! caller raises it (`--max-unpacked <size>` on `add`, `rollback` and `backup --restore`), and it holds
//! twice: [`checked_entries`] sums the uncompressed sizes the file entries declare and refuses the zip
//! before anything is written when the sum passes the bound, and the caller reads every entry it writes
//! through [`Unpacked::entry`], which counts the bytes and refuses once they pass the bound or once an
//! entry outgrows the size it declared (a zip that lies about its sizes). The zip crate itself stops an
//! entry at its declared size, and that refusal is reported as the same lie. Either way the error is
//! [`Error::ZipTooLarge`], naming the zip, the bound and, for a refusal while writing, the bytes written;
//! the caller removes what was written (the engine its staging folder, the restore its half-written `WTF`).
//!
//! The one folder-name rule (task 123) lives here too, next to the entry rule whose character and device
//! name checks it shares: [`is_folder_name`] says whether a name is one folder (or file) right under a
//! folder Windows will create as written. The install engine holds every staged addon folder and every
//! folder the sidecar records to it before it joins the name to `Interface\AddOns`, and the GitHub mapping
//! of adopt, the shareable list file and the set apply's undo record hold their names to it.

use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::{Error, Result};

/// The bound on what one zip unpacks to unless the caller raises it (task 039): 2 GiB, far above any addon
/// or `WTF` folder, far below a full disk.
pub const MAX_UNPACKED: u64 = 2 * 1024 * 1024 * 1024;

/// The bound on the entries of one zip (task 124): 100,000, some thirty times the files of the largest
/// addon (a few thousand) and five times a `WTF` folder of fifty characters with two hundred addons'
/// SavedVariables each, so a zip of a few hundred thousand empty files cannot fill `Interface\AddOns`, which
/// the client and the scan read on every start.
pub const MAX_ENTRIES: usize = 100_000;

/// What a zip is to its caller: the noun its unreadable message names and the folder its entries must stay
/// under. The messages of the two commands are written once, in [`Error`], with these words in them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipRole {
    /// An addon zip the install engine unpacks into its staging folder (task 004).
    Addon,
    /// A `WTF` backup a restore unpacks under `WTF` (task 013).
    Backup,
}

impl ZipRole {
    /// The zip in an unreadable message: `not a readable zip`, `not a readable backup`.
    pub fn noun(self) -> &'static str {
        match self {
            ZipRole::Addon => "zip",
            ZipRole::Backup => "backup",
        }
    }

    /// The folder an escaping entry would land outside of.
    pub fn folder(self) -> &'static str {
        match self {
            ZipRole::Addon => "the staging folder",
            ZipRole::Backup => "WTF",
        }
    }
}

/// How a zip passed the bound on what it unpacks to (task 039): the clause [`Error::ZipTooLarge`] prints
/// after the zip's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Excess {
    /// Its file entries declare more than the bound together, found before anything was written.
    Declared { declared: u64, limit: u64 },
    /// What its entries wrote passed the bound while every entry kept to its declared size.
    Written { written: u64, limit: u64 },
    /// An entry unpacked to more than it declares, so the sum the bound was checked against is a lie.
    Entry {
        entry: String,
        declared: u64,
        written: u64,
        limit: u64,
    },
    /// It holds more entries than [`MAX_ENTRIES`], found before any entry was read (task 124).
    Entries { entries: usize, limit: usize },
}

impl std::fmt::Display for Excess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Excess::Declared { declared, limit } => write!(
                formatter,
                "it declares {} unpacked, more than the bound of {}; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you trust",
                size_text(*declared),
                size_text(*limit)
            ),
            Excess::Written { written, limit } => write!(
                formatter,
                "it unpacked past the bound of {} ({} written, then removed)",
                size_text(*limit),
                size_text(*written)
            ),
            Excess::Entry {
                entry,
                declared,
                written,
                limit,
            } => write!(
                formatter,
                "the entry `{entry}` unpacks to more than the {} it declares, so the bound of {} cannot hold ({} written, then removed)",
                size_text(*declared),
                size_text(*limit),
                size_text(*written)
            ),
            Excess::Entries { entries, limit } => write!(
                formatter,
                "it holds {entries} entries, more than the bound of {limit} for one zip; nothing was written"
            ),
        }
    }
}

/// One entry that passed the check: its index in the zip, its path components under the target folder
/// (never empty), whether it is a folder and the uncompressed size it declares (task 039).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedEntry {
    pub index: usize,
    pub parts: Vec<String>,
    pub is_dir: bool,
    pub size: u64,
}

/// Checks every entry of an open zip before anything is written and returns the ones to write, in the
/// zip's order. The first entry that fails refuses the whole zip: [`Error::ZipEntryEscapes`] for a name
/// outside the target folder or a symbolic link, [`Error::ZipUnreadable`] for an entry that cannot be
/// opened for reading or whose name cannot be read. Once every entry passed, the uncompressed sizes the
/// file entries declare are summed and a sum past `max_unpacked` refuses the zip with
/// [`Error::ZipTooLarge`] (task 039). Before any of that, a zip of more than [`MAX_ENTRIES`] entries is
/// refused with the same error (task 124). `zip` is the path the errors name.
pub fn checked_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    zip: &Path,
    role: ZipRole,
    max_unpacked: u64,
) -> Result<Vec<CheckedEntry>> {
    checked_entries_within(archive, zip, role, max_unpacked, MAX_ENTRIES)
}

/// [`checked_entries`] with the bound on the entries given, so a test needs no zip of 100,000 entries.
fn checked_entries_within<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    zip: &Path,
    role: ZipRole,
    max_unpacked: u64,
    max_entries: usize,
) -> Result<Vec<CheckedEntry>> {
    if archive.len() > max_entries {
        return Err(Error::ZipTooLarge {
            role,
            path: zip.to_path_buf(),
            excess: Excess::Entries {
                entries: archive.len(),
                limit: max_entries,
            },
        });
    }
    let unreadable = |reason: String| Error::ZipUnreadable {
        role,
        path: zip.to_path_buf(),
        reason,
    };
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|reason| unreadable(format!("entry {index}: {reason}")))?;
        let raw = entry
            .name()
            // coverage: os-begin — `name` reads a non-UTF-8 name as CP437, which has every byte
            .map_err(|reason| unreadable(format!("entry {index}: {reason}")))?
            .into_owned();
        // coverage: os-end
        let escapes = || Error::ZipEntryEscapes {
            role,
            path: zip.to_path_buf(),
            entry: raw.clone(),
        };
        let parts = entry_path(&raw).ok_or_else(escapes)?;
        if entry.is_symlink() {
            return Err(escapes());
        }
        if parts.is_empty() {
            continue;
        }
        let is_dir = entry.is_dir();
        entries.push(CheckedEntry {
            index,
            parts,
            is_dir,
            size: if is_dir { 0 } else { entry.size() },
        });
    }
    let declared = entries
        .iter()
        .fold(0u64, |sum, entry| sum.saturating_add(entry.size));
    if declared > max_unpacked {
        return Err(Error::ZipTooLarge {
            role,
            path: zip.to_path_buf(),
            excess: Excess::Declared {
                declared,
                limit: max_unpacked,
            },
        });
    }
    Ok(entries)
}

/// The bytes a zip's entries have unpacked to so far, against the bound (task 039): the caller reads every
/// entry it writes through [`Unpacked::entry`].
#[derive(Debug)]
pub struct Unpacked {
    zip: PathBuf,
    role: ZipRole,
    limit: u64,
    written: u64,
}

impl Unpacked {
    /// Nothing unpacked yet from `zip`, with `limit` as the bound (the one [`checked_entries`] was given).
    pub fn new(zip: &Path, role: ZipRole, limit: u64) -> Unpacked {
        Unpacked {
            zip: zip.to_path_buf(),
            role,
            limit,
            written: 0,
        }
    }

    /// The bytes read through [`Unpacked::entry`] so far.
    pub fn written(&self) -> u64 {
        self.written
    }

    /// A reader over one checked entry's content that counts what it hands over. A read that would take
    /// the zip past the bound or the entry past its declared size fails with an I/O error that carries
    /// [`Error::ZipTooLarge`]; [`refusal`] gets it back out. The zip crate's own refusal of an entry
    /// that outgrows its declared size becomes the same error.
    pub fn entry<'a, R: Read>(
        &'a mut self,
        checked: &'a CheckedEntry,
        reader: R,
    ) -> Counted<'a, R> {
        Counted {
            unpacked: self,
            checked,
            reader,
            read: 0,
        }
    }

    fn refuse(&self, excess: Excess) -> io::Error {
        io::Error::other(Refusal {
            role: self.role,
            path: self.zip.clone(),
            excess,
        })
    }
}

/// One entry's content read through [`Unpacked::entry`].
#[derive(Debug)]
pub struct Counted<'a, R> {
    unpacked: &'a mut Unpacked,
    checked: &'a CheckedEntry,
    reader: R,
    read: u64,
}

impl<R> Counted<'_, R> {
    fn entry_lies(&self) -> io::Error {
        self.unpacked.refuse(Excess::Entry {
            entry: self.checked.parts.join("/"),
            declared: self.checked.size,
            written: self.unpacked.written,
            limit: self.unpacked.limit,
        })
    }
}

impl<R: Read> Read for Counted<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = match self.reader.read(buf) {
            Ok(count) => count as u64,
            Err(error) if outgrew_its_declared_size(&error) => return Err(self.entry_lies()),
            Err(error) => return Err(error),
        };
        if self.read.saturating_add(count) > self.checked.size {
            return Err(self.entry_lies());
        }
        if self.unpacked.written.saturating_add(count) > self.unpacked.limit {
            return Err(self.unpacked.refuse(Excess::Written {
                written: self.unpacked.written,
                limit: self.unpacked.limit,
            }));
        }
        self.read += count;
        self.unpacked.written += count;
        Ok(count as usize)
    }
}

/// The zip crate's refusal of an entry that hands over more than its declared uncompressed size (zip 9:
/// the entry reader stops there with this `InvalidData` error). Matched by its text, so a crate update
/// that rewords it turns the lie back into a plain I/O failure, still refused, and the fixture test
/// `a_zip_that_lies_about_its_size_is_refused_while_writing` says so.
fn outgrew_its_declared_size(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::InvalidData
        && error
            .to_string()
            .contains("larger than its declared uncompressed size")
}

/// The bound's refusal as an I/O error from a [`Counted`] reader carries it: the fields of
/// [`Error::ZipTooLarge`], which it prints as and converts into; a caller that prints the clause alone
/// reads `excess`.
#[derive(Debug, Clone)]
pub struct Refusal {
    pub role: ZipRole,
    pub path: PathBuf,
    pub excess: Excess,
}

impl From<Refusal> for Error {
    fn from(refusal: Refusal) -> Error {
        Error::ZipTooLarge {
            role: refusal.role,
            path: refusal.path,
            excess: refusal.excess,
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Error::from(self.clone()).fmt(formatter)
    }
}

impl std::error::Error for Refusal {}

/// The bound's [`Refusal`] an I/O error from a [`Counted`] reader carries, or the I/O error as it was.
pub fn refusal(error: io::Error) -> std::result::Result<Refusal, io::Error> {
    error.downcast::<Refusal>()
}

/// A byte count for a human: `512 B`, `16.0 KB`, `2.0 GB`, in steps of 1024. The one size formatter of
/// the messages here, the CLI's listings and the doctor's report (task 071).
pub fn size_text(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = UNITS[0];
    for next in &UNITS[1..] {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    format!("{value:.1} {unit}")
}

/// A size as `--max-unpacked` takes it: a whole number of bytes, or one with `K`, `M`, `G` or `T` in
/// steps of 1024 (`KB` and `KiB` the same, any case): `3G`, `500M`, `1048576`. `None` for anything
/// else, zero and a size past `u64`.
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let digits = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(digits);
    let number: u64 = number.parse().ok()?;
    let shift = match unit.trim_start().to_ascii_lowercase().as_str() {
        "" | "b" => 0,
        "k" | "kb" | "kib" => 10,
        "m" | "mb" | "mib" => 20,
        "g" | "gb" | "gib" => 30,
        "t" | "tb" | "tib" => 40,
        _ => return None,
    };
    number.checked_mul(1u64 << shift).filter(|size| *size > 0)
}

/// A zip entry name as path components under the target folder, or `None` when it would land outside it
/// (the name rule of the module doc).
fn entry_path(name: &str) -> Option<Vec<String>> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in name.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            part => {
                if refused_component(part) {
                    return None;
                }
                parts.push(part.to_string());
            }
        }
    }
    Some(parts)
}

/// The one folder-name rule (task 123): whether a name is one folder or file right under a folder, as
/// Windows creates it as written. It fails on an empty name, `.` and `..`, a path separator (`/`, `\`),
/// a trailing dot or space (Windows drops them, so `AddOns\.` and `AddOns\ ` are `AddOns` itself), a
/// character Windows refuses (a control character, `:<>"|?*`, the entry rule's) and a reserved device
/// name (`CON`, `NUL`, `COM1` … `LPT9`, with or without an extension). A leading space and an inner dot
/// pass. `.` and `..` end in a dot.
pub fn is_folder_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && !refused_component(name)
}

/// One path component Windows will not create as written, the entry rule's and the folder-name rule's: a
/// trailing dot or space (Windows drops it, so `Foo.` and `Foo` would unpack into one folder and the
/// sidecar's file list would name a path that is not on disk; task 124 for an entry), or what
/// [`refused_by_windows`] refuses.
fn refused_component(part: &str) -> bool {
    part.ends_with(['.', ' ']) || refused_by_windows(part)
}

/// One path component Windows will not create: a control character, `:<>"|?*` or a reserved device
/// name.
fn refused_by_windows(part: &str) -> bool {
    part.chars()
        .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        || is_reserved_device_name(part)
}

/// `CON`, `NUL`, `COM1` … `LPT9`, with or without an extension: a file Windows will not create. The zip
/// cache's names escape them too ([`crate::state::cache_name`], task 124).
pub(crate) fn is_reserved_device_name(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit()
                && stem.as_bytes()[3] != b'0'
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::io::{Cursor, Write};
    use std::path::PathBuf;

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    /// A zip in memory with the given entries (name, content, encrypted); a name with a trailing `/` is a
    /// folder.
    fn zip_of(entries: &[(&str, &[u8], bool)]) -> ZipArchive<Cursor<Vec<u8>>> {
        use zip::unstable::write::FileOptionsExt;
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, content, encrypted) in entries {
            if name.ends_with('/') {
                writer.add_directory(*name, stored).expect("a folder entry");
                continue;
            }
            let options = if *encrypted {
                stored
                    .with_deprecated_encryption(b"password")
                    .expect("ZipCrypto")
            } else {
                stored
            };
            writer.start_file(*name, options).expect("a file entry");
            writer.write_all(content).expect("content");
        }
        let bytes = writer.finish().expect("finished").into_inner();
        ZipArchive::new(Cursor::new(bytes)).expect("readable")
    }

    fn strings(parts: &[&str]) -> Option<Vec<String>> {
        Some(parts.iter().map(|part| part.to_string()).collect())
    }

    #[test]
    fn a_name_is_components_under_the_target_folder() {
        assert_eq!(entry_path("Foo/core.lua"), strings(&["Foo", "core.lua"]));
        assert_eq!(
            entry_path("Foo\\sub\\x.lua"),
            strings(&["Foo", "sub", "x.lua"])
        );
        assert_eq!(entry_path("./Foo//./x.lua"), strings(&["Foo", "x.lua"]));
        assert_eq!(
            entry_path("Account\\NAME\\./AddOns.txt"),
            strings(&["Account", "NAME", "AddOns.txt"])
        );
        assert_eq!(entry_path("Foo/"), strings(&["Foo"]));
        assert_eq!(entry_path(""), strings(&[]));
        assert_eq!(entry_path("../x.lua"), None);
        assert_eq!(entry_path("Foo/../../x.lua"), None);
        assert_eq!(entry_path("/abs/x.lua"), None);
        assert_eq!(entry_path("\\abs\\x.lua"), None);
        assert_eq!(entry_path("C:/x.lua"), None);
        assert_eq!(entry_path("C:\\x.lua"), None);
        assert_eq!(entry_path("Foo/a?b.lua"), None);
        assert_eq!(entry_path("Account/bad<name>.lua"), None);
        assert_eq!(entry_path("Foo/a\u{7}b"), None);
        assert_eq!(entry_path("Foo/NUL.lua"), None);
        assert_eq!(entry_path("Account/NUL"), None);
        assert_eq!(entry_path("Foo/COM1"), None);
        assert_eq!(entry_path("Account/com1.txt"), None);
        assert_eq!(entry_path("Foo/COM0.lua"), strings(&["Foo", "COM0.lua"]));
        assert_eq!(entry_path("COM10"), strings(&["COM10"]));
        assert!(is_reserved_device_name("lpt9.txt"));
        assert!(!is_reserved_device_name("lpt10"));
        assert!(!is_reserved_device_name("COMA"));
        assert_eq!(
            entry_path("Foo/Console.lua"),
            strings(&["Foo", "Console.lua"])
        );
        // Task 124: a component ending in a dot or a space is renamed by Windows, so it is refused; a
        // leading space and an inner dot pass.
        for name in [
            "Foo./x.lua",
            "Foo /x.lua",
            "Foo/x.lua.",
            "Foo/x.lua ",
            "Foo/.../x.lua",
        ] {
            assert_eq!(entry_path(name), None, "{name:?}");
        }
        assert_eq!(
            entry_path(" Foo/.x.lua/a.b"),
            strings(&[" Foo", ".x.lua", "a.b"])
        );
    }

    #[test]
    fn a_zip_of_more_entries_than_the_bound_is_refused_before_any_is_read() {
        // Task 124: the count comes from the central directory; with a bound of two, three entries are one
        // too many and even an escaping entry among them is never looked at.
        let mut archive = zip_of(&[
            ("Many/Many.toc", b"## Title: Many", false),
            ("Many/a.lua", b"", false),
            ("Many/../../b.lua", b"", false),
        ]);
        let zip = PathBuf::from("many.zip");
        let error = checked_entries_within(&mut archive, &zip, ZipRole::Addon, MAX_UNPACKED, 2)
            .expect_err("refused");
        assert!(
            matches!(
                &error,
                Error::ZipTooLarge {
                    role: ZipRole::Addon,
                    excess: Excess::Entries {
                        entries: 3,
                        limit: 2
                    },
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            "refused many.zip: it holds 3 entries, more than the bound of 2 for one zip; nothing was written"
        );
        // At the bound the entries are checked as usual.
        let error = checked_entries_within(&mut archive, &zip, ZipRole::Addon, MAX_UNPACKED, 3)
            .expect_err("the escaping entry");
        assert!(matches!(error, Error::ZipEntryEscapes { .. }), "{error}");
        assert_eq!(MAX_ENTRIES, 100_000);
    }

    // --- the folder-name rule (task 123), one test per case ---

    #[test]
    fn a_folder_name_passes_with_inner_dots_spaces_and_a_leading_space() {
        for name in [
            "OneFolder",
            "BigWigs_Core",
            "DBM-Core",
            "Lib.Sub",
            "Fixture Realm",
            " Leading",
            "1-before.txt",
            "COM0",
            "COM10",
            "Console",
            "..x",
        ] {
            assert!(is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn an_empty_folder_name_fails() {
        assert!(!is_folder_name(""));
    }

    #[test]
    fn a_dot_or_dot_dot_folder_name_fails() {
        assert!(!is_folder_name("."));
        assert!(!is_folder_name(".."));
    }

    #[test]
    fn a_folder_name_with_a_separator_fails() {
        for name in ["a/b", "a\\b", "/a", "a\\", "..\\Some"] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_folder_name_with_a_colon_fails() {
        for name in ["C:", "a:b", "Foo:stream"] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_folder_name_with_a_control_character_fails() {
        for name in ["a\u{1}b", "tab\there", "line\n", "\u{7f}"] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_folder_name_with_another_character_windows_refuses_fails() {
        for name in ["a<b", "a>b", "a\"b", "a|b", "a?b", "a*b"] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_folder_name_with_a_trailing_dot_or_space_fails() {
        for name in ["Foo.", "Foo ", "...", " ", "  ", ". ", " ."] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_reserved_device_name_fails_with_or_without_an_extension() {
        for name in [
            "CON", "con", "PRN", "AUX", "NUL", "nul.lua", "COM1", "com9.txt", "LPT1", "lpt9.toc",
        ] {
            assert!(!is_folder_name(name), "{name:?}");
        }
    }

    #[test]
    fn the_entry_rule_lets_the_root_toc_names_of_finding_h1_through() {
        // Why the folder rule is a rule of its own: none of these entries has a `.` or `..` component,
        // so the entry rule passes them, and their stems are `.`, `..` and a blank.
        for name in ["..toc", "...toc", " .toc"] {
            assert_eq!(entry_path(name), strings(&[name]));
            assert!(!is_folder_name(&name[..name.len() - 4]), "{name:?}");
        }
    }

    #[test]
    fn the_entries_to_write_come_back_in_order_with_their_folders() {
        let mut archive = zip_of(&[
            ("./", b"", false),
            ("Foo/", b"", false),
            ("Foo/Foo.toc", b"## Title: Foo", false),
        ]);
        let entries = checked_entries(
            &mut archive,
            Path::new("foo.zip"),
            ZipRole::Addon,
            MAX_UNPACKED,
        )
        .expect("checked");
        assert_eq!(
            entries,
            [
                CheckedEntry {
                    index: 1,
                    parts: vec!["Foo".into()],
                    is_dir: true,
                    size: 0,
                },
                CheckedEntry {
                    index: 2,
                    parts: vec!["Foo".into(), "Foo.toc".into()],
                    is_dir: false,
                    size: 13,
                },
            ]
        );
    }

    /// The first rule difference: the engine used to check an entry as it unpacked it. Now the good
    /// entries before an escaping one never reach the caller, so nothing of the zip is written.
    #[test]
    fn an_escaping_entry_after_good_ones_refuses_the_whole_zip() {
        let mut archive = zip_of(&[
            ("Escape/Escape.toc", b"## Title: Escape", false),
            ("Escape/core.lua", b"-- fine", false),
            ("Escape/../../Outside.lua", b"-- escapes", false),
        ]);
        let zip = PathBuf::from("escape.zip");
        let error =
            checked_entries(&mut archive, &zip, ZipRole::Addon, MAX_UNPACKED).expect_err("refused");
        match &error {
            Error::ZipEntryEscapes { role, path, entry } => {
                assert_eq!(*role, ZipRole::Addon);
                assert_eq!(path, &zip);
                assert_eq!(entry, "Escape/../../Outside.lua");
            }
            other => panic!("{other}"),
        }
    }

    /// The second rule difference: the restore used to check the raw entry only, so an encrypted entry
    /// passed and failed half way through the unpack. Now it makes the zip unreadable at the check.
    #[test]
    fn an_entry_that_cannot_be_unpacked_makes_the_zip_unreadable() {
        let mut archive = zip_of(&[
            ("Config.wtf", b"SET fine", false),
            ("Account/secret.lua", b"-- encrypted", true),
        ]);
        let zip = PathBuf::from("WTF-encrypted.zip");
        let error = checked_entries(&mut archive, &zip, ZipRole::Backup, MAX_UNPACKED)
            .expect_err("refused");
        match &error {
            Error::ZipUnreadable { role, path, reason } => {
                assert_eq!(*role, ZipRole::Backup);
                assert_eq!(path, &zip);
                assert!(reason.starts_with("entry 1: "), "{reason}");
            }
            other => panic!("{other}"),
        }
    }

    #[test]
    fn the_role_keeps_each_command_s_message() {
        let zip = PathBuf::from("x.zip");
        let escapes = |role| {
            Error::ZipEntryEscapes {
                role,
                path: zip.clone(),
                entry: "../y".into(),
            }
            .to_string()
        };
        let unreadable = |role| {
            Error::ZipUnreadable {
                role,
                path: zip.clone(),
                reason: "why".into(),
            }
            .to_string()
        };
        assert_eq!(
            escapes(ZipRole::Addon),
            "refused x.zip: the entry `../y` would land outside the staging folder"
        );
        assert_eq!(
            escapes(ZipRole::Backup),
            "refused x.zip: the entry `../y` would land outside WTF"
        );
        assert_eq!(
            unreadable(ZipRole::Addon),
            "not a readable zip: x.zip (why)"
        );
        assert_eq!(
            unreadable(ZipRole::Backup),
            "not a readable backup: x.zip (why)"
        );
    }

    #[test]
    fn a_symbolic_link_entry_is_refused() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer
            .add_symlink("Foo/link", "../../outside", options)
            .expect("a link entry");
        let bytes = writer.finish().expect("finished").into_inner();
        let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("readable");
        let error = checked_entries(
            &mut archive,
            Path::new("link.zip"),
            ZipRole::Backup,
            MAX_UNPACKED,
        )
        .expect_err("refused");
        assert!(
            matches!(&error, Error::ZipEntryEscapes { entry, .. } if entry == "Foo/link"),
            "{error}"
        );
    }

    // --- task 039: the bound on the unpacked size ---

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/zips")
            .join(name)
    }

    fn excess(error: &Error) -> &Excess {
        match error {
            Error::ZipTooLarge { excess, .. } => excess,
            other => panic!("{other}"),
        }
    }

    /// Reads a checked entry through the counter in reads of at most `chunk` bytes, as a writer would.
    fn drain<R: Read>(mut reader: R, chunk: usize) -> io::Result<u64> {
        let mut buffer = vec![0u8; chunk];
        let mut total = 0;
        loop {
            match reader.read(&mut buffer)? {
                0 => return Ok(total),
                count => total += count as u64,
            }
        }
    }

    #[test]
    fn the_declared_sizes_of_the_files_are_summed_against_the_bound() {
        let entries = [
            ("Foo/", &b""[..], false),
            ("Foo/a.lua", &b"0123456789"[..], false),
            ("Foo/b.lua", &b"0123456789"[..], false),
        ];
        let zip = PathBuf::from("foo.zip");
        let checked =
            checked_entries(&mut zip_of(&entries), &zip, ZipRole::Addon, 20).expect("at the bound");
        assert_eq!(
            checked.iter().map(|entry| entry.size).collect::<Vec<_>>(),
            [0, 10, 10]
        );
        let error = checked_entries(&mut zip_of(&entries), &zip, ZipRole::Addon, 19)
            .expect_err("past the bound");
        assert_eq!(
            excess(&error),
            &Excess::Declared {
                declared: 20,
                limit: 19
            }
        );
        assert!(
            matches!(&error, Error::ZipTooLarge { role: ZipRole::Addon, path, .. } if path == &zip),
            "{error}"
        );
    }

    /// The fixture declares 3 GiB in a few hundred bytes: the default bound refuses it from the central
    /// directory alone, a higher one lets it through.
    #[test]
    fn a_zip_declaring_more_than_the_default_bound_is_refused_before_writing() {
        let zip = fixture("declares-too-much-1.0.0.zip");
        let open =
            || ZipArchive::new(std::fs::File::open(&zip).expect("the fixture")).expect("readable");
        let error =
            checked_entries(&mut open(), &zip, ZipRole::Addon, MAX_UNPACKED).expect_err("refused");
        assert!(
            matches!(excess(&error), Excess::Declared { declared, limit: MAX_UNPACKED } if *declared > 3 << 30),
            "{error}"
        );
        assert_eq!(
            error.to_string(),
            format!(
                "refused {}: it declares 3.0 GB unpacked, more than the bound of 2.0 GB; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you trust",
                zip.display()
            )
        );
        let checked =
            checked_entries(&mut open(), &zip, ZipRole::Addon, 4 << 30).expect("under 4 GiB");
        assert_eq!(checked.len(), 2);
    }

    /// The counter on its own, over a plain reader: an entry that outgrows its declared size, and a zip
    /// whose entries keep to their sizes but pass the bound together.
    #[test]
    fn the_counter_refuses_an_entry_past_its_size_and_a_zip_past_the_bound() {
        let zip = PathBuf::from("counted.zip");
        let entry = |name: &str, size| CheckedEntry {
            index: 0,
            parts: vec![name.to_string()],
            is_dir: false,
            size,
        };
        let mut unpacked = Unpacked::new(&zip, ZipRole::Backup, 50);
        let small = entry("small.txt", 30);
        assert_eq!(
            drain(unpacked.entry(&small, &[7u8; 30][..]), 8).expect("within its size"),
            30
        );
        let liar = entry("liar.txt", 10);
        let error = drain(unpacked.entry(&liar, &[7u8; 40][..]), 4).expect_err("past its size");
        let text = error.to_string();
        let error = Error::from(refusal(error).expect("the bound's refusal"));
        assert_eq!(
            error.to_string(),
            text,
            "the I/O error prints as the refusal"
        );
        assert_eq!(
            excess(&error),
            &Excess::Entry {
                entry: "liar.txt".into(),
                declared: 10,
                written: 38,
                limit: 50,
            }
        );
        assert!(
            matches!(
                &error,
                Error::ZipTooLarge {
                    role: ZipRole::Backup,
                    ..
                }
            ),
            "{error}"
        );

        let mut unpacked = Unpacked::new(&zip, ZipRole::Addon, 50);
        let first = entry("first.txt", 30);
        drain(unpacked.entry(&first, &[7u8; 30][..]), 8).expect("within the bound");
        let second = entry("second.txt", 30);
        let error = drain(unpacked.entry(&second, &[7u8; 30][..]), 8).expect_err("past the bound");
        let error = Error::from(refusal(error).expect("the bound's refusal"));
        assert_eq!(
            excess(&error),
            &Excess::Written {
                written: 46,
                limit: 50
            }
        );
        assert_eq!(unpacked.written(), 46);
        assert_eq!(
            error.to_string(),
            "refused counted.zip: it unpacked past the bound of 50 B (46 B written, then removed)"
        );
    }

    /// The fixture declares 16 bytes for 66 000: the zip crate refuses the read that takes the entry past
    /// 16 (here the first one) and the counter reports that as the lie, with the bytes written so far (the
    /// TOC before it).
    #[test]
    fn a_zip_that_lies_about_its_size_is_refused_while_writing() {
        let zip = fixture("lies-about-its-size-1.0.0.zip");
        let mut archive =
            ZipArchive::new(std::fs::File::open(&zip).expect("the fixture")).expect("readable");
        let entries = checked_entries(&mut archive, &zip, ZipRole::Addon, MAX_UNPACKED)
            .expect("declares 125 bytes");
        let mut unpacked = Unpacked::new(&zip, ZipRole::Addon, MAX_UNPACKED);
        let mut outcome = Ok(0);
        for checked in &entries {
            let entry = archive.by_index(checked.index).expect("an entry");
            outcome = drain(unpacked.entry(checked, entry), 64 * 1024);
            if outcome.is_err() {
                break;
            }
        }
        let error =
            Error::from(refusal(outcome.expect_err("refused")).expect("the bound's refusal"));
        let toc = entries[0].size;
        assert_eq!(
            excess(&error),
            &Excess::Entry {
                entry: "LiesAboutItsSize/core.lua".into(),
                declared: 16,
                written: toc,
                limit: MAX_UNPACKED,
            }
        );
        assert_eq!(
            error.to_string(),
            format!(
                "refused {}: the entry `LiesAboutItsSize/core.lua` unpacks to more than the 16 B it declares, so the bound of 2.0 GB cannot hold ({} B written, then removed)",
                zip.display(),
                toc
            )
        );
    }

    /// A reader that fails with the given error on its first read.
    struct Failing(Option<io::Error>);

    impl Read for Failing {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(self.0.take().expect("read once"))
        }
    }

    #[test]
    fn a_read_failure_that_is_not_the_size_lie_comes_back_as_it_was() {
        // Only the zip crate's own refusal of an entry past its declared size is the lie; any other
        // failure, `InvalidData` with other words included, is the plain I/O error it was.
        let checked = CheckedEntry {
            index: 0,
            parts: vec!["Foo".into(), "core.lua".into()],
            is_dir: false,
            size: 16,
        };
        for error in [
            io::Error::new(io::ErrorKind::InvalidData, "Invalid checksum"),
            io::Error::other("the disk went away"),
        ] {
            let kind = error.kind();
            let text = error.to_string();
            let mut unpacked = Unpacked::new(Path::new("x.zip"), ZipRole::Addon, MAX_UNPACKED);
            let mut counted = unpacked.entry(&checked, Failing(Some(error)));
            let failure = counted.read(&mut [0u8; 8]).expect_err("fails");
            assert_eq!((failure.kind(), failure.to_string()), (kind, text.clone()));
            assert!(refusal(failure).is_err(), "no refusal in {text}");
            assert_eq!(unpacked.written(), 0);
        }
    }

    #[test]
    fn sizes_parse_and_print_in_steps_of_1024() {
        assert_eq!(parse_size("3G"), Some(3 << 30));
        assert_eq!(parse_size("500M"), Some(500 << 20));
        assert_eq!(parse_size("500mb"), Some(500 << 20));
        assert_eq!(parse_size("2GiB"), Some(MAX_UNPACKED));
        assert_eq!(parse_size("64k"), Some(64 << 10));
        assert_eq!(parse_size("1T"), Some(1 << 40));
        assert_eq!(parse_size("1048576"), Some(1 << 20));
        assert_eq!(parse_size(" 10 B "), Some(10));
        for bad in ["", "G", "0", "0G", "1.5G", "-1G", "3X", "99999999999T"] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
        assert_eq!(size_text(512), "512 B");
        assert_eq!(size_text(16 * 1024), "16.0 KB");
        assert_eq!(size_text(MAX_UNPACKED), "2.0 GB");
        assert_eq!(size_text(3 << 30), "3.0 GB");
    }
}
