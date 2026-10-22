# mrgl — development

How the repo is laid out, how CI and releases work, and what the helper scripts do. Build prerequisites, the
checks before a commit and the project rules are in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Layout

- `crates/mrgl-core` — the library: flavor detection (`flavor`), the AddOns scan and TOC parsing (`scan`,
  `toc`), the sources (`sources`: the `Source` trait, the `Pick` it hands to the install engine, the `Http`
  trait with the real `ureq` client and the folder-backed stub; `sources::github` reads the releases API,
  the packager's `release.json` and the asset names, with the release list cached by `ETag` under the mrgl
  home folder, task 003; the pick carries `notes`, the release body's first line with text once its
  Markdown markers are stripped, cut at `NOTES_MAX_CHARS` (120) with an ellipsis by `sources::notes_line`,
  and `page`, the release's page, both left out when absent, task 030; the `Target` carries the addon's
  channel, and on `prerelease` the newest release that is not a draft is current, a prerelease or not,
  task 021; the resolve walks the channel's releases from the current one down to the first with an asset
  for the flavor, the newest release that has one, and says so in the pick's reason when it passed newer
  ones, and the no-asset error names the newest release and how many releases were looked at, task 066;
  later Wago, WoWInterface and CurseForge), the install engine (`install`: a zip into
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
  sidecar with the source `link:<development folder>`, the TOC's version and no file list; `unlink` removes
  exactly those junctions, never a real folder or a junction mrgl did not make; Windows-only), the state
  sidecar (`state`, D-003; an entry's optional `pinned` version, `ignored` flag and release `channel`,
  `stable` or `prerelease`, with the queries and setters the update check and the CLI call, task 021; absent
  fields read as unpinned, not ignored, stable, and a reinstall keeps them), the toolbox (`backup`: the `WTF`
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
  that name is in the scan or the name starts with `Blizzard_`, an addon the client ships (task 064),
  *embedded* when a folder of that name sits inside the addon's own folder, at
  most three levels down, which a build without `nolib` carries, and *missing* otherwise; an optional
  dependency is never reported; the sidecar entry's `nolib` flag, recorded from the pick at install, turns
  the embedded lookup off for a `nolib` build, and an entry without it is unknown, embedded possible), the
  Forever notes (`forever`, task 011: the build signal `yes`, `no` or `unknown` that
  `sources::github::forever_build` derives from a release, and the curated `data/forever-notes.json`
  compiled in with `include_str!`, with the stale rule), the diagnostic report (`doctor`, task 024: the
  facts a bug report needs, with the profile folder as `~`, see [The doctor](#the-doctor)), the addon
  list (`list_file`, task 015: the export from the sidecar and the import through the source and the engine,
  below), the interface check (`interface`, task 015, below) and the voice table (`voice`, D-005). Synthetic
  fixtures are under `tests/fixtures/`.
  The SavedVariables are `savedvars` (task 025): the walk of `WTF\Account\<account>\SavedVariables\` and
  `WTF\Account\<account>\<realm>\<character>\SavedVariables\`, a `<Addon>.lua` and its `.lua.bak` as one
  entry with their sizes, the largest first, and the orphan rule: a file is an orphan when no folder of its
  name, compared without regard to case, is an addon the scan finds under `Interface\AddOns` (a folder with a
  TOC named after it, or one the scan cannot read); a disabled addon keeps its folder and is never one, a
  `Blizzard_` file is the client's own, and with no AddOns folder at all nothing is marked. The cleanup,
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
  <file.zip>` and `remove` (task 004; a running client is a warning line and `client_warning` in `--json`,
  task 038), `backup` with `--list`, `--restore` and `--yes` (task 013), `doctor [--flavor <flavor>]`
  (task 024), `link <folder>` and `unlink <folder>` (task 005), `export --flavor <flavor> [--to <file>]`
  and `import <file> --flavor <flavor> [--exact] [--dry-run]` (tasks 015 and 057), `pin <name> [version]`,
  `unpin`, `ignore`,
  `unignore` and `channel <name> <stable|prerelease>` on a managed addon, marked in `list` (task 021),
  `rollback <name>` (task 022), `set save|apply|list|show|remove` (task 014; `--character` on `save` and
  `apply`, task 027; below), `update --all | <name> --flavor <flavor> [--dry-run] [--yes]` (task 006,
  below), with `--json` for machines and `--serious` for the murloc-averse; its end-to-end tests under
  `tests/`. `list` and `add <file.zip>` run the dependency check
  (task 029; `scan` reads the folder only): a warning line per addon with a missing required dependency,
  `missing: <names>` at the end of the addon's `list` line, and in `--json` a `dependencies` object per
  addon of `list` and a `dependencies` array of the installed folders under `add`'s `addon`, each with
  `folder`, `required`, `met`, `embedded`, `missing` and `nolib` (`true`, `false` or `null` for unknown).
  `savedvars --flavor <flavor> [--clean]` (task 025) prints a data line per SavedVariables entry (size,
  addon, `account-wide` or `realm / character`, never the account folder's name, which `--json` carries,
  `orphan` or `client`) and a summary, `--clean` moves the orphans; `--json` is the `savedvars` or the
  `savedvars-clean` document.
- `crates/mrgl-app` — the `mrgl-app` executable: the Tauri 2 window (`src/`: the IPC commands in
  `commands.rs`, the flavor list in `list.rs`, task 009), the static UI under `ui/`, the capability file and
  the icon; see [The UI](#the-ui).
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
- `scripts/` — helper scripts for the icon, the release zip, the changelog and the commit rules.
- `docs/` — this page, the git workflow, the maintainer sessions, origin, research and the voice guide.
- `tasks/` — the backlog, the work in flight, the history and the decisions (the task standard).
- `.github/workflows/` — CI (fmt, clippy, test, the oldest supported Rust, the changelog check, the dependency
  policy, the workflow lint, the commit rules), the tagged release, the weekly advisory check and the task-to-issue
  mirror;
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
another format or schema is refused on import. `mrgl import` resolves every `github:` entry for the target
flavor's build and installs the current release through the engine under the entry's name, so the version
in the list is a record, not a pin; a zip in hand is reported as not importable, a `link:` entry is skipped,
and an addon the flavor already has (the same name, the same source, or a folder of the entry that is there,
managed or not) is reported and left alone. `--dry-run` resolves every entry and writes nothing under
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
name, else `stable`, `Engine::target_for`, task 021). An addon the sidecar has from that source at the
pick's version, its folders there, is installed already: nothing is downloaded and nothing moves. Otherwise
the asset is downloaded into a fresh folder under `staging\`, staged and committed with the pick's source
and version under the name the sidecar knows the source by (else the request's name, else the folders'),
and the download is removed whatever the install returned. `Engine::resolve` is the same resolve alone, for
a dry run, and `Engine::install_pick` the same path for a pick in hand. The link refusal of task 005 sits in
`Engine::commit`, so a zip in hand, a source, an import, an update and a rollback all refuse to install over
a linked addon or a junction mrgl did not make. The command line reports every install with one summary
(the `installed …`/`updated …` line and the `addon` block of the JSON document, with the dependency and
interface checks).

The folders' name (`install::derive_name`, for a zip in hand and a source install the sidecar and the
request do not name) is the folder every other folder of the zip extends with `_` or `-` (`BigWigs` for
`BigWigs_Core` and `BigWigs_Options`); else the folder named after the zip's stem without its version and
flavor suffix (`DBM-Core` for `DBM-Core-12.0.11-classic.zip`), else after the repository of a `github:`
source, both ASCII case-insensitive (task 065); else the first folder by name. A source install of an addon
the sidecar has from that source keeps the name it is recorded under.

The update check (task 006) is `update::check` in the core: every managed addon of the sidecar (or the one
a name or a folder names) against its source's current release on the addon's own channel, one
`UpdateState` each, read-only. `up-to-date`: the release's version is the installed one. `update-available`:
another version, picked from metadata. `guess-only`: another version whose asset for the flavor is a guess
by its name; it is reported, not installed, and `mrgl add <source>` installs it anyway. `pinned`: checked
and reported with the current release, left alone. `ignored`: skipped without a request. `source-gone`: no
release, no asset for the flavor or no repository (404) any more. `not-checked`: no source to ask (a zip in
hand, a linked folder, a source not handled yet). `failed`: the source could not be asked (offline, a rate
limit, a server error). Another version counts as an update whatever its number: the source's current
release on the addon's channel is the one to have. An addon whose folders were deleted by hand is left out
with a warning. `mrgl update --all` and `mrgl update <name>` print a plain data line per addon
(`Questie  1.2.2 -> 1.2.3  update available`), ask once for the updates (`--yes` answers; without a
terminal or with `--json` the command stops and says what `--yes` does, exit 1), install each through the
one install path with the pick the check made (`AddonUpdate::request`, no second resolve) and end with the
summary that counts every state (`1 addon updated, 2 up to date, 1 pinned, 1 ignored`; `--dry-run` says
`1 update available` and writes nothing). `--json` is one `update` document: every addon with `state`,
`latest`, `pick`, `updated` and the installed `addon` block, the `counts` per state and the warnings. An
update or a check that fails does not stop the others; the run then ends with exit code 1.

The interface check (task 015) compares the `## Interface` numbers of the TOC the flavor loads with the
installed build: an interface number's major version is the number divided by 10000 (`120105` is 12,
`16001` is 1), and the addon passes when one number of the list has the build's major. A mismatch is a
warning, never a refusal: `scan` and `list` mark the addon's line `interface mismatch` and add one warning
line, their JSON carries `interface_mismatches`, and `add <file.zip>` and `import` add a warning after the
install (`interface_mismatch` in their JSON). An unknown build, a TOC without an `## Interface` line and a
folder the flavor loads no TOC from give no warning.

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
`--flavor` and `--json` (`kind` `set-save`, `set-apply`, `set-list`, `set-show`, `set-remove`).

A set is per flavor, its target may be one character (task 027): `set apply <name> --character <name>
[--realm <realm>]` writes that character's `AddOns.txt` only and leaves every other character's file byte for
byte, and `set save <name> --character <name>` takes the set from that one file (so nothing is left out as a
disagreement). The character is picked from the `WTF` account folders by its folder's name, ignoring case;
`--realm` narrows a name more than one realm has. A name on two realms without `--realm` is refused with the
realms listed, and a name no character has with the characters listed; when one realm and name are under
two accounts the error names the accounts. The guard of task 023 holds as for every character: nothing is
written while the client runs. In `mrgl_core::sets` the choice is `Target` (`EveryCharacter`, or
`Character { name, realm }`) and `find_character` is the lookup the window can call.

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
- `api keys: MRGL_GITHUB_TOKEN set` or `not set` (`api_keys`: `name`, `present`): which keys are set, by
  name; an empty variable is not set. The CurseForge key joins with its source (M4).
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
- `  client: running (<exe>, pid <n>)`, `not running` or `unknown (<reason>)` (`client`: `status`, `pid`,
  `exe`, `reason`): whether a process runs from the flavor folder (task 023); `unknown` when the process list
  cannot be taken.
- `  warning: <text>` (`flavors[].warnings`): what reading the flavor folder met (an unknown id, a build
  neither file has).
- `warning: <text>` (`warnings`): what the run met before the flavors (an unreadable config, an unreadable
  game folder).

The summary names what needs a look: no game root, a missing flavor folder, an AddOns folder that takes no
write, a sidecar that is rebuilt or unreadable, a process list that cannot be taken, a used-up rate limit and
the warnings. The tests run the report offline: the core's tests hand it a fixed process list, an in-memory
HTTP stub and a fixed WebView2 and Windows answer; the end-to-end tests stub the WebView2 version with a
`webview2=<version>` item of `MRGL_DISCOVERY` and the Windows version with a
`windows=<display version>,<build>.<ubr>` item (`windows=24H2,26100.1234`, a part may be left out; any other
value of the variable is no runtime and an unknown Windows, so `off` reads no registry), answer the rate
limit from `releases/api.github.com/rate_limit.json` and assert that neither a key's value nor a `\Users\`
path is in the output.

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
  field, "Update all"), the addon list (one row per addon: name and folder, version and the version an update
  brings, the source badge, the state pill, the Forever note slot of task 011, and a row menu for the actions
  of tasks 010 and 021, with a progress slot per row), a legend, a status bar and the toast stack. The theme
  toggle and the serious switch sit at the bottom of the sidebar, with "Game folder…" and "Copy
  diagnostics" (task 009). The theme and the switch are kept by the shell in `config.json`, not in the web
  view's storage (task 034, below).
- `style.css` holds every color as a token on `:root`: the light set, the dark set under
  `prefers-color-scheme: dark` (unless the toggle says light) and under `data-theme="dark"` (the toggle), one
  token per state (managed, update available, linked, guess only, unmanaged) and per source badge (github,
  wago, wowi, curseforge, zip, linked; a dashed badge is a source known only from a TOC hint).
- `main.js` is the view: it reads `window.Data`, draws what it gets and never composes a murloc line. It does
  not know whether the rows are made up.
- `data.js` is the window's data module (task 009): the contract behind `window.Data`, each call one Tauri
  command of `src/commands.rs` (below). `index.html` loads it and nothing else for the data.
- `mock-data.js` is the data module of the sketch, made-up rows in the shape the core answers with; its header
  comment is the contract (`Data.version()`, `Data.flavors()`, `Data.chooseRoot()`, `Data.diagnostics()`,
  `Data.resolve(flavorId, folder)`, `Data.updateAll(flavorId, report)`; a flavor as `flavor.rs` serializes
  it plus its friendly name, the source of its build and its addons; an addon as `scan.rs` gives it plus the
  facts of the sidecar, the sources, the cached release list, the Forever notes, pin / ignore and the
  missing dependencies). For a look at the page without the core, open `crates/mrgl-app/ui/index.html` in a
  browser: without the Tauri global, `data.js` writes the mock's script tag instead, and the made-up rows
  show (the folder dialog and the clipboard need the window).
- The pure modules, each with a test file of the same name under `tests/ui/`: `state.js` (an addon's one state
  from its facts, with the precedence linked, update, managed, guess, unmanaged; the classes and labels),
  `search.js` (every word of the query in the name, folder, source kind or source id), `preferences.js`
  (the window's preferences from an answer of the shell: the theme, the switch, the switch locked when
  `MRGL_SERIOUS` decides it, and the one-time carry-over of a value the sketch kept in `localStorage`),
  `theme.js` (auto, light, dark), `flavors.js` (the navigation order and the friendly names) and `voice.js` (the two lines and
  where a message goes, see [nerglish.md](nerglish.md), "The window").

The IPC commands (`src/commands.rs`, task 009) are thin wrappers over the core and over the plain functions
of `src/list.rs`, which take the game root and the mrgl home folder as arguments, so `tests/flavor_list.rs`
checks the rows on a made-up game folder without a window. Every message they return is
`{ kind, murloc, plain }`: the core's voice kind (`Kind::as_str`) picks the place in the window, and a
serious voice sends the murloc line empty. Whatever reads the disk or the network runs on a blocking thread.

- `app_version`: the version for the status bar.
- `flavors`: the game root as every CLI command resolves it (`MRGL_ROOT`, the remembered root, then the
  discovery; a discovered root is used and not remembered), its flavor folders and their rows, and a banner.
  A row joins the scan (folder, the TOC's title without color escapes, a junction), the flavor's state
  sidecar (managed, the installed version, the source, pinned, ignored), a TOC hint for an unmanaged addon
  (Wago, then WoWInterface, then CurseForge, drawn as a guess), the dependency check (`missing`), and on
  every row of the Forever flavor its Forever status (`build`, `note`, `verified`, `source`, `stale`). The
  list never goes online: "update available" is the GitHub source's cached release list (the one `add`
  leaves), read through an `Http` that answers every request with "not modified", whose current release on
  the entry's channel is not the installed version; without a cached list no update is known.
- `choose_root`: the folder dialog (`tauri-plugin-dialog`, called from Rust); a game folder, or a flavor
  folder whose parent is one, is remembered in `config.json` as `mrgl root <folder>` does and answered with
  its flavors; a cancelled dialog answers nothing.
- `diagnostics`: the doctor's report (`mrgl_core::doctor`, see [The doctor](#the-doctor)) as the text
  `mrgl doctor` prints, with the message for a copy that worked and one for a copy the clipboard refused;
  the view writes it with `navigator.clipboard` (a selection copy as the fallback), so no clipboard plugin
  joins.
- `resolve`: a managed addon through its source, for the flavor's build and on its channel: the source's
  own `Pick`, the handover to the update actions of task 010.
- `update_all`: answers that the update actions are task 010, in the voice (a plain warning line: the core
  has no error for a command not built yet).
- `preferences` and `set_preferences` (task 034): the theme (`auto`, `light`, `dark`) and the serious
  switch as they apply, which source decided the switch (`environment` locks the window's switch: the
  window cannot override `MRGL_SERIOUS`), and what `config.json` stores (`theme`, `serious`, next to
  `root`). The view reads them at start, writes a toggle's change at once, and carries an old
  `localStorage` value over once for a key the file does not have, then removes the old keys; a cleared
  web view draws what the file says. Each writer of `config.json` sets only its own keys and keeps the
  others, so `mrgl root` keeps the preferences and the window keeps the root.

The application icon is generated once by `scripts\make-icon.py` (Pillow; run it as
`uv run --with pillow python -I scripts\make-icon.py`) and committed: `icon.ico` for the
Windows exe and `icon.png` for the window icon on the other targets. It is a glyph of our own, never Blizzard
artwork.

## CI and releases

`.github/workflows/ci.yml` runs six jobs on every push to `main` and every pull request: `test`
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
acceptance moves with the step and covers nothing else). `.github/workflows/commit-rules.yml` runs
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
`lint`, `msrv`, `non-windows`, `deny`, `workflow-lint` and `commit-rules` must pass, the history stays linear, and force
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

### The release

`scripts\release.ps1` builds the release profile (fat LTO, stripped) and packs `target\release\mrgl.exe`,
`target\release\mrgl-app.exe` and `THIRD-PARTY-LICENSES.html` into `dist\mrgl-<version>.zip`. The version is
the workspace `Cargo.toml` one, the only place it is written: `tauri-build` stamps it into the app exe's
properties and both exes print it. Next to the zip it writes `SHA256SUMS.txt` (the zip's SHA-256 in the
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

Pushing a tag `v<version>` runs the six jobs; when they pass, the `release` job checks that the tag is on
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
with an embedded and a missing required dependency, and a zip with no addon at all), a hand-written
`WTF` tree in the shape the client writes (`wtf/WTF/`: a `Config.wtf`, an account with its caches and
SavedVariables, a realm and a character with its `AddOns.txt` and SavedVariables; a few hundred bytes, made
up from the first line, task 013), a second `WTF` tree for the addon sets (`addon-sets/WTF/`, task 014:
two characters on two realms of one account, one `AddOns.txt` with CRLF endings, the other with LF, a line
the client never wrote and no last line break, and the account's `SavedVariables` that is no realm), a
third game folder for the dependency check (`dependencies/`, task
029: a `_forever_` flavor whose addons have a met, an embedded and a missing required dependency and one
on a client addon, `Blizzard_Something`, task 064, and a
library folder inside an addon that a `nolib` entry in a test's sidecar turns into a missing one), and a
development folder for the junction links (`dev/`: two addons with a TOC and a folder without one, task
005; the tests copy it into a temporary folder and make their junctions there). A real addon zip, a real
AddOns folder or a `WTF` folder never enters the
repo; a manual round on a real game folder is a task of its own, and its result is a dated line in that
task's Verification. The running-client tests copy the system's signed `ping.exe`
(`%SystemRoot%\System32\ping.exe`) into a temporary flavor folder as `client-stand-in.exe` and start it
against the loopback address with a count of an hour, so it runs until the test kills it (a drop guard kills
it on a failed assertion too); no game file and no committed executable is involved, and where no `ping.exe`
is found the test is skipped with a message. The stand-in is never a copy of the test binary itself: a
binary that copies itself and runs the copy is the pattern an antivirus heuristic reads as a dropper, and
Windows Defender quarantined the test binaries for it (task 063). The backup tests copy the fixture `WTF`
tree into such a folder.

The sources talk to the network through the `Http` trait of `mrgl_core::sources`. `MRGL_HTTP_STUB=<folder>`
makes the command line answer every request from that folder instead (the URL's host and path under it,
`.json` appended when the bare path is no file, a 404 for a missing file, an `ETag` from the content and a
304 for a matching `If-None-Match`), which is how the end-to-end tests run `add` offline against
`tests/fixtures/releases/`; a unit test builds a `FolderHttp` or an in-memory stub directly.
`MRGL_GITHUB_TOKEN` holds a personal access token for the GitHub API (60 requests an hour without one); it is
sent to `api.github.com` only, never with an asset download. The release list of a repository is cached with
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
sidecar `state.json`, the addon sets `sets.json` (task 014),
the install engine's `staging\` (a zip is unpacked and checked here before anything
moves; emptied after every install) and `cache\<addon>\<version>.zip` (the current and the previous
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
unmanaged addon) from the cache alone (`GitHub::cached_forever_build`): no request, so `list` stays offline
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
