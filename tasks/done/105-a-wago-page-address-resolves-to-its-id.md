---
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 105 — A Wago page address resolves to its `wago:<id>` through the keyless slugs list

## Context

`add https://addons.wago.io/addons/<slug>` (and the same address in a list) resolves the slug to the
addon's id through `Wago::id_of_slug` over the keyless slugs endpoint of the flavor's game version, and the
sidecar records `wago:<id>`; a slug the list does not carry says so.
Why: 092 refuses the Wago page address with a hint, since it names a slug, not an id; 095 brought the slugs
list that answers it without a key.
Done when: the page address of a fixture slug installs from the stub as `wago:<id>`, an unknown slug names
the game version it was looked up in, and a list entry with the address imports.

Decided 2026-10-22: the last of four source follow-ups on one branch (101, 106, 104, 105).

## Docs that move

- `docs/development.md`: the spec forms (the Wago page address)
- `README.md`: the accepted addresses in the usage text

## Not in this change

- A CurseForge page address (its slug needs the key, 096)

## Steps

1. [x] `sources.rs`: the Wago page address parses into a slug form that the router resolves to `wago:<id>`
       through the slugs list of the target's game version before the spec is recorded
2. [x] `add` and the list import record `wago:<id>`; an unknown slug names the game version it was looked up in
3. [x] Tests: a fixture slug installs from the stub, an unknown slug's line, a list entry with the address

## Log

- 2026-10-22 — promoted
- 2026-10-22 — the spec form: `SourceSpec::WagoPage { slug }`, read from `https://addons.wago.io/addons/<slug>`
  (the first path segment after `addons/`, ASCII letters, digits, `-` and `_`; a trailing path, a query and
  a fragment are dropped); its kind is Wago, its display the page address. Another `addons.wago.io` address
  is `unknown source` with the hint `expected https://addons.wago.io/addons/<slug>, the address of the
  addon's page, or wago:<id>, …`. A CurseForge addon page is still refused with where the id is.
- 2026-10-22 — the lookup: `Source::canonical_spec(spec, target)` (default: the spec itself), dispatched by
  the router to the source of the spec's kind. The Wago source looks the slug up in the keyless slug list of
  the target's game version (`Wago::id_of_slug`, now also ASCII case-insensitive) and answers `wago:<id>`,
  or the new `Error::UnknownWagoSlug`, `no Wago addon <slug> in the slug list of game version <v>: it has no
  release for <v>, or the address names no addon`. The source's own `resolve` and `resolve_version` look a
  page up first, so a pick always names `wago:<id>`; `missing` treats a page like `wago:<id>` (without the
  key: `source not configured`, since the install needs it).
- 2026-10-22 — `add`: after the check of what is missing, the spec is turned into the one recorded against
  the engine's flavor (an install) or the `add` target (a dry run), with the progress line `<address> is
  wago:<id>`; the JSON's `source` is the id. `import`: an entry's page address is turned into `wago:<id>`
  after the check of what is missing and checked again against what the flavor has from that source; an
  unknown slug fails that entry and names the game version. The window's badge of a hand-written page
  address shows its slug.
- 2026-10-22 — tests: `tests/cli.rs` `add_and_import_take_a_wago_page_address_and_record_its_id` (the
  fixture slug `mrgl-wago-one` installs from the stub as `wago:MrglWg01`, the line that says so, the dry
  run's JSON, an unknown slug's line naming `forever`, a list with the address and an unknown one: one
  installed as `wago:MrglWg01`, one failed); `tests/wago_source.rs`
  `a_page_address_turns_into_its_id_through_the_keyless_slug_list` (no key sent, a slug in other letter
  case, the unknown slug's line, another spec is its own, a resolve of the page names the id); core unit
  spec tests (the page form, its display and kind, the refused Wago addresses).
- 2026-10-22 — docs: `docs/development.md` (the spec forms, the Wago page under the Wago fixtures
  paragraph), `README.md` (the accepted addresses, a usage line), the `add` help text.

## Deviations from the plan

- The slug form is a variant of `SourceSpec` (`WagoPage`) rather than a separate parse result, so the
  check of what is missing, the list import's source reading and the router take it as they take any spec;
  `canonical_spec` is the one place it becomes `wago:<id>`.
- The core unit test `a_bad_spec_names_the_accepted_shape` refused `addons.wago.io/addons/details` with the
  slug hint of 092; that address now parses, so the test refuses other Wago addresses with the new hint.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 150, `tests/wago_source.rs` 11, core
  unit 234, the other targets unchanged). `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
  warnings`, `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green.
  The slug list is the fixture's; no Wago endpoint was asked.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
