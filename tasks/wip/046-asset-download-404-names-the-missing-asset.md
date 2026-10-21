---
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 046 — The asset download's error path: a 404 names the asset, the size is checked

## Context

When the GitHub source's asset download answers 404, the error names the asset and the release
instead of the release-list hint ("no such repository, or it is private"); and the downloaded asset's byte
count is compared with the `size` the release list names for it, a mismatch refused before the engine
opens the zip, with a line that names both numbers (a release without a size installs as before). The
hint sends the user to check the repository name when the asset was renamed, and a truncated download
today surfaces only as an unreadable zip, which reads like a packaging bug.
Done when: a fixture release whose asset is missing from the stub fails `add` with a line naming the
asset, a fixture asset shorter than its listed size is refused with the documented line, and the
release-list 404 keeps its hint.

Decided 2026-10-22: second of the chain 042 → 046 → 041.

## Docs that move

- `docs/development.md` (the source's errors) if the error list there changes; otherwise `none`.

## Not in this change

- The offline resolve (041, next in the chain); a retry of a failed download; a checksum from the release
  (GitHub publishes none).

## Steps

1. [ ] The download's 404 is its own error naming the asset and the release tag; the release-list 404
       keeps its hint
2. [ ] The byte count of the downloaded asset against the release list's `size`; a mismatch refused with
       both numbers before the engine opens the zip; no `size` installs as before
3. [ ] Tests on the stub: a missing asset, a short asset, a release without a size, the release-list 404
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 042 → 046 → 041

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
