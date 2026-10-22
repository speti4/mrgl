---
status: idea
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The install path picks the named entry among two sidecar entries of one source

What: when two sidecar entries carry the same source, the install path and `update <name>` work on the
entry of the addon that was named, not on whichever entry's name sorts first; `managed_entry` looks up by
source and name together where a name is known.
Why: with a renamed addon next to its old name, `update <name>` can update the other entry today, since
`managed_entry` finds the first entry of the source alone.
Done when: a fixture with two entries from one source, the old name sorting first, updates the named
entry under `update <name>` and leaves the other alone, in the text and the JSON.
