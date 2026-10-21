---
status: decided
date: 2026-10-21
cost: medium
sources: [wip/001-project-scaffolding.md, docs/nerglish.md]
---

# D-005 — Nerglish voice: the murloc language is the tool's voice, not an easter egg

## Context

The tool is named after the murloc's call. A name like that invites a joke in the `--help` text and nothing
more; the decision here is whether the murloc language is decoration or the voice of every message, and
what that must never cost in clarity, machine-readability or legal standing.

## Decision

- Every human-readable message of the command line and the window is **two lines**: a murloc line picked by
  the message's kind (banner, progress, success, nothing to do, warning, error, fatal) and a plain English
  line that carries every fact. Errors too: `Rwlrwlrwlrwl?!` then `addon not found: Questie (flavor forever)`.
- **JSON output and log lines are always plain.** `--serious` on any command, or `MRGL_SERIOUS=1`, drops the
  murloc line; the flag wins over the variable. Exit codes are ordinary numbers.
- The vocabulary lives in one table (`crates/mrgl-core/src/voice.rs`, documented in `docs/nerglish.md`);
  every message goes through it and nobody writes a murloc string by hand. Tests assert the plain line.
- **Blizzard's own files never enter the repo**: no sound clip, texture or icon from the game client, which
  the game's license forbids extracting and which this repo's MIT license could not cover. Our own
  murloc-inspired work is fine: the voice is text we wrote, and artwork we draw ourselves (an icon, an
  illustration) is fan art under Blizzard's fan content terms, non-commercial, without a Blizzard logo and
  without an implied endorsement. The README says the trademarks are Blizzard's. Decided 2026-10-21.

## Not chosen

- The murloc as a theme only (names, banner, help text) with plain messages: safer, but the name would
  promise more than the tool delivers.
- Murloc lines only on summaries and errors, not on progress: a reasonable reduction, kept as the fallback
  if the two-line output proves noisy in long `update --all` runs; the `serious` switch covers that need for
  now.
- A murloc line that carries facts (counts or names in Nerglish): a reader who skips the murloc line would
  lose information, and tests would have to assert murloc.
- Any Blizzard sound clip or game artwork: not ours to ship.
- No murloc depiction at all, only an abstract glyph: stricter than the fan content terms ask, and the
  name invites a face.
