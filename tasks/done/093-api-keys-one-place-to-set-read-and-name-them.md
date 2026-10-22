---
size: S
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 093 — API keys: one place to set, read and name them

## Context

A `keys` module in the core with the key names (`MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY`,
`MRGL_CURSEFORGE_KEY`) and the lookup order, the environment first, then the Windows Credential Manager entry
`mrgl:<source>` (a `windows-sys` feature, no new crate); `mrgl key set|clear <source>` reads the value from
stdin, never from an argument; `mrgl key list` and the doctor name each key and where it comes from
(environment, credential store, not set), never its value; a key is sent to its source's hosts only.
Decided 2026-10-22: only the user's own keys, never one shipped in the zip (D-002); the window's key field
comes after 096, not here.
Why: two of the three new sources need a key, and one redaction rule has to cover them all.
Done when: the doctor lists the three names with their origin, a test asserts no key value in any output,
and the credential store path is covered by a fake store in unit tests.

Decided 2026-10-22: runs after 092 on the same branch, since both change how the sources are built.

## Docs that move

- `docs/development.md`: the keys module, the doctor's `api keys:` line and JSON `origin`, the credential store
- `README.md`: the `key` command and where a key comes from
- `Cargo.toml`: the one-line reason for the `windows-sys` credential feature

## Not in this change

- The window's key field (after 096)
- A key store outside Windows (M6); the Wago and CurseForge sources that read the keys (095, 096)

## Steps

1. [x] `keys.rs` in the core: the names, the lookup order, the Windows Credential Manager read, write and
       delete behind a store trait with a fake store for tests; the non-Windows twin reports "not available"
2. [x] `github.rs`: the token comes from the module; `doctor.rs`: one line per key with its origin, JSON `origin`
3. [x] `main.rs`: `mrgl key set|clear <source>` (the value from stdin) and `mrgl key list`
4. [x] Tests: the doctor's three names with their origin, no key value in any text, JSON or error output,
       the fake store round trip

## Log

- 2026-10-22 — promoted; runs after 092 on the same branch
- 2026-10-22 — `mrgl_core::keys`: the names `MRGL_GITHUB_TOKEN`, `MRGL_WAGO_KEY`, `MRGL_CURSEFORGE_KEY`
  (`keys::KEYED` in that order; WoWInterface takes none), `keyed_source` (a source's prefix or name, or the
  variable; `no key for WoWInterface: its API needs none`, `no key source: <name> (github, wago or curse)`),
  `lookup` (the environment first, an empty or blank variable not set, then the store), `list`
  (`KeyState`: source, name, `Origin` `environment` / `credential_store` / `not_set`, written
  `set (environment)`, `set (credential store)`, `not set`), `read_key` (all of a pipe, one line from a
  terminal, trimmed; refuses nothing, more than one word or line, or more than 2560 bytes), `set` and
  `clear`. A `Key` holds the value and hides it from `Debug`; no error carries it.
- 2026-10-22 — the store: the `KeyStore` trait with `CredentialStore` (generic credential
  `mrgl:<source>`, user name `mrgl`, the key as its UTF-8 blob, `CRED_PERSIST_LOCAL_MACHINE`, through
  `CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`; `ERROR_NOT_FOUND` reads as no key and as nothing
  removed), its `cfg(not(windows))` twin (reads nothing, refuses a write: `the key store is not available:
  the Windows Credential Manager exists on Windows only`), `NoStore` (`MRGL_KEY_STORE=off` turns the store
  off for a run) and `MemoryStore` for the tests. `Cargo.toml` adds the `Win32_Security_Credentials`
  feature to `windows-sys` with its reason; no new crate, `Cargo.lock` unchanged.
- 2026-10-22 — `github.rs`: `GitHub::from_env` takes the token through `keys::of_process`, and
  `api_request` adds it only when `keys::may_send` allows the URL (HTTPS, exactly `api.github.com`);
  `keys::hosts` already names Wago's (`addons.wago.io`) and CurseForge's (`api.curseforge.com`,
  `edge.forgecdn.net`) for the sources to come. `TOKEN_ENV` stays as an alias of `keys::GITHUB_TOKEN`.
- 2026-10-22 — the doctor: `API_KEYS` names the three keys, `ApiKey` gains `origin` (JSON
  `api_keys[].origin`), `api_keys_with(env, store)` builds them from `keys::list` (`api_keys_from(env)`
  keeps the environment alone, for its tests), the line reads `api keys: MRGL_GITHUB_TOKEN set
  (environment), MRGL_WAGO_KEY not set, MRGL_CURSEFORGE_KEY set (credential store)`, and the rate-limit
  request takes its token through the module; the change stays inside the api-keys part of `doctor.rs`.
- 2026-10-22 — CLI: `mrgl key set <source>` checks the store, reads the key from the standard input
  (a progress line in a terminal), stores it and warns when the variable is set too; `mrgl key clear
  <source>` removes it (`nothing removed` when there was none); `mrgl key list` lists the three with their
  origin, a warning when the store is not available and `<n> of 3 keys set, none shown`; `--json` answers
  `kind: "keys"` and `kind: "key"`. A word after the source of `key set` is refused without being echoed
  (`a key is never an argument, the shell's history would keep it: …; nothing stored`), where clap's own
  refusal would print it.
- 2026-10-22 — tests: core unit `keys` (the names and sources, the environment before the store, the
  memory store's set, read and clear, a store that is off, reading the input, the hosts a key may go to,
  `Debug` without the value, and outside Windows the store's refusal) and the doctor's
  `the_keys_are_named_never_read_out` with the store; `tests/cli.rs`
  `key_list_names_every_key_and_its_origin_never_a_value`,
  `key_set_and_clear_read_stdin_and_never_print_the_key`,
  `no_key_value_reaches_any_output_of_the_doctor_or_a_source`. The CLI tests' `mrgl()` sets
  `MRGL_KEY_STORE=off` and removes the two new variables, so no test touches the real Credential Manager.
- 2026-10-22 — docs: `docs/development.md` (the layout's `keys`, the doctor's `api keys:` line and its JSON
  `origin`, a new section "The API keys" with the store, the command, the hosts and the tests, the token
  line under Fixtures), `README.md` (the `key` command in the usage block, where a key comes from).

## Deviations from the plan

- Three existing assertions changed with the doctor's line, as the task asks: the doctor's core unit tests
  (`api keys: … set` / `not set`, and the `ApiKey` literal of `the_keys_are_named_never_read_out`, which now
  lists three keys with their origin) and `tests/cli.rs`'s doctor tests (the `api keys:` line and the JSON
  `api_keys`, now three entries with `origin`). The CLI tests' `mrgl()` helper sets `MRGL_KEY_STORE=off`.
- The end-to-end tests cannot reach a store (the real one stays untouched): `mrgl key set|clear` is
  covered end to end for its refusals and for never printing the value, and the set, read and clear
  round trip runs in the core against `MemoryStore`. The real store was exercised once by hand (below).
- `MRGL_KEY_STORE=off` is new: the switch the end-to-end tests need to keep away from the real store; it
  turns the store off for a run, never on.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `cargo fmt --check`, `cargo clippy --all-targets
  --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked`, `python -I scripts/changelog.py
  --check` and `cargo deny check` (advisories, bans, licenses, sources ok) green; `Cargo.lock` unchanged.
  The `cfg(not(windows))` store is left to CI's `non-windows` job (a local check for another target stops
  at `ring`'s C build without a cross compiler).
- 2026-10-22 — by hand, the debug `mrgl` with `MRGL_HOME` under a temporary folder and no key variable:
  `key list` showed the three keys `not set`; a throwaway value piped into `key set curse` gave
  `MRGL_CURSEFORGE_KEY stored in the Windows Credential Manager as mrgl:curse`; `key list` and the doctor's
  line then read `MRGL_CURSEFORGE_KEY set (credential store)` (JSON `origin: "credential_store"`) without
  the value; `key clear curse` removed it, a second one said `nothing removed`, and `cmdkey /list` showed
  no `mrgl` entry afterwards. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
