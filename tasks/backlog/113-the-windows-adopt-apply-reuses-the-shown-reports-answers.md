---
status: idea
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's adopt apply reuses the shown report's answers

What: the toolbox's adopt apply writes the sidecar entries from the report the card shows instead of asking
every source again, and the report after the write is built from the written sidecar and the same answers;
a report older than the sources' cache rule is refreshed first.
Why: 109 asks every source twice more after the report the user saw, which spends requests against GitHub's
unauthenticated limit for nothing new.
Done when: a window test over the adopt fixture counts the stub's requests: one round for the report, none
for the apply and the report after it, and the written sidecar equals the one of 109's test.
