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
the updates with one confirmation. The voice's progress and summary lines (`2 addons updated, 12 up to date`);
the summary counts pinned and ignored addons separately (021).
Why: the daily command of the first weeks after launch.
Done when: a fixture sidecar against fixture releases reports every state, `--dry-run` writes nothing, and
the summary line counts are right.
