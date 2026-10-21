---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M5 Trust
---

# 039 — A bound on the unpacked size of a zip

## Context

The install engine (004) stops unpacking when a zip's declared or actual unpacked size passes a
bound (a generous constant, overridable by a flag for a known large addon), reports it as the documented
error and leaves the staging folder clean. A zip bomb from a compromised source would fill the disk from
the staging folder; the hardening review of M5 asks for a bound on every untrusted input.
Done when: a synthetic zip that declares more than the bound is refused before the bound is written, and a
normal fixture zip is unaffected.

Decided 2026-10-22: the bound lives in the shared zip-entry check (044, `zip_entry.rs`), so the backup
restore (013) gets it too; the declared sizes are summed before anything is written, the actual bytes
counted while writing; the constant is 2 GiB and `--max-unpacked <size>` on `add`, `rollback` and
`backup --restore` (or the flag name the CLI prefers) overrides it.

## Docs that move

- `docs/development.md` (the shared zip-entry check and the engine's errors): the bound and the flag.

## Not in this change

- A bound on the download size (046, on the GitHub source branch); any other hardening item of M5.

## Steps

1. [ ] The declared bound in `checked_entries` (sum of the entries' uncompressed sizes) and the actual
       bound while unpacking, one error for both naming the limit and the zip
2. [ ] The flag on the CLI commands that unpack; the staging folder left clean after a refusal
3. [ ] Tests: a synthetic zip declaring more than the bound refused before writing; one lying about its
       size refused while writing with the staging folder empty afterwards; a normal fixture unaffected;
       the restore path
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

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
