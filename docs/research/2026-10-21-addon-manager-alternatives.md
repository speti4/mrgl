# Addon manager alternatives and source APIs

Verified: 2026-10-21. The addon hosting sites change their terms; re-check the sources before relying on a
claim here.

## The alternatives

| Manager | Stack | License | Forever-aware | State | Gap for mrgl's goals |
|---|---|---|---|---|---|
| CurseForge app | Electron (Overwolf heritage) | proprietary | picks by "latest", flavor-blind for Forever | active | manages the AddOns folder itself; the app offers no API, other tools use the Core API with an approved key |
| WowUp | Electron | GPL-3.0 | unconfirmed | active | a runtime to install; CurseForge only through its separate CurseForge edition (since 2022) |
| Ajour | Rust (iced) | GPL-3.0 | no | archived 2024-09-17 | the one Rust manager; no longer maintained |
| instawow | Python CLI | GPL-3.0 | no | active | a Python runtime to install, CLI only, no Forever flavor |
| Strongbox | Clojure | AGPL-3.0 | no | active, Linux-first | a JVM to install, Linux-first |
| forever-addon-manager | Python / Tkinter | MIT | yes | early | a Python runtime to install, one flavor, no state, GitHub as the only source |

None combine a flavor-correct pick for Forever, a CurseForge-free source set, a portable zip with nothing to
install and a maintained code base. Decision D-001 builds our own.

## Source APIs and their terms

### GitHub releases and `release.json`

The BigWigs packager publishes, next to each release's zips, a `release.json` that names the game flavors each
zip is for. The Questie release of 2026-10-12 carries, for example, a `releases[].metadata[]` list with
`{ "flavor": "forever", "interface": 16001 }` next to the `mainline`, `classic` and other entries. The shape:

```json
{
  "releases": [
    {
      "name": "Questie v11.0.0",
      "version": "v11.0.0",
      "filename": "Questie-v11.0.0.zip",
      "nolib": false,
      "metadata": [
        { "flavor": "forever", "interface": 16001 },
        { "flavor": "classic", "interface": 11509 },
        { "flavor": "mainline", "interface": 120100 }
      ]
    }
  ]
}
```

Reading it needs no key; the GitHub REST API allows 60 unauthenticated requests per hour per IP and 5000 with
a token, and honors `ETag` / `If-None-Match`, so a cached release list costs nothing against the limit. When a
release has no `release.json`, the asset name is the fallback (`-forever`, `-classic`, `-mainline` suffixes),
flagged as a guess.

### Wago

Wago Addons exposes an external API for addon managers with a Bearer key the user creates on wago.io; it
returns releases per game version with the flavor named. The key is the user's, stored in mrgl's own
configuration, never in the repo.

### WoWInterface

WoWInterface has a public API (`api.mmoui.com`) that lists an addon's file versions and download links
without a key; it has no flavor metadata of its own, so the TOC's `## Interface` lines decide.

### CurseForge

The CurseForge Core API needs an API key issued per application (an application form on the CurseForge
developer console; the approval is manual). Every request carries `x-api-key`. Project authors may set
`allowModDistribution=false`, in which case the API returns no download link and a manager must send the user
to the project page or take a zip the user downloaded by hand. Scraping the website is against the terms
(Overwolf's own statement is in the sources). Decision D-002 therefore keeps CurseForge optional: the user's
own key, the API within its terms, "open the page / import a zip" when distribution is off, never scraping.
The key application is task 007; the application route and the terms the source follows are on
[the CurseForge API page](2026-10-22-curseforge-api-terms.md).

### The Ajour and WowUp history

Overwolf bought CurseForge in 2020 and announced in 2021 that the API would be restricted to approved
applications. In February 2022 CurseForge denied WowUp's application for API access after WowUp's partnership
with Wago; in November 2022 WowUp released a separate CurseForge edition with Overwolf's permission, and both
editions are maintained. Ajour's repository was archived on 2024-09-17. The lesson for D-002: a manager that
depends on one hosting site is one policy change from dead, so the sources that publish flavor metadata
without a gate (GitHub, Wago, WoWInterface) come first.

## What Forever changes for a manager

- The flavor is read from the game folder, not guessed from the install path: `.flavor.info` holds `wow`,
  `wow_classic`, `wow_classic_era` or `wowf`; `.build.info` holds the version and product. Decision D-003 uses
  that value as the state sidecar's folder name.
- TOC interface numbers at the time of writing: Forever `16001`, Classic Era `11509`, Midnight `120100`
  (they move with every patch). Multi-client addons use
  `MyAddon_Camelot.toc` (also `_Forever`), a comma-separated `## Interface:` line, or a shared `_Mainline.toc`.
- "Latest" is the wrong default: community reports from the beta say managers set to "latest" picked a Classic
  build for Forever. The manager has to match the flavor from `release.json` or the TOC suffix, and warn when
  the only match is a guess.

## Sources

- Questie releases: https://github.com/Questie/Questie/releases/latest
- BigWigs packager (the `release.json` writer): https://github.com/BigWigsMods/packager
- Wago addon API: https://docs.wago.io/
- CurseForge REST API: https://docs.curseforge.com/rest-api/
- Overwolf on scraping CurseForge: https://medium.com/overwolf/our-stance-on-scraping-curseforge-19f4559a322a
- The end of Ajour: https://www.wowhead.com/news/the-end-of-ajour-overwolf-announces-new-api-roadmap-to-restrict-third-party-324892
- Ajour: https://github.com/ajour/ajour
- WowUp: https://github.com/WowUp/WowUp and https://warcraft.wiki.gg/wiki/WowUp
- forever-addon-manager: https://github.com/Artithefax/forever-addon-manager
- instawow: https://github.com/layday/instawow
- Strongbox: https://flathub.org/apps/la.ogri.strongbox
- Forever API changes: https://warcraft.wiki.gg/wiki/Patch_1.60.1/API_changes
- Forever addon status per build (community): https://metammo.com/wow-forever/addons
