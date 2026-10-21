---
status: ready
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Flavor detection and the AddOns scan with TOC parsing; CLI `scan` and `list`

What: `flavor` reads a game folder's `.flavor.info` (`wow`, `wow_classic`, `wow_classic_era`, `wowf`) and
`.build.info` (version, product); `scan` lists every folder under `Interface\AddOns` with its TOC data; `toc`
parses the header lines, the multi-TOC suffixes (`_Mainline`, `_Vanilla`, `_TBC`, `_Wrath`, `_Cata`, `_Mists`,
`_Camelot`, `_Forever`), comma-separated `## Interface` lists, `X-Curse-Project-ID` / `X-Wago-ID` / `X-WoWI-ID`,
Title, Version and Dependencies. `mrgl scan <folder>` and `mrgl list --flavor <flavor>` print it, `--json`
too; `--flavor` takes a friendly name (`forever`, `retail`, `classic`, `era`), a `.flavor.info` id or a flavor
folder. The suffix list is checked against Blizzard's (`_Classic` and `_BCC` are missing above).
Why: everything else (sources, install, update) starts from knowing the flavor and what is installed.
Done when: `mrgl scan` over a real retail folder lists every addon with its TOC data, the synthetic fixtures
cover every TOC shape above, and the tests run offline.
