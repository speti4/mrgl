// An addon row's one state from the facts the data carries, and the class and label it is drawn with.
// Pure, so `tests/ui/state.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.State = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // Every state a row can be in, in the order the legend lists them.
  const STATES = ["managed", "update", "linked", "guess", "unmanaged"];

  const LABELS = {
    managed: "managed",
    update: "update available",
    linked: "linked",
    guess: "guess only",
    unmanaged: "unmanaged",
  };

  // The precedence, highest first: a junction is never updated, so linked wins over an update; an update
  // outranks the managed it implies; a source known only from a TOC hint is a guess until adopted.
  function stateOf(addon) {
    if (!addon) return "unmanaged";
    if (addon.linked) return "linked";
    if (addon.update) return "update";
    if (addon.managed) return "managed";
    if (addon.guessed) return "guess";
    return "unmanaged";
  }

  // The CSS class of a state; an unknown state draws as unmanaged, never as nothing.
  function classOf(state) {
    return `state-${STATES.includes(state) ? state : "unmanaged"}`;
  }

  function labelOf(state) {
    return LABELS[state] || LABELS.unmanaged;
  }

  // The source badges, in the order the legend lists them.
  const SOURCES = ["github", "wago", "wowi", "curseforge", "zip", "linked"];

  function sourceClass(source) {
    const kind = source && SOURCES.includes(source.kind) ? source.kind : "none";
    return `source-${kind}`;
  }

  // The rows "update all" would touch: an update that is neither pinned nor ignored.
  function countUpdates(addons) {
    return (addons || []).filter((addon) => addon.update && !addon.pinned && !addon.ignored).length;
  }

  // Whether any row of a flavor carries a Forever note (task 011: the build verdict, with or without the
  // curated text); the view shows the Forever column only then (task 035). The rows of the other flavors
  // carry none, so their list has no empty column.
  function hasForeverNotes(addons) {
    return (addons || []).some((addon) => Boolean(addon && addon.forever));
  }

  return { STATES, SOURCES, stateOf, classOf, labelOf, sourceClass, countUpdates, hasForeverNotes };
});
