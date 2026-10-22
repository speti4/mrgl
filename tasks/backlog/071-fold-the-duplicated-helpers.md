---
status: idea
size: S
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
milestone: M1 Core + CLI
changelog: housekeeping
---

# Fold the duplicates: one `WTF` walker, one size formatter, no dead error variant

What: `sets` and `savedvars` share one account, realm and character walker and one set of `WTF` constants;
the CLI's `human_bytes` and the core's `size_text` become one; `Error::NotImplemented` goes now that no stub
calls it; the stand-in test files stop importing what Linux does not use (the `non-windows` job's warnings).
Why: each was written twice in parallel branches and will drift apart.
Done when: one walker with both callers, one formatter, the variant gone, `cargo check` on Linux without
warnings, and every existing test unchanged.
