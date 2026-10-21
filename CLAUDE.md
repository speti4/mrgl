# mrgl

A flavor-correct World of Warcraft addon manager and toolbox for Windows 11, Forever-first: Rust core, a `mrgl`
command line and a Tauri 2 window (WebView2), one portable zip with two exes, nothing to install. It installs
and updates addons per flavor (`_retail_`, `_classic_`, `_classic_era_`, `_forever_`) from GitHub release
metadata, Wago, WoWInterface and hand-picked zips, is CurseForge-aware with the user's own API key but never
CurseForge-dependent, writes under `Interface\AddOns` (and under `WTF` only for a backup restore or an addon
set the user asked for), and never touches the game client.

## Layout
- `crates/mrgl-core` (flavor detection, AddOns scan and TOC parsing, sources, the install engine, the state
  sidecar, backups, the voice table), `crates/mrgl-cli` (the `mrgl` exe), `crates/mrgl-app` (the `mrgl-app` exe:
  Tauri 2 window, static UI under `ui/`)
- `docs/` (origin, research, development, git workflow, maintainer sessions, the voice), `tasks/`, the
  community files at the root and under
  `.github/`; the full layout is in `docs/development.md`

## Commands
```powershell
cargo build --release
cargo run -p mrgl-cli -- scan <flavor folder>   # the command line
cargo run -p mrgl-app                            # the window
cargo test
```
Before a commit: the checks in CONTRIBUTING.md, "Checks before a commit" (more of them when `Cargo.toml`,
`Cargo.lock`, `.github/` or the docs change); a change that closes a task regenerates `CHANGELOG.md`.

## Data outside the repo (never commit)
- The game folders, the `WTF` folder, a real addon zip or a downloaded release never enter the repo; test
  fixtures are small synthetic addons, TOCs, `release.json` files and zips only (`crates/mrgl-core/tests/fixtures/`)
- The state sidecar, the install engine's staging folder and its zip cache (`<flavor-id>\state.json`,
  `staging\`, `cache\<addon>\<version>.zip`), the `WTF` backups (`backups\<flavor-id>\WTF-<date>.zip`, and
  the `WTF-before-restore-<date>\` folders a restore moves aside), the SavedVariables cleanup zips
  (`backups\<flavor-id>\SavedVariables-<date>.zip`) and the CLI's `config.json` (the remembered
  game folder) live under `%LOCALAPPDATA%\mrgl\` (D-003; `MRGL_HOME` overrides it, tests point it at a temp
  folder); the WoW install path of the maintainer's machine stays in the gitignored `CLAUDE.local.md`
- Blizzard audio and artwork never enter the repo: the murloc voice is text only (D-005)

## Conventions
- Project conventions (language, names, docs to update) are in CONTRIBUTING.md; a new crate follows its
  "Dependencies: nothing to install" section (D-002): the one-line reason is a comment in `Cargo.toml`
- Every human-readable CLI message goes through `mrgl_core::voice` (D-005): a murloc line and a plain English
  line; JSON and log output stay plain; `--serious` or `MRGL_SERIOUS=1` turns the murloc off
- Git: branches, pull requests and commit rules follow @docs/git-workflow.md

## Tasks
- `tasks/` follows the project-pal task standard (`tasks/README.md`): approved plan → `/tasks promote` (or
  `/tasks add` for later), closing files every follow-up with `/tasks add`, `/tasks check` before a task commit;
  a task file reads as public text: it states decisions and results, not conversations; a decision stays
  `proposed` until the maintainer decides it
- `scripts/sync-issues.py` mirrors every numbered task and decision into a GitHub issue, one way (the
  `sync-issues` workflow, on every push to `main` that touches `tasks/`): edit the file, never the issue; a
  task file's H1 and Context are the issue text
- The repo is public: a task file's `plan:` field is `none` (a plan file lives outside the repo and its name is
  not public text), and no file carries a home folder path (`C:\Users\<name>\…`)
