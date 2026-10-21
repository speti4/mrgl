---
status: idea
size: S
origin: done/020-game-root-auto-discovery.md
created: 2026-10-21
milestone: M2 Window MVP
---

# A second registry source for the game root: the Uninstall entry, and expanded values

What: the discovery of 020 also reads the Uninstall entry's `InstallLocation`
(`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft`, and the same under `HKCU`)
after the launcher's key, for an install the launcher's key does not register; and a `REG_EXPAND_SZ` value
from either key is expanded (`ExpandEnvironmentStringsW`, the `Win32_System_Environment` feature) instead of
read as written. The stub of 020 covers both in the tests.
Why: an install moved or restored by hand carries the Uninstall entry but not the launcher's key, and
then `mrgl list` asks for a flag it should not need.
Done when: the stub's Uninstall value finds the root when the launcher's value is absent, the origin says
so, and an expanded value resolves to the same root as its literal.
