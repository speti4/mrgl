---
status: idea
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
milestone: M4 Sources
---

# The update check stops per source at a rate limit, not as a whole

What: when one source's request budget runs out, the update check marks only that source's entries as
not checked and goes on asking the other sources; the stop line names the source whose limit it hit.
Why: the stop of the check applies to the whole run today, since GitHub was the one source with a budget;
with the Wago source (095) a second budget joins and one exhausted limit would silence the other source.
Done when: a fixture check over a GitHub entry and a Wago entry with one budget exhausted reports the one
source as not checked and the other as checked, in the text and the JSON, and `update --all` agrees.
