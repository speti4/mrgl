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
  for a backup restore or an addon set you asked for. It lives next to the CurseForge app without a fight.
- **A toolbox, not only a manager.** WTF backup and restore, addon sets, a junction link for the addons you
  develop yourself, and a window on top of it all.
- **A murloc voice.** Every message is a murloc line and a plain English line; `--serious` (or
  `MRGL_SERIOUS=1`) keeps only the English, and JSON output is always plain ([docs/nerglish.md](docs/nerglish.md)).

## Build from source

mrgl is a Rust core with a `clap` command line and a Tauri 2 window on WebView2; why is in
[D-002](tasks/decisions/D-002-rust-tauri2-portable-zip-cf-aware.md). On Windows you need Rust stable with
the MSVC toolchain and the Visual Studio 2022 C++ build tools (details in [CONTRIBUTING.md](CONTRIBUTING.md)):

```powershell
cargo build --release
target\release\mrgl.exe --version
target\release\mrgl.exe list --flavor forever                               # what is installed, flavor-correctly
target\release\mrgl.exe root "D:\Games\World of Warcraft"                   # remember a game folder installed elsewhere
target\release\mrgl.exe add github:<owner>/<repo> --flavor forever --dry-run  # which release asset is the Forever one
target\release\mrgl.exe add SomeAddon-1.2.3.zip --flavor forever            # install a zip in hand
target\release\mrgl.exe remove SomeAddon --flavor forever                   # remove an addon mrgl installed
target\release\mrgl.exe backup --flavor forever                             # the WTF folder into a dated zip
target\release\mrgl.exe backup --list --flavor forever                      # the backups so far
target\release\mrgl.exe backup --restore WTF-2026-10-21-193605.zip --flavor forever   # one back, asks first
```

On a standard install no flag is needed: mrgl finds the game folder through the launcher's registry key or
under `Program Files (x86)` / `Program Files` and says so; `mrgl root` with no argument shows the folder a
run would use and where it came from, and `mrgl root <folder>` remembers one (also `--root <folder>` or
`MRGL_ROOT` for one run).

A backup is a plain zip under `%LOCALAPPDATA%\mrgl\backups\<flavor-id>\` (or the folder `--to` names), and a
restore moves the current `WTF` folder aside into the same place before it writes, so it can be undone by
hand; it is refused while the game client runs.

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
