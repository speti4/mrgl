---
size: S
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
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

1. [ ] One `WTF` walker (account, realm, character; the symlink warning of `savedvars` kept) and one set of
       constants, called by `sets` and `savedvars`
2. [ ] One size formatter in the core, used by the CLI and the core (and the doctor if byte-identical)
3. [ ] `Error::NotImplemented` removed with its last caller in `mrgl-app`; the stand-in test files gate
       their `std::process` imports with `cfg(windows)`
4. [ ] Tests unchanged and green; `cargo check --all-targets` on the non-Windows path without warnings
       (the `non-windows` job's command from `.github/workflows/ci.yml`)
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 071 → 072

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
