---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# Adopt, the report: what source each unmanaged addon of a folder has

What: `mrgl adopt --flavor <id>`, read-only: per unmanaged addon the sources its TOCs name (`X-Wago-ID`,
`X-WoWI-ID`, `X-Curse-Project-ID`, `X-Website` on github.com, then the curated GitHub mapping compiled in
from `data/github-sources.json`, keyed by folder name, checked by a test, kept by pull requests), each source
asked whether it publishes a build for the flavor, and a summary: how many addons can be adopted without
CurseForge, how many only with it, how many have no known source; a `.curseclient` marker is reported;
`--json` as every command. The first half of the former 026, after 094 and 095. Decided 2026-10-22: a
CurseForge-only addon is reported here and never written by 098 before the key exists; a source without its
key reads "needs <KEY NAME>". Why: useful before any write and any key; it measures the roadmap's M4 goal.
Done when: a fixture AddOns folder with every header shape (each id header, a GitHub and a non-GitHub website,
mapping only, no hint, a multi-folder addon, an unsubstituted version) reports the sources offline against the stubs.
