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
