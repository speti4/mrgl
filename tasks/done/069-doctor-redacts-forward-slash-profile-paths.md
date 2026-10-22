---
size: S
origin: done/009-tauri-shell-with-the-flavor-list.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 069 — The doctor report redacts a profile path written with forward slashes

## Context

The doctor's redaction of the user's profile folder (024) also matches the path spelled with `/` and in any
case, so "copy diagnostics" and `mrgl doctor` never carry the user name. A value read back from a config or
a web view can use forward slashes, and then the profile path lands in a public bug report.
Done when: the redaction test covers backslash, forward slash and mixed case, all printed as `~`.

Decided 2026-10-22: first of the chain 069 → 070 on one branch.

## Docs that move

- `docs/development.md`: the doctor's redaction rule, where it is stated.

## Not in this change

- Redacting anything other than the profile folder (an account name in a `WTF` path is the backup's and
  the sets' concern, not the doctor's); the window's diagnostics text beyond what the core redacts.

## Steps

1. [x] `doctor.rs`: `Redactor` matches the long and the 8.3 short form of the profile folder with `\` or
       `/` as the separator, in any case, with the same "separator or end follows" rule
2. [x] Tests: backslash, forward slash and mixed case, the short form too, all printed as `~`; a path that
       only starts with the profile's name is untouched
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 069 → 070
- 2026-10-22 — `doctor.rs`: `Redactor::text` compares through `path_prefix(text, form)`, character by
  character, with `\` and `/` as the same separator (`is_separator`) and a case-insensitive compare by
  `char::to_lowercase`, so a name beyond ASCII (`Ébredő` and `ÉBREDŐ`) matches too; the "separator or end
  follows" rule is unchanged and takes either separator. The forms (the long and the 8.3 short form, longest
  first) are deduplicated by the same compare, so a profile variable written with `/` hides the `\`
  spelling too. Nothing outside `Redactor` changed.
- 2026-10-22 — tests: `the_redactor_hides_the_profile_folder_with_either_separator_in_any_case` (forward
  slashes, upper and mixed case, mixed separators in one path, two matches in one line, a profile variable
  written with `/`, a name beyond ASCII; `someone2` and `someone.old` untouched);
  `the_redactor_hides_the_short_form_too` also checks both forms with `/` and in upper case;
  `tests/cli.rs` `doctor_hides_a_home_path_written_with_forward_slashes` runs `mrgl doctor` with `--root`
  and `MRGL_HOME` written with `/`, and `assert_no_home_path` now reads the output with `/` as `\`.
- 2026-10-22 — `docs/development.md` ("The doctor"): the rule names `HOME` as the fallback, the match in any
  case with either separator, and the "separator or end follows" rule with its example.

## Deviations from the plan

- The case-insensitive compare covers letters beyond ASCII, not only ASCII as before: Windows compares
  folder names that way, and a profile name with an accent is common.

## Verification

- 2026-10-22 — the new tests run against the old `Redactor` first: both unit tests failed
  (the made-up profile path written with `/` stayed as it was), and the CLI test failed with the `root:` and
  `home:` lines carrying the profile folder in full, written with `/`. With the
  change, all three redactor tests and the six doctor tests of `tests/cli.rs` pass; a run of the debug
  `mrgl --serious doctor --root <temp>/root` with the temporary folder written with `/` printed
  `root: ~/AppData/Local/Temp/mrgl-069-check/root (from --root)` and `home: ~/AppData/Local/Temp/mrgl-069-check/home`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core unit 174, `tests/cli.rs` 115, every other suite green) and `cargo +1.90 check
  --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the regeneration.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
