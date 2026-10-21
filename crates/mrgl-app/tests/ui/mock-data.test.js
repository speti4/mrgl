"use strict";
// The made-up data keeps the shape the view draws (the contract in the header of ui/mock-data.js) and
// covers every state and every source badge, so the sketch shows each of them at least once.
const test = require("node:test");
const assert = require("node:assert/strict");
const Data = require("../../ui/mock-data.js");
const State = require("../../ui/state.js");
const Voice = require("../../ui/voice.js");

Data.settings.delay = 0;

const ROW_KEYS = ["folder", "name", "version", "linked", "source", "managed", "guessed", "update", "pinned", "ignored", "forever"];
const FLAVOR_KEYS = ["id", "name", "folder", "version", "interface", "curse_client", "warnings", "addons"];

test("the answer carries a root, the four flavors and a banner", async () => {
  const answer = await Data.flavors();
  assert.equal(typeof answer.root, "string");
  assert.deepEqual(
    answer.flavors.map((flavor) => flavor.id).sort(),
    ["wow", "wow_classic", "wow_classic_era", "wowf"]
  );
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
