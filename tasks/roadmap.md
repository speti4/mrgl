# Roadmap

A flavor-correct World of Warcraft addon manager and toolbox for Windows 11, Forever-first. Order: first a
headless core and a command line that install from GitHub releases, then the window on top of it, then the
toolbox around it, then the other sources, then the trust work a public release needs, then a macOS preview. State lives in the task files and the generated index,
never here. The `## ` headings are the milestone names a task joins with `milestone:`.

## M1 Core + CLI

Flavor detection from the game folder, the AddOns scan with TOC parsing, the GitHub `release.json` source,
the zip install engine with the state sidecar, the `mrgl` command line with its murloc voice, and the
task-to-issue mirror of the public repo. Done when the Forever addon list (Questie, Bagnon or Baganator, Auctionator, Leatrix Plus and
Maps, Details and the rest) installs from GitHub into `_forever_` and updates flavor-correctly on the live
Forever client.

## M2 Window MVP

The Tauri 2 window: a list per flavor with status, update all, search, a source badge, a note on the Forever
restrictions of an addon, light and dark; the murloc icon and the first portable zip release. Done when the
daily addon maintenance goes without the CurseForge app.

## M3 Toolbox

WTF backup and restore, addon sets with enable and disable, list export and import, the junction link for
the addons one develops oneself, and a warning when a TOC's interface number does not match the folder's
build. Done when the character settings of the first weeks
after launch can be saved and restored, and an addon set switches with one command.

## M4 Sources

The Wago API, the WoWInterface API, the CurseForge Core API with the user's own key and fingerprint matching,
and the handling of `allowModDistribution=false` (open the page, import a zip). Done when most addons of the
retail and classic folders are recognized and updatable.

## M5 Trust

A usage guide as HTML inside the zip, Authenticode-signed exes through SignPath and a hardening review. Done
when a release carries two signed exes, a checksum, an attestation and generated notes.

## M6 macOS preview

The same tool on a Mac: the Tauri build, the `/Applications/World of Warcraft/` paths, an ad-hoc signed app
from CI. Done when a Mac user installs an addon from the release zip.
