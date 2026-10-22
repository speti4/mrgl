---
status: idea
size: S
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# API keys: one place to set, read and name them

What: a `keys` module in the core with the key names (`MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY`,
`MRGL_CURSEFORGE_KEY`) and the lookup order, the environment first, then the Windows Credential Manager entry
`mrgl:<source>` (a `windows-sys` feature, no new crate); `mrgl key set|clear <source>` reads the value from
stdin, never from an argument; `mrgl key list` and the doctor name each key and where it comes from
(environment, credential store, not set), never its value; a key is sent to its source's hosts only.
Decided 2026-10-22: only the user's own keys, never one shipped in the zip (D-002); the window's key field
comes after 096, not here.
Why: two of the three new sources need a key, and one redaction rule has to cover them all.
Done when: the doctor lists the three names with their origin, a test asserts no key value in any output,
and the credential store path is covered by a fake store in unit tests.
