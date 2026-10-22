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
const ROW_KEYS = ["folder", "name", "version", "linked", "source", "managed", "guessed", "update", "check", "channel", "pinned", "ignored", "forever", "missing"];
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

test("a row names its missing dependencies and a stale note", async () => {
  const { flavors } = await Data.flavors();
  const rows = flavors.flatMap((flavor) => flavor.addons);
  assert.ok(rows.every((row) => Array.isArray(row.missing)));
  assert.ok(rows.some((row) => row.missing.length > 0), "a row with a missing dependency");
  assert.ok(rows.some((row) => row.forever && row.forever.stale), "a stale Forever note");
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
