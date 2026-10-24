# mrgl

<!-- guide: start -->

A flavor-correct World of Warcraft addon manager for Windows 11, Forever-first: one portable zip with two exes
(the `mrgl` command line and the `mrgl-app` window), nothing to install, no Electron, and it never touches
your game client.

<!-- guide: end -->

[![CI](https://github.com/speti4/mrgl/actions/workflows/ci.yml/badge.svg)](https://github.com/speti4/mrgl/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/speti4/mrgl)](https://github.com/speti4/mrgl/releases/latest)
[![License: MIT](https://img.shields.io/github/license/speti4/mrgl)](LICENSE)

<!-- guide: start -->

A run of `update --all` over two managed addons, one of them behind its release:

```text
> mrgl update --all --flavor forever --yes
Mrrrgl… mrgl mrgl.
checking the managed addons of _forever_ against their sources
Mrrrgl… mrgl mrgl.
checking Plain against github:Example/Prerelease (1 of 2)
Plain  1.0.0  up to date
Mrrrgl… mrgl mrgl.
checking Questie against github:Questie/Questie (2 of 2)
Questie  1.2.2 -> 1.2.3  update available
Mrrrgl… mrgl mrgl.
updating Questie from 1.2.2 to 1.2.3
Mrrrgl… mrgl mrgl.
downloading Questie-v1.2.3-forever.zip
Mrrrgl… mrgl mrgl.
installing Questie-v1.2.3-forever.zip into _forever_
Mrglglglgl!
updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in _forever_
Mrglglglgl!
1 addon updated, 1 up to date
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
- **Updates you can read first.** `mrgl update --all --dry-run` checks every addon mrgl installed against
  its source, each on its own release channel, and says where each stands: up to date, update available, a
  guess only (no `release.json` names the flavor, so it is not installed blindly), source gone, pinned,
  ignored, pending approval (a WoWInterface file the site has not approved yet), only on CurseForge (an
  author who allows no download elsewhere: the line names the page and how its zip goes in), not checked
  (a zip in hand, a linked folder), or newer than the release (a deleted release, a
  channel switch: never downgraded); `mrgl update --all` installs the updates after one confirmation
  (`--yes` for scripts), leaves a pinned addon alone and skips an ignored one. It says which addon it is
  checking as it goes, and without a GitHub token it stops asking GitHub before the 60 requests an hour run
  out, with the time the limit resets, instead of failing addon after addon; a source's limit stops that
  source alone, and the addons of the other sources are still checked. The window does the same per flavor: "Check
  for updates", "Update all" with a progress bar per addon and a cancel that lets the addon in flight
  finish, "Add a zip…" for an addon zip from disk with the source it comes from (so the update check asks
  that source, as for a CurseForge addon whose author allows no download elsewhere), and on each row its
  update, pin, ignore and release channel, and the interface mark of `scan`
  and `list` (`interface mismatch`, `out of date`) with the warning's line in its tooltip; what the last
  check found that needs the network to know again (a source gone, a failed check, the addons left before
  the rate limit), the window's own or `mrgl update`'s, stays on the rows across a mark and a restart
  until the next check.
- **A known-good set on another machine.** `mrgl export` writes the managed addons into a list file and
  `mrgl import` installs it elsewhere, at the current releases, or with `--exact` at the versions the list
  names; a version the source no longer offers is reported and skipped.
- **A toolbox, not only a manager.** WTF backup and restore, addon sets (a raid set, a levelling set: the
  enabled addons of every character switched with one command), the SavedVariables sizes with the cleanup
  of files whose addon is gone, a junction link for the addons you develop yourself, and a window on top of
  it all. The window's Toolbox tab of each flavor applies a saved set to every character or to one picked
  by name (and by account, when one realm and name sit under two), takes the last apply back with its undo
  button, edits a saved set from its chips (enable, disable or drop one addon), and lists the
  SavedVariables with their sizes and the orphans, which one button moves into a dated backup zip; the
  apply, the undo and the cleanup refuse while the game runs.
- **Lists instead of a catalog.** An exported list names sources and versions only, so a friend can import
  it: a zip in hand or a linked folder stays out of it, and a list that names a local path is refused. The
  repo keeps a Forever starter list, the usual addons whose GitHub releases carry a Forever build, at
  [`lists/forever-starter.json`](lists/forever-starter.json); one command installs it from a copy of the
  file: `mrgl import lists/forever-starter.json --flavor forever` (add `--dry-run` to see the picks first).
  The release zip carries the list at the same path, so the command works from the unpacked zip as well.
- **A murloc voice.** Every message is a murloc line and a plain English line; `--serious` (or
  `MRGL_SERIOUS=1`) keeps only the English, and JSON output is always plain ([docs/nerglish.md](docs/nerglish.md)).

## Install

Download `mrgl-<version>.zip` and `SHA256SUMS.txt` from the
[latest release](https://github.com/speti4/mrgl/releases/latest), check the zip and unpack it into a folder
of your choice. Nothing is installed: the folder holds `mrgl.exe` (the command line), `mrgl-app.exe` (the
window), `GUIDE.html` (how to use mrgl: the user sections of the README as one page that opens offline),
`THIRD-PARTY-LICENSES.html` and `lists\forever-starter.json`, and WebView2 is part of Windows 11. The exes are
not signed yet, so SmartScreen may ask once before the first start.

```powershell
(Get-FileHash mrgl-0.1.0.zip -Algorithm SHA256).Hash      # the value in SHA256SUMS.txt, in capitals
gh attestation verify mrgl-0.1.0.zip --repo speti4/mrgl   # built by this repository's release workflow
Expand-Archive mrgl-0.1.0.zip mrgl; cd mrgl
.\mrgl-app.exe                                            # the window
.\mrgl.exe import lists\forever-starter.json --flavor forever --dry-run   # the starter list's picks
```

<!-- guide: end -->

## Build from source

mrgl is a Rust core with a `clap` command line and a Tauri 2 window on WebView2; why is in
[D-002](tasks/decisions/D-002-rust-tauri2-portable-zip-cf-aware.md). On Windows you need Rust stable with
the MSVC toolchain and the Visual Studio 2022 C++ build tools (details in [CONTRIBUTING.md](CONTRIBUTING.md)):

```powershell
cargo build --release
```

<!-- guide: start -->
<!-- guide: section The command line -->

```powershell
target\release\mrgl.exe --version
target\release\mrgl-app.exe                                                 # the window: every flavor's addons, their updates and states, its toolbox (sets with undo and edit, SavedVariables, adopt)
target\release\mrgl.exe list --flavor forever                               # what is installed, managed by mrgl or not
target\release\mrgl.exe root "D:\Games\World of Warcraft"                   # remember a game folder installed elsewhere
target\release\mrgl.exe add github:<owner>/<repo> --flavor forever            # install the release's Forever asset
target\release\mrgl.exe add github:<owner>/<repo> --flavor forever --dry-run  # which asset it would be, nothing installed
target\release\mrgl.exe add wowi:<id> --flavor retail                       # a WoWInterface project's file, no key needed
target\release\mrgl.exe add wago:<id> --flavor forever                      # a Wago addon's release, with your MRGL_WAGO_KEY
target\release\mrgl.exe add https://addons.wago.io/addons/<slug> --flavor forever  # the same by its Wago page, recorded as wago:<id>
target\release\mrgl.exe add curse:<id> --flavor retail                      # a CurseForge project's file, with your MRGL_CURSEFORGE_KEY
target\release\mrgl.exe add SomeAddon-1.2.3.zip --flavor forever            # install a zip in hand
target\release\mrgl.exe adopt --flavor retail                               # where the addons mrgl did not install come from
target\release\mrgl.exe adopt --flavor forever --apply                      # let mrgl manage them from now on, nothing downloaded
target\release\mrgl.exe remove SomeAddon --flavor forever                   # remove an addon mrgl installed
target\release\mrgl.exe link D:\dev\my-addons --flavor forever              # a junction per addon you develop
target\release\mrgl.exe link D:\dev\MyAddon --flavor forever                # or one addon whose repo is its folder
target\release\mrgl.exe unlink D:\dev\my-addons --flavor forever            # exactly those junctions go again
target\release\mrgl.exe pin SomeAddon 1.2.0 --flavor forever                # keep it at 1.2.0 (no version: the installed one)
target\release\mrgl.exe unpin SomeAddon --flavor forever                    # let updates reach it again
target\release\mrgl.exe rollback SomeAddon --flavor forever                 # the previous version back from the kept zip, pinned
target\release\mrgl.exe update --all --flavor forever --dry-run             # where every managed addon stands
target\release\mrgl.exe update --all --flavor forever                       # install the updates, asks first (--yes: no)
target\release\mrgl.exe update SomeAddon --flavor forever                   # the same for one addon
target\release\mrgl.exe ignore SomeAddon --flavor forever                   # updates skip it (unignore undoes it)
target\release\mrgl.exe channel SomeAddon prerelease --flavor forever       # follow its beta builds (stable undoes it)
target\release\mrgl.exe backup --flavor forever                             # the WTF folder into a dated zip
target\release\mrgl.exe backup --list --flavor forever                      # the backups so far
target\release\mrgl.exe backup --restore WTF-2026-10-21-193605.zip --flavor forever   # one back, asks first
target\release\mrgl.exe savedvars --flavor forever                          # the SavedVariables by size, orphans marked
target\release\mrgl.exe savedvars --clean --flavor forever                  # the orphans into a backup zip
target\release\mrgl.exe set save raid --flavor forever                     # the characters' enabled addons as the set "raid"
target\release\mrgl.exe set apply raid --flavor forever                    # every character's AddOns.txt switched to it
target\release\mrgl.exe set apply raid --character Mrgl --flavor forever   # one character only (--realm for a name on two realms)
target\release\mrgl.exe set undo --flavor forever                          # the AddOns.txt files the last apply replaced, back
target\release\mrgl.exe set disable raid BagSorter --flavor forever        # edit a stored set (enable, disable, drop one addon)
target\release\mrgl.exe set list --flavor forever                          # the sets so far (show and remove take a name)
target\release\mrgl.exe doctor                                             # the diagnostic report for a bug report
Get-Clipboard | target\release\mrgl.exe key set github                     # your own API key into the Windows Credential Manager
target\release\mrgl.exe key list                                           # which keys are set and where from, never their values
target\release\mrgl.exe export --flavor forever --to forever-addons.json    # the managed addons as a list file
target\release\mrgl.exe import forever-addons.json --flavor forever --dry-run   # what installing that list would do
target\release\mrgl.exe import forever-addons.json --flavor forever --exact     # the versions the list names, not the current ones
```

`add --dry-run` names the asset it picked and why; when the release has notes, the next line is their first
line and the release page, so you know what an update brings before you take it (the plain lines, as
`--serious` prints them):

```text
pick for forever: github:<owner>/<repo> v2.1.0 (Addon-v2.1.0-forever.zip, flavor forever, interface 16001): release.json names forever (interface 16001)
release notes: Quest arrows point the right way again (https://github.com/<owner>/<repo>/releases/tag/v2.1.0)
```

<!-- guide: section Sources -->

A source spec names where an addon comes from: `github:<owner>/<repo>` (or the repository's address),
`wago:<id>` (or the addon's Wago page, `addons.wago.io/addons/<slug>`), `wowi:<id>` (or the WoWInterface
page, `www.wowinterface.com/downloads/info<id>…`) and `curse:<id>` (or `www.curseforge.com/projects/<id>`).
A Wago page names the addon by its slug: mrgl looks the slug up in Wago's public list of the flavor's game
version and installs and records the addon as `wago:<id>` (a slug that list lacks says which game version
it was looked up in). A CurseForge addon page names a slug only the keyed API knows, so mrgl refuses it and
says where the id is (the `X-Curse-Project-ID` line of the addon's TOC, or the project id on the page).
This build has all four sources: `add` with a spec of Wago or CurseForge without your key says `source not
configured` and which key is missing, an imported list reports such an entry the same way, and the update
check leaves it not checked.

A WoWInterface project has one file, and WoWInterface names no flavor for it: mrgl reads the project's
compatibility list (the game versions its author ticked) and takes the file for the flavor whose game the
list names; without such an entry, and on Forever, which no project lists yet, the pick is a guess and says
so, and the interface check after the install is the safety net. A file that waits for WoWInterface's
approval is reported and not installed; the update check counts it as `pending approval`, not as a failure,
so a daily `mrgl update --all` stays green while the site approves it. A download whose MD5 is not the one
WoWInterface names is refused before anything opens it. The source needs no key.

The Wago API needs your own key (`MRGL_WAGO_KEY`, or `mrgl key set wago`; mrgl ships none). mrgl asks Wago
for the flavor's own game version (`forever`, `retail`, `classic` for Classic Era, the progressing classic by
its build), takes the stable release or, on an addon's `prerelease` channel, the newest of the stable, beta
and alpha ones, sends the key to `addons.wago.io` only, asks for every Wago addon of a flavor in one request
when it checks for updates, and stops asking Wago in an update check before the rate limit Wago's answers
name, while the other sources are still asked. A 401 says the key is missing or not accepted.

The CurseForge API needs your own key too (`MRGL_CURSEFORGE_KEY`, or `mrgl key set curse`; mrgl ships none,
and every other source works without it). mrgl asks CurseForge for the project's files of the flavor's own
game version (Forever included), takes the newest release or, on an addon's `prerelease` channel, the newest
release, beta or alpha file, and sends the key to `api.curseforge.com` and `edge.forgecdn.net` only (the
download needs it too). CurseForge's terms allow no cache of its answers, so mrgl keeps none: the Forever
note `list` shows for a CurseForge addon is what CurseForge said when it was installed. Some authors allow no
download outside CurseForge; for such an addon `add` names its page, and the zip you download there goes in
with its source recorded, so the update check still asks CurseForge for it and says `only on CurseForge`
with the page again, counted apart and not as a failure, so a daily `mrgl update --all` stays green:

```powershell
target\release\mrgl.exe add MyAddon-2.0.0.zip --as curse:<id> --flavor forever
```

<!-- guide: section Adopting the addons you have -->

`mrgl adopt` looks at the addons mrgl did not install and says where each one comes from: the Wago,
WoWInterface and CurseForge ids and the GitHub website its TOC names, or, for an addon whose TOC names none,
a curated list of GitHub repositories ([`data/github-sources.json`](data/github-sources.json), which grows
through pull requests). It asks each source whether it publishes a build for the flavor, keeps an addon's
folders together (an addon and its options folder are one), and counts how many addons can be adopted
without CurseForge, how many only with CurseForge and how many have no known source. A source whose key you
have not set says so (`needs MRGL_CURSEFORGE_KEY`), and a folder the CurseForge app manages is named. On its
own it writes nothing.

`adopt --apply` hands those addons to mrgl: it records each one's source, its version and its files in the
state under `%LOCALAPPDATA%\mrgl\`, downloads nothing and changes no folder, so the next `update` brings
the source's build in the clean way, as for any addon mrgl installed. An addon only CurseForge publishes
waits for your CurseForge key, as one that needs any key you have not set waits for it. A folder belongs to
one manager: in a flavor folder the CurseForge app manages (its `.curseclient` marker), `--apply` refuses
unless you add `--alongside-curseforge`, and even then the CurseForge app may still update those folders;
mrgl never removes its marker, and `add`, `import` and `update` warn there too,
as the window's update actions do.

The window's Toolbox tab has the same report in its Adopt card: "Ask the sources" lists every addon mrgl did
not install with its folders, its sources and what adopt makes of it, and the counts, and one button adopts
the addons the report can adopt, with the same lines as the command line. In a folder the CurseForge app
manages a dialog asks first, and its "Adopt alongside the CurseForge app" is the window's
`--alongside-curseforge`; the warning follows the adopt.

<!-- guide: section The game folder and the addon list -->

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

<!-- guide: section Forever notes -->

On Forever, `add --dry-run` and `list` show a Forever note for each addon. `forever build: yes`, `no` or
`unknown` says whether the addon's source publishes a Forever build at all: its `release.json` names one,
names other flavors only, or nothing says (a WoWInterface project is `yes` when its compatibility list
names Forever, `unknown` otherwise; a Wago addon is `yes` with a release for Forever and `no` without, which
Wago's public addon list answers even without a key). `list` looks at the release an install would take: the newest one
of the addon's channel with a Forever zip, past a newer release built for other flavors only. A curated note, when there is one, says why an addon with a
Forever build still only half works there (Forever hides combat values from addons), with the date it was
verified and its source; a note older than 90 days is marked stale. `list` stays offline and reads what the
last `add` of the same source fetched, so before that the build is `unknown`. The notes live in
[`data/forever-notes.json`](data/forever-notes.json) and grow through pull requests.

<!-- guide: section Backups, SavedVariables and sets -->

A backup is a plain zip under `%LOCALAPPDATA%\mrgl\backups\<flavor-id>\` (or the folder `--to` names), and a
restore moves the current `WTF` folder aside into the same place before it writes, so it can be undone by
hand; it is refused while the game client runs. A restore, a set apply, a set undo and a SavedVariables
cleanup look again once they are done: a game started in the meantime is a warning, since it writes `WTF`
again when you log out, so close it and run the step again if what it wrote is lost.

`mrgl savedvars` lists every `SavedVariables\<Addon>.lua` of a flavor, account-wide and per character, the
largest first with its `.bak`, and a total. A file is an orphan when no addon folder of its name is left under
`Interface\AddOns`; a disabled addon keeps its folder, so its settings are never one, and the client's own
`Blizzard_` files are not either. `savedvars --clean` moves the orphans and their `.bak` files into
`%LOCALAPPDATA%\mrgl\backups\<flavor-id>\SavedVariables-<date>.zip`, next to the backups, and is refused while
the game client runs: nothing is deleted, and unzipping that zip into the `WTF` folder puts every file back
where it was (`backup --restore` does not take it, a restore replaces the whole `WTF` folder).

An addon set is a named list of addons, each enabled or disabled, kept per flavor under
`%LOCALAPPDATA%\mrgl\<flavor-id>\sets.json`. `set save` takes it from the characters' `AddOns.txt` (an addon
the characters disagree on is left out, with a warning), and `set apply` switches every character of the
flavor to it by rewriting their `AddOns.txt` in the game's own format: exactly the set's addons change,
every other line stays as the game wrote it, and no addon folder is deleted or moved. Like a restore, an
apply is refused while the game client runs, since the game writes those files again when you log out.
`--character <name>` (with `--realm <realm>` when two realms have a character of that name) applies a set
to that one character and leaves the others as they are, so a raid character and a levelling alt of the
same flavor can each have their own set; on `set save` it takes the set from that character alone.
`--account <name>` tells apart a realm and name that sit under two account folders. An apply keeps the
`AddOns.txt` files it replaced, so `set undo` puts the last apply back byte for byte (refused while the
game runs, like the apply); `set enable`, `set disable` and `set drop <set> <addon>` change one addon of a
stored set without touching any `AddOns.txt`, and `set save` keeps an addon that is no longer installed,
with a warning that names it.

<!-- guide: section Dependencies -->

`list`, `add`, `update` and `import` check the required dependencies (the TOC's `## Dependencies` and
`## RequiredDeps`) of every addon they list or install and name what the game will not find, as a warning,
never a refusal:

```text
Mrgl? Mrgl…
Needy needs the missing dependency LibMissing: install it or the addon does not load
```

A dependency counts as there when an addon folder of that name is installed or when it is one of the game's
own `Blizzard_` addons. A library the addon carries only inside its own folder does not count: the game
says `Dependency missing` and does not load the addon, so mrgl names it missing too and says why. `--json`
has the details per addon under `dependencies`.

<!-- guide: section API keys -->

The sources' API keys are your own: `MRGL_GITHUB_TOKEN` (a GitHub personal access token lifts the limit of
60 requests an hour), `MRGL_WAGO_KEY` and `MRGL_CURSEFORGE_KEY` for the sources that need one. mrgl reads a key
from the environment variable of that name first, then from the Windows Credential Manager, where
`mrgl key set <github|wago|curse>` stores it as `mrgl:<source>` from the standard input (pipe it in, never as
an argument, so the shell's history does not keep it) and `mrgl key clear <source>` removes it. `mrgl key
list` and `mrgl doctor` name each key and where it comes from, never its value, and a key is sent to its
own source's servers only.

<!-- guide: section When something does not work -->

Something not working? `mrgl doctor` prints what a bug report needs: the game folder and how it was found,
each flavor's build, AddOns folder, sidecar, cache, running client and any staging folder kept with the addon
folders a failed update moved aside (with what to do with it), the WebView2 version, the GitHub rate
limit and which API keys are set (never their values). Paths under your profile folder show as `~`; paste the
block into the [bug report](https://github.com/speti4/mrgl/issues/new?template=bug-report.yml).

<!-- guide: section Developing addons -->

`link` makes a directory junction under `Interface\AddOns` for every subfolder of a development folder that
has a TOC named after it (no admin rights needed), so the game loads your addons straight from your repo. A
folder that is itself an addon (a TOC named after it, `MyAddon.toc` or `MyAddon_Forever.toc`) is linked as
that one addon, so a repository that is the addon folder needs no parent. A
folder of the same name already there is skipped, never replaced; an install never writes over a linked
addon, and `unlink` removes exactly the junctions `link` made, never a folder and never a file of yours.

<!-- guide: section Lists and the interface check -->

An exported list carries each managed addon's source and version; `import` installs the current release of
every entry the flavor does not have yet whose source this run can ask (`github:`, `wowi:`, and `wago:` and
`curse:` with their keys), reports an entry of a source not configured with what is missing, and leaves the
others alone (a zip in hand stays on the machine it came from, a linked folder is linked again). `list` and
an install warn when an addon's `## Interface` names no interface of the flavor's build, the first sign of a
zip for another flavor; `scan` and `list` mark an addon `out of date` when its interface is from the flavor's expansion but older
than the installed build, as the game's own addon list does after a patch, and an install or an update says
so for the addon it just installed. The window marks its rows the same way, `interface mismatch` or
`out of date` under the row's state.

<!-- guide: end -->

## Documentation

- [Development](docs/development.md): layout, CI, releases and helper scripts
- [Changelog](CHANGELOG.md): every release and the tasks it closed, with the zips and checksums on the
  [releases page](https://github.com/speti4/mrgl/releases) once there is a release
- [Contributing](CONTRIBUTING.md) and the [security policy](SECURITY.md)
- [Why mrgl exists](docs/origin.md) and [the alternatives it was measured against](docs/research/)
- [Decisions](tasks/decisions/): build vs. adopt, the stack, the state sidecar, the public repo, the voice
- [The voice](docs/nerglish.md): how mrgl speaks murloc without ever shipping a Blizzard asset

## Project status

Pre-alpha: 0.1.0 is the first release, a portable zip for testers ([Install](#install)), or build from
source as above. The plan lives in [`tasks/`](tasks/README.md): the backlog, the work in
flight and the history, one file per item. Windows 11 is the platform; a macOS preview is the last milestone.

mrgl is for anyone who keeps more than one WoW flavor and wants the right build of each addon in each of them.

<!-- guide: start -->

## License

[MIT](LICENSE). World of Warcraft and the murloc are Blizzard Entertainment's; mrgl is an independent,
non-commercial tool and ships no Blizzard audio or artwork.

<!-- guide: end -->
