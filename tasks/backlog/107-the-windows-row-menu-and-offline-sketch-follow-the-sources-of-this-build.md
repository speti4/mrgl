---
status: idea
size: S
origin: done/099-the-windows-update-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
milestone: M2 Window MVP
changelog: housekeeping
---

# The window's row menu and the offline sketch follow the sources of this build

What: the row menu's "why no update" line (`ui/actions.js`) says the core's words for a source this build
lacks or whose key is not set (`source not configured: …`, `needs <KEY NAME>`) instead of "the update check
does not ask this source yet", and the offline sketch (`ui/mock-data.js`) stops assuming a GitHub-only
build: its WoWInterface and Wago rows carry the answers the sources of 094 and 095 give; the Node tests
assert both.
Why: 092 to 095 made every source askable, and the sketch and the menu still describe the GitHub-only build.
Done when: the menu line and the sketch rows match the core's words for a WoWInterface, a Wago and a
not-configured row, asserted by the Node tests.
