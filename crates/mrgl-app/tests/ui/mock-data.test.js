"use strict";
// The made-up data keeps the shape the view draws (the contract in the header of ui/mock-data.js) and
// covers every state and every source badge, so the sketch shows each of them at least once.
const test = require("node:test");
const assert = require("node:assert/strict");
const Data = require("../../ui/mock-data.js");
const State = require("../../ui/state.js");
const Voice = require("../../ui/voice.js");
const Actions = require("../../ui/actions.js");

Data.settings.delay = 0;

// The keys of the contract (the header of ui/mock-data.js); `src/list.rs` serializes the same ones, which its
// tests hold it to.
const ROW_KEYS = ["folder", "name", "version", "linked", "source", "managed", "guessed", "update", "check", "channel", "pinned", "ignored", "forever", "missing", "interface_check"];
const FLAVOR_KEYS = ["id", "name", "folder", "version", "interface", "version_source", "version_file", "curse_client", "addons_folder", "warnings", "addons"];
const EVENT_KEYS = ["phase", "folder", "index", "total", "check", "message"];

test("the answer carries a root, the four flavors and a banner", async () => {
  const answer = await Data.flavors();
  assert.equal(typeof answer.root, "string");
  assert.deepEqual(
    answer.flavors.map((flavor) => flavor.id).sort(),
    ["wow", "wow_classic", "wow_classic_era", "wowf"]
  );
  assert.equal(answer.root_origin, "remembered");
  assert.equal(answer.message.kind, "banner");
  assert.equal(answer.message.murloc, "Mrglglgl.");
  assert.match(answer.message.plain, /^4 flavors under /);
  assert.equal(await Data.version(), "0.1.0");
});

test("every flavor and every row has every key of the contract", async () => {
  const { flavors } = await Data.flavors();
  for (const flavor of flavors) {
    assert.deepEqual(Object.keys(flavor).sort(), FLAVOR_KEYS.slice().sort(), flavor.id);
    for (const addon of flavor.addons) {
      assert.deepEqual(Object.keys(addon).sort(), ROW_KEYS.slice().sort(), `${flavor.id}/${addon.folder}`);
      if (addon.source) assert.ok(State.SOURCES.includes(addon.source.kind), addon.source.kind);
      if (addon.forever) assert.ok(["yes", "no", "unknown"].includes(addon.forever.build), addon.folder);
      assert.equal(addon.linked, Boolean(addon.source && addon.source.kind === "linked"), `${addon.folder}: linked rows carry the linked source`);
      assert.equal(addon.channel === null, !addon.managed, `${addon.folder}: a managed row has its channel`);
      assert.equal(addon.check === null, !addon.managed, `${addon.folder}: a managed row has the check's answer`);
      if (addon.update) assert.equal(addon.check.state, "update-available", addon.folder);
    }
  }
});

test("every state and every source badge shows up at least once", async () => {
  const { flavors } = await Data.flavors();
  const rows = flavors.flatMap((flavor) => flavor.addons);
  const states = new Set(rows.map(State.stateOf));
  for (const state of State.STATES) assert.ok(states.has(state), `no row in state ${state}`);
  const sources = new Set(rows.filter((row) => row.source).map((row) => row.source.kind));
  for (const kind of State.SOURCES) assert.ok(sources.has(kind), `no row from ${kind}`);
  assert.ok(rows.some((row) => row.pinned), "a pinned row");
  assert.ok(rows.some((row) => row.ignored), "an ignored row");
  const forever = flavors.find((flavor) => flavor.id === "wowf");
  assert.ok(forever.addons.every((row) => row.forever), "every Forever row has a Forever note slot filled");
  assert.ok(forever.addons.some((row) => row.forever.note), "a curated note");
});

test("only the Forever data carries Forever notes, so only its list shows the column", async () => {
  const { flavors } = await Data.flavors();
  for (const flavor of flavors) {
    assert.equal(State.hasForeverNotes(flavor.addons), flavor.id === "wowf", flavor.id);
  }
});

// A run with every event it sent.
async function runOf(call) {
  const events = [];
  const answer = await call((event) => events.push(event));
  return { answer, events };
}

test("update all on Forever reports each addon, then the summary, and flips the rows", async () => {
  const { flavors } = await Data.flavors();
  const forever = flavors.find((flavor) => flavor.id === "wowf");
  const before = State.countUpdates(forever.addons);
  assert.ok(before > 1, "the plural case");
  const { answer, events } = await runOf((progress) => Data.updateAll("wowf", progress));
  for (const event of events) assert.deepEqual(Object.keys(event).sort(), EVENT_KEYS.slice().sort());
  assert.ok(events.every((event) => event.message.kind === "progress"), "a run without a failure says progress only");
  const phases = new Set(events.map((event) => event.phase));
  for (const phase of ["checking", "checked", "updating", "downloading", "installing", "updated"]) assert.ok(phases.has(phase), phase);
  assert.equal(events.filter((event) => event.phase === "updated").length, before);
  const summary = answer.messages[answer.messages.length - 1];
  assert.equal(summary.kind, "success");
  assert.equal(summary.plain, `${before} addons updated, 1 up to date, 1 not checked`, "Plater (Wago) is asked, the zip is not");
  assert.equal(answer.flavor.id, "wowf");
  assert.equal(State.countUpdates(answer.flavor.addons), 0);
  for (const message of answer.messages) assert.ok(Voice.lines(message).length === 2, "both lines");
});

test("the other flavors answer with the other kinds", async () => {
  const era = await runOf((progress) => Data.updateAll("wow_classic_era", progress));
  const failed = era.events.filter((event) => event.phase === "failed");
  assert.equal(failed.length, 1);
  assert.equal(failed[0].message.kind, "error");
  const summary = era.answer.messages[era.answer.messages.length - 1];
  assert.equal(summary.kind, "warning");
  assert.match(summary.plain, /^nothing updated: 1 failed, /);
  assert.match(summary.plain, /1 newer than the release/);
  // Task 077: a pin above the current release reads as newer, in the data line and in the row's note.
  const held = era.answer.flavor.addons.find((row) => row.pinned);
  assert.deepEqual([held.check.state, held.check.order], ["pinned", "newer"]);
  assert.equal(held.check.line, "AtlasLoot Classic  v5.0.0-beta  pinned at v5.0.0-beta, newer than the current release v4.9.0, left alone");

  const classic = await runOf((progress) => Data.checkUpdates("wow_classic", progress));
  assert.ok(classic.events.every((event) => event.phase === "checking" || event.phase === "checked"));
  const checked = classic.answer.messages[classic.answer.messages.length - 1];
  assert.equal(checked.kind, "success");
  assert.match(checked.plain, /^checked _classic_: 2 updates available, .*1 pinned, 1 ignored/);
  const pinned = classic.answer.flavor.addons.find((row) => row.pinned);
  assert.equal(pinned.update, null, "a pinned row has no update");
  assert.equal(pinned.check.latest, "2.75");

  const unknown = await Data.updateAll("wow_mrgl", () => {});
  assert.equal(unknown.flavor, null);
  assert.equal(unknown.messages[0].kind, "error");
});

test("a run in a flavor the CurseForge app manages warns once, first, in the core's words (task 110)", async () => {
  const warning = (folder) =>
    `the CurseForge app manages ${folder} too (its .curseclient marker is there): it may update or replace what mrgl installs; a folder belongs to one manager`;
  const { flavors } = await Data.flavors();
  for (const [id, call] of [
    ["wow_classic", (progress) => Data.checkUpdates("wow_classic", progress)],
    ["wow", (progress) => Data.updateOne("wow", "Plater", progress)],
    ["wow_classic_era", (progress) => Data.updateAll("wow_classic_era", progress)],
  ]) {
    const flavor = flavors.find((item) => item.id === id);
    assert.equal(flavor.curse_client, true, id);
    const { answer, events } = await runOf(call);
    const warned = answer.messages.filter((message) => message.plain === warning(flavor.folder));
    assert.equal(warned.length, 1, id);
    assert.deepEqual([answer.messages[0].kind, answer.messages[0].plain], ["warning", warning(flavor.folder)], id);
    assert.ok(events.every((event) => !event.message.plain.includes(".curseclient")), `${id}: no event repeats it`);
  }
  const forever = await runOf((progress) => Data.checkUpdates("wowf", progress));
  assert.ok(forever.answer.messages.every((message) => !message.plain.includes(".curseclient")), "Forever has no marker");
});

test("the sketch's WoWInterface and Wago rows are asked, a source the build lacks says source not configured (tasks 102, 107)", async () => {
  const classic = await runOf((progress) => Data.checkUpdates("wow_classic", progress));
  const checked = (folder) => classic.events.find((event) => event.phase === "checked" && event.folder === folder).check;
  const asked = (folder) => classic.events.some((event) => event.phase === "checking" && event.folder === folder);
  const rows = classic.answer.flavor.addons;
  const row = (folder) => rows.find((item) => item.folder === folder);

  // WoWInterface (task 094): asked, its current release an update, the row's menu offers it.
  const wowi = checked("AtlasLootClassic");
  assert.ok(asked("AtlasLootClassic"));
  assert.deepEqual([wowi.state, wowi.latest, wowi.detail], ["update-available", "v4.2.1", null]);
  assert.equal(wowi.line, "AtlasLoot Classic  v4.2.0 -> v4.2.1  update available");
  assert.equal(Actions.whyNoUpdate(row("AtlasLootClassic")), null);

  // Wago (task 095, the key set): asked, up to date, nothing in the way of an update.
  const wago = checked("TomTom");
  assert.ok(asked("TomTom"));
  assert.deepEqual([wago.state, wago.latest, wago.detail], ["up-to-date", "v5.5.4-mists", null]);
  assert.equal(wago.line, "TomTom  v5.5.4-mists  up to date");
  assert.equal(Actions.whyNoUpdate(row("TomTom")), null);

  // CurseForge, which this build has no source for: not asked, the core's line word for word, and the
  // row's menu says it.
  const curse = checked("Leatrix_Plus");
  assert.ok(!asked("Leatrix_Plus"));
  const line = "source not configured: curse:289009 (this build has no CurseForge source)";
  assert.deepEqual([curse.state, curse.detail], ["not-checked", line]);
  assert.equal(curse.line, `Leatrix Plus  5.5.26  not checked: ${line}`);
  assert.equal(Actions.whyNoUpdate(row("Leatrix_Plus")), line);

  assert.ok(rows.every((row) => !row.check || !/does not handle|this build has no (WoWInterface|Wago)/.test(row.check.line)), "no line of the old sketch");
});

test("a cancel lets the addon in flight finish and starts no other", async () => {
  const { answer, events } = await runOf((progress) =>
    Data.updateAll("wow", (event) => {
      progress(event);
      if (event.phase === "downloading") Data.cancelUpdate();
    })
  );
  assert.deepEqual(
    events.filter((event) => ["updating", "updated"].includes(event.phase)).map((event) => [event.phase, event.folder]),
    [["updating", "BigWigs"], ["updated", "BigWigs"]]
  );
  const summary = answer.messages[answer.messages.length - 1];
  assert.equal(summary.kind, "warning");
  assert.match(summary.plain, /^1 addon updated, 1 not updated \(cancelled\), /);
  const nothing = await Data.cancelUpdate();
  assert.equal(nothing.kind, "nothing", "no run, nothing to cancel");
});

test("a row update and the marks answer the flavor as it is now", async () => {
  const one = await runOf((progress) => Data.updateOne("wow", "WeakAuras", progress));
  assert.ok(one.events.every((event) => event.folder === "WeakAuras"));
  assert.equal(one.answer.messages[one.answer.messages.length - 1].plain, "1 addon updated");
  const pin = await Data.mark("wowf", "Questie", "pin");
  assert.deepEqual([pin.messages[0].kind, pin.messages[0].plain], ["success", "pinned Questie at 11.3.0 in _forever_"]);
  assert.equal(pin.flavor.addons.find((row) => row.folder === "Questie").pinned, "11.3.0");
  assert.equal((await Data.mark("wowf", "Questie", "pin")).messages[0].kind, "nothing");
  assert.equal((await Data.mark("wowf", "Questie", "unpin")).messages[0].plain, "unpinned Questie in _forever_");
  const channel = await Data.mark("wowf", "Questie", "channel", "prerelease");
  assert.equal(channel.messages[0].plain, "Questie follows the prerelease channel in _forever_");
  assert.equal(channel.flavor.addons.find((row) => row.folder === "Questie").channel, "prerelease");
  assert.equal((await Data.mark("wowf", "RaidLogger", "ignore")).messages[0].kind, "error");
  assert.equal((await Data.mark("wowf", "Questie", "delete")).messages[0].kind, "error");
});

test("the other calls of the contract answer in the voice's shape", async () => {
  const choice = await Data.chooseRoot();
  assert.equal(choice.flavors, null);
  assert.equal(choice.message.kind, "warning");
  const report = await Data.diagnostics();
  assert.match(report.text, /^mrgl: /);
  assert.equal(report.copied.kind, "success");
  assert.equal(report.failed.kind, "error");
  const resolved = await Data.resolve("wowf", "BigWigs");
  assert.equal(resolved.pick.version, "v401.0");
  const refused = await Data.resolve("wowf", "RaidLogger");
  assert.equal(refused.pick, null);
  assert.equal(refused.message.kind, "error");
});

test("one row of each interface state, in the core's words for the flavor's build (task 081)", async () => {
  const { flavors } = await Data.flavors();
  const marked = flavors.flatMap((flavor) => flavor.addons.filter((row) => row.interface_check).map((row) => ({ flavor, row })));
  for (const state of State.INTERFACE_STATES) assert.ok(marked.some(({ row }) => row.interface_check.state === state), `no row ${state}`);
  for (const { flavor, row } of marked) {
    const mark = State.interfaceMark(row);
    assert.deepEqual(Object.keys(row.interface_check).sort(), ["label", "line", "state"], row.folder);
    assert.equal(mark.label, State.interfaceLabelOf(row.interface_check.state), row.folder);
    assert.ok(mark.title.startsWith(`${row.folder}: interface `), mark.title);
    assert.ok(mark.title.includes(`for the build ${flavor.version}`), mark.title);
  }
});

test("a row names its missing dependencies and a stale note", async () => {
  const { flavors } = await Data.flavors();
  const rows = flavors.flatMap((flavor) => flavor.addons);
  assert.ok(rows.every((row) => Array.isArray(row.missing)));
  assert.ok(rows.some((row) => row.missing.length > 0), "a row with a missing dependency");
  assert.ok(rows.some((row) => row.forever && row.forever.stale), "a stale Forever note");
});

// The keys of a toolbox answer (task 074); `tests/toolbox.rs` holds `src/toolbox.rs` to the same ones.
const TOOLBOX_KEYS = ["characters", "client", "flavor", "folder", "saved_variables", "sets", "sets_note", "undo"];
const SAVED_KEYS = ["bytes", "files", "orphan_bytes", "orphan_size", "orphans", "size", "summary"];
const FILE_KEYS = ["addon", "bak", "bytes", "character", "lua", "place", "realm", "size", "status"];

test("the toolbox of every flavor has every key of the contract", async () => {
  const { flavors } = await Data.flavors();
  for (const flavor of flavors) {
    const answer = await Data.toolbox(flavor.id);
    assert.deepEqual(Object.keys(answer).sort(), ["changes", "messages", "toolbox", "undone", "zip"]);
    const toolbox = answer.toolbox;
    assert.deepEqual(Object.keys(toolbox).sort(), TOOLBOX_KEYS, flavor.id);
    assert.deepEqual(Object.keys(toolbox.saved_variables).sort(), SAVED_KEYS);
    for (const file of toolbox.saved_variables.files) assert.deepEqual(Object.keys(file).sort(), FILE_KEYS);
    for (const set of toolbox.sets) {
      assert.deepEqual(Object.keys(set).sort(), ["addons", "disabled", "enabled", "name", "saved_at"]);
      for (const addon of set.addons) assert.deepEqual(Object.keys(addon).sort(), ["installed", "name", "state"]);
    }
    for (const character of toolbox.characters) assert.deepEqual(Object.keys(character).sort(), ["account", "label", "name", "realm"]);
    assert.equal(toolbox.saved_variables.orphans, toolbox.saved_variables.files.filter((file) => file.status === "orphan").length);
    assert.equal(Boolean(toolbox.client), flavor.id === "wow", "retail's client runs in the sketch");
  }
  const unknown = await Data.toolbox("wow_mrgl");
  assert.equal(unknown.toolbox, null);
  assert.equal(unknown.messages[0].kind, "error");
});

test("a set goes to one character and to every one, and the orphans into the zip", async () => {
  const one = await Data.applySet("wowf", "raid", { character: "Grrlwave", realm: "Second Realm" });
  assert.deepEqual(
    one.messages.map((message) => [message.kind, message.plain]),
    [["success", "applied set raid to Second Realm/Grrlwave of _forever_: 2 addon lines changed"]]
  );
  assert.deepEqual(one.changes.map((change) => [change.character.label, change.changed.length]), [["Second Realm/Grrlwave", 2]]);
  const every = await Data.applySet("wowf", "raid", { character: null, realm: null });
  assert.equal(every.messages[0].plain, "applied set raid to 3 characters of _forever_: 2 addon lines changed");
  const again = await Data.applySet("wowf", "raid");
  assert.deepEqual(
    [again.messages[0].kind, again.messages[0].plain],
    ["nothing", "set raid already matches every character of _forever_ (3 characters), nothing written"]
  );
  const levelling = await Data.applySet("wowf", "levelling");
  assert.deepEqual(levelling.messages.map((message) => message.kind), ["warning", "success"]);
  assert.equal(
    levelling.messages[0].plain,
    "set levelling names addons that are not installed in _forever_, their lines are written all the same: QuestHelperOld"
  );
  assert.equal((await Data.toolbox("wowf")).toolbox.sets_note.plain, "2 sets of _forever_ in %LOCALAPPDATA%\\mrgl\\wowf\\sets.json");
  assert.equal((await Data.applySet("wowf", "gone")).messages[0].kind, "error");
  assert.equal((await Data.applySet("wowf", "raid", { character: "Nobody" })).messages[0].kind, "error");

  const listed = (await Data.toolbox("wowf")).toolbox.saved_variables.summary;
  assert.equal(listed.kind, "warning");
  assert.match(listed.plain, /; 2 orphans \(2\.5 MB\) have no addon folder: the cleanup moves them into a backup zip$/);
  const clean = await Data.cleanSavedVariables("wowf");
  assert.equal(clean.messages[0].kind, "success");
  assert.match(clean.messages[0].plain, /^moved 2 orphans of _forever_ \(3 files, 2\.5 MB\) into .*SavedVariables-2026-10-22\.zip; /);
  assert.match(clean.zip, /SavedVariables-2026-10-22\.zip$/);
  assert.equal(clean.toolbox.saved_variables.orphans, 0);
  assert.equal(clean.toolbox.saved_variables.summary.kind, "success");
  const nothing = await Data.cleanSavedVariables("wowf");
  assert.deepEqual(
    [nothing.messages[0].kind, nothing.messages[0].plain, nothing.zip],
    ["nothing", "no orphans in the SavedVariables of _forever_, nothing moved", null]
  );
});

test("both toolbox writes are refused while the sketch's client runs", async () => {
  for (const answer of [await Data.applySet("wow", "raid"), await Data.cleanSavedVariables("wow")]) {
    assert.equal(answer.messages.length, 1);
    assert.equal(answer.messages[0].kind, "error");
    assert.match(answer.messages[0].plain, /^the client is running in _retail_ \(Wow\.exe, pid \d+\): close the game first/);
    assert.equal(answer.toolbox.saved_variables.orphans, 2, "nothing moved");
  }
});

test("the sketch's two accounts, the undo of an apply and the edit of a set (task 085)", async () => {
  const loaded = (await Data.toolbox("wow_classic")).toolbox;
  assert.deepEqual(
    loaded.characters.map((character) => [character.label, character.account]),
    [
      ["Made-up Realm/Mrglina", "MOCKACCOUNT"],
      ["Second Realm/Grrlwave", null],
      ["Made-up Realm/Mrglina", "MOCKTWIN"],
    ]
  );
  assert.equal(loaded.undo, null);
  const ambiguous = await Data.applySet("wow_classic", "levelling", { character: "Mrglina", realm: "Made-up Realm" });
  assert.equal(
    ambiguous.messages[0].plain,
    "character Mrglina matches 2 characters (flavor _classic_): Made-up Realm (MOCKACCOUNT), Made-up Realm (MOCKTWIN); the same realm is under more than one account: pick the account too"
  );

  const twin = await Data.applySet("wow_classic", "levelling", {
    character: "Mrglina",
    realm: "Made-up Realm",
    account: "MOCKTWIN",
  });
  assert.equal(twin.messages.at(-1).kind, "success");
  assert.deepEqual(twin.changes.map((change) => [change.character.label, change.character.account]), [
    ["Made-up Realm/Mrglina", "MOCKTWIN"],
  ]);
  const undo = twin.toolbox.undo;
  assert.equal(undo.set, "levelling");
  assert.deepEqual(undo.restore.map((character) => character.account), ["MOCKTWIN"]);

  const undone = await Data.undoSet("wow_classic");
  assert.match(undone.messages[0].plain, /^undid the apply of set levelling to _classic_ \(.+\): 1 AddOns\.txt put back$/);
  assert.deepEqual(undone.undone, undo);
  assert.equal(undone.toolbox.undo, null);
  const again = await Data.undoSet("wow_classic");
  assert.match(again.messages[0].plain, /^nothing to undo in _classic_: /);

  const raid = loaded.sets.find((set) => set.name === "raid");
  const first = raid.addons.find((addon) => addon.state === "enabled");
  const disabled = await Data.editSet("wow_classic", "raid", first.name, "disable");
  assert.match(disabled.messages[0].plain, new RegExp(`^${first.name} is disabled in set raid of _classic_ now, was enabled: `));
  const after = disabled.toolbox.sets.find((set) => set.name === "raid");
  assert.deepEqual(
    after.addons.filter((addon) => addon.name !== first.name),
    raid.addons.filter((addon) => addon.name !== first.name),
    "only the named addon"
  );
  assert.equal(after.addons.find((addon) => addon.name === first.name).state, "disabled");
  const dropped = await Data.editSet("wow_classic", "raid", first.name, "drop");
  assert.match(dropped.messages[0].plain, /^dropped .+ from set raid of _classic_: .+; no AddOns\.txt was touched$/);
  assert.equal((await Data.editSet("wow_classic", "raid", first.name, "drop")).messages[0].kind, "error");
});

test("the sketch's running client refuses the undo, not an edit", async () => {
  const undo = await Data.undoSet("wow");
  assert.match(undo.messages[0].plain, /^the client is running in _retail_/);
  const edit = await Data.editSet("wow", "raid", "QuestHelperOld", "enable");
  assert.deepEqual(
    edit.messages.map((message) => message.kind),
    ["warning", "success"]
  );
  assert.match(edit.messages[0].plain, /^QuestHelperOld is not installed in _retail_/);
});

test("the mock keeps the preferences for the page's life", async () => {
  const before = await Data.preferences();
  assert.deepEqual(before, { theme: "auto", serious: false, serious_source: "default", stored: { theme: null, serious: null } });
  const after = await Data.setPreferences({ theme: "dark", serious: true });
  assert.equal(after.theme, "dark");
  assert.equal(after.serious, true);
  assert.equal(after.serious_source, "file");
  assert.deepEqual((await Data.preferences()).stored, { theme: "dark", serious: true });
});

// The adopt answer of the contract (task 109); `src/toolbox.rs` serializes the same keys, which
// `tests/adopt.rs` holds it to. These tests come last: an apply turns rows into managed ones.
const ADOPT_KEYS = ["adopt", "adopted", "flavor", "messages"];
const ADOPT_VIEW_KEYS = ["answers", "counts", "curse_client", "flavor", "folder", "left_out", "marker", "rows", "summary"];
const ADOPT_ANSWERS_KEYS = ["asked_at", "flavor", "folders", "offers"];
const ADOPT_ROW_KEYS = ["adopt_as", "class", "details", "folders", "line", "name", "needs", "pages", "sources", "verdict", "version"];
const ADOPT_SOURCE_KEYS = ["build", "from", "missing", "needs", "source", "spec", "taken_by", "why"];
const sorted = (object) => Object.keys(object).sort();

test("the adopt report keeps the contract's keys, every class and the core's lines (task 109)", async () => {
  const classes = new Set();
  for (const id of ["wow", "wow_classic", "wow_classic_era", "wowf"]) {
    const answer = await Data.adopt(id);
    assert.deepEqual(sorted(answer), ADOPT_KEYS, id);
    assert.deepEqual(sorted(answer.adopt), ADOPT_VIEW_KEYS, id);
    assert.deepEqual(sorted(answer.adopt.answers), ADOPT_ANSWERS_KEYS, id);
    assert.deepEqual(answer.messages, [answer.adopt.summary], `${id}: the summary, said`);
    const { rows, counts } = answer.adopt;
    assert.equal(counts.addons, rows.length, id);
    assert.equal(counts.without_curseforge + counts.only_curseforge + counts.no_build + counts.no_source, rows.length, id);
    for (const row of rows) {
      assert.deepEqual(sorted(row), ADOPT_ROW_KEYS, `${id}/${row.name}`);
      for (const source of row.sources) assert.deepEqual(sorted(source), ADOPT_SOURCE_KEYS, `${id}/${row.name}`);
      assert.equal(row.details.length, row.sources.length + row.pages.length, `${id}/${row.name}`);
      classes.add(row.class);
    }
    assert.equal(Boolean(answer.adopt.marker), answer.adopt.curse_client, id);
  }
  assert.deepEqual([...classes].sort(), ["no-build", "no-source", "only-curseforge", "without-curseforge"]);

  const forever = (await Data.adopt("wowf")).adopt;
  assert.deepEqual(
    forever.rows.map((row) => row.line),
    [
      "Details #Details.20261010.14521.165: adopt as github:Tercioo/Details-Damage-Meter",
      "Leatrix_Plus 5.5.26: only with CurseForge, needs MRGL_CURSEFORGE_KEY",
      "RaidLogger 0.9: no known source",
    ]
  );
  assert.deepEqual(forever.rows[0].details, [
    "curse:61284 from X-Curse-Project-ID: needs MRGL_CURSEFORGE_KEY",
    "github:Tercioo/Details-Damage-Meter from the GitHub mapping: forever build yes (release.json names forever)",
  ]);
  assert.equal(
    forever.summary.plain,
    "3 unmanaged addons in _forever_: 1 can be adopted without CurseForge, 1 only with CurseForge, 1 with no known source; the Adopt button adopts 1 of them"
  );
  // The sources' answers the report was built from (task 113), for the apply to send back.
  assert.equal(forever.answers.flavor, "wowf");
  assert.equal(forever.answers.offers["github:Tercioo/Details-Damage-Meter"].build, "yes");
  const retail = (await Data.adopt("wow")).adopt;
  assert.equal(retail.marker.kind, "warning");
  assert.equal(
    retail.marker.plain,
    "the CurseForge app manages _retail_ (its .curseclient marker is there): a folder belongs to one manager, so the Adopt button writes nothing there without the confirmation to adopt alongside the CurseForge app"
  );
  // The window's words (task 112): the card's lines name no flag of the command line.
  for (const line of [forever.summary.plain, retail.summary.plain, retail.marker.plain]) assert.doesNotMatch(line, /--/);
  assert.equal(retail.rows[0].verdict, "needs MRGL_WAGO_KEY or MRGL_CURSEFORGE_KEY", "DBM-Core: either key would let it");
  assert.equal((await Data.adopt("wow_mrgl")).messages[0].kind, "error");
});

test("the adopt apply refuses a marked folder without the confirmation and warns with it (task 109)", async () => {
  const refused = await Data.adoptApply("wow_classic_era", false);
  assert.equal(refused.adopt, null);
  assert.deepEqual(refused.adopted, []);
  assert.equal(refused.messages.length, 1);
  assert.equal(refused.messages[0].kind, "error");
  assert.match(refused.messages[0].plain, /^the CurseForge app manages _classic_era_ .*: the Adopt button wrote nothing; the confirmation to adopt alongside the CurseForge app adopts /);
  const { flavors } = await Data.flavors();
  const bartender = () => flavors.find((flavor) => flavor.id === "wow_classic_era").addons.find((row) => row.folder === "Bartender4");
  assert.equal(bartender().managed, false, "nothing written");

  const shown = (await Data.adopt("wow_classic_era")).adopt.answers;
  const era = await Data.adoptApply("wow_classic_era", true, shown);
  assert.deepEqual(era.adopt.answers, shown, "the shown report's answers come back (task 113)");
  assert.deepEqual(era.adopted, ["adopted Bartender4 4.15.7 as wowi:11190 (1 folder: Bartender4)"]);
  assert.deepEqual(
    era.messages.map((message) => [message.kind, message.plain]),
    [
      [
        "warning",
        "adopting alongside the CurseForge app in _classic_era_: it may still update or flag the adopted folders, and its .curseclient marker stays where it is",
      ],
      [
        "success",
        "2 unmanaged addons in _classic_era_: 1 can be adopted without CurseForge, 0 only with CurseForge, 1 with no known source; adopted 1 addon into the sidecar, no download and no folder changed: the next update installs each from its source",
      ],
    ]
  );
  assert.equal(era.flavor.id, "wow_classic_era");
  assert.deepEqual([bartender().managed, bartender().guessed, bartender().source], [true, false, { kind: "wowi", id: "11190" }]);
  assert.deepEqual(era.adopt.rows.map((row) => row.name), ["RaidLogger"]);

  const again = await Data.adoptApply("wow_classic_era", true);
  assert.deepEqual(again.adopted, []);
  const last = again.messages[again.messages.length - 1];
  assert.equal(last.kind, "nothing");
  assert.match(last.plain, /; nothing adopted, the sidecar is unchanged$/);

  const retail = await Data.adoptApply("wow", true);
  assert.ok(retail.messages.some((message) => message.kind === "warning" && message.plain === "DBM-Core not adopted: needs MRGL_WAGO_KEY or MRGL_CURSEFORGE_KEY"));
  const forever = await Data.adoptApply("wowf", false);
  assert.deepEqual(forever.adopted, ["adopted Details #Details.20261010.14521.165 as github:Tercioo/Details-Damage-Meter (1 folder: Details)"]);
  assert.ok(forever.messages.every((message) => !message.plain.includes(".curseclient")), "Forever has no marker");
});
