---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
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

1. [x] `doctor.rs`: the report type with every line (root and origin, flavors with their info values,
       AddOns writable, sidecar state, cache size, WebView2 version, rate limit, keys present, client
       running), behind the discovery, process and HTTP stubs of 020, 023 and 003
2. [x] `mrgl-cli`: `doctor [--flavor <id>]`, plain through the voice (one line per fact) and `--json`
3. [x] Tests on a fixture game root, offline: every line present, the stubbed values shown, a home
       folder path never in the output, a key value never in the output
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `crates/mrgl-core/src/doctor.rs`: `Report` (the general facts, one `FlavorReport` per flavor
  folder, the warnings) with `Report::lines` and `Report::text` as the plain block, the derived `Serialize`
  as the JSON document and `Report::summary` as the closing message (a success, or a warning that names
  what needs a look). `report(Inputs, &Probes)` takes every input explicitly; `report_of_process(root_flag,
  only)` is the one call of the CLI and of the window later: the root as every command resolves it
  (`resolve_root_of_process`; an unreadable config is a warning there, never a failure), the home folder,
  the keys and the token from the environment, the system's process list, `http_from_env` and the runtime
  lookup. A fact that cannot be read is a value (`unknown`, `missing`), so the report always comes out
  whole.
- 2026-10-21 — The checks per flavor: the AddOns probe (an empty `.mrgl-doctor-probe-<pid>-<n>` created and
  removed in `Interface\AddOns`; a missing folder is `missing` and gets no probe), the sidecar
  (`parsed` with its managed count, `rebuilt` for a corrupt file or one of another schema or flavor,
  `missing`, `unreadable`; read through `read_state_at`, never written), the cache (bytes and files under
  `<home>\<flavor-id>\cache\`, links and junctions not followed) and the client (`client_running_among`
  over the probes' process list). A flavor `--flavor` names that has no folder under the root is a block
  of its own, `folder missing`, with what the home folder remembers of it.
- 2026-10-21 — WebView2: `discover::registry_string(Hive, key, value)` exposes the launcher lookup's
  registry reading (open, query, close) for any `HKLM` or `HKCU` string value; the doctor's
  `SystemRuntime` reads the `pv` value of the runtime's EdgeUpdate client key per machine (the
  `WOW6432Node` key, then the native one) before per user, and takes empty, `null` and `0.0.0.0` as no
  runtime. `MRGL_DISCOVERY` stubs it: a `webview2=<version>` item is the answer, any other value is no
  runtime, and the discovery's own parser skips the item.
- 2026-10-21 — The rate limit: one `GET https://api.github.com/rate_limit` through the `Http` trait, with
  the `MRGL_GITHUB_TOKEN` bearer when set; `resources.core` (or `rate`) gives the remaining, the limit and
  the reset (as RFC 3339 through `state::rfc3339_from_unix`); a transport error, a status other than 200
  or an answer without the numbers is `unknown (<reason>)`. The keys are named with `present` only.
- 2026-10-21 — `Redactor`: the profile folder (`USERPROFILE`, else `HOME`) and its 8.3 short form
  (`client::short_form`, a crate-private wrapper over the existing call) become `~`, ASCII
  case-insensitively and only when a separator or the end follows; a variable of fewer than two names
  under its root hides nothing. Every path and every warning of the report goes through it, the client's
  exe path included.
- 2026-10-21 — `mrgl-cli`: the `Doctor { flavor, root }` arm, its `run` arm and `doctor_command`; an
  unknown `--flavor` is the one error (exit code 1). Tests: nine in the core (every line on a synthetic
  root under a made-up profile folder, with a parsed and a corrupt sidecar, a cached zip, a stub client,
  the stub rate limit and runtime and a token, asserting the lines, the JSON, the one request with its
  bearer, a clean AddOns folder, and that neither the profile path nor the token is in the text or the
  JSON; offline with no root; one flavor and a missing one; the redactor; its short form; the rate limit
  answers; the variable; the keys; the byte counts) and four end to end (the plain block line by line, the
  JSON document, offline without a root, a missing and an unknown flavor), each asserting no key value and
  no `\Users\` path in the output. The fixture `releases/api.github.com/rate_limit.json` is the stub's
  answer.
- 2026-10-21 — Docs: `docs/development.md` (the `doctor` module and command in "Layout", a section "The
  doctor" with every line's meaning and its JSON field, the `webview2=` item of `MRGL_DISCOVERY`),
  `README.md` (the usage line and what to paste into a bug report), `docs/nerglish.md` (the report's lines
  are data lines), the bug report template (a `mrgl doctor` field), the fixtures README (the rate limit
  answer).

## Deviations from the plan

- Decided 2026-10-21: the plain output is a block of data lines, one fact per line, and one two-line
  summary through the voice after it, not a voice line per fact: a murloc line before every fact would
  double the block a user pastes, and the data lines of a listing are plain already (docs/nerglish.md).
- `--root <folder>` joins `--flavor`, as `list` has it, so a report can name the game folder for one run.
- The WebView2 lookup is a trait of the doctor (`RuntimeLookup`) over a registry reading the discovery
  now exposes, rather than a method of the discovery's `RootLookup`: the discovery's stub keeps its shape,
  and `MRGL_DISCOVERY` still governs every registry read of a run.
- The report adds the mrgl version, the home folder, the CurseForge marker and the interface number, which
  a bug report needs as much as the listed facts; the Windows version stays in the template's own field.

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core 98, CLI 55 passed and 1 ignored stand-in, the other suites green), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check`: all green on the task branch. No
  manifest and no workflow changed, so `cargo deny check` and `zizmor` did not apply; the bug report
  template parses as YAML with the new `doctor` field.
- 2026-10-21 — One look on the maintainer's machine, the debug exe, with the real environment and a
  `--root` that does not exist, so no game folder was probed: the home line showed the profile folder as
  `~`, the WebView2 line a per-machine version read from the registry, the rate limit line 60 of 60 left
  from one real request, the keys line `MRGL_GITHUB_TOKEN not set`, the missing root a warning, and the
  summary a warning with exit code 0. The game folder lines ran on the synthetic roots of the tests only.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
