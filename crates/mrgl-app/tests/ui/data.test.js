"use strict";
// The data module of the window: each call of the contract invokes its Tauri command with its arguments
// and hands the answer through as it is; an update run's events come back through a channel (task 010).
const test = require("node:test");
const assert = require("node:assert/strict");
const { create } = require("../../ui/data.js");

function fake(answers) {
  const calls = [];
  const invoke = (command, args) => {
    calls.push(args === undefined ? [command] : [command, args]);
    return Promise.resolve(answers[command]);
  };
  return { calls, invoke };
}

test("every call invokes its command and passes the answer through", async () => {
  const flavors = { root: "R", root_origin: "remembered", flavors: [], message: { kind: "nothing", murloc: "", plain: "x" } };
  const { calls, invoke } = fake({
    app_version: "0.1.0",
    flavors,
    choose_root: { flavors: null, message: null },
    diagnostics: { text: "mrgl: 0.1.0", copied: {}, failed: {} },
    resolve: { pick: null, message: { kind: "error", murloc: "", plain: "no" } },
  });
  const Data = create(invoke);
  assert.equal(await Data.version(), "0.1.0");
  assert.equal(await Data.flavors(), flavors);
  assert.deepEqual(await Data.chooseRoot(), { flavors: null, message: null });
  assert.equal((await Data.diagnostics()).text, "mrgl: 0.1.0");
  assert.equal((await Data.resolve("wowf", "Questie")).message.kind, "error");
  assert.deepEqual(calls, [
    ["app_version"],
    ["flavors"],
    ["choose_root"],
    ["diagnostics"],
    ["resolve", { flavor: "wowf", folder: "Questie" }],
  ]);
});

// A stand-in for Tauri's `Channel`: the test plays the command's side and sends on it.
class FakeChannel {
  constructor() {
    this.onmessage = null;
  }
  send(event) {
    this.onmessage(event);
  }
}

test("the update calls hand a channel to their command and the events reach the progress", async () => {
  const answer = { flavor: null, messages: [{ kind: "success", murloc: "", plain: "1 addon updated" }] };
  const calls = [];
  const invoke = (command, args) => {
    calls.push([command, args]);
    if (args && args.progress) {
      args.progress.send({ phase: "checking", folder: "Questie", message: { kind: "progress", murloc: "", plain: command } });
    }
    return Promise.resolve(answer);
  };
  const Data = create(invoke, FakeChannel);
  const heard = [];
  const progress = (event) => heard.push(event.message.plain);
  assert.equal(await Data.checkUpdates("wowf", progress), answer);
  assert.equal(await Data.updateAll("wowf", progress), answer);
  assert.equal(await Data.updateOne("wowf", "Questie", progress), answer);
  await Data.updateAll("wowf");
  assert.deepEqual(heard, ["check_updates", "update_all", "update_one"], "no progress, no call");
  assert.deepEqual(
    calls.map(([command, args]) => [command, args.flavor, args.folder, args.progress instanceof FakeChannel]),
    [
      ["check_updates", "wowf", undefined, true],
      ["update_all", "wowf", undefined, true],
      ["update_one", "wowf", "Questie", true],
      ["update_all", "wowf", undefined, true],
    ]
  );
});

test("cancel and the marks go to their commands, a missing value as null", async () => {
  const { calls, invoke } = fake({ cancel_update: { kind: "progress", murloc: "", plain: "cancelling" }, mark: { flavor: null, messages: [] } });
  const Data = create(invoke, FakeChannel);
  assert.equal((await Data.cancelUpdate()).plain, "cancelling");
  await Data.mark("wowf", "Questie", "pin");
  await Data.mark("wowf", "Questie", "channel", "prerelease");
  assert.deepEqual(calls, [
    ["cancel_update"],
    ["mark", { flavor: "wowf", folder: "Questie", action: "pin", value: null }],
    ["mark", { flavor: "wowf", folder: "Questie", action: "channel", value: "prerelease" }],
  ]);
});

test("the preferences go to their commands, a missing change as null", async () => {
  const answer = { theme: "dark", serious: true, serious_source: "file", stored: { theme: "dark", serious: true } };
  const { calls, invoke } = fake({ preferences: answer, set_preferences: answer });
  const Data = create(invoke);
  assert.equal(await Data.preferences(), answer);
  await Data.setPreferences({ theme: "dark" });
  await Data.setPreferences({ serious: false });
  assert.deepEqual(calls, [
    ["preferences"],
    ["set_preferences", { theme: "dark", serious: null }],
    ["set_preferences", { theme: null, serious: false }],
  ]);
});

test("the toolbox calls go to their commands, every character as a null target", async () => {
  const answer = { toolbox: null, changes: [], undone: null, zip: null, messages: [] };
  const { calls, invoke } = fake({
    toolbox: answer,
    apply_set: answer,
    undo_set: answer,
    edit_set: answer,
    clean_saved_variables: answer,
  });
  const Data = create(invoke);
  assert.equal(await Data.toolbox("wowf"), answer);
  await Data.applySet("wowf", "raid");
  await Data.applySet("wowf", "raid", { character: null, realm: null, account: "MRGLTWIN" });
  await Data.applySet("wowf", "raid", { character: "Grrlwave", realm: "Second Realm" });
  await Data.applySet("wowf", "raid", { character: "Grrlwave" });
  await Data.applySet("wowf", "raid", { character: "Mrglina", realm: "Made-up Realm", account: "MRGLTWIN" });
  assert.equal(await Data.undoSet("wowf"), answer);
  assert.equal(await Data.editSet("wowf", "raid", "RaidFrames", "disable"), answer);
  assert.equal(await Data.cleanSavedVariables("wowf"), answer);
  const none = { character: null, realm: null, account: null };
  assert.deepEqual(calls, [
    ["toolbox", { flavor: "wowf" }],
    ["apply_set", Object.assign({ flavor: "wowf", set: "raid" }, none)],
    ["apply_set", Object.assign({ flavor: "wowf", set: "raid" }, none)],
    ["apply_set", { flavor: "wowf", set: "raid", character: "Grrlwave", realm: "Second Realm", account: null }],
    ["apply_set", { flavor: "wowf", set: "raid", character: "Grrlwave", realm: null, account: null }],
    ["apply_set", { flavor: "wowf", set: "raid", character: "Mrglina", realm: "Made-up Realm", account: "MRGLTWIN" }],
    ["undo_set", { flavor: "wowf" }],
    ["edit_set", { flavor: "wowf", set: "raid", addon: "RaidFrames", action: "disable" }],
    ["clean_saved_variables", { flavor: "wowf" }],
  ]);
});
