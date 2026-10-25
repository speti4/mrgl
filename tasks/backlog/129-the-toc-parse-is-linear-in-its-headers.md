---
status: ready
size: S
origin: wip/128-a-manual-round-on-the-real-install-after-0-1-0.md
created: 2026-10-25
milestone: M1 Core + CLI
---

# The TOC parse is linear in its headers, so a generated TOC of 400,000 headers scans in seconds

What: `Toc::set` finds a header's key with a linear, case-insensitive scan over every key kept so far, so a
TOC is parsed in time quadratic in its headers; a Forever addon of the starter list ships generated TOC files
of 13 to 57 MB with 115,989 to 408,484 distinct `## X-…` headers, and every `scan`, `list`, `adopt` and
install of that folder ran for hours at one core (the zip's unpack took 2 seconds). The fix keeps the headers
under a case-folded key with the first spelling kept, and bounds what a TOC of that size costs in memory and
in `--json` output (the raw headers map is serialized whole today).
Why: the round of 128 found it on the real Forever folder; without the fix the starter list cannot be
installed or listed, and the launch-day round (008) depends on it.
Done when: a test TOC of 400,000 distinct headers parses in well under a second, the real bundle scans in
seconds, and the typed fields and the headers map read as before.
