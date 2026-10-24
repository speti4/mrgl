---
size: M
origin: session 2026-10-24
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 121 — A hardening review of the code that touches the disk, the network and the keys

## Context

A reading review of the non-test code with one question per area: can a crafted zip, a crafted TOC or
`release.json`, a crafted source answer, a path from the command line or the window, or a race on the game
folder make mrgl write outside `Interface\AddOns` or the asked `WTF` targets, leak a key, or follow a redirect
with it; the findings as a list, each with the file, the case and a proposed fix, severity-ranked.
Why: the M5 Trust roadmap line; the install engine, the sources and the keys grew over M1 to M4 and were never
read as one.
Done when: the list is in the task file, every finding is a backlog item with its own id or a line that says
why it needs none, and a fix of a high-severity finding is its own task, not this one.

Decided 2026-10-24: filed and started in one step from the roadmap's M5 line; the review reads and reports, it
changes no code (a one-line fix with its test may ride along only when the finding is otherwise trivially
lost; the Log says so).

## Docs that move

- `docs/development.md`: a "Hardening review" paragraph under the trust or security material naming the
  date, the areas read and where the findings went, if the docs keep such a record; else none, said here

## Not in this change

- The fixes (each a task of its own, filed by the main session from the list)
- The test files, the CI workflows and the scripts (the review reads the product code)
- A dependency audit (cargo-deny and the weekly advisories job do it)

## Steps

1. [ ] Read, in this order, with the question per area written down first: the zip extraction and the
       staging move (`zip_entry.rs`, `staging.rs`, `install.rs`, `link.rs`), the TOC and `release.json`
       parsing (`toc.rs`, `sources/`), the sources' HTTP and redirect handling and the key routing
       (`sources/*.rs`, `keys.rs`), the `WTF` writers (`wtf.rs`, `backup.rs`, `savedvars.rs`, `sets.rs`),
       the path inputs of the CLI and the window (`mrgl-cli/src/main.rs`, `mrgl-app/src/actions.rs`,
       `commands.rs`), the home and state files (`home.rs`, `state.rs`, `last_check.rs`)
2. [ ] For each finding: file and line, the crafted input or race, what happens today, the proposed fix,
       severity (high: a write outside the allowed targets or a key leak; medium: a wrong but contained
       write or a crash on crafted input; low: a sharper check or message)
3. [ ] The list in this file under Verification, severity-ranked, with the areas read and found clean named
       too; the follow-ups in the pull request under `## Follow-ups`, one line per finding

## Log

- 2026-10-24 — filed and promoted

## Deviations from the plan

- none yet

## Verification

- <dated: the areas read, the findings list, the clean areas>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
