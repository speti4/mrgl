// Made-up data in the shape the core will answer with (task 017). The view (`main.js`) reads `window.Data`
// and never knows whether the rows are made up: task 009 replaces this file with one that invokes the Tauri
// commands behind the same three calls, and the layout stays as it is.
//
//   Data.version()                  -> Promise<string>            the version for the status bar
//   Data.flavors()                  -> Promise<{ root, flavors, message }>
//   Data.updateAll(flavorId, report) -> Promise<void>              `report(message)` per step, in order
//
// A message is `{ kind, murloc, plain }` as `mrgl_core::voice` says it: the kind picks the murloc line, the
// plain line carries every fact, and a serious voice sends the murloc line empty (D-005). The murloc table
// below is a copy of `voice.rs` for the sketch only: the real answers come from Rust, and nothing in the view
// composes a murloc line.
//
// A flavor is `Flavor` of `flavor.rs` plus its friendly name and its scanned addons; an addon is `Addon` of
// `scan.rs` reduced to what a row shows, plus the facts the sidecar (004), the sources (003), the update
// check (006), the Forever notes (011) and pin / ignore (021) add:
//
//   { folder, name, version, linked,            from the scan: the folder, the TOC's plain title and version
//     source: { kind, id } | null,              github | wago | wowi | curseforge | zip | linked
//     managed, guessed,                         a sidecar entry; a source known only from a TOC hint
//     update: { version } | null,               the latest release for the flavor, when newer
//     pinned: string | null, ignored,           021
//     forever: { build, note } | null }         011: build yes | no | unknown, a curated note or null
//
// The view derives the row's one state from these flags (`state.js`); the data never names a state.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Data = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  const MURLOC = {
    banner: "Mrglglgl.",
    progress: "Mrrrgl… mrgl mrgl.",
    success: "Mrglglglgl!",
    nothing: "Mrgl.",
    warning: "Mrgl? Mrgl…",
    error: "Rwlrwlrwlrwl?!",
    fatal: "Aaaaaughibbrgubugbugrguburgle!",
  };

  function say(kind, plain) {
    return { kind, murloc: MURLOC[kind] || "", plain };
  }

  // `1 addon`, `2 addons`: the count-noun summary of docs/nerglish.md.
  function addons(count) {
    return `${count} addon${count === 1 ? "" : "s"}`;
  }

  function addon(folder, name, version, extra) {
    return Object.assign(
      {
        folder,
        name,
        version,
        linked: false,
        source: null,
        managed: false,
        guessed: false,
        update: null,
        pinned: null,
        ignored: false,
        forever: null,
      },
      extra
    );
  }

  const github = (id) => ({ kind: "github", id });
  const wago = (id) => ({ kind: "wago", id });
  const wowi = (id) => ({ kind: "wowi", id });
  const curseforge = (id) => ({ kind: "curseforge", id });
  const zip = (id) => ({ kind: "zip", id });
  const linked = (id) => ({ kind: "linked", id });

  const ROOT = "D:\\Games\\World of Warcraft";

  const flavors = [
    {
      id: "wow",
      name: "retail",
      folder: "_retail_",
      version: "12.1.5.70448",
      interface: 120105,
      curse_client: true,
      warnings: [],
      addons: [
        addon("BigWigs", "BigWigs", "v400.3", { source: github("BigWigsMods/BigWigs"), managed: true }),
        addon("Details", "Details! Damage Meter", "#Details.20261010.14521.165", { source: curseforge("61284"), guessed: true }),
        addon("DBM-Core", "Deadly Boss Mods", "12.1.12", { source: curseforge("3358"), guessed: true }),
        addon("Plater", "Plater Nameplates", "Plater-v665-Retail", { source: wago("kRNLep6o"), managed: true, update: { version: "Plater-v666-Retail" } }),
        addon("WeakAuras", "WeakAuras", "5.19.11", { source: github("WeakAuras/WeakAuras2"), managed: true }),
        addon("WeakAuras_Options", "WeakAuras Options", "5.19.11", { source: github("WeakAuras/WeakAuras2"), managed: true }),
        addon("MyUnitFrames", "My Unit Frames", "0.4.0", { source: zip("MyUnitFrames-0.4.0.zip"), managed: true }),
        addon("OldMinimap", "Old Minimap", "1.0", {}),
      ],
    },
    {
      id: "wow_classic",
      name: "classic",
      folder: "_classic_",
      version: "5.5.4.70531",
      interface: 50504,
      curse_client: true,
      warnings: [],
      addons: [
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true, update: { version: "11.3.0" } }),
        addon("AtlasLootClassic", "AtlasLoot Classic", "v4.2.0", { source: wowi("24976"), managed: true }),
        addon("Leatrix_Plus", "Leatrix Plus", "5.5.26", { source: curseforge("289009"), guessed: true }),
        addon("NovaWorldBuffs", "Nova World Buffs", "2.74", { source: github("mt-paul/NovaWorldBuffs"), managed: true, pinned: "2.74", update: { version: "2.75" } }),
        addon("TomTom", "TomTom", "v5.5.4-mists", { source: wago("4N2mZ3KL"), managed: true }),
        addon("ClassicCastbars", "ClassicCastbars", "v3.1.2", { source: github("wardz/ClassicCastbars"), managed: true, ignored: true }),
      ],
    },
    {
      id: "wow_classic_era",
      name: "era",
      folder: "_classic_era_",
      version: "1.15.9.70110",
      interface: 11509,
      curse_client: true,
      warnings: ["build unknown: no row for wow_classic_era in .build.info"],
      addons: [
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true }),
        addon("pfQuest", "pfQuest", "6.4.1", { source: github("shagu/pfQuest"), managed: true, update: { version: "6.5.0" } }),
        addon("Bartender4", "Bartender4", "4.15.7", { source: curseforge("4525"), guessed: true }),
        addon("RaidLogger", "Raid Logger", "0.9", {}),
      ],
    },
    {
      id: "wowf",
      name: "forever",
      folder: "_forever_",
      version: "1.60.1.70245",
      interface: 16001,
      curse_client: false,
      warnings: [],
      addons: [
        addon("BigWigs", "BigWigs", "v400.3", { source: github("BigWigsMods/BigWigs"), managed: true, update: { version: "v401.0" }, forever: { build: "yes", note: null } }),
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true, forever: { build: "yes", note: null } }),
        addon("Details", "Details! Damage Meter", "#Details.20261010.14521.165", { source: curseforge("61284"), guessed: true, forever: { build: "unknown", note: "hidden combat values: the meter shows nothing" } }),
        addon("Plater", "Plater Nameplates", "Plater-v665-Forever", { source: wago("kRNLep6o"), managed: true, update: { version: "Plater-v666-Forever" }, forever: { build: "yes", note: "no combat log: threat colors are off" } }),
        addon("MyAuras", "My Auras", "0.2.0-dev", { source: linked("D:\\code\\wow-addons\\MyAuras"), linked: true, forever: { build: "unknown", note: null } }),
        addon("MyUnitFrames", "My Unit Frames", "0.4.0", { source: linked("D:\\code\\wow-addons\\MyUnitFrames"), linked: true, forever: { build: "unknown", note: null } }),
        addon("Leatrix_Plus", "Leatrix Plus", "5.5.26", { source: curseforge("289009"), guessed: true, forever: { build: "no", note: null } }),
        addon("TomTom", "TomTom", "v5.5.4-mists", { source: wago("4N2mZ3KL"), managed: true, update: { version: "v5.5.5-forever" }, forever: { build: "yes", note: null } }),
        addon("RaidLogger", "Raid Logger", "0.9", { forever: { build: "unknown", note: null } }),
        addon("ClassicCastbars", "ClassicCastbars", "v3.1.2", { source: zip("ClassicCastbars-v3.1.2.zip"), managed: true, forever: { build: "no", note: "the castbar needs the combat log it no longer gets" } }),
      ],
    },
  ];

  function version() {
    return Promise.resolve("0.1.0");
  }

  function flavors_() {
    const count = flavors.reduce((sum, flavor) => sum + flavor.addons.length, 0);
    return Promise.resolve({
      root: ROOT,
      flavors,
      message: say("banner", `${flavors.length} flavors under ${ROOT}, ${addons(count)}`),
    });
  }

  // How long the made-up update takes; the tests set it to 0.
  const settings = { delay: 1200 };

  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  // The made-up "update all": one flavor per outcome, so every kind of the voice shows up somewhere. The
  // rows with an update flip to managed on success, as the real engine's answer would.
  async function updateAll(flavorId, report) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) {
      report(say("error", `no flavor ${flavorId} under ${ROOT}`));
      return;
    }
    const due = flavor.addons.filter((item) => item.update && !item.pinned && !item.ignored);
    const pinned = flavor.addons.filter((item) => item.update && item.pinned);
    const ignored = flavor.addons.filter((item) => item.ignored);
    const upToDate = flavor.addons.length - due.length - pinned.length - ignored.length;
    const held = pinned.map((item) => `${item.name} pinned at ${item.pinned}, ${item.update.version} available`);
    if (due.length === 0) {
      const summary = [`nothing to update in ${flavor.name}`, `${upToDate} up to date`, ...held];
      if (ignored.length) summary.push(`${ignored.length} ignored`);
      report(say(held.length ? "warning" : "nothing", summary.join(", ")));
      return;
    }
    report(say("progress", `updating ${addons(due.length)} in ${flavor.name}`));
    await wait(settings.delay);
    if (flavor.id === "wow_classic_era") {
      report(say("error", `could not reach github.com for ${due[0].name}: network unreachable`));
      return;
    }
    for (const item of due) {
      item.version = item.update.version;
      item.update = null;
    }
    const parts = [`${addons(due.length)} updated`, `${upToDate} up to date`, ...held];
    if (ignored.length) parts.push(`${ignored.length} ignored`);
    report(say(held.length ? "warning" : "success", parts.join(", ")));
  }

  return { version, flavors: flavors_, updateAll, settings };
});
