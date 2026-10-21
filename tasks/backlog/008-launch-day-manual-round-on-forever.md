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
read-only check that no fixture covers). Before the launch, as the first half of this round: `add <zip>`
and `remove` (004) on the real `_forever_` folder, once with the game on another drive than the mrgl home
(the copy path of the swap that no fixture covers), and `add github:… --dry-run` (003) against a real
repository with a Forever release, to confirm the packager's `forever` key as published.
Why: M1's done-when is this round; the fixtures prove the code, the game proves the flavor pick.
Done when: the list installs flavor-correctly, loads in-game, and updates; every failure is a task.
