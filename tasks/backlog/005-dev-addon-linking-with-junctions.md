---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M3 Toolbox
---

# Dev addon linking: junctions for a development folder's addons; CLI `link` and `unlink`

What: `mrgl link <folder> --flavor <id>` creates a directory junction under `Interface\AddOns` for every
subfolder of a development folder that has a TOC (junctions need no admin rights, unlike symlinks), records
them as linked in the sidecar, and `unlink` removes exactly those junctions. It replaces a hand-run link
script.
Why: addons one develops oneself are loaded from their repo, not copied; the manager must know them so an
update never overwrites a junction.
Done when: a fixture development folder links and unlinks in a temporary AddOns folder, a real folder is
never touched by `update`, and the scan lists a junction as linked.
