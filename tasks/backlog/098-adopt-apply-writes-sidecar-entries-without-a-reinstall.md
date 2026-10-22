---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# Adopt `--apply`: sidecar entries without a reinstall, and the `.curseclient` guard

What: `adopt --apply` writes one sidecar entry per adoptable addon of the report (097): the source spec, the
TOC `## Version` (`unknown` when absent), the folder group (the folders the source names, else the folders
whose TOCs carry the same source id, else the folder alone) and the file hashes, with no download and no
folder change, so the next `update` does a clean install. A flavor with a `.curseclient` marker refuses
`--apply` without `--alongside-curseforge`, and with it writes and warns that the CurseForge app may still
update those folders; the marker is never removed; `add`, `import` and `update` warn in a marked folder too.
Decided 2026-10-22: the flag is `--alongside-curseforge`; a CurseForge-only addon is skipped before the key.
Why: the one write of adopt, and the road to CurseForge independence. The second half of the former 026.
Done when: `--apply` on the fixture writes a sidecar that `update` accepts against the stubs, the marker
guard holds with and without the flag, and a second `--apply` changes nothing.
