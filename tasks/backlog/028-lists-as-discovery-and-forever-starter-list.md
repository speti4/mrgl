---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M3 Toolbox
---

# Lists as discovery: a shareable list format and a Forever starter list in the repo

What: the export format of 015 as a shareable list file (sources and versions, no local paths), and a
"Forever starter list" in the repo that `mrgl import` installs with one command, instead of a search backend.
Why: discovery without a catalog service: a list a friend sends, or the repo's own, is how a new Forever
player gets the usual addons on day one.
Done when: the starter list installs into an empty fixture flavor with `--dry-run`, and a list with a local
path is refused by the format check.
