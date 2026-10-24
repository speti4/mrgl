---
size: M
origin: session 2026-10-24
created: 2026-10-24
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 121 — A hardening review of the code that touches the disk, the network and the keys

## Context

A reading review of the non-test code with one question per area: can a crafted zip, a crafted TOC or
`release.json`, a crafted source answer, a path from the command line or the window, or a race on the game
folder make mrgl write outside `Interface\AddOns` or the asked `WTF` targets, leak a key, or follow a redirect
with it; the findings as a list, each with the file, the case and a proposed fix, severity-ranked.
Why: the M5 Trust roadmap line; the install engine, the sources and the keys grew over M1 to M4 and were never
read as one.
Done when: the list is in the task file, every finding is a backlog item with its own id or a line that says
why it needs none, and a fix of a high-severity finding is its own task, not this one.

Decided 2026-10-24: filed and started in one step from the roadmap's M5 line; the review reads and reports, it
changes no code (a one-line fix with its test may ride along only when the finding is otherwise trivially
lost; the Log says so).

## Docs that move

- `docs/development.md`: a "Hardening review" paragraph under the trust or security material naming the
  date, the areas read and where the findings went, if the docs keep such a record; else none, said here

## Not in this change

- The fixes (each a task of its own, filed by the main session from the list)
- The test files, the CI workflows and the scripts (the review reads the product code)
- A dependency audit (cargo-deny and the weekly advisories job do it)

## Steps

1. [x] Read, in this order, with the question per area written down first: the zip extraction and the
       staging move (`zip_entry.rs`, `staging.rs`, `install.rs`, `link.rs`), the TOC and `release.json`
       parsing (`toc.rs`, `sources/`), the sources' HTTP and redirect handling and the key routing
       (`sources/*.rs`, `keys.rs`), the `WTF` writers (`wtf.rs`, `backup.rs`, `savedvars.rs`, `sets.rs`),
       the path inputs of the CLI and the window (`mrgl-cli/src/main.rs`, `mrgl-app/src/actions.rs`,
       `commands.rs`), the home and state files (`home.rs`, `state.rs`, `last_check.rs`)
2. [x] For each finding: file and line, the crafted input or race, what happens today, the proposed fix,
       severity (high: a write outside the allowed targets or a key leak; medium: a wrong but contained
       write or a crash on crafted input; low: a sharper check or message)
3. [x] The list in this file under Verification, severity-ranked, with the areas read and found clean named
       too; the follow-ups in the pull request under `## Follow-ups`, one line per finding

## Log

- 2026-10-24 — filed and promoted
- 2026-10-24 — the questions, one per area, before the reading:
  - zip and staging: can an entry name, a TOC name or a zip name become a path outside the staging folder, or
    a folder name that is not one folder under `Interface\AddOns`; does a removal or a move follow a junction;
    is a zip bounded in bytes and in entries
  - TOC and `release.json`: does a parsed field ever become a path, a URL or a command
  - HTTP and keys: does a key go to another host, over a redirect, into a URL, a message, a file, a `Debug`
    print, the sidecar, `last-check.json` or the window's state; is an answer bounded in size and in time
  - `WTF`: is any `WTF` write reachable without the user's command, and is the running-client guard ahead of
    every write
  - the CLI and the window: does a path, a flavor id or a name from an argument or an IPC call leave its
    folder; does either spawn a process
  - home and state: can a flavor id, an addon name or a version leave the mrgl home
- 2026-10-24 — H1 reproduced with the debug CLI (`mrgl add <zip> --flavor <folder>`) against a synthetic flavor
  folder with two addons and a temporary `MRGL_HOME`, three crafted zips of two entries each, then removed;
  no code change and no test committed (the fix is not a one-liner: a folder name rule, its messages and
  its tests are a task of their own)
- 2026-10-24 — `std::fs::remove_dir_all` checked once on the pinned toolchain with a junction inside a removed
  folder and a junction removed directly: both junctions went, their target stayed whole (the basis of the
  junction lines among the clean areas)

## Deviations from the plan

- `docs/development.md` keeps no record of reviews (no trust, security or review section to add a paragraph
  to), so it is left alone; the findings live here and in the pull request
- The self-update of D-006 has no code yet (no `WinVerifyTrust` call, no `SHA256SUMS.txt` reader in the
  crates), so it was not read; task 031 is where it lands

## Verification

2026-10-24 — every area of step 1 read in the order given, the product code only (the test modules and the
test files only to see whether a case is covered). No cargo run was needed for the review itself; H1 was
reproduced with a debug build of the CLI, see the Log.

### High

- **H1 — A root TOC whose stem is `.`, `..` or blank becomes the folder name, and the swap moves
  `Interface\AddOns` or `Interface` itself.** `crates/mrgl-core/src/install.rs:1615-1634` (`root_toc_name`,
  called at `:1544`) takes the stem of a `.toc` at the zip's root as the addon's folder name with no check
  (`candidates.first().map(|stem| stem.to_string())`); `zip_entry::entry_path` (`zip_entry.rs:385-398`)
  passes the entry names `..toc`, `...toc` and ` .toc`, since none of their components is `.` or `..`. The
  folder name then reaches `swap_all` (`install.rs:1227-1238`, `addons_dir.join(folder)`), and Windows
  resolves `AddOns\.` and `AddOns\ ` to `AddOns` and `AddOns\..` to `Interface`. Reproduced: a zip of `..toc`
  and one Lua file, `mrgl add`, said `installed . 1.0.0 (1 folder: .)`, moved the whole `AddOns` folder into
  the run folder under `staging\`, put the zip's two files in its place and deleted the run folder with the
  previous `AddOns` in it: every other addon gone, managed or not (the dry run had said `would install .
  1.0.0`); ` .toc` did the same. `...toc` with the mrgl home and the game on one volume failed the rename
  and reverted; with the home on another volume (the usual case of a game on a second drive) `move_dir_with`
  (`install.rs:1786-1794`) copied `Interface` into the run folder, removed `Interface`, wrote the zip's files
  into `Interface` and deleted the copy: `Interface` and everything in it gone, a write outside
  `Interface\AddOns`. The sidecar then records the folder `.` or `..`, and a later `mrgl remove` of that
  entry runs `remove_dir_all(addons_dir.join(folder))` (`install.rs:942-944`) on the same path. Reachable from
  every install path: a zip in hand, every source (`install_from_source`), `import`, `update`, `rollback` and
  the window's update. Not covered by a test (the `root_toc_name` tests use plain stems). Fix: one folder-name
  rule (not empty, not `.` or `..`, no separator, `:` or control character, no trailing dot or space, not a
  reserved device name) applied to every staged folder in `Engine::stage` before anything is unpacked into
  place (the root TOC stem falls back to the zip's stem, else the zip is refused), and again in
  `Engine::commit`, `remove` and `refuse_links` for a folder read from the sidecar; the near-duplicates of
  that rule (`adopt.rs:97-101`, `list_file.rs:192-198`, `sets.rs:1235-1242`) become that one function.

### Medium

- **M1 — No timeout on reading an answer's body.** `crates/mrgl-core/src/sources.rs:885-890`: `UreqHttp::new`
  sets `timeout_connect` (20 s) and `timeout_recv_response` (30 s, the status and headers) only; ureq 3.5's
  `timeout_recv_body`, `timeout_per_read` and `timeout_global` default to none. A server, or a host in the
  path, that sends the headers and then the body a byte at a time holds `add`, `update`, `import` and the
  window's check open for as long as it likes, with no way out but killing the process (the window's cancel
  stops between addons, not inside a request). Fix: a body timeout, by read (`timeout_per_read`, say 60 s)
  and in total per request (`timeout_global`, minutes for an asset, less for an API answer).
- **M2 — Two runs on one flavor at once lose a sidecar entry.** `crates/mrgl-core/src/install.rs:845-897`
  (and `install`, `remove`, `rollback`, `mark`): each run reads `state.json` at its start and writes the whole
  file after its commit (`install.rs:778`, `state.rs:386-405`, a temporary file and a rename), with no lock
  across processes; the staging folder's lock (`staging.rs:104-112`) covers the run folder only, and the
  window's `UpdateRun` and `ToolboxRun` guards are in-process. The window updating while the command line
  installs (the case the staging lock was made for) ends with the second writer's sidecar: the first run's
  addon is installed but unmanaged, its marks and its rollback zip forgotten. Read, not reproduced. Fix: a
  flavor lock file under `<home>\<flavor-id>\`, held from the sidecar read to its write, the second run waiting
  a short while and then refused with a line that names the other run.

### Low

- **L1 — A download follows a redirect to plain HTTP.** `crates/mrgl-core/src/sources.rs:885-890`: the agent
  keeps ureq's `https_only(false)`, and ureq follows an `https` to `http` redirect (no key goes along: ureq drops
  `Authorization` on every redirect, and the CurseForge key never rides a client redirect). Every host mrgl asks
  is HTTPS. Fix: `.https_only(true)` on the agent.
- **L2 — Every answer shares the zip's 512 MiB body bound.** `crates/mrgl-core/src/sources.rs:930-933`: the
  release list, a `release.json`, the Wago and CurseForge answers and the WoWInterface file list are read into
  memory up to 512 MiB like an asset, then parsed. Fix: a bound per request, kilobytes to a few MiB for an API
  answer or a `release.json`, the 512 MiB for an asset only.
- **L3 — A GitHub spec of dots names a cache folder above the GitHub cache.**
  `crates/mrgl-core/src/sources.rs:250-255` (`github_path`) and `crates/mrgl-core/src/toc.rs:327-333`
  (`github_repo_of_website`, which adopt reads from a TOC's `## X-Website`) accept `.` and `..` as the owner and
  the repository, and `repo_cache` (`sources/github.rs:457-462`) joins them: `github:../..` is the mrgl home
  itself. Contained in the home, and the API answers 404 for such a path, so nothing is written today. Fix:
  refuse an owner or a repository of dots only, as GitHub does.
- **L4 — Control characters reach the terminal.** `crates/mrgl-core/src/sources.rs:1262-1277` (`notes_line`)
  keeps every character but whitespace and Markdown markers, and the release tag, the TOC title and the folder
  names of a zip are printed as they come through `voice::say`: an escape sequence in a release's notes or a
  TOC title rewrites the terminal's line, a fake prompt before `update`'s confirmation among them. The window
  is not affected (no `innerHTML`, text only). Fix: every human line the CLI prints replaces control
  characters (U+0000 to U+001F and U+007F to U+009F) with U+FFFD; the JSON output, which serde escapes,
  stays as it is.
- **L5 — A zip is bounded in bytes, not in entries.** `crates/mrgl-core/src/zip_entry.rs:153-182`
  (`checked_entries`): a zip of a few hundred thousand empty files passes the size bound and creates every
  file in staging and then under `AddOns`, which the client and the scan read on every start. Fix: a bound on
  the entries of one zip (an addon has hundreds to a few thousand files), refused before anything is written
  like the size bound.
- **L6 — An entry component ending in a dot or a space passes and is renamed by Windows.**
  `crates/mrgl-core/src/zip_entry.rs:385-398`: Windows drops a trailing dot or space when the file or folder is
  created, so `Foo.` and `Foo` unpack into one folder, and the file list the sidecar records names paths that
  are not on disk. Contained in the staging folder. Fix: refuse such a component, as the name rule refuses the
  other names Windows will not create.
- **L7 — Two addon names can share one cache folder.** `crates/mrgl-core/src/state.rs:67-75` and `:90-107`
  (`file_safe`): `Foo Bar` and `Foo_Bar` map to one `cache\Foo_Bar\`, as every name of dots only maps to
  `cache\_\`; `cache_zip`'s prune (`install.rs:1187-1202`) then deletes the other addon's zips and `remove`
  (`install.rs:958-959`) its whole cache folder, so its rollback fails with the zip missing. `NUL` also passes
  and is the null device as a last path component on Windows. Contained in the mrgl home. Fix: a mapping
  without collisions (a short hash of the name appended whenever `file_safe` changed it), reserved device names
  escaped.
- **L8 — Key values sit in types that derive `Debug`.** `crates/mrgl-core/src/sources.rs:785-791`
  (`Request`, whose headers carry `Authorization` and `x-api-key`) and `crates/mrgl-core/src/doctor.rs:637-646`
  (`Inputs::token`): no product code prints either today, but a `{:?}` or a panic message added later would
  print the key. The doctor's rate-limit request (`doctor.rs:328-334`) adds the token without
  `keys::may_send` (its URL is the API's own constant, so nothing goes elsewhere today). Fix: header values
  marked secret print as `<hidden>` in `Request`'s `Debug`, the doctor holds the token as a `keys::Key`, and its
  request goes through `may_send` like every other.
- **L9 — The running-client guard is a moment's check ahead of a long write.**
  `crates/mrgl-core/src/backup.rs:533` (`client::ensure_client_not_running(flavor)?;`), then the move aside
  and the unpack of the whole `WTF`; the same for a set apply (`sets.rs:979`), a set undo (`sets.rs:1390`) and
  the SavedVariables cleanup (`savedvars.rs:465`). A client started during a long restore writes `WTF` again
  at logout over the restored files. Fix: the restore asks again once the unpack is done and says so in a
  warning when a client started meanwhile (the writes cannot be held back, the user can still restore again).
- **L10 — An environment variable puts the keys in plain files.** `crates/mrgl-core/src/keys.rs:211-225` and
  `:395-425`: `MRGL_KEY_STORE=folder:<path>`, the end-to-end tests' store, is in every build, and with it `mrgl
  key set` writes the key as a file, against the rule that a key never goes to a file. It is named in the lines
  (`the folder key store`) and documented as the tests' store. Fix: build it only for the tests (a cargo
  feature the test runs turn on), or have `key list` and the doctor warn while it is in use.

### Read and found clean

- **Zip entry names.** `zip_entry.rs:379-402` refuses an absolute name, a drive letter (`:`), a `..` component,
  a control character, `<>"|?*` and a reserved device name, `\` counted as a separator; a symbolic link entry
  refuses the zip (`:169-171`); every entry is checked before the first is written (`install.rs:1461`,
  `backup.rs:608`); the size bound holds declared (`zip_entry.rs:183-195`) and written (`:271-291`). The
  fixtures under `zips/` cover each.
- **Names from source answers that become paths.** The asset name only through `file_safe` (`staging.rs:63`,
  `install.rs:879`), a release tag only through `file_safe` (`sources/github.rs:802-804`, `state.rs:72-75`);
  the Wago id is letters and digits (`sources.rs:263-266`), the WoWInterface and CurseForge ids numbers
  (`:279-285`); a `release.json`, a TOC field or an API answer never becomes a path, a URL that carries a key,
  or a command.
- **Junctions.** The commit refuses a staged or a dropped folder that is a linked addon or any junction
  (`link.rs:260-295`, called at `install.rs:714`), `remove` refuses a linked addon (`link.rs:299-309`, called
  at `install.rs:939`), `unlink` removes a junction only when it points into its development folder
  (`link.rs:514-529`); every removal (`RunDir`'s drop `staging.rs:98`, the sweep `:133`, `remove`
  `install.rs:944`, the restore `backup.rs:566`) is `remove_dir_all`, which removes a junction without following
  it (checked, see the Log); the sweep skips a junction (`staging.rs:129`, not `is_dir`).
- **The staging folder.** A run folder per run, locked while held (`staging.rs:56-112`); the sweep keeps a held
  lock, a young folder and a folder with `previous\` (`:189-194`).
- **`WTF` writes only on the user's command.** The callers of the four writers are the CLI's `backup --restore`
  (`main.rs:2556`), `savedvars --clean` (`:3317`), `set apply` and `set undo` (`:3591`, `:3596`) and the
  window's set and cleanup commands (`toolbox.rs:573`, `:629`, `:729`); each writer runs the guard first
  (`backup.rs:533`, `sets.rs:979`, `:1390`, `savedvars.rs:465`); the `WTF` walk never follows a link
  (`wtf.rs:89-91`), the backup leaves links out (`backup.rs:217-223`), the set undo record refuses a name that
  is a path (`sets.rs:1235-1242`).
- **The keys' hosts and redirects.** `keys::may_send` takes `https://` and the exact host only, no user part, no
  port (`keys.rs:95-106`); the GitHub token goes on API requests only (`sources/github.rs:549-553`), never with
  an asset (`:722`); the Wago key only on `may_send` (`sources/wago.rs:399-404`), and ureq 3.5 drops
  `Authorization` on every redirect (its default `RedirectAuthHeaders::Never`); CurseForge turns the client's
  redirects off and follows each hop itself with `may_send` per hop (`sources/curseforge.rs:418-441`). No key
  is in a URL; a status error carries the status, the URL and a hint, never a body
  (`sources/curseforge.rs:650-673`, `sources/wago.rs:804-832`); `Key`'s `Debug` hides the value
  (`keys.rs:159-168`); `key set` refuses whitespace, control characters and more than 2560 bytes
  (`keys.rs:282-310`); no CurseForge answer is cached (`sources/curseforge.rs:395-397`, no cache folder); the
  sidecar, `last-check.json` and the window's answers carry no key.
- **No process is spawned.** The product code has no `Command::new` (the one in `discover.rs:1356` is in its
  tests); the window registers the opener plugin but grants `core:default` only
  (`crates/mrgl-app/capabilities/default.json`) and never calls it; the UI writes text, never HTML, and the CSP
  allows the window's own scripts only (`crates/mrgl-app/tauri.conf.json`).
- **Flavor ids and window inputs.** A `.flavor.info` id is ASCII letters, digits and `_` (`flavor.rs:298-302`),
  so `FlavorHome::at` stays in the home; the window's flavor argument is a known id's folder or one of the
  listed flavors (`crates/mrgl-app/src/list.rs:672-682`); a folder argument is looked up among the sidecar's
  folders (`list.rs:621`) and never joined to a path.
- **The list file and adopt.** An entry whose folder is a path is refused (`list_file.rs:192-198`) and the
  list's name reaches a path only through `file_safe`; adopt groups scanned folders only, a source's `UIDir`
  never becomes a folder by itself (`adopt.rs:806-833`).
- **The running-client check.** The limited query right reads an elevated client's path too (`client.rs:377`);
  under `AddOns` a running client is a warning by design (task 038).
- **Home and state files.** The sidecar, the sets file and `last-check.json` are written through a temporary file
  and a rename (`state.rs:386-405`, `sets.rs:707-710`, `last_check.rs:206-207`); `MRGL_HOME` and
  `LOCALAPPDATA` are the only roots (`home.rs`).
- **D-006.** No self-update code exists yet; nothing to read.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
