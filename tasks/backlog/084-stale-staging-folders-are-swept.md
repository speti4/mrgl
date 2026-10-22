---
status: idea
size: S
origin: done/075-cli-output-survives-a-closed-pipe.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# Stale staging folders are swept, so a run that ends mid-install leaves nothing behind

What: the install engine removes the download and staging folders of earlier runs under `<flavor-id>\staging\`
before it stages (a folder no running mrgl holds, by age or by a lock file), and the command line lets the
addon in flight finish before it exits on a closed pipe (075) so the sweep has less to do.
Why: a closed pipe at an install's `downloading` or `installing` line ends the process before the
`RemoveOnDrop` guard runs, and the download folder stays under `staging\` forever.
Done when: a planted stale staging folder is gone after the next install of the fixture flavor, the folder
of the run itself is untouched while it runs, and the closed-pipe test of 075 leaves no staging folder.
