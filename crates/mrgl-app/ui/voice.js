// The UI side of the voice (D-005): a message from Rust is { kind, murloc, plain }; this module only lays it
// out. It never composes a murloc line of its own. Pure, so `tests/ui/voice.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Voice = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  const KINDS = ["banner", "progress", "success", "nothing", "warning", "error", "fatal"];

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

  // Where a message goes (docs/nerglish.md, "The window"): every message replaces the status bar's line, and
  // a finished message (success, warning, error, fatal) also pops a toast. A success or a warning toast goes
  // away by itself; an error or a fatal one stays until it is dismissed. Banner, progress and nothing-to-do
  // never toast: they are the window's quiet state. An unknown kind is treated as the quiet state.
  function placement(kind) {
    switch (kind) {
      case "success":
      case "warning":
        return { status: true, toast: true, sticky: false };
      case "error":
      case "fatal":
        return { status: true, toast: true, sticky: true };
      default:
        return { status: true, toast: false, sticky: false };
    }
  }

  // The CSS class of a kind, for the status dot and the toast's color; an unknown kind is `kind-plain`.
  function kindClass(kind) {
    return `kind-${KINDS.includes(kind) ? kind : "plain"}`;
  }

  // The same message with its murloc line dropped, for the serious preference of the window.
  function serious(message) {
    return { kind: message && message.kind, murloc: "", plain: message && message.plain ? message.plain : "" };
  }

  return { KINDS, lines, oneLine, placement, kindClass, serious };
});
