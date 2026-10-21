---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M3 Toolbox
---

# Addon list export and import, and a TOC-interface versus build warning

What: export a flavor's managed addon list (sources and versions) as a file, import it into another flavor
or machine, and warn when an installed addon's `## Interface` does not match the folder's `.build.info`.
Why: a fresh install or a second machine wants the same list; a mismatched interface is the first sign of a
wrong flavor pick.
Done when: an export imports into an empty fixture flavor with the same result, and the warning fires on a
fixture addon with a stale interface.
