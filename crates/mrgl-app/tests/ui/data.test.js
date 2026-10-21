"use strict";
// The data module of the window: each call of the contract invokes its Tauri command with its arguments
// and hands the answer through as it is; "update all" reports every message the command answers with.
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

test("update all reports each message in order", async () => {
  const messages = [
    { kind: "progress", murloc: "m", plain: "updating" },
    { kind: "success", murloc: "m", plain: "done" },
  ];
  const { calls, invoke } = fake({ update_all: messages });
  const heard = [];
  await create(invoke).updateAll("wow", (message) => heard.push(message));
  assert.deepEqual(heard, messages);
  assert.deepEqual(calls, [["update_all", { flavor: "wow" }]]);
});

test("an empty answer reports nothing", async () => {
  const { invoke } = fake({ update_all: null });
  const heard = [];
  await create(invoke).updateAll("wow", (message) => heard.push(message));
  assert.deepEqual(heard, []);
});
