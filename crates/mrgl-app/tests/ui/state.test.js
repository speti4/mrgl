"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const State = require("../../ui/state.js");

const row = (extra) => Object.assign({ folder: "X", name: "X", version: "1", linked: false, source: null, managed: false, guessed: false, update: null, pinned: null, ignored: false, forever: null }, extra);

test("the state follows the facts, highest precedence first", () => {
  assert.equal(State.stateOf(row({ linked: true, update: { version: "2" }, managed: true })), "linked");
  assert.equal(State.stateOf(row({ update: { version: "2" }, managed: true })), "update");
  assert.equal(State.stateOf(row({ managed: true, guessed: true })), "managed");
  assert.equal(State.stateOf(row({ guessed: true, source: { kind: "curseforge", id: "1" } })), "guess");
  assert.equal(State.stateOf(row({})), "unmanaged");
  assert.equal(State.stateOf(undefined), "unmanaged");
});

test("every state has a class and a label; an unknown state draws as unmanaged", () => {
  for (const state of State.STATES) {
    assert.equal(State.classOf(state), `state-${state}`);
    assert.ok(State.labelOf(state).length > 0, state);
  }
  assert.equal(State.labelOf("update"), "update available");
  assert.equal(State.labelOf("guess"), "guess only");
  assert.equal(State.classOf("mystery"), "state-unmanaged");
  assert.equal(State.labelOf("mystery"), "unmanaged");
});

test("the five states of the task are all there", () => {
  assert.deepEqual(State.STATES, ["managed", "update", "linked", "guess", "unmanaged"]);
});

test("a source badge has a class per kind and none without a source", () => {
  for (const kind of State.SOURCES) assert.equal(State.sourceClass({ kind, id: "x" }), `source-${kind}`);
  assert.deepEqual(State.SOURCES, ["github", "wago", "wowi", "curseforge", "zip", "linked"]);
  assert.equal(State.sourceClass(null), "source-none");
  assert.equal(State.sourceClass({ kind: "ftp", id: "x" }), "source-none");
});

test("update all counts the updates it would touch: not the pinned, not the ignored", () => {
  const addons = [
    row({ update: { version: "2" }, managed: true }),
    row({ update: { version: "2" }, managed: true, pinned: "1" }),
    row({ update: { version: "2" }, managed: true, ignored: true }),
    row({ managed: true }),
  ];
  assert.equal(State.countUpdates(addons), 1);
  assert.equal(State.countUpdates([]), 0);
  assert.equal(State.countUpdates(undefined), 0);
});

test("a flavor has the Forever column only when a row carries a Forever note", () => {
  assert.equal(State.hasForeverNotes([row({}), row({ forever: { build: "yes", note: null } })]), true);
  assert.equal(State.hasForeverNotes([row({ forever: { build: "no", note: "the castbar needs the combat log" } })]), true);
  assert.equal(State.hasForeverNotes([row({}), row({ managed: true })]), false);
  assert.equal(State.hasForeverNotes([]), false);
  assert.equal(State.hasForeverNotes(undefined), false);
});
