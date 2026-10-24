---
size: M
origin: done/121-a-hardening-review-of-the-code-that-touches-the-disk-the-network-and-the-keys.md
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
---

# 124 — The low findings of the hardening review in one sweep

## Context

The ten low findings of 121 (L1 to L10 under its Verification) fixed on one branch, each with its test:
`https_only`; a body bound per request kind; an owner or repository of dots only refused; control characters
replaced in the CLI's human lines; a bound on a zip's entry count; a component ending in a dot or a space
refused; a collision-free cache folder name, device names escaped; secret header values hidden in `Debug`, the
doctor's token through `may_send`; the running-client check repeated after a long `WTF` write; the folder key
store for the tests only or warned about. An eleventh item from 123: `add --dry-run` runs the refusals of the
folders the previous version drops (a foreign junction, a recorded `..`) as the real commit does.
Why: each is contained today; together they close the edges the review named, before the signed release of M5.
Done when: every item has its test and a line in the task file, the docs name the new bounds and rules, and the
coverage stays at 100%.

Decided 2026-10-24: one branch and one pull request for the eleven items, one commit per item or per group of
related items, the task file's Log one line per item; M1 and M2 of 121 are not part of it.

## Docs that move

- `docs/development.md`: the HTTP client's bounds and `https_only`, the zip entry rules (the count bound, the
  trailing dot or space), the cache folder naming, the keys section (the `Debug` rule, the folder store), the
  `WTF` writers' second client check, the dry run's refusals
- `README.md`: a user-visible line only if a bound or a warning changes what the user sees (the second client
  check's warning, the folder store's warning)
- `CONTRIBUTING.md`: only if the folder key store moves behind a test feature and the test instructions change

## Not in this change

- M1 (a body timeout) and M2 (a flavor lock across processes) of 121
- The CurseForge manual round (096) and anything that needs a real key

## Steps

1. [ ] L1 `https_only(true)` on the agent; L2 a body bound per request kind (API answer and `release.json`
       small, an asset 512 MiB); a test each against the local HTTP stub
2. [ ] L3 an owner or repository of dots only refused in the GitHub spec and in `X-Website`; L7 a collision-free
       cache folder name (a short hash appended when `file_safe` changed the name), reserved device names
       escaped; tests
3. [ ] L4 control characters (U+0000 to U+001F, U+007F to U+009F) replaced with U+FFFD in every human line the
       CLI prints, JSON untouched; a test through the voice
4. [ ] L5 a bound on the entries of one zip, refused before anything is written; L6 a component ending in a
       dot or a space refused by the entry rule (sharing `zip_entry`'s checks with 123's folder rule); fixtures
       and tests
5. [ ] L8 secret header values hidden in `Request`'s `Debug`, the doctor's token as a `keys::Key`, its
       rate-limit request through `may_send`; tests
6. [ ] L9 the running-client check repeated after the unpack of a `WTF` restore, a set apply, a set undo and
       the SavedVariables cleanup, with a warning line when a client started meanwhile; tests with the stand-in
7. [ ] L10 the folder key store built for the tests only (a cargo feature the test runs turn on) or named by
       `key list` and the doctor while active; decide, record, test
8. [ ] The dry run of `add` and `update` runs the refusals of the dropped folders as the commit does; a test
       over a foreign junction and a recorded `..`
9. [ ] The docs; `cargo test --locked` and the coverage at 100%

## Log

- 2026-10-24 — promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
