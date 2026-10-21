---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# `mrgl doctor`: a diagnostic report for a bug report

What: `mrgl doctor [--flavor <id>]` prints, plain and `--json`: the game root and how it was found, each
flavor's `.flavor.info` and `.build.info` values, whether `Interface\AddOns` is writable, the sidecar's state
(parsed, rebuilt, missing), the cache size, the WebView2 version, the GitHub rate limit left (one request),
which API keys are present (never their values), and whether the game is running (023). The window has a
"copy diagnostics" action with the same text. Before 012, the first release.
Why: the first bug report arrives with the first release; a report the user can paste is half of support.
Done when: the report runs offline on a fixture game root, every line has a documented meaning, and no key
value or home folder path appears in it.
