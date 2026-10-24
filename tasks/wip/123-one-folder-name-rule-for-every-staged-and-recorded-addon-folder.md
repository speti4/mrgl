---
size: M
origin: done/121-a-hardening-review-of-the-code-that-touches-the-disk-the-network-and-the-keys.md
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
---

# 123 — One folder-name rule for every staged and recorded addon folder

## Context

Finding H1 of the hardening review (121): a `.toc` at a zip's root whose stem is `.`, `..` or blank (`..toc`,
`...toc`, ` .toc`) becomes the addon's folder name unchecked (`install.rs`, `root_toc_name`), the swap then
joins it to `Interface\AddOns`, and Windows resolves `AddOns\.` and `AddOns\ ` to `AddOns` and `AddOns\..` to
`Interface`: the install replaces the whole AddOns folder with the zip's files (reproduced), and with the mrgl
home on another drive than the game it copies `Interface` aside, removes it and writes the zip into its place.
A later `remove` of the recorded entry removes the same path. Every install path reaches it: a zip in hand,
every source, `import`, `update`, `rollback` and the window's update.
Why: a crafted or merely odd zip must never write outside `Interface\AddOns\<folder>`; M5 is the trust
milestone.
Done when: one folder-name rule (not empty, not `.` or `..`, no separator, `:` or control character, no
trailing dot or space, not a reserved device name) is applied to every staged folder before anything is
unpacked into place (a root TOC whose stem fails it falls back to the zip's stem, else the zip is refused with
a line) and to every folder read from the sidecar before a commit, a remove or a link check; the three
near-duplicates of the rule (`adopt.rs`, `list_file.rs`, `sets.rs`) call that one function; regression tests
install the three zips and assert that `AddOns` and `Interface` are untouched and the sidecar records nothing.

Decided 2026-10-24: filed and started at once from the review's list; the fix stays in the core (the CLI and
the window only carry the refusal line through the voice).

## Docs that move

- `docs/development.md`: the install engine's folder-name rule (where the zip entry rules are described), the
  fixtures paragraph for the new zips
- `crates/mrgl-core/tests/fixtures/README.md`: the three zips

## Not in this change

- The other findings of 121 (124 bundles the low ones)
- A change to the zip entry name rules beyond what the folder rule needs (L5 and L6 are 124's)

## Steps

1. [ ] `crates/mrgl-core/src/install.rs` (or a small module the three callers share): the one rule as a
       function with a unit test per case; `root_toc_name` and `find_addons` apply it to every staged folder
       in `Engine::stage`, before the unpack into place: a root TOC stem that fails it falls back to the zip's
       stem, a folder that still fails refuses the zip with a line through the voice
2. [ ] `Engine::commit`, `remove`, `rollback` and `refuse_links` apply it to a folder read from the sidecar
       (a sidecar written by an older build could carry one), refusing with a line instead of joining the path
3. [ ] `adopt.rs`, `list_file.rs` and `sets.rs` call the one function instead of their own checks
4. [ ] Fixtures: zips with `..toc`, `...toc` and ` .toc` at the root (made by `zips/make-zips.py`), and the
       end-to-end tests: `add` of each refuses or falls back, `AddOns` and `Interface` untouched, the sidecar
       unchanged, exit code 1 with the line; a sidecar entry with folder `..` is refused by `remove`
5. [ ] The docs

## Log

- 2026-10-24 — filed and promoted

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
