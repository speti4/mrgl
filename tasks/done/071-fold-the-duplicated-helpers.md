---
size: S
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 071 — Fold the duplicates: one `WTF` walker, one size formatter, no dead error variant

## Context

`sets` and `savedvars` share one account, realm and character walker and one set of `WTF` constants; the
CLI's `human_bytes` and the core's `size_text` become one; `Error::NotImplemented` goes now that no stub
calls it; the stand-in test files stop importing what Linux does not use (the `non-windows` job's
warnings). Each was written twice in parallel branches and will drift apart.
Done when: one walker with both callers, one formatter, the variant gone, `cargo check` on Linux without
warnings, and every existing test unchanged.

Decided 2026-10-22: first of the chain 071 → 072 on one branch. The window's `update_all` stub still
answers `Error::NotImplemented` until 010 replaces it; this task removes the variant only together with
that stub's error (another existing variant or a plain message), so the chain does not wait for 010.

## Docs that move

- `docs/development.md`: the module map, where a helper moves.

## Not in this change

- The third size formatter's call sites' wording (the doctor's `bytes_text` joins the one formatter only if
  its output is byte-identical; otherwise it stays and is named as a follow-up); any behavior change in
  `sets`, `savedvars` or `doctor`.

## Steps

1. [x] One `WTF` walker (account, realm, character; the symlink warning of `savedvars` kept) and one set of
       constants, called by `sets` and `savedvars`
2. [x] One size formatter in the core, used by the CLI and the core (and the doctor if byte-identical)
3. [x] `Error::NotImplemented` removed with its last caller in `mrgl-app`; the stand-in test files gate
       their `std::process` imports with `cfg(windows)`
4. [x] Tests unchanged and green; `cargo check --all-targets` on the non-Windows path without warnings
       (the `non-windows` job's command from `.github/workflows/ci.yml`)
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 071 → 072
- 2026-10-22 — `wtf.rs`, a new module of the core: `ACCOUNT_DIR` and `SAVED_VARIABLES_DIR` (the one set,
  removed from `sets` and `savedvars`; `WTF` itself stays `backup::WTF_DIR`) and `wtf::walk(wtf, warnings,
  visit)`, which calls `visit` with `WtfFolder::Account` and then each `WtfFolder::Character` of that
  account, each level by name, the account's `SavedVariables` never a realm, a missing folder empty, a link
  or a junction a warning (`link_warning`, moved from `savedvars`) and never followed. The visit gets the
  warnings, so `savedvars` keeps every warning in the order it had (the account's SavedVariables read
  before its realms, each character's before the next realm). `sets::characters_in` collects the
  `Character` visits and passes the link warnings over, as its own walker did (a link was never a
  character there either); `savedvars::list_at` collects the account and character SavedVariables folders
  through the same walk. Both private `subfolders` functions are gone.
- 2026-10-22 — the size formatter: `zip_entry::size_text` is the one (its doc says so); the CLI's
  `human_bytes` is gone and its seven call sites call `zip_entry::size_text`; the doctor's `bytes_text` is
  an import of `size_text` under its old name, so its call sites and its test stay as they were. The three
  functions were the same code but for the last unit (`TB` in the CLI and the doctor, `PB` in
  `zip_entry`), so every size below 1 PiB reads byte for byte as before, the doctor test's four values
  included.
- 2026-10-22 — `Error::NotImplemented` removed; the window's `update_all` answers the same warning as a
  plain line, `update all in the window is not implemented yet (task 010)`, so its test
  (`update_all_answers_with_its_task`) is unchanged.
- 2026-10-22 — the non-Windows warnings, measured with `cargo check --all-targets --locked --target
  x86_64-apple-darwin` (the only non-Windows standard library installed here; the repo has no
  `target_os` or `unix` cfg, so every `cfg(windows)` branch reads the same as on Linux), with a stand-in C
  compiler for `ring`'s build script (a check does not link): before the change, `tests/client.rs`,
  `backup.rs`, `savedvars.rs`, `sets.rs` warned about `Child`, `Command`, `Stdio`, `client.rs` also about
  `Error`, `temp_game_root` and `read`, and `mrgl-cli/tests/cli.rs` about `Child`. Each is now under
  `#[cfg(windows)]` (in `client.rs` the `Flavor` import too, which only `read` uses).
- 2026-10-22 — `wtf::tests`: the walk's order over a temporary tree (accounts before their characters,
  each level by name, `SavedVariables` not a realm, files passed over), a missing `WTF` or `Account`
  folder with nothing under it, and on Windows a junction under a realm that is a warning and not visited.

## Deviations from the plan

- The walker lives in a new module, `wtf`, rather than in `sets` or `savedvars`: neither owns the other's
  folders.
- `tests/cli.rs` of `mrgl-cli` also imported `Child` ungated; it is gated with the four stand-in files, so
  the `non-windows` job has no warning left.
- A size of 1 PiB or more now reads in `PB` in the CLI and the doctor (`1.0 PB` where it was `1024.0 TB`):
  the one formatter is the one with the larger last unit.
- An I/O error of the walk on an entry's type names the entry's path in `sets` too, as `savedvars` did
  (`sets` named the folder being read).

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 176 with the three new `wtf` tests,
  `tests/sets.rs` 12, `tests/savedvars.rs` 6, `tests/backup.rs` 10, `tests/client.rs` 2, `tests/cli.rs`
  114, the window's 5 unit and 11 `flavor_list` tests); no existing test body changed (the test files
  changed only in `cfg(windows)` lines on their imports and two helpers). `cargo fmt --check`, `cargo
  clippy --all-targets --locked -- -D warnings` and `cargo +1.90 check --all-targets --locked` green.
- 2026-10-22 — `cargo check --workspace --all-targets --locked --target x86_64-apple-darwin` finishes with
  no warning (before: 8 warnings in five test files). Linux itself was not run here: the `non-windows` job
  is the check on Linux, and its graph differs only in the window's GTK libraries, not in the repo's cfg
  branches.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
