---
size: S
origin: done/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 076 — The dependency and interface checks say what the client says: an embedded-only dependency is missing, a lower interface number is out of date

## Context

A required dependency that exists only as a library embedded in the addon's own folder is reported as
missing, with the reason that the client does not count an embedded folder (029's rule changes; `nolib` is
unaffected); and `scan` and `list` mark a TOC whose highest interface number of the flavor's expansion is
below the installed build as `out of date`, in the client's words, apart from the other-expansion mismatch.
On the real client a probe addon with an embedded-only required dependency showed `Dependency missing`
and did not load (the reopen condition of 052), and after a client update every addon with the previous
interface number showed `Out of date` while `scan` said nothing.
Done when: the dependency fixture with an embedded-only required library reports it missing with the reason,
the embedded case of an optional dependency is unchanged, and a fixture TOC one interface number behind the
build is marked `out of date` while a current one and an other-expansion one keep their states.

Decided 2026-10-22: first of the chain 076 → 080 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the dependency check's *embedded* rule and the interface check's `out of date` state.
- `README.md`: the dependency paragraph, if it states when a dependency counts as there.

## Not in this change

- The optional dependencies (never reported); the window's row text beyond what the core reports; the
  shared `Blizzard_` constant (080).

## Steps

1. [ ] `dependencies.rs`: an embedded-only required dependency is *missing* with the reason the client does
       not count an embedded folder; the `embedded` list stays informative; `nolib` unaffected
2. [ ] `interface.rs`, `scan` and `list`: a TOC whose highest interface number of the flavor's expansion is
       below the installed build is `out of date`, next to the other-expansion `interface mismatch`
3. [ ] Tests over the fixtures: the embedded-only case, the optional embedded case unchanged, a TOC one
       number behind, a current one and an other-expansion one; CLI text and JSON; the window's rows
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 076 → 080

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
