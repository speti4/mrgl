---
status: idea
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
milestone: M2 Window MVP
---

# The install engine warns when the client runs

What: the install engine's commit (004) calls the running-client check of 023 at the place marked for it
before the first move, and turns a running client into a warning in the install result and the voice's
warning line (the client reads addons at `/reload`); never a refusal, unlike the `WTF` writes.
Why: both halves exist on `main`; the call site was left out so the two branches could merge apart.
Done when: an install with the stand-in client of 023 running reports the warning and still installs, and
without it reports nothing.
