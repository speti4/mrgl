---
status: idea
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# One core helper for resolve, download and install

What: the resolve, download and engine-install steps that `add github:` (040, `Engine::install_pick`) and
`import` (015, `list_file::import`) each wrote fold into one core function both call, with one staging
folder rule and one already-installed rule; `update --all` (006) calls the same.
Why: two copies of the one path that writes the user's folder drift apart; the third caller is weeks away.
Done when: one function, three callers (`add github:`, `import`, `update`), the CLI tests of both existing
paths unchanged.
