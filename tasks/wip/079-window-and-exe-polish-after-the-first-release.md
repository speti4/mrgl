---
size: S
origin: done/010-update-all-and-update-one-in-the-window.md
created: 2026-10-22
started: 2026-10-22
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

1. [ ] `mrgl-app/src/actions.rs`: after an update the dependency and interface checks of the installed
       folders run as the CLI's install summary does, their warnings in the answer's messages and the row's
       `missing`
2. [ ] The Tauri window's theme follows the preference: `auto` leaves the system's, `light` and `dark` set
       the window theme at start and on a change of the preference, so the title bar matches the page
3. [ ] `mrgl-cli/build.rs`: a `VS_VERSION_INFO` resource with the workspace version, written into the same
       `.res` as the icon with the standard library alone; other targets skip it
4. [ ] Tests: a flavor list test sees the dependency warning after a fixture update; the preferences test
       covers the theme the window applies; the exe's version resource reads the `Cargo.toml` version
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 078 → 079

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
