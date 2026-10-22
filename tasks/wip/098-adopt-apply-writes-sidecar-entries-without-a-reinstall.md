---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
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

1. [ ] `adopt.rs`: the folder group of an addon by the three rules (the source's folders: WoWInterface
       `UIDir`; else the folders whose TOCs carry the same source id; else the folder alone), the entry with
       the spec, the TOC version or `unknown`, the group and the file hashes, no download
2. [ ] `--apply` writes the entries through the state sidecar; a second `--apply` changes nothing; a
       CurseForge-only addon is skipped with a line
3. [ ] The `.curseclient` guard: `--apply` refuses without `--alongside-curseforge` and warns with it; `add`,
       `import` and `update` warn in a marked folder
4. [ ] Tests: `--apply` on the fixture, then `update` against the stubs installs cleanly over an adopted
       entry; the guard with and without the flag; the second apply; the warnings of the three commands

## Log

- 2026-10-22 — promoted; runs after 097 on the same branch

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
