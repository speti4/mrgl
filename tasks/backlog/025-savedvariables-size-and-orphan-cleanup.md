---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M3 Toolbox
---

# SavedVariables: size per addon and orphan cleanup into a backup zip; CLI `savedvars`

What: `mrgl savedvars --flavor <id>` lists the `SavedVariables\<Addon>.lua` sizes per account and per
character, and marks orphans (a file whose addon folder no longer exists under `Interface\AddOns`; a disabled
addon keeps its folder, so it is never an orphan). `--clean` moves the orphans and their `.bak` files into a
dated zip next to the backups of 013, never deletes, and refuses while the game runs (023). The third `WTF`
write of D-002 and D-003 (amended 2026-10-21).
Why: the classic `WTF` bloat: megabytes of settings for addons removed months ago, and a 50 MB file behind a
long loading screen.
Done when: a fixture `WTF` folder lists sizes and orphans correctly, `--clean` round-trips the moved files
through the zip byte for byte, and nothing outside `SavedVariables` is touched.
