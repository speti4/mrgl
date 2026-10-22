---
size: S
origin: done/065-derive-name-prefers-the-zip-or-repository-folder.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 077 — The marks survive a reinstall under a new derived name, and the pinned line uses the version order

## Context

When a zip in hand is staged under a name the naming rule now derives (065) while the sidecar records
the same folders under the old name, the commit carries the old entry's pin, channel and previous version
over to the new entry instead of dropping them with a warning; and the update check's pinned line applies
the version order of 068, so an installed version newer than the current release is not reported as `<latest>
is out`. An addon recorded as `DBM-Brawlers` before 065 loses its marks on the next install from a zip, and
the window shows "pinned at 1.1.0-beta, 1.0.0 is out" (010).
Done when: a fixture sidecar with the old name, a pin and a previous version keeps all three after the
reinstall, and a pinned fixture addon newer than the current release reports the pin without `is out`.

Decided 2026-10-22: first of the chain 077 → 075 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the install engine's commit rule for an entry recorded under another name, and
  the pinned line of the update check.

## Not in this change

- A rename command for a managed addon; the window's pin at a given version; the version order itself (068).

## Steps

1. [ ] `install.rs`: when the staged name differs from the entry that records the same folders, the commit
       carries the old entry's pin, channel and previous version (and its rollback zip) over to the new entry
2. [ ] `update.rs`: the pinned line uses the version order of 068 (`listed_versions`, else `lenient_order`), so
       a pinned installed version newer than the current release carries no `<latest> is out`; the window's
       note (`ui/actions.js`) and the CLI line agree
3. [ ] Tests: a fixture sidecar with the old name, a pin and a previous version keeps all three after the
       reinstall from a zip; a pinned fixture addon newer than the current release reports the pin without
       `is out`; the CLI text and JSON, the window's row
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 077 → 075

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
