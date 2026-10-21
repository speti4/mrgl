---
status: idea
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
milestone: M3 Toolbox
---

# `link` takes a single addon folder as well as a development folder

What: `mrgl link <folder>` given a folder that is itself an addon (a TOC named after it) links that one
folder instead of answering "no addon in the development folder"; `unlink` matches.
Why: a developer with one addon keeps its repository as the addon folder, not under a parent.
Done when: a fixture addon folder links and unlinks on its own, and a development folder still links
its subfolders.
