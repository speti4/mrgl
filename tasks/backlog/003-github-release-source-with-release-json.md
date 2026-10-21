---
status: ready
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# GitHub release source: `release.json`, flavor-correct asset pick, ETag cache; CLI `add github:<owner>/<repo>`

What: fetch a repository's releases over the GitHub REST API (an optional token), read the release's
`release.json` as the BigWigs packager writes it, pick the asset whose metadata names the flavor (flavor key
and interface number), fall back to asset-name heuristics (`-forever`, `-classic`, `-mainline`) flagged as a
guess, and cache the release list with `ETag`. `mrgl add github:<owner>/<repo> --flavor <id> [--dry-run]`
resolves and reports; the install itself is task 004. Network access sits behind a trait with a local stub.
Why: the Forever addon list publishes on GitHub with flavor metadata; this source covers the Forever list.
Done when: `add github:Questie/Questie --flavor forever --dry-run` names the `forever` asset, the fixture
`release.json` files (several flavors, no metadata, no release.json) resolve as documented, and no test
touches the network.
