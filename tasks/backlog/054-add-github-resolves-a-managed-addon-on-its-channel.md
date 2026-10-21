---
status: idea
size: S
origin: done/021-pin-ignore-and-prerelease-channel.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# `add github:` resolves a managed addon on its own channel

What: when `add github:<owner>/<repo>` names a source the sidecar already manages, the resolve takes
that entry's channel (021, `Target::with_channel`) instead of the stable default, so a reinstall of an
addon on the prerelease channel picks the prerelease.
Why: today `add github:` always resolves stable, which silently drops an addon off its channel on a
reinstall.
Done when: a fixture sidecar entry on the prerelease channel reinstalls the prerelease pick through
`add github:`, and an unmanaged source still resolves stable.
