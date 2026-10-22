---
status: idea
size: L
origin: session 2026-10-22
created: 2026-10-22
milestone: M5 Trust
---

# Test hardening toward full line and branch coverage

What: the tests reach 100% line and branch coverage of the three crates as `cargo +nightly llvm-cov
--workspace --branch` measures it, every uncovered line either tested or excluded with a reason in the
code (the Tauri start-up of `mrgl-app`'s `main.rs` and `lib.rs`, the real Windows Credential Manager behind
the fake store, and the like, listed in `docs/development.md`), the measurement documented there with the
tool set-up, and the exclusion list the only thing below 100%. Measured 2026-10-22: 89.6% lines, 79.6%
branches overall (core 92.1% / 79.6%, cli 88.7% / 84.4%, app 68.2% / 67.6%; `commands.rs` 26.9% lines,
`keys.rs` 59.1% branches, `discover.rs` 64.3%, `backup.rs` 69.4%).
Why: the untested branches are where a real folder meets a case no fixture has; M5 is the hardening milestone.
Done when: the report shows 100% outside the documented exclusions, and a CI job prints the numbers on every
pull request so a drop is seen before the merge.
