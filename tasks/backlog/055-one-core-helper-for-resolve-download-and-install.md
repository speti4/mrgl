---
status: idea
size: M
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# One install path: a core helper for resolve, download and install, the link refusal in the commit

What: the resolve, download and engine-install steps that `add github:` (040, `Engine::install_pick`) and
`import` (015, `list_file::import`) each wrote fold into one core function both call, with one staging
folder rule and one already-installed rule, that resolves a managed addon on its own channel (021,
`Target::with_channel`) instead of the stable default; the refusal to install over a linked addon or a
foreign junction (005, `refuse_links`) moves from `Engine::install` into `Engine::commit`, so every path
that stages and commits is covered; and the CLI's install summary line and JSON block become one helper.
Why: the one path that writes the user's folder was written twice and is about to be written a third time
(`update --all`, 006); a caller that forgets the link check overwrites a junction into a development folder.
Done when: one core function with three callers (`add github:`, `import`, `update`), the check once in
`commit`, one summary helper, and the CLI tests of the existing paths unchanged.
