---
status: idea
size: M
origin: done/121-a-hardening-review-of-the-code-that-touches-the-disk-the-network-and-the-keys.md
created: 2026-10-24
milestone: M5 Trust
---

# The low findings of the hardening review in one sweep

What: the ten low findings of 121 (L1 to L10 under its Verification) fixed on one branch, each with its test:
`https_only` on the agent; a body bound per request kind; an owner or repository of dots only refused; control
characters replaced in every human line the CLI prints; a bound on a zip's entry count; a component ending in
a dot or a space refused; a collision-free cache folder name with reserved device names escaped; secret header
values hidden in `Request`'s `Debug`, the doctor's token as a `keys::Key` through `may_send`; the running-client
check repeated after a long `WTF` write with a warning; the folder key store built for the tests only or warned
about.
Why: each is contained today; together they close the edges the review named, before the signed release of M5.
Done when: every item has its test and a line in the task file, the docs name the new bounds and rules, and the
coverage stays at 100%.
