---
size: S
origin: done/010-update-all-and-update-one-in-the-window.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 079 — Window and exe polish after the first release: the checks after an update, the title bar theme, the CLI exe's version

## Context

After an update the window runs the dependency and interface checks the CLI's install summary reports
and shows their warnings; the Tauri window's theme follows the preference (`auto`, `light`, `dark`) so the
native title bar matches the page instead of the system theme; and `mrgl.exe` carries the workspace version
in its file properties through the CLI's build script, the way it carries the icon (012). The window's update
is silent about a dependency that went missing (010), a light page under a dark title bar looks broken on a
dark system (012), and the CLI exe's properties show no version while the app exe's do (012).
Done when: a flavor list test sees the dependency warning after a fixture update, the window's title bar is
light in the light theme on a dark system, and the exe's version resource reads the `Cargo.toml` version.

Decided 2026-10-22: second of the chain 078 → 079 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the update commands' answer (the checks after an update), the preferences
  paragraph (the native theme follows the preference), and the CLI's `build.rs` paragraph (the version
  resource next to the icon).

## Not in this change

- Signed exes (M5); a version resource for the app exe (tauri-build stamps it); the window's cached view
  (078).

## Steps

1. [x] `mrgl-app/src/actions.rs`: after an update the dependency and interface checks of the installed
       folders run as the CLI's install summary does, their warnings in the answer's messages and the row's
       `missing`
2. [x] The Tauri window's theme follows the preference: `auto` leaves the system's, `light` and `dark` set
       the window theme at start and on a change of the preference, so the title bar matches the page
3. [x] `mrgl-cli/build.rs`: a `VS_VERSION_INFO` resource with the workspace version, written into the same
       `.res` as the icon with the standard library alone; other targets skip it
4. [x] Tests: a flavor list test sees the dependency warning after a fixture update; the preferences test
       covers the theme the window applies; the exe's version resource reads the `Cargo.toml` version
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 078 → 079
- 2026-10-22 — the checks after an update: `Engine::check_installed(folders)` in the core runs, on one
  scan, the dependency check of the installed folders against the flavor as it is now and the interface
  check of the primary folder, and `InstalledChecks::lines(folder_name)` gives their warnings (a scan that
  failed, each missing dependency, then `InterfaceMismatch::after_install_line`). The command line's
  install summary calls it in place of its own `installed_dependencies` and `installed_mismatch`, which are
  gone, and the after-install interface line moved into the core with the same text, so the command line
  says what it said. `actions::run` adds the lines of every addon it installed to the answer's warnings;
  the row's `missing` comes from the rows read again after the run, as before.
- 2026-10-22 — the title bar: `commands::native_theme` maps the preference to the window's theme (`light`
  and `dark` to `tauri::Theme`, `auto` to `None`, the system's) and `apply_native_theme` sets it through
  `WebviewWindow::set_theme`; the `setup` hook of `lib.rs` applies the stored preference to the main
  window at start, and `set_preferences` (which takes the calling window) applies the answer's theme after
  every store. On Windows, Tauri hands the window's theme to WebView2 as its preferred color scheme, so
  `prefers-color-scheme` in the page follows the window too; the page sets `data-theme` for `light` and
  `dark` itself, and `auto` follows the system as before. A platform that cannot set the theme leaves the
  title bar as it is; that is no error.
- 2026-10-22 — the CLI exe's version: `crates/mrgl-cli/build.rs` writes one `.res` (now `mrgl.res`) with the
  icon's resources and an `RT_VERSION` resource (id 1, US English): `VS_FIXEDFILEINFO` with the package
  version as `major.minor.patch.0` for the file and the product (the prerelease flag for a version with a
  suffix, `VOS_NT_WINDOWS32`, `VFT_APP`), a `StringFileInfo` table `040904B0` with `FileDescription`,
  `FileVersion`, `InternalName`, `OriginalFilename`, `ProductName` and `ProductVersion`, and the
  `VarFileInfo` translation. The standard library alone, as for the icon; other targets skip it. No
  copyright or company string: the app exe takes its copyright from `tauri.conf.json`, and the build
  script reads no JSON.
- 2026-10-22 — tests: `tests/update_actions.rs` `an_update_reports_the_dependency_and_interface_checks`
  (the fixture zips `needs-library-1.0.0` and `stale-interface-1.0.0` installed as 0.9.0 and updated by a
  row's "Update"); `commands.rs` `the_window_takes_the_theme_the_preference_applies`; `tests/cli.rs`
  `the_exe_carries_the_workspace_version_in_its_properties` (Windows with MSVC), which reads the built
  `mrgl.exe` through `mrgl_core::pe::read_file_version`.

## Deviations from the plan

- The install checks moved into the core (`Engine::check_installed`, `InterfaceMismatch::after_install_line`)
  so the window and the command line share them, instead of the window repeating the command line's
  helpers.
- The `.res` the build script writes is named `mrgl.res` instead of `icon.res`, since it holds the version
  too.
- The version resource is read back from the built exe itself; the `.res` fallback was not needed.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`mrgl-app` unit 6, was 5; `tests/update_actions.rs`
  10, was 9; `tests/cli.rs` 119, was 118; `tests/flavor_list.rs` 14, core unit 186 and the other core tests
  unchanged; the Node tests through `ui_smoke`). A row's update of `NeedsLibrary` from 0.9.0 to 1.0.0 answers
  the warning `NeedsLibrary needs the missing dependency LibMissing: install it or the addon does not load`
  and `1 addon updated`, and the row reads 1.0.0 with `missing` `LibMissing`; the update of
  `StaleInterface` answers `StaleInterface: interface 110207 is not for the build 1.60.1.70245 of
  _forever_; the zip may be for another flavor`. The command line's `add` and `import` tests of the
  dependency and interface warnings pass unchanged on the shared core check.
- 2026-10-22 — the window (`target/debug/mrgl-app.exe` of the worktree) on a temporary copy of the fixture
  game folder (`MRGL_ROOT`, `MRGL_HOME` with a `config.json` of its own, `MRGL_DISCOVERY=off`), on a
  system in the dark app theme, its top 90 pixels captured from the screen: with `theme` `light` the title
  bar is light (a pixel 239,247,236) over the light page (244,247,246), with `auto` and with `dark` it is
  dark (30,33,29) over the dark page (20,32,31). The change through the page's toggle (`set_preferences`)
  was not driven by hand; it runs the same `apply_native_theme` as the start. No real game folder was read.
- 2026-10-22 — the built `target/debug/mrgl.exe`: Windows reads `FileVersion` and `ProductVersion` `0.1.0`,
  `FileVersionRaw` 0.1.0.0, `FileDescription` `mrgl command line`, `OriginalFilename` `mrgl.exe`, not a
  prerelease, language English (United States); its associated icon is still the murloc.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the regeneration. No
  manifest changed, no new crate.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
