"use strict";
// The toolbox panel of task 074: the target picker, the set the picker shows, the two buttons, the
// SavedVariables rows and the lines of what an apply changed.
const test = require("node:test");
const assert = require("node:assert/strict");
const Toolbox = require("../../ui/toolbox.js");

const say = (kind, plain) => ({ kind, murloc: "", plain });

const characters = [
  { name: "Mrgltest", realm: "Fixture Realm", label: "Fixture Realm/Mrgltest" },
  { name: "Grrlwave", realm: "Second Realm", label: "Second Realm/Grrlwave" },
];

const raid = {
  name: "raid",
  saved_at: "2026-10-22T10:00:00Z",
  enabled: 2,
  disabled: 1,
  addons: [
    { name: "QuestArrows", state: "disabled", installed: true },
    { name: "RaidFrames", state: "enabled", installed: true },
    { name: "BossTimers", state: "enabled", installed: false },
  ],
};

function file(addon, status, bytes, bak) {
  return { addon, place: "account-wide", realm: null, character: null, bytes, size: `${bytes} B`, lua: true, bak: Boolean(bak), status };
}

function toolbox(extra) {
  return Object.assign(
    {
      flavor: "wowf",
      folder: "_forever_",
      sets: [raid, Object.assign({}, raid, { name: "solo", addons: [] })],
      sets_note: say("success", "2 sets of _forever_"),
      characters,
      saved_variables: {
        files: [file("OldMeter", "orphan", 900, true), file("RaidFrames", "installed", 500), file("Blizzard_Console", "client", 20)],
        bytes: 1420,
        size: "1420 B",
        orphans: 1,
        orphan_bytes: 900,
        orphan_size: "900 B",
        summary: say("warning", "3 SavedVariables files"),
      },
      client: null,
    },
    extra || {}
  );
}

test("the target picker offers every character, then each by name and realm", () => {
  const options = Toolbox.targetOptions(characters);
  assert.deepEqual(
    options.map((option) => [option.value, option.label, option.character, option.realm]),
    [
      [Toolbox.EVERY, "Every character (2)", null, null],
      ["Fixture Realm/Mrgltest", "Mrgltest (Fixture Realm)", "Mrgltest", "Fixture Realm"],
      ["Second Realm/Grrlwave", "Grrlwave (Second Realm)", "Grrlwave", "Second Realm"],
    ]
  );
  assert.equal(Toolbox.targetOptions([]).length, 1);
  assert.equal(Toolbox.targetOptions(undefined)[0].label, "Every character (0)");
});

test("an option names its target, and a gone character falls back to every character", () => {
  assert.deepEqual(Toolbox.targetOf("Second Realm/Grrlwave", characters), { character: "Grrlwave", realm: "Second Realm" });
  assert.deepEqual(Toolbox.targetOf(Toolbox.EVERY, characters), { character: null, realm: null });
  assert.deepEqual(Toolbox.targetOf("Gone Realm/Nobody", characters), { character: null, realm: null });
});

test("the picker shows the set named, else the first, else none", () => {
  const sets = toolbox().sets;
  assert.equal(Toolbox.pickSet(sets, "solo").name, "solo");
  assert.equal(Toolbox.pickSet(sets, "gone").name, "raid");
  assert.equal(Toolbox.pickSet(sets, null).name, "raid");
  assert.equal(Toolbox.pickSet([], "raid"), null);
});

test("a set's addons list the enabled ones first and note a missing folder", () => {
  assert.deepEqual(
    Toolbox.setAddons(raid).map((addon) => [addon.name, addon.state, addon.note]),
    [
      ["BossTimers", "enabled", "not installed"],
      ["RaidFrames", "enabled", ""],
      ["QuestArrows", "disabled", ""],
    ]
  );
  assert.deepEqual(Toolbox.setAddons(null), []);
  assert.equal(raid.addons[0].name, "QuestArrows", "the set itself is left as it was");
});

test("the apply waits for a set, a character and the write going, never for the client", () => {
  const ready = Toolbox.applyButton(toolbox(), raid, false);
  assert.equal(ready.enabled, true);
  assert.equal(ready.label, "Apply raid");
  assert.match(ready.title, /3 addon lines/);
  const running = Toolbox.applyButton(toolbox({ client: say("warning", "the client is running") }), raid, false);
  assert.equal(running.enabled, true, "the core's guard answers a running client");
  assert.deepEqual(
    [
      Toolbox.applyButton(null, null, false),
      Toolbox.applyButton(toolbox(), raid, true),
      Toolbox.applyButton(toolbox({ sets: [] }), null, false),
      Toolbox.applyButton(toolbox({ characters: [] }), raid, false),
    ].map((button) => [button.enabled, button.title]),
    [
      [false, "the toolbox is not loaded"],
      [false, "a set apply or a cleanup is running"],
      [false, "no set saved yet: mrgl set save <name> saves one"],
      [false, "no character under WTF yet: log in with a character once"],
    ]
  );
});

test("the cleanup names the orphans and waits for an update and the write going", () => {
  const ready = Toolbox.cleanButton(toolbox(), false, false);
  assert.deepEqual([ready.enabled, ready.label], [true, "Move 1 orphan into a backup zip"]);
  assert.match(ready.title, /900 B/);
  const two = toolbox();
  two.saved_variables = Object.assign({}, two.saved_variables, { orphans: 2 });
  assert.equal(Toolbox.cleanButton(two, false, false).label, "Move 2 orphans into a backup zip");
  const none = toolbox();
  none.saved_variables = Object.assign({}, none.saved_variables, { orphans: 0 });
  assert.deepEqual(
    [
      Toolbox.cleanButton(null, false, false),
      Toolbox.cleanButton(toolbox(), true, false),
      Toolbox.cleanButton(toolbox(), false, true),
      Toolbox.cleanButton(none, false, false),
    ].map((button) => [button.enabled, button.label, button.title]),
    [
      [false, "No orphans to move", "the toolbox is not loaded"],
      [false, "Move 1 orphan into a backup zip", "a set apply or a cleanup is running"],
      [false, "Move 1 orphan into a backup zip", "an update is running; the cleanup waits until it ends"],
      [false, "No orphans to move", "every SavedVariables file has its addon folder"],
    ]
  );
});

test("the table shows every file or the orphans alone, in the listing's order", () => {
  assert.deepEqual(
    Toolbox.savedVariablesRows(toolbox(), false).map((row) => row.addon),
    ["OldMeter", "RaidFrames", "Blizzard_Console"]
  );
  assert.deepEqual(Toolbox.savedVariablesRows(toolbox(), true).map((row) => row.addon), ["OldMeter"]);
  assert.deepEqual(Toolbox.savedVariablesRows(null, false), []);
  assert.deepEqual(
    ["installed", "orphan", "client", "unknown"].map(Toolbox.statusLabel),
    ["installed", "orphan", "the client's own", "not compared"]
  );
  assert.deepEqual(
    [file("A", "orphan", 1, true), file("B", "orphan", 1), Object.assign(file("C", "orphan", 1, true), { lua: false })].map(
      Toolbox.filesOf
    ),
    [".lua + .bak", ".lua", ".bak"]
  );
});

test("what an apply changed reads as the command line's data lines", () => {
  const changes = [
    {
      character: characters[0],
      changed: [
        { name: "BossTimers", state: "disabled" },
        { name: "Missing", state: "enabled" },
      ],
      created: false,
    },
    { character: characters[1], changed: [], created: false },
    { character: { name: "New", realm: "R", label: "R/New" }, changed: [{ name: "A", state: "enabled" }], created: true },
  ];
  assert.deepEqual(Toolbox.changeLines(changes), [
    "Fixture Realm/Mrgltest: BossTimers disabled, Missing enabled",
    "Second Realm/Grrlwave: unchanged",
    "R/New: A enabled (a new AddOns.txt)",
  ]);
  assert.deepEqual(Toolbox.changeLines(undefined), []);
});
