"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Theme = require("../../ui/theme.js");

test("the toggle cycles auto, light, dark", () => {
  assert.equal(Theme.next("auto"), "light");
  assert.equal(Theme.next("light"), "dark");
  assert.equal(Theme.next("dark"), "auto");
  assert.equal(Theme.next("nonsense"), "light");
  assert.equal(Theme.next(null), "light");
});

test("auto follows the system and a choice overrides it", () => {
  assert.equal(Theme.resolve("auto", true), "dark");
  assert.equal(Theme.resolve("auto", false), "light");
  assert.equal(Theme.resolve("light", true), "light");
  assert.equal(Theme.resolve("dark", false), "dark");
  assert.equal(Theme.resolve(undefined, true), "dark");
});

test("a stored value that is not a theme is auto", () => {
  assert.equal(Theme.normalize("dark"), "dark");
  assert.equal(Theme.normalize("blue"), "auto");
  assert.equal(Theme.normalize(null), "auto");
});

test("the label names the preference and what auto resolves to", () => {
  assert.equal(Theme.label("auto", true), "Theme: auto (dark)");
  assert.equal(Theme.label("auto", false), "Theme: auto (light)");
  assert.equal(Theme.label("light", true), "Theme: light");
  assert.equal(Theme.label("dark", false), "Theme: dark");
});
