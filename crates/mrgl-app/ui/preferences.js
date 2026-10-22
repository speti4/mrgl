// The window's preferences (task 034): the theme and the serious switch live in the mrgl home folder's
// `config.json` (the `preferences` and `set_preferences` commands), not in the web view's storage, so a
// cleared web view keeps them and `mrgl` reads the same switch. Pure, so `tests/ui/preferences.test.js` runs
// it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Preferences = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  const THEMES = ["auto", "light", "dark"];

  // The `localStorage` keys of the sketch, read once to carry an old value over and then removed.
  const LEGACY_KEYS = ["theme", "serious"];

  // What the view draws from an answer of `Data.preferences()`: the theme, the switch, and whether the
  // switch is locked because `MRGL_SERIOUS` decides it (the window cannot override the environment).
  function view(answer) {
    const theme = answer && THEMES.includes(answer.theme) ? answer.theme : "auto";
    const serious = Boolean(answer && answer.serious);
    const locked = Boolean(answer && answer.serious_source === "environment");
    return {
      theme,
      serious,
      locked,
      note: locked ? "set by MRGL_SERIOUS" : "",
    };
  }

  // The one-time carry-over of the sketch's `localStorage` values: a value the file does not store yet and
  // the web view has (`theme` one of the themes, `serious` "1" or "0"). `null` when there is nothing to
  // carry over.
  function migration(answer, legacy) {
    const stored = (answer && answer.stored) || {};
    const old = legacy || {};
    const changes = {};
    if (stored.theme == null && THEMES.includes(old.theme)) changes.theme = old.theme;
    if (stored.serious == null && (old.serious === "1" || old.serious === "0")) changes.serious = old.serious === "1";
    return Object.keys(changes).length ? changes : null;
  }

  return { THEMES, LEGACY_KEYS, view, migration };
});
