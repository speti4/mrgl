"use strict";
// The window's preferences come from the config file through Rust: a cleared web view (no localStorage)
// draws exactly what the file says, MRGL_SERIOUS locks the switch, and an old web view value is carried
// over once, only for a key the file does not store.
const test = require("node:test");
const assert = require("node:assert/strict");
const Preferences = require("../../ui/preferences.js");

const answer = (theme, serious, source, stored) => ({ theme, serious, serious_source: source, stored });

test("a cleared web view draws what the file stores", () => {
  const fromFile = answer("dark", true, "file", { theme: "dark", serious: true });
  assert.deepEqual(Preferences.view(fromFile), { theme: "dark", serious: true, locked: false, note: "" });
  assert.equal(Preferences.migration(fromFile, { theme: null, serious: null }), null, "nothing to carry over");
});

test("MRGL_SERIOUS locks the switch to what it says", () => {
  const view = Preferences.view(answer("auto", false, "environment", { theme: null, serious: true }));
  assert.equal(view.serious, false);
  assert.equal(view.locked, true);
  assert.equal(view.note, "set by MRGL_SERIOUS");
});

test("an old web view value is carried over once, for a key the file lacks", () => {
  const empty = answer("auto", false, "default", { theme: null, serious: null });
  assert.deepEqual(Preferences.migration(empty, { theme: "light", serious: "1" }), { theme: "light", serious: true });
  assert.deepEqual(Preferences.migration(empty, { theme: null, serious: "0" }), { serious: false });
  const themed = answer("dark", false, "default", { theme: "dark", serious: null });
  assert.deepEqual(Preferences.migration(themed, { theme: "light", serious: null }), null, "the file wins");
  assert.equal(Preferences.migration(empty, { theme: "blue", serious: "yes" }), null, "odd values are dropped");
});

test("an answer that is missing or odd draws the defaults", () => {
  assert.deepEqual(Preferences.view(null), { theme: "auto", serious: false, locked: false, note: "" });
  assert.equal(Preferences.view(answer("blue", true, "file", {})).theme, "auto");
  assert.deepEqual(Preferences.LEGACY_KEYS, ["theme", "serious"]);
});
