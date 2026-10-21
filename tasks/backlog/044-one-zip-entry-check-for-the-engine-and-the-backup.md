---
status: idea
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
milestone: M3 Toolbox
---

# One zip-entry check for the install engine and the backup restore

What: the install engine's path check (004, `safe_relative_path`) and the restore's (013, `safe_entry_path`)
fold into one function with one `Error` pair (an unreadable zip, an escaping entry) that both call; the
two fixture tests stay and run against the shared check.
Why: two copies of the one rule that keeps a zip from writing outside its folder drift apart, and the
hardening review of M5 wants one place to read.
Done when: one function, both callers, both escape tests green, and the duplicated error variants gone.
