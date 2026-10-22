---
size: S
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 093 — API keys: one place to set, read and name them

## Context

A `keys` module in the core with the key names (`MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY`,
`MRGL_CURSEFORGE_KEY`) and the lookup order, the environment first, then the Windows Credential Manager entry
`mrgl:<source>` (a `windows-sys` feature, no new crate); `mrgl key set|clear <source>` reads the value from
stdin, never from an argument; `mrgl key list` and the doctor name each key and where it comes from
(environment, credential store, not set), never its value; a key is sent to its source's hosts only.
Decided 2026-10-22: only the user's own keys, never one shipped in the zip (D-002); the window's key field
comes after 096, not here.
Why: two of the three new sources need a key, and one redaction rule has to cover them all.
Done when: the doctor lists the three names with their origin, a test asserts no key value in any output,
and the credential store path is covered by a fake store in unit tests.

Decided 2026-10-22: runs after 092 on the same branch, since both change how the sources are built.

## Docs that move

- `docs/development.md`: the keys module, the doctor's `api keys:` line and JSON `origin`, the credential store
- `README.md`: the `key` command and where a key comes from
- `Cargo.toml`: the one-line reason for the `windows-sys` credential feature

## Not in this change

- The window's key field (after 096)
- A key store outside Windows (M6); the Wago and CurseForge sources that read the keys (095, 096)

## Steps

1. [ ] `keys.rs` in the core: the names, the lookup order, the Windows Credential Manager read, write and
       delete behind a store trait with a fake store for tests; the non-Windows twin reports "not available"
2. [ ] `github.rs`: the token comes from the module; `doctor.rs`: one line per key with its origin, JSON `origin`
3. [ ] `main.rs`: `mrgl key set|clear <source>` (the value from stdin) and `mrgl key list`
4. [ ] Tests: the doctor's three names with their origin, no key value in any text, JSON or error output,
       the fake store round trip

## Log

- 2026-10-22 — promoted; runs after 092 on the same branch

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
