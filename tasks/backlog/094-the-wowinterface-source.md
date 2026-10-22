---
status: idea
size: M
origin: session 2026-10-22
created: 2026-10-22
milestone: M4 Sources
---

# The WoWInterface source

What: `wowi:<id>` resolves through the MMOUI API (`filedetails/<id>.json`, no key), one file per project; the
flavor comes from the project's compatibility list (a metadata pick for retail and the classic flavors, a
guess without one and for Forever), a pending file is reported and not installed, the download is checked
against the API's MD5, the answer is cached under `cache\wowi\` (the API allows an hour), and the Forever
signal stays `unknown` until WoWInterface lists a Forever compatibility. Runs after 092 and 093, before 095.
Decided 2026-10-22: the first new source, since it works without a key; the interface check after install
is the safety net of a Forever guess. Why: the one keyless source after GitHub, and the proof of the router.
Done when: `add wowi:<id>` installs from the stub, a compatibility list picks by metadata and none is a
guess, a pending file is reported, a wrong MD5 is refused before the zip opens, a list entry imports, and a
fixture answer classifies the Forever build as `unknown`.
