//! The one check every zip entry passes before mrgl writes it (task 044): the install engine's unpack into
//! its staging folder ([`crate::install`], task 004) and the `WTF` restore ([`crate::backup`], task 013) both
//! call [`checked_entries`] and write only what it hands back, so the rule that keeps a zip from writing
//! outside its folder lives here and nowhere else.
//!
//! An entry passes when it can be opened for reading, its name (read as UTF-8, else as CP437) is a relative
//! path under the target folder and it is not a symbolic link. A name fails on an absolute path, a drive
//! letter, a `..` component, a component with a character Windows refuses (a control character,
//! `:<>"|?*`) or a reserved device name (`CON`, `NUL`, `COM1` … `LPT9`, with or without an extension);
//! `\` counts as a separator, `.` and empty components are dropped, and a name with no component left (`./`)
//! is skipped. A zip that fails refuses as a whole: [`Error::ZipEntryEscapes`] for a name or a link,
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

use std::io::{Read, Seek};
use std::path::Path;

use zip::ZipArchive;

use crate::error::{Error, Result};

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

/// One entry that passed the check: its index in the zip, its path components under the target folder
/// (never empty) and whether it is a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedEntry {
    pub index: usize,
    pub parts: Vec<String>,
    pub is_dir: bool,
}

/// Checks every entry of an open zip before anything is written and returns the ones to write, in the
/// zip's order. The first entry that fails refuses the whole zip: [`Error::ZipEntryEscapes`] for a name
/// outside the target folder or a symbolic link, [`Error::ZipUnreadable`] for an entry that cannot be
/// opened for reading or whose name cannot be read. `zip` is the path the errors name.
pub fn checked_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    zip: &Path,
    role: ZipRole,
) -> Result<Vec<CheckedEntry>> {
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
            .map_err(|reason| unreadable(format!("entry {index}: {reason}")))?
            .into_owned();
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
        entries.push(CheckedEntry {
            index,
            parts,
            is_dir: entry.is_dir(),
        });
    }
    Ok(entries)
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
                if part
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
                    || is_reserved_device_name(part)
                {
                    return None;
                }
                parts.push(part.to_string());
            }
        }
    }
    Some(parts)
}

/// `CON`, `NUL`, `COM1` … `LPT9`, with or without an extension: a file Windows will not create.
fn is_reserved_device_name(part: &str) -> bool {
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
        assert_eq!(
            entry_path("Foo/Console.lua"),
            strings(&["Foo", "Console.lua"])
        );
    }

    #[test]
    fn the_entries_to_write_come_back_in_order_with_their_folders() {
        let mut archive = zip_of(&[
            ("./", b"", false),
            ("Foo/", b"", false),
            ("Foo/Foo.toc", b"## Title: Foo", false),
        ]);
        let entries =
            checked_entries(&mut archive, Path::new("foo.zip"), ZipRole::Addon).expect("checked");
        assert_eq!(
            entries,
            [
                CheckedEntry {
                    index: 1,
                    parts: vec!["Foo".into()],
                    is_dir: true,
                },
                CheckedEntry {
                    index: 2,
                    parts: vec!["Foo".into(), "Foo.toc".into()],
                    is_dir: false,
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
        let error = checked_entries(&mut archive, &zip, ZipRole::Addon).expect_err("refused");
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
        let error = checked_entries(&mut archive, &zip, ZipRole::Backup).expect_err("refused");
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
        let error = checked_entries(&mut archive, Path::new("link.zip"), ZipRole::Backup)
            .expect_err("refused");
        assert!(
            matches!(&error, Error::ZipEntryEscapes { entry, .. } if entry == "Foo/link"),
            "{error}"
        );
    }
}
