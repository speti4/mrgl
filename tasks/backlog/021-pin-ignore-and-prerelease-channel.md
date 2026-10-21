---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Pin, ignore and the pre-release channel per addon; CLI `pin`, `unpin`, `ignore` and `unignore`

What: the sidecar entry of an addon gains `pinned` (a version, or none), `ignored` (a flag) and `channel`
(`stable` or `prerelease`, the GitHub `prerelease` flag). `update --all` reports a pinned addon (`pinned at
v1.2, v1.3 available`), counts an ignored one (`3 skipped`) and touches neither; a prerelease channel lets one
addon follow beta builds. CLI `pin <name> [version]`, `unpin`, `ignore` and `unignore`; the window's row
actions belong to task 010.
Why: an update that breaks an addon is the daily case of the first weeks; every other manager has pin and
ignore, and `update --all` is not usable without them.
Done when: a fixture sidecar with a pinned, an ignored and a prerelease addon produces the documented report
and summary counts, and `--dry-run` writes nothing.
