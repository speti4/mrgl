# Fixtures

Synthetic test data only, written by hand or by a deterministic generator; a real addon zip, a real AddOns
folder or a `WTF` folder never enters the repo (CONTRIBUTING.md, "No game data, no Blizzard assets"). Every
test runs offline against what is here. `.gitattributes` keeps the files' own line endings, so a CRLF or a
BOM in a fixture survives every checkout.

- `flavors/` — a made-up game folder (task 002): `.build.info` at its root with a row per product, and one
  flavor folder per id, each with its `.flavor.info`. `_retail_` holds the TOC shapes the parser covers,
  one addon folder each: a base-only TOC with a comment and file lines (`PlainToc`), game-type and family
  suffixes (`ThreeWays`, `Midnight`, `Camelot`, `ClassicOnly`), legacy `_BCC` and hyphen files next to the
  base (`OldSuffixes`), an uppercase suffix (`LoudCase`), `## Interface` lists with and without spaces
  (`ThreeWays`, `Spaced`), a BOM with CRLF (`BomCrlf`), color escapes (`Colored`), the dependency keys
  (`Deps`), the three source hints with a `.curseclient` marker (`Hinted`), a repeated and a lowercase key
  with a localized title (`Repeated`), a `.toc` with a foreign stem (`ForeignStem`) and a folder with no TOC
  (`NoToc`). `_classic_` and `_classic_era_` show the game-type pick, `_forever_` has no `Interface` folder
  yet (the client before launch), `_unknown_` carries an id the manager does not know, and `_no_info_` has
  no `.flavor.info` at all.
- `release-json/*.json` — `release.json` files in the shape the BigWigs packager writes, with `forever` next
  to other flavors and a release without metadata (task 003).
- `zips/*.zip` — small archives with one and with several addon folders, and one with a path that tries to
  escape the staging folder (task 004). `.gitignore` allows zips here and nowhere else.
