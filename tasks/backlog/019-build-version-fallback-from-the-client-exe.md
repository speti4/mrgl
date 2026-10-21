---
status: idea
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# Build version fallback from the client exe when `.build.info` has no row for the flavor

What: when the install root's `.build.info` has no row for a flavor's product, read the version from the
client exe's version resource (`Wow.exe` or `WowClassic.exe` in the flavor folder, the `FileVersion` of its
`VERSIONINFO`), so `Flavor.version` and the interface number are known; `.build.info` stays the first source.
Why: on a real install the Classic Era row was missing from `.build.info` while the exe carried `1.15.7`;
the TOC chain of Era does not need the version, but the interface-versus-build warning of task 015 does.
Done when: a flavor folder with a `.build.info` row missing reports the exe's version with no warning, and
a fixture exe stub (a minimal PE with a version resource, synthetic) covers the parser offline.
