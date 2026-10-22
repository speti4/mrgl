---
status: idea
size: S
origin: done/094-the-wowinterface-source.md
created: 2026-10-22
milestone: M4 Sources
---

# A WoWInterface file waiting for approval is a state of its own in the update check

What: a WoWInterface project whose newest file waits for the site's approval (`UIPending`) gets its own
state in the update check and the window's row (a pending release, nothing to do yet), counted apart from
the failed checks, so `update --all` ends with exit code 0 while the site approves it.
Why: 094 reports a pending file as a failed check, which turns a daily run red for a day or two that is the
site's, not the user's.
Done when: the pending fixture (id 1003) ends an `update --all` with exit code 0 and its own line and JSON
state, the window's row shows it, and a failed check still ends with 1.
