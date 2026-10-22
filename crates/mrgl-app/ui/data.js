// The data module of the window (task 009): the contract of `mock-data.js` behind `window.Data`, answered by
// the Tauri commands of `src/commands.rs`. The view (`main.js`) reads `window.Data` and does not know which
// module answers. Every message arrives from Rust as `{ kind, murloc, plain }` (D-005); nothing here
// composes one.
//
//   Data.version()                   -> Promise<string>             `app_version`
//   Data.flavors()                   -> Promise<{ root, root_origin, flavors, message }>   `flavors`
//   Data.chooseRoot()                -> Promise<{ flavors, message }>  `choose_root`: the folder dialog;
//                                       both null when it was cancelled
//   Data.diagnostics()               -> Promise<{ text, copied, failed }>  `diagnostics`: the doctor's text
//   Data.resolve(flavorId, folder)   -> Promise<{ pick, message }>  `resolve`: the source's Pick
//   Data.checkUpdates(flavorId, progress)        -> Promise<{ flavor, messages }>  `check_updates` (010)
//   Data.updateAll(flavorId, progress)           -> Promise<{ flavor, messages }>  `update_all` (010)
//   Data.updateOne(flavorId, folder, progress)   -> Promise<{ flavor, messages }>  `update_one` (010)
//                                       `progress(event)` per event of the run, as it happens, through a
//                                       Tauri channel; the answer is the flavor as it is now and the
//                                       messages, the summary last
//   Data.cancelUpdate()              -> Promise<message>            `cancel_update`: before the next addon
//   Data.mark(flavorId, folder, action, value) -> Promise<{ flavor, messages }>  `mark`: pin | unpin |
//                                       ignore | unignore | channel, `value` the version or the channel
//   Data.preferences()               -> Promise<{ theme, serious, serious_source, stored }>  `preferences`
//   Data.setPreferences(changes)     -> Promise<the same>           `set_preferences`: `{ theme?, serious? }`
//                                       into `config.json` (task 034); rejects with a message
//   Data.toolbox(flavorId)           -> Promise<{ toolbox, changes, zip, messages }>  `toolbox` (074): the
//                                       sets, the characters, the SavedVariables and the running client
//   Data.applySet(flavorId, set, target)  -> the same            `apply_set`: `target` `{ character, realm,
//                                       account }` (no character or no target: every character; the
//                                       account only for a realm and name under two accounts, task 085)
//   Data.undoSet(flavorId)           -> the same                 `undo_set` (085): the last apply put back
//   Data.editSet(flavorId, set, addon, action)  -> the same      `edit_set` (085): enable | disable | drop
//   Data.cleanSavedVariables(flavorId)    -> the same            `clean_saved_variables`: the orphans into
//                                       the dated zip
//   Data.adopt(flavorId)             -> Promise<{ adopt, flavor, adopted, messages }>  `adopt` (109): the
//                                       adopt report, nothing written
//   Data.adoptApply(flavorId, alongside, answers)  -> the same   `adopt_apply` (109): the adoptable addons
//                                       into the sidecar; `alongside` the confirmation in a folder the
//                                       CurseForge app manages; `answers` the shown report's `adopt.answers`
//                                       (task 113), so no source is asked again (null asks them all)
//
// Opened in a plain browser (no Tauri global, `ui/index.html` from the disk), the module writes the script
// tag of `mock-data.js` instead, so the page shows the sketch's made-up rows: a look at the layout without
// the core. `document.write` is what keeps the order: the mock runs before the deferred `main.js`.
(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
    return;
  }
  const module_ = factory();
  const tauri = root.__TAURI__;
  if (tauri && tauri.core && typeof tauri.core.invoke === "function") {
    root.Data = module_.create(tauri.core.invoke, tauri.core.Channel);
  } else if (root.document) {
    root.document.write('<script src="mock-data.js"></script>');
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The module over an `invoke(command, args)` that returns a promise and a `Channel` class whose
  // instances take an `onmessage`: Tauri's in the window, fakes in the tests.
  function create(invoke, Channel) {
    // A run's progress channel: each event the command sends reaches `progress`.
    function channel(progress) {
      const events = new Channel();
      events.onmessage = (event) => {
        if (typeof progress === "function") progress(event);
      };
      return events;
    }

    return {
      version: () => invoke("app_version"),
      flavors: () => invoke("flavors"),
      chooseRoot: () => invoke("choose_root"),
      diagnostics: () => invoke("diagnostics"),
      resolve: (flavorId, folder) => invoke("resolve", { flavor: flavorId, folder }),
      checkUpdates: (flavorId, progress) => invoke("check_updates", { flavor: flavorId, progress: channel(progress) }),
      updateAll: (flavorId, progress) => invoke("update_all", { flavor: flavorId, progress: channel(progress) }),
      updateOne: (flavorId, folder, progress) =>
        invoke("update_one", { flavor: flavorId, folder, progress: channel(progress) }),
      cancelUpdate: () => invoke("cancel_update"),
      mark: (flavorId, folder, action, value) =>
        invoke("mark", { flavor: flavorId, folder, action, value: value === undefined ? null : value }),
      preferences: () => invoke("preferences"),
      setPreferences: (changes) =>
        invoke("set_preferences", {
          theme: changes && changes.theme !== undefined ? changes.theme : null,
          serious: changes && changes.serious !== undefined ? changes.serious : null,
        }),
      toolbox: (flavorId) => invoke("toolbox", { flavor: flavorId }),
      applySet: (flavorId, set, target) =>
        invoke("apply_set", {
          flavor: flavorId,
          set,
          character: target && target.character ? target.character : null,
          realm: target && target.character && target.realm ? target.realm : null,
          account: target && target.character && target.account ? target.account : null,
        }),
      undoSet: (flavorId) => invoke("undo_set", { flavor: flavorId }),
      editSet: (flavorId, set, addon, action) => invoke("edit_set", { flavor: flavorId, set, addon, action }),
      cleanSavedVariables: (flavorId) => invoke("clean_saved_variables", { flavor: flavorId }),
      adopt: (flavorId) => invoke("adopt", { flavor: flavorId }),
      adoptApply: (flavorId, alongside, answers) =>
        invoke("adopt_apply", { flavor: flavorId, alongside: Boolean(alongside), answers: answers || null }),
    };
  }

  return { create };
});
