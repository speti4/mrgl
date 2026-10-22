---
status: idea
size: M
origin: done/097-adopt-the-report-of-the-sources-of-an-addons-folder.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's adopt view: the report and `--apply` in the toolbox

What: the toolbox shows the adopt report of the chosen flavor (097: per unmanaged addon its sources and
whether each offers a build, the summary counts, the `.curseclient` marker) and applies it with one click
(098: the same sidecar write, the `--alongside-curseforge` confirmation as a dialog, the warning after),
over the core's lines and counts, with the same Rust and Node tests as the toolbox's other views.
Why: adopt is the road to CurseForge independence, and the window is where most users will take it.
Done when: a window test over the adopt fixture shows the report's rows and counts, the apply writes the
sidecar the command line's test accepts, and the marker guard holds with and without the confirmation.
