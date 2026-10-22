---
size: S
origin: done/084-stale-staging-folders-are-swept.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 088 — The doctor names a staging folder kept for its `previous\` folders

## Context

`mrgl doctor` lists every run folder under `<flavor-id>\staging\` that the sweep of 084 keeps because
it holds a `previous\` folder (the addon folders a failed swap moved aside), with its age and size, and says
what to do with it.
Why: after the error of a failed revert nothing points the user to that folder again, and the sweep never
removes it.
Done when: a planted run folder with a `previous\` folder shows in the doctor's text and JSON, and one
without it does not.

Decided 2026-10-22: the first of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the doctor's checks (the kept staging folders line and its JSON)

## Not in this change

- Removing or restoring the kept folder for the user (the doctor names it, the user decides)
- The sweep rules of 084

## Steps

1. [x] `doctor.rs`: a check over `<flavor-id>\staging\` listing every run folder with a `previous\` folder,
       with its age and size and the advice line, in the text and the JSON
2. [x] Tests: a planted run folder with `previous\` shows, one without it does not

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-core/src/staging.rs`: `kept(staging)` lists every folder the sweep keeps for
  its `previous\` folder, as a `KeptRun` (the path, the creation time from the name or else the modified
  time, the names under `previous\`), sorted by name. A folder whose lock a running mrgl holds is a swap in
  progress and is left out. One unit test.
- 2026-10-22 — `crates/mrgl-core/src/doctor.rs`: `FlavorReport::kept_staging` (`KeptStaging`: `path` with
  the profile folder as `~`, `created` in UTC, `age_seconds`, `bytes`, `files`, `previous`, `advice`), for a
  flavor folder that is there and for a missing one alike. The block gains
  `  kept staging: <folder> (<age> old, <size> in <n> files; previous: <names>)` per folder after the cache
  line, then once `  kept staging advice: <KEPT_STAGING_ADVICE>`; the summary names
  `a kept staging folder of <flavor folder>` (or the count), so it turns into a warning. Ages read in whole
  units (`less than a minute`, `5 minutes`, `2 hours`, `3 days`). Two unit tests.
- 2026-10-22 — `crates/mrgl-cli/tests/reports.rs` (new, so the shared `tests/cli.rs` stays untouched): the
  doctor's text and JSON over a planted kept folder and a planted plain one, then over the same folder once
  its `previous\` is gone.
- 2026-10-22 — docs: `docs/development.md` (the doctor's line and its JSON fields, the summary's list, a
  pointer from the staging folder's lifetime), `README.md` (the doctor paragraph).

## Deviations from the plan

- The listing of kept folders lives in `mrgl_core::staging` (`kept`), next to the sweep whose rule it
  mirrors; `doctor.rs` adds the age, the size and the lines.
- A kept folder is also a concern of the summary, so `mrgl doctor` ends with a warning while one is there.

## Verification

- 2026-10-22 — `mrgl-core` unit tests: a planted folder named two hours old with `previous\OneFolder`
  shows as `  kept staging: ~\AppData\Local\mrgl\wowf\staging\<name> (2 hours old, 30 B in 2 files;
  previous: OneFolder)` with the advice line after it, its JSON entry carries the path under `~`, the
  bytes, the files, the names, the advice and an age of two hours; a planted folder without `previous\`
  is not named, and once the `previous\` folder is removed the list is empty and no line is printed.
- 2026-10-22 — end to end (`tests/reports.rs`): `mrgl --serious doctor --flavor forever` prints the line
  and the advice and ends with `diagnostics of 1 flavor, 1 thing to look at (a kept staging folder of
  _forever_)`; `--json` carries one `kept_staging` entry; without `previous\` both say nothing of it.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, `mrgl-core` lib 204), `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
