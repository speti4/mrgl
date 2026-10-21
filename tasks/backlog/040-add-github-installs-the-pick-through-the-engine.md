---
status: idea
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# `add github:<owner>/<repo>` installs the pick through the engine

What: `mrgl add github:<owner>/<repo> --flavor <id>` without `--dry-run` downloads the picked asset (003)
to the flavor's staging folder and hands it to the engine's one install call (004) with the pick's source
and version as the metadata; the pick's version (the tag without a leading `v`) is what the sidecar
records, the TOC's `## Version` only a fallback. The "not implemented yet" answer goes away.
Why: the two halves are on `main`; the first M1 command a user types is this one.
Done when: `add github:Questie/Questie --flavor forever` against the stub installs the fixture asset into
a temporary AddOns folder, the sidecar names `github:Questie/Questie` and `1.2.3`, and a second run
reports it as installed already.
