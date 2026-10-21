---
size: S
origin: done/029-dependency-check-from-toc-and-nolib.md
created: 2026-10-21
closed: 2026-10-21
reason: folded into 045, the pre-launch manual round, as one check on the real client
reopen-when: the client's behavior disagrees with the 029 rule; then the rule change is a task of its own
milestone: M1 Core + CLI
---

# Does an embedded library satisfy a required dependency in the client?

What: check on the real client whether a `## Dependencies` name is satisfied by a library folder
embedded inside the addon (the rule 029 implements) or only by an addon of that name under `AddOns`;
if the client wants the addon, the embedded case becomes missing and the 029 rule and its docs change.
Why: the check is only useful if it agrees with the client; a wrong rule either hides a real miss or
nags about one that is not there.
Done when: the client's behavior is written down with the build it was checked on, and the 029 rule
matches it, with its tests.
