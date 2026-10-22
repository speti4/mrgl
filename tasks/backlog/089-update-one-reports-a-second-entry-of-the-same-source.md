---
status: idea
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# `update <name>` says what happened when another entry of the same source already has the pick's version

What: when the one install path finds a second sidecar entry from the same source at the pick's version
and answers "installed already", `update <name>` reports that entry and the outcome instead of counting
the addon as updated and printing no line; the JSON carries the same.
Why: two sidecar entries from one source (a renamed addon next to its old name) make a silent, miscounted
update today.
Done when: a fixture with two entries from one source prints the line and counts the addon right, in the
text and the JSON, and `update --all` agrees.
