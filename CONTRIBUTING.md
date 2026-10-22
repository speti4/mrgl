# Contributing to mrgl

Thank you for taking the time. mrgl is a small project with a few firm rules: the game folders stay yours
(mrgl writes under `Interface\AddOns`, under `WTF` only for a backup restore or an addon set the user asked
for, and never touches the client), the shipped artifact stays one portable zip with two exes and nothing to
install, and the task files are the plan. Everything below follows from these.

## Start with an issue

Open an issue before a pull request: a [bug report](https://github.com/speti4/mrgl/issues/new?template=bug-report.yml)
or a [feature request](https://github.com/speti4/mrgl/issues/new?template=feature-request.yml). A short
exchange about the approach saves a rewrite. Security problems go to a private report instead
([SECURITY.md](SECURITY.md)).

The issues labeled `mirrored` are the task files' public face: every numbered file under `tasks/` is one
issue, mirrored one way, and a comment on it is welcome. The file is what the maintainer edits; the next sync
overwrites the issue body.

## No game data, no Blizzard assets

Nothing from a game install enters the repo: no real addon zip, no `WTF` folder, no account or character
names, no screenshots of a real AddOns folder. Reproduce a problem with a made-up addon: a folder with a TOC
and a `release.json` of a few lines. The fixtures under `crates/mrgl-core/tests/fixtures/` are all synthetic
and show the shapes that matter (multi-TOC suffixes, `## Interface` lists, the `X-*-ID` headers, several
folders in one zip, a `release.json` with several flavors). `.gitignore` keeps `*.zip`, `AddOns/` and `WTF/`
out of the repo, with the fixtures as the only exception. Blizzard audio and artwork never enter the repo
either: the murloc voice is text only (decision D-005 in `tasks/decisions/`).

## Forever notes

A Forever note says why an addon that has a Forever build still only half works there. Add or update one in
[`data/forever-notes.json`](data/forever-notes.json) through a pull request: the addon's source spec
(`github:<owner>/<repo>`) as the key, the restriction it runs into (`kind`), a one-line `reason`, the date you
`verified` it and the `source` URL that shows it. State the fact and its source, nothing more; `cargo test`
checks the file's shape, and takes a `verified` date up to one day after today in UTC, so your local day
passes east of UTC too (the rule for adding a note:
[docs/development.md](docs/development.md#forever-notes)).

## Build prerequisites

On Windows 11 you need:

- Rust via rustup, with the `x86_64-pc-windows-msvc` target. `rust-toolchain.toml` pins a stable version,
  which rustup installs on the first `cargo` call in the repo
  ([docs/development.md](docs/development.md#the-rust-toolchain)).
- The Visual Studio 2022 C++ build tools and a Windows 11 SDK, which the MSVC linker needs.
- Node.js (optional). `cargo test` runs the UI's Node tests when `node` is on `PATH` and skips them
  otherwise.

```powershell
winget install Rustlang.Rustup
# Visual Studio Installer: add "MSVC v143 build tools" and "Windows 11 SDK"
cargo build --release
cargo run -p mrgl-cli -- scan <flavor folder>   # the command line, during development
cargo run -p mrgl-app                            # the window
```

The UI under `crates/mrgl-app/ui/` is plain HTML, CSS and JavaScript embedded in the exe, with no bundler and
no Tauri CLI. A change there takes effect on the next `cargo run`. The layout, the CLI, the release and the
helper scripts are described in [docs/development.md](docs/development.md).

## Branches, pull requests and commits

Work on a branch and open a pull request; it is squash-merged once CI is green, so its title and description
become the commit message. Commit subjects and pull request titles are imperative and start with a type in
lower case, `type: subject` or `type(scope): subject`, the type one of `feat`, `fix`, `docs`, `ci`, `chore`,
`tasks` and `deps`; a `Merge …` or `Revert "…"` subject passes as git writes it. The hook below checks this. The
full rules are in [docs/git-workflow.md](docs/git-workflow.md). Enable the hook once per clone:

```powershell
git config core.hooksPath .githooks
```

## Checks before a commit

The same checks as CI, job by job ([docs/development.md](docs/development.md#ci-and-releases)). Every change:

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
# The oldest supported Rust, the rust-version of Cargo.toml (rustup toolchain install 1.90 --profile minimal, once).
cargo +1.90 check --all-targets --locked
python -I scripts/changelog.py --check
```

`--locked` fails when `Cargo.lock` is out of step with the manifests instead of rewriting it. The changelog
check wants `CHANGELOG.md` to match `tasks/done/` exactly, so a change that closes a task runs `python -I
scripts/changelog.py` and commits the result with it. The commit messages and the pull request text follow the
commit rules, which the hook above checks locally and the `commit-rules` job again in CI.

When `Cargo.toml`, `Cargo.lock` or `.github/` change, also:

```powershell
cargo deny check      # Cargo.toml, Cargo.lock, deny.toml, about.toml
uvx zizmor .github    # .github/
```

CI (`.github/workflows/ci.yml` and `commit-rules.yml`) runs all of them on every push to `main` and every pull
request, and one more that a Windows machine cannot: the `non-windows` job compiles the `cfg(not(windows))`
branches on Linux, so a change to a Windows-only path keeps its non-Windows twin in step. A change in behavior comes with a test: Rust tests sit beside the code or under each crate's
`tests/`, and the UI modules have Node tests under `crates/mrgl-app/tests/ui/`. Tests never touch the network
or a real game folder: sources are tested against the fixtures and a local HTTP stub.

## Dependencies: nothing to install

The shipped artifact is one portable zip with two exes, with nothing for the user to install (decision D-002
in `tasks/decisions/`). WebView2 is part of Windows 11; everything else is compiled in. A new crate is fine
when it earns its place. Add it to `[workspace.dependencies]` in `Cargo.toml` with a one-line comment saying
why it is needed and why the existing ones do not cover it. Crates that compile C code or pull a runtime
download need a strong reason. The crate's license has to be on the allowlist in `deny.toml` and its source
has to be crates.io; `cargo deny check` says so before CI does.

## Tasks and decisions

The plan lives in `tasks/`: one file per item under `backlog/`, `wip/`, `done/` and `dropped/`, decisions
under `decisions/`, milestones in `tasks/roadmap.md`, and a generated index in `tasks/README.md`. The
maintainer turns an accepted issue into a task file. `scripts/sync-issues.py` mirrors every numbered task and
decision into a GitHub issue labeled `mirrored`, one way
([docs/development.md](docs/development.md#the-issue-mirror)): edit the file, never the issue; the next sync
overwrites the issue body.

## Conventions

- Rust edition 2024. `cargo fmt` and `clippy -D warnings` are clean.
- Code, comments, commit messages, docs and UI strings are in English with American spelling (license,
  behavior, color); Blizzard's own terms are written as Blizzard writes them. The murloc lines come from the
  voice table in `mrgl-core` ([docs/nerglish.md](docs/nerglish.md)); tests assert the plain English line.
- File, folder, branch and crate names are kebab-case and descriptive, without ambiguous abbreviations.
- Commit subjects are imperative and start with a type: `feat:`, `fix:`, `docs:`, `ci:`, `chore:`, `tasks:`,
  `deps:` (dependabot), or the same with a scope, `fix(core):`.

## License

mrgl is MIT-licensed ([LICENSE](LICENSE)). By contributing, you agree that your contribution is licensed
under the same terms.
