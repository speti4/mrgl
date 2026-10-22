---
size: S
origin: done/098-adopt-apply-writes-sidecar-entries-without-a-reinstall.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 110 — The window's install paths warn in a folder the CurseForge app manages

## Context

The window's add and update actions warn in a flavor folder with a `.curseclient` marker, with the
core's line of 098 (the CurseForge app may still update or flag those folders), shown once per run on the
flavor's view, as `add`, `import` and `update` do on the command line.
Why: 098 added the warning to the three command-line paths and left the window's actions without it, so
two managers on one folder is said in one place and not the other.
Done when: a window test over a marked flavor folder shows the warning on add and on update, and an
unmarked folder shows none.

Decided 2026-10-22: the first of two window follow-ups of adopt on one branch (110, 109), on the branch
of 097 and 098.

## Docs that move

- `docs/development.md`: the window's actions and the marker warning

## Not in this change

- Removing the marker; the adopt view itself (109)

## Steps

1. [x] `actions.rs`: the add and the update actions carry the core's marker warning once per run; the UI
       shows it on the flavor's view
2. [x] Tests: a window test over a marked flavor folder (add and update) and an unmarked one; the Node tests
       where the UI renders the warning

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `actions::run`: the first warning of the answer is `adopt::curse_client_warning` of the
  engine's flavor, through the run's one `warn`, so it is said once per run whatever else warns; the
  check, "update all" and a row's update alike, the cancelled run too. No progress event repeats it. The
  view needs no change: it says an answer's warnings in order (a toast and the status bar), the summary
  last, on the flavor the run was started on.
- 2026-10-22 — the sketch (`mock-data.js`): a run in a flavor with `curse_client` (retail, classic, era)
  answers the core's line first, in the words of `adopt::curse_client_warning`; Forever has none.
- 2026-10-22 — tests: `tests/update_actions.rs` `a_folder_the_curseforge_app_manages_warns_once_per_run`
  (a `.curseclient` file in the made-up Forever flavor: the check, "update all" and a row's update each
  answer the line once, first, as a warning, no event repeats it, the updates install and the marker stays)
  and `a_folder_without_the_curseforge_marker_has_no_such_warning`; the Node test of the sketch's runs in
  three marked flavors and in Forever.
- 2026-10-22 — docs: `docs/development.md`, the marker paragraph of "Adopt" and the update actions'
  commands.

## Deviations from the plan

- The window has no add (nor import) action: installing a new addon is the command line's. Its install
  paths are the update actions, so the warning is on "Check for updates", "Update all" and a row's
  "Update", as `mrgl update` (and its dry run) warns; the done-when line's "add" has no window path to test.
- The check alone warns too, as `mrgl update --dry-run` does on the command line: it is the first step of
  every update run, and the line names the folder, not the install.

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them `crates/mrgl-app/tests/update_actions.rs`
  (14, with the two marker tests) and `ui_smoke.rs`, whose `node --test` runs the Node tests of `tests/ui/`
  (82, with the marker test of the sketch). `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green; `tasks_check` without `write`: only the stale index,
  which the main session regenerates.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
