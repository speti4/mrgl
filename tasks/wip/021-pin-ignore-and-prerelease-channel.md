---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 021 — Pin, ignore and the pre-release channel per addon; CLI `pin`, `unpin`, `ignore` and `unignore`

## Context

The sidecar entry of an addon gains `pinned` (a version, or none), `ignored` (a flag) and `channel`
(`stable` or `prerelease`, the GitHub `prerelease` flag). `update --all` reports a pinned addon (`pinned
at v1.2, v1.3 available`), counts an ignored one (`3 skipped`) and touches neither; a prerelease channel
lets one addon follow beta builds. CLI `pin <name> [version]`, `unpin`, `ignore` and `unignore`; the
window's row actions belong to task 010. An update that breaks an addon is the daily case of the first
weeks; every other manager has pin and ignore, and `update --all` is not usable without them.
Done when: a fixture sidecar with a pinned, an ignored and a prerelease addon produces the documented
report and summary counts, and `--dry-run` writes nothing.

Decided 2026-10-21: the three fields are optional in the sidecar schema (absent means unpinned, not
ignored, stable), so an existing `state.json` reads unchanged; the GitHub source honors the channel when
it picks the current release (a prerelease only on the `prerelease` channel, as before otherwise); the
`update --all` report itself is 006, this task ships the fields, the four commands, `list`'s marks and
the core query the update check calls (`pinned`, `ignored`, `channel` per entry).

## Docs that move

- `README.md`: the four usage lines.
- `docs/development.md` ("Layout"): the sidecar fields and the channel in the source's pick.
- `tasks/decisions/D-003-state-sidecar-under-localappdata.md`: none (the sidecar's schema version does
  not change; the fields are optional).

## Not in this change

- `update --all` and its report (006); the rollback that pins (022); the window's row actions (010).

## Steps

1. [ ] `state.rs`: `pinned`, `ignored`, `channel` on the entry, optional in the JSON, with a round-trip test
2. [ ] The GitHub source picks a prerelease when the target's channel says so (the current-release rule
       takes the channel)
3. [ ] `mrgl-cli`: `pin <name> [version]` (no version pins the installed one), `unpin`, `ignore`,
       `unignore`, each on a managed addon of the flavor, the voice lines, `--json`; `list` marks a
       pinned, an ignored and a prerelease addon
4. [ ] Tests: a fixture sidecar with the three cases, the four commands end to end, an unmanaged name
       refused with a clear error
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
