---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 098 — Adopt `--apply`: sidecar entries without a reinstall, and the `.curseclient` guard

## Context

`adopt --apply` writes one sidecar entry per adoptable addon of the report (097): the source spec, the
TOC `## Version` (`unknown` when absent), the folder group (the folders the source names, else the folders
whose TOCs carry the same source id, else the folder alone) and the file hashes, with no download and no
folder change, so the next `update` does a clean install. A flavor with a `.curseclient` marker refuses
`--apply` without `--alongside-curseforge`, and with it writes and warns that the CurseForge app may still
update those folders; the marker is never removed; `add`, `import` and `update` warn in a marked folder too.
Decided 2026-10-22: the flag is `--alongside-curseforge`; a CurseForge-only addon is skipped before the key.
Why: the one write of adopt, and the road to CurseForge independence. The second half of the former 026.
Done when: `--apply` on the fixture writes a sidecar that `update` accepts against the stubs, the marker
guard holds with and without the flag, and a second `--apply` changes nothing.

Decided 2026-10-22: runs after 097 on the same branch.

## Docs that move

- `docs/development.md`: the adopt write, the folder groups, the marker guard on every install path
- `README.md`: `adopt --apply`, `--alongside-curseforge`, the one-manager-per-folder line

## Not in this change

- A reinstall or download at adopt time (the next `update` does it); removing the marker; the CurseForge
  source (096)

## Steps

1. [x] `adopt.rs`: the folder group of an addon by the three rules (the source's folders: WoWInterface
       `UIDir`; else the folders whose TOCs carry the same source id; else the folder alone), the entry with
       the spec, the TOC version or `unknown`, the group and the file hashes, no download
2. [x] `--apply` writes the entries through the state sidecar; a second `--apply` changes nothing; a
       CurseForge-only addon is skipped with a line
3. [x] The `.curseclient` guard: `--apply` refuses without `--alongside-curseforge` and warns with it; `add`,
       `import` and `update` warn in a marked folder
4. [x] Tests: `--apply` on the fixture, then `update` against the stubs installs cleanly over an adopted
       entry; the guard with and without the flag; the second apply; the warnings of the three commands

## Log

- 2026-10-22 — promoted; runs after 097 on the same branch
- 2026-10-22 — the folder group by the three rules came with 097's report (`adopt::report`); this task
  writes it.
- 2026-10-22 — `adopt::apply(report, state) -> Applied`: per row with a source to be adopted as, an `Entry`
  with the spec, the TOC's version (`unknown` when it has none or an unsubstituted token), the group, the
  SHA-256 of every file as `<Folder>/<path>` (`install::sha256_of`; a link inside a folder not followed), no
  previous version, no pin, no mark, the stable channel, `installed_at` the time of the adopt. A row with
  sources the run cannot write (a missing key, CurseForge only) is a `NotAdopted` with the report's verdict as
  its reason; a name or folder the state knows already is skipped. `Adopted::line`, `NotAdopted::line` and
  `Report::applied_summary` are the lines; the report's own summary gained `; adopt --apply adopts <k> of
  them`, and its marker line names the flag.
- 2026-10-22 — the guard: `adopt::marker_refusal` (the `--apply` refusal in a folder with the
  `.curseclient` marker, before any source is asked), `adopt::alongside_warning` (with
  `--alongside-curseforge`) and `adopt::curse_client_warning` (the install paths). The marker is read, never
  written.
- 2026-10-22 — CLI: `adopt --apply` and `--alongside-curseforge` (which needs `--apply`); the sidecar is
  written once, and only when an entry was added; the lines are the report's, then one per adopted addon,
  the warnings (the marker or the alongside warning, the skipped addons) and the summary; `--json` gained
  `apply`, `adopted[]` and `skipped[]`. `add` (a zip, its dry run, and the install of a source), `import` and
  `update` add the marker warning to their warnings.
- 2026-10-22 — tests: the core's `tests/adopt.rs` (the entries of `--apply` with the Wago key, their hashes
  and the `unknown` version, the second run over the written state, the run without the key, the three
  marker lines) and the CLI's `tests/adopt.rs` (`--apply` with the key, `list` reading the adopted folders as
  managed, `update --all --dry-run` over every adopted entry, `update MrglWagoOne --yes` installing the
  source's 2.0.0 over the adopted 1.0.0 and removing the options folder the build does not have; the second
  `--apply` leaving the sidecar byte for byte; the guard without the flag in text and JSON, the flag without
  `--apply`, the flag with it and the marker left in place; the warning of `add`, `import --dry-run` and
  `update --dry-run --json` in the marked folder and none in the other).
- 2026-10-22 — docs: `docs/development.md` (the CLI's `adopt` flags, the JSON fields, a paragraph on the
  write and one on the marker guard on every install path), `README.md` (an `adopt --apply` line in the
  usage block, a paragraph on `--apply`, `--alongside-curseforge` and one manager per folder).

## Deviations from the plan

- The folder group landed with 097 (its report counts addons); this task writes it.
- Not only a CurseForge-only addon is skipped: any addon whose sources all need something the run lacks is
  skipped with what it needs (a Wago addon without `MRGL_WAGO_KEY` reads `needs MRGL_WAGO_KEY`), by the same
  reasoning as the decision on CurseForge: an entry the next update cannot ask about is not written.
- An adopted entry whose TOC version equals the source's current version reads `up to date`, and nothing is
  installed until the source's version moves; the clean install comes with the first version that differs.
- The entry has no field of its own that says it was adopted; `installed_at` is the time of the adopt and the
  sidecar schema stays 1.
- The dry run of `add <spec>` reads no game folder and has no marker warning; every other run of `add`,
  `import` and `update` has it. The window's install paths have no such warning yet (a follow-up).
- In the fixture, WoWInterface project `1006` is in the file list alone (094's shape), so after an adopt the
  update check reports it as source gone; that is the stub's shape, not the adopt's.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 237, the core's `tests/adopt.rs` 7, the
  CLI's `tests/adopt.rs` 9, `tests/cli.rs` 147, the other targets unchanged). `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green.
- 2026-10-22 — by hand, the debug `mrgl` with `MRGL_HOME` under a temporary folder, the key store off, no
  discovery, `MRGL_HTTP_STUB` at a temporary copy of the `releases/` and `sources/` fixtures, `MRGL_ROOT` at a
  temporary copy of `tests/fixtures/adopt/` and a made-up Wago key: `adopt --flavor forever --apply` adopted
  six addons and skipped `MrglCurseOnly`, a second run adopted nothing, `update --all --dry-run` checked all
  six (four updates, one up to date, `1006` source gone), `update MrglWagoOne --yes` installed 2.0.0 and
  removed `MrglWagoOne_Options`; `adopt --flavor retail --apply` refused with exit 1 and wrote nothing, with
  `--alongside-curseforge` it adopted `MrglCompat` with the warning, and `update --all --dry-run --flavor
  retail` carried the marker warning; the `.curseclient` file stayed. No game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
