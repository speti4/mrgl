---
status: idea
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The GitHub source looks past the newest release for the flavor's asset and for a named version

What: the resolve takes the newest release that has an asset for the flavor, not only the newest release; and
`import --exact` (057) walks further pages for a named version instead of stopping where 042 stops.
Why: Grid2's newest release is on the classic line with no Forever zip, so it resolves to nothing on Forever
and stays out of the starter list (028); an older listed version beyond page one reads as not offered.
Done when: a fixture repository whose newest release lacks a Forever asset resolves the one before it, and a
named version on page two installs; both against the stub.
