# Fixtures

Synthetic test data only, written by hand or by a deterministic generator; a real addon zip, a real AddOns
folder or a `WTF` folder never enters the repo (CONTRIBUTING.md, "No game data, no Blizzard assets"). Every
test runs offline against what is here.

Planned layout (the tasks that add the fixtures name the files):

- `addons/<Name>/` — generated addon folders: multi-TOC suffixes, `## Interface` lists, the
  `X-Curse-Project-ID` / `X-Wago-ID` / `X-WoWI-ID` headers, title, version and dependency fields (task 002).
- `release-json/*.json` — `release.json` files in the shape the BigWigs packager writes, with `forever` next
  to other flavors and a release without metadata (task 003).
- `zips/*.zip` — small archives with one and with several addon folders, and one with a path that tries to
  escape the staging folder (task 004). `.gitignore` allows zips here and nowhere else.
