---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 037 — `list` reads the sidecar: managed (source, version) and unmanaged addons

## Context

`mrgl list --flavor <id>` loads the flavor's sidecar (004) and marks every scanned addon managed (with
its source and installed version) or unmanaged (with the TOC's source hints, as today), in the summary
line and in `--json`; a missing or corrupt sidecar is rebuilt silently, as D-003 says. The `list` JSON
also carries `root_origin` (020), as the `root` document does. 004 writes the sidecar but `list` still
shows every addon as the scan sees it; the daily overview needs the managed state before `update --all`
(006) and the window (009) build on it.
Done when: a fixture flavor with one installed addon lists it as managed with source and version, the
rest unmanaged, and the JSON carries the same fields.

Decided 2026-10-21: `scan` stays the raw folder view and does not read the sidecar; `list` is the view
with the managed state.

## Docs that move

- `README.md`: the `list` usage line and what managed and unmanaged mean.
- `docs/development.md` ("Layout"): `list` reads the sidecar through `state::classify`.

## Not in this change

- The update check (006), the Forever note (011), the dependency report (029), the interface warning
  (015): each adds its own column or field later.

## Steps

1. [ ] `list` loads the sidecar through the engine's state call and classifies the scan (`state::classify`)
2. [ ] The plain line per addon (managed: source and version; unmanaged: the TOC hints) and the summary
       counts through the voice
3. [ ] `--json`: `managed`, `source`, `version` per addon, the counts, and `root_origin` as `root` has it
4. [ ] End-to-end tests on a fixture flavor with one installed addon (through the engine against a
       fixture zip) and the rest unmanaged; a corrupt sidecar is rebuilt without a word to the user
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
