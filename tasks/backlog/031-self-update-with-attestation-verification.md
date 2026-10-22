---
status: waiting
waits-for: the signed exes of 103 (D-006), whose certificate subject the check pins
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M5 Trust
---

# Self-update of the portable zip with signature verification; CLI `self-update`

What: `mrgl self-update` fetches the latest release zip from the repo's releases, verifies the checksum and
the Authenticode signature of both exes through `WinVerifyTrust` against the pinned subject (D-006, 103),
unpacks it next to the running exes and swaps the two exes on the next start (the running exe renamed aside,
cleaned up at the next start); the window offers the same with one click, and `--check` only reports.
Why: a portable zip has no installer to update it, and a manager that cannot update itself is a chore.
Done when: a fixture release updates a copy of the zip and an unsigned exe, a wrong signer or a bad checksum
is refused; the Sigstore attestation stays the `gh attestation verify` check of the README (D-006).
