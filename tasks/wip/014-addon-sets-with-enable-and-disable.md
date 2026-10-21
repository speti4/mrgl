---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 014 — Addon sets with enable and disable

## Context

Named sets of addons per flavor (raid, levelling, minimal), stored under the flavor's sidecar folder
(`sets.json`), switched with one command or click by rewriting every character's `AddOns.txt` under `WTF` in
the client's own format, never by deleting folders; refused while the game runs (023). Decided 2026-10-21: a
set applies to every character of the flavor; one chosen character is task 027. Switching a dozen addons by
hand before a raid is the second most common chore.
Done when: a set switch changes exactly the listed addons in a fixture flavor, the game's own files are left
in the format the client wrote, and a switch is refused while the client runs.

Decided 2026-10-22: first of the chain 014 → 027 on one branch; the CLI surface is `mrgl set` with
`save <name>` (from the current `AddOns.txt` state), `apply <name>`, `list` and `remove <name>`, or the
verbs the CLI's existing commands make natural; the window's click is 010's or a later task's.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md` ("Layout", the home folder's files): the
  `set` command, `sets.json`, and the `AddOns.txt` rule (the second `WTF` write of D-002 and D-003).

## Not in this change

- One chosen character (027, next in the chain); the window's set switch; the SavedVariables cleanup (025).

## Steps

1. [ ] The `AddOns.txt` reader and writer in the core (every account and character under `WTF`), round-
       tripping the client's format byte for byte when nothing changes; the `WTF` write guard of 023 before
       every write
2. [ ] `sets.json` under the flavor's sidecar folder: save, apply, list, remove; an apply changes exactly
       the listed addons and leaves the rest as they are
3. [ ] The CLI command with the voice's lines and `--json`
4. [ ] Tests on a fixture `WTF` with two characters: the round-trip, an apply, the refusal while the
       stand-in client runs
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 014 → 027

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
