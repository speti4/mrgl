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
const ROW_KEYS = ["folder", "name", "version", "linked", "source", "managed", "guessed", "update", "pinned", "ignored", "forever", "missing"];
const FLAVOR_KEYS = ["id", "name", "folder", "version", "interface", "version_source", "version_file", "curse_client", "warnings", "addons"];

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

test("update all on Forever reports progress then success and flips the rows", async () => {
  const { flavors } = await Data.flavors();
  const forever = flavors.find((flavor) => flavor.id === "wowf");
  const before = State.countUpdates(forever.addons);
  assert.ok(before > 0);
  const heard = [];
  await Data.updateAll("wowf", (message) => heard.push(message));
  assert.deepEqual(heard.map((message) => message.kind), ["progress", "success"]);
  assert.equal(heard[0].plain, `updating ${before} addons in forever`);
  assert.match(heard[1].plain, new RegExp(`^${before} addons updated, \\d+ up to date$`));
  assert.ok(before > 1, "the plural case; the singular is the era flavor's one update");
  assert.equal(State.countUpdates(forever.addons), 0);
  for (const message of heard) assert.ok(Voice.lines(message).length === 2, "both lines");
});

test("the other flavors answer with the other kinds", async () => {
  const all = [];
  const kinds = async (id) => {
    const heard = [];
    await Data.updateAll(id, (message) => heard.push(message));
    all.push(...heard);
    return heard.map((message) => message.kind);
  };
  assert.deepEqual(await kinds("wow_classic_era"), ["progress", "error"]);
  assert.equal(all[0].plain, "updating 1 addon in era", "one addon, no plural");
  assert.deepEqual(await kinds("wow_classic"), ["progress", "warning"]);
  assert.deepEqual(await kinds("wow_classic"), ["warning"], "the second run has only the pinned one left to report");
  assert.deepEqual(await kinds("wow_mrgl"), ["error"]);
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
