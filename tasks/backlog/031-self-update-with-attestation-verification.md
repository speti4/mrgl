---
status: idea
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M5 Trust
---

# Self-update of the portable zip with attestation verification; CLI `self-update`

What: `mrgl self-update` fetches the latest release zip from the repo's releases, verifies the checksum and
the attestation (M5), unpacks it next to the running exes and swaps the two exes on the next start; the window
offers the same with one click, and `--check` only reports.
Why: a portable zip has no installer to update it, and a manager that cannot update itself is a chore.
Done when: a fixture release updates a copy of the zip and a bad attestation or checksum is refused.
