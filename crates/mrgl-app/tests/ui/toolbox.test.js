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
  assert.deepEqual(Toolbox.targetOf("Second Realm/Grrlwave", characters), { character: "Grrlwave", realm: "Second Realm", account: null });
  assert.deepEqual(Toolbox.targetOf(Toolbox.EVERY, characters), { character: null, realm: null, account: null });
  assert.deepEqual(Toolbox.targetOf("Gone Realm/Nobody", characters), { character: null, realm: null, account: null });
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
      [false, Toolbox.BUSY],
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
      [false, "Move 1 orphan into a backup zip", Toolbox.BUSY],
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

// Task 085: the account of a realm and name under two accounts, the undo and the chips' edit controls.

const twins = [
  { name: "Mrgltest", realm: "Fixture Realm", label: "Fixture Realm/Mrgltest", account: "MRGLSETS" },
  { name: "Grrlwave", realm: "Second Realm", label: "Second Realm/Grrlwave", account: null },
  { name: "Mrgltest", realm: "Fixture Realm", label: "Fixture Realm/Mrgltest", account: "MRGLTWIN" },
];

test("two characters of one realm and name are told apart by their account", () => {
  const options = Toolbox.targetOptions(twins);
  assert.deepEqual(
    options.map((option) => [option.value, option.label, option.account]),
    [
      [Toolbox.EVERY, "Every character (3)", null],
      ["MRGLSETS/Fixture Realm/Mrgltest", "Mrgltest (Fixture Realm, account MRGLSETS)", "MRGLSETS"],
      ["Second Realm/Grrlwave", "Grrlwave (Second Realm)", null],
      ["MRGLTWIN/Fixture Realm/Mrgltest", "Mrgltest (Fixture Realm, account MRGLTWIN)", "MRGLTWIN"],
    ]
  );
  assert.equal(new Set(options.map((option) => option.value)).size, options.length, "every value is its own");
  assert.deepEqual(Toolbox.targetOf("MRGLTWIN/Fixture Realm/Mrgltest", twins), {
    character: "Mrgltest",
    realm: "Fixture Realm",
    account: "MRGLTWIN",
  });
  assert.equal(Toolbox.characterName(twins[2]), "Fixture Realm/Mrgltest (account MRGLTWIN)");
  assert.equal(Toolbox.characterName(twins[1]), "Second Realm/Grrlwave");
});

const undo = {
  set: "solo",
  applied_at: "2026-10-22T10:00:00Z",
  restore: [characters[0], twins[2]],
  remove: [characters[1]],
};

test("the undo names what it puts back and waits for a record and the write going", () => {
  assert.equal(
    Toolbox.undoNote(undo),
    "the undo of set solo (applied 2026-10-22T10:00:00Z) puts back the AddOns.txt of Fixture Realm/Mrgltest, Fixture Realm/Mrgltest (account MRGLTWIN) as it was and removes the AddOns.txt it created for Second Realm/Grrlwave"
  );
  assert.equal(
    Toolbox.undoNote(Object.assign({}, undo, { remove: [] })),
    "the undo of set solo (applied 2026-10-22T10:00:00Z) puts back the AddOns.txt of Fixture Realm/Mrgltest, Fixture Realm/Mrgltest (account MRGLTWIN) as it was"
  );
  assert.equal(Toolbox.undoNote(null), "");
  const ready = Toolbox.undoButton(toolbox({ undo }), false);
  assert.deepEqual([ready.enabled, ready.label, ready.title], [true, "Undo solo", Toolbox.undoNote(undo)]);
  const running = Toolbox.undoButton(toolbox({ undo, client: say("warning", "the client is running") }), false);
  assert.equal(running.enabled, true, "the core's guard answers a running client");
  assert.deepEqual(
    [Toolbox.undoButton(null, false), Toolbox.undoButton(toolbox({ undo }), true), Toolbox.undoButton(toolbox({ undo: null }), false)].map(
      (button) => [button.enabled, button.label, button.title]
    ),
    [
      [false, "Undo", "the toolbox is not loaded"],
      [false, "Undo solo", Toolbox.BUSY],
      [false, "Undo", "nothing to undo: an apply that changes a file keeps what it replaced, and an undo puts it back once"],
    ]
  );
});

test("what an undo put back reads as the command line's data lines", () => {
  assert.deepEqual(Toolbox.undoneLines(undo), [
    "Fixture Realm/Mrgltest: AddOns.txt put back as it was",
    "Fixture Realm/Mrgltest (account MRGLTWIN): AddOns.txt put back as it was",
    "Second Realm/Grrlwave: AddOns.txt removed, the apply had created it",
  ]);
  assert.deepEqual(Toolbox.undoneLines(null), []);
  const changes = [{ character: twins[2], changed: [{ name: "A", state: "enabled" }], created: false }];
  assert.deepEqual(Toolbox.changeLines(changes), ["Fixture Realm/Mrgltest (account MRGLTWIN): A enabled"]);
});

test("a chip toggles its addon and drops it, and waits only for the write going", () => {
  const [questArrows, raidFrames, bossTimers] = raid.addons;
  const disabled = Toolbox.chipActions(raid, questArrows, false);
  assert.deepEqual(
    [disabled.toggle.action, disabled.toggle.label, disabled.toggle.enabled],
    ["enable", "QuestArrows", true]
  );
  assert.equal(disabled.toggle.title, "disabled; click to enable it in set raid, no AddOns.txt is touched");
  assert.deepEqual(
    [disabled.drop.action, disabled.drop.label, disabled.drop.title],
    ["drop", "Drop QuestArrows from set raid", "drop QuestArrows from set raid: an apply then leaves its lines alone"]
  );
  assert.equal(Toolbox.chipActions(raid, raidFrames, false).toggle.action, "disable");
  assert.equal(
    Toolbox.chipActions(raid, bossTimers, false).toggle.title,
    "enabled (not installed; an apply writes its line all the same); click to disable it in set raid, no AddOns.txt is touched"
  );
  const busy = Toolbox.chipActions(raid, raidFrames, true);
  assert.deepEqual(
    [busy.toggle.enabled, busy.toggle.title, busy.drop.enabled, busy.drop.title],
    [false, Toolbox.BUSY, false, Toolbox.BUSY]
  );
});
