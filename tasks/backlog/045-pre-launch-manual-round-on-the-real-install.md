---
status: ready
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# Pre-launch manual round on the real install: the commands no fixture covers

What: one dated round on the real game folders before the Forever launch, under the one-at-a-time rule,
written as this task's Verification without local paths: `add <zip>` and `remove` (004) on the real
`_forever_` folder, once with the game on another drive than the mrgl home (the copy path of the swap);
`backup`, `--list` and `--restore` (013) on a flavor with a real `WTF`, the moved-aside folder checked;
`scan` and `list --json` with the client running (023); `scan` on the era folder without a `.build.info`
row, the exe's version with no warning (019); `add github:… --dry-run` (003) against a real repository
with a Forever release, to confirm the packager's `forever` key as published.
Why: the fixtures prove the code; these paths touch the real folders, the real registry and the real API,
and a failure here is cheaper before launch day (008) than on it.
Done when: every command above has a dated line in Verification, and every failure is a task.
