# mrgl — development

How the repo is laid out, how CI and releases work, and what the helper scripts do. Build prerequisites, the
checks before a commit and the project rules are in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Layout

- `crates/mrgl-core` — the library: flavor detection (`flavor`), the AddOns scan and TOC parsing (`scan`,
  `toc`; `scan` also holds `CLIENT_ADDON_PREFIX` and `is_client_addon`, the one `Blizzard_` rule that the
  dependency check and the SavedVariables share, task 080), the sources (`sources`: the `Source` trait, the `Pick` it hands to the install engine, the `Http`
  trait with the real `ureq` client and the folder-backed stub; `sources::github` reads the releases API,
  the packager's `release.json` and the asset names, with the release list cached by `ETag` under the mrgl
  home folder, task 003; the pick carries `notes`, the release body's first line with text once its
  Markdown markers are stripped, cut at `NOTES_MAX_CHARS` (120) with an ellipsis by `sources::notes_line`,
  and `page`, the release's page, both left out when absent, task 030; the `Target` carries the addon's
  channel, and on `prerelease` the newest release that is not a draft is current, a prerelease or not,
  task 021; the resolve walks the channel's releases from the current one down to the first with an asset
  for the flavor, the newest release that has one, and says so in the pick's reason when it passed newer
  ones, and the no-asset error names the newest release and how many releases were looked at, task 066;
  the spec forms, `SourceSpec`: `github:<owner>/<repo>`, `wago:<id>`, `wowi:<id>` and `curse:<id>`, and the
  page addresses an id is read from without a request, `github.com/<owner>/<repo>`,
  `www.wowinterface.com/downloads/info<id>…` and `www.curseforge.com/projects/<id>`, while a CurseForge
  addon page names a slug and is refused with where the id is, task 092; a Wago page,
  `addons.wago.io/addons/<slug>`, is `SourceSpec::WagoPage`, which the router turns into `wago:<id>` through
  the source's `canonical_spec` before `add` and `import` resolve, install or record anything, task 105;
  the router,
  `sources::Sources` in `sources/router.rs`, one slot per `SourceKind`, the one value the command line and
  the window build per run, `Sources::from_env`, and the window's offline view over a client that fails
  every request, `Sources::cached`: it hands each spec to its source and answers
  `Error::SourceNotConfigured`, `source not configured: <spec> (<what is missing>)`, for a source this build
  does not have or cannot ask, which `Source::missing` tells before any request, so `add` fails, the list
  import reports `source-not-configured` and the update check leaves the entry not checked; the Forever
  signal from the cache is the trait's `cached_forever_build`, `unknown` wherever a source does not name
  Forever, and the request budget of a spec is `request_budget_for`, task 092; `sources::wowinterface`, the
  WoWInterface source of task 094, see [Fixtures](#fixtures): the project's one file from the MMOUI API without
  a key, the flavor from the compatibility list of the all-projects file list, read once a day, a pending
  file reported, the download checked against the API's MD5 before it is written (`Pick::md5`,
  `sources::check_md5`, on RustCrypto's `md-5`), the Forever signal `yes` for a Forever compatibility and
  `unknown` otherwise; `sources::wago`, the Wago source of task 095, see [Fixtures](#fixtures): the
  external API with the user's key, the game version per flavor, the three recent releases onto the
  channels, the rate limit its answers name, the Forever signal from a release for `forever` or the keyless
  slug list, and the update check's bulk lookup of task 106 behind the trait's `prefetch` and `holds`;
  `sources::curseforge`, the CurseForge source of task 096, see [Fixtures](#fixtures): the Core API with the
  user's key, the project and its files per game version type, the release types onto the channels, no
  cache of any answer (the API's terms), the Forever signal recorded in the sidecar at install, and a
  project whose author allows no download outside CurseForge named with its page; adopt's question to a source, whether it
  publishes a build for a flavor, is the trait's `offers`, and the folders a source names without a download
  are its `folders`, task 097), the adopt report (`adopt`, task 097, see [Adopt](#adopt)), the install engine (`install`: a zip into
  a staging folder once every entry passed `zip_entry`, its addon folders swapped into `Interface\AddOns`
  one at a time with a revert, the previous version's zip kept; task 004; its commit and its removal ask
  the running-client check once before the first folder moves, and a running client is a warning on the
  result, never a refusal, task 038; its commit, `Engine::commit`, refuses to install over a linked addon
  or a junction it did not make before the first folder moves, so every path that stages and commits does,
  task 055, and the guard covers the staged folders and the previous version's folders the new one drops,
  so a dropped folder that is now a junction is refused instead of moved aside and deleted, task 067; every source-backed install takes one path, `Engine::install_from_source`, tasks 040 and 055,
  below; the rollback, `Engine::rollback`, sends the entry's previous
  version and its zip from the cache through the same stage and swap with the entry's source, pins the
  addon there and leaves both zips in the cache, so a second rollback goes forward again, task 022), the
  dev addon links (`link`, task 005: a directory junction under `Interface\AddOns` for every
  addon of a development folder, a mount-point reparse point made through the Windows API, recorded in the
  sidecar with the source `link:<development folder>`, the TOC's version and no file list; a folder that is
  itself an addon, a TOC named after it with or without a multi-TOC suffix, is linked as that one addon
  under its own name with the source `link:<addon folder>`, and that shape wins over its subfolders, which
  are its own, task 062, `link::shape_of` and `Linked::shape`; `unlink` removes exactly those junctions,
  never a real folder or a junction mrgl did not make; Windows-only), the state
  sidecar (`state`, D-003; an entry's optional `pinned` version, `ignored` flag and release `channel`,
  `stable` or `prerelease`, with the queries and setters the update check and the CLI call, task 021; absent
  fields read as unpinned, not ignored, stable, and a reinstall keeps them; the optional `forever_build`,
  the Forever signal a CurseForge pick named at install, task 096, since that source keeps no cache:
  `list` and the window read a source's cache first and this field when the cache says `unknown`,
  `sources::entry_forever_build`, and a reinstall from the same source that names none keeps it; every
  such field is additive, absent when unset, and the schema stays 1), the install engine's run
  folders under `staging\` and the sweep of their leftovers (`staging`, task 084, below), the toolbox (`backup`: the `WTF`
  backup into a dated zip, the listing and the restore that checks every entry through `zip_entry`, moves
  the current `WTF` aside and reverts on a failure, task 013), the addon sets (`sets`, task 014, below),
  the zip entry check
  both of them call (`zip_entry`, task 044: every entry readable, inside its folder and no link, checked
  before the first is written), the running-client check (`client`,
  task 023: a process found by its executable's path under the flavor folder, the guard every `WTF` write
  calls and the install engine's warning), the mrgl home folder with the remembered game root (`home`,
  D-003), the game root resolution (`discover`, task 020: the flag, `MRGL_ROOT`, the remembered root, then
  the launcher's registry key, the game's Uninstall entry (task 043) and the usual install folders, behind
  a lookup trait with a stub), the
  dependency check (`dependencies`, task 029: every required name of the TOC the flavor loads, `## Dependencies`,
  `## RequiredDeps` and the other `## Dep…` lines, ASCII case-insensitive, is *met* when an addon folder of
  that name is in the scan or the name is one of the client's own `Blizzard_` addons
  (`scan::is_client_addon`), and *missing*
  otherwise; a missing name that a folder inside the addon's own folder answers, at most three levels down,
  as a build without `nolib` carries it, is also listed as *embedded*, and stays missing (task 076: the
  client does not count a library embedded in the addon's own folder, so the addon shows `Dependency
  missing` and does not load), with the reason in its warning; an optional dependency is never reported; the
  sidecar entry's `nolib` flag, recorded from the pick at install, turns the embedded lookup off for a
  `nolib` build, whose warning says so instead, and an entry without it is unknown, embedded possible), the
  Forever notes (`forever`, task 011: the build signal `yes`, `no` or `unknown` that
  `sources::github::forever_build` derives from a release, and the curated `data/forever-notes.json`
  compiled in with `include_str!`, with the stale rule; from the cache alone,
  the GitHub source's `cached_forever_build` takes the release a Forever resolve on the addon's channel would take,
  `sources::github::forever_build_on`: the channel's releases newest first down to the first with a
  Forever asset, as the resolve walks them since task 066, `unknown` when the walk meets a release whose
  `release.json` is not cached, and the newest release's own signal when none has one, task 078), the
  last online check's kept answers (`last_check`, task 078, below), the diagnostic report (`doctor`, task 024: the
  facts a bug report needs, with the profile folder as `~`, see [The doctor](#the-doctor)), the API keys
  (`keys`, task 093, see [The API keys](#the-api-keys)), the addon
  list (`list_file`, task 015: the export from the sidecar and the import through the source and the engine,
  below), the interface check (`interface`, task 015, below) and the voice table (`voice`, D-005). Synthetic
  fixtures are under `tests/fixtures/`.
  The SavedVariables are `savedvars` (task 025): the walk of `WTF\Account\<account>\SavedVariables\` and
  `WTF\Account\<account>\<realm>\<character>\SavedVariables\`, a `<Addon>.lua` and its `.lua.bak` as one
  entry with their sizes, the largest first, and the orphan rule: a file is an orphan when no folder of its
  name, compared without regard to case, is an addon the scan finds under `Interface\AddOns` (a folder with a
  TOC named after it, or one the scan cannot read); a disabled addon keeps its folder and is never one, a
  `Blizzard_` file is the client's own (`scan::is_client_addon`), and with no AddOns folder at all nothing
  is marked. The cleanup,
  the third `WTF` write of D-002 and D-003, runs the 023 guard first, writes the orphans and their `.bak`
  files with the backup's zip writer, entry names relative to `WTF`, and removes each file only once its
  entry, read back from the finished zip, matches it byte for byte; `backup::restore_wtf` refuses that zip.
  `savedvars` and `sets` read `WTF` through one walk, `wtf::walk` (task 071), with the one set of its
  folder names (`wtf::ACCOUNT_DIR`, `wtf::SAVED_VARIABLES_DIR`; `WTF` itself is `backup::WTF_DIR`): every
  account folder, then every character folder of it, each level by name, the account's `SavedVariables`
  never a realm, a missing folder empty, and a link or a junction a warning that is never followed
  (`savedvars` lists it among its warnings, `sets` passes it over). The core's one size formatter is
  `zip_entry::size_text` (`512 B`, `34.2 KB`, `1.5 MB`, in steps of 1024, up to `PB`): the zip bound's
  messages, the CLI's listings and the doctor's report all write sizes with it (task 071).
- `crates/mrgl-cli` — the `mrgl` executable: `scan`, `list` and `root` (task 002; `list` reads the flavor's
  sidecar through the install engine and marks every addon managed or unmanaged with `state::classify`,
  task 037, while `scan` stays the raw folder view), `add github:<owner>/<repo>
  --flavor <flavor> [--dry-run]` (task 003: resolves and reports the pick, with the release notes' line and
  link under it when the release has notes, task 030; task 040 installs it through the engine or reports
  it installed already), `add
  <file.zip> [--as <spec>]` and `remove` (task 004; a running client is a warning line and `client_warning`
  in `--json`, task 038; `--as` records a spec as the zip's source instead of its path, task 096), `backup` with `--list`, `--restore` and `--yes` (task 013), `doctor [--flavor <flavor>]`
  (task 024), `link <folder>` and `unlink <folder>` (task 005; the folder is a development folder or one
  addon folder, task 062, and `link --json` names it as `shape`, `development-folder` or `addon-folder`),
  `export --flavor <flavor> [--to <file>]`
  and `import <file> --flavor <flavor> [--exact] [--dry-run]` (tasks 015 and 057), `pin <name> [version]`,
  `unpin`, `ignore`,
  `unignore` and `channel <name> <stable|prerelease>` on a managed addon, marked in `list` (task 021),
  `rollback <name>` (task 022), `adopt --flavor <flavor> [--apply [--alongside-curseforge]]` (tasks 097
  and 098, see [Adopt](#adopt)),
  `set save|apply|list|show|remove` (task 014; `--character` on `save` and
  `apply`, task 027; `set undo`, `set enable|disable|drop <set> <addon>` and `--account`, task 073;
  below), `update --all | <name> --flavor <flavor> [--dry-run] [--yes]` (task 006,
  below), with `--json` for machines and `--serious` for the murloc-averse; its end-to-end tests under
  `tests/`. `list`, `add`, `update` and `import` (task 090) run the dependency check
  (task 029; `scan` reads the folder only): a warning line per addon with a missing required dependency,
  `missing: <names>` at the end of the addon's `list` line, and in `--json` a `dependencies` object per
  addon of `list`, a `dependencies` array of the installed folders under `add`'s `addon` and on every
  installed entry of the `import` document, each with
  `folder`, `required`, `met`, `embedded` (a subset of `missing`), `missing` and `nolib` (`true`, `false` or
  `null` for unknown).
  `savedvars --flavor <flavor> [--clean]` (task 025) prints a data line per SavedVariables entry (size,
  addon, `account-wide` or `realm / character`, never the account folder's name, which `--json` carries,
  `orphan` or `client`) and a summary, `--clean` moves the orphans; `--json` is the `savedvars` or the
  `savedvars-clean` document. Its `build.rs` (task 012) gives `mrgl.exe` the window's icon as a Windows
  resource, with the standard library alone: it rewrites `crates/mrgl-app/icons/icon.ico` as a compiled
  resource file (`.res`, one `RT_ICON` per image and the `RT_GROUP_ICON` that lists them) and passes it to
  the MSVC linker, which takes a `.res` as an input file; other targets skip it. The same `.res` carries the
  version resource (task 079): an `RT_VERSION` whose `VS_VERSION_INFO` block holds the package's version,
  the workspace one, as `major.minor.patch.0` in `VS_FIXEDFILEINFO` (the prerelease flag set for a version
  with a suffix) and as written in `Cargo.toml` in the `FileVersion` and `ProductVersion` strings, next to
  `FileDescription`, `InternalName`, `OriginalFilename` and `ProductName`, in one US English Unicode string
  table; so the exe's properties show the version, as the app exe's do through `tauri-build`. A test of
  `tests/cli.rs` reads it back from the built exe with the core's PE reader (`pe::read_file_version`).
- `crates/mrgl-app` — the `mrgl-app` executable: the Tauri 2 window (`src/`: the IPC commands in
  `commands.rs`, the flavor list in `list.rs`, task 009; the update check, the updates and the row marks in
  `actions.rs`, task 010; a flavor's addon sets and SavedVariables in `toolbox.rs`, task 074, with the adopt
  card of task 109), the static UI
  under `ui/`, the capability file and the icon; see
  [The UI](#the-ui).
- `crates/mrgl-test-support` — the helpers the tests of the three crates share (task 118), a dev-dependency of
  each and never part of an exe: `stand_in`, the stand-in game client of the running-client tests (see
  [Fixtures](#fixtures)), and `locked`, the file and folder handles a test holds open so that a read, a move or
  a removal under them fails. The standard library only and Windows only (the other targets compile an empty
  crate); its code is test code, left out of the coverage measurement (see [Coverage](#coverage)).
- `data/` — `forever-notes.json`, the curated Forever notes compiled into the core (see
  [Forever notes](#forever-notes)).
- `lists/` — shareable addon lists in the format `mrgl export` writes (`list_file`, task 015), read by
  `mrgl import <file> --flavor <flavor>`: `forever-starter.json`, the Forever starter list (task 028). The
  format rule: an entry names a source and a version, its folders are plain folder names, and nothing in it
  is a path of one machine; reading a list refuses an entry whose source is a zip file, a linked folder
  (`link:`) or another local or absolute path, or whose folder is a path, and names every such entry, and
  the export leaves an addon from a zip in hand or a linked folder out with a warning. The starter list
  holds GitHub entries only, each one checked against its repository's latest release (a `release.json`
  that names `forever` or `camelot` for an asset of the release, or a `-forever` asset); the check, with
  the release tag and the date, and the candidates without a Forever build yet are in task 028's Log. A
  change to the list changes its stub under `crates/mrgl-core/tests/fixtures/starter-list/` with it (see
  [Fixtures](#fixtures)).
- `scripts/` — helper scripts for the icon, the release zip, the changelog, the commit rules and the coverage
  table (`coverage-check.py`, with the pins `coverage-toolchain.txt` and `cargo-llvm-cov-version.txt`, see
  [Coverage](#coverage)).
- `docs/` — this page, the git workflow, the maintainer sessions, origin, research and the voice guide.
- `tasks/` — the backlog, the work in flight, the history and the decisions (the task standard).
- `.github/workflows/` — CI (fmt, clippy, test, the oldest supported Rust, the changelog check, the dependency
  policy, the workflow lint, the coverage measurement, the commit rules), the tagged release, the weekly advisory
  check and the task-to-issue mirror;
  `.github/actions/install-rust-toolchain/` — the toolchain install step the CI jobs share;
  `.github/dependabot.yml` — the weekly dependency updates.
- `rust-toolchain.toml` — the pinned Rust version (see [The Rust toolchain](#the-rust-toolchain)).
- `.githooks/` — the `commit-msg` hook over `scripts/check-commit.sh`, enabled per clone with
  `git config core.hooksPath .githooks` ([git-workflow.md](git-workflow.md)).
- `deny.toml` — the dependency policy `cargo deny` checks: licenses, advisories, bans, sources;
  `about.toml` — the same licenses for the third-party license page of the release zip.
- `CHANGELOG.md` — every release and the tasks it closed, generated from `tasks/done/` (see
  [The changelog](#the-changelog)).

The addon list (task 015) is a JSON file, `format` `mrgl-list`, `schema` 1: the flavor id it was exported
from (`flavor`), when (`exported_at`, RFC 3339 in UTC) and one entry per managed addon under `addons`, with
the sidecar's `name`, `source` and `version` and the addon's `folders`. `mrgl export` writes the sidecar's
entries whose folders are still there (the list itself on the standard output without `--to`); a file of
another format or schema is refused on import. `mrgl import` resolves every entry of a source spec for
the target flavor's build through the run's sources and installs the current release through the engine
under the entry's name, so the version in the list is a record, not a pin; an entry of a source this build
does not have or cannot ask is reported as `source not configured` with what is missing (outcome
`source-not-configured`, task 092), a zip in hand is reported as not importable, a `link:` entry is skipped,
and an addon the flavor already has (the same name, the same source, or a folder of the entry that is there,
managed or not) is reported and left alone. Once every entry is done, `import` runs the checks of the
install summary of `add` and `update` over the addons it installed (`Engine::check_installed_each`, one
scan of the flavor as the import left it, so a library installed later on the list counts as there; task
090): a missing required dependency is a warning line (`NeedsLibrary needs the missing dependency
LibMissing: install it or the addon does not load`) in the entry's `warnings` and the document's, and the
entry's `dependencies` carries the check of each installed folder; the interface warnings follow (task 082,
below). `--dry-run` resolves every entry and writes nothing under
`Interface\AddOns` or in the sidecar (the release list cache of the GitHub source is written as on any
resolve). An entry that fails does not stop the others; the run then ends with exit code 1. The list may
come from another flavor: the picks are made for the flavor it is imported into, with a warning.

`mrgl import --exact` (task 057) installs the version the list names for each entry instead of the current
release: the request asks the source for that version (`Wanted::Version`, `Source::resolve_version`), and
the GitHub source takes the release of the same list whose tag is the version, with or without its leading
`v`, never a draft, on any channel (`sources::github::release_of_version`), and picks its asset for the
flavor as for the current one. An entry whose version the source no longer offers is reported (`<name>
skipped, version <v> not offered: no release <v> in <source>`, outcome `version-not-offered` in the JSON,
`version not offered` in the summary, a warning) and skipped; it does not fail the run. The lookup reads the
releases the release list holds (the first page, and the pages after it while none holds a stable release,
task 042) and, when the version is not among them, the pages after those, `page=<n>` of the API, until the
version turns up, a page is short (the last one) or five pages are held in all (`MAX_PAGES`, task 066);
only then does it read as not offered. These further pages are not cached, and offline the cached list is
all there is. The JSON document carries `exact`.

The one install path (task 055) is `Engine::install_from_source` in `install`: `add github:`, `import`
and `update` call it, the window will too. It takes a `SourceRequest` (the spec, which release, `Wanted::Current`
or a pick made already, and the name the list file gives the addon) and resolves it for the flavor's build
on the addon's own channel, the channel of its sidecar entry from that source (else of the entry of that
name, else `stable`, `Engine::target_for`, task 021). The sidecar entry is looked up by the source and the
request's name together where the request names the addon (`update`, `import`; task 100: of two entries
from one source, a renamed addon next to its old name, the one named answers, not the one whose name sorts
first), else by the source alone, else by the name alone. An addon the sidecar has from that source at the
pick's version, its folders there, is installed already: nothing is downloaded and nothing moves. Otherwise
the asset is downloaded into a fresh folder under `staging\`, staged and committed with the pick's source
and version under the name of that entry (else the request's name, else the folders'),
and the download is removed whatever the install returned. `Engine::resolve` is the same resolve alone, for
a dry run, and `Engine::install_pick` the same path for a pick in hand. The link refusal of task 005 sits in
`Engine::commit`, so a zip in hand, a source, an import, an update and a rollback all refuse to install over
a linked addon or a junction mrgl did not make. The command line reports every install with one summary
(the line and the `addon` block of the JSON document, with the dependency and interface checks). The line
follows the commit's outcome (`InstallOutcome`, told by the entry the commit replaces, task 083): `installed
OneFolder 1.0.0 (1 folder: OneFolder) into _forever_` without an entry, `updated OneFolder from 1.0.0 to
1.1.0 (…) in _forever_` over an entry of another version, and `reinstalled OneFolder 1.1.0 (…) in _forever_`
over an entry of the same version, whose previous version stays what it was, so `rollback` still goes back
to it; `add <file.zip> --dry-run` says `would install`, `would update` or `would reinstall` by the same rule.
The `addon` block carries the outcome as `outcome`: `installed`, `updated` or `reinstalled` (`null` for an
addon found installed already, where nothing was installed).

The staging folder's lifetime (task 084, `mrgl_core::staging`): every download and every unpacked zip gets
a run folder of its own under `<flavor-id>\staging\` (`<stem>-<nanoseconds since 1970>-<attempt>`), and
the run holds an exclusive lock on the `.mrgl-run.lock` file in it for as long as the folder is its own.
The guard (`RunDir`) removes the folder on every path out; a run that ends before its guard goes (a killed
process, a crash) leaves it behind, so `Engine::stage` sweeps the leftovers first, for every install, update,
import, rollback and dry run of a zip. A folder is a leftover when its lock is free or missing, it is older
than an hour (by the time in its name, else its modified time) and it holds no `previous\` folder. A held
lock keeps the run's own folders and those of another running mrgl on the same flavor (the window next to
the command line); the hour keeps a folder made a moment ago that is not locked yet, and a run of an mrgl
from before task 084, which takes no lock; `previous\` holds the folders an interrupted or failed swap moved
aside, perhaps the only copy of the version installed before, so that folder stays until it is removed by
hand, and `mrgl doctor` names it with its age, its size and what to do with it (task 088, see
[The doctor](#the-doctor)). A leftover that cannot be removed is a warning on the install's result, never an error.

The folders' name (`install::derive_name`, for a zip in hand and a source install the sidecar and the
request do not name) is the folder every other folder of the zip extends with `_` or `-` (`BigWigs` for
`BigWigs_Core` and `BigWigs_Options`); else the folder named after the zip's stem without its version and
flavor suffix (`DBM-Core` for `DBM-Core-12.0.11-classic.zip`), else after the repository of a `github:`
source, both ASCII case-insensitive (task 065); else the first folder by name. A source install of an addon
the sidecar has from that source keeps the name it is recorded under. The commit replaces the entry of the
staged name, else the entry that records the same addon under another name (task 077, `renamed_entry`): the
entry that records the folder the staged name names while its own name is a folder of the staged addon, so a
zip in hand recorded as `DBM-Brawlers` before task 065 and staged as `DBM-Core` now is the same addon, while
an entry that only records one folder the zip also ships keeps its name and loses that folder. The new entry
keeps the replaced one's pin, ignored mark and channel and takes its version as the previous one (its
previous version when the version is the same), the zips of its cache folder move to the new name's, so
`rollback` finds the previous version's zip, and the install says so in one warning (`DBM-Brawlers is managed
as DBM-Core now, with its marks and its previous version`).

The update check (task 006) is `update::check` in the core: every managed addon of the sidecar (or the one
a name or a folder names) against its source's current release on the addon's own channel, one
`UpdateState` each, read-only. `up-to-date`: the release's version is the installed one. `update-available`:
another version, picked from metadata. `guess-only`: another version whose asset for the flavor is a guess
by its name; it is reported, not installed, and `mrgl add <source>` installs it anyway. `pinned`: checked
and reported with the current release, left alone; its line reads the installed version against the release
by the version order below (task 077): `pinned at 2.0.0, 2.1.0 is out, left alone` when the installed version
is older (or the order cannot tell), `pinned at 1.1.0-beta, newer than the current release 1.0.0, left
alone` when it is newer (with the `is no longer listed` reason as `installed-newer` has it), `pinned at
1.0.0, up to date` when it is the same. `ignored`: skipped without a request. `pending` (task 104): the
source's newest file waits for the site's approval (WoWInterface's `UIPending`), so there is nothing to
install yet and nothing for the user to do: its line is `MrglPending  1.0.0  pending approval: the newest
file waits for approval on WoWInterface; nothing to install until it is approved`, it is counted apart
(`1 pending approval`), it is no failure (a run with pending addons alone ends with exit code 0) and the
summary needs no look for it. `distribution-off` (task 115): the author of a CurseForge project allows no
download outside CurseForge (`Error::DistributionOff` of task 096, below), so there is nothing mrgl can
download; it is the author's choice, not an error: its line is the source's, `MrglNoDistribution  2.0.0
only on CurseForge: the author of curse:4244 (Mrgl No Distribution) allows no download outside CurseForge:
download the zip from <page> and install it with `mrgl add <file.zip> --as curse:4244``, it is counted
apart (`1 only on CurseForge`), a run whose only other answers are no failure ends with exit code 0, and
the summary needs no look for it; a pinned entry stays `pinned`, as for any answer without a release.
`source-gone`: no
release, no asset for the flavor or no repository (404) any more. `not-checked`: no source to ask (a zip in
hand, a linked folder, a source not handled yet). `failed`: the source could not be asked (offline, a rate
limit, a server error). `installed-newer` (task 068): the installed version is newer than the current
release on the addon's channel (a deleted release, a switch from the prerelease channel to the stable one);
it is not an update and is left alone, its line `Switched  1.1.0-beta  newer than the current release
1.0.0, left alone`, with `(<version> is no longer listed)` when the release list lacks the installed
version. The order is the GitHub source's cached release list (`Source::listed_versions`, newest first,
no request) where it lists both versions, else `update::lenient_order`: a leading `v` dropped, the numbers
before the first `-` or `+` compared as numbers (`1.2` is `1.2.0`), and on equal numbers a version with a
suffix before one without (`2.0.0-beta` before `2.0.0`); when neither can tell, another version counts as
an update. `rate-limited`: not asked, the check stopped asking the addon's source before its request limit
(below). An addon whose
folders were deleted by hand is left out with a warning.

`update::check` takes a progress callback, `&mut dyn FnMut(CheckProgress)`, the one the command line and
the window (task 010) share: `Checking { index, total, name, source }` before an addon the source is asked
about, `Checked { index, total, addon }` after every addon with its `AddonUpdate`, and `Stopped { stop }`
once per source when the check stops asking that source before its request limit. The request budget:
without a token the GitHub source counts the API requests of its run against the 60 an hour
(`Source::request_budget`), and once an answer names
`x-ratelimit-remaining` that count, less the requests since, takes over with the answer's reset time (a
429 leaves nothing); an asset download is not counted, a token turns the budget off; the Wago source keeps
a budget of its own (below). Before each addon that needs a request the check reads the budget of the
addon's source (`Source::request_budget_for`), and with nothing left it stops asking that source alone
(task 101): the addons of that source left are `rate-limited` (`not asked before GitHub's rate limit`), and
the addons of the other sources go on being asked. `UpdateCheck::stopped` holds one `BudgetStop` per source
that stopped, in the order they stopped, each with its source (`github`, `wago`), the limit, the requests
sent, the reset time and how many addons of that source it left, with its line `stopped asking GitHub
before its rate limit of 60 requests an hour without a token: 2 addons not checked, the limit resets at
<local time> (local time); set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour` (`within
the hour` when no answer named the reset), and the summary counts the addons left per source (`2 not
checked (GitHub's rate limit)`). `mrgl update --all` and `mrgl update <name>` print a progress line through the voice before
each addon the source is asked about (`checking Questie against github:Questie/Questie (10 of 12)`), each
source's stop as a warning when it happens, and a plain data line per addon as soon as it is checked
(`Questie  1.2.2 -> 1.2.3  update available`), ask once for the updates (`--yes` answers; without a
terminal or with `--json` the command stops and says what `--yes` does, exit 1), install each through the
one install path with the pick the check made (`AddonUpdate::request`, no second resolve), say each
install's summary line with the dependency and interface warnings of the installed addon, and end with the
summary that counts every state (`1 addon updated, 2 up to date, 1 pinned, 1 ignored`; `--dry-run` says
`1 update available` and writes nothing under `Interface\AddOns` or in the sidecar). An update the one
install path answers with "installed already" is not counted as updated (task 089): under the addon's own
entry (another run updated it since the check) `update` says the nothing-to-do line of `add`; under another
entry of the same source, which the install path falls back to only when no entry of the addon's name has
that source any more (task 100; an install earlier in the run took its folders over, say), it says a
warning that names that entry, `QuestieClassic not updated: Questie, a second entry of github:Questie/Questie, has 1.2.3 installed already in _forever_; QuestieClassic stays at 1.2.2`, and leaves
both entries as they are; the summary counts either as `1 installed already`. The words are the core's
(`AlreadyInstalled::update_line`), and the window's update says them too (task 099). An install that takes
every folder of another entry over drops that entry and names it in `Installed::folded` (task 108, the
core's `Folded`: the dropped entry's name, version and folders, the addon it joined and that addon's
version), next to the warnings `<entry> lost 1 folder to <addon>` and `<entry> is no longer managed: <addon>
replaced every folder of it`. When a release zip of one source carries the folder of a second entry of that
source, `update --all` updates the first entry and then, for the second, installs nothing and says the
core's line (`Folded::update_line`) through the voice: `SeveralFolders_Options folded into SeveralFolders:
SeveralFolders 1.2.3 ships its folder SeveralFolders_Options in _forever_, SeveralFolders_Options has no
entry of its own now`; the summary counts it as `1 folded into another entry`, neither updated nor
failed, and the sidecar holds the one entry after. The window's update says the same line as its own phase
(task 114). Right after the check,
before the confirmation and on a dry run too, `update` records the check's answers in `last-check.json`
as the window's check does (task 087, below): a source gone, a failed check, an addon left before the
rate limit and a project without distribution (task 115) are kept for the window's rows, an addon answered otherwise drops its kept answer, and a file
that cannot be written is a warning (`the answers of this check are not kept for the window: …`). `--json` is one `update` document: every addon with `state`,
`latest`, `order` (the installed version against `latest`: `older`, `same`, `newer`, or `null` when there is
no `latest` or the order cannot tell; task 077), `pick`, `updated`, the installed `addon` block and the
interface check of the installed addon, `interface_mismatch` and `interface_out_of_date` (task 082, below),
`installed_already` (task 089: the entry the install path found at the pick's version, as an `addon` block
with `same_entry` and the line as `message`; `null` otherwise), `folded_into` (task 108: the addon an
install earlier in the run folded the entry into, `{ name, version, folders, message }`; `null` otherwise),
the document's `updated`, `installed_already` and `folded` counts, the `counts` per state (`rate-limited` the
sum over the sources), `stopped` (a list of the `BudgetStop`s, one per source that stopped, each with its
`source` and its `message`; empty when the check asked about every addon, task 101) and the warnings. An
update or a check that fails does not stop the others; the run then ends with exit code 1. A stop before
a rate limit is a warning in the summary (`2 not checked (Wago's rate limit)`), not a failure: exit code 0.

The interface check (task 015) compares the `## Interface` numbers of the TOC the flavor loads with the
installed build: an interface number's major version is the number divided by 10000 (`120105` is 12,
`16001` is 1), and the addon passes when one number of the list has the build's major. A mismatch is a
warning, never a refusal: `scan` and `list` mark the addon's line `interface mismatch` and add one warning
line, their JSON carries `interface_mismatches`. An addon that passes but whose highest number of the
build's major is below the build's own interface number (`120100` under `12.1.5`, interface `120105`) is
what the client lists as out of date (task 076): `scan` and `list` mark its line `out of date`, apart from
the mismatch (an addon is one or the other), and add one warning line, and their JSON carries
`interface_out_of_date`, each with `folder`, `toc`, `interfaces`, `build`, the build's `interface` and the
TOC's `highest` number of the build's major; a number above the build's is current. After an install, `add`
(a zip in hand or a source), `import` and `update` check the installed addon's primary folder for both
states (`Engine::check_installed`, task 082) and say one warning line through the voice: `StaleInterface:
interface 110207 is not for the build 1.60.1.70245 of _forever_; the zip may be for another flavor` for a
mismatch, `BehindInterface: interface 16000 is out of date for the build 1.60.1.70245 (16001) of _forever_;
the client lists it as out of date` for out of date. Their JSON carries both keys, `interface_mismatch`
and `interface_out_of_date`, the object of the state or `null`: at the top of the `add` document, on every
entry of the `import` document, and on every addon of the `update` document (`null` for an addon that was
not updated). The window's update answer reads the same core check (task 079). An unknown build, a TOC
without an `## Interface` line and a folder the flavor loads no TOC from give no warning.

The addon sets (task 014, `mrgl_core::sets`) are named lists of addons per flavor, each addon `enabled` or
`disabled`, in `sets.json` under the flavor's folder of the mrgl home (`<home>\<flavor-id>\sets.json`, next to
the sidecar: `format` `mrgl-sets`, `schema` 1, the `flavor` id and `sets`, each with `saved_at` and its
`addons`). The file is the user's data, not a cache: one that cannot be read, or that belongs to another
flavor or schema, is an error that names it, and mrgl never overwrites it. The characters are the folders
`WTF\Account\<account>\<realm>\<character>\` (the account's `SavedVariables` is no realm), and each keeps
the client's `AddOns.txt`, one `Name: enabled` or `Name: disabled` line per addon. `mrgl set save <name>`
reads every character's file and keeps each addon with its state; an addon the characters disagree on is
left out (an apply then leaves it alone) and named in a warning, and a set of the same name is replaced.
`mrgl set apply <name>` is the second `WTF` write of D-002 and D-003: the guard of task 023 refuses it while
the client runs, before anything is written; then exactly the set's addons get its state in every
character's file and every other byte stays as the client wrote it (the line endings, a missing last one, a
byte order mark, the order, a line mrgl does not understand); an addon a file does not name gets a line at
its end with the file's line ending, a character without the file gets one (CRLF), and a file that would not
change is not written. Every new content is worked out before the first write, each file goes through a
temporary file renamed over it, and a failure part way puts the files already written back. No folder is
deleted or moved; an addon of the set that is not installed is a warning and its line is written all the
same. `set list`, `set show <name>` and `set remove <name>` read and edit `sets.json` only; every verb takes
`--flavor` and `--json` (`kind` `set-save`, `set-apply`, `set-undo`, `set-edit`, `set-list`, `set-show`,
`set-remove`).

A set is per flavor, its target may be one character (task 027): `set apply <name> --character <name>
[--realm <realm>]` writes that character's `AddOns.txt` only and leaves every other character's file byte for
byte, and `set save <name> --character <name>` takes the set from that one file (so nothing is left out as a
disagreement). The character is picked from the `WTF` account folders by its folder's name, ignoring case;
`--realm` narrows a name more than one realm has. A name on two realms without `--realm` is refused with the
realms listed, and a name no character has with the characters listed; when one realm and name are under
two accounts the error names the accounts and asks to pick the account (the core's words name no flag, so
the window shows them as they are; the command line adds `, with --account`, task 091), and `--account <name>`
on `save` and `apply` (task 073, with
`--character`) picks the account folder, ignoring case; an account with no character is refused with the
accounts listed. The guard of task 023 holds as for every character: nothing is
written while the client runs. In `mrgl_core::sets` the choice is `Target` (`EveryCharacter`, or
`Character { name, realm }`) and `find_character` is the lookup the window can call; the account narrows
them through `targets_in_account`, `find_character_in_account`, `save_set_in_account` and
`apply_set_in_account`, so `save_set`, `apply_set` and `find_character` keep their signatures and results.

A set apply can be undone (task 073). Every apply that changes a file keeps an undo record under the
flavor's folder of the mrgl home, next to `sets.json` (`<home>\<flavor-id>\set-undo\`: `undo.json`
with `format` `mrgl-set-undo`, `schema` 1, the `flavor` id, the `set`, `applied_at` and per file the
account, realm and character folder names, the `<n>-before.txt` copy of the bytes it replaced, absent for
a file the apply created, and the `<n>-after.txt` copy of what it wrote). The record is the last apply's
only: it is written into `set-undo.new\` before the first `AddOns.txt` write and moved in place of the
one before, which is put back when the apply fails and its files are as they were; an apply that changes
nothing leaves the record of the one before. `mrgl set undo` (`sets::undo_apply`) reads the record and
every kept file first (a record of another flavor or schema, a name that is a path, or a missing copy is
refused before anything is written), runs the guard of task 023, puts every replaced file back byte for
byte and removes every file the apply created, each write through a temporary file with the files
written so far put back on a failure, and then deletes the record, so it undoes once; a file that changed
since the apply (the client writes it at logout) is put back all the same with a warning, and a character
folder that is gone is passed over with one. `mrgl set enable|disable|drop <set> <addon>`
(`sets::edit_set`) changes one addon of a stored set in `sets.json` only: the name is matched ignoring
ASCII case and keeps the set's spelling, an enable or a disable of an addon the set does not have adds it
(an addon that is not installed is a warning), a drop of one it does not have is refused with the set's
addons listed, an edit to the state the addon has already writes nothing, and one that changes the set
moves its `saved_at`; no `AddOns.txt` is touched and no undo record is written. `set save` keeps an addon
of the characters' files that has no folder under `Interface\AddOns` and names it in a warning
(`not_installed` in `--json`): a set is the user's data, and `set apply` already writes such a line.

## The unpacked-size bound

The shared zip-entry check (`zip_entry`) also bounds what one zip unpacks to (task 039), so a zip bomb from a
compromised source cannot fill the disk from the install engine's staging folder or from `WTF`. The bound is
`zip_entry::MAX_UNPACKED`, 2 GiB; `--max-unpacked <size>` on `add`, `rollback` and `backup --restore` raises
or lowers it for one run (`3G`, `500M`, `64K` or a number of bytes, in steps of 1024; a size that does not
parse, zero, or the flag without `--restore` on `backup` is a usage error). In the core the bound is
`Engine::with_max_unpacked` for every stage of that engine (an install, a pick, a rollback) and
`backup::restore_wtf_bounded` for a restore. It holds twice:

- **Declared.** `checked_entries` sums the uncompressed sizes the file entries declare once every entry
  passed its name check, and a sum past the bound refuses the zip before anything is written:
  ``refused <zip>: it declares 3.0 GB unpacked, more than the bound of 2.0 GB; nothing was written, and
  `--max-unpacked <size>` raises the bound for a zip you trust``.
- **Written.** Each caller reads every entry it writes through `Unpacked::entry`, which counts the bytes and
  refuses once they pass the bound or once an entry outgrows the size it declared (a zip that lies about
  its sizes). The zip crate itself stops an entry at its declared size; the counter reports that refusal
  as the same lie: ``refused <zip>: the entry `<name>` unpacks to more than the 16 B it declares, so the
  bound of 2.0 GB cannot hold (109 B written, then removed)``. The engine removes its staging folder, as
  after every failed stage; the restore removes the half-written `WTF`, puts the previous one back and
  reports `restoring <zip> failed and WTF is as it was: ` with the same clause.

Both are `Error::ZipTooLarge` (the restore's write-time refusal is wrapped in `Error::RestoreFailed`, as
every failure after `WTF` moved aside is), exit code 1 on the CLI and the `message` of the JSON error
document. Sizes in these messages are written in steps of 1024, as the `backup --list` sizes are. The
fixtures under `zips/` that test it, written by `make-zips.py`: `declares-too-much-1.0.0.zip` and
`wtf-declares-too-much.zip` declare 3 GiB for a few bytes, `lies-about-its-size-1.0.0.zip` and
`wtf-lies-about-its-size.zip` declare 16 bytes for about 64 KB of padding; each is a few hundred bytes.

## The voice

Every human-readable message of the CLI and the window goes through `mrgl_core::voice`: a murloc line and a
plain English line, the murloc one picked by the message's kind (banner, progress, success, nothing to do,
warning, error, fatal) from the table in `voice.rs`. JSON output (`--json`) and log lines are plain.
`--serious` or `MRGL_SERIOUS=1` drops the murloc line. The switch has three sources, the first that says
wins: the `--serious` flag, then a non-empty `MRGL_SERIOUS` (`0` keeps the murloc, anything else drops it),
then the `serious` preference in `config.json` that the window's switch writes (task 034,
`voice::serious_from`). Tests assert the plain line, so a change to the murloc
vocabulary breaks no test. The vocabulary and the rules are in [nerglish.md](nerglish.md); the decision is
D-005.

A summary line that the command line and the window both print lives in the core, once, as a `(Kind, plain
line)` pair the caller says through its own voice: the update check's `UpdateCheck::summary` (task 010), and
the toolbox's `sets::Applied::summary` (`mrgl set apply`, the window's set switch),
`savedvars::Listing::summary` (`mrgl savedvars`, the window's SavedVariables card; `CleanupBy` names
`` `mrgl savedvars --clean` `` or the window's cleanup) and `savedvars::Cleaned::summary` (`mrgl savedvars
--clean`, the window's cleanup), task 086, then `sets::Undone::summary` (`mrgl set undo`, the window's
undo) and `sets::Edited::summary` with its `Edited::not_installed` warning (`mrgl set
enable|disable|drop`, the window's chips), task 085, then the set list note `sets::SetsFile::summary`
(`mrgl set list`, the note under the window's set picker) and the apply's `sets::Applied::not_installed_warning`
(`mrgl set apply`, the window's set switch), task 091, and adopt's `adopt::Report::summary`, its
`Report::marker_line` and `adopt::marker_refusal` (`mrgl adopt`, the window's Adopt card; `AdoptBy` names
`adopt --apply` and `--alongside-curseforge`, or the window's Adopt button and the confirmation of its
dialog, task 112). A count and its verb agree (`voice::count_noun`: `1 orphan (12 B)
has no addon folder`, `2 orphans (50 B) have …`). The CLI tests and the window's `tests/toolbox.rs` assert
the same lines.

The command line writes every line through one helper (`emit` in `mrgl-cli/src/main.rs`, task 075): the
`Reporter`'s messages, data lines and JSON documents and `export`'s list on stdout, its warnings and errors on
stderr. A closed pipe on either (`BrokenPipe`, os error 232 on Windows: the reader of
`mrgl list --json | head -c 200` is gone) ends the run there, quietly and with exit code 0, since nobody
reads the rest; any other write error panics as `println!` would. A new output site writes through `emit`
too, never through `println!`. While an addon's install is in flight (`add github:` and `update`, which
write their progress lines during the install: the progress callback holds an `InFlight` guard, task 084;
`import` and `add <file.zip>` write nothing while the engine works), a closed pipe does not end the
run at the next write: the lines are dropped, the swap, the sidecar write and the removal of the run's
staging folders finish, and the run ends with 0 once that addon's install returns, before the next one
starts.

## The doctor

`mrgl doctor [--flavor <flavor>] [--root <folder>]` prints the diagnostic report a bug report asks for (task
024): a plain block, one fact per line like the data lines of a listing, so it pastes as it is, then the
two-line summary through the voice (a success when nothing needs a look, a warning that names what does);
`--json` is the same report as one document with `kind: "doctor"`. The report is the core's
(`mrgl_core::doctor`: `report_of_process` builds it, `Report::lines` and `Report::text` are the block), so the
window's "copy diagnostics" action (task 009) copies the same text. It reads and never repairs: the AddOns
probe is its one write. A fact it cannot read is a value (`unknown`, `missing`), never an error, and only an
unknown `--flavor` fails the command. Every path under the user's profile folder (`USERPROFILE`, else `HOME`,
in its long and its 8.3 short form) is shown with that folder as `~`: matched in any case and with `\` or `/`
(or a mix) as the separator, and only when a separator or the end of the path follows, so a folder that only
starts with the profile's name (`someone2` next to the profile `someone`) stays as it is (task 069). An API
key is named, never read out, so a pasted report carries no home folder path and no secret.

The lines, with the JSON field in brackets:

- `mrgl: <version>` (`mrgl_version`): the version of the exe.
- `windows: Windows 11 24H2, build 26100.1234` (`windows`: `name`, `display_version`, `build`, `ubr`,
  `product_name`; task 051): the Windows version from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`,
  read like the WebView2 version below: `DisplayVersion`, `CurrentBuildNumber` and the update build revision
  `UBR` (a `REG_DWORD`) after the dot. The name comes from the build (22000 and later is Windows 11, 10240
  and later Windows 10), since `ProductName`, kept as written in the JSON, says Windows 10 on Windows 11 too;
  a product name of another Windows (a server) is shown as written. A value that is not there is left out
  (`Windows 11, build 26100`) or `build unknown`, `null` in the JSON, and with none of them the line is
  `windows: unknown`.
- `root: <folder> (<origin>)` (`root`, `root_origin`): the game root the run resolved, as every command does,
  and how: `from --root`, `from MRGL_ROOT`, `remembered`, `found in the launcher's registry key`, `found in
  the game's Uninstall registry entry`, `found in a usual install folder` (`flag`, `environment`,
  `remembered`, `registry`, `uninstall_entry`, `common_folder`); `none found` when nothing names or finds
  one.
- `home: <folder>` (`home`): the mrgl home folder (`MRGL_HOME`, else `%LOCALAPPDATA%\mrgl`).
- `webview2: <version> (per-machine)` or `(per-user)` (`webview2`: `version`, `scope`): the WebView2 runtime
  the window runs on, the `pv` value under `Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`,
  read the way the game root discovery reads the launcher's key: per machine (`HKLM\SOFTWARE\WOW6432Node\…`,
  then `HKLM\SOFTWARE\…`) before per user (`HKCU\Software\…`); `unknown` when no key names a version.
- `github rate limit: <n> of <limit> left, resets <time>` (`github_rate_limit`: `remaining`, `limit`,
  `reset`, `reason`): one `GET https://api.github.com/rate_limit` (GitHub does not count it), with the token
  when one is set; the limit is 60 requests an hour without a token and 5000 with one, the reset time is UTC.
  `unknown (<reason>)` when offline, on another status than 200, or for an answer without the numbers.
- `api keys: MRGL_GITHUB_TOKEN set (environment), MRGL_WAGO_KEY set (credential store),
  MRGL_CURSEFORGE_KEY not set` (`api_keys`: `name`, `present`, `origin`: `environment`, `credential_store`
  or `not_set`): every key of the sources by name and where it comes from (task 093, [The API
  keys](#the-api-keys)); an empty variable is not set, and a value is never read into the report.
- `flavor: <folder> (<id>)` (`flavors[]`: `folder`, `id`, `found`): one block per flavor folder of the root,
  or the one `--flavor` names; the id is the folder's `.flavor.info` value. `folder missing` when the asked
  flavor has no folder under the root: the block then says what the home folder remembers of it.
- `  folder: <path>` (`path`): the flavor folder.
- `  build: <version>, interface <n>, from <file>` (`version`, `interface`, `version_source`,
  `version_file`): the installed build and the file it came from, `.build.info` or the client exe (task 019);
  `unknown` when neither has it.
- `  curseforge app: yes` or `no` (`curse_client`): the CurseForge app's `.curseclient` marker is in the
  folder.
- `  addons folder: writable`, `not writable (<reason>)` or `missing` (`addons`: `status`, `reason`): an empty
  probe file `.mrgl-doctor-probe-<pid>-<n>` was created in `Interface\AddOns` and removed at once; `missing`
  when there is no such folder (a client before its first launch) or no flavor folder, and then nothing is
  written.
- `  sidecar: parsed, <n> managed addons`, `rebuilt (<reason>)`, `missing` or `unreadable (<reason>)`
  (`sidecar`: `status`, `managed`, `reason`): the state sidecar `<home>\<flavor-id>\state.json` (D-003).
  `parsed` reads; `rebuilt` is there but corrupt, of another schema or of another flavor, so the next command
  that reads it starts over from a scan; `missing` means nothing was installed through mrgl yet; `unreadable`
  is a file that cannot be opened at all.
- `  cache: <size> in <n> files` or `empty` (`cache`: `bytes`, `files`, `reason`): the install engine's zips
  under `<home>\<flavor-id>\cache\`, links and junctions not followed.
- `  kept staging: <folder> (<age> old, <size> in <n> files; previous: <names>)`, one line per folder, then
  once `  kept staging advice: <what to do>` (`kept_staging[]`: `path`, `created`, `age_seconds`, `bytes`,
  `files`, `previous`, `advice`; task 088): every run folder under `<home>\<flavor-id>\staging\` that holds
  a `previous\` folder, the addon folders an interrupted or failed swap moved aside, which the sweep of task
  084 never removes (`mrgl_core::staging::kept`). A folder whose lock a running mrgl holds is a swap in
  progress and is left out. The age comes from the time in the folder's name, else its modified time
  (`created` in UTC); the size counts every file under it, links and junctions not followed. The advice
  says to copy a folder from `previous\` back into `Interface\AddOns` when that addon is missing or broken,
  then delete the staging folder: the doctor names it, the user decides, and mrgl never removes it. No line
  and an empty list when there is none.
- `  client: running (<exe>, pid <n>)`, `not running` or `unknown (<reason>)` (`client`: `status`, `pid`,
  `exe`, `reason`): whether a process runs from the flavor folder (task 023); `unknown` when the process list
  cannot be taken.
- `  warning: <text>` (`flavors[].warnings`): what reading the flavor folder met (an unknown id, a build
  neither file has).
- `warning: <text>` (`warnings`): what the run met before the flavors (an unreadable config, an unreadable
  game folder).

The summary names what needs a look: no game root, a missing flavor folder, an AddOns folder that takes no
write, a sidecar that is rebuilt or unreadable, a kept staging folder, a process list that cannot be taken, a
used-up rate limit and the warnings. The tests run the report offline: the core's tests hand it a fixed process list, an in-memory
HTTP stub and a fixed WebView2 and Windows answer; the end-to-end tests stub the WebView2 version with a
`webview2=<version>` item of `MRGL_DISCOVERY` and the Windows version with a
`windows=<display version>,<build>.<ubr>` item (`windows=24H2,26100.1234`, a part may be left out; any other
value of the variable is no runtime and an unknown Windows, so `off` reads no registry), answer the rate
limit from `releases/api.github.com/rate_limit.json` and assert that neither a key's value nor a `\Users\`
path is in the output.

## The API keys

The sources' keys are the user's own (D-002: mrgl ships none), and `mrgl_core::keys` is the one place that
names, reads and stores them (task 093). The names are `MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY` and
`MRGL_CURSEFORGE_KEY`; WoWInterface takes none. A key is looked up in the environment first (a variable that
is empty or only spaces is not set), then in the key store: the Windows Credential Manager's generic
credential `mrgl:<source>` (`mrgl:github`, `mrgl:wago`, `mrgl:curse`; user name `mrgl`, the key as its
UTF-8 blob, `CRED_PERSIST_LOCAL_MACHINE`: this user on this machine), through `windows-sys`'s
`Win32_Security_Credentials` (`CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`), visible and removable
under Control Panel, Credential Manager, Windows Credentials. Outside Windows the store is not available:
reads find nothing and a write is refused (a key store there is M6). `MRGL_KEY_STORE=off` turns the store
off for a run the same way, and `MRGL_KEY_STORE=folder:<path>` keeps the keys as files under that folder
(`mrgl-<source>`, the key as the content), the tests' store, the mirror of `MRGL_HTTP_STUB`; any other value
is the Credential Manager.

`mrgl key set <source>` (`github`, `wago`, `curse`, a source's name, or the variable) reads the key from the
standard input, all of it from a pipe (`Get-Clipboard | mrgl key set wago`) or one line from a terminal,
trimmed, and refuses nothing, more than one word or line, or more than 2560 bytes (the Credential Manager's
limit); a key after the source on the command line is refused without being echoed, since the shell's
history would keep it. `mrgl key clear <source>` removes the entry (`nothing removed` when there was none),
and both warn when the variable is set too, since it comes first. `mrgl key list` names every key and its
origin, `set (environment)`, `set (credential store)` or `not set`, then a warning when the store is not
available and the count; `--json` is `kind: "keys"` with `api_keys[]` (`source`, `name`, `origin`),
`store` and `store_unavailable`, and `set` and `clear` answer `kind: "key"` with `action`, `source`,
`name`, `target`, `changed` and `warnings`. The doctor's `api keys:` line is the same listing.

A key's value lives in a `keys::Key`, whose `Debug` hides it, and no error, listing or JSON carries it. It
goes to its source's hosts only, over HTTPS (`keys::may_send`): the GitHub token to `api.github.com`
(never with an asset download), the Wago key to `addons.wago.io` (the external API and a download link
there, never the keyless data endpoints, task 095), the CurseForge key to
`api.curseforge.com` and `edge.forgecdn.net` in the `x-api-key` header (task 096; the CDN asks for it since
2026-07-16). A CurseForge request goes with the client's own redirects off (`Request::no_redirects`) and
the source follows a redirect itself, the key going along only to a next address on those two hosts, so a
hop of the CDN to another host never carries it. The unit tests run every store path against
`keys::MemoryStore`; the end-to-end tests set `MRGL_KEY_STORE=off` and remove the three variables, so no
test reads or writes the real Credential Manager, and they assert that a key given in the environment
appears in no output of `key`, `doctor` or `add`, text, JSON or error line.

## Adopt

`mrgl adopt --flavor <flavor>` (task 097, `mrgl_core::adopt`) says where the unmanaged addons of a flavor
come from, before anything is written and before any key exists; its counts measure the roadmap's M4 goal.
An addon folder the state sidecar does not know is unmanaged (D-003); a junction or link mrgl did not make
is left out with a warning. The sources of a folder come from all of its TOC files, the one the flavor
loads first, in this order: `## X-Wago-ID` (eight ASCII letters and digits, `toc::is_wago_id`),
`## X-WoWI-ID` and `## X-Curse-Project-ID` (a number, `toc::is_number_id`), `## X-Website` when it is a
github.com repository's address (`Toc::website`, read by `toc::github_repo_of_website`:
`github:<owner>/<repo>`, with or without `www.`, `.git` or a deeper path), then the curated GitHub mapping
by the folder's name ([The GitHub mapping](#the-github-mapping)). A header of another shape is no source
(its raw value stays in the typed field and in `Toc::headers`), and a website on another host is reported
as a page, never as a source. The TOC parser reads an unsubstituted packager token
(`## Version: @project-version@`, `toc::is_packager_token`) as no version, everywhere.

Each source is asked whether it publishes a build for the flavor, `Source::offers` through the router:
`yes`, `no` or `unknown`, with why. The trait's default resolves the spec (a pick by metadata `yes`, a guess
`unknown`, no release, no asset for the flavor or a 404 `no`, any other failure `unknown` with its line); the
GitHub source keeps it. WoWInterface answers from the compatibility list of its file list alone, read once a
day, with no request per project: `yes` for an entry of the flavor's game (Forever once the list names a
Forever version), `unknown` otherwise, never `no`. Wago answers from the keyless slug list of the flavor's
game version, with or without the key: `yes` when it names the id, `no` when it does not. A source the run
cannot ask (`Source::missing`) is `unknown` without a request; one whose key is not set reads
`needs <KEY NAME>` (`keys::name_of`), and so does CurseForge without `MRGL_CURSEFORGE_KEY`. With the key
(task 096) CurseForge answers from the project's `latestFilesIndexes` and its files of the flavor's game
version type: `no` when the project names no file of the type, `yes` with the file the channel takes (and a
note when its author allows no download outside CurseForge, so it installs from a zip in hand), `unknown`
for a guess of the game type.

The folders of one addon are one row, its folder group: the folders the source names (`Source::folders`:
WoWInterface's `UIDir` from the file list, CurseForge's `modules` of the file its answer for the flavor
took, task 096) when they include the folder, else the folders whose TOCs or
mapping name the same source, else the folder alone. A folder joins one group only, never a managed or a
linked one, and the folders with a source are grouped before the ones without, so an options folder without
a header joins the group its addon's source names. The row's name is the install engine's
(`install::derive_name`), its version the `## Version` of that folder's TOC. The source a row is adopted as
is the first, in the order above, that the run can ask and that no managed addon has as its source already,
with `yes`, else the first such with `unknown`. A row's class: `without-curseforge` when a source other than
CurseForge does not say `no`, `only-curseforge` when only CurseForge does not, `no-build` when every source
says `no`, `no-source` when there is none.

The command prints a line per addon: its name, its version or `(no version)`, its folders when there are
several, and `adopt as <spec>`, `needs <KEY NAME>`, `only with CurseForge, needs MRGL_CURSEFORGE_KEY`,
`no <flavor> build at its sources` or `no known source`. Under it a line per source,
`  <spec> from <hint>: <flavor> build <yes|no|unknown> (<why>)`, with `; needs <KEY NAME>` when the key is
missing, or `needs <KEY NAME>` alone when the source could not be asked, and a page as
`  page <url> from X-Website, not a source mrgl reads`. Then a warning when the flavor folder has the
CurseForge app's `.curseclient` marker, and the summary: `<n> unmanaged addons in <folder>: <a> can be
adopted without CurseForge, <b> only with CurseForge, <c> with no known source`, with
`, <d> with no <flavor> build at their sources` when there are such and `; adopt --apply adopts <k> of them`
when there are such. `--json` is `kind: "adopt"` with `flavor`, `curse_client`, `apply`, `addons[]` (`name`,
`folders`, `version`, `sources[]` with `spec`, `source`, `from`, `build`, `why`, `needs`, `missing` and
`taken_by`, `pages`, `class`, `adopt_as`, `needs`), `left_out`, `counts` (`addons`, `without_curseforge`,
`only_curseforge`, `no_build`, `no_source`, `adoptable_now`), `adopted[]` and `skipped[]` (empty without
`--apply`) and `warnings`. Without `--apply` the report reads the sidecar and writes nothing; the sources
keep their own caches.

`--apply` (task 098, `adopt::apply`) writes one sidecar entry per row that has a source to be adopted as: the
spec, the TOC's `## Version` (`unknown` when it has none or an unsubstituted token), the folder group, the
SHA-256 of every file in it as it is now (`<Folder>/<path>`, as the install engine records them; a link
inside a folder is not followed), no previous version, no pin, no mark, the stable channel. Nothing is
downloaded and no folder changes. The next `update` resolves the entry like any other: a source's version
that differs from the TOC's (`unknown` always does) is an update, and installing it is the engine's one path
(`Engine::install_from_source`): the source's build is swapped in over the group, and a folder of the group
the build does not have is removed as a folder of the previous version. A source whose version equals the
TOC's reads `up to date` and installs nothing. A row the run cannot write (a key it lacks, CurseForge only)
is a warning, `<name> not adopted: <why>`, never an entry: a CurseForge-only addon waits for the CurseForge
key (decided 2026-10-22; the source is there since task 096), and a Wago addon for the key; a row without a source is no line. A name or a
folder the sidecar knows already is skipped too, so the adopted folders are managed from then on and a
second `--apply` writes nothing (the sidecar is written only when an entry was added). The lines are the
report's, then `adopted <name> <version> as <spec> (<n> folders: …)` per entry, the warnings and the
summary, the counts with `; adopted <n> addons into the sidecar, no download and no folder changed: the next
update installs each from its source` or `; nothing adopted, the sidecar is unchanged`.

A folder belongs to one manager. A flavor folder with the CurseForge app's `.curseclient` marker refuses
`--apply` before any source is asked (`adopt::marker_refusal`, exit 1, nothing written) unless
`--alongside-curseforge` (which needs `--apply`) says so; then the entries are written and a warning says
that the CurseForge app may still update or flag those folders and that its marker stays
(`adopt::alongside_warning`). mrgl never removes the marker. Every install path warns in such a folder too:
`add` (a zip or a source, not the dry run of a source, which reads no folder), `import` and `update` add
`adopt::curse_client_warning` to their warnings, on the screen and in the JSON, and so do the window's
update actions (task 110: "Check for updates", "Update all" and a row's "Update", once per run, below).

The window's toolbox has the report and the apply in its Adopt card (task 109, the commands `adopt` and
`adopt_apply` under [The UI](#the-ui)): the same `adopt::report` and `adopt::apply` over the same sidecar,
the same lines, and the marker guard with a confirmation dialog in place of `--alongside-curseforge`. The
summary, the marker line and the refusal name the window's Adopt button and its confirmation instead of the
flags (`adopt::AdoptBy::Window`, task 112): `; the Adopt button adopts <k> of them`, `so the Adopt button
writes nothing there without the confirmation to adopt alongside the CurseForge app`. The card applies the
report it shows (task 113): the report records its sources' answers (`adopt::Answers`: the flavor, when the
asking began, per spec its build answer with what the run lacked then, and the folders it names), through
`adopt::Answering`, a source that answers `offers` and `folders` from the answers it holds and asks the
source under it for the rest (`adopt::report_answering`); the card hands them back with the apply, and the
apply and the report after it are built over the folders and the sidecar as they are then and those
answers, asking no source again. A spec the answers lack (a folder that arrived since), or one whose
`missing` changed (a key set since), is asked. The answers stand for an hour (`adopt::ANSWERS_MAX_AGE_SECONDS`,
the shortest cache rule of the sources: WoWInterface's project answer, `Cache-Control: max-age=3600`);
older ones, another flavor's or none are asked again first. The command line reads once and writes, as
before.

### The GitHub mapping

`data/github-sources.json` (task 097) is compiled into the core with `include_str!`
(`adopt::MAPPING_JSON`), like the Forever notes: no file to ship and none to read at run time. It names the
source of an addon whose TOC names none, by its folder:

```json
{
  "schema": 1,
  "about": "what the file is and how an entry joins",
  "folders": {
    "<addon folder>": "github:<owner>/<repo>"
  }
}
```

Every folder of a multi-folder addon is a key of its own with the same source, which makes them one folder
group. An entry joins by a pull request, and only for a repository whose releases carry the addon's zip: a
repository without release zips is no source mrgl can install from. The schema check (`adopt::tests`,
`GithubMapping::problems`) rejects an unknown field, a schema other than 1, a key that is no folder name (a
path separator, surrounding space) or repeats another without regard to case, and a value that is not a
canonical `github:<owner>/<repo>` spec, and it holds the seed: every folder of the Forever starter list
(`lists/forever-starter.json`) maps to that list's source.

## The UI

The UI is plain HTML, CSS and JavaScript embedded in the exe: no Node toolchain, no bundler, no Tauri CLI,
no CDN (the content security policy in `tauri.conf.json` allows the embedded files only, so there is no inline
script or style either). A change under `crates/mrgl-app/ui/` takes effect on the next `cargo run`. Pure
modules get Node tests under `crates/mrgl-app/tests/ui/`, which `cargo test` runs through `tests/ui_smoke.rs`
when `node` is on `PATH`.

The files under `ui/` (the layout is task 017's sketch, drawn before the shell):

- `index.html` is the one page: a sidebar with the flavor navigation (the four flavors by their friendly
  names, in the order retail, classic, era, forever, each with its addon count and its count of updates; the
  window opens on Forever), the chosen flavor's header (name, folder, build, the CurseForge marker, the search
  field, "Check for updates" and "Update all", with "Cancel" in its place while a run goes; task 010), the
  addon list (one row per addon: name and folder, version and the version an update brings with the note under
  it, the source badge, the state pill with the interface mark under it (task 081: `interface mismatch` or
  `out of date`, as `scan` and `list` mark the line, the warning's line in its title) and the step a run is
  at, the Forever note slot of task 011, and a row
  menu with "Update", pin or unpin, ignore or unignore and the channel, with a progress slot per row; tasks
  010 and 021), a legend, a status bar and the toast stack. Under the header, two tabs: "Addons" (the list
  above) and "Toolbox" (task 074), the flavor's toolbox in three cards: "Addon sets" (the set picker with each
  set's enabled and disabled count, "Apply to" with every character or one character by name and realm, and
  by account too when two accounts have the realm and the name (task 085), the apply button, the undo button
  named after the set of the last apply with the line under it that says which `AddOns.txt` files it puts
  back and which it removes (task 085), the set's addons as chips, enabled first, a dashed chip for an addon
  with no folder, each chip a toggle between enabled and disabled in the set with a `×` that drops the addon
  (task 085: `sets.json` only), and what the last apply changed or the last undo put back, one line per
  character), "SavedVariables" (the listing's summary, an "orphans only" filter, the table of addon,
  place, files, size and status, the largest first, and the cleanup button that names the orphans it moves)
  and "Adopt" (task 109: "Ask the sources" reads the adopt report on demand, since it asks every source of
  every unmanaged addon; then the report's summary, the CurseForge marker's warning line, a table row per
  unmanaged addon with its folders, version, the core's line per source and page, and its verdict after a
  pill of its class, and the button that adopts the addons the report can adopt, named with their count;
  in a folder the CurseForge app manages a `<dialog>` shows the marker's line first, and only "Adopt
  alongside the CurseForge app" sends the apply; the `adopted …` lines of the last apply under the table).
  A warning line over the cards names the running client; the buttons stay enabled, since the core's guard
  answers the apply, the undo and the cleanup (an edit is not refused). The toolbox is read when the tab
  opens and again after every write; the adopt report when its button is pressed, and again by an apply.
  The theme
  toggle and the serious switch sit at the bottom of the sidebar, with "Game folder…" and "Copy
  diagnostics" (task 009). The theme and the switch are kept by the shell in `config.json`, not in the web
  view's storage (task 034, below).
- `style.css` holds every color as a token on `:root`: the light set, the dark set under
  `prefers-color-scheme: dark` (unless the toggle says light) and under `data-theme="dark"` (the toggle), one
  token per state (managed, update available, linked, guess only, unmanaged), per interface mark
  (interface mismatch, out of date; task 081) and per source badge (github, wago, wowi, curseforge, zip,
  linked; a dashed badge is a source known only from a TOC hint).
- `main.js` is the view: it reads `window.Data`, draws what it gets and never composes a murloc line. It does
  not know whether the rows are made up.
- `data.js` is the window's data module (task 009): the contract behind `window.Data`, each call one Tauri
  command of `src/commands.rs` (below). `index.html` loads it and nothing else for the data.
- `mock-data.js` is the data module of the sketch, made-up rows in the shape the core answers with; its header
  comment is the contract (`Data.version()`, `Data.flavors()`, `Data.chooseRoot()`, `Data.diagnostics()`,
  `Data.resolve(flavorId, folder)`, `Data.checkUpdates(flavorId, progress)`, `Data.updateAll(flavorId,
  progress)`, `Data.updateOne(flavorId, folder, progress)`, `Data.cancelUpdate()`, `Data.mark(flavorId,
  folder, action, value)`, the preferences, `Data.toolbox(flavorId)`, `Data.applySet(flavorId, set,
  target)`, `Data.undoSet(flavorId)`, `Data.editSet(flavorId, set, addon, action)`,
  `Data.cleanSavedVariables(flavorId)`, `Data.adopt(flavorId)`, `Data.adoptApply(flavorId, alongside,
  answers)`; a
  flavor as `flavor.rs` serializes it plus its friendly name, the
  source of its build, whether its `Interface\AddOns` folder is there and its addons; an addon as `scan.rs`
  gives it plus the facts of the sidecar, the sources, the update check, the Forever notes, pin / ignore /
  channel, the missing dependencies and the interface check; a progress event of a run; a toolbox answer;
  an adopt answer).
  For a look at the page without the core, open `crates/mrgl-app/ui/index.html` in a browser: without the
  Tauri global, `data.js` writes the mock's script tag instead, and the made-up rows show (retail with one row
  of each interface mark), with a made-up check (task 107: the sketch's build asks GitHub, WoWInterface and
  Wago, its key set, and a managed CurseForge row is not checked, with the core's line `source not
  configured: curse:<id> (this build has no CurseForge source)`), update, cancel and marks, and a made-up
  toolbox per flavor
  with one realm and name under two made-up accounts and a made-up undo and edit, whose client "runs" in
  retail, so the refusal of the `WTF` writes shows there, a made-up adopt report per flavor (made-up
  sources for its unmanaged rows, a row of every class, the marker's guard and dialog in the three flavors
  the CurseForge app manages) and the marker's warning first in their update runs (the folder dialog and
  the clipboard need the window). A browser blocks some scripts of a page opened
  from the disk; `python -I -m http.server --directory crates/mrgl-app/ui` serves the same files.
- The pure modules, each with a test file of the same name under `tests/ui/`: `state.js` (an addon's one state
  from its facts, with the precedence linked, update, managed, guess, unmanaged; the classes and labels; the
  interface mark of a row's `interface_check`, task 081),
  `search.js` (every word of the query in the name, folder, source kind or source id), `preferences.js`
  (the window's preferences from an answer of the shell: the theme, the switch, the switch locked when
  `MRGL_SERIOUS` decides it, and the one-time carry-over of a value the sketch kept in `localStorage`),
  `theme.js` (auto, light, dark), `flavors.js` (the navigation order and the friendly names), `actions.js`
  (task 010: a row menu's items and why one is disabled, for a row the check did not ask the check's own
  detail, the core's `source not configured: <spec> (<what is missing>)` line, task 107; the progress slot
  each event of a run moves, a
  check's answer laid over its row, the note under a row's version, whose pin reads the check's `order` as
  the command line's pinned line does: `pinned at 1.1.0-beta, newer than the release`, or `pinned at 2.0.0,
  2.1.0 is out`; task 077; the check states the note names, `NOTED`: `installed-newer`, `guess-only`,
  `pending` of task 104, `distribution-off` of task 115, `source-gone`, `failed` and `rate-limited`, each
  by the core's label), `toolbox.js`
  (tasks 074 and 085: the target
  picker's options and the `{ character, realm, account }` an option names, the account in an option's
  value and label only when the answer carries one, the set the picker shows, the set's addons in their
  order and each chip's toggle and drop, the state, label and title of the apply, undo and cleanup buttons,
  the line that says what the undo puts back, the SavedVariables rows with the "orphans only" filter, the
  words of a status, and the lines of what an apply changed and an undo put back; task 109: the adopt
  card's rows, the words of a row's class, the ask button and the adopt button with its count, why it
  waits and whether the dialog asks first) and
  `voice.js` (the two lines and where a message goes, see [nerglish.md](nerglish.md), "The window").

The IPC commands (`src/commands.rs`, task 009) are thin wrappers over the core and over the plain functions
of `src/list.rs`, `src/actions.rs` and `src/toolbox.rs`, which take the game root, the mrgl home folder, the
engine and the source as arguments, so `tests/flavor_list.rs` checks the rows on a made-up game folder,
`tests/update_actions.rs` the update runs and the marks on a made-up flavor with a stub source, and
`tests/toolbox.rs` the toolbox on a made-up flavor with the `WTF` tree of the addon-sets fixture, and
`tests/adopt.rs` the adopt card over a copy of the adopt fixture with the source stubs of tasks 094 and 095
(the apply over the shown report's answers counts the stub's requests: one round for the report, none for
the apply and the report after it, task 113), without a window. Every message they return is
`{ kind, murloc, plain }`: the core's voice kind (`Kind::as_str`) picks the place in the window, and a
serious voice sends the murloc line empty. Whatever reads the disk or the network runs on a blocking thread.

- `app_version`: the version for the status bar.
- `flavors`: the game root as every CLI command resolves it (`MRGL_ROOT`, the remembered root, then the
  discovery; a discovered root is used and not remembered), its flavor folders and their rows, and a banner.
  A row joins the scan (folder, the TOC's title without color escapes, a junction), the flavor's state
  sidecar (managed, the installed version, the source, pinned, ignored), a TOC hint for an unmanaged addon
  (Wago, then WoWInterface, then CurseForge, drawn as a guess), the dependency check (`missing`), the
  interface check of the TOC the flavor loads against its build (`interface_check`, task 081: `null` when
  current or when nothing can be compared, else `state` `mismatch` or `out-of-date` from
  `interface::check_addon` and `interface::out_of_date_addon`, `label` the mark `scan` and `list` append to
  the line, `interface mismatch` or `out of date`, and `line` the warning's plain line), and on
  every row of the Forever flavor its Forever status (`build`, `note`, `verified`, `source`, `stale`). The
  list never goes online (task 010): a row's `check` (`state`, `label`, `latest`, `order`, `detail` and the
  check's data `line`) and its `update` are the answer of `update::check`, with its version order (task 068), run
  against the run's sources (`Sources::cached`, task 092) over an `Http` that fails every request as an
  unreachable network does, so the GitHub source answers from the release list and `release.json` that the
  last check or install cached (its
  offline rule, task 041), counts no request against the rate limit and never stamps the cache as
  confirmed. `update` is set for `update-available` only, so a newer installed version, a pin or a guess is no
  update; an addon whose answer needs the network (nothing cached) has no `check`. What the cache cannot
  tell again is the last online check's (task 078): a check of the window, and `mrgl update --all` or
  `mrgl update <name>` on the command line (task 087), keeps every addon's answer of `source-gone`,
  `failed`, `rate-limited` or `distribution-off` (task 115: the CurseForge source keeps no cache and the
  offline view has no key, so only the kept answer tells it) in `last-check.json` under the flavor's folder of the mrgl home
  (`mrgl_core::last_check::record_check`, one rule for both writers), and every list lays those answers
  over the offline ones, so a redraw after a
  mark and a restart show them, until the next check of the addon replaces its answer (dropped for any
  other state). The file is a cache: a missing, unreadable or foreign one is no answer. An answer holds
  while the sidecar entry keeps the source and the version it was given for, so an update or a reinstall
  (by the command line too) drops it; an ignored addon shows its ignore instead. The Forever `build` is
  the source's `cached_forever_build` on the addon's channel (`unknown` from a source that does not name
  Forever, task 092): for GitHub the release the resolve would take, not the
  channel's newest one (task 078). A row also carries its
  `channel` (`null` when unmanaged), and a flavor `addons_folder`, false when the client was never started
  for it.
- `choose_root`: the folder dialog (`tauri-plugin-dialog`, called from Rust); a game folder, or a flavor
  folder whose parent is one, is remembered in `config.json` as `mrgl root <folder>` does and answered with
  its flavors; a cancelled dialog answers nothing.
- `diagnostics`: the doctor's report (`mrgl_core::doctor`, see [The doctor](#the-doctor)) as the text
  `mrgl doctor` prints, with the message for a copy that worked and one for a copy the clipboard refused;
  the view writes it with `navigator.clipboard` (a selection copy as the fallback), so no clipboard plugin
  joins.
- `resolve`: a managed addon through its source, for the flavor's build and on its channel: the source's
  own `Pick`.
- `check_updates`, `update_all` and `update_one` (task 010, `actions::run`): the update check of a flavor's
  managed addons (or of the one a row's folder names) through `update::check_until`, which asks the sources
  (`MRGL_HTTP_STUB` answers for them as on the command line), then, for `update_all` and `update_one`, every
  `update-available` addon through the one install path (`Engine::install_from_source` with the check's own
  pick, task 055). Each takes a Tauri channel (`tauri::ipc::Channel`) and sends one event per step as it
  happens: `{ phase, folder, index, total, check, message }`, the phase `checking`, `checked` (with the
  addon's `check`), `stopped` (the request budget's stop line of task 068, a warning, sent once per source
  that stopped, task 101, and not repeated in the answer), `updating`, `downloading`, `installing`, `updated`, `installed-already`,
  `folded` or `failed` (an error), the folder the addon's first one, the message in the voice. `installed-already`
  (task 099) is the install path's "installed already" answer in the command line's words of task 089
  (`AlreadyInstalled::update_line` of the core, which both say): the nothing-to-do line under the addon's
  own entry, else the warning that names the second entry of the same source (`OneFolder not updated:
  OldOneFolder, a second entry of github:Example/Twice, has 1.1.0 installed already in _forever_; OneFolder
  stays at 1.0.0`); the summary counts it as `1 installed already`, not as updated, and the row of an addon
  another entry stood in for keeps the check's answer. `folded` (task 114) is an addon whose entry an
  install earlier in the run dropped, since that install's zip carried every folder of it (task 108,
  `Installed::folded`): nothing is installed and nothing more is asked, the event says the core's line
  (`Folded::update_line`: `SeveralFolders_Options folded into SeveralFolders: SeveralFolders 2.1.0 ships its
  folder SeveralFolders_Options in _forever_, SeveralFolders_Options has no entry of its own now`), the
  summary counts it as `1 folded into another entry`, and its row reads from the disk again, a folder of
  the addon it joined. The answer is `{ flavor, messages }`: the flavor as
  it is now (its rows read again, with the check's answer laid over them except for the addons this run
  updated; the answers the cache cannot give again are kept in `last-check.json` for the next list, task
  078, and a file that cannot be written is a warning) and the warnings, then the summary, `UpdateCheck::summary` of the core, which the command line
  shares (`checked _forever_: 1 update available, 2 up to date`, `2 addons updated, 1 failed, 1 up to
  date`). After each install the run reports the checks the command line's install summary reports
  (task 079, `Engine::check_installed`, shared with it): a missing required dependency of the installed
  folders (`NeedsLibrary needs the missing dependency LibMissing: install it or the addon does not load`)
  and an interface of the primary folder that is not for the build (`… of _forever_; the zip may be for
  another flavor`), each a warning of the answer; the row, read again, carries the dependency in its
  `missing`. In a flavor folder with the CurseForge app's `.curseclient` marker the first warning of the
  answer is the core's `adopt::curse_client_warning` (task 110), once per run, as `mrgl update` warns
  there; the window has no add or import of its own, so its update actions are its install paths. One
  run at a time: a second one is refused with a warning. A flavor without an AddOns folder
  answers that the client has not been started for it yet. The window's "Check for updates" is the check
  alone; "Update all" and a row's "Update" run the check first, so they never install on an old answer.
- `cancel_update`: asks the run going to stop. The cancel rule: the flag is read before each addon of the
  check and before each install, never during one; an install that has started runs to its end through the
  engine's stage, swap and revert (task 004), so a cancel never leaves an addon half-written. A cancelled run
  answers what it did (`1 addon updated, 2 not updated (cancelled), …`, or `update cancelled after checking
  3 of 12 addons in _forever_; nothing updated`).
- `mark` (task 010 over task 021): `pin` (at the given version, else the installed one), `unpin`, `ignore`,
  `unignore` or `channel` (`stable`, `prerelease`) on a managed addon by its folder, through
  `Engine::apply_mark` and the line of `Mark::line`, both shared with the command line; the answer is the
  flavor as it is now and the line. Refused while an update runs, since both write the flavor's sidecar,
  and while a toolbox write goes (task 109: an adopt writes the sidecar too).
- `preferences` and `set_preferences` (task 034): the theme (`auto`, `light`, `dark`) and the serious
  switch as they apply, which source decided the switch (`environment` locks the window's switch: the
  window cannot override `MRGL_SERIOUS`), and what `config.json` stores (`theme`, `serious`, next to
  `root`). The view reads them at start, writes a toggle's change at once, and carries an old
  `localStorage` value over once for a key the file does not have, then removes the old keys; a cleared
  web view draws what the file says. The native window follows the theme too (task 079), so the title bar
  matches the page: at start (the `setup` hook of `lib.rs`) and on every `set_preferences`, `light` and
  `dark` set the window's theme (`WebviewWindow::set_theme`) and `auto` leaves the system's
  (`commands::native_theme`); a light page on a dark system has a light title bar. Each writer of `config.json` sets only its own keys and keeps the
  others, so `mrgl root` keeps the preferences and the window keeps the root.
- `toolbox` (task 074, `toolbox::load`): a flavor's toolbox, read only. Its sets from
  `<home>\<flavor-id>\sets.json` (`sets::read_sets_at`: each set's name, save time, enabled and disabled
  counts and addons, each with its state and whether a folder of its name is under `Interface\AddOns`) with
  the note under the picker (the core's `SetsFile::summary`, the line `mrgl set list` prints: `2 sets of
  _forever_ in …`, or that there is none yet, with the hint to `mrgl set save <name>`; saving a set stays on
  the command line), its characters (`sets::characters`: name,
  realm, the `<realm>/<character>` label and `account`; the account folder's name stays out of the answer,
  as out of the command line's plain lines, unless another account has a character of the same realm and
  name, task 085), `undo` (task 085, `sets::undo_record`: the last apply that changed a file, its set, when
  it ran, the characters whose `AddOns.txt` an undo puts back and those whose file the apply created; null
  when there is nothing to undo), its SavedVariables (`savedvars::list_saved_variables`: per entry the
  addon, the place, the `.lua` and `.bak` flags, the size in bytes and as `size_text` writes it, and the
  status `installed`, `orphan`, `client` or `unknown`; the totals and the summary `mrgl savedvars` prints,
  the core's `Listing::summary` with the cleanup named instead of `--clean`), and `client`, a warning with
  the guard's refusal line while the client runs from the flavor folder (task 023). A part that cannot be
  read is empty, with its error among the messages. The answer is `{ toolbox, changes, undone, zip,
  messages }`, the same for the writes.
- `apply_set` (task 074 over tasks 014 and 027, `toolbox::apply`): a stored set into the `AddOns.txt` of
  every character, or of the one `character` (and `realm`, and `account` when the realm and the name sit
  under two accounts, task 085) names. The running-client guard (`client::ensure_client_not_running`) comes
  first and its refusal is the answer (an error, nothing written); then the pick is pinned to one character
  through `sets::find_character_in_account` (an unknown or ambiguous name is the core's error), and
  `sets::apply_set_in_account` with its `Target` writes only the files that change. The answer carries the
  toolbox as it is now (its `undo` now this apply), `changes` (per character the lines it changed or added,
  and whether the file is new), the not-installed warning and the summary of `mrgl set apply` (the core's
  `Applied::not_installed_warning` and `Applied::summary`: `applied set raid to Second Realm/Grrlwave of _forever_: 3 addon lines changed`, `set
  raid already matches every character of _forever_ (2 characters), nothing written`).
- `undo_set` (task 085 over task 073, `toolbox::undo`): the guard first, then `sets::undo_apply`, which puts
  every `AddOns.txt` the last apply replaced back byte for byte and removes the ones it created. The answer
  carries the toolbox as it is now (`undo` null), `undone` (the set, when it was applied, the characters
  whose file is back and those whose file is gone) and the warnings and the summary of `mrgl set undo` (the
  core's `Undone::summary`: `undid the apply of set solo to _forever_ (…): 1 AddOns.txt put back, 1
  AddOns.txt the apply created removed`); a second undo is the core's `nothing to undo in _forever_: …`.
- `edit_set` (task 085 over task 073, `toolbox::edit`): `action` `enable`, `disable` or `drop` of one
  `addon` in a stored `set`, through `sets::edit_set`, in `sets.json` only. No `AddOns.txt` is touched, so,
  as `mrgl set enable|disable|drop`, it is not behind the running-client guard. The answer carries the
  toolbox as it is now and the not-installed warning and the summary of the command line (the core's
  `Edited::not_installed` and `Edited::summary`: `RaidFrames is disabled in set raid of _forever_ now, was
  enabled: 3 addons (1 enabled, 2 disabled); no AddOns.txt was touched`).
- `clean_saved_variables` (task 074 over task 025, `toolbox::clean`): the guard first, then
  `savedvars::clean_orphans` into the flavor's backups folder under the mrgl home, which moves the orphans
  and their `.bak` files into `SavedVariables-<date>.zip` and removes a file only once its copy in the zip
  reads back equal. The answer carries the toolbox as it is now, `zip`, and the summary of `mrgl savedvars
  --clean` (the core's `Cleaned::summary`: `moved 2 orphans of _forever_ (3 files, 2.5 MB) into …; unzip it
  into the WTF folder to put them back`, or `no orphans …, nothing moved`). Refused with a warning while an
  update runs: an install swaps addon folders, and a folder away for a moment would read as an orphan; an
  update run waits for a toolbox write the same way. One toolbox write at a time (`toolbox::ToolboxRun`):
  an apply, an undo, an edit, an adopt or a cleanup; a second one is refused with a warning.
- `adopt` (task 109 over task 097, `toolbox::adopt_report`): the adopt report of a flavor, `adopt::report`
  over its scan and its sidecar (the entries whose folders are gone dropped first, as `mrgl adopt` reads it)
  through the router (`MRGL_HTTP_STUB` answers for it as on the command line) and the key lookup of
  `keys::of_process`, so a missing key reads `needs <KEY NAME>`. It writes nothing, and runs while an update
  or a toolbox write goes. The answer is `{ adopt, flavor, adopted, messages }`: `adopt` the report as the
  card draws it (`flavor`, `folder`, `curse_client`, `marker` the warning of `Report::marker_line` or null,
  `rows` with the core's row as `mrgl adopt --json` writes it plus its `line`, `verdict` and `details` (the
  lines under it, without the indent), `left_out`, `counts`, `summary`, `Report::summary`, and `answers`,
  the sources' answers the report was built from, task 113: `flavor`, `asked_at`, `offers` per spec with
  `build`, `why` and `missing`, `folders` per spec), and the messages: the warnings of the scan, the
  sidecar and the left-out folders, then the summary.
- `adopt_apply` (task 109 over task 098, `toolbox::adopt_apply`): `alongside` is the dialog's confirmation,
  `answers` the `adopt.answers` of the report the card shows (task 113; `toolbox::AdoptRequest` carries
  both). In a flavor folder with the `.curseclient` marker and without the confirmation, the answer is the
  refusal of `adopt::marker_refusal` (an error) before any source is asked, and nothing is written;
  otherwise the report again over the folders and the sidecar as they are now and the shown answers
  (`adopt::report_answering`: no source is asked while the answers stand, an hour; older ones, another
  flavor's or none are asked again first), `adopt::apply` into the sidecar, written once when an entry was
  added. The answer carries the report as it is now, over the written sidecar and the same answers (the
  adopted addons are managed, so they are no rows any more), `flavor` with its
  rows as they are now, `adopted` (the `Adopted::line` of each entry) and the messages of `mrgl adopt
  --apply`: `adopt::alongside_warning` in a marked folder, the warnings, a `<name> not adopted: <why>` per
  skipped addon, and `Report::applied_summary`. Refused while an update runs (both write the sidecar) and
  while another toolbox write goes; an update and a mark wait for it the same way. Without the mrgl home
  folder both commands answer an error and read nothing.

The application icon is a murloc head of our own drawing (task 012; fan art within D-005, no Blizzard
artwork, no game file): geometry in `scripts\make-icon.py`, rendered with Pillow and committed. `icon.ico`
holds every size the Windows shell asks for (16, 20, 24, 32, 40, 48, 64, 96, 128 and 256 px, each drawn on its
own, with a thicker outline and fewer details below 32 px so the face reads at 16), `icon.png` (512 px) is the
window icon on the other targets. Its colors are the theme tokens of `style.css`, so one drawing serves the
light and the dark theme. `tauri-build` embeds the `.ico` in `mrgl-app.exe` (the title bar and the taskbar take
it from there), the CLI's `build.rs` embeds the same file in `mrgl.exe`, and both build scripts rerun when it
changes. To change the drawing, edit the script and regenerate both files:

```powershell
uv run --with pillow python -I scripts\make-icon.py   # crates\mrgl-app\icons\icon.ico and icon.png
```

## CI and releases

`.github/workflows/ci.yml` runs seven jobs on every push to `main` and every pull request: `test`
(windows-latest: `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`), `lint` (ubuntu:
`cargo fmt --check` and `scripts/changelog.py --check`, the latter skipped on a tag, see
[The changelog](#the-changelog)), `msrv` (windows-latest: `cargo check` with the toolchain that `rust-version`
in `Cargo.toml` names, so nothing newer than the declared minimum slips in; it runs on Windows because the
Windows target needs tauri-build's resource compiler, which a Linux runner lacks), `non-windows` (ubuntu:
`cargo check --all-targets --locked` over the workspace, below), `deny` (`cargo deny check`
against `deny.toml` for the Windows target: licenses from the allowlist, no RustSec advisory or yanked crate,
crates.io as the only source; and `about.toml` accepts the same licenses) and `workflow-lint` (zizmor over the
workflows, the local action and `dependabot.yml`; a finding accepted on purpose carries
`# zizmor: ignore[<audit>]` on the name line of its step and its reason in the comment above, so the
acceptance moves with the step and covers nothing else) and `coverage` (windows-latest: the tests once more
under cargo-llvm-cov on a pinned nightly, the table in the job summary, red below the floors, see
[Coverage](#coverage)). `.github/workflows/commit-rules.yml` runs
`commit-rules` (`scripts/check-commit.sh` over every pushed commit and the pull request text, the rules of
[git-workflow.md](git-workflow.md)) on the same pushes and pull requests. `.github/workflows/advisories.yml`
runs `cargo deny check advisories` every Monday and by hand (`gh workflow run advisories.yml`), so a RustSec
advisory for a crate already in `Cargo.lock` shows up between pull requests; it is not a required check.
`.github/workflows/sync-issues.yml` runs `scripts/sync-issues.py` on every push to `main` that touches `tasks/`
or the script (and by hand, `gh workflow run sync-issues.yml`): it mirrors the task files into GitHub, one way
(see [The issue mirror](#the-issue-mirror)); it is not a required check either. The workflows run with a
read-only token by default; only the release job may write contents, and only the `sync` job may write issues.
Cargo runs with `--locked` in CI and in the release script, so a `Cargo.lock` out of step with the manifests
fails the build instead of being rewritten.

The repo is public from its first commit (D-004), so the Actions minutes of the standard runners are free and
the Windows jobs run on every change. A macOS job joins with the macOS preview (M6).

The `non-windows` job (task 061) compiles the `cfg(not(windows))` branches (the junctions of `link`, the process
list of `client`, the launcher lookup of `discover`, the time zone of `backup`, and the tests that only build off
Windows), so a broken non-Windows path fails the pull request that breaks it instead of turning up at the macOS
preview; a Windows machine never compiles them. It checks and does not test: the fixtures are shaped for Windows
paths. The window's crate is in the check too: the job installs the WebKitGTK and D-Bus headers its Linux graph
builds against (`libwebkit2gtk-4.1-dev`, `libdbus-1-dev`), which took 35 seconds of a run of about two minutes
when the job landed; if that install ever outgrows a minute, the check narrows to `-p mrgl-core -p mrgl-cli`.

Some of the setup lives in the repository settings, not in a file: CodeQL runs as GitHub's default setup; a
ruleset protects `main` (changes arrive through a pull request with squash merge only, the checks `test`,
`lint`, `msrv`, `non-windows`, `deny`, `workflow-lint`, `coverage` and `commit-rules` must pass, the history stays linear, and force
pushes and deletion are blocked); another ruleset keeps `v*` tags from being moved or deleted; squash merge
is the only merge method, with the pull request title and description as the default commit message; private
vulnerability reporting is on (the link in SECURITY.md and the code of conduct depends on it); Dependabot
security updates are on.

Every action is pinned by commit SHA with the version in a comment. Dependabot proposes the updates for the
actions and for `Cargo.lock` once a week, a release only after it is a week old; a pin is changed through such a
pull request, never by retagging by hand. Before a change to the dependencies or the workflows, the same
checks run locally:

```powershell
cargo install cargo-deny --locked   # once
cargo deny check
uvx zizmor .github
```

### The Rust toolchain

`rust-toolchain.toml` pins one stable version with `rustfmt` and `clippy`, and every build uses it: rustup installs
it on the first `cargo` call in the repo, and in CI the local action `.github/actions/install-rust-toolchain`
installs it with the runner's own rustup (no third-party action). The pin keeps a new Rust release and its new
clippy lints from turning CI red without a code change; dependabot does not watch the file, so the bump is a
deliberate pull request of its own:

1. Set `channel` to the new stable version (`rustup check` names it) and run the checks of CONTRIBUTING.md,
   "Checks before a commit".
2. Fix what the new lints find in the same pull request, titled `chore: bump the Rust toolchain to <version>`.

`rust-version` in `Cargo.toml` (the oldest supported Rust, which the `msrv` job checks) is a separate number and
does not move with the pin.

### Coverage

The `coverage` job (task 117) measures how much of the three crates the tests run: line and branch coverage
of the Windows build, with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) on a nightly toolchain,
since branch coverage is nightly-only. The test files, the build scripts and the `cfg(not(windows))` twins (which
the `non-windows` job compiles and nothing runs) are not counted, nor are the test helpers of
`crates/mrgl-test-support`, which carry the exclusion attribute below. The whole suite runs once more under
instrumentation; the exe the command line's end-to-end tests spawn writes its own profile, so its lines count
like any other, and the core's lines the command line alone reaches count only in a workspace run, never in a
run of the core crate by itself. A process that ends through `std::process::exit` writes no profile (the C
runtime's exit handlers, one of which writes it, do not run), so the command line's quiet exit on a closed pipe
writes it first under `cfg(coverage)`, the cfg cargo-llvm-cov sets on every measured build; clap's own
usage-error exits write none, which is why the argument parsers have unit tests. Two pins, each the one place its number is written, read by the job and by the commands of
CONTRIBUTING.md, "Checks before a commit": `scripts/coverage-toolchain.txt` (the nightly by date) and
`scripts/cargo-llvm-cov-version.txt`. Dependabot sees neither; a bump is a hand edit in a `chore: bump the
coverage nightly to <date>` pull request that re-measures (a nightly can change the branch count without a
code change) and adjusts the floors with a sentence why.

`scripts/coverage-check.py` reads the job's lcov export, prints the table (per crate, then per file by missed
lines) into the job summary, and fails below its two floors, `LINES_FLOOR` and `BRANCHES_FLOOR` at its top: the
one place they are written. They rose with every pull request of task 117 and stand at 100 since its close, so
a pull request that leaves a line or a branch untested is red before the merge. The lcov export, not the JSON summary, is the measure: the summary counts every
instantiation of a generic function on its own, so a branch one instantiation never takes shows as missed there
while the text and HTML reports, which merge the instantiations, show it covered; the lcov lines and branches
are that merged view, the one a reader of the report can act on.

What no test can reach is excluded in the code, never by a file pattern. There are two forms. The item carries
`#[cfg_attr(coverage_nightly, coverage(off))]` under one comment line `// coverage: off — <reason>` (further
comment lines continue the reason), and the list below carries the same path and the same words. The script
checks the two against each other and fails on an attribute without a reason or on a list that differs from the
code, so the list cannot go stale. `coverage_nightly` is the cfg cargo-llvm-cov sets on nightly; the attribute
and the crate-level feature gate behind it exist only there, and the workspace's `unexpected_cfgs` lint
(`[workspace.lints.rust]` in `Cargo.toml`) declares the cfg so the stable build stays warning-free. An
exclusion is allowed only on a function or a module whose failure arms only the operating system or the desktop
can provoke; the logic around such a call is split into a function a test reaches. The one other listed
exclusion is the test code of `mrgl-test-support`: test code like a `#[cfg(test)]` module, but a library of its
own, which the `#[cfg(test)]` class below does not cover. Two classes need no entry
below: every `#[cfg(test)]` module and item carries the attribute (the test code is not the subject, and its
`assert!` arms would be phantom branches), and the `cfg(not(windows))` twins are not measured at all.

The second form is for a single arm inside a tested function: a failure only the operating system can provoke
(a read of a folder the code just listed, a write to a file it just created, a rename of a path it just made, a
system call's own failure, a race between two reads of one path). Such a line ends with `// coverage: os`, or
the lines sit between a `// coverage: os-begin` line and a `// coverage: os-end` line; the script leaves those
lines and their branches out of the counts, and fails on a marked line a test does reach, so a mark cannot
outlive the line's testability. An arm a test can provoke with a locked file, a file where a folder is expected,
a read-only attribute, a stub process list or a stub rename is tested, never marked. The marks are the register
of this form (`grep -rn "coverage: os" crates/`); the list below is the register of the attribute form, the
reasoned exclusions:

- `mrgl-app/src/main.rs`: the Tauri event loop needs a WebView2 window and a desktop session
- `mrgl-app/src/lib.rs`: the Tauri event loop needs a WebView2 window and a desktop session
- `mrgl-app/src/commands.rs`: the native folder dialog; the choice itself is `choose`, tested without a dialog
- `mrgl-core/src/keys.rs`: the user's real Credential Manager; the unit tests run the same contract against the
  memory store, the end-to-end tests run with the store off
- `mrgl-test-support/src/lib.rs`: test code the tests of the three crates share, not the subject of the
  measurement, like a `#[cfg(test)]` module

Branch coverage counts the conditions of `if`, `&&`, `||` and `let … else`, and macro expansions can add
branches nothing reaches. A dead defensive condition is simplified with a comment rather than tested; a residue
of phantom branches no code change removes is listed here with the measured number the branch floor is set to.

### The release

`scripts\release.ps1` builds the release profile (fat LTO, stripped) and packs `target\release\mrgl.exe`,
`target\release\mrgl-app.exe`, `THIRD-PARTY-LICENSES.html` and the Forever starter list into
`dist\mrgl-<version>.zip`. The list sits at `lists\forever-starter.json` in the zip, its path in the repo, so
`mrgl import lists\forever-starter.json --flavor forever` works from the unpacked zip without the repo (task
012); both exes carry the murloc icon ([The UI](#the-ui)). The version is
the workspace `Cargo.toml` one, the only place it is written: `tauri-build` stamps it into the app exe's
properties, the CLI's `build.rs` into `mrgl.exe`'s (task 079), and both exes print it. Next to the zip it writes `SHA256SUMS.txt` (the zip's SHA-256 in the
`sha256sum -c` format) and `release-notes-<version>.md`. The license page comes from
[cargo-about](https://github.com/EmbarkStudios/cargo-about) over the crates compiled into the exes (it reads
the graph through `cargo metadata`, which wants the sources of every platform's crates, so the script runs
`cargo fetch` first and cargo-about offline after it): `about.toml` accepts the same licenses as `deny.toml` (a
license added to one is added to the other; the `deny` job compares the two), `scripts/third-party-licenses.hbs`
is the page, and every crate links to its source, which the MPL-2.0 crates ask for. cargo-about is a
build-time tool, never part of the exe. Its version is written in one place, `scripts/cargo-about-version.txt`:
CI installs that version, and `release.ps1` stops when the cargo-about on `PATH` is another one, so a page
rendered by hand is the page a release renders. Dependabot does not see that file; a new cargo-about is a hand
edit of its one line:

```powershell
cargo install cargo-about --version (Get-Content scripts\cargo-about-version.txt -Raw).Trim() --locked --features cli
scripts\release.ps1                     # dist\: the zip, SHA256SUMS.txt, release-notes-<version>.md
```

Pushing a tag `v<version>` runs the same jobs; when the six the release needs pass, the `release` job checks that the tag is on
`main`, installs the pinned cargo-about, runs `release.ps1`, attests the build provenance of the zip and
publishes it with the checksum file as a GitHub release titled `mrgl <version>`, with the notes file as its
text, through the runner's `gh` CLI. The release job builds without the Rust cache, so nothing a pull request
run cached reaches the zip. A downloaded zip is checked against its attestation with
`gh attestation verify <zip> --repo speti4/mrgl`. A usage guide inside the zip and signed exes are M5. The steps
of a release, from the version bump to closing its milestone, are the list in [The changelog](#the-changelog).

### The changelog

`CHANGELOG.md` is generated from the task files by `scripts/changelog.py`: every file under `tasks/done/` is a
line (its H1, linked to the file) under its milestone, in the section of the first release tag whose tree holds
it under `tasks/done/`, or under *Unreleased* when no tag holds it yet. The tag trees decide rather than the
`closed` date, because tasks close both before and after a tag on the same day. The summary under a heading is
written by hand and kept by every run; the lists are never edited by hand. A task that changed the project rather
than the app (the repo, CI, the process) carries `changelog: housekeeping` in its frontmatter and is listed last,
under *Project housekeeping*. The release notes are one section of the changelog without that group (one line
counts it and links the changelog), with a footer on the checksum, the attestation and the license page. The
script reads the task files through the parser of `scripts/sync-issues.py`, the script of the issue mirror.
A release goes like this:

1. In a pull request: bump the version in `Cargo.toml`, write the summary under *Unreleased* in
   `CHANGELOG.md`, run `python -I scripts/changelog.py` and merge.
2. Tag the merge commit `v<version>` and push the tag; the release job takes the notes from the changelog (the
   new tag takes the *Unreleased* summary).
3. Afterwards, `python -I scripts/changelog.py` stamps the section with the version and the tag's date; commit
   it with the next change. Until then `--check` reads the newest tag without a section as not there yet and
   prints a notice, so the pull requests opened in between, dependabot's included, stay green.
4. Once the release is published, close the GitHub milestone it completes (task 033): the issue mirror creates
   a milestone for every roadmap heading and never closes one, and an open milestone with every issue closed
   tells a contributor the work is still going on. Find the milestone by its title, the roadmap heading, and
   close it by its number when its `open` count is 0; a task of that milestone that the release does not ship
   moves to a later milestone in its task file first (its `milestone:` field), and the mirror moves its
   issue. A release that completes two milestones closes both. By hand, never from a script:

   ```powershell
   # The number, title and counts of the milestone with that title (the roadmap heading).
   gh api "repos/speti4/mrgl/milestones?state=open&per_page=100" `
     --jq '.[] | select(.title == "M2 Window MVP") | "\(.number)  \(.title)  open \(.open_issues), closed \(.closed_issues)"'
   # Close it by that number once its open count is 0.
   gh api -X PATCH repos/speti4/mrgl/milestones/<number> -f state=closed
   ```

```powershell
python -I scripts/changelog.py                 # rewrite CHANGELOG.md
python -I scripts/changelog.py --check         # exit 1 when it is out of date
python -I scripts/changelog.py --notes 0.1.0   # one release's notes (a version without a tag: Unreleased)
```

Between releases the committed *Unreleased* list matches `tasks/done/` exactly: the lint job's `--check` fails
on any difference, so the change that closes a task also runs `python -I scripts/changelog.py` and commits the
regenerated `CHANGELOG.md`. Two task branches that both add a line meet in the changelog; the later one reruns
the script after its rebase instead of merging the lists by hand.

### The issue mirror

`scripts/sync-issues.py` (from the project-pal task standard, `/tasks mirror`; `--version` prints the
template version it was copied from, and the copy is never edited per repo) mirrors the task files into GitHub
issues and milestones, one way. Every numbered file under `tasks/backlog/`, `wip/`, `done/` and `dropped/` and
every `tasks/decisions/D-NNN-*.md` is one issue: `NNN — <title>` as the title, a line of metadata (stage,
status, size, milestone, dates, origin; a decision's status, date and cost), then the file's `## Context` (a
decision's whole body), with a link to the file. The folder sets the state: `done/` closes the issue as
completed, `dropped/` and a superseded decision as not planned. The labels `mirrored`, `stage:*`, `status:*`,
`size:*`, `dropped` and `decision` are created by the script; every `## ` heading of `tasks/roadmap.md` is a
milestone with the heading's paragraph as its description (the script never closes a milestone; a release
does, by hand, step 4 of the list in [The changelog](#the-changelog)). An issue is found again by a hidden
marker in its body, so a run patches only what differs; an issue whose file is gone is reported as an orphan
and left alone, and a file without a numbered name is reported and not mirrored.

The `sync-issues` workflow runs it on `main` with the job's token (`issues: write`, nothing else), so the first
run after the workflow landed created every issue and milestone at once, and a run that follows a task commit
reports `created 0, updated 0` when nothing moved. Nothing flows back: an issue is never edited by hand, a
comment on it is welcome, and the change goes into the task file. By hand, from a logged-in `gh`:

```powershell
python -I scripts/sync-issues.py --dry-run     # what a run would change, writes nothing
python -I scripts/sync-issues.py --check       # exit 1 when the issues drift from the files
```

## Fixtures

Every test runs offline and against synthetic data under `crates/mrgl-core/tests/fixtures/`: a hand-written
game folder (`flavors/`: a `.build.info` with a row per product and a flavor folder per id, whose made-up
addons cover the multi-TOC suffixes, the `## Interface` lists, the `X-Curse-Project-ID` / `X-Wago-ID` /
`X-WoWI-ID` headers and the other shapes its README lists), the answers of the GitHub source (`releases/`:
a folder per URL the source asks, the release lists as the API writes them and `release.json` files in the
shape the BigWigs packager writes, `forever` next to other flavors, one without metadata, one repository
without `release.json`, one with no release, one with a draft and a prerelease above the stable release),
and small zips for the install engine (`zips/`, written by the deterministic `make-zips.py` next to them:
one addon folder in three versions, several folders in one archive, several whose core is not the first by
name and no folder extended by the others (`boss-core-5.0.0.zip`, task 065), a TOC at the zip's root, a wrapper
folder, a TOC with an interface for another build, a path that tries to escape the staging folder, an addon
with an embedded-only and a missing required dependency, and a zip with no addon at all), a hand-written
`WTF` tree in the shape the client writes (`wtf/WTF/`: a `Config.wtf`, an account with its caches and
SavedVariables, a realm and a character with its `AddOns.txt` and SavedVariables; a few hundred bytes, made
up from the first line, task 013), a second `WTF` tree for the addon sets (`addon-sets/WTF/`, task 014:
two characters on two realms of one account, one `AddOns.txt` with CRLF endings, the other with LF, a line
the client never wrote and no last line break, and the account's `SavedVariables` that is no realm), a
third game folder for the dependency check (`dependencies/`, task
029: a `_forever_` flavor whose addons have a met, an embedded-only and a missing required dependency and
an embedded optional one, and a library folder inside an addon that a `nolib` entry in a test's sidecar
keeps the check from looking into), a fourth game folder for the interface states (`interface/`, task 076:
a `_retail_` flavor with a current TOC, one an interface number behind the build and one of another
expansion; the window's flavor list reads it in place too, task 081), and a
development folder for the junction links (`dev/`: two addons with a TOC and a folder without one, task
005; the tests copy it into a temporary folder and make their junctions there), and a single addon folder
for the same links (`single-addon/SoloAddon/`: a TOC with the Forever suffix only and an embedded library
folder with a TOC of its own, task 062). A real addon zip, a real
AddOns folder or a `WTF` folder never enters the
repo; a manual round on a real game folder is a task of its own, and its result is a dated line in that
task's Verification. The running-client tests copy the system's signed `ping.exe`
(`%SystemRoot%\System32\ping.exe`) into a temporary flavor folder as `client-stand-in.exe` and start it
against the loopback address with a count of an hour, so it runs until the test kills it (a drop guard kills
it on a failed assertion too); no game file and no committed executable is involved, and where no `ping.exe`
is found the test is skipped with a message. The stand-in is never a copy of the test binary itself: a
binary that copies itself and runs the copy is the pattern an antivirus heuristic reads as a dropper, and
Windows Defender quarantined the test binaries for it (task 063). The stand-in is written once, in
`mrgl_test_support::stand_in` (`spawn_stand_in`, the drop guard `StandIn` and the skip line `SKIPPED`), and the
tests of the three crates reach it as a dev-dependency (task 118); the handles the tests hold open so that a
read, a move or a removal fails (`locked::file`, `locked::file_sharing_reads`, `locked::folder`) come from the
same crate. The backup tests copy the fixture `WTF`
tree into such a folder.

The sources talk to the network through the `Http` trait of `mrgl_core::sources`, a GET and a POST (task
092; a client that only answers GETs refuses a POST as the network does). `MRGL_HTTP_STUB=<folder>`
makes the command line answer every request from that folder instead (the URL's host and path under it,
`.json` appended when the bare path is no file, a 404 for a missing file, an `ETag` from the content and a
304 for a matching `If-None-Match`), which is how the end-to-end tests run `add` offline against
`tests/fixtures/releases/`; a unit test builds a `FolderHttp` or an in-memory stub directly. The query
string picks a file of its own first (task 092): `…/addons/abc?game_version=retail` is answered from
`…/addons/abc@game_version=retail.json` (or the name without `.json`), the pairs sorted by name with `&`
between them, `page` left out (pages have their own suffix, below) and a character Windows refuses in a
file name (`<>:"/\|?*`) written as `_`; without such a file the query is ignored, as for every fixture
before it, so two answers of one path that differ only in the query (two flavors of one addon) come from
two files. A POST is answered from the same files as a GET of its URL, and the stub keeps its body with
the request for the test to read (`FolderHttp::requests`).
`MRGL_GITHUB_TOKEN` (or the key store's `mrgl:github`, [The API keys](#the-api-keys)) holds a personal
access token for the GitHub API (60 requests an hour without one); it is sent to `api.github.com` only,
never with an asset download. The release list of a repository is cached with
its `ETag` under `<home>/cache/github/<owner>/<repo>/releases.json`, so a repeated run costs one conditional
request; the cache can be deleted any time. Next to it, `release-json/tag-<tag>.json` keeps each release's
`release.json` with that release's tag (tasks 011, 042): this is how `list` reads the Forever build offline.
Whenever the API answers with the list, a 200 or a 304, the `tag-*.json` files of tags the list no longer
holds are removed, and so is a `release-json.json` next to `releases.json`, the single file of task 011 that
nothing reads any more (task 072); the list read offline removes nothing. Every
fixture repository under `releases/` is classified for the Forever build in `tests/github_source.rs`, so a
new fixture names its expected build there.

The release list walks its pages (task 042): when the first page of 30 holds no stable release (a repository
that publishes a nightly prerelease every day), the source follows the `Link` header's `rel="next"` page,
on `api.github.com` only, with the token, until a page holds a stable release or five pages are read
(`sources::github::MAX_PAGES`); past the bound the newest prerelease is current, as for a repository without
a stable release. The resolve then walks the releases its channel may take, newest first, down to the first
with an asset for the flavor (task 066): a repository whose newest release is built for other flavors only
resolves to the release before it, never a draft and never a prerelease on the stable channel while a
stable release exists. The cache keeps every page read with the first page's `ETag`, so a repeated run is still one
conditional request. A release's `release.json` is read from `release-json/tag-<tag>.json` when the file
names the same tag and the same asset URL and size, so a repeated resolve of an unchanged release costs that
one request and no download. A 429, or a 403 with `x-ratelimit-remaining: 0`, is the rate limit: its line
names the local time it resets (`x-ratelimit-reset`, else `retry-after`) and `MRGL_GITHUB_TOKEN`. The folder
stub serves a `page` above 1 from `<answer>.page-<n>.json`, and a `<answer>.headers` file next to an answer
adds its `Name: value` lines to the headers, a `Status: <code>` line setting the status, which is how the
paged fixture `Example/Nightly` (30 nightly prereleases, the stable release on page two) and the rate-limit
tests are served.

An asset download has its own errors (task 046). A 404 on an asset the release list names (a zip or
`release.json`) is `the asset <name> of <source> <tag> is missing: GitHub answered 404 for <url> (removed or
renamed after the release was listed)`; the release list's own 404 keeps `no such repository, or it is
private`. The downloaded byte count is compared with the asset's `size` in the release list before the file
is written, so the engine never opens a cut download: a mismatch is `refused <asset> of <source> <tag>: <n>
bytes downloaded, the release list names <size>; the download was cut short or the asset changed, try
again`, and an asset the list gives no size installs as before (the Forever zip of the `Questie/Questie`
fixture lists none, so a test can put any fixture zip behind it).

The offline rule (task 041): when the release list's request fails on the network (no connection, a
timeout: `Error::Network`) and the list is cached, the resolve answers from the cache, the release's cached
`release-json/tag-<tag>.json` included, so a stale pick needs no network at all; with nothing cached the
network error stands, and a status answer (a 404, the rate limit) is never read as offline. The cache file
keeps `checked_at`, when the API last confirmed the list (RFC 3339 in UTC, refreshed by a 200 and by a 304),
and the pick carries `stale: { checked_at, reason }` (`null` for a fresh pick). `Pick::stale_warning` is the
warning line, `offline: <source> <tag> is from the release list cached <local time>, the pick may be stale
(<the network error>)` (`cached at an unknown time` for a cache without the date): `add --dry-run` and `add`
print it as a warning and list it in the JSON `warnings`, and so do `import` (the entry's `warnings`) and
`update` (one line per stale pick of the check), through the one install path of task 055. The asset itself is never cached before an install, so an `add` offline only gets as far as
finding the release installed already. The folder stub with a folder that does not exist answers every
request with a network error, which is how the tests go offline.

The WoWInterface source (task 094, `sources::wowinterface`) asks the MMOUI API, which takes no key:
`api.mmoui.com/v3/game/WOW/filedetails/<id>.json` answers an array of one project, its file's version
(`UIVersion`; the version is the text without a leading `v`, as for a tag), name, download address and MD5
(`UIFileName`, `UIDownload`, `UIMD5`), its date in milliseconds (`UIDate`) and `UIPending`. A project has
one file and both channels take it. `UIPending: "1"` (the newest file waits for the site's approval) is the
error `the newest file of wowi:<id> waits for approval on WoWInterface: nothing is installed until it is
approved`: `add` fails with it and `import` reports it as failed, while the update check reports it as the
state `pending` (task 104), not as a failed check. The
download (`cdn.wowinterface.com/downloads/getfile.php?id=<id>&d=<date>&minion`) is compared with `UIMD5`
before it is written, so the engine never opens it: a mismatch is `refused <file> of wowi:<id> <version>: its
MD5 is <md5>, the source names <md5>; the download was damaged or the file changed, try again`. The flavor
comes from the project's compatibility list (`UICompatibility`, the author's "works with" game versions),
which only the all-projects answer `filelist.json` carries (some 5.6 MB): the source reads it at most once a
run and once a day and keeps it trimmed to each project's compatibility, folders (`UIDir`) and page under
`<home>/cache/wowi/filelist.json`, the cache of any age standing in when the API cannot answer. A
compatibility version names its game (`sources::wowinterface::game_of`: `1.13` to `1.59` Classic Era, `1.60`
and up Forever, `2` to `5` the progressing classic by its major, Titan's `3.80` apart from Wrath's `3.4`,
`6` and up retail), and an entry of the flavor's game makes the pick one by metadata (`WoWInterface lists
compatibility 12.0.7 (<patch name>)`; the progressing classic without a known build is a guess of the game
type); without one, without a list and on Forever the project's one file is a guess that says what the list
names, which the update check reports as `guess only`, and the interface check after the install is the
safety net. The Forever signal is `yes` when the list names a Forever version and `unknown` otherwise, never
inferred from a classic or retail entry; `list` reads it from the cached file list alone. The project's
answer is cached under `<home>/cache/wowi/<id>.json` with `checked_at` (the API allows an hour,
`Cache-Control: max-age=3600`); every resolve asks again, and when the API cannot be reached the cached
answer stands with `stale`, the offline rule of the GitHub source. The fixture answers are under
`tests/fixtures/sources/` (`api.mmoui.com/…` and the stored zips under `cdn.wowinterface.com/downloads/`,
their query mapped to the file name, written by `zips/make-zips.py`): `1001` lists retail, Classic Era and
Cataclysm Classic, `1002` no compatibility, `1003` waits for approval, `1004` names an MD5 its file does not
have, `1005` lists a made-up Forever version, and `1006` is in the file list alone with two folders; the
end-to-end tests point `MRGL_HTTP_STUB` at that folder for a `wowi:` spec.

The Wago source (task 095, `sources::wago`) asks the external API with the user's own key
(`MRGL_WAGO_KEY` or the key store's `mrgl:wago`; without it a `wago:` spec is `source not configured:
wago:<id> (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)` and
nothing is asked): `addons.wago.io/api/external/addons/<id>?game_version=<v>` with `Authorization: Bearer
<key>`, the game version per flavor (`sources::wago::game_version`: `retail`, `classic` for Classic Era,
`forever`, and the progressing classic by the installed build, `bc`, `wotlk`, `titan` for `3.80`, `cata`,
`mop`; without a build `mop`, a guess of the game type). The answer's `recent_release.{stable,beta,alpha}`
map onto the channels (`sources::wago::release_on`): the stable channel takes `stable`, else the newest of
the other two (a prerelease, and the reason says so), the prerelease channel the newest of the three by
`created_at`. The pick's version is the `label` (without a leading `v`), its asset `<slug>-<label>.zip`, its
interface the newest of `supported_<v>_patches` (`supported_<v>_patch` or `supported_patches` when that is
missing), and `import --exact` finds a version among the three recent releases; the download link is
fetched with the same key when it is on `addons.wago.io`. A 401 is `Wago answered 401 for <url>:
MRGL_WAGO_KEY is missing or not accepted: …`; an answer without a release for the game version is `no asset
for flavor <flavor> in wago:<id> game version <v> (Wago lists no stable, beta or alpha release for it)`. Every
request to Wago's host counts against the limit its answers name (`x-ratelimit-limit`, 60 until one says,
and `x-ratelimit-remaining`): the update check reads it before each Wago addon and, at none left, stops
asking Wago while the other sources are still asked (task 101), `stopped asking Wago before its rate limit
of 60 requests: <n> addons not checked, the limit resets after a short wait (Wago does not say when); run
the check again then` (the words for the limit's window come with the budget, `RequestBudget::window` and
`resets`). The whole update check asks in bulk (task 106): before its first addon it hands every spec it
will ask to its source (`Source::prefetch`, through the router per source), and the Wago source sends one
`POST addons.wago.io/api/external/addons/_recents?game_version=<v>` with `{"addons": [<id>, …]}` and the
key for the game version of the flavor (`Wago::recents`), unless the budget has no request left. The answer,
the addons under `addons` by id (or a list, `sources::wago::parse_recents`), is held for the run and written
into the same cache files a single answer goes to; the check reads no budget for an addon the source holds
(`Source::holds`) and resolves it without a request, while an addon the bulk answer lacks (one without a
release for the game version), or every addon after a bulk request that failed, is asked alone as before.
`add`, `import` and `update <name>` keep the single request. The answers are cached per addon, one per game
version, under `<home>/cache/wago/<id>.json` with `checked_at`, and the offline rule of the GitHub source holds; the
window's offline view (`Sources::cached`) has no key and reads that cache alone. The Forever signal is `yes`
when the addon has a release for `forever` and `no` when Wago answers without one; the keyless data endpoint
`addons.wago.io/api/data/slugs?game_version=forever` (every addon with a release for that game version, slug
and id) answers it without a key, and the id of a slug (`Wago::id_of_slug`), read at most once a day and
cached under `<home>/cache/wago/slugs-<v>.json`. `list` reads the signal from the cached `forever` answer,
else the cached slug list. A Wago page address (task 105), `https://addons.wago.io/addons/<slug>` (a
trailing path or query is dropped), parses into `SourceSpec::WagoPage`; `Source::canonical_spec` looks the
slug up in the slug list of the target's game version (as written, else ASCII case-insensitive, without a
key) and answers `wago:<id>`, or `no Wago addon <slug> in the slug list of game version <v>: it has no
release for <v>, or the address names no addon` (`Error::UnknownWagoSlug`). `add` says `<address> is
wago:<id>` and records the id, as `import` does for a list entry with the address (an unknown slug is the
entry's failure); the resolve of a page spec looks it up first too, so a pick always names `wago:<id>`.
Without the key the address is `source not configured` like `wago:<id>`, since the install needs the key. The fixture answers are under `tests/fixtures/sources/addons.wago.io/`
(`api/external/addons/<id>@game_version=<v>.json`, the stub's query mapping): `MrglWg01` has a Forever and a
retail release with a stored zip behind each (`downloads/MrglWg01/2.0.0-<v>.zip`, written by
`zips/make-zips.py`), `MrglWg02` answers for Forever without a release, `MrglWg03` has the three
stabilities for retail, none stable for Forever and one for `mop`, `MrglWg04` answers 401 and `MrglWg05`
answers with `X-RateLimit-Remaining: 0` (both through a `.headers` file), and
`api/data/slugs@game_version=forever.json` names `MrglWg01`, `MrglWg03` and `MrglWg05`. The bulk answers
are `api/external/addons/_recents@game_version=<v>.json`, which the stub answers the POST from: `MrglWg01`
and `MrglWg03` for Forever (`MrglWg05` is left out, so its single answer still uses the limit up in the
budget tests), `MrglWg01` to `MrglWg03` for retail. The tests pass a
made-up key and assert that it goes to `addons.wago.io` only and reaches no output and no cache file.

The CurseForge source (task 096, `sources::curseforge`) asks the Core API with the user's own key
(`MRGL_CURSEFORGE_KEY` or the key store's `mrgl:curse`; without it a `curse:` spec is `source not configured:
curse:<id> (MRGL_CURSEFORGE_KEY is not set: the CurseForge API needs your own key, `mrgl key set curse` stores
it)` and nothing is asked). A resolve sends two requests with the key in `x-api-key`:
`api.curseforge.com/v1/mods/<id>` (the project: its name, its page `links.websiteUrl`,
`allowModDistribution`, `latestFilesIndexes`; a project whose `gameId` is not 1 is no addon) and
`api.curseforge.com/v1/mods/<id>/files?gameVersionTypeId=<t>&pageSize=50` (the first page of its files of a
game version type, which the API sorts newest first). The type per flavor is
`sources::curseforge::game_version_type`: retail 517, Classic Era 67408, Forever 88568, and the progressing
classic by the installed build, TBC 73246, Wrath 73713, Titan 81212 for `3.80`, Cataclysm 77522, Mists 79434
(without a build Mists, a guess of the game type); the names in a message and the pick's flavor are Wago's
(`retail`, `classic`, `forever`, `bc`, `wotlk`, `titan`, `cata`, `mop`). The ids are constants that another
client extracted from `GET /v1/games/1/version-types`; they are re-read from that endpoint once a key exists
(task 096 stays open for it). A file's `releaseType` maps onto the channels (`sources::curseforge::file_on`):
the stable channel takes the newest release (1), else the newest file, the prerelease channel the newest file,
a beta (2) or an alpha (3) included, newest by `fileDate`; a file that is not available or names only another
type's game versions is left out. The pick's version is the file's `displayName`, its release the file id,
its asset the `fileName` behind `downloadUrl`, its size `fileLength`, its MD5 the `hashes` entry of `algo` 2
(the SHA-1 of `algo` 1 is not read: no SHA-1 is in the tree, and the length and the MD5 catch a damaged or a
replaced file), its interface the newest `sortableGameVersions` entry of the type, its page the project's,
and `import --exact` finds a version by `displayName` (or the file id) on that first page. The download goes
to `edge.forgecdn.net` with the key; its length and MD5 are checked before it is written. A 401 (and a 403)
is `CurseForge answered 401 for <url>: MRGL_CURSEFORGE_KEY is missing or not accepted: …`. A project whose
author allows no download outside CurseForge (`allowModDistribution: false`, or a file whose `downloadUrl` is
`null`) is `Error::DistributionOff`, `the author of curse:<id> (<name>) allows no download outside
CurseForge: download the zip from <page> and install it with …`, the command `mrgl add <file.zip> --as
curse:<id>` at its end: `add` fails with it and the update check
reports it as the state `distribution-off` (task 115, above), no failed check; `mrgl add <file.zip> --as <spec>` installs a zip in hand with that spec as its
recorded source (any spec but a Wago page address, which only a request turns into its id; `--as` without
a zip is refused), the version from the TOC, so the update check asks the source. The API's terms allow no
saving or caching of its data: nothing of an answer is written, an answer is kept in memory for the run alone
(adopt asks the project once for its build and its folders), `listed_versions` stays `None` (the update check
orders two `displayName`s by `update::lenient_order`), the source has no request budget (no limit is
published), and the window's offline view has no key and asks nothing (`CurseForge::cached`: a `curse:`
row has no offline answer). The Forever signal is `yes` for a Forever pick and otherwise the project's
`latestFilesIndexes` naming type 88568 or not (`yes`, `no`; `unknown` without such a list); the install
records it on the sidecar entry (`forever_build`) and `list` reads it there, else `unknown`. The fixture
answers are under `tests/fixtures/sources/api.curseforge.com/v1/mods/` (`<id>.json` and
`<id>/files@gameVersionTypeId=<t>&pageSize=50.json`, the stub's query mapping; made-up ids and names):
`4242` has a Forever release with a newer beta and a retail release (its Forever file under
`edge.forgecdn.net/files/7001/1/`, its retail one behind a 302 of the CDN, a `.headers` file, to
`mediafilez.forgecdn.net/files/7002/2/`, both written by `zips/make-zips.py`), `4243` has retail files only,
`4244`'s author allows no download outside CurseForge (its zip in hand is
`zips/MrglNoDistribution-2.0.0.zip`), and `4245` answers 401. The tests pass a made-up key and assert that it
reaches `api.curseforge.com` and `edge.forgecdn.net` only, never the redirect's other host, no output and no
file under the home folder.

The adopt fixture (task 097) is a game folder of its own, `adopt/`: a `.build.info` with a Forever and a
retail row, `_forever_` with one addon folder per header shape, and `_retail_` with the CurseForge app's
`.curseclient` marker (a made-up id) and `MrglCompat` (`X-WoWI-ID: 1001`). The Forever folders:
`MrglWagoOne` and `MrglWagoOne_Options` (`X-Wago-ID: MrglWg01` in both, one group by the same source),
`MrglNoList` (the three id headers: `MrglWg02`, which the slug list does not name, `1002` and a CurseForge
id), `MrglForever` (`1005`, CRLF lines), `MrglElsewhere` with `MrglElsewhere_Options` (`1006`, whose `UIDir`
names both; the options TOC has no header), `MrglCurseOnly` (a CurseForge id alone), `MrglSite` (`X-Website`
on `github.com/Example/Nightly` and `## Version: @project-version@`), `MrglNoForever` (`X-Website` on
`github.com/Example/MainlineOnly`, no Forever asset), `MrglPage` (`X-Website` on another host), `MrglNoHint`
(nothing) and `Questie` with `QuestieDB` (no header: the mapping's folders). The tests copy `releases/` and
`sources/` into one temporary folder for the stub; the end-to-end tests and the window's adopt card
(`crates/mrgl-app/tests/adopt.rs`, task 109) copy the game folder too.

The Forever starter list (`lists/forever-starter.json`, task 028) has a stub of its own,
`starter-list/`: one release per entry's repository in the shape that repository's latest release had at
the check (the tag, the asset names, a `release.json` with the filename and the flavor and interface pairs
only, every size 2048, no zip), so the end-to-end test imports the list with `--dry-run` into an empty
`_forever_` and sees a Forever pick for every entry. A test checks that the list and the stub name the same
repositories and that every line names the list's version, so an entry added or bumped in the list comes
with its stub repository.

The build version fallback (task 019: the client exe's version resource when `.build.info` has no row for
the flavor) is tested against a second synthetic game folder, `client-exe/`: a `.build.info` with a row for
retail only, and client exe stubs written by the deterministic `make-stubs.py` next to them (`python -I
crates/mrgl-core/tests/fixtures/client-exe/make-stubs.py` rewrites the same bytes). A stub is a minimal
PE file with one `.rsrc` section and no code: `_classic_era_\WowClassic.exe` carries a version resource
(the fallback's happy path), `_retail_\Wow.exe` carries one the `.build.info` row wins over,
`_classic_\WowClassic.exe` has no version resource (the build stays unknown), `_forever_` has no exe at
all, and `stubs\` holds a 32-bit image and a resource without a string table for the parser's unit tests.
No stub is a game executable or a copy of one.

The command line and the window keep `config.json` under the mrgl home folder (`root`, the remembered game
folder, written by `mrgl root` and the window's chooser; `theme` and `serious`, the window's preferences of
task 034, the second also the third source of the voice's serious switch),
`%LOCALAPPDATA%\mrgl\` (D-003), and under it one folder per flavor id (`wowf\`, `wow\`, …) with the state
sidecar `state.json`, the addon sets `sets.json` (task 014), the last set apply's undo record
`set-undo\` (task 073), the last online check's answers the window shows,
`last-check.json` (task 078, written by the window's check and by `mrgl update`, task 087; a `curse:`
entry's answer keeps its state and reason and never a version, since the CurseForge API's terms allow no
saving of its data, task 096; a cache,
deleting it only drops a source gone or a failed check until the next check),
the install engine's `staging\` (a zip is unpacked and checked here before anything
moves, a download waits here; every run folder is removed when its run ends, and the leftovers of a run
that ended before that are swept, see "The staging folder's lifetime") and `cache\<addon>\<version>.zip` (the current and the previous
version's zip for `mrgl rollback`; never required, deleting it only disables the rollback), and the `WTF`
backups under `backups\<flavor-id>\`: `WTF-<date>.zip` per backup (a plain zip, the date local, so the
names sort by age) and `WTF-before-restore-<date>\` for every `WTF` folder a restore moved aside; nothing
prunes that folder, deleting a zip or an aside folder by hand is fine. `MRGL_HOME` overrides the home
folder, and the end-to-end tests set it to a temporary folder so no test reads or writes the real one. A
run's game folder is resolved in one order by
every command (`discover::resolve_root_of_process`, task 020): the `--root` flag, then `MRGL_ROOT`, then the
remembered one, then the discovery: the launcher's registry value (`HKLM\SOFTWARE\WOW6432Node\Blizzard
Entertainment\World of Warcraft`, `InstallPath`; a value that names a flavor folder counts for its parent),
then the game's Uninstall entry (below), then `Program Files (x86)\World of Warcraft` and the same under
`Program Files`. A discovered hit counts only
when it is a game root (a `.build.info` and at least one flavor folder); it is used for the run and reported
with its origin (a plain line in `list`, the answer of `mrgl root` with no argument, `root_origin` in
`--json`), and never written to `config.json`: `mrgl root <folder>` is the one place that remembers it.
`MRGL_DISCOVERY` replaces the lookup for one run: `off` finds nothing, and a `;`-separated list
(`registry=<path>` for the launcher's value, `webview2=<version>` and `windows=<version>` for the doctor's
WebView2 and Windows version, every other item a usual folder) is the stub the end-to-end tests run on, so
no test reads the real registry or looks under `Program Files`.

The same `backups\<flavor-id>\` folder takes the zips of `mrgl savedvars --clean` (task 025):
`SavedVariables-<date>.zip` per cleanup, the orphan SavedVariables files (no addon folder of their name, in
any case, under `Interface\AddOns`; a disabled addon keeps its folder) and their `.bak` files, each under its
path relative to `WTF`, so unzipping it into `WTF` by hand puts every file back. `backup --list` shows it
among the zips; `backup --restore` refuses it, since a restore replaces the whole `WTF` folder.

The game's Uninstall entry (task 043) is the `InstallLocation` value of the `World of Warcraft` key under
`Microsoft\Windows\CurrentVersion\Uninstall`, read in the 32-bit view of `HKLM` first
(`HKLM\SOFTWARE\WOW6432Node\…`, where the installer writes it as it writes the launcher's key), then in the
native `HKLM\SOFTWARE\…`, then in `HKCU\Software\…`: an install moved or restored by hand may carry it
without the launcher's key. Its origin is `the game's Uninstall registry entry` (`uninstall_entry` in
`--json`). A value of the type `REG_EXPAND_SZ`, from the launcher's key or the Uninstall entry, is expanded
from the environment (`ExpandEnvironmentStringsW`; a variable that is not set stays as written) before it is
tried; a `REG_SZ` value is taken as written, `%` and all (`discover::registry_path`). The stub takes
`uninstall=<path>` for an Uninstall value, and `registry-expand=<text>` and `uninstall-expand=<text>` for an
expandable value, which go through the same environment call, so the end-to-end tests cover the expansion
without a registry.

## Forever notes

Two signals mark an addon on Forever (task 011), derived first and curated second. The derived one is the
source's: `sources::github::forever_build` reads the current release's `release.json`: `yes` when it names
`forever` (or `camelot`) for a zip the release has, `no` when it names other flavors only and no asset name
ends in `-forever` or `-camelot`, `unknown` without a readable `release.json` or when only an asset name
says forever (a pick by name). The pick carries it (`forever_build`); `add --dry-run --flavor forever` prints
it on a line after the pick, and `--json` writes `forever: { build, note, kind, verified, source, stale }`
next to `pick` (the window's `{ build, note }` with the date, the source and the stale flag; `null` for
another flavor). `list` on Forever marks each managed addon's primary folder (`forever:yes`, with `,note` and
`,stale`, a warning line per curated note, and the same `forever` object in the JSON, `null` for an
unmanaged addon) from the cache alone (`Source::cached_forever_build`, through the router): no request, so `list` stays offline
and fast, and without a cached release list and the `release.json` of its current release the build is
`unknown`.

The curated one is `data/forever-notes.json`, compiled into the core with `include_str!`: no file to ship and
none to read at run time. It holds the crippled cases only: an addon that has a Forever build but half works
under Forever's combat restrictions. The schema (version 1):

```json
{
  "schema": 1,
  "about": "what the file is",
  "kinds": {
    "<kebab-case id>": { "reason": "one line", "verified": "YYYY-MM-DD", "source": "https://…" }
  },
  "addons": {
    "github:<owner>/<repo>": {
      "kind": "<a kinds id>", "reason": "one line", "verified": "YYYY-MM-DD", "source": "https://…"
    }
  }
}
```

`kinds` are Forever's restrictions, each described once with its source; `addons` are the notes, keyed by the
canonical source spec. To add a note, add an entry under `addons` with the restriction it runs into, a
one-line reason (lower case, no trailing full stop: what does not work), the day it was checked against the
source and the source's URL, then run `cargo test`. The schema check (`forever::tests`) rejects an unknown
field or kind, a key that is not a canonical spec, a reason over one line, a date that is no real day or lies
in the future, and a source that is not an `https://` URL. The `verified` date is the day of the check where
you are: the schema check compares it with today in UTC and allows one day of slack
(`forever::VERIFIED_SLACK_DAYS`, task 070), so a note dated by the local day east of UTC passes between local
and UTC midnight, while a date two days ahead fails. A note whose `verified` date is more than 90 days old
(`forever::STALE_AFTER_DAYS`) is shown as stale until someone checks it again and moves the date; nothing
checks the age in CI.

Where a note comes from (task 058): a public source that a page under `docs/research/` cites, an addon author's
statement, a release note or the project page, never a community guide alone; an addon the sources disagree
on stays on the research page as an open question and out of the file. The key is the spec mrgl installs the
addon from, so an addon whose repository publishes no release zip stays on the research page too: its note
would never meet a managed addon. The first notes and the open questions are in
[the Forever restrictions research](research/2026-10-22-forever-addon-restrictions.md).
