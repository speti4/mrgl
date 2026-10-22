---
status: waiting
waits-for: the CurseForge API key of task 007, for the manual round and the game version type ids
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# The CurseForge source with the user's key

What: `curse:<id>` resolves through the CurseForge Core API with the user's key (`MRGL_CURSEFORGE_KEY`, 093):
the mod and its files per game version type (Forever included), the release types mapped onto the channels,
the download carrying the key too; a mod with distribution off fails with a line naming its page, and a zip
in hand installs with the source recorded; no API answer is cached, as the terms require, so the Forever
signal in `list` comes from what the sidecar recorded at install, else `unknown`. The code and the stubs land
before the key; the manual round and the type ids wait for it. Decided 2026-10-22: only the user's own key
(D-002, 093); never a cache of CurseForge answers. Why: the retail and classic folders are mostly CurseForge.
Done when: `add curse:<id>` installs from the stub with a key and names the key without one, a
distribution-off mod names its page and installs from a zip, a test asserts the key reaches the two CurseForge
hosts only and never a cache or the sidecar, a list entry imports, and a fixture classifies the Forever build.
