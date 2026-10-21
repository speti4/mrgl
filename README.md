# mrgl

A flavor-correct World of Warcraft addon manager for Windows 11, Forever-first: one portable zip with two exes
(the `mrgl` command line and the `mrgl-app` window), nothing to install, no Electron, and it never touches
your game client.

[![CI](https://github.com/speti4/mrgl/actions/workflows/ci.yml/badge.svg)](https://github.com/speti4/mrgl/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/speti4/mrgl)](https://github.com/speti4/mrgl/releases/latest)
[![License: MIT](https://img.shields.io/github/license/speti4/mrgl)](LICENSE)

Planned output (M1):

```text
> mrgl update --all --flavor forever
Mrrrgl… mrgl mrgl.
checking 14 addons in _forever_ against their sources
Mrglglglgl!
2 addons updated, 12 up to date
```

## What it will do

mrgl is under construction; the plan and its state are in [`tasks/`](tasks/README.md), the milestones in
[`tasks/roadmap.md`](tasks/roadmap.md). The first milestone is the core and the command line:

- **Flavor-correct installs.** Each game folder (`_retail_`, `_classic_`, `_classic_era_`, `_forever_`) is
  detected from its own `.flavor.info`, and an addon's release is picked for that flavor, never "latest".
- **Sources that work without CurseForge.** GitHub releases with `release.json` first, then the Wago and
  WoWInterface APIs and hand-picked zips; CurseForge with your own API key, within its terms, never scraped.
- **Only `Interface\AddOns`.** mrgl writes addon folders and keeps its own state under `%LOCALAPPDATA%\mrgl\`;
  the AddOns folder stays the truth and the state is a cache you can delete any time. It touches `WTF` only
  for a backup restore, an addon set or a SavedVariables cleanup you asked for. It lives next to the
  CurseForge app without a fight.
- **A toolbox, not only a manager.** WTF backup and restore, addon sets, the SavedVariables sizes with the
  cleanup of files whose addon is gone, a junction link for the addons you develop yourself, and a window on
  top of it all.
- **A murloc voice.** Every message is a murloc line and a plain English line; `--serious` (or
  `MRGL_SERIOUS=1`) keeps only the English, and JSON output is always plain ([docs/nerglish.md](docs/nerglish.md)).

## Build from source

mrgl is a Rust core with a `clap` command line and a Tauri 2 window on WebView2; why is in
[D-002](tasks/decisions/D-002-rust-tauri2-portable-zip-cf-aware.md). On Windows you need Rust stable with
the MSVC toolchain and the Visual Studio 2022 C++ build tools (details in [CONTRIBUTING.md](CONTRIBUTING.md)):

```powershell
cargo build --release
target\release\mrgl.exe --version
target\release\mrgl.exe list --flavor forever                               # what is installed, managed by mrgl or not
target\release\mrgl.exe root "D:\Games\World of Warcraft"                   # remember a game folder installed elsewhere
target\release\mrgl.exe add github:<owner>/<repo> --flavor forever            # install the release's Forever asset
target\release\mrgl.exe add github:<owner>/<repo> --flavor forever --dry-run  # which asset it would be, nothing installed
target\release\mrgl.exe add SomeAddon-1.2.3.zip --flavor forever            # install a zip in hand
target\release\mrgl.exe remove SomeAddon --flavor forever                   # remove an addon mrgl installed
target\release\mrgl.exe link D:\dev\my-addons --flavor forever              # a junction per addon you develop
target\release\mrgl.exe unlink D:\dev\my-addons --flavor forever            # exactly those junctions go again
target\release\mrgl.exe pin SomeAddon 1.2.0 --flavor forever                # keep it at 1.2.0 (no version: the installed one)
target\release\mrgl.exe unpin SomeAddon --flavor forever                    # let updates reach it again
target\release\mrgl.exe rollback SomeAddon --flavor forever                 # the previous version back from the kept zip, pinned
target\release\mrgl.exe ignore SomeAddon --flavor forever                   # updates skip it (unignore undoes it)
target\release\mrgl.exe channel SomeAddon prerelease --flavor forever       # follow its beta builds (stable undoes it)
target\release\mrgl.exe backup --flavor forever                             # the WTF folder into a dated zip
target\release\mrgl.exe backup --list --flavor forever                      # the backups so far
target\release\mrgl.exe backup --restore WTF-2026-10-21-193605.zip --flavor forever   # one back, asks first
target\release\mrgl.exe savedvars --flavor forever                          # the SavedVariables by size, orphans marked
target\release\mrgl.exe savedvars --clean --flavor forever                  # the orphans into a backup zip
target\release\mrgl.exe doctor                                             # the diagnostic report for a bug report
target\release\mrgl.exe export --flavor forever --to forever-addons.json    # the managed addons as a list file
target\release\mrgl.exe import forever-addons.json --flavor forever --dry-run   # what installing that list would do
```

`add --dry-run` names the asset it picked and why; when the release has notes, the next line is their first
line and the release page, so you know what an update brings before you take it (the plain lines, as
`--serious` prints them):

```text
pick for forever: github:<owner>/<repo> v2.1.0 (Addon-v2.1.0-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)
release notes: Quest arrows point the right way again (https://github.com/<owner>/<repo>/releases/tag/v2.1.0)
```

On a standard install no flag is needed: mrgl finds the game folder through the registry (the launcher's
key, or the game's Uninstall entry) or under `Program Files (x86)` / `Program Files` and says so; `mrgl root` with no argument shows the folder a
run would use and where it came from, and `mrgl root <folder>` remembers one (also `--root <folder>` or
`MRGL_ROOT` for one run).

`mrgl list` marks every addon folder managed or unmanaged. A managed addon is one mrgl installed: its line
shows the version mrgl installed and the source it came from, as the state under `%LOCALAPPDATA%\mrgl\`
remembers them. Every other folder is unmanaged (put there by hand or by another manager): its line shows the
TOC's version and the CurseForge, Wago and WoWInterface ids the TOC names, and mrgl leaves it alone until it
is installed through mrgl. The summary counts both; `--json` carries `managed`, `source` and `version` per
addon.

On Forever, `add --dry-run` and `list` show a Forever note for each addon. `forever build: yes`, `no` or
`unknown` says whether the addon's source publishes a Forever build at all: its `release.json` names one,
names other flavors only, or nothing says. A curated note, when there is one, says why an addon with a
Forever build still only half works there (Forever hides combat values from addons), with the date it was
verified and its source; a note older than 90 days is marked stale. `list` stays offline and reads what the
last `add` of the same source fetched, so before that the build is `unknown`. The notes live in
[`data/forever-notes.json`](data/forever-notes.json) and grow through pull requests.

A backup is a plain zip under `%LOCALAPPDATA%\mrgl\backups\<flavor-id>\` (or the folder `--to` names), and a
restore moves the current `WTF` folder aside into the same place before it writes, so it can be undone by
hand; it is refused while the game client runs.

`mrgl savedvars` lists every `SavedVariables\<Addon>.lua` of a flavor, account-wide and per character, the
largest first with its `.bak`, and a total. A file is an orphan when no addon folder of its name is left under
`Interface\AddOns`; a disabled addon keeps its folder, so its settings are never one, and the client's own
`Blizzard_` files are not either. `savedvars --clean` moves the orphans and their `.bak` files into
`%LOCALAPPDATA%\mrgl\backups\<flavor-id>\SavedVariables-<date>.zip`, next to the backups, and is refused while
the game client runs: nothing is deleted, and unzipping that zip into the `WTF` folder puts every file back
where it was (`backup --restore` does not take it, a restore replaces the whole `WTF` folder).

`list` and `add` check every addon's required dependencies (the TOC's `## Dependencies` and
`## RequiredDeps`) and name what the game will not find, as a warning, never a refusal:

```text
Mrgl? Mrgl…
Needy needs the missing dependency LibMissing: install it or the addon does not load
```

A dependency counts as there when an addon folder of that name is installed, or when the addon carries the
library inside its own folder (a release that is not a `nolib` build). `--json` has the details per addon
under `dependencies`.

Something not working? `mrgl doctor` prints what a bug report needs: the game folder and how it was found,
each flavor's build, AddOns folder, sidecar, cache and running client, the WebView2 version, the GitHub rate
limit and which API keys are set (never their values). Paths under your profile folder show as `~`; paste the
block into the [bug report](https://github.com/speti4/mrgl/issues/new?template=bug-report.yml).

`link` makes a directory junction under `Interface\AddOns` for every subfolder of a development folder that
has a TOC named after it (no admin rights needed), so the game loads your addons straight from your repo. A
folder of the same name already there is skipped, never replaced; an install never writes over a linked
addon, and `unlink` removes exactly the junctions `link` made, never a folder and never a file of yours.

An exported list carries each managed addon's source and version; `import` installs the current release of
every `github:` entry the flavor does not have yet, and leaves the others alone (a zip in hand stays on the
machine it came from, a linked folder is linked again). `list` and an install warn when an addon's
`## Interface` names no interface of the flavor's build, the first sign of a zip for another flavor.

## Documentation

- [Development](docs/development.md): layout, CI, releases and helper scripts
- [Changelog](CHANGELOG.md): every release and the tasks it closed, with the zips and checksums on the
  [releases page](https://github.com/speti4/mrgl/releases) once there is a release
- [Contributing](CONTRIBUTING.md) and the [security policy](SECURITY.md)
- [Why mrgl exists](docs/origin.md) and [the alternatives it was measured against](docs/research/)
- [Decisions](tasks/decisions/): build vs. adopt, the stack, the state sidecar, the public repo, the voice
- [The voice](docs/nerglish.md): how mrgl speaks murloc without ever shipping a Blizzard asset

## Project status

Pre-alpha: nothing is released or downloadable yet, and the commands answer with the task that will build
them; build from source as above. The plan lives in [`tasks/`](tasks/README.md): the backlog, the work in
flight and the history, one file per item. Windows 11 is the platform; a macOS preview is the last milestone.

mrgl is for anyone who keeps more than one WoW flavor and wants the right build of each addon in each of them.

## License

[MIT](LICENSE). World of Warcraft and the murloc are Blizzard Entertainment's; mrgl is an independent,
non-commercial tool and ships no Blizzard audio or artwork.
