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
- 2026-10-24 — L1: the real client's agent is `https_only(true)`, so a plain `http://` address or a redirect
  to one fails before a connection (`configured for https only`); a unit test asks `http://127.0.0.1:9/`
  and gets the error without the port being asked.
- 2026-10-24 — L2: `Request::max_body`, the bound of the answer's body, `sources::MAX_ANSWER_BYTES` (32 MiB)
  by default and `sources::MAX_DOWNLOAD_BYTES` (512 MiB) through `Request::download`, which the asset
  download of every source asks for (GitHub, Wago, WoWInterface, CurseForge on every hop); a longer body
  fails as the network does (`reading the body: the answer is longer than the bound of 32.0 MB for an API
  answer`), in the real client and in the folder stub alike. Decided 2026-10-24: two bounds, not one per
  answer: the largest API answer, WoWInterface's file list of every project, is some 5.6 MB, so 32 MiB
  keeps a margin for its growth and still cuts the memory an API answer can take from 512 MiB to 32;
  a `release.json` gets no bound of its own, since it shares the API answers' and a third constant would
  guard nothing more. Tests: the bound in the real client's reader and in the stub, and each source's
  recorded requests carry the download bound on the asset and the answer bound on the API.
- 2026-10-24 — L3: one rule for a GitHub owner or repository, `sources::is_github_name` (ASCII letters,
  digits, `-`, `_` and `.`, and not dots only), which the spec, its page address and adopt's `## X-Website`
  share; `github:../..`, `github:./x`, `github:o/...` and `https://github.com/o/..` are refused with the
  spec form, `.github` still passes. Tests in `sources` and `toc`.
- 2026-10-24 — L7: the zip cache's folder and file names go through `state::cache_name`: the text itself
  when `file_safe` keeps it as it is and it is no reserved device name, else its safe form with `-` and the
  first 16 hex digits of the text's SHA-256 appended, a `_` in front of a device name (with or without an
  extension, the check of `zip_entry`). Decided 2026-10-24: 16 hex digits (64 bits), since the names come
  from zips and source answers and 8 digits would let a crafted name be searched to share another
  addon's folder in minutes; and no fallback to the old folder name, since that folder may be another
  addon's (the collision itself): a zip cached before this change under a name or a version that now
  changes is not found by the rollback, which says the zip is missing. Unit test over safe names, names
  that differ only in the characters `file_safe` replaces, names of dots only and device names.
- 2026-10-24 — L4: `voice::printable` turns every control character (U+0000 to U+001F, U+007F to U+009F) but
  the line break into U+FFFD, and the command line's `Reporter` passes every human line through it, a
  message on stdout or stderr and a data line alike; the JSON documents stay serde's. A unit test through
  the voice (an escape sequence, a carriage return, a tab, DEL and a C1 control) and an end-to-end `scan` of
  a TOC whose version holds `ESC[2J` and a carriage return: U+FFFD on the screen, `\u001b` and `\r` in the
  JSON.
- 2026-10-24 — L5: `zip_entry::MAX_ENTRIES`, 100,000 entries, checked first in `checked_entries` against
  the central directory's count, before any entry is read or written, for an addon zip and a `WTF` backup
  alike: `refused <zip>: it holds <n> entries, more than the bound of 100000 for one zip; nothing was
  written` (`Error::ZipTooLarge` with `Excess::Entries`). Decided 2026-10-24: one bound for both roles and
  no flag to raise it (the size bound's `--max-unpacked` stays the size bound's): some thirty times the
  files of the largest addon and five times a `WTF` folder of fifty characters with two hundred addons'
  SavedVariables each. The unit test runs the check with a bound of two through the private
  `checked_entries_within`, so no fixture of 100,001 entries is needed.
- 2026-10-24 — L6: `zip_entry::refused_component`, a trailing dot or space or what `refused_by_windows`
  refuses, is the one component check of the entry rule and of the folder-name rule of 123
  (`is_folder_name` keeps its empty and separator checks; `.` and `..` end in a dot); `Foo./x.lua`,
  `Foo /x.lua`, `Foo/x.lua.` and `Foo/.../x.lua` are refused, a leading space and an inner dot pass. The
  fixture `trailing-dot-entry.zip` (`TrailingDot/` next to `TrailingDot./`, 550 bytes) is refused by
  `install` with nothing written; a rerun of `make-zips.py` left every other fixture zip byte-equal.

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
