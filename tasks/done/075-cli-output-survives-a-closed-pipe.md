---
size: S
origin: done/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 075 — The CLI's output survives a closed pipe

## Context

Every command's stdout and stderr writes go through one helper that treats a closed pipe (`BrokenPipe`,
os error 232 on Windows) as the end of the run: the command exits quietly with 0 instead of panicking.
`mrgl list --json | head -c 200` on the real install panicked with `failed printing to stdout`, which
breaks every shell pipeline that reads only the start of the output.
Done when: a test pipes `list --json` into a reader that closes early and sees exit 0 with no panic text, and
the plain and JSON outputs are unchanged otherwise.

Decided 2026-10-22: second of the chain 077 → 075 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the command line's output rule for a closed pipe, next to the voice paragraph.

## Not in this change

- The window (no pipe); a change to any message text.

## Steps

1. [x] `mrgl-cli/src/main.rs`: the `Reporter` methods and the remaining raw `println!`/`eprintln!` sites write
       through one helper that maps a closed pipe to a quiet end of the run (exit 0)
2. [x] Test: `list --json` piped into a reader that closes early exits 0 with no panic text; the plain and
       JSON outputs of the existing tests unchanged
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 077 → 075
- 2026-10-22 — the failure before the change: the new test's `mrgl --json list` into a reader that closes
  after 200 bytes ended with exit code 101 and `failed printing to stdout: … (os error 232)` on stderr.
- 2026-10-22 — `mrgl-cli/src/main.rs`: `emit(Stream, fmt::Arguments)` writes and flushes through the locked
  stdout or stderr; `is_closed_pipe` takes `ErrorKind::BrokenPipe` and, on Windows, the raw os error 232
  (`ERROR_NO_DATA`); a closed pipe ends the process with `std::process::exit(0)`, any other write error
  panics with the message `println!` gives. `Reporter::say`, `fail`, `line` and `document` write through it,
  and a new `Reporter::say_on_stderr` carries the two-line messages that go to stderr whatever the mode
  (`fail` in human mode, the warnings and the summary of `export` without a file); `--version --json` and
  `export`'s list on stdout write through `emit` too. No `println!`, `eprintln!` or `print!` is left in the
  crate, and no message text changed.
- 2026-10-22 — test: `tests/cli.rs`
  `list_json_into_a_reader_that_closes_early_ends_quietly_with_exit_0` (600 folders made by hand, so the
  document outgrows the pipe's buffer; the reader takes 200 bytes and is dropped; exit 0, stderr empty).
- 2026-10-22 — docs: `docs/development.md` states the output rule after the voice section; the CLI's
  module doc names the exit code of a closed pipe.

## Deviations from the plan

- `Reporter` gains `say_on_stderr` for the two-line messages on stderr, so the raw sites of `export` write
  through the same helper as `fail` instead of their own `eprintln!`.

## Verification

- 2026-10-22 — with 600 folders made by hand in a temporary game (`list --json` is 503104 bytes): in Git
  Bash, `mrgl list --json --flavor <folder> | head -c 200` printed the start of the document and the exit
  code of `mrgl` was 0 with nothing on stderr; `mrgl list … | head -2` and `mrgl scan … | head -1` printed
  their lines and exited 0 as well. Before the change the new test saw exit 101 and the panic text.
- 2026-10-22 — the new test passed eight runs in a row. `cargo fmt --check`, `cargo clippy --all-targets
  --locked -- -D warnings`, `cargo test --locked` (`tests/cli.rs` 120, every other suite unchanged and
  green), `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green;
  the changelog regenerated with the close. The `cfg(not(windows))` side of `is_closed_pipe` is compiled
  by CI's `non-windows` job (no cross C toolchain here).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
