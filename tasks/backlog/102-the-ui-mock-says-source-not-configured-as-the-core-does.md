---
status: idea
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
milestone: M2 Window MVP
changelog: housekeeping
---

# The window's offline sketch says "source not configured" as the core does

What: the mock data of the window's offline sketch (`crates/mrgl-app/ui/mock-data.js`) writes the core's
line for a row whose source this build lacks, `source not configured: <spec> (this build has no <Name>
source)`, and its Node tests assert the same words.
Why: the sketch still says "a source the update check does not handle yet", the line the core wrote
before the router of 092.
Done when: the sketch's row text matches the core's line word for word and a Node test asserts it.
