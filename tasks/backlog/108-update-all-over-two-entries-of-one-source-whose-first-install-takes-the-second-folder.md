---
status: idea
size: S
origin: done/100-the-install-path-picks-the-named-entry-among-two-of-one-source.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# `update --all` over two entries of one source whose first install takes the second entry's folder

What: when a release zip carries the folder of a second sidecar entry of the same source, the first
install takes that folder over and drops the second entry; `update --all` then reports the dropped entry
as folded into the first (its name, the folder, the entry it joined) instead of a "stays at <version>" line
about an entry that no longer exists, in the text and the JSON.
Why: after 100 the line for the second entry describes a sidecar entry the first install already removed.
Done when: a fixture with two entries of one source and a zip carrying both folders reports the second
entry as folded into the first under `update --all`, text and JSON, and the sidecar holds one entry after.
