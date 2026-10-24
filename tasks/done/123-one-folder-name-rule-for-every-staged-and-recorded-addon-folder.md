---
size: M
origin: done/121-a-hardening-review-of-the-code-that-touches-the-disk-the-network-and-the-keys.md
created: 2026-10-24
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M5 Trust
---

# 123 — One folder-name rule for every staged and recorded addon folder

## Context

Finding H1 of the hardening review (121): a `.toc` at a zip's root whose stem is `.`, `..` or blank (`..toc`,
`...toc`, ` .toc`) becomes the addon's folder name unchecked (`install.rs`, `root_toc_name`), the swap then
joins it to `Interface\AddOns`, and Windows resolves `AddOns\.` and `AddOns\ ` to `AddOns` and `AddOns\..` to
`Interface`: the install replaces the whole AddOns folder with the zip's files (reproduced), and with the mrgl
home on another drive than the game it copies `Interface` aside, removes it and writes the zip into its place.
A later `remove` of the recorded entry removes the same path. Every install path reaches it: a zip in hand,
every source, `import`, `update`, `rollback` and the window's update.
Why: a crafted or merely odd zip must never write outside `Interface\AddOns\<folder>`; M5 is the trust
milestone.
Done when: one folder-name rule (not empty, not `.` or `..`, no separator, `:` or control character, no
trailing dot or space, not a reserved device name) is applied to every staged folder before anything is
unpacked into place (a root TOC whose stem fails it falls back to the zip's stem, else the zip is refused with
a line) and to every folder read from the sidecar before a commit, a remove or a link check; the three
near-duplicates of the rule (`adopt.rs`, `list_file.rs`, `sets.rs`) call that one function; regression tests
install the three zips and assert that `AddOns` and `Interface` are untouched and the sidecar records nothing.

Decided 2026-10-24: filed and started at once from the review's list; the fix stays in the core (the CLI and
the window only carry the refusal line through the voice).

## Docs that move

- `docs/development.md`: the install engine's folder-name rule (where the zip entry rules are described), the
  fixtures paragraph for the new zips
- `crates/mrgl-core/tests/fixtures/README.md`: the three zips

## Not in this change

- The other findings of 121 (124 bundles the low ones)
- A change to the zip entry name rules beyond what the folder rule needs (L5 and L6 are 124's)

## Steps

1. [x] `crates/mrgl-core/src/install.rs` (or a small module the three callers share): the one rule as a
       function with a unit test per case; `root_toc_name` and `find_addons` apply it to every staged folder
       in `Engine::stage`, before the unpack into place: a root TOC stem that fails it falls back to the zip's
       stem, a folder that still fails refuses the zip with a line through the voice
2. [x] `Engine::commit`, `remove`, `rollback` and `refuse_links` apply it to a folder read from the sidecar
       (a sidecar written by an older build could carry one), refusing with a line instead of joining the path
3. [x] `adopt.rs`, `list_file.rs` and `sets.rs` call the one function instead of their own checks
4. [x] Fixtures: zips with `..toc`, `...toc` and ` .toc` at the root (made by `zips/make-zips.py`), and the
       end-to-end tests: `add` of each refuses or falls back, `AddOns` and `Interface` untouched, the sidecar
       unchanged, exit code 1 with the line; a sidecar entry with folder `..` is refused by `remove`
5. [x] The docs

## Log

- 2026-10-24 — filed and promoted
- 2026-10-24 — The rule is `zip_entry::is_folder_name`, next to the zip entry rule, whose character and
  device-name checks it shares through one private function (`refused_by_windows`): not empty, not `.` or
  `..`, no `/` or `\`, no `:`, no control character and none of `<>"|?*`, no trailing dot or space, no
  reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1` … `COM9`, `LPT1` … `LPT9`, with or without an
  extension); a leading space and an inner dot pass. One unit test per case in `zip_entry::tests`, and one
  that shows the entry rule lets `..toc`, `...toc` and ` .toc` through.
- 2026-10-24 — Staging: `root_toc_name` never takes a stem that fails the rule, so `..toc`, `...toc` and
  ` .toc` fall back to the zip's stem, as root TOCs without a common stem already did; `Engine::stage` then
  holds every staged folder to the rule before anything leaves the staging folder, and a folder that still
  fails (a zip whose own stem fails too, `..zip`, `...zip`, ` .zip`) refuses the zip with
  `Error::FolderNameRefused` (``refused <zip>: its addon folder would be `..`, which is no folder name under
  Interface\AddOns (…); nothing was installed``). Decided 2026-10-24: the fallback, as the task's Done-when
  says, since the zip's stem is the name the engine already gives root TOCs it cannot name; the refusal
  stays for the case where the stem fails as well.
- 2026-10-24 — Sidecar: `Engine::refuse_recorded_folders` refuses the first recorded folder that fails the
  rule with `Error::RecordedFolderRefused` (``refused: the sidecar of <flavor> records the folder `..`, …;
  nothing was moved or removed, and the entry is yours to correct in <state.json>``), before any path is
  joined. `Engine::remove` and `Engine::rollback` call it on their entry (the rollback before it stages the
  cached zip), and `Engine::refuse_links` calls it first, so every commit (it runs before the first folder
  moves, over the staged folders and the dropped folders of the previous version) and the dry run of `add`
  are covered.
- 2026-10-24 — The window: its update and its add call `Engine::install_from_source` and `Engine::install`,
  which stage and commit through the engine, so the core's refusal covers them and the window shows the
  error line it shows for every failed install; no window code changed.
- 2026-10-24 — The near-duplicates: `GithubMapping::problems` (adopt) calls the rule and keeps its own
  leading-space check (its schema refuses surrounding space); `ListEntry::not_shareable` calls the rule
  without the trim it had and says ``its folder `<folder>` is not a folder name``; `sets.rs`'s `is_plain_name`
  is gone, the undo record's names go through the rule.
- 2026-10-24 — The other joins of a recorded folder to `Interface\AddOns` read only: `unlink` takes the
  `symlink_metadata` of the path and removes a junction only (a real folder is left in place), the
  SavedVariables cleanup's `read_dir` of a skipped folder comes from the scan, not the sidecar; they stay as
  they are.
- 2026-10-24 — Fixtures: `dot-toc.zip`, `dot-dot-toc.zip` and `blank-toc.zip`, each a root TOC and one Lua
  file (339 to 341 bytes), written by `make-zips.py`; a rerun left every other fixture zip byte-equal.

## Deviations from the plan

- Step 2: `Engine::commit` gets no check of its own: its first call is `Engine::refuse_links`, which applies
  the rule to every folder the commit is about to touch, so one check covers the commit and the dry run of
  `add` alike, and no second copy is a branch no test reaches.
- The rule also refuses `<>"|?*`, the characters the zip entry rule refuses, since the two share one check;
  Windows creates no name with them either.
- `list_file.rs` changed its message from `its folder <folder> is a path, not a folder name` to ``its folder
  `<folder>` is not a folder name``, since a name like `CON` or `Some.` is no path, and it checks the folder
  as written instead of trimmed.

## Verification

- 2026-10-24 — `cargo test --locked`: every test passes, among them the new ones: `zip_entry::tests` (ten,
  one per case), `install::tests::a_root_toc_stem_that_is_no_folder_name_falls_back_to_the_zip_stem`, the
  core's `tests/install.rs` (`a_root_toc_whose_stem_is_no_folder_name_installs_under_the_zip_stem_and_nowhere_else`,
  `a_root_toc_and_a_zip_stem_that_are_no_folder_names_refuse_the_zip_and_touch_nothing`,
  `a_recorded_folder_that_is_no_folder_name_is_refused_by_remove_rollback_and_a_commit`: the game folder
  and the mrgl home compared file by file before and after every refused call) and the command line's
  (`add_of_a_root_toc_whose_stem_is_no_folder_name_installs_under_the_zip_stem`,
  `add_refuses_a_zip_whose_root_toc_and_stem_are_no_folder_names_with_exit_code_one`, with and without
  `--dry-run`, exit code 1, the murloc line and the plain line, and
  `remove_refuses_a_sidecar_entry_whose_folder_is_dot_dot`).
- 2026-10-24 — The same six end-to-end tests with the rule switched off for one run (a temporary early
  `return true` in `is_folder_name`, then removed): all six fail. The install records the folder `.`, the
  copy `..zip` installs as `.` instead of being refused, and `remove` of the entry that records `..`
  succeeds; with the rule on, all six pass.
- 2026-10-24 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked`, `python -I scripts/usage-guide.py --check`: clean.
- 2026-10-24 — The coverage measurement of CONTRIBUTING (the pinned nightly, `cargo llvm-cov --workspace
  --branch`, `scripts/coverage-check.py`): 100.00% lines (0 missed of 15710) and 100.00% branches (0 missed of
  2282) on all three crates. It ran twice. During the first run a doc comment in `list_file.rs` was edited, so
  the check read the `coverage: os` marks two lines off the instrumented build and reported a false hit. The
  second run, on the settled tree, passed.
- 2026-10-24 — `python -I scripts/changelog.py` regenerated `CHANGELOG.md`; `--check` passes.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-24`
