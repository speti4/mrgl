---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M3 Toolbox
---

# 005 — Dev addon linking: junctions for a development folder's addons; CLI `link` and `unlink`

## Context

`mrgl link <folder> --flavor <id>` creates a directory junction under `Interface\AddOns` for every
subfolder of a development folder that has a TOC (junctions need no admin rights, unlike symlinks),
records them as linked in the sidecar, and `unlink` removes exactly those junctions. It replaces a
hand-run link script. Addons one develops oneself are loaded from their repo, not copied; the manager
must know them so an update never overwrites a junction.
Done when: a fixture development folder links and unlinks in a temporary AddOns folder, a real folder is
never touched by `update`, and the scan lists a junction as linked.

Decided 2026-10-21: the junction is made by the tool itself (a reparse point through the Windows API or
a small crate on the license allowlist), never by shelling out to `mklink`; a linked addon is a sidecar
entry with the source `link:<development folder>`, so the install engine refuses to install over it and
`remove` is not the way out, `unlink` is; a name that already exists under `AddOns` as a real folder is
skipped with a warning, never replaced; on a non-Windows target the two commands report that junctions
are Windows-only and the tests that need one are Windows-only.

## Docs that move

- `README.md`: the `link` and `unlink` usage lines.
- `docs/development.md` ("Layout"): the `link` module and the `link:` source in the sidecar.
- `CLAUDE.md` ("Data outside the repo"): no change (a junction target is the user's folder, never in the
  repo).

## Not in this change

- The window's link action (M2); watching the development folder for new addons; symlinks.

## Steps

1. [ ] `link.rs`: find the addon folders of a development folder (a TOC named after the folder), make
       the junctions under `AddOns`, record them in the sidecar as `link:<folder>`; `unlink` removes
       exactly those junctions and their entries
2. [ ] The install engine refuses to install over a linked entry with a clear error; the scan's `linked`
       flag is set for a junction (`scan.rs` already reads the reparse flag; confirm against a real junction)
3. [ ] `mrgl-cli`: `link <folder> --flavor <id>` and `unlink <folder> --flavor <id>` replace the
       `not_yet` stub (the stub test then points at `update`, task 006), the voice lines, `--json`
4. [ ] Tests on a fixture development folder and a temporary AddOns folder: link, scan shows linked,
       install over it refused, unlink leaves the folder empty and the user's folder untouched
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
