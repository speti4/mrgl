---
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 066 — The GitHub source looks past the newest release for the flavor's asset and for a named version

## Context

The resolve takes the newest release that has an asset for the flavor, not only the newest release; and
`import --exact` (057) walks further pages for a named version instead of stopping where 042 stops. Grid2's
newest release is on the classic line with no Forever zip, so it resolves to nothing on Forever and stays
out of the starter list (028); an older listed version beyond page one reads as not offered.
Done when: a fixture repository whose newest release lacks a Forever asset resolves the one before it, and a
named version on page two installs; both against the stub.

Decided 2026-10-22: third of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the GitHub source's pick rule (the newest release with an asset for the flavor) and
  the page walk for a named version.

## Not in this change

- The other sources (056); a bound change on the page walk beyond what a named version needs; the starter
  list's entries (028, rechecked after the launch in 008).

## Steps

1. [ ] `sources/github.rs`: the resolve walks the cached release list from the newest release down to the
       first one with a pick for the flavor (drafts and the channel rule unchanged); the detail of
       `NoAssetForFlavor` says how many releases were looked at
2. [ ] `resolve_version` walks further pages for a named version the first pages lack, within the page bound
       of 042, and only then answers `VersionNotOffered`
3. [ ] Tests against the stub: a fixture repository whose newest release has no Forever asset resolves the
       one before it; a named version on page two installs through `import --exact`
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; third of the chain 064 → 065 → 066 → 067

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
