// The theme preference: `auto` follows the system (`prefers-color-scheme`), `light` and `dark` override it.
// The view writes the resolved theme into `data-theme` on <html>; the CSS reads it. Pure, so
// `tests/ui/theme.test.js` runs it under Node.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Theme = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  const THEMES = ["auto", "light", "dark"];

  // A stored value that is not a theme is `auto`.
  function normalize(theme) {
    return THEMES.includes(theme) ? theme : "auto";
  }

  // The toggle cycles auto -> light -> dark -> auto.
  function next(theme) {
    return THEMES[(THEMES.indexOf(normalize(theme)) + 1) % THEMES.length];
  }

  // The theme the page draws with: the preference, or the system's when the preference is `auto`.
  function resolve(theme, systemDark) {
    const wanted = normalize(theme);
    if (wanted === "auto") return systemDark ? "dark" : "light";
    return wanted;
  }

  // The toggle's text: the preference, and what it resolves to when that is not the same word.
  function label(theme, systemDark) {
    const wanted = normalize(theme);
    const resolved = resolve(wanted, systemDark);
    return wanted === "auto" ? `Theme: auto (${resolved})` : `Theme: ${wanted}`;
  }

  return { THEMES, normalize, next, resolve, label };
});
