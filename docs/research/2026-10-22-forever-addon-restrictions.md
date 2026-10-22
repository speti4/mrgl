# Forever addon restrictions: the partly restricted addons

Verified: 2026-10-22, twelve days before the launch on 2026-11-04. Forever's restrictions and the addons move
with every client build and every release; re-check a source before relying on a claim here. A note in
`data/forever-notes.json` that rests on this page turns stale 90 days after its date.

## The question

Which addons have a Forever build but only half work there, because Forever's combat restrictions hide what
they read? That is the curated half of the Forever notes (task 011, `docs/development.md`, "Forever notes").
Whether a repository publishes a Forever build at all is a different question, answered from the release's
`release.json` (the derived signal) and by the Forever starter list (task 028); an addon without a Forever
build is not a case for this page.

The rule for the data file (decided 2026-10-22): a note enters `data/forever-notes.json` only with a public
source this page cites, an addon author's statement, a release note or the project page; an addon the
sources disagree on, or that only a community guide names, stays here as an open question.

## The restriction kinds

- **Secret combat values** (`secret-combat-values`). Forever runs the modern client's addon API, and the
  Patch 1.60.1 API notes on the community wiki say: "The addon disarmament changes introduced in Midnight,
  including secrets, will also be active in Forever." In combat, unit health, aura details, cooldowns and
  other players' casts come back as secret values: an addon can hand them to the game's own widgets, which
  draw them, but cannot compare, sort or compute with them. Blizzard's Midnight article states the aim:
  "Addons should no longer offer a competitive advantage in WoW combat." The same wiki page lists the 12.1.0
  aura container changes as available on Forever, the frames through which the game itself draws auras for an
  addon.
- **Combat log events** (`combat-log-events`). The combat log events are restricted for addons on this
  client: the Musca Auras README says auras that read the combat log "never fire", the SoundAlerter Forever
  author finds the combat log detections dead, and the Thanks for the Buff author leaves a combat-log feature
  out of the Forever build. What an addon sees directly (target, mouseover, nameplate units) still arrives,
  with the combat values secret.
- **Client faults, not restrictions.** Some failures on Forever are bugs of the client, not a policy: the
  TellMeWhen author reports the weapon imbue API as not working at all on Forever and reported it to Blizzard
  (TellMeWhen issue 2504, 2026-10-10); a community page reports that the beta client did not read the saved
  variables back. A fault is expected to be fixed, so it gets no kind and no note.

No new kind was needed: every confirmed case below fits one of the two kinds `data/forever-notes.json`
already has.

## The addons

The first four rows are in `data/forever-notes.json`: each repository publishes a Forever build (a
`release.json` naming `forever`, or a release zip for Forever) and its author states the limit. The other
three are confirmed by their authors too, but their repositories publish no release zip that
`add github:<owner>/<repo>` can install, so a note keyed by their spec would never show; they stay on this
page.

| Addon | Spec | Kind | What does not work on Forever | Source | Statement | Read |
|---|---|---|---|---|---|---|
| Grid2 | `github:michaelnpsp/Grid2` | secret-combat-values | auras are hidden from addons in combat; aura timers and remaining-time bars work only through the new `bar(auras)` and `text(auras)` indicators that the game draws | the author on Grid2 issue 471 | 2026-09-25 | 2026-10-22 |
| TellMeWhen | `github:ascott18/TellMeWhen` | secret-combat-values | cooldowns are secret in combat; icons cannot sort, anchor or decide on a cooldown's state there | the author on TellMeWhen issue 2503; the 12.1.3 release note on secret visibility | 2026-10-10 | 2026-10-22 |
| Musca Auras | `github:dldvlpr-addons-wow/Musca-Auras` | combat-log-events | combat log triggers never fire; conditions and thresholds on hidden combat data wait until combat ends; imported auras need adapting | the project README at release 1.13.0, whose `release.json` names `forever` | 2026-10-21 | 2026-10-22 |
| ForeverAuras | `github:neroxrw/foreverauras` | secret-combat-values | the legacy aura trigger rarely works in combat (the modern aura trigger does); a glow switched in combat on a modern aura icon applies when combat ends | the author on ForeverAuras issues 104 and 99 | 2026-10-13 | 2026-10-22 |
| Thanks for the Buff | `github:Gogo1951/Thanks-for-the-Buff` | combat-log-events | the combat-log alert (Peer Pressure) is left out of the Forever build; whispers go out only out of combat | the author on Thanks for the Buff issue 26 | 2026-10-21 | 2026-10-22 |
| SoundAlerter Forever | `github:th3pajay/soundalerter-forever` | combat-log-events | combat log detections and per-spell sound alerts for off-screen events; other players' spell IDs are secret | the author on SoundAlerter Forever issue 1 | 2026-10-17 | 2026-10-22 |
| ForeverAuras (Tulkas22 fork) | `github:Tulkas22/ForeverAuras` | secret-combat-values | the parts of WeakAuras that read secret values are removed or reworked; errors are expected in combat | the project README | undated | 2026-10-22 |

Two projects carry the name ForeverAuras: `neroxrw/foreverauras` (releases, in the data file) and the
`Tulkas22/ForeverAuras` fork of WeakAuras2 (no release).

## Open questions

The sources disagree on these, or only a community guide names them; none of them is in the data file.

- **Plater Nameplates** (`github:Tercioo/Plater-Nameplates`, Forever build since v656). A community status
  page says the display works and scripts that react to hidden combat state do not. Plater's own release notes
  say "Baseline compatibility with WoW Forever" (v656) and "Re-enable resources for WoW Forever" (v657, v658),
  and its collaborators answer Forever reports with "use the latest alpha"; no statement from the project
  names a lasting limit.
- **Deadly Boss Mods** (`github:DeadlyBossMods/DeadlyBossMods`) and **BigWigs** (`github:BigWigsMods/BigWigs`),
  both with a Forever build. One community page expects the same limits as on retail, another says neither
  ships Forever encounter modules. The changelogs show work in progress, not a limit: DBM 12.1.15 extends its
  secret-based trash and boss alerts to Forever; BigWigs keeps its retail content modules from loading on
  Forever. Whether a Forever encounter is covered is a question of content, not of a restriction.
- **Details! Damage Meter** (`github:Tercioo/Details-Damage-Meter`). The repository's last GitHub release is
  from 2021 (a collaborator points to CurseForge's alpha channel, issue 1133); a user, not the author, reports
  that names are secret in combat (issue 1135). Forever ships the game's own damage meter. Neither a note nor
  an install path through `github:` today.
- **ElvUI.** One community page says its combat-reactive features are blocked; no statement from the project
  was found, and its repository publishes no GitHub release.
- **Spy.** A community page says its detection relies on combat data the client restricts and calls it
  possibly restricted; no author statement was found.
- **Parrot 3** (`github:NeeRgY/Parrot3`, a Forever TOC). A user's analysis (issue 1, 2026-10-19) finds every
  hit dropped on secret values with a proposed workaround; the maintainer had not answered on 2026-10-22.
- **WeakAuras** (`github:WeakAuras/WeakAuras2`). Not a partly restricted case: its 5.22.1 `release.json`
  names no `forever` flavor, so the derived signal says `no`, and the community pages agree there is no
  Forever build. The Forever forks above take its place.

## How the sources were weighed

- An author's statement counts: a comment by the repository's owner or a collaborator on a Forever issue, a
  release note, or the project's README. A user's report, however detailed, does not.
- The community guide pages (Forever status and guide sites, several of them unsigned) disagree
  with each other on versions and on which addon is limited; each was read, none confirms an addon on its
  own. They are useful for the restriction kinds and as a list of names to check.
- Blizzard's own statements describe the restrictions in kind and name no addon. The source base for the
  addons is thin: one author statement each, all from the beta, before the launch.

## Sources

Read on 2026-10-22.

- Patch 1.60.1 API changes (community wiki): https://warcraft.wiki.gg/wiki/Patch_1.60.1/API_changes
- Combat philosophy and addon disarmament in Midnight (Blizzard):
  https://news.blizzard.com/en-us/article/24246290/combat-philosophy-and-addon-disarmament-in-midnight
- Grid2, the author on Forever auras: https://github.com/michaelnpsp/Grid2/issues/471
- Grid2 releases (4.1.9, `release.json` with `forever`): https://github.com/michaelnpsp/Grid2/releases
- TellMeWhen, the author on secret cooldowns: https://github.com/ascott18/TellMeWhen/issues/2503
- TellMeWhen, the weapon imbue fault: https://github.com/ascott18/TellMeWhen/issues/2504
- TellMeWhen releases (12.1.3, 12.1.5): https://github.com/ascott18/TellMeWhen/releases
- Musca Auras README: https://github.com/dldvlpr-addons-wow/Musca-Auras/blob/main/README.md
- ForeverAuras, the author on the legacy trigger: https://github.com/neroxrw/foreverauras/issues/104
- ForeverAuras, the author on glows in combat: https://github.com/neroxrw/foreverauras/issues/99
- ForeverAuras (Tulkas22 fork) README: https://github.com/Tulkas22/ForeverAuras
- Thanks for the Buff, the Forever limit: https://github.com/Gogo1951/Thanks-for-the-Buff/issues/26
- SoundAlerter Forever, the combat log: https://github.com/th3pajay/soundalerter-forever/issues/1
- Plater releases: https://github.com/Tercioo/Plater-Nameplates/releases
- Deadly Boss Mods releases: https://github.com/DeadlyBossMods/DeadlyBossMods/releases
- BigWigs releases: https://github.com/BigWigsMods/BigWigs/releases
- Details!, Forever issues: https://github.com/Tercioo/Details-Damage-Meter/issues/1133 and
  https://github.com/Tercioo/Details-Damage-Meter/issues/1135
- Parrot 3, the secret values report: https://github.com/NeeRgY/Parrot3/issues/1
- WeakAuras releases (5.22.1): https://github.com/WeakAuras/WeakAuras2/releases
- Forever addon status per build (community): https://metammo.com/wow-forever/addons
- Community guides: https://wowforeversite.wiki/guides/addons/, https://foreverchanges.pro/addons,
  https://endgametools.com/en/wow-forever/news/wow-forever-addons and https://wowforevertierlist.com/addons
