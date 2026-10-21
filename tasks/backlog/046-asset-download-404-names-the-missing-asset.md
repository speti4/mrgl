---
status: idea
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# An asset download that returns 404 names the missing asset

What: when the GitHub source's asset download answers 404, the error names the asset and the release
instead of the release-list hint ("no such repository, or it is private"), which is wrong for an asset
that disappeared after the list was fetched.
Why: the hint sends the user to check the repository name when the release was edited or the asset
renamed; the right next step is a fresh resolve.
Done when: a fixture release whose asset is missing from the stub fails `add` with a line naming the
asset, and the release-list 404 keeps its hint.
