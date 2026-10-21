---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Dependency check from the TOC `## Dependencies` and `## RequiredDeps` and the `release.json` `nolib` flag

What: read `## Dependencies` and `## RequiredDeps` from each installed addon's TOC and the `nolib` flag of the
source's `release.json`, and report a missing required dependency after an install and in `list`; an
embedded library (a `nolib` build not chosen) is not missing.
Why: a missing library is the second most common "it does not load" after a wrong flavor pick.
Done when: fixture addons with met, missing and embedded dependencies report as documented, offline.
