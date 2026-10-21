//! The running-client check (task 023) against a real process: this test binary copied into a synthetic
//! flavor folder under a temporary dir and spawned as a stand-in client (never a game file), detected
//! while it runs and not after it is killed; the guard refuses with the documented line; the no-client
//! path over the fixture tree is cheap.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use mrgl_core::Error;
use mrgl_core::client;
use mrgl_core::flavor::{self, Flavor};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flavors")
}

/// A synthetic game folder under the temp dir, with a `.build.info` row for `wowf` and a `_forever_` folder.
fn temp_game_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mrgl-client-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("_forever_")).expect("the flavor folder");
    std::fs::write(
        root.join(".build.info"),
        "Product!STRING:0|Version!STRING:0\nwowf|1.60.1.70245\n",
    )
    .expect(".build.info");
    std::fs::write(
        root.join("_forever_").join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    )
    .expect(".flavor.info");
    root
}

/// A copy of this test binary in the flavor folder, started as the stand-in client: it runs the
/// `stand_in_client` test below, which waits for its stdin to close.
fn spawn_stand_in(flavor_folder: &Path) -> Child {
    let exe = std::env::current_exe().expect("this test binary");
    let name = format!(
        "client-stand-in{}",
        exe.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default()
    );
    let copy = flavor_folder.join(name);
    std::fs::copy(&exe, &copy).expect("the copy");
    Command::new(&copy)
        .args(["--exact", "stand_in_client", "--include-ignored"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the stand-in starts")
}

/// The stand-in client's body: it blocks until the parent closes its stdin or kills it. Ignored, so a
/// plain `cargo test` never runs it; the copy runs it by name.
#[test]
#[ignore]
fn stand_in_client() {
    let mut sink = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut sink);
}

fn read(root: &Path) -> Flavor {
    flavor::read_flavor(&root.join("_forever_")).expect("a flavor")
}

#[cfg(windows)]
#[test]
fn a_process_started_from_the_flavor_folder_is_the_client_until_it_ends() {
    let root = temp_game_root("detect");
    let flavor = read(&root);
    assert_eq!(client::client_running(&flavor).expect("a list"), None);
    assert!(client::ensure_client_not_running(&flavor).is_ok());
    assert_eq!(client::warn_if_client_running(&flavor), None);

    let mut child = spawn_stand_in(&flavor.folder);
    let client = client::client_running(&flavor)
        .expect("a list")
        .expect("the stand-in is the client");
    assert_eq!(client.pid, child.id());
    assert_eq!(client.exe_name(), "client-stand-in.exe");
    // The reported path may carry 8.3 components (a `RUNNER~1` temp folder on a CI runner), so the
    // folder is compared by its canonical form only.
    let reported = std::fs::canonicalize(client.exe.parent().expect("a parent")).expect("exists");
    assert_eq!(
        reported,
        std::fs::canonicalize(&flavor.folder).expect("exists"),
        "{}",
        client.exe.display()
    );

    let error = client::ensure_client_not_running(&flavor).expect_err("the guard refuses");
    assert!(
        matches!(&error, Error::ClientRunning { flavor, client } if flavor == "_forever_" && client.pid == child.id()),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "the client is running in _forever_ (client-stand-in.exe, pid {}): close the game first, nothing under WTF is written while it runs",
            child.id()
        )
    );
    assert_eq!(
        client::warn_if_client_running(&flavor).as_deref(),
        Some(
            format!(
                "the client is running in _forever_ (client-stand-in.exe, pid {}): the installed addons load at /reload or the next login",
                child.id()
            )
            .as_str()
        )
    );

    child.kill().expect("killed");
    child.wait().expect("waited");
    assert_eq!(client::client_running(&flavor).expect("a list"), None);
    assert!(client::ensure_client_not_running(&flavor).is_ok());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_check_is_cheap_when_no_client_runs() {
    let flavor = flavor::read_flavor(&fixtures().join("_forever_")).expect("a flavor");
    let started = Instant::now();
    for _ in 0..10 {
        assert_eq!(client::client_running(&flavor).expect("a list"), None);
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "ten checks took {elapsed:?}: one snapshot and no file I/O per process is the budget"
    );
}
