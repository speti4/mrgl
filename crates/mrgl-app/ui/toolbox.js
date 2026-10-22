// The toolbox panel of a flavor (tasks 074 and 085), the pure part: the target picker's options (with the
// account when two accounts have the realm and the name) and the target an option names, the set the
// picker shows, the edit controls of its chips, the state of the apply, undo and cleanup buttons, what the
// undo puts back, the SavedVariables rows the table shows and the lines of what an apply changed or an
// undo put back. Pure, so `tests/ui/toolbox.test.js` runs it under Node; `main.js` draws what it answers.
// Every message comes from Rust; the labels here are the panel's own words.
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory();
  else root.Toolbox = factory();
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The value of the target picker's first option: every character of the flavor.
  const EVERY = "";

  // Why a toolbox button waits while a toolbox write goes (one at a time).
  const BUSY = "a set apply, undo or edit or a cleanup is running";

  // A character's option value: `<realm>/<name>`, with `<account>/` in front when the realm and the name
  // sit under two accounts (task 085: only then does the answer carry the account). A folder name cannot
  // hold a `/`, so two characters never share a value.
  function characterValue(character) {
    const place = `${character.realm}/${character.name}`;
    return character.account ? `${character.account}/${place}` : place;
  }

  // A character as the panel names it: `<realm>/<name>`, and its account when the answer carries one.
  function characterName(character) {
    return character.account ? `${character.label} (account ${character.account})` : character.label;
  }

  // The target picker's options, in order: every character, then each character by name and realm (and
  // account, when two accounts have the realm and the name).
  function targetOptions(characters) {
    const list = characters || [];
    const options = [
      {
        value: EVERY,
        label: list.length === 1 ? "Every character (1)" : `Every character (${list.length})`,
        character: null,
        realm: null,
        account: null,
      },
    ];
    for (const character of list) {
      options.push({
        value: characterValue(character),
        label: character.account
          ? `${character.name} (${character.realm}, account ${character.account})`
          : `${character.name} (${character.realm})`,
        character: character.name,
        realm: character.realm,
        account: character.account || null,
      });
    }
    return options;
  }

  // The target an option value names, as `Data.applySet` takes it: `{ character, realm, account }`, all
  // null for every character, and for a value whose character is gone (the next load redraws the picker).
  function targetOf(value, characters) {
    const found = (characters || []).find((character) => characterValue(character) === value);
    return found
      ? { character: found.name, realm: found.realm, account: found.account || null }
      : { character: null, realm: null, account: null };
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
        ? BUSY
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

  // The line under the apply that says what the undo puts back (task 085): the set, when it was applied,
  // the `AddOns.txt` files it restores and the ones it removes; empty with nothing to undo.
  function undoNote(undo) {
    if (!undo) return "";
    const names = (characters) => characters.map(characterName).join(", ");
    const parts = [];
    if (undo.restore.length) parts.push(`puts back the AddOns.txt of ${names(undo.restore)} as it was`);
    if (undo.remove.length) parts.push(`removes the AddOns.txt it created for ${names(undo.remove)}`);
    return `the undo of set ${undo.set} (applied ${undo.applied_at}) ${parts.join(" and ")}`;
  }

  // The undo button: `{ enabled, label, title }`. Like the apply, a running client does not disable it:
  // the core's guard answers.
  function undoButton(toolbox, busy) {
    const undo = toolbox ? toolbox.undo : null;
    const why = !toolbox
      ? "the toolbox is not loaded"
      : busy
        ? BUSY
        : !undo
          ? "nothing to undo: an apply that changes a file keeps what it replaced, and an undo puts it back once"
          : null;
    return { enabled: !why, label: undo ? `Undo ${undo.set}` : "Undo", title: why || undoNote(undo) };
  }

  // What an undo put back, one line per character, as `mrgl set undo` prints its data lines.
  function undoneLines(undone) {
    if (!undone) return [];
    return [
      ...undone.restore.map((character) => `${characterName(character)}: AddOns.txt put back as it was`),
      ...undone.remove.map((character) => `${characterName(character)}: AddOns.txt removed, the apply had created it`),
    ];
  }

  // The edit controls of one chip (task 085): the chip itself toggles the addon between enabled and
  // disabled in the set, its `×` drops it; both change `sets.json` only, so a running client does not
  // stop them. `{ toggle, drop }`, each `{ action, label, title, enabled }`.
  function chipActions(set, addon, busy) {
    const toggle = addon.state === "enabled" ? "disable" : "enable";
    const why = busy ? BUSY : null;
    const note = addon.installed ? "" : " (not installed; an apply writes its line all the same)";
    return {
      toggle: {
        action: toggle,
        label: addon.name,
        title: why || `${addon.state}${note}; click to ${toggle} it in set ${set.name}, no AddOns.txt is touched`,
        enabled: !why,
      },
      drop: {
        action: "drop",
        label: `Drop ${addon.name} from set ${set.name}`,
        title: why || `drop ${addon.name} from set ${set.name}: an apply then leaves its lines alone`,
        enabled: !why,
      },
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
        ? BUSY
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
      const label = characterName(change.character);
      if (!change.changed.length) return `${label}: unchanged`;
      const lines = change.changed.map((line) => `${line.name} ${line.state}`).join(", ");
      return `${label}: ${lines}${change.created ? " (a new AddOns.txt)" : ""}`;
    });
  }

  return {
    EVERY,
    BUSY,
    characterName,
    targetOptions,
    targetOf,
    pickSet,
    setAddons,
    applyButton,
    undoNote,
    undoButton,
    undoneLines,
    chipActions,
    cleanButton,
    savedVariablesRows,
    statusLabel,
    filesOf,
    changeLines,
  };
});
