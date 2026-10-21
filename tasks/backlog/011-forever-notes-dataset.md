---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# Forever notes dataset: dead, crippled or viable, from the research

What: two signals on an addon's row in the window and in `mrgl list`: derived, whether the addon's source
publishes a `forever` build (`release.json`: yes, no or unknown); and curated, a small JSON data file in the
repo for the crippled cases only (a build exists, but the hidden combat values or the missing combat log leave
it half-working), one line per addon with a reason, a `Verified:` date and a source, extended through pull
requests. Decided 2026-10-21: the Forever status is derived first, curated second.
Why: Forever hides combat values and removes the combat log; a user should see why an addon will not work
before installing it.
Done when: the derived signal covers every fixture `release.json`, the curated file carries the research
page's crippled addons with dates, and a stale entry is visible from its date.
