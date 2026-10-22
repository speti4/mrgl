---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# The Wago source

What: `wago:<id>` resolves through the Wago Addons external API with the user's Bearer key (`MRGL_WAGO_KEY`,
093), the game version per flavor (`retail`, `classic`, `forever`, the progressing classic by its build), the
three recent releases mapped onto the channels, the download fetched with the same key and sent to Wago's
host only, the rate-limit headers feeding the request budget, the answer cached under `cache\wago\`, a 401
naming the key; the Forever signal is `yes` or `no` from a release for `forever`, and the keyless public data
endpoints (the game list, the slugs per game version) answer the signal and the id lookup without a key.
Decided 2026-10-22: the keyless data endpoints are used as they are; the answers are cached like GitHub's.
Why: the one source after GitHub whose API names the Forever flavor. Done when: `add wago:<id>` installs from
the stub for Forever and retail, a 401 stub names the key, the channels are tested over the three stabilities,
the check stops before the stub's rate limit, a list entry imports, and fixtures classify Forever `yes` and `no`.
