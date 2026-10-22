// The row and flavor actions of task 010, the pure part: the items of a row's menu (update, pin or unpin,
// ignore or unignore, the channel), the per-row progress slot an update run's events drive, and the note
// under a row's version. Pure, so `tests/ui/actions.test.js` runs it under Node; `main.js` draws what it
// answers. Every message comes from Rust; the labels here are the menu's own words.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Actions = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // Why a managed addon's "Update" does nothing, by what the row says; null when it can be updated. The
  // update check asks GitHub only for now (task 056 brings the other sources).
  function whyNoUpdate(addon) {
    if (!addon || !addon.managed) return "not managed by mrgl";
    if (addon.linked) return "a linked development folder is never updated";
    if (!addon.source || addon.source.kind !== "github") {
      return addon.source && addon.source.kind === "zip"
        ? "a zip in hand: a newer zip goes in with mrgl add <zip>"
        : "the update check does not ask this source yet";
    }
    if (addon.pinned) return `pinned at ${addon.pinned}: unpin it to update`;
    if (addon.ignored) return "ignored: unignore it to update";
    return null;
  }

  // The items of a row's menu, in order: `{ action, value, label, enabled, title }`. `action` is what the
  // view calls: `update` (Data.updateOne) or a mark (Data.mark with `value`). While an update runs every
  // item waits, since a mark and an install both write the flavor's sidecar.
  function menuItems(addon, busy) {
    const managed = Boolean(addon && addon.managed);
    const waiting = busy ? "an update is running" : null;
    const unmanaged = managed ? null : "not managed by mrgl";
    const noUpdate = whyNoUpdate(addon);
    const update = {
      action: "update",
      value: null,
      label: addon && addon.update ? `Update to ${addon.update.version}` : "Update",
      enabled: !busy && !noUpdate,
      title: waiting || noUpdate || "check this addon and install its update",
    };
    const mark = (action, value, label, title) => ({
      action,
      value,
      label,
      enabled: managed && !busy,
      title: waiting || unmanaged || title,
    });
    const pin =
      addon && addon.pinned
        ? mark("unpin", null, `Unpin (pinned at ${addon.pinned})`, "let updates in again")
        : mark("pin", null, addon && addon.version ? `Pin at ${addon.version}` : "Pin", "keep this version; updates leave it alone");
    const ignore =
      addon && addon.ignored
        ? mark("unignore", null, "Unignore", "check this addon again")
        : mark("ignore", null, "Ignore", "the update check skips it");
    const prerelease = addon && addon.channel === "prerelease";
    const channel = prerelease
      ? mark("channel", "stable", "Follow stable releases", "on the prerelease channel now")
      : mark("channel", "prerelease", "Follow prereleases", "on the stable channel now");
    return [update, pin, ignore, channel];
  }

  // Where each phase of a run puts a row's progress slot: a label and how far the bar is (null: moving,
  // the length unknown). `checked` clears the slot; the run's answer clears every slot.
  const STEPS = {
    checking: { label: "checking", value: null },
    updating: { label: "updating", value: 0.1 },
    downloading: { label: "downloading", value: 0.4 },
    installing: { label: "installing", value: 0.75 },
    updated: { label: "updated", value: 1 },
    failed: { label: "failed", value: 1, failed: true },
  };

  // The slots after an event: a new object, the folder's slot set or cleared; an event without a folder
  // (the stop before the rate limit) leaves them as they were.
  function applyProgress(slots, event) {
    const next = Object.assign({}, slots || {});
    if (!event || !event.folder) return next;
    const step = STEPS[event.phase];
    if (step) next[event.folder] = Object.assign({ phase: event.phase }, step);
    else delete next[event.folder];
    return next;
  }

  // A row with the check's answer of a `checked` event laid over it: the fact, and the update when the
  // check found one (and none when it did not).
  function applyCheck(addon, check) {
    if (!addon || !check) return addon;
    const update =
      check.state === "update-available" && check.latest ? { version: check.latest, release: null, prerelease: false } : null;
    return Object.assign({}, addon, { check, update });
  }

  // The check states a row names under its version: what needs a look or explains a missing arrow.
  const NOTED = ["installed-newer", "guess-only", "source-gone", "failed", "rate-limited"];

  // The note under a row's version: the pin, the ignore, the prerelease channel and the check's word.
  function noteOf(addon) {
    if (!addon) return "";
    const note = [];
    if (addon.pinned) {
      // A pinned addon's check names the release it leaves alone.
      const latest = addon.check && addon.check.state === "pinned" ? addon.check.latest : null;
      note.push(latest && latest !== addon.version ? `pinned at ${addon.pinned}, ${latest} is out` : `pinned at ${addon.pinned}`);
    }
    if (addon.ignored) note.push("ignored");
    if (addon.channel === "prerelease") note.push("prereleases");
    if (addon.check && NOTED.includes(addon.check.state)) note.push(addon.check.label);
    return note.join(", ");
  }

  return { menuItems, whyNoUpdate, STEPS, applyProgress, applyCheck, noteOf };
});
