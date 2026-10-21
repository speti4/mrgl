// The UI side of the voice (D-005): a message from Rust is { murloc, plain }; this module only lays it out.
// It never composes a murloc line of its own. Pure, so `tests/ui/voice.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Voice = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The lines to show, in order: the murloc line only when it is there (a serious voice sends it empty).
  function lines(message) {
    const out = [];
    if (message && message.murloc) out.push(message.murloc);
    out.push(message && message.plain ? message.plain : "");
    return out;
  }

  // One string for a title or a tooltip: "murloc — plain" or the plain line alone.
  function oneLine(message) {
    return lines(message).join(" — ");
  }

  return { lines, oneLine };
});
