---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 024 — `mrgl doctor`: a diagnostic report for a bug report

## Context

`mrgl doctor [--flavor <id>]` prints, plain and `--json`: the game root and how it was found, each
flavor's `.flavor.info` and `.build.info` values, whether `Interface\AddOns` is writable, the sidecar's
state (parsed, rebuilt, missing), the cache size, the WebView2 version, the GitHub rate limit left (one
request), which API keys are present (never their values), and whether the game is running (023). The
window has a "copy diagnostics" action with the same text. Before 012, the first release. The first bug
report arrives with the first release; a report the user can paste is half of support.
Done when: the report runs offline on a fixture game root, every line has a documented meaning, and no
key value or home folder path appears in it.

Decided 2026-10-21: the report is a core document (`doctor.rs`) the CLI prints and the window will copy
once the shell binds the core (009); the WebView2 version comes from the registry through the same
reading as the game root discovery (020), the rate limit through the source's `Http` trait (offline or
stubbed: `unknown`, never an error); a path under the user's profile is printed with the profile folder
replaced by `~`, so the report carries no home folder path.

## Docs that move

- `README.md`: the `doctor` usage line and what to paste into a bug report.
- `docs/development.md` ("Layout"): the `doctor` module and every line's meaning.
- `.github/ISSUE_TEMPLATE/` bug report: asks for the `mrgl doctor` output, if the template exists.

## Not in this change

- The window's "copy diagnostics" button (lands with the shell, 009); a self-check that fixes anything;
  log files.

## Steps

1. [ ] `doctor.rs`: the report type with every line (root and origin, flavors with their info values,
       AddOns writable, sidecar state, cache size, WebView2 version, rate limit, keys present, client
       running), behind the discovery, process and HTTP stubs of 020, 023 and 003
2. [ ] `mrgl-cli`: `doctor [--flavor <id>]`, plain through the voice (one line per fact) and `--json`
3. [ ] Tests on a fixture game root, offline: every line present, the stubbed values shown, a home
       folder path never in the output, a key value never in the output
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

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
