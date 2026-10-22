//! The account, realm and character folders under a flavor's `WTF` folder (task 071): the one walk the
//! addon sets ([`crate::sets`], task 014) and the SavedVariables listing ([`crate::savedvars`], task 025)
//! both read `WTF` with.
//!
//! The client keeps one folder per account under `WTF\Account\`, one per realm under an account and one
//! per character under a realm; next to the realm folders, an account holds its account-wide
//! `SavedVariables` folder, which is never a realm. The walk visits every account folder, then every
//! character folder of that account, each level sorted by name. A missing folder has nothing under it,
//! never an error; files are passed over, and a link or a junction is neither followed nor visited: it is
//! a warning, which a caller without a warnings channel leaves unsaid. The `WTF` folder itself is
//! [`crate::backup::WTF_DIR`] under the flavor folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The folder of the account folders, under `WTF`.
pub const ACCOUNT_DIR: &str = "Account";
/// The folder of the SavedVariables files, under an account or a character folder; the account's one sits
/// next to the realm folders and is never a realm.
pub const SAVED_VARIABLES_DIR: &str = "SavedVariables";

/// A folder the walk visits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WtfFolder<'a> {
    /// An account folder, `WTF\Account\<account>\`, visited before its characters.
    Account { account: &'a str, folder: &'a Path },
    /// A character folder, `WTF\Account\<account>\<realm>\<character>\`.
    Character {
        account: &'a str,
        realm: &'a str,
        name: &'a str,
        folder: &'a Path,
    },
}

/// Walks the account and character folders under a `WTF` folder in name order, calling `visit` for each
/// with the warnings, so a warning the visit adds stays in the order the folders were read. A link or a
/// junction met on the way is a warning in `warnings`; the first error of the walk or of a visit ends it.
pub fn walk(
    wtf: &Path,
    warnings: &mut Vec<String>,
    mut visit: impl FnMut(WtfFolder<'_>, &mut Vec<String>) -> Result<()>,
) -> Result<()> {
    for (account, account_dir) in subfolders(&wtf.join(ACCOUNT_DIR), warnings)? {
        visit(
            WtfFolder::Account {
                account: &account,
                folder: &account_dir,
            },
            warnings,
        )?;
        for (realm, realm_dir) in subfolders(&account_dir, warnings)? {
            if realm.eq_ignore_ascii_case(SAVED_VARIABLES_DIR) {
                continue;
            }
            for (name, folder) in subfolders(&realm_dir, warnings)? {
                visit(
                    WtfFolder::Character {
                        account: &account,
                        realm: &realm,
                        name: &name,
                        folder: &folder,
                    },
                    warnings,
                )?;
            }
        }
    }
    Ok(())
}

/// The subfolders of a folder by name, with their paths; a missing folder has none, and a link is a
/// warning and not followed.
fn subfolders(dir: &Path, warnings: &mut Vec<String>) -> Result<Vec<(String, PathBuf)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error(dir, source)),
    };
    let mut folders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error(dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| io_error(&path, source))?; // coverage: os
        if file_type.is_symlink() {
            warnings.push(link_warning(&path));
        } else if file_type.is_dir() {
            folders.push((entry.file_name().to_string_lossy().into_owned(), path));
        }
    }
    folders.sort();
    Ok(folders)
}

/// The warning for a link or a junction under `WTF`, which is never followed.
pub(crate) fn link_warning(path: &Path) -> String {
    format!(
        "not read: {} is a link, not a folder or a file",
        path.display()
    )
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        context: "reading WTF",
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn temp_wtf(name: &str) -> PathBuf {
        let wtf = std::env::temp_dir()
            .join(format!("mrgl-wtf-test-{}-{name}", std::process::id()))
            .join("WTF");
        let _ = fs::remove_dir_all(wtf.parent().expect("a parent"));
        wtf
    }

    fn visits(wtf: &Path) -> (Vec<String>, Vec<String>) {
        let mut seen = Vec::new();
        let mut warnings = Vec::new();
        walk(wtf, &mut warnings, |folder, _| {
            seen.push(match folder {
                WtfFolder::Account { account, folder } => {
                    assert!(folder.ends_with(account));
                    format!("account {account}")
                }
                WtfFolder::Character {
                    account,
                    realm,
                    name,
                    folder,
                } => {
                    assert!(folder.ends_with(Path::new(realm).join(name)));
                    format!("character {account}/{realm}/{name}")
                }
            });
            Ok(())
        })
        .expect("a walk");
        (seen, warnings)
    }

    #[test]
    fn the_walk_visits_each_account_before_its_characters_in_name_order() {
        let wtf = temp_wtf("order");
        let accounts = wtf.join(ACCOUNT_DIR);
        for folder in [
            "B/Realm/Zed",
            "B/Realm/Ann",
            "A/Other Realm/Cid",
            "A/SavedVariables/NotACharacter",
        ] {
            fs::create_dir_all(accounts.join(folder)).expect("a folder");
        }
        fs::write(accounts.join("A").join("config-cache.wtf"), "").expect("a file");
        fs::write(accounts.join("B").join("Realm").join("file.txt"), "").expect("a file");

        let (seen, warnings) = visits(&wtf);
        assert_eq!(
            seen,
            [
                "account A",
                "character A/Other Realm/Cid",
                "account B",
                "character B/Realm/Ann",
                "character B/Realm/Zed",
            ]
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        let _ = fs::remove_dir_all(wtf.parent().expect("a parent"));
    }

    #[test]
    fn a_missing_wtf_or_account_folder_has_nothing_under_it() {
        let wtf = temp_wtf("missing");
        assert_eq!(visits(&wtf), (Vec::new(), Vec::new()));
        fs::create_dir_all(&wtf).expect("WTF");
        assert_eq!(visits(&wtf), (Vec::new(), Vec::new()));
        let _ = fs::remove_dir_all(wtf.parent().expect("a parent"));
    }

    #[test]
    fn an_account_folder_that_is_a_file_is_an_error_that_names_it() {
        let wtf = temp_wtf("file");
        let accounts = wtf.join(ACCOUNT_DIR);
        fs::create_dir_all(&wtf).expect("WTF");
        fs::write(&accounts, "a file where the folder goes").expect("a file");
        let mut warnings = Vec::new();
        let error = walk(&wtf, &mut warnings, |_, _| Ok(())).expect_err("not a folder");
        assert!(
            matches!(&error, Error::Io { context: "reading WTF", path, .. } if path == &accounts),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .starts_with(&format!("reading WTF: {}: ", accounts.display())),
            "{error}"
        );
        assert!(warnings.is_empty());
        let _ = fs::remove_dir_all(wtf.parent().expect("a parent"));
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_is_a_warning_and_not_followed() {
        let wtf = temp_wtf("junction");
        let realm = wtf.join(ACCOUNT_DIR).join("A").join("Realm");
        fs::create_dir_all(realm.join("Real")).expect("a character");
        let elsewhere = wtf.parent().expect("a parent").join("elsewhere");
        fs::create_dir_all(elsewhere.join("Inside")).expect("a folder");
        crate::link::create_junction(&realm.join("Linked"), &elsewhere).expect("a junction");

        let (seen, warnings) = visits(&wtf);
        assert_eq!(seen, ["account A", "character A/Realm/Real"]);
        assert_eq!(warnings, [link_warning(&realm.join("Linked"))]);
        let _ = fs::remove_dir(realm.join("Linked"));
        let _ = fs::remove_dir_all(wtf.parent().expect("a parent"));
    }
}
