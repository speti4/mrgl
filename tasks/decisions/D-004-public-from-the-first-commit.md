---
status: decided
date: 2026-10-21
cost: cheap
sources: [wip/001-project-scaffolding.md, D-002-rust-tauri2-portable-zip-cf-aware.md]
---

# D-004 — Public from the first commit; the full CI matrix on every push, macOS only from M6

## Context

mrgl is built as a community tool, not a private utility, and a public repository changes two things at
once: every file is public text from the first commit, and the Actions minutes of the standard runners are
free, so a CI budget is no reason to skip a Windows job. The scaffold inherits a CI layout (Windows tests,
Linux lint, the oldest supported Rust, the dependency policy, the workflow lint, the commit rules) that was
designed under a minutes budget.

## Decision

The GitHub repository `speti4/mrgl` is public from its first commit. CI runs the Windows `test` and `msrv`
jobs and the Linux `lint`, `deny` and `workflow-lint` jobs on every push to `main` and every pull request;
nothing is gated on pull requests only. No macOS job until the macOS preview (M6). Everything a public repo
asks of the files holds from the start: a task file's `plan:` is `none`, no file carries a home folder path,
no game data or Blizzard asset enters the repo, and the community files (CONTRIBUTING, SECURITY, the code of
conduct, the issue and pull request templates) ship with the scaffold. A commit message or a pull request
description carries no `Claude-Session:` trailer and no session link (they point at a private session; the
hook and the `commit-rules` job reject both), and the maintainer commits with the GitHub noreply address.
Decided 2026-10-21.

## Not chosen

- Private until a first release, with the Windows `test` job on pull requests only: the minutes concern it
  answered does not exist on a public repo, and a private start would postpone the community files and the
  habit of writing public text.
- Private until the project is "presentable": the task files and decisions are the presentation.
- A macOS job from the start: nothing compiles for macOS before M6, and the macOS runners are the slow and
  scarce ones.
