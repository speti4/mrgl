"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Flavors = require("../../ui/flavors.js");

const flavor = (id, folder) => ({ id, folder, addons: [] });

test("the four flavors show in the order retail, classic, era, forever, whatever the folder order", () => {
  const scanned = [flavor("wow_classic", "_classic_"), flavor("wow_classic_era", "_classic_era_"), flavor("wowf", "_forever_"), flavor("wow", "_retail_")];
  assert.deepEqual(
    Flavors.displayOrder(scanned).map((item) => item.id),
    ["wow", "wow_classic", "wow_classic_era", "wowf"]
  );
  assert.equal(scanned[0].id, "wow_classic", "the given array is not changed");
});

test("an unknown id comes last, by folder, and is its own name", () => {
  const scanned = [flavor("wow_zzz", "_zzz_"), flavor("wowf", "_forever_"), flavor("wow_aaa", "_aaa_")];
  assert.deepEqual(
    Flavors.displayOrder(scanned).map((item) => item.id),
    ["wowf", "wow_aaa", "wow_zzz"]
  );
  assert.equal(Flavors.friendlyName("wow_zzz"), "wow_zzz");
  assert.equal(Flavors.friendlyName(undefined), "");
});

test("the friendly names are the command line's", () => {
  assert.deepEqual(
    Flavors.KNOWN.map(([id]) => Flavors.friendlyName(id)),
    ["retail", "classic", "era", "forever"]
  );
});

test("the window opens on Forever when it is there, else on the first flavor", () => {
  assert.equal(Flavors.defaultId([flavor("wow", "_retail_"), flavor("wowf", "_forever_")]), "wowf");
  assert.equal(Flavors.defaultId([flavor("wow_classic", "_classic_"), flavor("wow", "_retail_")]), "wow");
  assert.equal(Flavors.defaultId([]), null);
  assert.equal(Flavors.defaultId(undefined), null);
});
