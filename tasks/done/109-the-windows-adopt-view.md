---
size: M
origin: done/097-adopt-the-report-of-the-sources-of-an-addons-folder.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 109 — The window's adopt view: the report and `--apply` in the toolbox

## Context

The toolbox shows the adopt report of the chosen flavor (097: per unmanaged addon its sources and
whether each offers a build, the summary counts, the `.curseclient` marker) and applies it with one click
(098: the same sidecar write, the `--alongside-curseforge` confirmation as a dialog, the warning after),
over the core's lines and counts, with the same Rust and Node tests as the toolbox's other views.
Why: adopt is the road to CurseForge independence, and the window is where most users will take it.
Done when: a window test over the adopt fixture shows the report's rows and counts, the apply writes the
sidecar the command line's test accepts, and the marker guard holds with and without the confirmation.

Decided 2026-10-22: the second of two window follow-ups of adopt on one branch (110, 109).

## Docs that move

- `docs/development.md`: the toolbox's adopt view and its commands
- `README.md`: the window's adopt paragraph, next to the command line's

## Not in this change

- A new core line or count (the view shows the core's); the CurseForge source (096)

## Steps

1. [x] `toolbox.rs` and `commands.rs`: the adopt report of a flavor as a toolbox view over `adopt.rs`'s
       report, the apply command over the same write, the marker confirmation and the warning after
2. [x] The UI: the adopt section of the toolbox (rows, counts, the apply button, the confirmation dialog), the
       sketch's mock data for it
3. [x] Tests: the window tests over the adopt fixture (the rows and counts, the apply's sidecar, the guard with
       and without the confirmation), the Node tests of the section

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `toolbox.rs`: `adopt_report(root, flavor, home, sources, has_key, voice)` scans the flavor,
  reads its sidecar with the entries whose folders are gone dropped (as `mrgl adopt` does) and answers
  `adopt::report` as an `AdoptView`: the flavor, the folder, `curse_client`, `marker` (the warning of
  `Report::marker_line`), `rows` (the core's `Adoptable` as `mrgl adopt --json` writes it, with its `line`,
  `verdict` and `details`, the lines under it without the indent), `left_out`, `counts` and `summary`
  (`Report::summary`); the messages are the scan's, the sidecar's and the left-out warnings, then the
  summary. `adopt_apply(root, flavor, alongside, …)` refuses a flavor with the `.curseclient` marker without
  `alongside` (`adopt::marker_refusal`, an error, before any source is asked), else reports, applies
  (`adopt::apply`), writes the sidecar once when an entry was added, and answers the report as it is now
  (a second report over the written sidecar: the adopted addons are no rows any more), the flavor's rows
  as they are now, the `Adopted::line` of each entry, and the messages of `adopt --apply`
  (`adopt::alongside_warning` in a marked folder, the warnings, a `NotAdopted::line` per skipped addon,
  `Report::applied_summary`). Without the mrgl home both answer an error and read nothing.
- 2026-10-22 — `commands.rs`: `adopt` (the report through `Sources::from_env` and `keys::of_process`, on a
  blocking thread, writing nothing) and `adopt_apply` (refused while an update runs, one toolbox write at a
  time through `ToolboxRun`, so an update waits for it too); `mark` is refused while a toolbox write goes,
  since an adopt writes the sidecar as a mark does. The busy lines name the adopt.
- 2026-10-22 — the UI: a third toolbox card, "Adopt": "Ask the sources" (the report on demand), the
  summary, the marker's warning line, a table row per unmanaged addon (name with its folders, version, the
  core's line per source and page, the verdict after a pill of its class), the adopt button with the count
  (`Adopt 6 addons`, `Nothing to adopt`, and why it waits in its title), the `adopted …` lines of the last
  apply; in a folder the CurseForge app manages the button opens a `<dialog>` with the marker's line, and
  only "Adopt alongside the CurseForge app" sends the apply with `alongside`. An apply puts the flavor's new
  rows in place, so the Addons tab shows the adopted addons as managed. `toolbox.js`: `adoptRows`,
  `classLabel`, `askButton`, `adoptButton`; `data.js`: `Data.adopt` and `Data.adoptApply`.
- 2026-10-22 — the sketch (`mock-data.js`): made-up sources for the unmanaged rows of every flavor (a row of
  each class, a Wago source that needs its key, a page), the report and the apply in the core's words, the
  guard and the warning in the three flavors the CurseForge app manages, the adopted rows turned managed.
- 2026-10-22 — tests: `crates/mrgl-app/tests/adopt.rs` over a copy of the adopt fixture with the source
  stubs: the report's lines, counts and summary as `mrgl adopt` prints them, the Wago key kept out of the
  answer; the marker's line in retail; the apply with the Wago key (the six `adopted …` lines, the skipped
  CurseForge-only addon and the summary of the command line, the sidecar entries the command line's test
  checks, no folder changed, the remaining rows, the flavor's rows managed), then the window's own update
  of `MrglWagoOne` installing the source's 2.0.0 over the adopted 1.0.0 and dropping the options folder,
  and a second apply leaving the sidecar byte for byte; the guard without the confirmation (the refusal,
  nothing written) and with it (`MrglCompat` adopted, the warning first, the marker kept); no home folder;
  the keys of the contract. Node: the card's rows, class words and both buttons (`toolbox.test.js`), the
  two commands (`data.test.js`), the sketch's report and apply with the guard (`mock-data.test.js`).
- 2026-10-22 — docs: `docs/development.md` (the layout, the "Adopt" section, the UI's toolbox card, the
  sketch, `toolbox.js`, the window tests, the commands `adopt` and `adopt_apply`, the mark and the one
  toolbox write), `README.md` (the window's adopt paragraph next to the command line's),
  `crates/mrgl-core/tests/fixtures/README.md` (the window's use of the adopt fixture).

## Deviations from the plan

- The report is read when "Ask the sources" is pressed, not when the toolbox tab opens: it asks the sources
  of every unmanaged addon over the network, where the other cards read the disk only.
- The apply asks the sources again, as `adopt --apply` does, and does not write the report the card shows:
  the sidecar and the folders may have changed since. Its answer carries a second report over the written
  sidecar, so the card shows what is left to adopt.
- A mark is refused while a toolbox write goes, and the busy lines of the toolbox name the adopt: an adopt
  writes the flavor's sidecar, as a mark and an update do.
- The confirmation is a `<dialog>` of the page, not a native message box: it shows the core's marker line,
  and the page's own words are its question and its two buttons.
- The card shows the core's lines as the command line prints them, among them the ones that name the
  command line's flags (`adopt --apply adopts 5 of them`, `--alongside-curseforge` in the marker's line and
  in the refusal); no core line was added (a follow-up).

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them `crates/mrgl-app/tests/adopt.rs` (6),
  `tests/update_actions.rs` (14), `tests/toolbox.rs` (13) and `ui_smoke.rs`, whose `node --test` runs the
  Node tests of `tests/ui/` (87, with the adopt card's, the commands' and the sketch's). `cargo fmt
  --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets
  --locked` and `python -I scripts/changelog.py --check` green; `tasks_check` without `write`: only the
  stale index, which the main session regenerates.
- 2026-10-22 — the sketch (`ui/` served by `python -I -m http.server`, no core) in a headless browser:
  Forever's card asked its sources and drew three rows, the summary and `Adopt 1 addon`; in retail the
  marker's line showed over the buttons, the button opened the dialog with the marker's line, "Adopt
  alongside the CurseForge app" adopted `Details` with the warning first, the skipped `DBM-Core` and the
  summary as toasts, and the card then read `Nothing to adopt`. No manual round on the window itself (one
  manual round at a time across the sessions).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
