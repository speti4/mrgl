---
status: idea
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# GitHub source: pagination past 30 releases, the rate-limit reset time, `release.json` cached per tag

What: three edges of the GitHub source (003): the release list follows the `Link` header past the first
page of 30 when the first page holds no stable release; a 403 or 429 hint names the time the rate limit
resets (`x-ratelimit-reset`) and the token variable; and a release's `release.json` is cached per tag next
to the release list, so a repeated resolve costs one conditional request, not two.
Why: a repository that publishes nightly prereleases pushes the stable release off the first page, and a
user without a token hits the hourly limit on an `update --all` over a long list.
Done when: a fixture repository with 35 releases resolves the stable one on page two, the 403 hint carries
the reset time, and a second resolve of the same tag reads `release.json` from the cache (the stub's log
shows one request).
