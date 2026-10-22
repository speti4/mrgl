---
status: idea
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
milestone: M4 Sources
---

# The Wago update check asks for its addons in bulk, one request per game version

What: the update check gathers the Wago entries per game version and asks `POST /addons/_recents` once
for each, through the `Http::post` of 092, instead of one request per addon; the cache and the per-addon
answer shape stay the same, and a single `add` or `update <name>` still asks for the one addon.
Why: one request per addon against a limit of 60 runs out on a folder of a few dozen Wago addons; the
bulk endpoint makes the check one request per game version.
Done when: a fixture check over several Wago entries makes one stubbed POST per game version, the answers
land in the same cache files, and the budget test of 095 still stops before the limit.
