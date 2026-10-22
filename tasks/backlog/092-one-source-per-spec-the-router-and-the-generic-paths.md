---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# One source per spec: the router, the spec forms and the generic paths

What: `SourceSpec` gains `wago:<id>`, `wowi:<id>` and `curse:<id>` (and the addon page URLs an id can be read
from), a `Sources` value implements `Source` by dispatching on the spec and replaces every direct `GitHub`
construction in the command line and the window; the Forever build signal moves onto the trait (default
`unknown`); the list import and the update check ask any configured source, and an entry of a source this
build lacks reads "source not configured", not "not importable"; the test stub maps a query string to a
file and `Http` gains `post`. The first task of M4: the source tasks 094 to 096 plug in here.
Why: every caller is GitHub-only today, so each new source would touch the same eight places.
Done when: every existing test passes unchanged, `add wago:x` without the Wago source names what is missing,
a `wowi:` list entry reads "source not configured", and the stub serves two flavors of one URL from two files.
