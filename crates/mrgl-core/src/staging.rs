//! The run folders under a flavor's `staging\` (task 084): every download and every unpacked zip of the
//! install engine ([`crate::install`]) gets a fresh folder of its own, [`RunDir`], which the run removes on
//! every path out. A run that ends before its guard goes (a killed process, a crash) leaves its folder
//! behind, so the engine sweeps the leftovers ([`sweep`]) before it stages.
//!
//! The rule that tells a running mrgl's folder from a leftover is a lock and an age. A run holds an
//! exclusive lock on the [`LOCK_FILE`] in its folder for as long as the folder is its own (the operating
//! system releases it when the process ends, however it ends), and a folder whose lock is held is never
//! touched: the run's own, and another run's on the same flavor (the window and the command line). A folder
//! with a free lock, or with no lock file at all (one made by an mrgl before task 084, or one a run created
//! a moment ago and has not locked yet), is a leftover only once it is older than [`STALE_AFTER`], by the
//! creation time its name carries (`<stem>-<nanoseconds since 1970>-<attempt>`), else by its modified
//! time. A folder that holds a `previous\` folder is never swept: it holds the addon folders an interrupted
//! or failed swap moved aside, which may be the only copy of the version installed before. A removal that
//! fails is a warning, never an error. Such a folder stays until the user removes it, so [`kept`] lists
//! them for the doctor (task 088).

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::state;

/// The lock file in a run's folder, locked by the run for as long as it holds the folder.
pub const LOCK_FILE: &str = ".mrgl-run.lock";
/// The folder under a run's folder where a swap moves the folders it replaces.
pub const PREVIOUS_DIR: &str = "previous";
/// How old a folder whose lock is not held has to be before the sweep removes it: long past any download
/// and install, so a run of an mrgl that takes no lock is not swept while it works.
pub const STALE_AFTER: Duration = Duration::from_secs(60 * 60);

/// A run's own folder under `staging\`, fresh and locked. Dropping it releases the lock and removes the
/// folder, unless [`RunDir::keep`] was called (a failed revert's folder, which holds the previous folders).
#[derive(Debug)]
pub struct RunDir {
    path: PathBuf,
    lock: Option<File>,
    keep: bool,
}

impl RunDir {
    /// A fresh folder under `staging`, named after the stem and the clock, never one that exists, with its
    /// lock taken. A lock that cannot be taken leaves the folder to the age rule; the run goes on.
    pub fn create(staging: &Path, stem: &str) -> Result<RunDir> {
        let io = |path: &Path, source| Error::Io {
            context: "creating the staging folder",
            path: path.to_path_buf(),
            source,
        };
        std::fs::create_dir_all(staging).map_err(|source| io(staging, source))?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let stem = state::file_safe(stem);
        for attempt in 0u32.. {
            let path = staging.join(format!("{stem}-{nanos}-{attempt}"));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    let lock = take_lock(&path.join(LOCK_FILE));
                    return Ok(RunDir {
                        path,
                        lock,
                        keep: false,
                    });
                }
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io(&path, source)),
            }
        }
        unreachable!("the attempts never run out")
    }

    /// The folder.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Keeps the folder when the guard goes: the lock is released, the folder stays.
    pub fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for RunDir {
    fn drop(&mut self) {
        // The lock goes first, so the folder's last handle is closed before the folder is removed.
        drop(self.lock.take());
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// The lock file created and locked; `None` when either fails.
fn take_lock(path: &Path) -> Option<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .ok()?;
    file.try_lock().ok()?;
    Some(file)
}

/// Removes the leftover folders under `staging` (see the module doc): every folder whose lock is free or
/// missing, older than [`STALE_AFTER`] and without a `previous\` folder. Returns a warning for each folder
/// that could not be removed; a `staging` that does not exist is nothing to sweep.
pub fn sweep(staging: &Path) -> Vec<String> {
    sweep_at(staging, SystemTime::now())
}

/// [`sweep`] at a given time (the tests' clock).
pub fn sweep_at(staging: &Path, now: SystemTime) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(staging) else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if !is_dir || !is_leftover(&path, now) {
            continue;
        }
        if let Err(error) = std::fs::remove_dir_all(&path) {
            warnings.push(format!(
                "a leftover staging folder could not be removed: {}: {error}",
                path.display()
            ));
        }
    }
    warnings
}

/// A run folder the sweep keeps because it holds a `previous\` folder (task 088): the addon folders an
/// interrupted or failed swap moved aside, which the doctor names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptRun {
    /// The run folder.
    pub path: PathBuf,
    /// When it was made: the time its name carries, else its modified time.
    pub created: Option<SystemTime>,
    /// The names under its `previous\` folder, sorted.
    pub previous: Vec<String>,
}

/// Every folder under `staging` that the sweep keeps for a `previous\` folder in it and that no running
/// mrgl holds (a held lock is a swap in progress), sorted by name; a `staging` that does not exist holds
/// none. Reads only: a lock this process can take is released at once, as the sweep does.
pub fn kept(staging: &Path) -> Vec<KeptRun> {
    let Ok(entries) = std::fs::read_dir(staging) else {
        return Vec::new();
    };
    let mut kept: Vec<KeptRun> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|path| path.join(PREVIOUS_DIR).exists() && lock_is_free(&path.join(LOCK_FILE)))
        .map(|path| {
            let mut previous: Vec<String> = std::fs::read_dir(path.join(PREVIOUS_DIR))
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            previous.sort();
            KeptRun {
                created: created_at(&path),
                previous,
                path,
            }
        })
        .collect();
    kept.sort_by(|a, b| a.path.cmp(&b.path));
    kept
}

/// Whether a folder under `staging\` is a leftover: old enough, no `previous\` in it, and its lock free.
fn is_leftover(path: &Path, now: SystemTime) -> bool {
    let Some(created) = created_at(path) else {
        return false;
    };
    let old = now
        .duration_since(created)
        .is_ok_and(|age| age >= STALE_AFTER);
    if !old || path.join(PREVIOUS_DIR).exists() {
        return false;
    }
    lock_is_free(&path.join(LOCK_FILE))
}

/// When a run's folder was made: the time its name carries, else its modified time.
fn created_at(path: &Path) -> Option<SystemTime> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let mut parts = name.rsplit('-');
    let named = parts
        .next()
        .filter(|attempt| attempt.parse::<u32>().is_ok())
        .and_then(|_| parts.next())
        .and_then(|nanos| nanos.parse::<u64>().ok())
        .map(|nanos| UNIX_EPOCH + Duration::from_nanos(nanos));
    named.or_else(|| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    })
}

/// Whether nobody holds the lock of a run's folder: a missing lock file is free, a lock this process can
/// take is free (and released at once), and anything else (held, or a file that cannot be opened) is not.
fn lock_is_free(lock: &Path) -> bool {
    match File::open(lock) {
        Ok(file) => match file.try_lock() {
            Ok(()) => true,
            Err(TryLockError::WouldBlock | TryLockError::Error(_)) => false,
        },
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn temp_staging(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mrgl-staging-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("staging")
    }

    /// A folder as a run of the given age would have named it, with a file in it.
    fn planted(staging: &Path, stem: &str, age: Duration) -> PathBuf {
        let nanos = (SystemTime::now() - age)
            .duration_since(UNIX_EPOCH)
            .expect("after 1970")
            .as_nanos();
        let dir = staging.join(format!("{stem}-{nanos}-0"));
        std::fs::create_dir_all(dir.join("unpacked")).expect("the planted folder");
        std::fs::write(dir.join("unpacked").join("core.lua"), "-- left over\n").expect("a file");
        dir
    }

    const OLD: Duration = Duration::from_secs(2 * 60 * 60);

    #[test]
    fn an_old_folder_with_no_lock_is_swept_and_a_young_one_stays() {
        let staging = temp_staging("old-and-young");
        let old = planted(&staging, "download-Old.zip", OLD);
        let young = planted(&staging, "download-Young.zip", Duration::from_secs(60));
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(!old.exists(), "the leftover is gone");
        assert!(
            young.is_dir(),
            "a folder made a minute ago may be a run that has not locked yet"
        );
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn a_locked_folder_stays_however_old_and_goes_once_its_run_ends() {
        let staging = temp_staging("locked");
        let run = RunDir::create(&staging, "Fixture.zip").expect("a run folder");
        assert!(run.path().join(LOCK_FILE).is_file());
        let later = SystemTime::now() + OLD;
        assert_eq!(sweep_at(&staging, later), Vec::<String>::new());
        assert!(
            run.path().is_dir(),
            "the run's own folder stays while it runs"
        );

        // A run that ends without its guard (a killed process) releases the lock and leaves the folder.
        let mut ended = run;
        let path = ended.path().to_path_buf();
        ended.keep();
        drop(ended);
        assert!(path.is_dir());
        assert_eq!(sweep_at(&staging, later), Vec::<String>::new());
        assert!(!path.exists(), "a free lock and an old folder: a leftover");
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn a_lock_held_through_another_handle_keeps_the_folder() {
        // Another run (the window next to the command line) holds the lock of an old folder.
        let staging = temp_staging("other-run");
        let other = planted(&staging, "Other.zip", OLD);
        let lock = File::create(other.join(LOCK_FILE)).expect("the lock file");
        lock.try_lock().expect("the other run's lock");
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(other.is_dir());
        drop(lock);
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(!other.exists());
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn a_folder_with_previous_folders_in_it_is_never_swept() {
        let staging = temp_staging("previous");
        let kept = planted(&staging, "Kept.zip", OLD);
        std::fs::create_dir_all(kept.join(PREVIOUS_DIR).join("OneFolder")).expect("previous");
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(kept.join(PREVIOUS_DIR).join("OneFolder").is_dir());
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn kept_lists_a_folder_with_previous_folders_and_no_other() {
        let staging = temp_staging("kept");
        let kept_dir = planted(&staging, "Kept.zip", OLD);
        std::fs::create_dir_all(kept_dir.join(PREVIOUS_DIR).join("Two")).expect("previous");
        std::fs::create_dir_all(kept_dir.join(PREVIOUS_DIR).join("One")).expect("previous");
        planted(&staging, "Plain.zip", OLD);
        // A swap in progress: its run holds the lock.
        let running = RunDir::create(&staging, "Running.zip").expect("a run folder");
        std::fs::create_dir_all(running.path().join(PREVIOUS_DIR).join("Three")).expect("previous");
        let found = kept(&staging);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path, kept_dir);
        assert_eq!(found[0].previous, ["One", "Two"]);
        let age = SystemTime::now()
            .duration_since(found[0].created.expect("the time in the name"))
            .expect("in the past");
        assert!(age >= OLD && age < OLD + Duration::from_secs(60), "{age:?}");
        drop(running);
        assert_eq!(kept(&staging.join("none")), Vec::new());
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn a_name_without_a_time_falls_back_to_the_modified_time_and_files_stay() {
        let staging = temp_staging("no-time");
        std::fs::create_dir_all(staging.join("by-hand")).expect("a folder by hand");
        std::fs::write(staging.join("note.txt"), "not a run folder\n").expect("a file");
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(staging.join("by-hand").is_dir(), "modified just now");
        assert_eq!(
            sweep_at(&staging, SystemTime::now() + OLD),
            Vec::<String>::new()
        );
        assert!(!staging.join("by-hand").exists());
        assert!(staging.join("note.txt").is_file(), "only folders are swept");
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }

    #[test]
    fn no_staging_folder_is_nothing_to_sweep() {
        let staging = temp_staging("none");
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(!staging.exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_removal_that_fails_is_a_warning() {
        use std::os::windows::fs::OpenOptionsExt;
        let staging = temp_staging("fails");
        let stuck = planted(&staging, "Stuck.zip", OLD);
        let open = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(stuck.join("unpacked").join("core.lua"))
            .expect("a handle with no sharing");
        let warnings = sweep(&staging);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with(&format!(
                "a leftover staging folder could not be removed: {}: ",
                stuck.display()
            )),
            "{}",
            warnings[0]
        );
        drop(open);
        assert_eq!(sweep(&staging), Vec::<String>::new());
        assert!(!stuck.exists());
        let _ = std::fs::remove_dir_all(staging.parent().expect("the test folder"));
    }
}
