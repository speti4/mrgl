---
status: ready
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Update check: installed state against the latest release per flavor; `update --all` with dry-run and `--yes`

What: compare every managed addon's sidecar entry with its source's latest release for the flavor, report
up to date / update available / source gone / guess only, and `mrgl update --all [--dry-run] [--yes]` apply
the updates with one confirmation, each through the engine's one install call of 004 with the source's
download (003) and the pick's version as the metadata. The voice's progress and summary lines (`2 addons updated, 12 up to date`);
an ignored addon is skipped, a pinned one reported and left alone, each addon resolved on its own channel
(021), and the summary counts pinned and ignored addons separately.
Why: the daily command of the first weeks after launch.
Done when: a fixture sidecar against fixture releases reports every state, `--dry-run` writes nothing, and
the summary line counts are right.
