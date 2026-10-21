---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Rollback to the previous version from the kept zip; CLI `rollback`

What: `mrgl rollback <name>` installs the previous version's zip the install engine (004) kept in the cache
(`%LOCALAPPDATA%\mrgl\<flavor-id>\cache\<addon>\<version>.zip`, one previous version per addon) with the same
atomic swap, and pins the addon at that version (021). Decided 2026-10-21: a rollback pins, so the next
`update --all` does not reinstall the broken build; `unpin` is the deliberate step back. The swap's own revert
on a failed install in 004 is not this command.
Why: with daily broken builds after launch, "put yesterday's back" is one command instead of a hunt for a zip.
Done when: an install, an update and a rollback in a temporary AddOns folder leave the first version's files
byte for byte, the sidecar shows the pin, and a rollback with no kept zip is a clear error.
