# The voice: Nerglish

mrgl speaks murloc. Not as an easter egg: the murloc language is the tool's voice (decision D-005), and this page
is the vocabulary and the rules every message follows. Everything here is text we wrote, and every picture in
the repo is our own drawing: no sound clip, texture or icon from the game client is ever shipped or committed.

## The two-line rule

Every human-readable message is two lines: a murloc line, then a plain English line that says the same thing for
humans who do not speak murloc.

```text
Mrglglglgl!
2 addons updated, 12 up to date
```

Errors too:

```text
Rwlrwlrwlrwl?!
addon not found: Questie (flavor forever)
```

The plain line carries every fact (counts, names, paths, the reason). The murloc line carries the mood. A reader
who skips the murloc line loses nothing.

## What stays plain

- `--json` output, on every command: machines read it, so it is plain JSON with no murloc anywhere.
- `--help` and the usage errors: `clap` writes them, and a reader looking up a flag is not in the mood.
- Log lines and anything written to a file.
- `--serious` on any command, or the environment variable `MRGL_SERIOUS=1`: the murloc line is dropped and only
  the plain line prints. The flag wins over the variable when both are set.
- Exit codes: `0` success, `1` an error the plain line explains, `2` a usage error (`clap`). Never a murloc
  code.

## The vocabulary

The murloc line is picked by the message's kind, from the table in `crates/mrgl-core/src/voice.rs`:

| Kind | Murloc line | When |
|---|---|---|
| banner | `Mrglglgl.` | `--version`, the greeting of the empty window |
| progress | `Mrrrgl… mrgl mrgl.` | something is running: a scan, a download, an install |
| success | `Mrglglglgl!` | a command finished and did what it said |
| nothing to do | `Mrgl.` | a command finished and found nothing to change |
| warning | `Mrgl? Mrgl…` | finished, but with something to look at |
| error | `Rwlrwlrwlrwl?!` | a command failed and the plain line says why |
| fatal | `Aaaaaughibbrgubugbugrguburgle!` | mrgl itself is broken: a panic, a corrupt sidecar it cannot rebuild |

Variation is fine (a longer `Mrglglglglgl!` for a bigger success), but a kind always sounds like itself: a
reader learns the seven sounds in a day, and an error must never sound like a success.

## Rules for writing a message

1. Call `Voice::say(kind, plain)` on the run's voice (`Voice::from_env`, or the CLI's `Reporter`); never print a murloc string by hand.
2. Write the plain line first, as a full sentence or a `count noun` summary, lower case, no trailing full stop.
3. Tests assert the plain line. The murloc line is the table's business, so a vocabulary change breaks no test.
4. Nouns stay English in both lines: `addon`, `flavor`, `release`. The murloc line never carries a path, a
   count or a name, so it can never be the only place a fact lives.
5. The window shows the same two lines in its status bar and its toasts; the serious setting is a preference.
