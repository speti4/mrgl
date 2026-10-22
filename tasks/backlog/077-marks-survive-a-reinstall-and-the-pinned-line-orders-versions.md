---
status: idea
size: S
origin: done/065-derive-name-prefers-the-zip-or-repository-folder.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The marks survive a reinstall under a new derived name, and the pinned line uses the version order

What: when a zip in hand is staged under a name the naming rule now derives (065) while the sidecar records
the same folders under the old name, the commit carries the old entry's pin, channel and previous version
over to the new entry instead of dropping them with a warning; and the update check's pinned line applies
the version order of 068, so an installed version newer than the current release is not reported as `<latest>
is out`.
Why: an addon recorded as `DBM-Brawlers` before 065 loses its marks on the next install from a zip, and the
window shows "pinned at 1.1.0-beta, 1.0.0 is out" (010).
Done when: a fixture sidecar with the old name, a pin and a previous version keeps all three after the
reinstall, and a pinned fixture addon newer than the current release reports the pin without `is out`.
