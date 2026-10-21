# Origin

mrgl started as a greenfield project with one concrete problem behind it.

## Starting point

A typical Windows setup with four World of Warcraft flavors installed side by side: `_retail_`, `_classic_`,
`_classic_era_` and, since the beta, `_forever_` (World of Warcraft: Forever, codename Camelot, client
`1.60.1`, TOC interface `16001`, launch 2026-11-04). A few hundred addons across the first three folders, all
managed by the CurseForge app, which marks them with a `.curseclient` file. Forever's folder is empty and will
not stay so.

## Problem

Forever changes the rules for addons: it runs the modern Mainline API with the combat-related values hidden,
so most addons need a Forever-specific build, published next to the Classic and Retail ones. The addon
managers that exist today pick "latest", and latest is often the Classic build, which does not load. Beyond
that, the CurseForge app is an Electron desktop app that owns the AddOns folder; WowUp is Electron too, with
Forever support unconfirmed; Ajour, the one Rust manager, was archived in 2024; the Forever-specific
managers are early, single-flavor scripts. The details and the sources are in the
[research page](research/2026-10-21-addon-manager-alternatives.md).

A manager for this situation has to:

1. know which flavor a game folder is, from the folder itself (`.flavor.info`), and install the build published
   for that flavor, never "latest",
2. work without CurseForge, from the sources that publish flavor metadata (GitHub `release.json` as the
   BigWigs packager writes it, the Wago and WoWInterface APIs, a zip in hand), and use CurseForge only within its
   terms, with the user's own key,
3. leave the client alone: write under `Interface\AddOns`, keep its own state elsewhere, and live next to the
   CurseForge app for the folders that app still manages.

## Need

First, look for an existing manager that does this (see `research/`); none did. Second, build one of our own
for Windows 11 with as few dependencies as possible, ideally none for the person running it: a Rust core, a
command line and a window on top of it.

## Usage profile

Maintenance in bursts: a round after a game patch or a big addon release, a daily "update all" in the first
weeks of a new flavor, a backup before an experiment with the UI. One person at first, then anyone with more
than one flavor; later a toolbox around the manager (WTF backups, addon sets, a link for the addons one
develops oneself).
