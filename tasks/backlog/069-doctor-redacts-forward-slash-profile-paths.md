---
status: idea
size: S
origin: done/009-tauri-shell-with-the-flavor-list.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The doctor report redacts a profile path written with forward slashes

What: the doctor's redaction of the user's profile folder (024) also matches the path spelled with `/` and in
any case, so "copy diagnostics" and `mrgl doctor` never carry the user name.
Why: a value read back from a config or a web view can use forward slashes, and then the profile path lands in
a public bug report.
Done when: the redaction test covers backslash, forward slash and mixed case, all printed as `~`.
