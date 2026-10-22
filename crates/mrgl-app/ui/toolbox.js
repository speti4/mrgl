// The toolbox panel of a flavor (task 074), the pure part: the target picker's options and the target an
// option names, the set the picker shows, the state of the apply and cleanup buttons, the SavedVariables
// rows the table shows and the lines of what an apply changed. Pure, so `tests/ui/toolbox.test.js` runs it
// under Node; `main.js` draws what it answers. Every message comes from Rust; the labels here are the
// panel's own words.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Toolbox = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The value of the target picker's first option: every character of the flavor.
  const EVERY = "";

  // A character's option value: `<realm>/<name>`. A folder name cannot hold a `/`, so the value splits back.
  function characterValue(character) {
    return `${character.realm}/${character.name}`;
  }

  // The target picker's options, in order: every character, then each character by name and realm.
  function targetOptions(characters) {
    const list = characters || [];
    const options = [
      {
        value: EVERY,
        label: list.length === 1 ? "Every character (1)" : `Every character (${list.length})`,
        character: null,
        realm: null,
      },
    ];
    for (const character of list) {
      options.push({
        value: characterValue(character),
        label: `${character.name} (${character.realm})`,
        character: character.name,
        realm: character.realm,
      });
    }
    return options;
  }

  // The target an option value names, as `Data.applySet` takes it: `{ character, realm }`, both null for
  // every character, and for a value whose character is gone (the next load redraws the picker).
  function targetOf(value, characters) {
    const found = (characters || []).find((character) => characterValue(character) === value);
    return found ? { character: found.name, realm: found.realm } : { character: null, realm: null };
  }

  // The set the picker shows: the one named, else the first; null without a set.
  function pickSet(sets, name) {
    const list = sets || [];
    return list.find((set) => set.name === name) || list[0] || null;
  }

  // The addons of a set as the panel lists them: the enabled ones first, then by name; a missing folder
  // is noted, since the apply writes its line all the same.
  function setAddons(set) {
    if (!set) return [];
    return set.addons
      .slice()
      .sort((a, b) => (a.state === b.state ? a.name.localeCompare(b.name) : a.state === "enabled" ? -1 : 1))
      .map((addon) => Object.assign({}, addon, { note: addon.installed ? "" : "not installed" }));
  }

  // The apply button: `{ enabled, label, title }`. A running client does not disable it: the core's guard
  // answers, and the client may be closed by the time the button is pressed.
  function applyButton(toolbox, set, busy) {
    const label = set ? `Apply ${set.name}` : "Apply";
    const why = !toolbox
      ? "the toolbox is not loaded"
      : busy
        ? "a set apply or a cleanup is running"
        : !set
          ? "no set saved yet: mrgl set save <name> saves one"
          : toolbox.characters.length === 0
            ? "no character under WTF yet: log in with a character once"
            : null;
    return {
      enabled: !why,
      label,
      title: why || `write the set's ${set.addons.length} addon lines into AddOns.txt; every other line stays as it is`,
    };
  }

  // The cleanup button: `{ enabled, label, title }`, disabled while a toolbox write or an update runs (an
  // install swaps addon folders, which the orphan rule reads) and when there is no orphan.
  function cleanButton(toolbox, busy, updating) {
    const orphans = toolbox ? toolbox.saved_variables.orphans : 0;
    const label =
      orphans === 0 ? "No orphans to move" : `Move ${orphans} orphan${orphans === 1 ? "" : "s"} into a backup zip`;
    const why = !toolbox
      ? "the toolbox is not loaded"
      : busy
        ? "a set apply or a cleanup is running"
        : updating
          ? "an update is running; the cleanup waits until it ends"
          : orphans === 0
            ? "every SavedVariables file has its addon folder"
            : null;
    return {
      enabled: !why,
      label,
      title:
        why ||
        `move the orphans and their .bak files (${toolbox.saved_variables.orphan_size}) into SavedVariables-<date>.zip; unzip it into WTF to put them back`,
    };
  }

  // The SavedVariables rows the table shows: every one, or the orphans alone; in the listing's order (the
  // largest first).
  function savedVariablesRows(toolbox, orphansOnly) {
    const files = toolbox ? toolbox.saved_variables.files : [];
    return orphansOnly ? files.filter((file) => file.status === "orphan") : files.slice();
  }

  // The words of a SavedVariables status.
  const STATUS_LABELS = {
    installed: "installed",
    orphan: "orphan",
    client: "the client's own",
    unknown: "not compared",
  };

  function statusLabel(status) {
    return STATUS_LABELS[status] || String(status || "");
  }

  // The files of a row: `.lua`, `.lua + .bak` or `.bak`.
  function filesOf(file) {
    if (file.lua && file.bak) return ".lua + .bak";
    return file.bak ? ".bak" : ".lua";
  }

  // What an apply changed, one line per character, as `mrgl set apply` prints its data lines.
  function changeLines(changes) {
    return (changes || []).map((change) => {
      const label = change.character.label;
      if (!change.changed.length) return `${label}: unchanged`;
      const lines = change.changed.map((line) => `${line.name} ${line.state}`).join(", ");
      return `${label}: ${lines}${change.created ? " (a new AddOns.txt)" : ""}`;
    });
  }

  return {
    EVERY,
    targetOptions,
    targetOf,
    pickSet,
    setAddons,
    applyButton,
    cleanButton,
    savedVariablesRows,
    statusLabel,
    filesOf,
    changeLines,
  };
});
