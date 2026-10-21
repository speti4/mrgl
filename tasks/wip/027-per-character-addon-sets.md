---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 027 — Per-character addon sets

## Context

A set (014) applied to one chosen character's `AddOns.txt` instead of every character of the flavor,
with the character picked by name from the `WTF` account folders; refused while the game runs (023).
014 applies a set to every character of the flavor (decided 2026-10-21); a raid character and a
levelling alt on the same flavor want different sets.
Done when: a fixture flavor with two characters switches one and leaves the other's file byte for byte.

Decided 2026-10-22: second of the chain 014 → 027; a `--character <name>` flag on `set apply` (a name
that matches two realms is refused with the realms listed, `--realm` narrows it).

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the flag and the character rule.

## Not in this change

- A per-character set definition (a set is per flavor, the target is per character); the window.

## Steps

1. [ ] The character list from the `WTF` account folders (account, realm, character) in the core
2. [ ] `set apply <name> --character <name> [--realm <realm>]` writes one `AddOns.txt`; an ambiguous name
       is refused with the realms listed; the `WTF` write guard holds
3. [ ] Tests on the fixture with two characters: one switched, the other byte for byte; the ambiguity
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
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
