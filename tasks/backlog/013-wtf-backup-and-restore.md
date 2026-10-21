---
status: idea
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M3 Toolbox
---

# WTF backup and restore

What: `mrgl backup --flavor <id> --to <folder>` copies the flavor's `WTF` folder into a dated zip, and
`--restore` puts one back after a confirmation; the window gets the same two actions.
Why: the character and addon settings of the first weeks after launch are the most expensive thing to lose.
Done when: a backup round-trips a fixture WTF folder byte for byte and a restore never writes outside `WTF`.
