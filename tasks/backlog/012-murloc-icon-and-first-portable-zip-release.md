---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# The murloc icon and the first portable zip release

What: an icon of our own drawing, a murloc face as fan art within D-005 (no game file, no Blizzard logo),
replacing the placeholder glyph in `crates/mrgl-app/icons/` and the title bar; then the first `v0.1.0`
release through `scripts/release.ps1` and the tag workflow. The themes are in the design sketch (017). The
zip carries `lists/forever-starter.json` (028) so its one import command works without the repo, and the
release closes its milestone by step 4 of the release list (033).
Why: a face for the name, and a release is how anyone else runs it.
Done when: the icon shows in both exes, the window and the taskbar, and the release page carries the zip, the
checksum and the attestation.
