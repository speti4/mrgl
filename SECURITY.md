# Security policy

mrgl downloads archives that strangers published and unpacks them into a folder the game loads code from. A
flaw in how it does that matters, and a report is welcome.

## Supported versions

Only the latest release on the [Releases page](https://github.com/speti4/mrgl/releases) gets fixes. A fix
ships as a new release; there are no backports. There is no release yet: until the first one, reports
against `main` are welcome and the fix lands there.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting:
[Report a vulnerability](https://github.com/speti4/mrgl/security/advisories/new) (the *Security* tab →
*Report a vulnerability*). Please do not open a public issue for a security problem.

Never attach a real game folder, account detail or API key to a report. Describe the problem with a made-up
addon: a folder with a TOC and a `release.json` of a few lines are enough to reproduce almost anything (see the
synthetic fixtures under `crates/mrgl-core/tests/fixtures/`).

You can expect a first answer within a week. Once a fix is released, the advisory is published with credit to
you unless you ask otherwise.

## What is in scope

The places where mrgl handles untrusted input or writes to disk:

- **The install engine.** A downloaded zip is unpacked into a staging folder and then swapped into
  `Interface\AddOns`. A zip entry that writes outside the staging folder (`..`, an absolute path, a symlink),
  an install that leaves `AddOns` half-written after a failure, or a file written anywhere else than
  `Interface\AddOns`, the state sidecar under `%LOCALAPPDATA%\mrgl\` and a backup folder the user chose, is a
  vulnerability.
- **Sources.** Release metadata and downloads come over HTTPS from GitHub, Wago, WoWInterface and, with the
  user's own key, CurseForge. A way to make mrgl fetch from another host, to downgrade to plain HTTP, to accept
  an asset whose size or hash does not match the metadata, or to read a key from anywhere but the user's own
  configuration is a vulnerability.
- **The sidecar and the backups.** `state.json` is a cache that a corrupt file may only force to rebuild; a
  crafted state file that makes mrgl delete or overwrite something outside `Interface\AddOns` is in scope.
- **Links.** Links opened from the window go to the system browser, and only for `http` and `https`. Any other
  scheme getting through is a vulnerability.
- **The application shell.** The Tauri capability file and the window's content security policy
  (`crates/mrgl-app/capabilities/`, `crates/mrgl-app/tauri.conf.json`).

## Out of scope

- The exes are not code-signed, so SmartScreen warns on the first start.
- What an addon does once the game loads it: mrgl installs the files a source published and does not inspect
  Lua. Report a malicious addon to its hosting site.
- Anything that needs an attacker who can already write to the user's disk or run code as the user.
- Bugs in WebView2 or Windows itself. Report those to Microsoft. A way mrgl could defend against one is welcome
  as an ordinary issue.
