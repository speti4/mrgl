---
status: idea
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
milestone: M5 Trust
---

# The asset download is checked against the release's size

What: the downloaded asset's byte count is compared with the `size` the release list names for it, and
a mismatch is refused before the engine opens the zip, with a line that names both numbers.
Why: a truncated download today surfaces only as an unreadable zip, which reads like a packaging bug;
the size check names the real cause, and the hardening review of M5 wants the download bounded.
Done when: a fixture asset shorter than its listed size is refused with the documented line, and a
release without a size installs as before.
