---
status: idea
size: S
origin: done/010-update-all-and-update-one-in-the-window.md
created: 2026-10-22
milestone: M2 Window MVP
---

# Window and exe polish after the first release: the checks after an update, the title bar theme, the CLI exe's version

What: after an update the window runs the dependency and interface checks the CLI's install summary reports
and shows their warnings; the Tauri window's theme follows the preference (`auto`, `light`, `dark`) so the
native title bar matches the page instead of the system theme; and `mrgl.exe` carries the workspace version
in its file properties through the CLI's build script, the way it carries the icon (012).
Why: the window's update is silent about a dependency that went missing (010), a light page under a dark
title bar looks broken on a dark system (012), and the CLI exe's properties show no version while the app
exe's do (012).
Done when: a flavor list test sees the dependency warning after a fixture update, the window's title bar is
light in the light theme on a dark system, and the exe's version resource reads the `Cargo.toml` version.
