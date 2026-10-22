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
  (`Deps`), the three source hints (`Hinted`), a repeated and a lowercase key
  with a localized title (`Repeated`), a `.toc` with a foreign stem (`ForeignStem`) and a folder with no TOC
  (`NoToc`); its `.curseclient` marker is the CurseForge app's, with a made-up id. `_classic_` and
  `_classic_era_` show the game-type pick, `_forever_` has no `Interface` folder
  yet (the client before launch), `_unknown_` carries an id the manager does not know, and `_no_info_` has
  no `.flavor.info` at all.
- `releases/` — the answers of the GitHub source (task 003), laid out as the folder stub reads them: the
  URL's host and path under this folder (`api.github.com/repos/<owner>/<repo>/releases.json` is the release
  list, `github.com/<owner>/<repo>/releases/download/<tag>/release.json` the packager's metadata of one
  release, `api.github.com/rate_limit.json` the rate limit `mrgl doctor` reports). `Questie/Questie`
  carries a draft above a stable release whose `release.json` names `mainline`, `classic`, `mists` and
  `forever`, with `nolib` twins; `Example/NoMetadata` has a `release.json` in the old
  shape without flavor metadata; `Example/NoReleaseJson` has zips only (`-forever`, `-cata` and the unsuffixed
  retail one); `Example/Empty` has no release; `Example/Prerelease` has a draft, a prerelease and a stable
  release; `Example/BetaOnly` has a prerelease alone; `Example/MainlineOnly` names retail only. The asset
  names are made up; one zip sits behind them, `Questie-v1.2.3-forever.zip` under
  `github.com/Questie/Questie/releases/download/v1.2.3/` (task 040: one `Questie` folder whose TOC says
  `v1.2.3` while the pick says `1.2.3`), written by `zips/make-zips.py`; every other asset is a 404, the
  fetch test serves a text file from a temporary folder, and the import test of task 015 copies this tree
  into a temporary folder with a fixture zip of `zips/` behind the Forever asset of `Questie/Questie`; that
  asset lists no `size`, so the size check of task 046 lets any zip behind it through.
  `Example/Nightly` is paged (task 042): `releases.json` holds 30 nightly prereleases and, in
  `releases.json.headers`, the `Link` header to `releases.page-2.json`, whose first release is the stable
  `v1.0.0` with a `release.json` that names `forever`; the folder stub serves a `page` above 1 from
  `<answer>.page-<n>.json` and adds the lines of a `<answer>.headers` file to the answer's headers.
- `zips/*.zip` — small archives for the install engine (task 004), written by `zips/make-zips.py` (fixed
  timestamps and entry order, so a rerun changes no byte; `python -I` it after editing a shape): one addon
  folder in three versions (`one-folder-1.0.0` with `old.lua`, `1.1.0` with `new.lua` instead, `1.2.0`),
  several folders in one archive with a root `README.md` and a `Docs/` folder the install ignores
  (`several-folders-2.0.0`; `2.1.0` drops the `_Data` folder), a TOC at the zip's root with a `_Vanilla`
  twin (`root-toc-3.0.0`), a wrapper folder around the addon folder (`wrapped-4.0.0`), a TOC without a
  `## Version` (`no-version`), a TOC whose `## Interface` (110207) has another major version than every
  fixture build, for the interface warning of task 015 (`stale-interface-1.0.0`), a TOC one interface
  number behind the Forever fixture build (16000 under 1.60.1), out of date for task 082
  (`behind-interface-1.0.0`), a path that tries to
  escape the staging folder (`escaping-path`), an addon that requires `LibEmbedded`, carried under its
  `Libs/`, and `LibMissing`, carried nowhere (`needs-library-1.0.0`, task 029), and a zip with no addon at
  all (`no-addon`). `.gitignore` allows zips here and nowhere else.
- `dependencies/` — a third made-up game folder for the dependency check (task 029): a `.build.info` row
  and a `_forever_` flavor whose `Needy` requires `LibMet` (an addon folder of its own: met), `LibEmbedded`
  (a folder under `Needy/Libs/` only: embedded, and missing, since the client does not count it, task 076)
  and `LibMissing` (nowhere: missing), with `libmet` repeated in lower case and an optional `LibOptional`,
  embedded under `Needy/Libs/` too, that is never reported; `NolibBuild` carries a `Libs/LibEmbedded`
  folder that the check names as embedded until a test's sidecar marks the addon a `nolib` build.
- `interface/` — a fourth made-up game folder for the interface states (task 076): a `.build.info` row for
  `wow` at `12.1.5.70448` (interface 120105) and a `_retail_` flavor with `Current` (`120105, 50504`),
  `Behind` (`120100`, one interface number behind the build: out of date) and `Elsewhere` (`110207`, another
  expansion: an interface mismatch). The window's flavor list test reads it in place (task 081).
- `client-exe/` — a second made-up game folder for the build version fallback (task 019): its `.build.info`
  has a row for `wow` only, and the flavor folders hold client exe stubs written by `make-stubs.py` next to
  them (deterministic; `python -I crates/mrgl-core/tests/fixtures/client-exe/make-stubs.py` rewrites the
  same bytes). Each stub is a minimal PE file, one `.rsrc` section and no code, never a game executable:
  `_classic_era_\WowClassic.exe` has a version resource (`1.15.7.70100`, the build number above 65535 so
  only the `FileVersion` string holds it whole), `_retail_\Wow.exe` has one the `.build.info` row wins
  over, `_classic_\WowClassic.exe` has a resource section without a version resource, `_forever_` has no
  exe; `stubs\pe32.exe` (a 32-bit image) and `stubs\fixed-only.exe` (a version resource with the fixed
  info and no string table) serve the parser's unit tests.
- `wtf/WTF/` — a hand-written settings tree in the shape the client writes, for the backup and restore of
  task 013: `Config.wtf` at the root, an account folder (`Account/MRGLFIXTURE/`) with its `config-cache.wtf`,
  `bindings-cache.wtf` and account-wide `SavedVariables/` (a `.lua` and its `.lua.bak`), and a realm with a
  character (`Fixture Realm/Mrgltest/`) with its `AddOns.txt`, `layout-local.txt` and character
  `SavedVariables/`. Every name and value is made up, a few hundred bytes in all; the tests copy the tree
  into a temporary flavor folder and never write here; the SavedVariables tests of task 025 add their
  made-up files and an `Interface\AddOns` folder to such a copy. `.gitignore` allows a `WTF` folder here and
  nowhere else.
- `addon-sets/WTF/` — a second hand-written settings tree, for the addon sets of task 014: `Config.wtf`, an
  account (`Account/MRGLSETS/`) with its `config-cache.wtf` and an account-wide `SavedVariables/` (which is
  no realm), and two characters on two realms, each with only an `AddOns.txt`: `Fixture Realm/Mrgltest/`
  with CRLF line endings, `Second Realm/Grrlwave/` with LF endings, a line the client never wrote and no
  last line break. The two disagree on two addons (`BossTimers`, `QuestArrows`), which a save leaves out.
  Every name is made up; the tests copy the tree into a temporary flavor folder and never write here.
- `dev/` — a development folder for the junction links of task 005: `OneFolder` (a TOC with `## Version:
  9.9.9-dev`, the same folder name as the `one-folder` zips, so an install over the linked addon can be
  refused) and `DevAddon` (a TOC without a version) are addons, `Media` holds a text file and no TOC. The
  tests copy the folder into a temporary one and make their junctions there, never here.
