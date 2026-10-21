---
size: S
origin: done/024-doctor-diagnostic-report.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
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

1. [x] The three registry values through the doctor's registry reader, one line in the plain report and
       one object in the JSON
2. [x] Tests on the stub: present, partly present, absent
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 043
- 2026-10-22 — `crates/mrgl-core/src/discover.rs`: `registry_dword(Hive, key, value)` beside
  `registry_string`, one `RegQueryValueExW` into four bytes that counts only a `REG_DWORD`; the key
  opening is one `open` helper both readers share. `MRGL_DISCOVERY`'s parser skips a `windows=` item as
  it skips `webview2=`.
- 2026-10-22 — `crates/mrgl-core/src/doctor.rs`: `WindowsVersion` (`name`, `display_version`, `build`,
  `ubr`, `product_name`, each `None` when the value is not there) with `from_values` (trims, an empty
  value is none, the build is a decimal number) and `text` (`Windows 11 24H2, build 26100.1234`; a missing
  part left out, a missing build `build unknown`, nothing at all `unknown`). The name comes from the
  build: 22000 and later is Windows 11, 10240 and later Windows 10, because `ProductName` says Windows 10
  on Windows 11 too; another product name (a server) stays as written. `RuntimeLookup` gains `windows()`:
  `SystemRuntime` reads `ProductName`, `DisplayVersion`, `CurrentBuildNumber` and `UBR` under
  `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`, `FixedRuntime` returns its `windows` field.
  `Report.windows` is one object in the JSON; the plain block prints `windows: …` right after `mrgl: …`.
- 2026-10-22 — The stub: a `windows=<display version>,<build>.<ubr>` item of `MRGL_DISCOVERY`
  (`windows=24H2,26100.1234`, a part may be left out); any other stub value is an unknown Windows, so no
  end-to-end test reads the real registry.
- 2026-10-22 — Tests: in the core, the present, partly present and absent values (a Windows 11 product
  name that says Windows 10, a Windows 10 build, a server name, a build alone, an empty display version,
  a build that is no number, a revision without a build, nothing), the line and the JSON object in the
  report on the fixture root, `windows: unknown` and an all-`null` object offline, and the stub item; end
  to end, a new test reads the line and the JSON object from the stub item, and the existing doctor tests
  expect `windows: unknown` as the second line.
- 2026-10-22 — Docs: `docs/development.md` ("The doctor": the line, its JSON fields and the name rule; the
  `windows=` stub item there and in the root discovery paragraph), the bug report template (the doctor
  field names the Windows version, the OS field points at the `windows:` line).

## Deviations from the plan

- `ProductName` is read as well: it names a Windows that is neither 10 nor 11 (a server) and stays in the
  JSON as written; the line names Windows 10 or 11 by the build, since the product name of Windows 11
  still says Windows 10.
- The line is the second of the block, after `mrgl:`, in the order the bug report asks for them; the
  existing end-to-end doctor tests move their line numbers by one.
- `FixedRuntime` holds named fields (`webview2`, `windows`) instead of the WebView2 answer alone, and
  `RuntimeLookup` has a second method; both live in the doctor alone, and `report_of_process`, `Report`'s
  existing fields and `discover::registry_string` keep their shape.

## Verification

- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo test
  --locked` (core 142 passed, CLI 83 passed and 1 ignored stand-in, the other suites green): all green on
  the task branch.
- 2026-10-22 — One look on the maintainer's machine, the debug exe with the real registry: `mrgl doctor`
  printed `windows: Windows 11 <display version>, build <build>.<revision>` with the machine's values, while
  the JSON's `product_name` read Windows 10, as the name rule expects; `--json` carried the same values
  as numbers and strings.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
