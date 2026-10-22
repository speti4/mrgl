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
//   Data.resolve(flavorId, folder)   -> Promise<{ pick, message }>  `resolve`: the source's Pick (task 010)
//   Data.updateAll(flavorId, report) -> Promise<void>               `update_all`, `report(message)` per step
//   Data.preferences()               -> Promise<{ theme, serious, serious_source, stored }>  `preferences`
//   Data.setPreferences(changes)     -> Promise<the same>           `set_preferences`: `{ theme?, serious? }`
//                                       into `config.json` (task 034); rejects with a message
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
    root.Data = module_.create(tauri.core.invoke);
  } else if (root.document) {
    root.document.write('<script src="mock-data.js"></script>');
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // The module over an `invoke(command, args)` that returns a promise: Tauri's in the window, a fake in the
  // tests.
  function create(invoke) {
    return {
      version: () => invoke("app_version"),
      flavors: () => invoke("flavors"),
      chooseRoot: () => invoke("choose_root"),
      diagnostics: () => invoke("diagnostics"),
      resolve: (flavorId, folder) => invoke("resolve", { flavor: flavorId, folder }),
      preferences: () => invoke("preferences"),
      setPreferences: (changes) =>
        invoke("set_preferences", {
          theme: changes && changes.theme !== undefined ? changes.theme : null,
          serious: changes && changes.serious !== undefined ? changes.serious : null,
        }),
      async updateAll(flavorId, report) {
        const messages = await invoke("update_all", { flavor: flavorId });
        for (const message of messages || []) report(message);
      },
    };
  }

  return { create };
});
