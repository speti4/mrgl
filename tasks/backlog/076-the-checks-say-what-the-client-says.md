---
status: idea
size: S
origin: wip/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The dependency and interface checks say what the client says: an embedded-only dependency is missing, a lower interface number is out of date

What: a required dependency that exists only as a library embedded in the addon's own folder is reported as
missing, with the reason that the client does not count an embedded folder (029's rule changes; `nolib` is
unaffected); and `scan` and `list` mark a TOC whose highest interface number of the flavor's expansion is
below the installed build as `out of date`, in the client's words, apart from the other-expansion mismatch.
Why: on the real client a probe addon with an embedded-only required dependency showed `Dependency missing`
and did not load (the reopen condition of 052), and after a client update every addon with the previous
interface number showed `Out of date` while `scan` said nothing.
Done when: the dependency fixture with an embedded-only required library reports it missing with the reason,
the embedded case of an optional dependency is unchanged, and a fixture TOC one interface number behind the
build is marked `out of date` while a current one and an other-expansion one keep their states.
