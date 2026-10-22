---
status: idea
size: S
origin: wip/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The CLI's output survives a closed pipe

What: every command's stdout and stderr writes go through one helper that treats a closed pipe (`BrokenPipe`,
os error 232 on Windows) as the end of the run: the command exits quietly with 0 instead of panicking.
Why: `mrgl list --json | head -c 200` on the real install panicked with `failed printing to stdout`, which
breaks every shell pipeline that reads only the start of the output.
Done when: a test pipes `list --json` into a reader that closes early and sees exit 0 with no panic text, and
the plain and JSON outputs are unchanged otherwise.
