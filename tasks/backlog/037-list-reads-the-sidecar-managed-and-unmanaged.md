---
status: idea
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# `list` reads the sidecar: managed (source, version) and unmanaged addons

What: `mrgl list --flavor <id>` loads the flavor's sidecar (004) and marks every scanned addon managed
(with its source and installed version) or unmanaged (with the TOC's source hints, as today), in the
summary line and in `--json`; a missing or corrupt sidecar is rebuilt silently, as D-003 says.
Why: 004 writes the sidecar but `list` still shows every addon as the scan sees it; the daily overview
needs the managed state before `update --all` (006) and the window (009) build on it.
Done when: a fixture flavor with one installed addon lists it as managed with source and version, the
rest unmanaged, and the JSON carries the same fields.
