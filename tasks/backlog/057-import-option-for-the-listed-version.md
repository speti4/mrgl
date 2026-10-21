---
status: idea
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
milestone: M3 Toolbox
---

# An import option that installs the listed version, not the current release

What: `import --exact` (or the name the CLI's flags prefer) installs the version the list file records
for each entry, through the source's release list (the tag of that version), and reports an entry whose
version the source no longer offers; the default stays the current release.
Why: a list exported from a working folder is a known-good set; a second machine wants that set, not
whatever shipped since.
Done when: a fixture list with an older version installs that version from the stub, and a version the
stub lacks is reported, not installed.
