---
status: idea
size: S
origin: done/058-forever-notes-for-the-partly-restricted-addons.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The Forever notes schema check allows one day of slack on `verified`

What: the schema test of `data/forever-notes.json` (011) accepts a `verified` date up to one day ahead of
today in UTC, and `CONTRIBUTING.md` ("Forever notes") points to the rule for adding a note in
`docs/development.md`.
Why: east of UTC, a note dated by the local day fails as "in the future" between local and UTC midnight.
Done when: a note dated tomorrow in UTC passes, one dated two days ahead fails, and CONTRIBUTING links the rule.
