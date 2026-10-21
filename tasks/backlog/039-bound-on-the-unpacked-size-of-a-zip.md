---
status: idea
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
milestone: M5 Trust
---

# A bound on the unpacked size of a zip

What: the install engine (004) stops unpacking when a zip's declared or actual unpacked size passes a
bound (a generous constant, overridable by a flag for a known large addon), reports it as the documented
error and leaves the staging folder clean.
Why: a zip bomb from a compromised source would fill the disk from the staging folder; the hardening
review of M5 asks for a bound on every untrusted input.
Done when: a synthetic zip that declares more than the bound is refused before the bound is written, and a
normal fixture zip is unaffected.
