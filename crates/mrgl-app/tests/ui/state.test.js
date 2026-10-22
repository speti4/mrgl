"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const State = require("../../ui/state.js");

const row = (extra) => Object.assign({ folder: "X", name: "X", version: "1", linked: false, source: null, managed: false, guessed: false, update: null, pinned: null, ignored: false, forever: null, interface_check: null }, extra);

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

test("a row's interface mark follows the core's interface check (task 081)", () => {
  assert.deepEqual(State.INTERFACE_STATES, ["mismatch", "out-of-date"]);
  assert.equal(State.interfaceMark(row({})), null, "a current row has no mark");
  assert.equal(State.interfaceMark(undefined), null);
  const behind = "Behind: interface 120100 is out of date for the build 12.1.5.70448 (120105)";
  assert.deepEqual(State.interfaceMark(row({ interface_check: { state: "out-of-date", label: "out of date", line: behind } })), {
    state: "out-of-date",
    className: "interface-out-of-date",
    label: "out of date",
    title: behind,
  });
  const elsewhere = "Elsewhere: interface 110207 is not for the build 12.1.5.70448";
  const mismatch = State.interfaceMark(row({ interface_check: { state: "mismatch", label: "interface mismatch", line: elsewhere } }));
  assert.equal(mismatch.className, "interface-mismatch");
  assert.equal(mismatch.label, "interface mismatch", "the mark `scan` and `list` append to the line");
  assert.equal(mismatch.title, elsewhere, "the warning's plain line for the title");
  assert.equal(State.interfaceMark(row({ interface_check: { state: "mismatch" } })).label, "interface mismatch", "the table's words without a label");
  assert.equal(State.interfaceMark(row({ interface_check: { state: "mystery", label: "?", line: "" } })), null, "an unknown state draws nothing");
  for (const state of State.INTERFACE_STATES) assert.ok(State.interfaceLabelOf(state).length > 0, state);
  assert.equal(State.interfaceLabelOf("mystery"), "");
});
