---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M3 Toolbox
---

# 005 — Dev addon linking: junctions for a development folder's addons; CLI `link` and `unlink`

## Context

`mrgl link <folder> --flavor <id>` creates a directory junction under `Interface\AddOns` for every
subfolder of a development folder that has a TOC (junctions need no admin rights, unlike symlinks),
records them as linked in the sidecar, and `unlink` removes exactly those junctions. It replaces a
hand-run link script. Addons one develops oneself are loaded from their repo, not copied; the manager
must know them so an update never overwrites a junction.
Done when: a fixture development folder links and unlinks in a temporary AddOns folder, a real folder is
never touched by `update`, and the scan lists a junction as linked.

Decided 2026-10-21: the junction is made by the tool itself (a reparse point through the Windows API or
a small crate on the license allowlist), never by shelling out to `mklink`; a linked addon is a sidecar
entry with the source `link:<development folder>`, so the install engine refuses to install over it and
`remove` is not the way out, `unlink` is; a name that already exists under `AddOns` as a real folder is
skipped with a warning, never replaced; on a non-Windows target the two commands report that junctions
are Windows-only and the tests that need one are Windows-only.

## Docs that move

- `README.md`: the `link` and `unlink` usage lines.
- `docs/development.md` ("Layout"): the `link` module and the `link:` source in the sidecar.
- `CLAUDE.md` ("Data outside the repo"): no change (a junction target is the user's folder, never in the
  repo).

## Not in this change

- The window's link action (M2); watching the development folder for new addons; symlinks.

## Steps

1. [x] `link.rs`: find the addon folders of a development folder (a TOC named after the folder), make
       the junctions under `AddOns`, record them in the sidecar as `link:<folder>`; `unlink` removes
       exactly those junctions and their entries
2. [x] The install engine refuses to install over a linked entry with a clear error; the scan's `linked`
       flag is set for a junction (`scan.rs` already reads the reparse flag; confirm against a real junction)
3. [x] `mrgl-cli`: `link <folder> --flavor <id>` and `unlink <folder> --flavor <id>` replace the
       `not_yet` stub (the stub test then points at `update`, task 006), the voice lines, `--json`
4. [x] Tests on a fixture development folder and a temporary AddOns folder: link, scan shows linked,
       install over it refused, unlink leaves the folder empty and the user's folder untouched
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `link.rs`: `link` canonicalizes the development folder, finds its addons with the scan's own
  rule (`scan::scan_addons`: a subfolder with a base, suffixed or legacy TOC named after it), reconciles the
  sidecar, and per addon makes a junction under `AddOns` and records an entry `link:<folder>` with the TOC's
  version (or `unknown`), the one folder and an empty file list. `unlink` takes the entries of that source:
  a junction whose target lies under the development folder is removed (`remove_dir`, the link itself), a
  folder gone already is forgotten, a real folder is left in place and forgotten, a junction pointing
  elsewhere is left in place with its entry. The development folder may be gone (`dev_folder_lenient`).
- 2026-10-21 — the junction is made by the tool itself: an empty folder, opened with
  `FILE_FLAG_OPEN_REPARSE_POINT`, and one `DeviceIoControl` with `FSCTL_SET_REPARSE_POINT` and a mount-point
  `REPARSE_DATA_BUFFER` (`\??\<target>` as the substitute name, the Win32 path as the print name); the
  folder is removed again when the call fails. `windows-sys` gets `Win32_System_IO` and `Win32_System_Ioctl`
  (one feature list change with its reason in `Cargo.toml`); no crate joins, `cargo deny check` green.
- 2026-10-21 — confirmed: `std` reports a junction as a symbolic link (`FileType::is_symlink` is true for a
  name-surrogate reparse tag, and the mount point's `0xA0000003` is one), so the scan's `linked` flag is set
  for a junction with no change; the scan's warning now names a junction whose target cannot be read
  (a dangling one) as such instead of following it.
- 2026-10-21 — the guard: `Engine::refuse_links` (in `link.rs`) runs in `Engine::install` after the staging
  and before the commit, and in the CLI's `add --dry-run`: a folder owned by a `link:` entry is refused with
  `Error::LinkedAddon`, a junction or symbolic link mrgl did not make with `Error::ForeignJunction`.
  `Engine::remove` refuses a linked entry (`refuse_remove_linked`), so `unlink` is the only way out.
- 2026-10-21 — `mrgl link` / `mrgl unlink` replace the `not_yet` stub (its test now points at `update --all`,
  task 006): a progress line, a warning line per addon skipped or kept, a summary (success, nothing to do
  when every addon was linked already, warning when one was skipped), and the JSON documents `link`
  (`folder`, `source`, `linked` with `name`, `version`, `target`, `already_linked`, `skipped`, `without_toc`,
  `warnings`) and `unlink` (`folder`, `source`, `unlinked`, `kept`, `warnings`). The appended errors:
  `JunctionsWindowsOnly`, `NoAddonInDevFolder`, `LinkedAddon`, `ForeignJunction`.
- 2026-10-21 — the synthetic development folder `tests/fixtures/dev/` (`OneFolder` with a version, the same
  name as the `one-folder` zips; `DevAddon` without one; `Media` without a TOC), the core tests in
  `tests/link.rs`, the end-to-end tests in the CLI's `tests/cli.rs`; README, `docs/development.md` and the
  fixtures README describe them.

## Deviations from the plan

- The install engine refuses a junction mrgl did not make too (`ForeignJunction`), not only a linked entry,
  and `add --dry-run` answers the same refusal; the plan named the linked entry only.
- A junction under `AddOns` that already points to the same development folder (one made by hand) is taken
  over as linked instead of being skipped as a name in the way.
- An entry whose junction was replaced by a real folder by hand is forgotten, by `link` and by `unlink`, so
  the folder is the user's and an install may take it over.
- The refusal sits in `Engine::install` (and the dry run), outside `commit`, because two other branches
  change `commit` at the same time; an install path that calls `stage` and `commit` directly has to call
  `refuse_links` itself until the check moves into `commit` (a follow-up of the pull request).
- `client::strip_verbatim` is shared with the link module (`pub(crate)`); the scan's warning text for an
  unreadable junction changed (`scan.rs`).
- No round on a real game folder in this change (the real AddOns folders are a shared resource of the
  sessions); that round is a follow-up of the pull request.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `crates/mrgl-core/tests/link.rs` (6 tests, Windows)
  copies the fixture development folder into a temporary one and, against a temporary game folder and mrgl
  home: `link` makes `DevAddon` and `OneFolder` (versions `unknown` and `9.9.9-dev`) and leaves `Media` out;
  the scan lists both as linked with the version read through the junction; the entries carry
  `link:<folder>` and no file list; a second `link` reports both as linked already; installing
  `one-folder-1.0.0.zip` fails with `refused to install over OneFolder (flavor forever): it is linked from
  <folder>; `mrgl unlink` with that folder removes the link` and `remove` with `refused to remove OneFolder
  …`, the junction still in place; `unlink` leaves the AddOns folder empty, the sidecar without an entry,
  and the development folder equal byte for byte to its snapshot before the link; the zip then installs as
  a real folder. A real folder named `DevAddon` is skipped (`a folder of that name is in AddOns already and
  stays (not a junction)`) with its file untouched, a junction to another folder is skipped and an install
  over it refused (`ForeignJunction`), and once its target is deleted it is reported as `a dangling junction
  to … (its target is gone)` and the scan skips it with a warning. A hand-made junction to the same folder
  is taken over; a junction replaced by a real folder is kept and forgotten; with the development folder
  deleted, `unlink` removes the dangling junction of its own. A folder without addons and a missing folder
  are errors. The unit tests check the mount-point reparse buffer byte for byte. The same through the exe
  in `crates/mrgl-cli/tests/cli.rs` (3 tests): the two-line progress and summary, `list` flagging both lines
  `linked`, `add` and `add --dry-run` and `remove` refused with exit 1, the murloc lines on a second `link`,
  the `link` and `unlink` documents, the warning line for a real folder in the way, and `nothing linked from
  …` on a second `unlink`.
- 2026-10-21 — a round with the debug exe in a temporary game folder: after `mrgl link`, PowerShell's
  `Get-ChildItem` reports both folders with `LinkType` `Junction` and the target in the development folder,
  `fsutil reparsepoint query` reports the tag `0xa0000003` (Microsoft, name surrogate) and `dir` lists both
  as `<JUNCTION>`; `mrgl list` shows them `linked`; after `mrgl unlink` the AddOns folder is empty and the
  development folder still holds its five files.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked`, `python -I scripts/changelog.py --check` and `cargo deny check` (the manifests
  changed): all green. The non-Windows paths (`ensure_supported`, the `cfg(not(windows))` tests) are not
  compiled locally: a check for another target stops at `ring`'s C build without a cross compiler.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
