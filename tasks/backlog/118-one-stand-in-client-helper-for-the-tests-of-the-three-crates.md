---
status: idea
size: S
origin: wip/117-test-hardening-toward-full-line-and-branch-coverage.md
created: 2026-10-23
milestone: M5 Trust
---

# One stand-in client helper for the tests of the three crates

What: the stand-in game client the tests spawn (`ping.exe` copied into a temp flavor folder as the client's
executable, with its drop guard) is written once, in a place the tests of the core, the command line and the
window share, instead of the copy each test file carries today (seven files); the same place takes the locked
file helpers of the core's tests.
Why: 117 added cases to every copy; a change to the stand-in (a new flag, a new skip rule) is seven edits.
Done when: one definition, every test file uses it, and `cargo test --locked` is green on every crate.
