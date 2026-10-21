"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Search = require("../../ui/search.js");

const addons = [
  { folder: "Questie", name: "Questie", source: { kind: "github", id: "Questie/Questie" } },
  { folder: "DBM-Core", name: "Deadly Boss Mods", source: { kind: "curseforge", id: "3358" } },
  { folder: "MyAuras", name: "My Auras", source: null },
  { folder: "Plater", name: "Plater Nameplates", source: { kind: "wago", id: "kRNLep6o" } },
];

const names = (rows) => rows.map((row) => row.folder);

test("an empty or blank query keeps every row in order", () => {
  assert.deepEqual(names(Search.filter(addons, "")), ["Questie", "DBM-Core", "MyAuras", "Plater"]);
  assert.deepEqual(names(Search.filter(addons, "   ")), ["Questie", "DBM-Core", "MyAuras", "Plater"]);
  assert.deepEqual(names(Search.filter(addons, undefined)), ["Questie", "DBM-Core", "MyAuras", "Plater"]);
  assert.deepEqual(Search.filter(undefined, "x"), []);
});

test("the query matches the name, the folder and the source, case-insensitive", () => {
  assert.deepEqual(names(Search.filter(addons, "deadly")), ["DBM-Core"]);
  assert.deepEqual(names(Search.filter(addons, "dbm")), ["DBM-Core"]);
  assert.deepEqual(names(Search.filter(addons, "GitHub")), ["Questie"]);
  assert.deepEqual(names(Search.filter(addons, "krnl")), ["Plater"]);
  assert.deepEqual(names(Search.filter(addons, "auras")), ["MyAuras"]);
});

test("every word of the query has to match", () => {
  assert.deepEqual(names(Search.filter(addons, "boss mods")), ["DBM-Core"]);
  assert.deepEqual(names(Search.filter(addons, "boss questie")), []);
  assert.deepEqual(Search.words("  Boss   Mods "), ["boss", "mods"]);
});

test("the filter never changes the given rows", () => {
  const copy = JSON.parse(JSON.stringify(addons));
  Search.filter(addons, "questie");
  assert.deepEqual(addons, copy);
});
