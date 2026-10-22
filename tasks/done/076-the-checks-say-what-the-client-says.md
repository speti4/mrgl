---
size: S
origin: done/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 076 — The dependency and interface checks say what the client says: an embedded-only dependency is missing, a lower interface number is out of date

## Context

A required dependency that exists only as a library embedded in the addon's own folder is reported as
missing, with the reason that the client does not count an embedded folder (029's rule changes; `nolib` is
unaffected); and `scan` and `list` mark a TOC whose highest interface number of the flavor's expansion is
below the installed build as `out of date`, in the client's words, apart from the other-expansion mismatch.
On the real client a probe addon with an embedded-only required dependency showed `Dependency missing`
and did not load (the reopen condition of 052), and after a client update every addon with the previous
interface number showed `Out of date` while `scan` said nothing.
Done when: the dependency fixture with an embedded-only required library reports it missing with the reason,
the embedded case of an optional dependency is unchanged, and a fixture TOC one interface number behind the
build is marked `out of date` while a current one and an other-expansion one keep their states.

Decided 2026-10-22: first of the chain 076 → 080 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the dependency check's *embedded* rule and the interface check's `out of date` state.
- `README.md`: the dependency paragraph, if it states when a dependency counts as there.

## Not in this change

- The optional dependencies (never reported); the window's row text beyond what the core reports; the
  shared `Blizzard_` constant (080).

## Steps

1. [x] `dependencies.rs`: an embedded-only required dependency is *missing* with the reason the client does
       not count an embedded folder; the `embedded` list stays informative; `nolib` unaffected
2. [x] `interface.rs`, `scan` and `list`: a TOC whose highest interface number of the flavor's expansion is
       below the installed build is `out of date`, next to the other-expansion `interface mismatch`
3. [x] Tests over the fixtures: the embedded-only case, the optional embedded case unchanged, a TOC one
       number behind, a current one and an other-expansion one; CLI text and JSON; the window's rows
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 076 → 080
- 2026-10-22 — `dependencies.rs`: a required name that no addon folder of its own answers is *missing*;
  when a folder inside the addon's own folder answers it, it is also listed in `embedded`, which is now
  informative, a subset of `missing`. The warning names the reason after the usual line: `(LibEmbedded is
  only embedded inside Needy, and the client does not count a library inside an addon's own folder)`. A
  `nolib` entry still skips the lookup and keeps its own reason; an optional dependency, embedded or not, is
  never reported. `list`, `add` and the window's `missing` field follow without code of their own (they
  read `missing`).
- 2026-10-22 — `interface.rs`: `InterfaceOutOfDate` (`folder`, `toc`, `interfaces`, `build`, the build's
  `interface`, the TOC's `highest` number of the build's major) with `out_of_date_highest`,
  `out_of_date_addon` and `out_of_date_scan`. A TOC is out of date when its highest number of the build's
  major is below `BuildVersion::interface()`; a TOC with no number of that major stays an
  `InterfaceMismatch` and is never out of date as well; a number above the build's is current. `scan` and
  `list` append `  out of date` to the line, add one warning (`1 addon is out of date for the build
  12.1.5.70448 (interface 120105): Behind (120100); the client lists it as out of date`, a `Kind::Warning`
  of the voice) and carry `interface_out_of_date` in their JSON next to `interface_mismatches`.
- 2026-10-22 — fixtures: `dependencies/_forever_/.../Needy/Libs/LibOptional/` (the embedded optional
  dependency) and a fourth game folder `interface/` (`_retail_` at `12.1.5.70448` with `Current`, `Behind`
  at 120100 and `Elsewhere` at 110207); `tests/fixtures/README.md` lists both.
- 2026-10-22 — tests: new `an_embedded_only_dependency_is_missing_with_the_reason` and
  `a_lower_number_of_the_builds_major_is_out_of_date_and_not_a_mismatch` (core unit),
  `scan_and_list_mark_an_out_of_date_toc_apart_from_an_interface_mismatch` (CLI: text of `scan` and `list`,
  and both JSON documents); `the_fixture_flavors_have_no_mismatch` also asserts no out-of-date addon. The
  existing expectations the rule change moves: core unit `a_required_name_is_met_embedded_or_missing`
  (`missing` and the warning) and `a_nolib_build_does_not_count_a_folder_inside_as_embedded` (one assertion
  added); `tests/dependencies.rs` `met_embedded_and_missing_over_the_fixture` (`Needy` and `NolibBuild`
  missing `LibEmbedded`, the warning, `LibOptional` never reported) and the nolib test, renamed to
  `the_sidecars_nolib_flag_skips_the_folder_inside_and_names_the_nolib_build` with its body unchanged (the
  folder inside no longer turns anything into missing, it already is); CLI
  `list_warns_about_a_missing_dependency_and_marks_the_line`, `list_json_carries_the_dependencies_of_every_addon`
  and `add_warns_after_the_install_when_a_dependency_is_missing` (the embedded-only name in `missing` and the
  warning); the window's `flavor_list` fixture gains `Needy/Libs/LibInside`, required by `Needy`, and the
  row's `missing` is `["LibGone", "LibInside"]`.
- 2026-10-22 — docs: `docs/development.md` (the dependency check's *embedded* rule, the `embedded` field as a
  subset of `missing`, the interface check's `out of date` state and its JSON, the fixtures), `README.md`
  (when a dependency counts as there; the `out of date` mark).

## Deviations from the plan

- The out-of-date fixtures are a fourth game folder, `interface/`, rather than more addons under
  `flavors/_retail_`: the existing `scan` tests count that flavor's addons.
- Out of date is a type of its own, `InterfaceOutOfDate`, under its own JSON key, `interface_out_of_date`;
  `InterfaceMismatch` and `interface_mismatches` keep their shape. `add <file.zip>` and `import` still warn
  about a mismatch only (the task names `scan` and `list`); the window's row contract is unchanged, since no
  core type of a row changed.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 186, `tests/dependencies.rs` 2, CLI
  `tests/cli.rs` 119, the window's `flavor_list` 13 and the UI smoke test with Node on `PATH`).
  `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.
- 2026-10-22 — `mrgl scan` over `fixtures/interface/_retail_` (a temporary `MRGL_HOME`): `Behind  1.0.0
  120100  out of date`, `Current  1.0.0  120105,50504` with no mark, `Elsewhere  1.0.0  110207  interface
  mismatch`, each warning under its murloc line; `mrgl --serious list --flavor` over
  `fixtures/dependencies/_forever_`: `Needy ... missing: LibEmbedded, LibMissing` and `NolibBuild ...
  missing: LibEmbedded`, each warning with the embedded reason. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
