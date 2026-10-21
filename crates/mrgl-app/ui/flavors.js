// The flavor navigation's order and names: the four known flavors in the order `FlavorId` lists them
// (retail, classic, era, forever), an unknown id after them by folder, shown by its id. Pure, so
// `tests/ui/flavors.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Flavors = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The ids as `.flavor.info` spells them, with the friendly name the command line accepts.
  const KNOWN = [
    ["wow", "retail"],
    ["wow_classic", "classic"],
    ["wow_classic_era", "era"],
    ["wowf", "forever"],
  ];

  function rank(id) {
    const index = KNOWN.findIndex(([known]) => known === id);
    return index === -1 ? KNOWN.length : index;
  }

  // The friendly name of an id; an unknown id is its own name.
  function friendlyName(id) {
    const known = KNOWN.find(([candidate]) => candidate === id);
    return known ? known[1] : String(id || "");
  }

  // The flavors in navigation order, without changing the given array.
  function displayOrder(flavors) {
    return (flavors || [])
      .slice()
      .sort((a, b) => rank(a.id) - rank(b.id) || String(a.folder).localeCompare(String(b.folder)));
  }

  // The flavor the window opens on: Forever first, then the first in navigation order.
  function defaultId(flavors) {
    const ordered = displayOrder(flavors);
    const forever = ordered.find((flavor) => flavor.id === "wowf");
    return (forever || ordered[0] || { id: null }).id;
  }

  return { KNOWN, friendlyName, displayOrder, defaultId };
});
