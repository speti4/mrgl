---
status: decided
date: 2026-10-21
cost: one-way
sources: [wip/001-project-scaffolding.md, docs/research/2026-10-21-addon-manager-alternatives.md, D-001-build-our-own-manager-instead-of-adopting-one.md]
---

# D-002 — Rust + Tauri 2, one portable zip with two exes; CurseForge-aware, not CurseForge-dependent

## Context

D-001 builds our own manager. The stack has to give one portable artifact with nothing to install, a command
line and a window, and sources that survive a hosting site's policy change (the Ajour and WowUp history in
the research). A manager that scrapes CurseForge is one terms-of-service change from dead; one that ignores
it cannot update most of the retail and classic folders.

## Decision

- **Rust** for the core and the command line (`crates/mrgl-core`, `crates/mrgl-cli`, the exe `mrgl`), **Tauri 2**
  on WebView2 for the window (`crates/mrgl-app`), the UI as plain HTML, CSS and JavaScript with no bundler and
  no Tauri CLI. One portable zip with the two exes; WebView2 is part of Windows 11, everything else is
  compiled in. A new crate needs a one-line reason in `Cargo.toml` and a license on the allowlist.
- **Sources, in this order:** GitHub releases with the BigWigs packager's `release.json` (flavor and
  interface named per asset), the Wago external API with the user's key, the WoWInterface public API, a zip in
  hand. **CurseForge** through its Core API only, with the user's own key sent as `x-api-key`; a project with
  `allowModDistribution=false` gets "open the page / import a zip"; the website is never scraped.
- **The game stays the user's.** mrgl writes under `Interface\AddOns`, and under `WTF` only for a backup
  restore, an addon set or a SavedVariables cleanup the user asked for (M3), never while the client runs; it
  never touches the client or its launcher, and lives next to the CurseForge app for the folders that app
  still manages.
- The cost is one-way: the stack and the source order shape every crate. Decided 2026-10-21.
- Amended 2026-10-21: the `WTF` sentence names the two toolbox writes (a backup restore, an addon set), so
  the M3 tasks and this decision agree.
- Amended 2026-10-21: a third toolbox write, the SavedVariables cleanup of orphan files into a backup zip
  (task 025), joins the two; every `WTF` write refuses while the client runs (task 023).

## Not chosen

- Electron or a web stack: the weight and the runtime the CurseForge app and WowUp carry are the problem
  being solved.
- A pure CLI with no window: the daily "update all" and the per-flavor overview want a list on screen (M2).
- A TUI instead of Tauri: no system web view to lean on for a list with badges, search and themes, and the
  pattern (Tauri 2, plain UI, Node tests for the pure modules) is proven.
- CurseForge-first: a dependency on an API that was closed to third parties once already.
- Scraping CurseForge: against the terms, and a site change away from breaking.
