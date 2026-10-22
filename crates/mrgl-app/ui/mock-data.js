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
//   Data.resolve(flavorId, folder)   -> Promise<{ pick, message }>  the source's `Pick`
//   Data.checkUpdates(flavorId, progress)       -> Promise<{ flavor, messages }>  task 010: every managed
//                                       addon against its source, nothing installed
//   Data.updateAll(flavorId, progress)          -> Promise<{ flavor, messages }>  the check, then every update
//   Data.updateOne(flavorId, folder, progress)  -> Promise<{ flavor, messages }>  the check of one addon,
//                                       then its update; `progress(event)` per event as it happens, the
//                                       answer the flavor as it is now and the messages, the summary last;
//                                       one run at a time; in a flavor with the CurseForge app's marker
//                                       the first message is its warning (task 110)
//   Data.cancelUpdate()              -> Promise<message>           the run stops before its next addon; the
//                                       addon in flight finishes
//   Data.mark(flavorId, folder, action, value) -> Promise<{ flavor, messages }>  pin (at `value`, else the
//                                       installed version), unpin, ignore, unignore, channel (`value`
//                                       stable | prerelease)
//   Data.preferences()               -> Promise<{ theme, serious, serious_source, stored }>  task 034: the
//                                       theme and the serious switch as they apply, which source decided the
//                                       switch (flag | environment | file | default), what config.json stores
//   Data.setPreferences(changes)     -> Promise<the same>          `{ theme?, serious? }` into config.json
//   Data.toolbox(flavorId)           -> Promise<{ toolbox, changes, undone, zip, messages }>  task 074: the
//                                       flavor's sets, characters, last apply to undo and SavedVariables,
//                                       and whether its client runs
//   Data.applySet(flavorId, set, target)  -> the same            a stored set into the AddOns.txt of every
//                                       character, or of the one `target` `{ character, realm, account }`
//                                       names (the account for a realm and name under two accounts, 085)
//   Data.undoSet(flavorId)           -> the same                 085: the last apply that changed a file
//                                       put back
//   Data.editSet(flavorId, set, addon, action) -> the same       085: enable | disable | drop one addon of
//                                       a stored set, in sets.json only
//   Data.cleanSavedVariables(flavorId)    -> the same            the orphan SavedVariables into the dated
//                                       zip; the apply, the undo and the cleanup are refused while the client
//                                       runs (task 023), an edit is not
//   Data.adopt(flavorId)             -> Promise<{ adopt, flavor, adopted, messages }>  task 109: the adopt
//                                       report of the flavor's unmanaged addons (`mrgl adopt`), nothing written
//                                       with `adopt.answers`, the sources' answers it was built from (task
//                                       113: `{ flavor, asked_at, offers: { <spec>: { build, why, missing } },
//                                       folders: { <spec>: [folder] | null } }`)
//   Data.adoptApply(flavorId, alongside, answers) -> the same    the report's adoptable addons into the
//                                       sidecar (`adopt --apply`); in a flavor with the CurseForge app's marker
//                                       refused unless `alongside` (the dialog's confirmation, the window's
//                                       `--alongside-curseforge`), then a warning; `answers` the shown
//                                       report's, so no source is asked again while they are younger than an
//                                       hour (task 113)
//
// A message is `{ kind, murloc, plain }` as `mrgl_core::voice` says it: the kind picks the murloc line, the
// plain line carries every fact, and a serious voice sends the murloc line empty (D-005). The murloc table
// below is a copy of `voice.rs` for the sketch only: the real answers come from Rust, and nothing in the view
// composes a murloc line.
//
// A progress event of a run (task 010, `src/actions.rs`):
//
//   { phase,                                    checking | checked | stopped | updating | downloading |
//                                               installing | updated | installed-already | failed
//     folder, index, total,                     the row (the addon's first folder), its place and the count
//     check,                                    on `checked`: the check's answer, as a row's `check`
//     message }                                 progress; the stop's warning; 099: installed already, the
//                                               core's line (a warning that names a second entry of the
//                                               same source); a failed update's error
//
// A flavor is `Flavor` of `flavor.rs` plus its friendly name and its scanned addons:
//
//   { id, name, folder,                         the `.flavor.info` id, the friendly name, the folder's name
//     version, interface,                       the installed build and its interface number, or null
//     version_source, version_file,             019: build-info | client-exe, and the file it was read from
//     curse_client, addons_folder,              the CurseForge app's marker; 010: `Interface\AddOns` is there
//     warnings, addons }                        what reading the flavor met
//
// An addon is `Addon` of `scan.rs` reduced to what a row shows, plus the facts the sidecar (004), the
// sources (003), the update check (006, 068, 010), the Forever notes (011), pin / ignore / channel (021) and
// the dependency check (029) add:
//
//   { folder, name, version, linked,            from the scan: the folder, the TOC's plain title and version
//     source: { kind, id } | null,              github | wago | wowi | curseforge | zip | linked
//     managed, guessed,                         a sidecar entry; a source known only from a TOC hint
//     update: { version, release, prerelease } | null,  the update the check found
//     check: { state, label, latest, order, detail, line } | null,  the check's answer: offline from the
//                                               cache, or the window's last check; state as `UpdateState`
//                                               says it, order older | same | newer | null (077)
//     channel: stable | prerelease | null,      021, null for an unmanaged addon
//     pinned: string | null, ignored,           021
//     forever: { build, note, kind, verified, source, stale } | null,  011, on every row of Forever only:
//                                               build yes | no | unknown, a curated note or null
//     missing: [string],                        029: the required dependencies no addon folder answers (076: an embedded one too)
//     interface_check: { state, label, line } | null }  081: the interface check of the TOC the flavor
//                                               loads, state mismatch | out-of-date, label the mark `scan`
//                                               and `list` append to the line, line the warning's plain line
//
// The view derives the row's one state from these flags (`state.js`); the data never names that state. The
// interface state is the core's, drawn as a mark next to it.
//
// A toolbox answer (tasks 074 and 085, `src/toolbox.rs`): the toolbox as it is now (null when the flavor
// cannot be read), what an apply changed per character, what an undo put back, the zip a cleanup wrote, and
// the messages, the summary last:
//
//   { toolbox: { flavor, folder,                the flavor's id and folder
//                sets: [{ name, saved_at, enabled, disabled,
//                         addons: [{ name, state, installed }] }],  state enabled | disabled
//                sets_note,                     a message: how many sets, or none yet
//                characters: [character],       a character is `{ name, realm, label, account }`: label
//                                               `<realm>/<name>`, account null unless two accounts have the
//                                               realm and the name (085)
//                undo: { set, applied_at,       085: the last apply that changed a file, or null
//                        restore: [character],  the AddOns.txt the undo puts back
//                        remove: [character] } | null,  the AddOns.txt the apply created, which it removes
//                saved_variables: { files: [{ addon, place, realm, character, bytes, size, lua, bak,
//                                             status }],          installed | orphan | client | unknown
//                                   bytes, size, orphans, orphan_bytes, orphan_size, summary },
//                client } | null,               a warning message while the client runs, else null
//     changes: [{ character, changed: [{ name, state }], created }],
//     undone: { set, applied_at, restore, remove } | null,  085: what an undo put back
//     zip: string | null,
//     messages: [message] }
//
// An adopt answer (task 109, `src/toolbox.rs` over `mrgl_core::adopt`): the report as it is now (null when
// nothing was read, or the apply was refused before it asked a source), the flavor as it is now after an
// apply (the adopted rows managed), the `adopted …` line of each entry written, the messages, the summary
// last:
//
//   { adopt: { flavor, folder, curse_client,    the flavor's id and folder, the CurseForge app's marker
//              marker,                          its warning line, or null
//              rows: [{ name, folders, version, the core's row, as `mrgl adopt --json` writes it
//                       sources: [{ spec, source, from, build, why, needs, missing, taken_by }],
//                       pages, class, adopt_as, needs,   class without-curseforge | only-curseforge |
//                                               no-build | no-source
//                       line, verdict, details }],  the addon's line, what adopt makes of it, a line
//                                               per source and page
//              left_out, counts: { addons, without_curseforge, only_curseforge, no_build, no_source,
//                                  adoptable_now },
//              summary } | null,                a message: the counts and how many an apply adopts
//     flavor: flavor | null,
//     adopted: [string],
//     messages: [message] }
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

  // The words of the check's states, as `UpdateState::label` gives them.
  const LABELS = {
    "up-to-date": "up to date",
    "installed-newer": "newer than the release",
    "update-available": "update available",
    "guess-only": "guess only",
    pinned: "pinned",
    ignored: "ignored",
    pending: "pending approval",
    "source-gone": "source gone",
    "not-checked": "not checked",
    failed: "check failed",
    "rate-limited": "not checked (rate limit)",
  };

  // A check's answer for a row, with the data line the check writes; the order of the installed version
  // against the made-up release is the row's `newer` mark, else the same or an older one.
  function checkFact(row, state, latest, detail) {
    const versions = state === "update-available" ? `${row.version} -> ${latest}` : row.version;
    const order = !latest ? null : latest === row.version ? "same" : row.newer ? "newer" : "older";
    let text = LABELS[state];
    if (state === "pinned") {
      const against = { newer: `newer than the current release ${latest}, left alone`, older: `${latest} is out, left alone`, same: "up to date" };
      text = `pinned at ${row.pinned}, ${against[order]}`;
    }
    if (state === "installed-newer") text = `newer than the current release ${latest}, left alone`;
    if (state === "ignored") text = "ignored, not checked";
    if (detail && ["not-checked", "pending", "source-gone", "failed", "rate-limited"].includes(state)) text = `${text}: ${detail}`;
    return { state, label: LABELS[state], latest: latest || null, order, detail: detail || null, line: `${row.name}  ${versions}  ${text}` };
  }

  // The core's line for a spec of a source this build does not have (task 092, `sources/router.rs`): the
  // spec in its canonical spelling and the source's name. Task 107: the sketch's build has the sources of
  // the core's router, GitHub, WoWInterface (094) and Wago (095, its key set), and no CurseForge source.
  const SPEC_PREFIX = { github: "github", wago: "wago", wowi: "wowi", curseforge: "curse" };
  const SOURCE_NAME = { github: "GitHub", wago: "Wago", wowi: "WoWInterface", curseforge: "CurseForge" };
  const BUILD_SOURCES = ["github", "wowi", "wago"];
  function notConfigured(source) {
    const spec = `${SPEC_PREFIX[source.kind]}:${source.id}`;
    return `source not configured: ${spec} (this build has no ${SOURCE_NAME[source.kind]} source)`;
  }

  // What the made-up check answers for a managed row: a row of a source of the build from the made-up
  // current release, a source the sketch's build does not have not asked, in the core's words.
  function checkOf(row) {
    if (!row.managed) return null;
    if (row.linked) return checkFact(row, "not-checked", null, "a linked development folder");
    if (row.ignored) return checkFact(row, "ignored", null, null);
    if (row.source.kind === "zip") return checkFact(row, "not-checked", null, "a zip in hand; a newer zip goes in with mrgl add <zip>");
    if (!BUILD_SOURCES.includes(row.source.kind)) return checkFact(row, "not-checked", null, notConfigured(row.source));
    const latest = row.latest || row.version;
    if (row.pinned) return checkFact(row, "pinned", latest, null);
    if (row.newer) return checkFact(row, "installed-newer", latest, null);
    return checkFact(row, latest === row.version ? "up-to-date" : "update-available", latest, null);
  }

  // A made-up row; `latest` (the current release of its source) and `newer` (installed is newer than it)
  // are the mock's own and never reach the view.
  function addon(folder, name, version, extra) {
    const row = Object.assign(
      {
        folder,
        name,
        version,
        linked: false,
        source: null,
        managed: false,
        guessed: false,
        update: null,
        check: null,
        channel: null,
        pinned: null,
        ignored: false,
        forever: null,
        missing: [],
        interface_check: null,
      },
      extra
    );
    if (row.managed && !row.channel) row.channel = "stable";
    const latest = row.latest;
    const newer = row.newer;
    delete row.latest;
    delete row.newer;
    Object.defineProperty(row, "latest", { value: latest, writable: true, enumerable: false });
    Object.defineProperty(row, "newer", { value: Boolean(newer), writable: true, enumerable: false });
    refresh(row);
    return row;
  }

  // The row's check and update from its made-up release, as the offline check would answer.
  function refresh(row) {
    row.check = checkOf(row);
    row.update =
      row.check && row.check.state === "update-available"
        ? { version: row.check.latest, release: `v${row.check.latest}`, prerelease: row.channel === "prerelease" }
        : null;
  }

  // The interface check of a row (task 081), in the core's words (`interface.rs`): a TOC for another
  // expansion, or an interface of the build's own expansion below the build's.
  const mismatch = (folder, interfaces, build) => ({
    state: "mismatch",
    label: "interface mismatch",
    line: `${folder}: interface ${interfaces} is not for the build ${build}`,
  });
  const outOfDate = (folder, highest, build, interface_) => ({
    state: "out-of-date",
    label: "out of date",
    line: `${folder}: interface ${highest} is out of date for the build ${build} (${interface_})`,
  });

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
      addons_folder: true,
      warnings: [],
      addons: [
        addon("BigWigs", "BigWigs", "v400.3", { source: github("BigWigsMods/BigWigs"), managed: true, latest: "v401.0" }),
        addon("Details", "Details! Damage Meter", "#Details.20261010.14521.165", { source: curseforge("61284"), guessed: true }),
        addon("DBM-Core", "Deadly Boss Mods", "12.1.12", { source: curseforge("3358"), guessed: true }),
        addon("Plater", "Plater Nameplates", "Plater-v665-Retail", { source: wago("kRNLep6o"), managed: true }),
        addon("WeakAuras", "WeakAuras", "5.19.11", { source: github("WeakAuras/WeakAuras2"), managed: true, latest: "5.19.12" }),
        addon("WeakAuras_Options", "WeakAuras Options", "5.19.11", { source: github("WeakAuras/WeakAuras2"), managed: true }),
        addon("MyUnitFrames", "My Unit Frames", "0.4.0", { source: zip("MyUnitFrames-0.4.0.zip"), managed: true, interface_check: outOfDate("MyUnitFrames", 120100, "12.1.5.70448", 120105) }),
        addon("OldMinimap", "Old Minimap", "1.0", { interface_check: mismatch("OldMinimap", "110207", "12.1.5.70448") }),
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
      addons_folder: true,
      warnings: [],
      addons: [
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true, latest: "11.3.0-beta", channel: "prerelease" }),
        addon("AtlasLootClassic", "AtlasLoot Classic", "v4.2.0", { source: wowi("24976"), managed: true, latest: "v4.2.1" }),
        addon("Leatrix_Plus", "Leatrix Plus", "5.5.26", { source: curseforge("289009"), managed: true }),
        addon("NovaWorldBuffs", "Nova World Buffs", "2.74", { source: github("mt-paul/NovaWorldBuffs"), managed: true, pinned: "2.74", latest: "2.75" }),
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
      addons_folder: true,
      warnings: ["no row for wow_classic_era in .build.info: the build was read from WowClassic.exe"],
      addons: [
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true, latest: "11.2.0", newer: true }),
        addon("pfQuest", "pfQuest", "6.4.1", { source: github("shagu/pfQuest"), managed: true, latest: "6.5.0" }),
        addon("AtlasLootClassic", "AtlasLoot Classic", "v5.0.0-beta", { source: github("Hoizame/AtlasLootClassic"), managed: true, pinned: "v5.0.0-beta", latest: "v4.9.0", newer: true }),
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
      addons_folder: true,
      warnings: [],
      addons: [
        addon("BigWigs", "BigWigs", "v400.3", { source: github("BigWigsMods/BigWigs"), managed: true, latest: "v401.0", forever: { build: "yes", note: null } }),
        addon("Questie", "Questie", "11.2.3", { source: github("Questie/Questie"), managed: true, latest: "11.3.0", forever: { build: "yes", note: null } }),
        addon("Details", "Details! Damage Meter", "#Details.20261010.14521.165", { source: curseforge("61284"), guessed: true, forever: { build: "unknown", note: "hidden combat values: the meter shows nothing" } }),
        addon("Plater", "Plater Nameplates", "Plater-v665-Forever", { source: wago("kRNLep6o"), managed: true, forever: { build: "yes", note: "no combat log: threat colors are off" } }),
        addon("MyAuras", "My Auras", "0.2.0-dev", { source: linked("D:\\code\\wow-addons\\MyAuras"), linked: true, forever: { build: "unknown", note: null } }),
        addon("MyUnitFrames", "My Unit Frames", "0.4.0", { source: linked("D:\\code\\wow-addons\\MyUnitFrames"), linked: true, forever: { build: "unknown", note: null } }),
        addon("Leatrix_Plus", "Leatrix Plus", "5.5.26", { source: curseforge("289009"), guessed: true, forever: { build: "no", note: null } }),
        addon("TomTom", "TomTom", "v5.5.4-mists", { source: github("Ludovicus-Maior/TomTom"), managed: true, latest: "v5.5.5-forever", forever: { build: "yes", note: null } }),
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

  // How long each step of the made-up run takes; the tests set it to 0.
  const settings = { delay: 400 };

  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  // The one run at a time, and its cancel.
  const run = { going: false, cancel: false };

  function cancelUpdate() {
    if (!run.going) return Promise.resolve(say("nothing", "no update is running, nothing to cancel"));
    run.cancel = true;
    return Promise.resolve(say("progress", "cancelling: the addon in flight finishes, no other one starts"));
  }

  // The summary as `UpdateCheck::summary` writes it, from the states the check found: what was updated or
  // is available, every other state's count, the kind by what needs a look.
  function summary(flavor, states, of) {
    if (states.length === 0) return say("nothing", `no managed addons in ${flavor.folder}, nothing to update`);
    const count = (state) => states.filter((found) => found === state).length;
    const available = count("update-available");
    const parts = [];
    if (of.check) {
      if (available) parts.push(available === 1 ? "1 update available" : `${available} updates available`);
    } else {
      if (of.updated) parts.push(`${addons(of.updated)} updated`);
      if (of.failed) parts.push(`${of.failed} failed`);
      if (of.left) parts.push(`${of.left} not updated (cancelled)`);
    }
    for (const state of ["up-to-date", "installed-newer", "pinned", "ignored", "pending", "guess-only", "source-gone", "not-checked", "rate-limited", "failed"]) {
      const n = count(state);
      if (n) parts.push(`${n} ${LABELS[state]}`);
    }
    const attention = of.failed || of.left;
    const found = of.check ? available > 0 : of.updated > 0;
    const kind = attention ? "warning" : found ? "success" : "nothing";
    const text = of.check ? `checked ${flavor.folder}: ${parts.join(", ")}` : of.updated ? parts.join(", ") : `nothing updated: ${parts.join(", ")}`;
    return say(kind, text);
  }

  // The warning of an install path in a folder the CurseForge app manages, as `adopt::curse_client_warning`
  // says it (task 098); null without the marker.
  function curseClientWarning(flavor) {
    if (!flavor.curse_client) return null;
    return say(
      "warning",
      `the CurseForge app manages ${flavor.folder} too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager`
    );
  }

  // The made-up run of `checkUpdates`, `updateAll` and `updateOne`: the check of every managed row (or the
  // one `folder` names), then, with `install`, every update it found. In era the first update fails, so the
  // error and the warning show somewhere. A flavor the CurseForge app manages warns first (task 110).
  async function runUpdate(flavorId, folder, progress, install) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return { flavor: null, messages: [say("error", `no flavor ${flavorId} under ${ROOT}`)] };
    if (run.going) return { flavor: null, messages: [say("warning", "an update is running already; wait for it to end or cancel it")] };
    run.going = true;
    run.cancel = false;
    const warned = (messages) => [curseClientWarning(flavor), ...messages].filter(Boolean);
    const emit = (phase, row, index, total, kind, plain, check) =>
      typeof progress === "function" &&
      progress({ phase, folder: row ? row.folder : null, index, total, check: check || null, message: say(kind, plain) });
    try {
      const rows = flavor.addons.filter((row) => row.managed && (!folder || row.folder === folder));
      const total = rows.length;
      let checked = 0;
      for (const row of rows) {
        if (run.cancel) break;
        checked += 1;
        const asked = row.check && !["ignored", "not-checked"].includes(row.check.state);
        if (asked) {
          emit("checking", row, checked, total, "progress", `checking ${row.name} against ${row.source.kind}:${row.source.id} (${checked} of ${total})`);
          await wait(settings.delay / 4);
        }
        refresh(row);
        emit("checked", row, checked, total, "progress", row.check.line, row.check);
      }
      if (run.cancel) {
        const what = install ? "update" : "check";
        return { flavor, messages: warned([say("nothing", `${what} cancelled after checking ${checked} of ${total} addons in ${flavor.folder}; nothing updated`)]) };
      }
      const states = rows.map((row) => row.check.state);
      if (!install) return { flavor, messages: warned([summary(flavor, states, { check: true })]) };
      const due = rows.filter((row) => row.update);
      const of = { updated: 0, failed: 0, left: 0 };
      for (const [position, row] of due.entries()) {
        if (run.cancel) {
          of.left = due.length - position;
          break;
        }
        const index = position + 1;
        const asset = `${row.folder}-${row.update.version}.zip`;
        emit("updating", row, index, due.length, "progress", `updating ${row.name} from ${row.version} to ${row.update.version} (${index} of ${due.length})`);
        await wait(settings.delay);
        emit("downloading", row, index, due.length, "progress", `downloading ${asset}`);
        await wait(settings.delay);
        emit("installing", row, index, due.length, "progress", `installing ${asset} into ${flavor.folder}`);
        await wait(settings.delay);
        if (flavor.id === "wow_classic_era" && position === 0) {
          emit("failed", row, index, due.length, "error", `${row.name}: cannot reach https://github.com/: network unreachable`);
          of.failed += 1;
          continue;
        }
        const from = row.version;
        row.version = row.update.version;
        refresh(row);
        emit("updated", row, index, due.length, "progress", `updated ${row.name} from ${from} to ${row.version} in ${flavor.folder}`);
        of.updated += 1;
      }
      return { flavor, messages: warned([summary(flavor, states, of)]) };
    } finally {
      run.going = false;
      run.cancel = false;
    }
  }

  const checkUpdates = (flavorId, progress) => runUpdate(flavorId, null, progress, false);
  const updateAll = (flavorId, progress) => runUpdate(flavorId, null, progress, true);
  const updateOne = (flavorId, folder, progress) => runUpdate(flavorId, folder, progress, true);

  // The made-up marks, with the lines of `Mark::line`.
  function mark(flavorId, folder, action, value) {
    const flavor = flavors.find((item) => item.id === flavorId);
    const row = flavor && flavor.addons.find((item) => item.folder === folder);
    if (!row) return Promise.resolve({ flavor: null, messages: [say("error", `addon not found: ${folder} (flavor ${flavorId})`)] });
    if (!row.managed) {
      return Promise.resolve({ flavor, messages: [say("error", `${folder} is not managed by mrgl in ${flavor.folder}; only an addon mrgl installed can be marked`)] });
    }
    const name = row.name;
    const where = flavor.folder;
    let line;
    let changed = true;
    if (action === "pin") {
      const at = value || row.version;
      changed = row.pinned !== at;
      line = changed ? `pinned ${name} at ${at} in ${where}` : `${name} is already pinned at ${at} in ${where}`;
      row.pinned = at;
    } else if (action === "unpin") {
      changed = Boolean(row.pinned);
      line = changed ? `unpinned ${name} in ${where}` : `${name} is not pinned in ${where}`;
      row.pinned = null;
    } else if (action === "ignore") {
      changed = !row.ignored;
      line = changed ? `ignoring ${name} in ${where}` : `${name} is already ignored in ${where}`;
      row.ignored = true;
    } else if (action === "unignore") {
      changed = row.ignored;
      line = changed ? `${name} is no longer ignored in ${where}` : `${name} is not ignored in ${where}`;
      row.ignored = false;
    } else if (action === "channel" && (value === "stable" || value === "prerelease")) {
      changed = row.channel !== value;
      line = changed ? `${name} follows the ${value} channel in ${where}` : `${name} is already on the ${value} channel in ${where}`;
      row.channel = value;
    } else {
      return Promise.resolve({ flavor: null, messages: [say("error", `unknown row action ${action}: pin, unpin, ignore, unignore or channel`)] });
    }
    refresh(row);
    return Promise.resolve({ flavor, messages: [say(changed ? "success" : "nothing", line)] });
  }

  // ---- the toolbox (task 074): made-up characters, sets and SavedVariables per flavor. Retail's client
  // "runs", so the refusal of both writes shows in the sketch.

  const HOME = "%LOCALAPPDATA%\\mrgl";

  // A byte count as `size_text` of the core writes it.
  function sizeText(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB", "TB", "PB"];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value.toFixed(1)} ${units[unit]}`;
  }

  const count = (n, noun) => (n === 0 ? `no ${noun}s` : n === 1 ? `1 ${noun}` : `${n} ${noun}s`);

  // One flavor's made-up toolbox: three characters with their AddOns.txt, two of them of one realm and name
  // under two made-up accounts (task 085), two sets, the SavedVariables, no apply to undo yet.
  function toolboxState(flavor) {
    const folders = flavor.addons.map((addon) => addon.folder);
    const characters = [
      { account: "MOCKACCOUNT", name: "Mrglina", realm: "Made-up Realm", addons: {} },
      { account: "MOCKACCOUNT", name: "Grrlwave", realm: "Second Realm", addons: {} },
      { account: "MOCKTWIN", name: "Mrglina", realm: "Made-up Realm", addons: {} },
    ];
    for (const character of characters) for (const folder of folders) character.addons[folder] = "enabled";
    if (folders[1]) characters[1].addons[folders[1]] = "disabled";
    const raid = {};
    for (const folder of folders.slice(0, 3)) raid[folder] = "enabled";
    if (folders[3]) raid[folders[3]] = "disabled";
    const levelling = {};
    if (folders[0]) levelling[folders[0]] = "disabled";
    if (folders[1]) levelling[folders[1]] = "enabled";
    levelling.QuestHelperOld = "enabled";
    const file = (addon, place, bytes, status, bak) => ({
      addon,
      place,
      realm: place === "account-wide" ? null : place.split(" / ")[0],
      character: place === "account-wide" ? null : place.split(" / ")[1],
      bytes,
      size: sizeText(bytes),
      lua: true,
      bak: Boolean(bak),
      status,
    });
    const files = [
      ...folders.map((folder, index) =>
        file(folder, "account-wide", 4096 * (folders.length - index) + 517, "installed", index % 2 === 0)
      ),
      file("OldDamageMeter", "account-wide", 2621440, "orphan", true),
      file("Blizzard_Console", "account-wide", 812, "client"),
      file("RetiredBags", "Second Realm / Grrlwave", 18944, "orphan"),
    ].sort((a, b) => b.bytes - a.bytes);
    return {
      characters,
      sets: {
        raid: { saved_at: "2026-10-20T19:02:11Z", addons: raid },
        levelling: { saved_at: "2026-10-21T08:40:00Z", addons: levelling },
      },
      files,
      undo: null,
      client: flavor.id === "wow" ? { exe: "Wow.exe", pid: 4242 } : null,
    };
  }

  // A made-up character as `CharacterView` has it: the account only when another account has the realm
  // and the name.
  function characterView(state, character) {
    const twin = state.characters.some(
      (other) => other !== character && other.realm === character.realm && other.name === character.name
    );
    return {
      name: character.name,
      realm: character.realm,
      label: `${character.realm}/${character.name}`,
      account: twin ? character.account : null,
    };
  }

  // The undo record as `UndoView` has it: every file the apply replaced goes back (none was created).
  function undoView(state, undo) {
    return {
      set: undo.set,
      applied_at: undo.applied_at,
      restore: undo.files.map((file) => characterView(state, file.character)),
      remove: [],
    };
  }

  const toolboxStates = new Map();

  function toolboxStateOf(flavor) {
    if (!toolboxStates.has(flavor.id)) toolboxStates.set(flavor.id, toolboxState(flavor));
    return toolboxStates.get(flavor.id);
  }

  function refusal(flavor, client) {
    return `the client is running in ${flavor.folder} (${client.exe}, pid ${client.pid}): close the game first, nothing under WTF is written while it runs`;
  }

  function installedOf(flavor) {
    return new Set(flavor.addons.map((addon) => addon.folder.toLowerCase()));
  }

  // The toolbox of a flavor, in the shape of `ToolboxView`.
  function toolboxView(flavor) {
    const state = toolboxStateOf(flavor);
    const installed = installedOf(flavor);
    const sets = Object.keys(state.sets)
      .sort()
      .map((name) => {
        const set = state.sets[name];
        const addons = Object.keys(set.addons)
          .sort()
          .map((addon) => ({ name: addon, state: set.addons[addon], installed: installed.has(addon.toLowerCase()) }));
        return {
          name,
          saved_at: set.saved_at,
          enabled: addons.filter((addon) => addon.state === "enabled").length,
          disabled: addons.filter((addon) => addon.state === "disabled").length,
          addons,
        };
      });
    const files = state.files;
    const bytes = files.reduce((sum, item) => sum + item.bytes, 0);
    const orphans = files.filter((item) => item.status === "orphan");
    const orphanBytes = orphans.reduce((sum, item) => sum + item.bytes, 0);
    const counted = files.length === 1 ? "1 SavedVariables file" : `${files.length} SavedVariables files`;
    // The line of the core's `Listing::summary` (task 086): the verb agrees with the orphan count.
    const [has, them] = orphans.length === 1 ? ["has", "it"] : ["have", "them"];
    const summary =
      files.length === 0
        ? say("nothing", `no SavedVariables files in the WTF folder of ${flavor.folder}`)
        : orphans.length === 0
          ? say("success", `${counted} of ${flavor.folder}, ${sizeText(bytes)} in all, no orphans`)
          : say(
              "warning",
              `${counted} of ${flavor.folder}, ${sizeText(bytes)} in all; ${count(orphans.length, "orphan")} (${sizeText(orphanBytes)}) ${has} no addon folder: the cleanup moves ${them} into a backup zip`
            );
    return {
      flavor: flavor.id,
      folder: flavor.folder,
      sets,
      sets_note:
        sets.length === 0
          ? say("nothing", `no sets of ${flavor.folder} yet: \`mrgl set save <name>\` saves one`)
          : say("success", `${count(sets.length, "set")} of ${flavor.folder} in ${HOME}\\${flavor.id}\\sets.json`),
      characters: state.characters.map((character) => characterView(state, character)),
      undo: state.undo ? undoView(state, state.undo) : null,
      saved_variables: {
        files: files.map((item) => Object.assign({}, item)),
        bytes,
        size: sizeText(bytes),
        orphans: orphans.length,
        orphan_bytes: orphanBytes,
        orphan_size: sizeText(orphanBytes),
        summary,
      },
      client: state.client ? say("warning", refusal(flavor, state.client)) : null,
    };
  }

  function toolboxAnswer(flavor, messages, extra) {
    return Object.assign({ toolbox: toolboxView(flavor), changes: [], undone: null, zip: null, messages }, extra || {});
  }

  function noFlavor(flavorId) {
    return {
      toolbox: null,
      changes: [],
      undone: null,
      zip: null,
      messages: [say("error", `no flavor ${flavorId}: no such flavor folder under ${ROOT}`)],
    };
  }

  function toolbox(flavorId) {
    const flavor = flavors.find((item) => item.id === flavorId);
    return Promise.resolve(flavor ? toolboxAnswer(flavor, []) : noFlavor(flavorId));
  }

  // The made-up apply, with the lines of `mrgl set apply`.
  function applySet(flavorId, setName, target) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(noFlavor(flavorId));
    const state = toolboxStateOf(flavor);
    if (state.client) return Promise.resolve(toolboxAnswer(flavor, [say("error", refusal(flavor, state.client))]));
    const name = Object.keys(state.sets).find((key) => key.toLowerCase() === String(setName).toLowerCase());
    if (!name) {
      const known = Object.keys(state.sets).sort().join(", ");
      return Promise.resolve(
        toolboxAnswer(flavor, [say("error", `set not found: ${setName} (flavor ${flavor.folder}); the saved sets: ${known}`)])
      );
    }
    const one = Boolean(target && target.character);
    const same = (a, b) => String(a).toLowerCase() === String(b).toLowerCase();
    const chosen = one
      ? state.characters.filter(
          (character) =>
            same(character.name, target.character) &&
            (!target.realm || same(character.realm, target.realm)) &&
            (!target.account || same(character.account, target.account))
        )
      : state.characters;
    if (one && chosen.length > 1) {
      const where = chosen.map((character) => `${character.realm} (${character.account})`).join(", ");
      return Promise.resolve(
        toolboxAnswer(flavor, [
          say(
            "error",
            `character ${target.character} matches ${chosen.length} characters (flavor ${flavor.folder}): ${where}; the same realm is under more than one account: pick the account too`
          ),
        ])
      );
    }
    if (one && chosen.length !== 1) {
      const known = state.characters.map((character) => `${character.name} (${character.realm})`).join(", ");
      return Promise.resolve(
        toolboxAnswer(flavor, [
          say("error", `character not found: ${target.character} (flavor ${flavor.folder}); the characters: ${known}`),
        ])
      );
    }
    const set = state.sets[name].addons;
    const kept = [];
    const changes = chosen.map((character) => {
      const before = Object.assign({}, character.addons);
      const changed = [];
      for (const addon of Object.keys(set).sort()) {
        if (character.addons[addon] !== set[addon]) {
          character.addons[addon] = set[addon];
          changed.push({ name: addon, state: set[addon] });
        }
      }
      if (changed.length) kept.push({ character, addons: before });
      return { character: characterView(state, character), changed, created: false };
    });
    const lines = changes.reduce((sum, change) => sum + change.changed.length, 0);
    // An apply that changes a file keeps what it replaced for the undo; one that changes nothing keeps the
    // record before it.
    if (kept.length) state.undo = { set: name, applied_at: new Date().toISOString().replace(/\.\d+Z$/, "Z"), files: kept };
    const installed = installedOf(flavor);
    const missing = Object.keys(set)
      .sort()
      .filter((addon) => !installed.has(addon.toLowerCase()));
    const messages = [];
    if (missing.length) {
      messages.push(
        say(
          "warning",
          `set ${name} names addons that are not installed in ${flavor.folder}, their lines are written all the same: ${missing.join(", ")}`
        )
      );
    }
    const whom = one ? changes[0].character.label : count(changes.length, "character");
    const matched = one ? `${whom} of ${flavor.folder}` : `every character of ${flavor.folder} (${whom})`;
    messages.push(
      lines === 0
        ? say("nothing", `set ${name} already matches ${matched}, nothing written`)
        : say("success", `applied set ${name} to ${whom} of ${flavor.folder}: ${lines} addon line${lines === 1 ? "" : "s"} changed`)
    );
    return Promise.resolve(toolboxAnswer(flavor, messages, { changes }));
  }

  // The made-up undo, with the lines of `mrgl set undo`.
  function undoSet(flavorId) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(noFlavor(flavorId));
    const state = toolboxStateOf(flavor);
    if (state.client) return Promise.resolve(toolboxAnswer(flavor, [say("error", refusal(flavor, state.client))]));
    const undo = state.undo;
    if (!undo) {
      return Promise.resolve(
        toolboxAnswer(flavor, [
          say(
            "error",
            `nothing to undo in ${flavor.folder}: no set apply is recorded; an apply that changes a file keeps what it replaced, and an undo puts it back once`
          ),
        ])
      );
    }
    for (const file of undo.files) file.character.addons = Object.assign({}, file.addons);
    const undone = undoView(state, undo);
    state.undo = null;
    const files = undo.files.length === 1 ? "1 AddOns.txt" : `${undo.files.length} AddOns.txt files`;
    const line = `undid the apply of set ${undo.set} to ${flavor.folder} (${undo.applied_at}): ${files} put back`;
    return Promise.resolve(toolboxAnswer(flavor, [say("success", line)], { undone }));
  }

  // The made-up edit of one addon in a stored set, with the lines of `mrgl set enable|disable|drop`; it
  // writes the set only, so a running client does not refuse it.
  function editSet(flavorId, setName, addonName, action) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(noFlavor(flavorId));
    const state = toolboxStateOf(flavor);
    const name = Object.keys(state.sets).find((key) => key.toLowerCase() === String(setName).toLowerCase());
    if (!name) return Promise.resolve(toolboxAnswer(flavor, [say("error", `set not found: ${setName} (flavor ${flavor.folder})`)]));
    if (!["enable", "disable", "drop"].includes(action)) {
      return Promise.resolve(toolboxAnswer(flavor, [say("error", `no set edit ${action}: enable, disable or drop`)]));
    }
    const set = state.sets[name];
    const stored = Object.keys(set.addons).find((key) => key.toLowerCase() === String(addonName).toLowerCase());
    const addon = stored || addonName;
    const before = stored ? set.addons[stored] : null;
    const after = action === "drop" ? null : `${action}d`;
    if (action === "drop" && !before) {
      return Promise.resolve(
        toolboxAnswer(flavor, [say("error", `set ${name} (flavor ${flavor.folder}) has no addon ${addonName}`)])
      );
    }
    if (before === after) {
      return Promise.resolve(
        toolboxAnswer(flavor, [say("nothing", `set ${name} of ${flavor.folder} has ${addon} ${after} already, nothing changed`)])
      );
    }
    if (after) set.addons[addon] = after;
    else delete set.addons[addon];
    set.saved_at = new Date().toISOString().replace(/\.\d+Z$/, "Z");
    const states = Object.values(set.addons);
    const enabled = states.filter((state_) => state_ === "enabled").length;
    const counted = `${count(states.length, "addon").replace(/^no /, "0 ")} (${enabled} enabled, ${states.length - enabled} disabled)`;
    const what = !after
      ? `dropped ${addon} from set ${name} of ${flavor.folder}: ${counted}`
      : before
        ? `${addon} is ${after} in set ${name} of ${flavor.folder} now, was ${before}: ${counted}`
        : `added ${addon} to set ${name} of ${flavor.folder}, ${after}: ${counted}`;
    const messages = [];
    if (after && !installedOf(flavor).has(addon.toLowerCase())) {
      messages.push(say("warning", `${addon} is not installed in ${flavor.folder}; the set keeps it all the same`));
    }
    messages.push(say("success", `${what}; no AddOns.txt was touched`));
    return Promise.resolve(toolboxAnswer(flavor, messages));
  }

  // The made-up cleanup, with the lines of `mrgl savedvars --clean`.
  function cleanSavedVariables(flavorId) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(noFlavor(flavorId));
    const state = toolboxStateOf(flavor);
    if (state.client) return Promise.resolve(toolboxAnswer(flavor, [say("error", refusal(flavor, state.client))]));
    const moved = state.files.filter((item) => item.status === "orphan");
    if (moved.length === 0) {
      return Promise.resolve(
        toolboxAnswer(flavor, [say("nothing", `no orphans in the SavedVariables of ${flavor.folder}, nothing moved`)])
      );
    }
    state.files = state.files.filter((item) => item.status !== "orphan");
    const files = moved.reduce((sum, item) => sum + (item.lua ? 1 : 0) + (item.bak ? 1 : 0), 0);
    const bytes = moved.reduce((sum, item) => sum + item.bytes, 0);
    const zip = `${HOME}\\backups\\${flavor.id}\\SavedVariables-2026-10-22.zip`;
    const line = `moved ${count(moved.length, "orphan")} of ${flavor.folder} (${count(files, "file")}, ${sizeText(bytes)}) into ${zip}; unzip it into the WTF folder to put them back`;
    return Promise.resolve(toolboxAnswer(flavor, [say("success", line)], { zip }));
  }

  // ---- adopt (task 109): the made-up sources of the unmanaged rows, in the order of the core's hints
  // (`adopt.rs`: `X-Wago-ID`, `X-WoWI-ID`, `X-Curse-Project-ID`, `X-Website` on github.com, then the GitHub
  // mapping), each with a made-up build answer for the flavor. The sketch's run has no key: a Wago or a
  // CurseForge source needs its key and is never written. A row without an entry has no source.
  const HINT_LABELS = {
    "wago-id": "X-Wago-ID",
    "wowi-id": "X-WoWI-ID",
    "curse-project-id": "X-Curse-Project-ID",
    website: "X-Website",
    mapping: "the GitHub mapping",
  };
  const KEY_NAMES = { wago: "MRGL_WAGO_KEY", curse: "MRGL_CURSEFORGE_KEY" };
  const KEY_SOURCES = { wago: "Wago", curse: "CurseForge" };

  function candidate(spec, from, build, why) {
    const source = spec.slice(0, spec.indexOf(":"));
    const needs = KEY_NAMES[source] || null;
    return {
      spec,
      source,
      from,
      build: needs && source === "curse" ? "unknown" : build,
      why: needs && source === "curse" ? `source not configured: ${spec} (${KEY_NAMES[source]} is not set)` : why,
      needs,
      missing: needs ? `source not configured: ${spec} (${needs} is not set; ${KEY_SOURCES[source]} needs its key)` : null,
      taken_by: null,
    };
  }

  const ADOPT_SOURCES = {
    Details: {
      sources: (flavor) => [
        candidate("curse:61284", "curse-project-id"),
        candidate("github:Tercioo/Details-Damage-Meter", "mapping", "yes", `release.json names ${flavorName(flavor)}`),
      ],
    },
    "DBM-Core": {
      sources: (flavor) => [
        candidate("wago:qv63LLKb", "wago-id", "yes", `Wago lists a release for ${flavorName(flavor)}`),
        candidate("curse:3358", "curse-project-id"),
      ],
    },
    Leatrix_Plus: { sources: () => [candidate("curse:289009", "curse-project-id")] },
    Bartender4: {
      sources: (flavor) => [
        candidate("wowi:11190", "wowi-id", "yes", "WoWInterface lists compatibility 1.15.9 (Made-up Era Patch)"),
        candidate("curse:4525", "curse-project-id"),
      ],
    },
    OldMinimap: {
      sources: (flavor) => [
        candidate(
          "github:example/OldMinimap",
          "website",
          "no",
          `no asset for flavor ${flavorName(flavor)} in github:example/OldMinimap v1.0 (release.json names mainline; assets: OldMinimap-1.0.zip, release.json; looked at 1 release)`
        ),
      ],
    },
    RaidLogger: { sources: () => [], pages: ["https://www.example.invalid/raid-logger"] },
  };

  // The flavor as the core's lines name it: the friendly name.
  function flavorName(flavor) {
    return flavor.name;
  }

  // The source's line under its addon, as `Candidate::line` writes it (without the indent).
  function candidateLine(source, flavor) {
    let line = `${source.spec} from ${HINT_LABELS[source.from]}: `;
    const lacks = source.needs ? `needs ${source.needs}` : source.missing;
    if (lacks && source.build === "unknown") line += lacks;
    else {
      line += `${flavorName(flavor)} build ${source.build} (${source.why})`;
      if (lacks) line += `; ${lacks}`;
    }
    if (source.taken_by) line += `; the source of the managed ${source.taken_by} already`;
    return line;
  }

  // One unmanaged row of the report, as `adopt::report` makes it: the class from the answers, the spec
  // to adopt it as (the first usable `yes`, else the first usable `unknown`), the keys that would let it.
  function adoptRow(row, flavor) {
    const made = ADOPT_SOURCES[row.folder] || { sources: () => [] };
    const sources = made.sources(flavor);
    const pages = made.pages || [];
    const offers = sources.filter((source) => source.build !== "no");
    const klass = offers.some((source) => source.source !== "curse")
      ? "without-curseforge"
      : offers.length
        ? "only-curseforge"
        : sources.length
          ? "no-build"
          : "no-source";
    const usable = (source) => !source.missing && !source.taken_by;
    const chosen =
      sources.find((source) => usable(source) && source.build === "yes") ||
      sources.find((source) => usable(source) && source.build === "unknown");
    const adopt_as = chosen ? chosen.spec : null;
    const needs = adopt_as ? [] : [...new Set(offers.map((source) => source.needs).filter(Boolean))];
    const needsText = needs.length ? `needs ${needs.join(" or ")}` : null;
    const verdict = adopt_as
      ? `adopt as ${adopt_as}`
      : klass === "without-curseforge"
        ? needsText || "no source this run can write"
        : klass === "only-curseforge"
          ? needsText
            ? `only with CurseForge, ${needsText}`
            : "only with CurseForge"
          : klass === "no-build"
            ? `no ${flavorName(flavor)} build at its sources`
            : "no known source";
    const folders = [row.folder];
    const version = row.version || null;
    return {
      name: row.folder,
      folders,
      version,
      sources,
      pages,
      class: klass,
      adopt_as,
      needs,
      line: `${row.folder} ${version || "(no version)"}: ${verdict}`,
      verdict,
      details: [
        ...sources.map((source) => candidateLine(source, flavor)),
        ...pages.map((page) => `page ${page} from X-Website, not a source mrgl reads`),
      ],
    };
  }

  // The counts line of `Report::counts_line`.
  function countsLine(flavor, counts) {
    if (counts.addons === 0) return ["nothing", `no unmanaged addons in ${flavor.folder}: nothing to adopt`];
    let line = `${count(counts.addons, "unmanaged addon")} in ${flavor.folder}: ${counts.without_curseforge} can be adopted without CurseForge, ${counts.only_curseforge} only with CurseForge, ${counts.no_source} with no known source`;
    if (counts.no_build > 0) line += `, ${counts.no_build} with no ${flavorName(flavor)} build at their sources`;
    return ["success", line];
  }

  // The report of a flavor in the shape of `AdoptView`: its unmanaged rows (not linked), by name.
  function adoptView(flavor) {
    const rows = flavor.addons
      .filter((row) => !row.managed && !row.linked)
      .map((row) => adoptRow(row, flavor))
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    const counts = { addons: rows.length, without_curseforge: 0, only_curseforge: 0, no_build: 0, no_source: 0, adoptable_now: 0 };
    for (const row of rows) {
      counts[row.class.replace(/-/g, "_")] += 1;
      if (row.adopt_as) counts.adoptable_now += 1;
    }
    const [kind, line] = countsLine(flavor, counts);
    return {
      flavor: flavor.id,
      folder: flavor.folder,
      curse_client: flavor.curse_client,
      marker: flavor.curse_client
        ? say(
            "warning",
            `the CurseForge app manages ${flavor.folder} (its .curseclient marker is there): a folder belongs to one manager, so the Adopt button writes nothing there without the confirmation to adopt alongside the CurseForge app`
          )
        : null,
      rows,
      left_out: [],
      counts,
      summary: say(kind, counts.adoptable_now > 0 ? `${line}; the Adopt button adopts ${counts.adoptable_now} of them` : line),
      answers: answersOf(flavor, rows),
    };
  }

  // The answers a made-up report was built from, in the shape of `adopt::Answers` (task 113): the sketch
  // asks no source, so they are its made-up sources' answers.
  function answersOf(flavor, rows) {
    const offers = {};
    for (const row of rows) {
      for (const source of row.sources) offers[source.spec] = { build: source.build, why: source.why, missing: source.missing };
    }
    return { flavor: flavor.id, asked_at: new Date().toISOString().replace(/\.\d+Z$/, "Z"), offers, folders: {} };
  }

  function adoptAnswer(adopt, messages, extra) {
    return Object.assign({ adopt, flavor: null, adopted: [], messages }, extra || {});
  }

  // The made-up report: the core's lines, nothing written.
  function adopt(flavorId) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(adoptAnswer(null, [say("error", `no flavor ${flavorId}: no such flavor folder under ${ROOT}`)]));
    const view = adoptView(flavor);
    return Promise.resolve(adoptAnswer(view, [view.summary]));
  }

  // The made-up apply, with the guard and the lines of `adopt --apply`: each adoptable row becomes a
  // managed row of its source, no folder changes. The shown report's answers come back with the report after
  // it, as the window's do (task 113); the sketch asks no source either way.
  function adoptApply(flavorId, alongside, answers) {
    const flavor = flavors.find((item) => item.id === flavorId);
    if (!flavor) return Promise.resolve(adoptAnswer(null, [say("error", `no flavor ${flavorId}: no such flavor folder under ${ROOT}`)]));
    if (flavor.curse_client && !alongside) {
      return Promise.resolve(
        adoptAnswer(null, [
          say(
            "error",
            `the CurseForge app manages ${flavor.folder} (its .curseclient marker is there) and a folder belongs to one manager: the Adopt button wrote nothing; the confirmation to adopt alongside the CurseForge app adopts its addons anyway, and the CurseForge app may still update those folders`
          ),
        ])
      );
    }
    const report = adoptView(flavor);
    const adopted = [];
    const messages = [];
    if (flavor.curse_client) {
      messages.push(
        say(
          "warning",
          `adopting alongside the CurseForge app in ${flavor.folder}: it may still update or flag the adopted folders, and its .curseclient marker stays where it is`
        )
      );
    }
    for (const row of report.rows) {
      if (!row.adopt_as) {
        if (row.class === "without-curseforge" || row.class === "only-curseforge") messages.push(say("warning", `${row.name} not adopted: ${row.verdict}`));
        continue;
      }
      const addonRow = flavor.addons.find((item) => item.folder === row.name);
      const [kind, id] = [row.adopt_as.slice(0, row.adopt_as.indexOf(":")), row.adopt_as.slice(row.adopt_as.indexOf(":") + 1)];
      const version = row.version || "unknown";
      addonRow.source = { kind: kind === "curse" ? "curseforge" : kind, id };
      addonRow.managed = true;
      addonRow.guessed = false;
      addonRow.channel = "stable";
      addonRow.version = version;
      refresh(addonRow);
      adopted.push(`adopted ${row.name} ${version} as ${row.adopt_as} (1 folder: ${row.name})`);
    }
    const [, line] = countsLine(flavor, report.counts);
    messages.push(
      adopted.length
        ? say(
            "success",
            `${line}; adopted ${count(adopted.length, "addon")} into the sidecar, no download and no folder changed: the next update installs each from its source`
          )
        : say("nothing", `${line}; nothing adopted, the sidecar is unchanged`)
    );
    const after = adoptView(flavor);
    if (answers && answers.flavor === flavor.id) after.answers = answers;
    return Promise.resolve(adoptAnswer(after, messages, { flavor, adopted }));
  }

  return {
    version,
    flavors: flavors_,
    chooseRoot,
    diagnostics,
    resolve,
    checkUpdates,
    updateAll,
    updateOne,
    cancelUpdate,
    mark,
    preferences,
    setPreferences,
    toolbox,
    applySet,
    undoSet,
    editSet,
    cleanSavedVariables,
    adopt,
    adoptApply,
    settings,
  };
});
