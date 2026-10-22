// An addon row's one state from the facts the data carries, and the class and label it is drawn with; the
// interface mark the core sets on a row (task 081) next to it. Pure, so `tests/ui/state.test.js` runs it
// under Node.
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

  // The interface states of the core (task 081), in the order the legend lists them, with the marks `scan`
  // and `list` append to an addon's line: a TOC for another expansion (015), an older interface of the
  // build's own (076, the client's words).
  const INTERFACE_STATES = ["mismatch", "out-of-date"];

  const INTERFACE_LABELS = {
    mismatch: "interface mismatch",
    "out-of-date": "out of date",
  };

  // The mark a row draws for its `interface_check`: the class, the mark's words (the core's label, else the
  // table's) and the warning's plain line for the title; null for a current row or an unknown state.
  function interfaceMark(addon) {
    const fact = addon && addon.interface_check;
    if (!fact || !INTERFACE_STATES.includes(fact.state)) return null;
    return {
      state: fact.state,
      className: `interface-${fact.state}`,
      label: fact.label || INTERFACE_LABELS[fact.state],
      title: fact.line || "",
    };
  }

  function interfaceLabelOf(state) {
    return INTERFACE_LABELS[state] || "";
  }

  return {
    STATES,
    SOURCES,
    INTERFACE_STATES,
    stateOf,
    classOf,
    labelOf,
    sourceClass,
    countUpdates,
    hasForeverNotes,
    interfaceMark,
    interfaceLabelOf,
  };
});
