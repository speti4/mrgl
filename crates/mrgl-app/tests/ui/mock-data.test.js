"use strict";
// The made-up data keeps the shape the view draws (the contract in the header of ui/mock-data.js) and
// covers every state and every source badge, so the sketch shows each of them at least once.
const test = require("node:test");
const assert = require("node:assert/strict");
const Data = require("../../ui/mock-data.js");
const State = require("../../ui/state.js");
const Voice = require("../../ui/voice.js");

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
  assert.equal(summary.plain, `${before} addons updated, 2 not checked`, "Plater (Wago) and the zip are not asked");
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
  const checked = classic.answer.messages[0];
  assert.equal(checked.kind, "success");
  assert.match(checked.plain, /^checked _classic_: 1 update available, .*1 pinned, 1 ignored/);
  const pinned = classic.answer.flavor.addons.find((row) => row.pinned);
  assert.equal(pinned.update, null, "a pinned row has no update");
  assert.equal(pinned.check.latest, "2.75");

  const unknown = await Data.updateAll("wow_mrgl", () => {});
  assert.equal(unknown.flavor, null);
  assert.equal(unknown.messages[0].kind, "error");
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
  assert.equal(one.answer.messages[0].plain, "1 addon updated");
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
