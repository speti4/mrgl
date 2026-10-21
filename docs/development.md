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
  and `page`, the release's page, both left out when absent, task 030; later Wago, WoWInterface and
  CurseForge), the install engine (`install`: a zip into
  a staging folder once every entry passed `zip_entry`, its addon folders swapped into `Interface\AddOns`
  one at a time with a revert, the previous version's zip kept; task 004; its commit and its removal ask
  the running-client check once before the first folder moves, and a running client is a warning on the
  result, never a refusal, task 038; it refuses to install over a linked addon or a junction it did not
  make), the dev addon links (`link`, task 005: a directory junction under `Interface\AddOns` for every
  addon of a development folder, a mount-point reparse point made through the Windows API, recorded in the
  sidecar with the source `link:<development folder>`, the TOC's version and no file list; `unlink` removes
  exactly those junctions, never a real folder or a junction mrgl did not make; Windows-only), the state
  sidecar (`state`, D-003), the toolbox (`backup`: the `WTF`
  backup into a dated zip, the listing and the restore that checks every entry through `zip_entry`, moves
  the current `WTF` aside and reverts on a failure, task 013; later the addon sets), the zip entry check
  both of them call (`zip_entry`, task 044: every entry readable, inside its folder and no link, checked
  before the first is written), the running-client check (`client`,
  task 023: a process found by its executable's path under the flavor folder, the guard every `WTF` write
  calls and the install engine's warning), the mrgl home folder with the remembered game root (`home`,
  D-003), the game root resolution (`discover`, task 020: the flag, `MRGL_ROOT`, the remembered root, then
  the launcher's registry key and the usual install folders, behind a lookup trait with a stub), the
  dependency check (`dependencies`, task 029: every required name of the TOC the flavor loads, `## Dependencies`,
  `## RequiredDeps` and the other `## Dep…` lines, ASCII case-insensitive, is *met* when an addon folder of
  that name is in the scan, *embedded* when a folder of that name sits inside the addon's own folder, at
  most three levels down, which a build without `nolib` carries, and *missing* otherwise; an optional
  dependency is never reported; the sidecar entry's `nolib` flag, recorded from the pick at install, turns
  the embedded lookup off for a `nolib` build, and an entry without it is unknown, embedded possible), the
  Forever notes (`forever`, task 011: the build signal `yes`, `no` or `unknown` that
  `sources::github::forever_build` derives from a release, and the curated `data/forever-notes.json`
  compiled in with `include_str!`, with the stale rule), the diagnostic report (`doctor`, task 024: the
  facts a bug report needs, with the profile folder as `~`, see [The doctor](#the-doctor)) and the voice
  table (`voice`, D-005). Synthetic fixtures are under `tests/fixtures/`.
- `crates/mrgl-cli` — the `mrgl` executable: `scan`, `list` and `root` (task 002), `add github:<owner>/<repo>
  --flavor <flavor> [--dry-run]` (task 003: resolves and reports the pick, with the release notes' line and
  link under it when the release has notes, task 030; the install from the pick is task 040), `add
  <file.zip>` and `remove` (task 004; a running client is a warning line and `client_warning` in `--json`,
  task 038), `backup` with `--list`, `--restore` and `--yes` (task 013), `doctor [--flavor <flavor>]`
  (task 024), `link <folder>` and `unlink <folder>` (task 005), `update` (a stub that answers with the task
  that builds it until that task lands), with `--json` for machines and `--serious` for the
  murloc-averse; its end-to-end tests under `tests/`. `list` and `add <file.zip>` run the dependency check
  (task 029; `scan` reads the folder only): a warning line per addon with a missing required dependency,
  `missing: <names>` at the end of the addon's `list` line, and in `--json` a `dependencies` object per
  addon of `list` and a `dependencies` array of the installed folders under `add`'s `addon`, each with
  `folder`, `required`, `met`, `embedded`, `missing` and `nolib` (`true`, `false` or `null` for unknown).
- `crates/mrgl-app` — the `mrgl-app` executable: the Tauri 2 window (`src/`), the static UI under `ui/`, the
  capability file and the icon.
- `data/` — `forever-notes.json`, the curated Forever notes compiled into the core (see
  [Forever notes](#forever-notes)).
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

## The voice

Every human-readable message of the CLI and the window goes through `mrgl_core::voice`: a murloc line and a
plain English line, the murloc one picked by the message's kind (banner, progress, success, nothing to do,
warning, error, fatal) from the table in `voice.rs`. JSON output (`--json`) and log lines are plain.
`--serious` or `MRGL_SERIOUS=1` drops the murloc line. Tests assert the plain line, so a change to the murloc
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
unknown `--flavor` fails the command. Every path under the user's profile folder (`USERPROFILE`, in its long
and its 8.3 short form) is shown with that folder as `~`, and an API key is named, never read out, so a pasted
report carries no home folder path and no secret.

The lines, with the JSON field in brackets:

- `mrgl: <version>` (`mrgl_version`): the version of the exe.
- `root: <folder> (<origin>)` (`root`, `root_origin`): the game root the run resolved, as every command does,
  and how: `from --root`, `from MRGL_ROOT`, `remembered`, `found in the launcher's registry key`, `found in a
  usual install folder` (`flag`, `environment`, `remembered`, `registry`, `common_folder`); `none found` when
  nothing names or finds one.
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
HTTP stub and a fixed WebView2 answer; the end-to-end tests stub the WebView2 version with a
`webview2=<version>` item of `MRGL_DISCOVERY` (any other value of the variable is no runtime, so `off` reads
no registry), answer the rate limit from `releases/api.github.com/rate_limit.json` and assert that neither a
key's value nor a `\Users\` path is in the output.

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
  toggle and the serious switch sit at the bottom of the sidebar.
- `style.css` holds every color as a token on `:root`: the light set, the dark set under
  `prefers-color-scheme: dark` (unless the toggle says light) and under `data-theme="dark"` (the toggle), one
  token per state (managed, update available, linked, guess only, unmanaged) and per source badge (github,
  wago, wowi, curseforge, zip, linked; a dashed badge is a source known only from a TOC hint).
- `main.js` is the view: it reads `window.Data`, draws what it gets and never composes a murloc line. It does
  not know whether the rows are made up.
- `mock-data.js` is the data module of the sketch, made-up rows in the shape the core answers with; its header
  comment is the contract (`Data.version()`, `Data.flavors()`, `Data.updateAll(flavorId, report)`, a flavor
  as `flavor.rs` serializes it plus its friendly name and addons, an addon as `scan.rs` gives it plus the
  facts of the sidecar, the sources, the update check, the Forever notes and pin / ignore). Task 009 adds the
  module that invokes the Tauri commands behind the same `window.Data`, `index.html` loads that one instead,
  and the layout stays; the mock stays for a look at the page without the core.
- The pure modules, each with a test file of the same name under `tests/ui/`: `state.js` (an addon's one state
  from its facts, with the precedence linked, update, managed, guess, unmanaged; the classes and labels),
  `search.js` (every word of the query in the name, folder, source kind or source id), `theme.js` (auto,
  light, dark), `flavors.js` (the navigation order and the friendly names) and `voice.js` (the two lines and
  where a message goes, see [nerglish.md](nerglish.md), "The window").

The application icon is generated once by `scripts\make-icon.py` (Pillow; run it as
`uv run --with pillow python -I scripts\make-icon.py`) and committed: `icon.ico` for the
Windows exe and `icon.png` for the window icon on the other targets. It is a glyph of our own, never Blizzard
artwork.

## CI and releases

`.github/workflows/ci.yml` runs five jobs on every push to `main` and every pull request: `test`
(windows-latest: `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`), `lint` (ubuntu:
`cargo fmt --check` and `scripts/changelog.py --check`, the latter skipped on a tag, see
[The changelog](#the-changelog)), `msrv` (windows-latest: `cargo check` with the toolchain that `rust-version`
in `Cargo.toml` names, so nothing newer than the declared minimum slips in; it runs on Windows because the
Windows target needs tauri-build's resource compiler, which a Linux runner lacks), `deny` (`cargo deny check`
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

Some of the setup lives in the repository settings, not in a file: CodeQL runs as GitHub's default setup; a
ruleset protects `main` (changes arrive through a pull request with squash merge only, the checks `test`,
`lint`, `msrv`, `deny`, `workflow-lint` and `commit-rules` must pass, the history stays linear, and force
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

Pushing a tag `v<version>` runs the five jobs; when they pass, the `release` job checks that the tag is on
`main`, installs the pinned cargo-about, runs `release.ps1`, attests the build provenance of the zip and
publishes it with the checksum file as a GitHub release titled `mrgl <version>`, with the notes file as its
text, through the runner's `gh` CLI. The release job builds without the Rust cache, so nothing a pull request
run cached reaches the zip. A downloaded zip is checked against its attestation with
`gh attestation verify <zip> --repo speti4/mrgl`. A usage guide inside the zip and signed exes are M5.

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
does). An issue is found again by a hidden marker in its body, so a run patches only what differs; an issue
whose file is gone is reported as an orphan and left alone, and a file without a numbered name is reported and
not mirrored.

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
one addon folder in three versions, several folders in one archive, a TOC at the zip's root, a wrapper
folder, a path that tries to escape the staging folder, an addon with an embedded and a missing required
dependency, and a zip with no addon at all), a hand-written
`WTF` tree in the shape the client writes (`wtf/WTF/`: a `Config.wtf`, an account with its caches and
SavedVariables, a realm and a character with its `AddOns.txt` and SavedVariables; a few hundred bytes, made
up from the first line, task 013), a third game folder for the dependency check (`dependencies/`, task
029: a `_forever_` flavor whose addons have a met, an embedded and a missing required dependency, and a
library folder inside an addon that a `nolib` entry in a test's sidecar turns into a missing one), and a
development folder for the junction links (`dev/`: two addons with a TOC and a folder without one, task
005; the tests copy it into a temporary folder and make their junctions there). A real addon zip, a real
AddOns folder or a `WTF` folder never enters the
repo; a manual round on a real game folder is a task of its own, and its result is a dated line in that
task's Verification. The running-client tests copy their own test binary into a temporary flavor folder and
start it as a stand-in client (it waits for its stdin to close), so no game file and no committed
executable is involved; the backup tests copy the fixture `WTF` tree into such a folder.

The sources talk to the network through the `Http` trait of `mrgl_core::sources`. `MRGL_HTTP_STUB=<folder>`
makes the command line answer every request from that folder instead (the URL's host and path under it,
`.json` appended when the bare path is no file, a 404 for a missing file, an `ETag` from the content and a
304 for a matching `If-None-Match`), which is how the end-to-end tests run `add` offline against
`tests/fixtures/releases/`; a unit test builds a `FolderHttp` or an in-memory stub directly.
`MRGL_GITHUB_TOKEN` holds a personal access token for the GitHub API (60 requests an hour without one); it is
sent to `api.github.com` only, never with an asset download. The release list of a repository is cached with
its `ETag` under `<home>/cache/github/<owner>/<repo>/releases.json`, so a repeated run costs one conditional
request; the cache can be deleted any time. Next to it, `release-json.json` keeps the current release's
`release.json` with that release's tag (task 011): this is how `list` reads the Forever build offline. Every
fixture repository under `releases/` is classified for the Forever build in `tests/github_source.rs`, so a
new fixture names its expected build there.

The build version fallback (task 019: the client exe's version resource when `.build.info` has no row for
the flavor) is tested against a second synthetic game folder, `client-exe/`: a `.build.info` with a row for
retail only, and client exe stubs written by the deterministic `make-stubs.py` next to them (`python -I
crates/mrgl-core/tests/fixtures/client-exe/make-stubs.py` rewrites the same bytes). A stub is a minimal
PE file with one `.rsrc` section and no code: `_classic_era_\WowClassic.exe` carries a version resource
(the fallback's happy path), `_retail_\Wow.exe` carries one the `.build.info` row wins over,
`_classic_\WowClassic.exe` has no version resource (the build stays unknown), `_forever_` has no exe at
all, and `stubs\` holds a 32-bit image and a resource without a string table for the parser's unit tests.
No stub is a game executable or a copy of one.

The command line keeps `config.json` (the remembered game folder) under the mrgl home folder,
`%LOCALAPPDATA%\mrgl\` (D-003), and under it one folder per flavor id (`wowf\`, `wow\`, …) with the state
sidecar `state.json`, the install engine's `staging\` (a zip is unpacked and checked here before anything
moves; emptied after every install) and `cache\<addon>\<version>.zip` (the current and the previous
version's zip for a later rollback; never required, deleting it only disables the rollback), and the `WTF`
backups under `backups\<flavor-id>\`: `WTF-<date>.zip` per backup (a plain zip, the date local, so the
names sort by age) and `WTF-before-restore-<date>\` for every `WTF` folder a restore moved aside; nothing
prunes that folder, deleting a zip or an aside folder by hand is fine. `MRGL_HOME` overrides the home
folder, and the end-to-end tests set it to a temporary folder so no test reads or writes the real one. A
run's game folder is resolved in one order by
every command (`discover::resolve_root_of_process`, task 020): the `--root` flag, then `MRGL_ROOT`, then the
remembered one, then the discovery: the launcher's registry value (`HKLM\SOFTWARE\WOW6432Node\Blizzard
Entertainment\World of Warcraft`, `InstallPath`; a value that names a flavor folder counts for its parent),
then `Program Files (x86)\World of Warcraft` and the same under `Program Files`. A discovered hit counts only
when it is a game root (a `.build.info` and at least one flavor folder); it is used for the run and reported
with its origin (a plain line in `list`, the answer of `mrgl root` with no argument, `root_origin` in
`--json`), and never written to `config.json`: `mrgl root <folder>` is the one place that remembers it.
`MRGL_DISCOVERY` replaces the lookup for one run: `off` finds nothing, and a `;`-separated list
(`registry=<path>` for the launcher's value, `webview2=<version>` for the doctor's WebView2 version, every
other item a usual folder) is the stub the end-to-end tests run on, so no test reads the real registry or
looks under `Program Files`.

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
in the future, and a source that is not an `https://` URL. A note whose `verified` date is more than 90 days
old (`forever::STALE_AFTER_DAYS`) is shown as stale until someone checks it again and moves the date; nothing
checks the age in CI.
