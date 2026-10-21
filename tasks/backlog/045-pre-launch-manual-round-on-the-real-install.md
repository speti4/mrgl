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
`_forever_` folder, once with the game on another drive than the mrgl home; `backup`, `--list` and
`--restore` (013) on a flavor with a real `WTF`; `scan` and `list --json` with the client running (023);
`scan` on the era folder without a `.build.info` row (019); `add github:… --dry-run` (003) against a real
repository with a Forever release; `mrgl doctor` (024); `link` and `unlink` (005) with a real development
folder; and whether the client loads an addon whose required dependency is only a library embedded in its
own folder, the rule 029 implements (a wrong rule is a task).
Why: the fixtures prove the code; these touch the real folders, registry, API and client, before launch day (008).
Done when: every command above has a dated line in Verification, and every failure is a task.
