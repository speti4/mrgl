---
status: idea
size: S
origin: done/064-blizzard-addons-are-not-missing-dependencies.md
created: 2026-10-22
milestone: M1 Core + CLI
changelog: housekeeping
---

# Housekeeping after the M1 fixes: one `Blizzard_` prefix, the doctor's formatter alias, the voice sample in the docs

What: `dependencies::CLIENT_PREFIX` and `savedvars::CLIENT_PREFIX` become one shared constant and helper;
the doctor calls `zip_entry::size_text` directly and the `bytes_text` alias import goes; and the planned voice
sample in `README.md` and `docs/nerglish.md` shows the per-addon progress lines of the update check (068) as
the command line and the window (010) print them.
Why: two copies of the `Blizzard_` rule will drift apart (064, 071), the alias was a bridge for the parallel
branches of wave 5, and the sample predates the progress lines.
Done when: one constant with both callers, no alias import, the sample matches a real `update --all` run's
lines, and every existing test is unchanged.
