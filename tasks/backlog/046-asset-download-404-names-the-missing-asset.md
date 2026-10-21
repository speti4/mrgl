---
status: idea
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# The asset download's error path: a 404 names the asset, the size is checked

What: when the GitHub source's asset download answers 404, the error names the asset and the release
instead of the release-list hint ("no such repository, or it is private"); and the downloaded asset's byte
count is compared with the `size` the release list names for it, a mismatch refused before the engine
opens the zip, with a line that names both numbers (a release without a size installs as before).
Why: the hint sends the user to check the repository name when the asset was renamed, and a truncated
download today surfaces only as an unreadable zip, which reads like a packaging bug.
Done when: a fixture release whose asset is missing from the stub fails `add` with a line naming the
asset, a fixture asset shorter than its listed size is refused with the documented line, and the
release-list 404 keeps its hint.
