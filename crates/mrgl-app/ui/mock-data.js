// Made-up data in the shape the core answers with (task 017). The view (`main.js`) reads `window.Data`
// and never knows whether the rows are made up: in the window `data.js` answers through the Tauri commands
// (task 009); opened in a plain browser, `data.js` loads this module instead, for a look at the layout
// without the core. The contract, which `data.js` keeps:
//
//   Data.version()                   -> Promise<string>            the version for the status bar
//   Data.flavors()                   -> Promise<{ root, root_origin, flavors, message }>
//   Data.chooseRoot()                -> Promise<{ flavors, message }>  the folder dialog: the answer of
//                                       `flavors()` for the chosen root, or a message, or both null
//   Data.diagnostics()               -> Promise<{ text, copied, failed }>  the doctor's text (task 024) and
//                                       the message for a copy that worked and for one that did not
//   Data.resolve(flavorId, folder)   -> Promise<{ pick, message }>  the source's `Pick` (task 010 builds on it)
//   Data.updateAll(flavorId, report) -> Promise<void>              `report(message)` per step, in order
//   Data.preferences()               -> Promise<{ theme, serious, serious_source, stored }>  task 034: the
//                                       theme and the serious switch as they apply, which source decided the
//                                       switch (flag | environment | file | default), what config.json stores
//   Data.setPreferences(changes)     -> Promise<the same>          `{ theme?, serious? }` into config.json
//
// A message is `{ kind, murloc, plain }` as `mrgl_core::voice` says it: the kind picks the murloc line, the
// plain line carries every fact, and a serious voice sends the murloc line empty (D-005). The murloc table
// below is a copy of `voice.rs` for the sketch only: the real answers come from Rust, and nothing in the view
// composes a murloc line.
//
// A flavor is `Flavor` of `flavor.rs` plus its friendly name and its scanned addons:
//
//   { id, name, folder,                         the `.flavor.info` id, the friendly name, the folder's name
//     version, interface,                       the installed build and its interface number, or null
//     version_source, version_file,             019: build-info | client-exe, and the file it was read from
//     curse_client, warnings, addons }          the CurseForge app's marker; what reading the flavor met
//
// An addon is `Addon` of `scan.rs` reduced to what a row shows, plus the facts the sidecar (004), the
// sources (003), the cached release list, the Forever notes (011), pin / ignore (021) and the dependency
// check (029) add:
//
//   { folder, name, version, linked,            from the scan: the folder, the TOC's plain title and version
//     source: { kind, id } | null,              github | wago | wowi | curseforge | zip | linked
//     managed, guessed,                         a sidecar entry; a source known only from a TOC hint
//     update: { version, release, prerelease } | null,  the cached current release, when not the installed one
//     pinned: string | null, ignored,           021
//     forever: { build, note, kind, verified, source, stale } | null,  011, on every row of Forever only:
//                                               build yes | no | unknown, a curated note or null
//     missing: [string] }                       029: the required dependencies no folder answers
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
        missing: [],
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
      version_source: "build-info",
      version_file: ".build.info",
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
      version_source: "build-info",
      version_file: ".build.info",
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
      version_source: "client-exe",
      version_file: "WowClassic.exe",
      interface: 11509,
      curse_client: true,
      warnings: ["no row for wow_classic_era in .build.info: the build was read from WowClassic.exe"],
      addons: [
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true }),
        addon("pfQuest", "pfQuest", "6.4.1", { source: github("shagu/pfQuest"), managed: true, update: { version: "6.5.0" } }),
        addon("Bartender4", "Bartender4", "4.15.7", { source: curseforge("4525"), guessed: true }),
        addon("RaidLogger", "Raid Logger", "0.9", { missing: ["LibRaidLog"] }),
      ],
    },
    {
      id: "wowf",
      name: "forever",
      folder: "_forever_",
      version: "1.60.1.70245",
      version_source: "build-info",
      version_file: ".build.info",
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
        addon("ClassicCastbars", "ClassicCastbars", "v3.1.2", { source: zip("ClassicCastbars-v3.1.2.zip"), managed: true, forever: { build: "no", note: "the castbar needs the combat log it no longer gets", verified: "2026-06-01", stale: true } }),
      ],
    },
  ];

  function version() {
    return Promise.resolve("0.1.0");
  }

  // The browser look keeps the preferences for the page's life only; the window keeps them in config.json.
  const stored = { theme: null, serious: null };

  function preferencesAnswer() {
    return {
      theme: stored.theme || "auto",
      serious: Boolean(stored.serious),
      serious_source: stored.serious === null ? "default" : "file",
      stored: Object.assign({}, stored),
    };
  }

  function preferences() {
    return Promise.resolve(preferencesAnswer());
  }

  function setPreferences(changes) {
    if (changes && changes.theme !== undefined) stored.theme = changes.theme;
    if (changes && changes.serious !== undefined) stored.serious = changes.serious;
    return Promise.resolve(preferencesAnswer());
  }

  // The folder dialog needs the window; the browser look answers with what it would do.
  function chooseRoot() {
    return Promise.resolve({
      flavors: null,
      message: say("warning", "the folder dialog opens in the window: run `cargo run -p mrgl-app`"),
    });
  }

  function diagnostics() {
    const text = [
      "mrgl: 0.1.0",
      `root: ${ROOT} (remembered)`,
      "home: ~\\AppData\\Local\\mrgl",
      "webview2: unknown",
      "github rate limit: unknown (made-up data)",
      "api keys: MRGL_GITHUB_TOKEN not set",
    ].join("\n");
    return Promise.resolve({
      text,
      copied: say("success", "diagnostics of 4 flavors copied, nothing to look at; paste them into a bug report"),
      failed: say("error", "the clipboard took no text; run `mrgl doctor` and copy its lines instead"),
    });
  }

  function resolve(flavorId, folder) {
    const flavor = flavors.find((item) => item.id === flavorId);
    const row = flavor && flavor.addons.find((item) => item.folder === folder);
    if (!row || !row.managed) return Promise.resolve({ pick: null, message: say("error", `${folder} is not managed by mrgl`) });
    const version = row.update ? row.update.version : row.version;
    const pick = { source: `${row.source.kind}:${row.source.id}`, release: version, version, asset: `${folder}-${version}.zip`, guess: false };
    return Promise.resolve({ pick, message: say("success", `${row.name} ${row.version} → ${version}`) });
  }

  function flavors_() {
    const count = flavors.reduce((sum, flavor) => sum + flavor.addons.length, 0);
    return Promise.resolve({
      root: ROOT,
      root_origin: "remembered",
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

  return { version, flavors: flavors_, chooseRoot, diagnostics, resolve, updateAll, preferences, setPreferences, settings };
});
