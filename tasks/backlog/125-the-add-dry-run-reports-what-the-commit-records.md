---
status: idea
size: S
origin: done/124-the-low-findings-of-the-hardening-review-in-one-sweep.md
created: 2026-10-24
milestone: M5 Trust
---

# The `add` dry run reports what the commit records

What: `add <zip> --dry-run` says what the real install would record, in the same shape as the install's
report: the renamed entry of 077 as its outcome, and the folders the new version drops in the JSON's
`removed_folders`; today the dry run runs the commit's refusals (124) but not its bookkeeping lines.
Why: a script that reads the dry run's JSON to decide should see the same fields the install writes, and a
user should see the folders an update would drop before saying yes.
Done when: the dry run of the 077 and 108 fixtures prints the renamed entry and the dropped folders, text and
JSON, and the install's report over the same zips matches field for field.
