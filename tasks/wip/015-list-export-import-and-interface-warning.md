---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M3 Toolbox
---

# 015 — Addon list export and import, and a TOC-interface versus build warning

## Context

Export a flavor's managed addon list (sources and versions) as a file, import it into another flavor or
machine, and warn when an installed addon's `## Interface` does not match the folder's `.build.info`. A
fresh install or a second machine wants the same list; a mismatched interface is the first sign of a
wrong flavor pick.
Done when: an export imports into an empty fixture flavor with the same result, and the warning fires on
a fixture addon with a stale interface.

Decided 2026-10-21: the list file is JSON (`mrgl-list` schema 1: the flavor it came from, one entry per
managed addon with source and version); `export` writes it or prints it to stdout, `import` resolves and
installs every `github:` entry through the source and the engine (a zip-in-hand entry is reported as
not importable, a linked one skipped), with `--dry-run` listing what would happen; the interface warning
compares the primary TOC's `## Interface` with the flavor's build (the exact major of 002's build
reading; several interface numbers in a TOC pass when one matches) and is a warning in `list` and after
an install, never a refusal.

## Docs that move

- `README.md`: the `export` and `import` usage lines.
- `docs/development.md` ("Layout"): the list file format and the interface check.

## Not in this change

- Addon sets (014), the shareable list format for discovery (028, which may reuse this file), the
  window's import button.

## Steps

1. [ ] The list file: a type with its schema, written from the sidecar's managed entries, read back
2. [ ] `mrgl-cli`: `export --flavor <id> [--to <file>]` and `import <file> --flavor <id> [--dry-run]`
       (the installs through the source and the engine), the voice lines, `--json`
3. [ ] The interface warning: the TOC's interface against the flavor's build, in `list` and after an
       install, with a documented line
4. [ ] Tests: export from a fixture sidecar, import into an empty fixture flavor against the folder
       stub, the same sidecar after; the warning on a fixture addon with a stale interface
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
