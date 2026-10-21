## What and why

<!-- What changes, and why. Link the issue (Fixes #N) or the task file (tasks/wip/NNN-….md). -->

## Follow-ups

<!-- Work this change found but did not do, one line each; the maintainer files them as tasks. "none" is fine. -->

## Checklist

- [ ] `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked` pass
- [ ] The `rust-version` check passes (CONTRIBUTING, 'Checks before a commit')
- [ ] `python -I scripts/changelog.py --check` passes; a closed task brings its `CHANGELOG.md` line
- [ ] When `Cargo.toml`, `Cargo.lock` or `.github/` change: `cargo deny check` and `uvx zizmor .github` pass,
      or neither changed
- [ ] No game data anywhere: fixtures, tests, screenshots and the description use made-up addons only; no
      Blizzard audio or artwork
- [ ] A new crate has its one-line reason in `Cargo.toml`, so the shipped artifact stays one portable zip
      (CONTRIBUTING, 'Dependencies: nothing to install'), or there is none
- [ ] Every new human-readable message goes through `mrgl_core::voice`; tests assert the plain line
- [ ] The commits pass the hook (`git config core.hooksPath .githooks`)
