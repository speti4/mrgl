---
status: idea
size: S
origin: done/079-window-and-exe-polish-after-the-first-release.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# An install and an update report the whole interface check: out of date next to the mismatch, and `update` says it at all

What: `add <file.zip>`, `import` and `update` warn after an install when the addon's TOC is behind the
build (`out of date`, 076) as they warn about a mismatch, in the text and in the JSON (`interface_out_of_date`
next to `interface_mismatch`); and `mrgl update` prints the interface warning it computes in
`install_summary` and carries it in its JSON document, which it drops today.
Why: after a client update every updated addon with the previous interface number shows `Out of date` in the
client while the install summary says nothing, and `update` says nothing about a mismatch either.
Done when: the CLI tests of `add`, `import` and `update` over the `stale-interface` and a behind-the-build
fixture zip see both warnings in the text and the JSON.
