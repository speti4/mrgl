//! The stand-in client (task 063): a copy of the signed `ping.exe` of the system folder, named
//! `client-stand-in.exe` in the flavor folder and pinging the loopback address for an hour, so it runs until
//! the test stops it. Never a copy of the test binary: a binary that copies itself and runs the copy is what
//! an antivirus heuristic reads as a dropper. Where the system folder has no `ping.exe`, [`spawn_stand_in`]
//! answers `None` and the test is skipped with [`SKIPPED`].

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// The line a test prints when it is skipped for want of `ping.exe`.
pub const SKIPPED: &str = "skipped: no ping.exe in the system folder to start the stand-in client";

/// Copies `ping.exe` into `flavor_folder` as `client-stand-in.exe` and starts it; `None` when the system
/// folder has no `ping.exe`.
pub fn spawn_stand_in(flavor_folder: &Path) -> Option<StandIn> {
    let ping = PathBuf::from(std::env::var_os("SystemRoot")?)
        .join("System32")
        .join("ping.exe");
    if !ping.is_file() {
        return None;
    }
    let copy = flavor_folder.join("client-stand-in.exe");
    std::fs::copy(&ping, &copy).expect("the copy");
    let child = Command::new(&copy)
        .args(["-n", "3600", "127.0.0.1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the stand-in starts");
    Some(StandIn {
        child,
        stopped: false,
    })
}

/// The running stand-in client: `stop` kills and waits for it, and so does a drop, so a failing assertion
/// leaves no process behind.
pub struct StandIn {
    child: Child,
    stopped: bool,
}

impl StandIn {
    /// The process id the running-client check reports.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Kills the stand-in and waits until it is gone.
    pub fn stop(&mut self) {
        self.child.kill().expect("killed");
        self.child.wait().expect("waited");
        self.stopped = true;
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
