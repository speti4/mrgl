"use strict";
// The row menu, the progress slot and the row note of task 010.
const test = require("node:test");
const assert = require("node:assert/strict");
const Actions = require("../../ui/actions.js");

const managed = {
  folder: "Questie",
  name: "Questie",
  version: "1.2.2",
  linked: false,
  source: { kind: "github", id: "Questie/Questie" },
  managed: true,
  guessed: false,
  update: { version: "1.2.3", release: "v1.2.3", prerelease: false },
  check: { state: "update-available", label: "update available", latest: "1.2.3", detail: null, line: "x" },
  channel: "stable",
  pinned: null,
  ignored: false,
  forever: null,
  missing: [],
};

const byAction = (items) => Object.fromEntries(items.map((item) => [item.action, item]));

test("a managed GitHub addon offers its update and the four marks", () => {
  const items = Actions.menuItems(managed, false);
  assert.deepEqual(
    items.map((item) => [item.action, item.value, item.label, item.enabled]),
    [
      ["update", null, "Update to 1.2.3", true],
      ["pin", null, "Pin at 1.2.2", true],
      ["ignore", null, "Ignore", true],
      ["channel", "prerelease", "Follow prereleases", true],
    ]
  );
});

test("each mark turns into its counterpart once it is set", () => {
  const marked = Object.assign({}, managed, { pinned: "1.2.2", ignored: true, channel: "prerelease", update: null });
  const items = byAction(Actions.menuItems(marked, false));
  assert.equal(items.unpin.label, "Unpin (pinned at 1.2.2)");
  assert.equal(items.unignore.label, "Unignore");
  assert.deepEqual([items.channel.value, items.channel.label], ["stable", "Follow stable releases"]);
  assert.equal(items.update.label, "Update");
  assert.equal(items.update.enabled, false);
  assert.equal(items.update.title, "pinned at 1.2.2: unpin it to update");
});

test("an unmanaged, a linked and a zip row say why they cannot be updated", () => {
  const unmanaged = Object.assign({}, managed, { managed: false, update: null, channel: null });
  const items = Actions.menuItems(unmanaged, false);
  assert.ok(items.every((item) => !item.enabled), "nothing to do on an unmanaged row");
  assert.ok(items.every((item) => item.title === "not managed by mrgl"));
  const linked = Object.assign({}, managed, { linked: true, source: { kind: "linked", id: "D:\\dev" } });
  assert.equal(Actions.whyNoUpdate(linked), "a linked development folder is never updated");
  assert.equal(byAction(Actions.menuItems(linked, false)).pin.enabled, true, "a linked addon can still be marked");
  const zip = Object.assign({}, managed, { source: { kind: "zip", id: "x.zip" } });
  assert.match(Actions.whyNoUpdate(zip), /^a zip in hand/);
  assert.equal(Actions.whyNoUpdate(managed), null);
});

test("a source the check did not ask says the core's words, an asked one updates (task 107)", () => {
  const notChecked = (source, detail) =>
    Object.assign({}, managed, {
      source,
      update: null,
      check: { state: "not-checked", label: "not checked", latest: null, order: null, detail, line: "x" },
    });
  const noKey =
    "source not configured: wago:x (MRGL_WAGO_KEY is not set: the Wago API needs your own key, `mrgl key set wago` stores it)";
  const wago = notChecked({ kind: "wago", id: "x" }, noKey);
  assert.equal(Actions.whyNoUpdate(wago), noKey);
  const update = byAction(Actions.menuItems(wago, false)).update;
  assert.deepEqual([update.enabled, update.title], [false, noKey]);
  const curse = notChecked({ kind: "curseforge", id: "42" }, "source not configured: curse:42 (this build has no CurseForge source)");
  assert.equal(Actions.whyNoUpdate(curse), "source not configured: curse:42 (this build has no CurseForge source)");
  for (const source of [{ kind: "wowi", id: "24976" }, { kind: "wago", id: "4N2mZ3KL" }]) {
    const asked = Object.assign({}, managed, { source });
    assert.equal(Actions.whyNoUpdate(asked), null, source.kind);
    assert.equal(byAction(Actions.menuItems(asked, false)).update.enabled, true, source.kind);
  }
  for (const row of [wago, curse]) assert.ok(!/does not ask this source/.test(Actions.whyNoUpdate(row)), "no line of the old wording");
});

test("while an update runs every item waits", () => {
  const items = Actions.menuItems(managed, true);
  assert.ok(items.every((item) => !item.enabled && item.title === "an update is running"));
});

test("the progress slot follows a row through the run", () => {
  let slots = {};
  const step = (phase, folder) => {
    slots = Actions.applyProgress(slots, { phase, folder, message: { kind: "progress", murloc: "", plain: phase } });
    return slots[folder];
  };
  assert.deepEqual(step("checking", "Questie"), { phase: "checking", label: "checking", value: null });
  assert.equal(step("checked", "Questie"), undefined, "a checked addon clears its slot");
  assert.equal(step("updating", "Questie").value, 0.1);
  assert.equal(step("downloading", "Questie").value, 0.4);
  assert.equal(step("installing", "Questie").value, 0.75);
  assert.equal(step("updated", "Questie").value, 1);
  assert.deepEqual(step("installed-already", "QuestieClassic"), {
    phase: "installed-already",
    label: "installed already",
    value: 1,
  });
  assert.deepEqual(step("folded", "SeveralFolders_Options"), {
    phase: "folded",
    label: "folded into another entry",
    value: 1,
  });
  assert.equal(step("failed", "BigWigs").failed, true);
  const before = slots;
  const after = Actions.applyProgress(slots, { phase: "stopped", folder: null });
  assert.deepEqual(after, before, "the stop belongs to no row");
  assert.notEqual(after, before, "a new object every time");
});

test("a checked event lays the check's answer over the row", () => {
  const plain = Object.assign({}, managed, { update: null, check: null });
  const checked = Actions.applyCheck(plain, managed.check);
  assert.deepEqual(checked.update.version, "1.2.3");
  assert.equal(checked.check.state, "update-available");
  const newer = Actions.applyCheck(managed, { state: "installed-newer", label: "newer than the release", latest: "1.2.0" });
  assert.equal(newer.update, null, "a newer installed version is no update");
  assert.equal(Actions.applyCheck(managed, null), managed);
});

test("the note names the marks and a check state that needs a word", () => {
  assert.equal(Actions.noteOf(managed), "");
  const noted = Object.assign({}, managed, {
    pinned: "1.0",
    ignored: true,
    channel: "prerelease",
    check: { state: "installed-newer", label: "newer than the release" },
  });
  assert.equal(Actions.noteOf(noted), "pinned at 1.0, ignored, prereleases, newer than the release");
  const gone = Object.assign({}, managed, { check: { state: "source-gone", label: "source gone" } });
  assert.equal(Actions.noteOf(gone), "source gone");
  const pending = Object.assign({}, managed, { check: { state: "pending", label: "pending approval" } });
  assert.equal(Actions.noteOf(pending), "pending approval", "a file that waits for approval (task 104)");
  const off = Object.assign({}, managed, { check: { state: "distribution-off", label: "only on CurseForge" } });
  assert.equal(Actions.noteOf(off), "only on CurseForge", "a project without distribution (task 115)");
  const upToDate = Object.assign({}, managed, { check: { state: "up-to-date", label: "up to date" } });
  assert.equal(Actions.noteOf(upToDate), "");
  const held = Object.assign({}, managed, { pinned: "1.2.2", check: { state: "pinned", label: "pinned", latest: "1.2.3" } });
  assert.equal(Actions.noteOf(held), "pinned at 1.2.2, 1.2.3 is out", "the release a pin leaves alone");
});

test("a pinned row's note follows the check's version order", () => {
  const pinned = (version, latest, order) =>
    Actions.noteOf(Object.assign({}, managed, { version, pinned: version, check: { state: "pinned", label: "pinned", latest, order } }));
  assert.equal(pinned("1.1.0-beta", "1.0.0", "newer"), "pinned at 1.1.0-beta, newer than the release", "no release is out after it");
  assert.equal(pinned("1.0.0", "1.1.0", "older"), "pinned at 1.0.0, 1.1.0 is out");
  assert.equal(pinned("v1.0.0", "1.0.0", "same"), "pinned at v1.0.0", "the same version by the order");
  assert.equal(pinned("1.0.0", "1.1.0", null), "pinned at 1.0.0, 1.1.0 is out", "no order: another version is out");
  assert.equal(pinned("1.0.0", "1.0.0", null), "pinned at 1.0.0");
});

test("the add form waits for a run and an AddOns folder and trims its source (task 116)", () => {
  const flavor = { id: "wowf", addons_folder: true, addons: [] };
  assert.deepEqual(Actions.addZipButton(flavor, false), {
    enabled: true,
    title: "install an addon zip from disk, with the source it comes from",
  });
  assert.deepEqual(Actions.addZipButton(flavor, true), { enabled: false, title: "an update or an add is running" });
  assert.deepEqual(Actions.addZipButton(Object.assign({}, flavor, { addons_folder: false }), false), {
    enabled: false,
    title: "the client has not been started for this flavor yet",
  });
  assert.equal(Actions.addZipButton(null, false).enabled, false);
  assert.deepEqual(Actions.addZipRequest("D:\\Downloads\\A-1.0.zip", "  curse:4244 "), {
    zip: "D:\\Downloads\\A-1.0.zip",
    source: "curse:4244",
  });
  assert.deepEqual(Actions.addZipRequest("A.zip", "   "), { zip: "A.zip", source: null });
  assert.deepEqual(Actions.addZipRequest("A.zip", undefined), { zip: "A.zip", source: null });
  assert.equal(Actions.fileName("D:\\Downloads\\MrglNoDistribution-2.0.0.zip"), "MrglNoDistribution-2.0.0.zip");
  assert.equal(Actions.fileName("/tmp/a/B-1.zip"), "B-1.zip");
  assert.equal(Actions.fileName(null), "");
});
