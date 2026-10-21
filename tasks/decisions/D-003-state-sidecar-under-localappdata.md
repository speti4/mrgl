---
status: decided
date: 2026-10-21
cost: medium
sources: [wip/001-project-scaffolding.md, D-002-rust-tauri2-portable-zip-cf-aware.md]
---

# D-003 — The state is a sidecar under `%LOCALAPPDATA%\mrgl\` per flavor; the AddOns folder is the truth

## Context

A manager needs to remember where each addon came from and which version is installed, or it cannot update.
The CurseForge app keeps that in its own database and marks folders with `.curseclient`; a manager that
writes bookkeeping files into the game folders would leave litter the user did not ask for, and a manager
whose database is the truth breaks when the user deletes a folder by hand.

## Decision

Per flavor, one `state.json` under `%LOCALAPPDATA%\mrgl\<flavor-id>\`, where the flavor id is the value of the
game folder's `.flavor.info` (`wow`, `wow_classic`, `wow_classic_era`, `wowf`, …). It records, per installed
addon, the source, the version, the file list with hashes and the install time. The `Interface\AddOns`
folder is the truth and the state is a cache: a missing, corrupt or stale state is rebuilt from a scan and
never an error the user has to fix; an addon the scan finds without a state entry is listed as "unmanaged"
with its TOC's source hints. Nothing is written into the game folder except the addon folders themselves,
and the `WTF` files of a backup restore, an addon set or a SavedVariables cleanup the user asked for (M3),
never while the client runs. Decided 2026-10-21.
Amended 2026-10-21: the last sentence names the two toolbox writes, as D-002 does.
Amended 2026-10-21: a third toolbox write, the SavedVariables cleanup of orphan files into a backup zip
(task 025), joins the two; every `WTF` write refuses while the client runs (task 023).

## Not chosen

- A marker file per addon folder inside `Interface\AddOns`: litter in the user's folder, and still no place
  for the hashes.
- A single database for all flavors: one corrupt file would take every flavor down, and the folder name is
  the flavor id anyway.
- The state as the truth: deleting a folder by hand must not strand the manager.
- Next to the game folder (`<flavor>\.mrgl\`): the game folder is not ours to write bookkeeping into, and
  the folder may not be writable.
