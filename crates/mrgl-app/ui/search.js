// The search field's filter: every word of the query has to appear in the row's name, folder, source kind
// or source id, case-insensitive. Pure, so `tests/ui/search.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Search = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  function words(query) {
    return String(query || "")
      .toLowerCase()
      .split(/\s+/)
      .filter((word) => word.length > 0);
  }

  function haystack(addon) {
    const parts = [addon.name, addon.folder];
    if (addon.source) parts.push(addon.source.kind, addon.source.id);
    return parts
      .filter((part) => typeof part === "string")
      .join("\n")
      .toLowerCase();
  }

  // The rows that match, in their order; an empty query keeps every row.
  function filter(addons, query) {
    const needles = words(query);
    if (needles.length === 0) return (addons || []).slice();
    return (addons || []).filter((addon) => {
      const text = haystack(addon);
      return needles.every((needle) => text.includes(needle));
    });
  }

  return { filter, words };
});
