"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Voice = require("../../ui/voice.js");

test("a message with a murloc line gives two lines", () => {
  assert.deepEqual(Voice.lines({ murloc: "Mrglglglgl!", plain: "2 addons updated" }), ["Mrglglglgl!", "2 addons updated"]);
});

test("a serious message gives the plain line alone", () => {
  assert.deepEqual(Voice.lines({ murloc: "", plain: "2 addons updated" }), ["2 addons updated"]);
});

test("a missing message gives one empty line, never a crash", () => {
  assert.deepEqual(Voice.lines(undefined), [""]);
});

test("one line joins with a dash", () => {
  assert.equal(Voice.oneLine({ murloc: "Mrglglgl.", plain: "mrgl 0.1.0" }), "Mrglglgl. — mrgl 0.1.0");
  assert.equal(Voice.oneLine({ murloc: "", plain: "mrgl 0.1.0" }), "mrgl 0.1.0");
});

test("every kind reaches the status bar", () => {
  for (const kind of Voice.KINDS) assert.equal(Voice.placement(kind).status, true, kind);
  assert.equal(Voice.placement("something else").status, true);
});

test("a finished message also toasts; the quiet kinds never do", () => {
  assert.deepEqual(Voice.placement("success"), { status: true, toast: true, sticky: false });
  assert.deepEqual(Voice.placement("warning"), { status: true, toast: true, sticky: false });
  for (const kind of ["banner", "progress", "nothing", "something else", undefined]) {
    assert.equal(Voice.placement(kind).toast, false, String(kind));
  }
});

test("an error or a fatal toast stays until dismissed", () => {
  assert.deepEqual(Voice.placement("error"), { status: true, toast: true, sticky: true });
  assert.deepEqual(Voice.placement("fatal"), { status: true, toast: true, sticky: true });
});

test("a kind has a class and an unknown one draws plain", () => {
  assert.equal(Voice.kindClass("error"), "kind-error");
  assert.equal(Voice.kindClass("banner"), "kind-banner");
  assert.equal(Voice.kindClass("murloc"), "kind-plain");
  assert.equal(Voice.kindClass(undefined), "kind-plain");
});

test("serious drops the murloc line and keeps the kind and the facts", () => {
  const message = { kind: "error", murloc: "Rwlrwlrwlrwl?!", plain: "addon not found: Questie (flavor forever)" };
  assert.deepEqual(Voice.serious(message), { kind: "error", murloc: "", plain: "addon not found: Questie (flavor forever)" });
  assert.deepEqual(Voice.serious(undefined), { kind: undefined, murloc: "", plain: "" });
});
