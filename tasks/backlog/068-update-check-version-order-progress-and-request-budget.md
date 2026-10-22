---
status: idea
size: M
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The update check: version order, progress, and a request budget without a token

What: `update::check` reports "installed is newer than the current release" as its own state instead of an
update (a deleted release, a channel switch); it reports progress per addon through a callback the CLI and
the window (010) share; and it stops with a clear line before a large flavor exhausts the 60 unauthenticated
requests an hour (042's reset time in the line).
Why: today any other version counts as an update, a long check is silent, and a long list without a token hits
the rate limit part way.
Done when: fixture sidecars cover the newer-installed state, the progress lines, and a stubbed rate limit.
