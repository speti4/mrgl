---
status: idea
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# `import` runs the dependency check after an install, as `add` and `update` do

What: `mrgl import` reports the missing dependencies of every addon it installed, through the same
`Engine::check_installed` the install summary of `add` and `update` reads (079, 082), in the text and the
JSON next to the interface warnings it already prints.
Why: an imported list is the one path that installs many addons at once, and the one that says nothing
when a required library is not on the list.
Done when: a fixture list that installs an addon with a missing dependency prints the warning and carries
it in the JSON, and a complete list prints none.
