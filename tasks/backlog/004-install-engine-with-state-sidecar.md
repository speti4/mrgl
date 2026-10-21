---
status: ready
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Install engine: zip to staging, addon-folder detection, atomic swap, rollback, the state sidecar; `update` and `remove`

What: unpack a downloaded zip into a staging folder (rejecting entries that escape it), find the addon
folders in it (several per zip, each with its TOC), swap them into `Interface\AddOns` one folder at a time
with a rollback when a swap fails, and record the install in the state sidecar of D-003 (source, version, file
list with hashes, time). `mrgl update <name>` and `mrgl remove <name>` build on it; a missing or corrupt
sidecar is rebuilt from the scan.
Why: the write path of the manager, and the one place that touches the user's folder, so it has to be
atomic and bounded.
Done when: the fixture zips (one folder, several folders, an escaping path) install, update and remove
correctly in a temporary AddOns folder, a failed swap leaves the folder as it was, and the sidecar round-trips.
