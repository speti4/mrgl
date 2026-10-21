---
status: idea
size: S
origin: done/024-doctor-diagnostic-report.md
created: 2026-10-21
milestone: M2 Window MVP
---

# A Windows version line in the doctor report

What: the doctor report (024) gains the Windows build and display version from the registry, read the
same way as the WebView2 version, so the bug report template's OS field can come from the report.
Why: the OS version is the second thing asked in every bug report, and the user guesses it today.
Done when: the line is in the plain and the JSON report, stubbed in the tests, and documented with the
other lines.
