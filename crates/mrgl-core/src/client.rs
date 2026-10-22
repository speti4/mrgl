//! The running game client (task 023): a process whose executable lives under a flavor folder, found by
//! path and never by name, since the Forever executable's name is unknown before launch. The client
//! rewrites `AddOns.txt` on logout and holds `WTF` files open, so every `WTF` write (a backup restore, an
//! addon set, a SavedVariables cleanup; D-002, D-003) calls [`ensure_client_not_running`] first and refuses
//! with the documented line while the client runs. A write under `Interface\AddOns` is safe, the client
//! reads the folders again at `/reload` or the next login, so the install engine turns a running client
//! into a warning instead ([`RunningClient::install_warning`], [`RunningClient::remove_warning`]; task 038),
//! asking the process list it was given, and [`warn_if_client_running`] is the same check on the system's
//! list. `scan` and `list` report [`client_running`] in their summary and in `--json`.
//!
//! Behind the three calls, [`ProcessList`] lists the running processes with their executable paths:
//! [`SystemProcesses`] asks the operating system (on Windows one Toolhelp snapshot and one handle per
//! process, no file I/O, so the check costs nothing noticeable when no client runs) and [`FixedProcesses`]
//! is the stub for tests. Paths are compared component by component, case-insensitively on Windows, after
//! the flavor folder is canonicalized once (a junction into the game folder resolves to the path the
//! kernel reports for the process).

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf, Prefix};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::flavor::Flavor;

/// A running process: its id and the full path of its executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningProcess {
    pub pid: u32,
    pub exe: PathBuf,
}

/// The running processes of this machine, as far as this user may see them.
pub trait ProcessList {
    /// Every visible process with its executable's path. A process whose path cannot be read (the system
    /// processes, another user's) is left out, never an error; the error is a process list that cannot
    /// be taken at all.
    fn executables(&self) -> std::io::Result<Vec<RunningProcess>>;
}

/// The operating system's process list.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProcesses;

impl ProcessList for SystemProcesses {
    fn executables(&self) -> std::io::Result<Vec<RunningProcess>> {
        system::executables()
    }
}

/// A fixed process list, the stub for tests: the processes it is given, in that order.
#[derive(Debug, Clone, Default)]
pub struct FixedProcesses(pub Vec<RunningProcess>);

impl FixedProcesses {
    /// A stub from `(pid, exe)` pairs.
    pub fn new<P: Into<PathBuf>>(processes: impl IntoIterator<Item = (u32, P)>) -> Self {
        Self(
            processes
                .into_iter()
                .map(|(pid, exe)| RunningProcess {
                    pid,
                    exe: exe.into(),
                })
                .collect(),
        )
    }
}

impl ProcessList for FixedProcesses {
    fn executables(&self) -> std::io::Result<Vec<RunningProcess>> {
        Ok(self.0.clone())
    }
}

/// The game client found running from a flavor folder; `--json` carries it as `{pid, exe}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunningClient {
    pub pid: u32,
    /// The executable's full path, under the flavor folder.
    pub exe: PathBuf,
}

impl RunningClient {
    /// The executable's file name, for the plain lines.
    pub fn exe_name(&self) -> String {
        self.exe
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.exe.display().to_string())
    }

    /// The refusal of a `WTF` write while this client runs, as the error whose `Display` is the plain line.
    pub fn refusal(&self, flavor: &Flavor) -> Error {
        Error::ClientRunning {
            flavor: flavor.folder_name(),
            client: self.clone(),
        }
    }

    /// The plain warning line of a write under `Interface\AddOns` while this client runs.
    pub fn install_warning(&self, flavor: &Flavor) -> String {
        format!(
            "the client is running in {} ({}, pid {}): the installed addons load at /reload or the next login",
            flavor.folder_name(),
            self.exe_name(),
            self.pid
        )
    }

    /// The plain warning line of an addon removal under `Interface\AddOns` while this client runs.
    pub fn remove_warning(&self, flavor: &Flavor) -> String {
        format!(
            "the client is running in {} ({}, pid {}): the removed addons stay loaded until /reload or the next login",
            flavor.folder_name(),
            self.exe_name(),
            self.pid
        )
    }
}

/// The first process of a list whose executable lives under a folder. The folder is resolved once, into
/// its canonical form and, on Windows, its 8.3 short form (a missing folder is taken as given, absolute);
/// the process paths are compared as they are, since the kernel reports an image's path as the process
/// was started (a short-name component stays short) and a file lookup per process would cost what the
/// check must not.
pub fn client_running_among(
    processes: &dyn ProcessList,
    folder: &Path,
) -> Result<Option<RunningClient>> {
    let forms = resolved(folder);
    let found = processes
        .executables()
        .map_err(|source| Error::ProcessList { source })?
        .into_iter()
        .find(|process| is_under(&process.exe, &forms))
        .map(|process| RunningClient {
            pid: process.pid,
            exe: process.exe,
        });
    Ok(found)
}

/// The client running from a flavor's folder, through the system's process list; `None` when none runs.
/// The error is a process list that cannot be taken, not a running client.
pub fn client_running(flavor: &Flavor) -> Result<Option<RunningClient>> {
    client_running_among(&SystemProcesses, &flavor.folder)
}

/// The guard every `WTF` write calls first: `Ok(())` when no client runs from the flavor folder,
/// [`Error::ClientRunning`] with the refusal line when one does, and the process-list error when it
/// cannot tell (a write is refused rather than risked).
pub fn ensure_client_not_running(flavor: &Flavor) -> Result<()> {
    ensure_client_not_running_among(&SystemProcesses, flavor)
}

/// [`ensure_client_not_running`] against the process list it is given.
pub fn ensure_client_not_running_among(processes: &dyn ProcessList, flavor: &Flavor) -> Result<()> {
    match client_running_among(processes, &flavor.folder)? {
        Some(client) => Err(client.refusal(flavor)),
        None => Ok(()),
    }
}

/// The check the install engine turns into a warning before a write under `Interface\AddOns`: the plain
/// line when a client runs from the flavor folder, or when the process list cannot be taken (the install
/// goes on either way); `None` when no client runs.
pub fn warn_if_client_running(flavor: &Flavor) -> Option<String> {
    warn_if_client_running_among(&SystemProcesses, flavor)
}

/// [`warn_if_client_running`] against the process list it is given.
pub fn warn_if_client_running_among(
    processes: &dyn ProcessList,
    flavor: &Flavor,
) -> Option<String> {
    match client_running_among(processes, &flavor.folder) {
        Ok(Some(client)) => Some(client.install_warning(flavor)),
        Ok(None) => None,
        Err(error) => Some(error.to_string()),
    }
}

/// The forms a process path under the folder may take: the canonical (long-name) form when the folder
/// exists, absolute otherwise, and on Windows the 8.3 short form when the volume keeps one and it
/// differs; both without the `\\?\` prefix.
fn resolved(folder: &Path) -> Vec<PathBuf> {
    let path = std::fs::canonicalize(folder)
        .or_else(|_| std::path::absolute(folder))
        .unwrap_or_else(|_| folder.to_path_buf());
    let long = strip_verbatim(&path);
    let mut forms = vec![long];
    if let Some(short) = system::short_form(&path).map(|short| strip_verbatim(&short))
        && short != forms[0]
    {
        forms.push(short);
    }
    forms
}

/// The 8.3 short form of an existing path, without the `\\?\` prefix (`C:\Users\RUNNER~1`); `None`
/// outside Windows, for a missing path or a volume that keeps no short names. The doctor hides the profile
/// folder in both of its forms (task 024).
pub(crate) fn short_form(path: &Path) -> Option<PathBuf> {
    system::short_form(path).map(|short| strip_verbatim(&short))
}

/// `\\?\C:\x` as `C:\x` and `\\?\UNC\server\share` as `\\server\share`, so a canonical path and a
/// process's Win32 path compare component by component; the dev addon links (task 005) name their
/// folders the same way.
pub(crate) fn strip_verbatim(path: &Path) -> PathBuf {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_path_buf();
    };
    let root = match prefix.kind() {
        Prefix::VerbatimDisk(letter) => format!("{}:\\", char::from(letter)),
        Prefix::VerbatimUNC(server, share) => format!(
            "\\\\{}\\{}\\",
            server.to_string_lossy(),
            share.to_string_lossy()
        ),
        _ => return path.to_path_buf(),
    };
    let mut result = PathBuf::from(root);
    for component in components {
        if let Component::Normal(part) = component {
            result.push(part);
        }
    }
    result
}

/// Whether `exe` lies strictly under the folder, component by component, each component matching the
/// same component of any of the folder's forms (long or short names mixed, as a path may carry them); the
/// folder itself does not count. A form with another number of components than the first is ignored.
fn is_under(exe: &Path, forms: &[PathBuf]) -> bool {
    let Some(first) = forms.first() else {
        return false;
    };
    let wanted: Vec<Vec<Component<'_>>> = forms
        .iter()
        .map(|form| form.components().collect())
        .collect();
    let length = wanted[0].len();
    let wanted: Vec<&Vec<Component<'_>>> =
        wanted.iter().filter(|form| form.len() == length).collect();
    debug_assert_eq!(first.components().count(), length);
    let mut found = exe.components();
    for index in 0..length {
        let Some(component) = found.next() else {
            return false;
        };
        if !wanted
            .iter()
            .any(|form| same_component(component.as_os_str(), form[index].as_os_str()))
        {
            return false;
        }
    }
    found.next().is_some()
}

/// Two path components, case-insensitively on Windows (the file systems there are), exactly elsewhere.
#[cfg(windows)]
fn same_component(found: &OsStr, wanted: &OsStr) -> bool {
    found.to_string_lossy().to_lowercase() == wanted.to_string_lossy().to_lowercase()
}

/// Two path components, exactly: the file systems outside Windows keep the case.
#[cfg(not(windows))]
fn same_component(found: &OsStr, wanted: &OsStr) -> bool {
    found == wanted
}

#[cfg(windows)]
mod system {
    //! One `CreateToolhelp32Snapshot` over the processes, then per process an `OpenProcess` with the
    //! limited query right and `QueryFullProcessImageNameW` for the Win32 path of its image: no file is
    //! opened and nothing is read from disk. A process that cannot be opened (a system process, another
    //! user's with no right to query it) is left out.

    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    use super::RunningProcess;

    /// The 8.3 short form of an existing path (`C:\Users\RUNNER~1\…`), as a process started through it
    /// carries it; `None` when Windows has none to give (the volume keeps no short names, the path is
    /// gone). One call for the flavor folder, never per process.
    pub(super) fn short_form(path: &Path) -> Option<PathBuf> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut buffer = vec![0u16; 1024];
        for _ in 0..2 {
            // SAFETY: `wide` is NUL-terminated and the buffer is `buffer.len()` UTF-16 units long; the
            // return value is the length written, or the length needed when the buffer is too small.
            let length = unsafe {
                GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
            };
            if length == 0 {
                return None;
            }
            if (length as usize) <= buffer.len() {
                buffer.truncate(length as usize);
                return Some(PathBuf::from(OsString::from_wide(&buffer)));
            }
            buffer = vec![0u16; length as usize];
        }
        None
    }

    /// A kernel handle closed on drop.
    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful CreateToolhelp32Snapshot or OpenProcess and is
            // closed exactly once, here.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn executables() -> std::io::Result<Vec<RunningProcess>> {
        // SAFETY: a plain call with constant flags; the process id argument is unused for a process snapshot.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let snapshot = Handle(snapshot);
        // SAFETY: PROCESSENTRY32W is a plain C struct; all-zero is a valid value, and dwSize is set
        // before the first call as the API requires.
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        // SAFETY: the snapshot handle is valid and `entry` is a correctly sized, writable struct.
        if unsafe { Process32FirstW(snapshot.0, &mut entry) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut processes = Vec::new();
        loop {
            let pid = entry.th32ProcessID;
            if let Some(exe) = image_path(pid) {
                processes.push(RunningProcess { pid, exe });
            }
            // SAFETY: as for Process32FirstW; a zero return is the end of the snapshot.
            if unsafe { Process32NextW(snapshot.0, &mut entry) } == 0 {
                break;
            }
        }
        Ok(processes)
    }

    /// The Win32 path of a process's image, or `None` when the process cannot be opened or queried.
    fn image_path(pid: u32) -> Option<PathBuf> {
        if pid == 0 {
            return None;
        }
        // SAFETY: a plain call; a null handle is the failure case and is checked.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let handle = Handle(handle);
        // Most paths fit MAX_PATH; a longer one gets one retry with the largest path Windows allows.
        for capacity in [1024usize, 32_768] {
            let mut buffer = vec![0u16; capacity];
            let mut length = capacity as u32;
            // SAFETY: the handle is open with the query right, the buffer is `length` UTF-16 units long
            // and `length` is updated to the characters written.
            let ok = unsafe {
                QueryFullProcessImageNameW(
                    handle.0,
                    PROCESS_NAME_WIN32,
                    buffer.as_mut_ptr(),
                    &mut length,
                )
            };
            if ok != 0 {
                buffer.truncate(length as usize);
                return Some(PathBuf::from(OsString::from_wide(&buffer)));
            }
        }
        None
    }
}

#[cfg(not(windows))]
mod system {
    //! The other targets list nothing yet: the check reports no client and the guard lets a write
    //! through. The macOS preview (M6) brings the macOS process list.

    use std::path::{Path, PathBuf};

    use super::RunningProcess;

    pub(super) fn executables() -> std::io::Result<Vec<RunningProcess>> {
        Ok(Vec::new())
    }

    /// No second form of a path outside Windows.
    pub(super) fn short_form(_path: &Path) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::flavor::FlavorId;

    fn flavor(folder: &str) -> Flavor {
        Flavor {
            id: FlavorId::Forever,
            folder: PathBuf::from(folder),
            version: None,
            version_source: None,
            curse_client: false,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn a_process_under_the_folder_is_the_client() {
        // Made-up paths on a drive that does not exist here: a missing folder is taken as given.
        let processes = FixedProcesses::new([
            (4, "C:\\Windows\\System32\\svchost.exe"),
            (120, "Q:\\wow\\_retail_\\Wow.exe"),
            (121, "Q:\\wow\\_forever_\\Launcher\\Helper.exe"),
            (122, "Q:\\wow\\_forever_\\WowForever.exe"),
        ]);
        let folder = Path::new("Q:\\wow\\_forever_");
        let client = client_running_among(&processes, folder)
            .expect("a list")
            .expect("the client");
        assert_eq!(
            client.pid, 121,
            "the first process under the folder, any depth"
        );
        assert_eq!(client.exe_name(), "Helper.exe");
        assert_eq!(
            client_running_among(&processes, Path::new("Q:\\wow\\_classic_")).expect("a list"),
            None
        );
    }

    #[test]
    fn a_sibling_folder_with_the_same_prefix_is_not_the_client() {
        let processes = FixedProcesses::new([(7, "Q:\\wow\\_forever_2\\Wow.exe")]);
        let folder = Path::new("Q:\\wow\\_forever_");
        assert_eq!(
            client_running_among(&processes, folder).expect("a list"),
            None
        );
        let forms = [PathBuf::from("Q:\\wow\\_forever_")];
        assert!(!is_under(Path::new("Q:\\wow\\_forever_"), &forms));
        assert!(!is_under(Path::new("Q:\\wow"), &forms));
        assert!(!is_under(Path::new("Q:\\wow\\_forever_\\Wow.exe"), &[]));
    }

    #[test]
    fn a_component_matches_any_form_of_the_folder() {
        let forms = [
            PathBuf::from("C:\\Users\\runneradmin\\AppData\\Local\\Temp\\mrgl-test\\_forever_"),
            PathBuf::from("C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\MRGL-T~1\\_FOREV~1"),
        ];
        // Long and short components mixed, as a process started through a partly short path carries them.
        assert!(is_under(
            Path::new("C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\mrgl-test\\_FOREV~1\\Wow.exe"),
            &forms
        ));
        assert!(!is_under(
            Path::new("C:\\Users\\RUNNER~2\\AppData\\Local\\Temp\\mrgl-test\\_forever_\\Wow.exe"),
            &forms
        ));
        // A form with another number of components than the first is ignored, never a match.
        let odd = [
            PathBuf::from("Q:\\wow\\_forever_"),
            PathBuf::from("Q:\\elsewhere\\_forever_\\extra"),
        ];
        assert!(is_under(Path::new("Q:\\wow\\_forever_\\Wow.exe"), &odd));
        assert!(!is_under(
            Path::new("Q:\\elsewhere\\_forever_\\extra\\Wow.exe"),
            &odd
        ));
    }

    #[cfg(windows)]
    #[test]
    fn a_process_started_through_the_short_name_is_still_the_client() {
        let root = std::env::temp_dir().join(format!(
            "mrgl-client-short-name-test-{}",
            std::process::id()
        ));
        let folder = root.join("a folder name long enough for a short one");
        std::fs::create_dir_all(&folder).expect("the folder");
        let canonical = std::fs::canonicalize(&folder).expect("canonical");
        // With short names disabled on the volume the short form is the long one; the match still holds.
        // A process path carries no `\\?\` prefix, so the stub's does not either.
        let short = strip_verbatim(&system::short_form(&canonical).expect("a short form"));
        let processes = FixedProcesses::new([(5, short.join("Wow.exe"))]);
        let client = client_running_among(&processes, &folder)
            .expect("a list")
            .expect("the client");
        assert_eq!(client.pid, 5);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_compare_case_insensitively_and_without_the_verbatim_prefix() {
        assert!(is_under(
            Path::new("q:\\WOW\\_FOREVER_\\wow.exe"),
            &[PathBuf::from("Q:\\wow\\_forever_")]
        ));
        assert_eq!(
            strip_verbatim(Path::new("\\\\?\\Q:\\wow\\_forever_")),
            PathBuf::from("Q:\\wow\\_forever_")
        );
        assert_eq!(
            strip_verbatim(Path::new("\\\\?\\UNC\\server\\share\\_forever_")),
            PathBuf::from("\\\\server\\share\\_forever_")
        );
        assert_eq!(
            strip_verbatim(Path::new("Q:\\wow\\_forever_")),
            PathBuf::from("Q:\\wow\\_forever_")
        );
    }

    #[test]
    fn the_refusal_and_the_warning_are_the_documented_lines() {
        let client = RunningClient {
            pid: 4242,
            exe: PathBuf::from("Q:\\wow\\_forever_\\WowForever.exe"),
        };
        let flavor = flavor("Q:\\wow\\_forever_");
        assert_eq!(
            client.refusal(&flavor).to_string(),
            "the client is running in _forever_ (WowForever.exe, pid 4242): close the game first, nothing under WTF is written while it runs"
        );
        assert_eq!(
            client.install_warning(&flavor),
            "the client is running in _forever_ (WowForever.exe, pid 4242): the installed addons load at /reload or the next login"
        );
        assert_eq!(
            client.remove_warning(&flavor),
            "the client is running in _forever_ (WowForever.exe, pid 4242): the removed addons stay loaded until /reload or the next login"
        );
        let json = serde_json::to_value(&client).expect("serializes");
        assert_eq!(json["pid"], 4242);
        assert!(
            json["exe"]
                .as_str()
                .is_some_and(|exe| exe.ends_with("WowForever.exe"))
        );
    }

    #[test]
    fn a_missing_folder_is_taken_as_given() {
        let folder = std::env::temp_dir().join("mrgl-client-test-no-such-folder");
        assert_eq!(
            resolved(&folder),
            [std::path::absolute(&folder).expect("absolute")],
            "one form only, as given"
        );
        let processes = FixedProcesses::new([(9, folder.join("Wow.exe"))]);
        let client = client_running_among(&processes, &folder)
            .expect("a list")
            .expect("the client");
        assert_eq!(client.pid, 9);
    }

    /// A process list that cannot be taken.
    struct BrokenProcesses;

    impl ProcessList for BrokenProcesses {
        fn executables(&self) -> std::io::Result<Vec<RunningProcess>> {
            Err(std::io::Error::other("no snapshot"))
        }
    }

    #[test]
    fn a_process_list_that_cannot_be_taken_refuses_the_write_and_warns_the_install() {
        let flavor = flavor("Q:\\wow\\_forever_");
        let error = ensure_client_not_running_among(&BrokenProcesses, &flavor)
            .expect_err("refused rather than risked");
        assert!(matches!(error, Error::ProcessList { .. }), "{error}");
        assert_eq!(
            error.to_string(),
            "cannot list the running processes: no snapshot"
        );
        assert_eq!(
            warn_if_client_running_among(&BrokenProcesses, &flavor).as_deref(),
            Some("cannot list the running processes: no snapshot"),
            "the install goes on with the line"
        );
        // The same two calls over a list: the client found, and none.
        let running = FixedProcesses::new([(77, "Q:\\wow\\_forever_\\WowForever.exe")]);
        assert_eq!(
            ensure_client_not_running_among(&running, &flavor)
                .expect_err("the client runs")
                .to_string(),
            "the client is running in _forever_ (WowForever.exe, pid 77): close the game first, nothing under WTF is written while it runs"
        );
        assert_eq!(
            warn_if_client_running_among(&running, &flavor).as_deref(),
            Some(
                "the client is running in _forever_ (WowForever.exe, pid 77): the installed addons load at /reload or the next login"
            )
        );
        let none = FixedProcesses::default();
        assert!(ensure_client_not_running_among(&none, &flavor).is_ok());
        assert_eq!(warn_if_client_running_among(&none, &flavor), None);
    }

    #[test]
    fn an_executable_without_a_file_name_is_named_by_its_path() {
        let client = RunningClient {
            pid: 1,
            exe: PathBuf::from("Q:\\"),
        };
        assert_eq!(client.exe_name(), "Q:\\");
    }

    #[test]
    fn a_path_without_a_prefix_is_kept_as_it_is() {
        assert_eq!(
            strip_verbatim(Path::new("relative\\_forever_")),
            PathBuf::from("relative\\_forever_")
        );
        // An empty path cannot be made absolute: it is taken as given, one form.
        assert_eq!(resolved(Path::new("")), [PathBuf::new()]);
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_whose_names_are_short_already_has_one_form() {
        let system_root = PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot"));
        let forms = resolved(&system_root);
        assert_eq!(forms.len(), 1, "{forms:?}");
        assert!(
            forms[0]
                .to_string_lossy()
                .eq_ignore_ascii_case(&system_root.to_string_lossy()),
            "{forms:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_long_name_gets_a_short_form_when_the_volume_keeps_one() {
        let root =
            std::env::temp_dir().join(format!("mrgl-client-long-name-test-{}", std::process::id()));
        let folder = root.join("a folder name long enough for a short one");
        std::fs::create_dir_all(&folder).expect("the folder");
        let forms = resolved(&folder);
        assert!(!forms[0].to_string_lossy().starts_with("\\\\?\\"));
        match forms.as_slice() {
            // The volume keeps no short names: the short form is the long one and is not repeated.
            [_] => {}
            [long, short] => {
                assert_ne!(long, short);
                assert_eq!(
                    short,
                    &short_form(&folder).expect("a short form"),
                    "the second form is the 8.3 one"
                );
            }
            other => panic!("one or two forms, not {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn a_path_longer_than_the_first_buffer_still_gets_its_short_form() {
        // Six names of 200 characters each: beyond the 1024 units of the first buffer, within the
        // 32K Windows allows with the `\\?\` prefix.
        let root =
            std::env::temp_dir().join(format!("mrgl-client-deep-path-test-{}", std::process::id()));
        let mut deep = PathBuf::from(format!("\\\\?\\{}", root.display()));
        for level in 0..6 {
            deep.push(format!("{level}{}", "n".repeat(199)));
        }
        std::fs::create_dir_all(&deep).expect("the deep folder");
        assert!(deep.as_os_str().len() > 1024);
        let short = system::short_form(&deep).expect("a short form, after a second round");
        assert!(
            std::fs::metadata(&short).is_ok_and(|metadata| metadata.is_dir()),
            "{}",
            short.display()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
