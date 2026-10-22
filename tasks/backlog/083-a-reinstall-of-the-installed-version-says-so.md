---
status: idea
size: S
origin: done/077-marks-survive-a-reinstall-and-the-pinned-line-orders-versions.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# A reinstall of the installed version says it was reinstalled, not updated from the previous version

What: when a zip in hand or a pick installs the version the sidecar already records, the install summary
says `reinstalled <name> <version>` instead of `updated <name> from <previous> to <same version>`, in the
text and as an `outcome` in the JSON; the entry's previous version stays what it was (077).
Why: `Installed::previous_version` carries the entry's older version, so a reinstall reads as an update
between two different versions that are the same.
Done when: a fixture reinstall of the installed version prints the reinstalled line and keeps the previous
version for `rollback`, and a real update still prints `updated … from … to …`.
