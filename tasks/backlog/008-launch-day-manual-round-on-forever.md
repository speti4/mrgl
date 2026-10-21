---
status: waiting
waits-for: 2026-11-04, the World of Warcraft Forever launch
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Launch-day manual round on the Forever folder

What: once Forever is live, install the Forever addon list from GitHub into the real `_forever_` folder with the
CLI, start the game, confirm every addon loads, run `update --all` once a release lands, and write the result
as the dated Verification of this task. With the client running: `scan` and `list --json` report it (023, a
read-only check that no fixture covers).
Why: M1's done-when is this round; the fixtures prove the code, the game proves the flavor pick.
Done when: the list installs flavor-correctly, loads in-game, and updates; every failure is a task.
