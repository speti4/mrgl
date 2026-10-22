---
size: M
origin: session 2026-10-21
created: 2026-10-21
closed: 2026-10-22
reason: split into 097 (the adopt report) and 098 (adopt --apply) after the M4 planning round of 2026-10-22
reopen-when: never
milestone: M4 Sources
---

# Adopt: take over an existing AddOns folder from its TOC source headers; CLI `adopt`

What: `mrgl adopt --flavor <id>` reports, per unmanaged addon, the source its TOC names (`X-Wago-ID` to the
Wago API, `X-WoWI-ID` to the WoWInterface API, `X-Curse-Project-ID` to CurseForge with the user's key,
`X-Website` on github.com and a curated mapping file in the repo for GitHub) and whether that source publishes
a build for the flavor; the summary counts how many addons run without CurseForge. `--apply` writes sidecar
entries from the TOC `## Version` and the file hashes without reinstalling; the next update does a clean
install. A `.curseclient` marker is reported and never removed (a flavor belongs to one manager), and `--apply`
on a marked folder needs an explicit confirmation flag; the usage guide says so. The closing task of M4.
Why: the retail and classic folders are CurseForge-managed today; adopt is the road to CurseForge independence.
Done when: a fixture AddOns folder with every header shape reports the documented sources offline (the APIs
stubbed), `--apply` produces a sidecar that `update` accepts, and the marked-folder guard holds.
