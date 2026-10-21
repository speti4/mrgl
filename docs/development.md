# mrgl — development

How the repo is laid out, how CI and releases work, and what the helper scripts do. Build prerequisites, the
checks before a commit and the project rules are in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Layout

- `crates/mrgl-core` — the library: flavor detection (`flavor`), the AddOns scan and TOC parsing (`scan`,
  `toc`), the sources (`sources`: GitHub `release.json` first, later Wago, WoWInterface and CurseForge), the
  install engine (`install`), the state sidecar (`state`, D-003), backups and addon sets (`backup`) and the
  voice table (`voice`, D-005). Synthetic fixtures are under `tests/fixtures/`.
- `crates/mrgl-cli` — the `mrgl` executable: `scan`, `list` and `root` (task 002), `add`, `update`, `remove`,
  `link`, `unlink`, `backup` (stubs that answer with the task that builds them until that task lands), with
  `--json` for machines and `--serious` for the murloc-averse; its end-to-end tests under `tests/`.
- `crates/mrgl-app` — the `mrgl-app` executable: the Tauri 2 window (`src/`), the static UI under `ui/`, the
  capability file and the icon.
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

## The UI

The UI is plain HTML, CSS and JavaScript embedded in the exe: no Node toolchain, no bundler, no Tauri CLI. A
change under `crates/mrgl-app/ui/` takes effect on the next `cargo run`. Pure modules get Node tests under
`crates/mrgl-app/tests/ui/`, which `cargo test` runs through `tests/ui_smoke.rs` when `node` is on `PATH`.
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
`X-WoWI-ID` headers and the other shapes its README lists), `release.json` files in the shape the BigWigs
packager writes (`forever` next to other flavors), and small zips with several addon folders in one archive.
A real addon zip, a real AddOns folder or a `WTF` folder never enters the repo; a manual round on a real game
folder is a task of its own, and its result is a dated line in that task's Verification.

The command line keeps `config.json` (the remembered game folder) under the mrgl home folder,
`%LOCALAPPDATA%\mrgl\` (D-003); `MRGL_HOME` overrides the home folder, and the end-to-end tests set it to a
temporary folder so no test reads or writes the real one. `MRGL_ROOT` names the game folder for one run,
between the `--root` flag and the remembered one.
