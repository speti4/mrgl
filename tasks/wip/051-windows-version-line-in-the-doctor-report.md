---
size: S
origin: done/024-doctor-diagnostic-report.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 051 — A Windows version line in the doctor report

## Context

The doctor report (024) gains the Windows build and display version from the registry, read the
same way as the WebView2 version, so the bug report template's OS field can come from the report. The OS
version is the second thing asked in every bug report, and the user guesses it today.
Done when: the line is in the plain and the JSON report, stubbed in the tests, and documented with the
other lines.

Decided 2026-10-22: paired with 043 on one branch (both read the Windows registry through the stubs of
024 and 020); `CurrentBuildNumber`, `UBR` and `DisplayVersion` under
`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` render as one line (`Windows 11 24H2, build 26100.1234`).

## Docs that move

- `docs/development.md` ("The doctor"): the new line; `.github/ISSUE_TEMPLATE` bug report if its OS
  field names the doctor.

## Not in this change

- The window's "copy diagnostics" (009); any other registry source for the game root (043, same branch).

## Steps

1. [ ] The three registry values through the doctor's registry reader, one line in the plain report and
       one object in the JSON
2. [ ] Tests on the stub: present, partly present, absent
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 043

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
