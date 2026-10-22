---
size: S
origin: done/065-derive-name-prefers-the-zip-or-repository-folder.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 077 — The marks survive a reinstall under a new derived name, and the pinned line uses the version order

## Context

When a zip in hand is staged under a name the naming rule now derives (065) while the sidecar records
the same folders under the old name, the commit carries the old entry's pin, channel and previous version
over to the new entry instead of dropping them with a warning; and the update check's pinned line applies
the version order of 068, so an installed version newer than the current release is not reported as `<latest>
is out`. An addon recorded as `DBM-Brawlers` before 065 loses its marks on the next install from a zip, and
the window shows "pinned at 1.1.0-beta, 1.0.0 is out" (010).
Done when: a fixture sidecar with the old name, a pin and a previous version keeps all three after the
reinstall, and a pinned fixture addon newer than the current release reports the pin without `is out`.

Decided 2026-10-22: first of the chain 077 → 075 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the install engine's commit rule for an entry recorded under another name, and
  the pinned line of the update check.

## Not in this change

- A rename command for a managed addon; the window's pin at a given version; the version order itself (068).

## Steps

1. [x] `install.rs`: when the staged name differs from the entry that records the same folders, the commit
       carries the old entry's pin, channel and previous version (and its rollback zip) over to the new entry
2. [x] `update.rs`: the pinned line uses the version order of 068 (`listed_versions`, else `lenient_order`), so
       a pinned installed version newer than the current release carries no `<latest> is out`; the window's
       note (`ui/actions.js`) and the CLI line agree
3. [x] Tests: a fixture sidecar with the old name, a pin and a previous version keeps all three after the
       reinstall from a zip; a pinned fixture addon newer than the current release reports the pin without
       `is out`; the CLI text and JSON, the window's row
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 077 → 075
- 2026-10-22 — `install.rs`: `Engine::commit` replaces the entry of the staged name, else
  `renamed_entry`: the entry that records the folder the staged name names while its own name is a folder
  of the staged addon (both ASCII case-insensitive). Requiring both directions keeps an entry that only
  records one folder the zip also ships (a copied library folder) out: it keeps its name and its marks and
  loses that folder, as before. The replaced entry's pin, ignored mark and channel carry over through
  `keep_marks`, its version becomes the previous one (its previous version on the same version), the old
  entry goes without the `no longer managed` warning, and the install says
  `<old> is managed as <new> now, with its marks and its previous version`. `Engine::carry_cache` moves
  the zips of `cache\<old>\` into `cache\<new>\` (a zip already there stays) before the cache is pruned to
  the current and the previous version, and removes the old folder once it is empty; a failed move is a
  warning, the install stands.
- 2026-10-22 — `update.rs`: `AddonUpdate::order` (`VersionOrder`: `older`, `same`, `newer`; `None` without a
  current release or when neither the list nor `lenient_order` can tell) is set for every addon the source
  answered, from the same `installed_order` as the states of 068. The pinned line reads it:
  `pinned at <v>, newer than the current release <latest>, left alone` (with the `is no longer listed`
  reason when the list lacks the installed version), `pinned at <v>, up to date` for the same version,
  `pinned at <v>, <latest> is out, left alone` for an older version or an order nobody can tell.
- 2026-10-22 — window: `list::CheckFact` carries `order`; `ui/actions.js` `noteOf` reads it through
  `pinNote` (`pinned at 1.1.0-beta, newer than the release`, `pinned at 2.0.0, 2.1.0 is out`, no word for
  the same version); `ui/mock-data.js` gives every check fact an `order` and the era flavor a pinned row
  above its release (`AtlasLootClassic`).
- 2026-10-22 — tests: `tests/install.rs`
  `a_reinstall_under_the_derived_name_keeps_the_marks_the_previous_version_and_the_rollback` (the boss zip
  recorded as `Boss-Arena` at 5.0.0 after 4.0.0, pinned, on the prerelease channel; the zip in hand is
  `Boss-Core` with the pin, the channel, the previous version 4.0.0, both cached zips under `Boss-Core`, the
  old cache folder gone, and a rollback to 4.0.0) and
  `an_entry_that_records_a_folder_of_the_zip_under_another_addon_keeps_its_name`; `update::tests`
  `the_pinned_line_follows_the_version_order`; `tests/cli.rs`
  `update_reports_a_pin_newer_than_the_current_release_without_is_out` (text and JSON);
  `tests/flavor_list.rs` (a pinned `Held` row read offline); `tests/ui/actions.test.js` and
  `tests/ui/mock-data.test.js` (the note and the mock's data line).
- 2026-10-22 — docs: `docs/development.md` states the commit rule for an entry recorded under another name,
  the pinned line and the `order` key of the `update` document and of a row's `check`. The naming paragraph
  of task 065 that it extends was missing from `docs/development.md` on `main`, so it is put back with it.

## Deviations from the plan

- The ignored mark carries over too: `keep_marks` keeps the pin, the ignored mark and the channel together,
  as every reinstall does since task 021.
- The pinned line needs the order the check computed, so `AddonUpdate` and the window's `CheckFact` gain one
  field, `order`; the `update` JSON document and a row's `check` carry it as a new key, every other key is
  unchanged.
- `docs/development.md` also gets back the naming paragraph of task 065, which the new rule extends.

## Verification

- 2026-10-22 — by hand in a temporary game and `MRGL_HOME`: `mrgl --serious add boss-core-5.0.0.zip`
  installed `Boss-Core`; the sidecar entry was renamed to `Boss-Arena` with `pinned: 5.0.0`, `channel:
  prerelease`, `previous_version: 4.0.0` and the cache folder `Boss-Arena` holding `4.0.0.zip` and
  `5.0.0.zip`. The same `add` again printed `Boss-Arena is managed as Boss-Core now, with its marks and its
  previous version` and exited 0; `list` showed `Boss-Core  5.0.0  16001  managed  …  pinned at 5.0.0
  prerelease` with `Boss-Arena` and `Boss-GUI` `managed by Boss-Core`, the cache folder `Boss-Core` held
  `4.0.0.zip` and `5.0.0.zip` and no `Boss-Arena` folder was left; `rollback Boss-Core` printed `rolled
  back Boss-Core from 5.0.0 to 4.0.0 (3 folders: …) in _forever_; pinned at 4.0.0, …`, exit 0.
- 2026-10-22 — `update --all --dry-run` over a fixture sidecar prints `Behind  2.0.0  pinned at 2.0.0,
  2.1.0 is out, left alone`, `Held  1.1.0-beta  pinned at 1.1.0-beta, newer than the current release 1.0.0,
  left alone`, `Kept  1.5.0  pinned at 1.5.0, newer than the current release 1.0.0 (1.5.0 is no longer
  listed), left alone` and `dry run: 3 pinned; nothing written`; the JSON document has `order` `older`,
  `newer`, `newer`. The window's offline row `Held` has the check `pinned`, `latest` 1.0.0, `order` `newer`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core unit 185, `tests/install.rs` 36, `tests/cli.rs` 119, `tests/flavor_list.rs` 13, the UI's
  Node tests 60), `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`
  green; the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
