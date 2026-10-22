// The view (task 017): the flavor navigation, the addon list, the search, the voice's two places and the
// theme; the game folder chooser and "copy diagnostics" (task 009); the update check, "update all", a row's
// update with its progress slot, the cancel and the row menu's marks (task 010). It reads `window.Data`
// and draws what it gets; it does not know whether the rows are made up (`mock-data.js`) or the core's
// answers (`data.js`), and it never composes a murloc line.
(function () {
  "use strict";

  const Voice = window.Voice;
  const State = window.State;
  const Search = window.Search;
  const Theme = window.Theme;
  const Flavors = window.Flavors;
  const Actions = window.Actions;
  const Data = window.Data;

  const $ = (id) => document.getElementById(id);

  // ---- preferences: the theme and the serious switch, kept by the shell in the home folder's config.json
  // (task 034), so a cleared web view keeps them and `mrgl` reads the same switch

  const Preferences = window.Preferences;
  const systemDark = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  let theme = "auto";
  let serious = false;

  function applyTheme() {
    const dark = Boolean(systemDark && systemDark.matches);
    document.documentElement.dataset.theme = Theme.resolve(theme, dark);
    $("theme-toggle").textContent = Theme.label(theme, dark);
  }

  const seriousToggle = $("serious-toggle");

  // Draws an answer of `Data.preferences()` or `Data.setPreferences()`.
  function applyPreferences(answer) {
    const shown = Preferences.view(answer);
    theme = shown.theme;
    serious = shown.serious;
    seriousToggle.checked = serious;
    seriousToggle.disabled = shown.locked;
    seriousToggle.parentElement.title = shown.note;
    applyTheme();
    if (lastMessage) showStatus(lastMessage);
  }

  async function savePreferences(changes) {
    try {
      applyPreferences(await Data.setPreferences(changes));
    } catch (error) {
      // The command's error is a message in the voice; anything else is the bridge failing.
      say(error && error.plain ? error : { kind: "error", murloc: "", plain: `preferences not saved: ${error}` });
    }
  }

  // The sketch kept both in `localStorage`: a value the file does not have yet is carried over once, and
  // the old keys go.
  function legacyPreferences() {
    const legacy = {};
    try {
      for (const key of Preferences.LEGACY_KEYS) {
        legacy[key] = window.localStorage.getItem(key);
        window.localStorage.removeItem(key);
      }
    } catch (_) {
      // A web view without storage has nothing to carry over.
    }
    return legacy;
  }

  async function loadPreferences() {
    try {
      const answer = await Data.preferences();
      const changes = Preferences.migration(answer, legacyPreferences());
      applyPreferences(changes ? await Data.setPreferences(changes) : answer);
    } catch (error) {
      say(error && error.plain ? error : { kind: "error", murloc: "", plain: `preferences not read: ${error}` });
    }
  }

  $("theme-toggle").addEventListener("click", () => {
    theme = Theme.next(theme);
    applyTheme();
    savePreferences({ theme });
  });
  if (systemDark && systemDark.addEventListener) systemDark.addEventListener("change", applyTheme);
  applyTheme();

  seriousToggle.addEventListener("change", () => {
    serious = seriousToggle.checked;
    if (lastMessage) showStatus(lastMessage);
    savePreferences({ serious });
  });

  // ---- the voice's two places: the status bar (every message) and the toasts (finished ones)

  let lastMessage = null;

  function heard(message) {
    return serious ? Voice.serious(message) : message;
  }

  function showStatus(message) {
    const shown = heard(message);
    const dot = $("status-dot");
    dot.className = `dot ${Voice.kindClass(shown.kind)}`;
    $("status-murloc").textContent = shown.murloc;
    $("status-plain").textContent = shown.plain;
    $("status").title = Voice.oneLine(shown);
  }

  function showToast(message, sticky) {
    const shown = heard(message);
    const toast = document.createElement("div");
    toast.className = `toast ${Voice.kindClass(shown.kind)}`;
    const murloc = document.createElement("p");
    murloc.className = "murloc";
    murloc.textContent = shown.murloc;
    const plain = document.createElement("p");
    plain.className = "plain";
    plain.textContent = shown.plain;
    const close = document.createElement("button");
    close.type = "button";
    close.className = "close";
    close.setAttribute("aria-label", "Dismiss");
    close.textContent = "×";
    close.addEventListener("click", () => toast.remove());
    toast.append(murloc, plain, close);
    toast.classList.toggle("sticky", Boolean(sticky));
    const stack = $("toasts");
    stack.append(toast);
    // The cap (Voice.TOAST_CAP): the oldest brief toasts make room, a sticky one stays until dismissed.
    const stacked = Array.from(stack.children, (element) => ({ element, sticky: element.classList.contains("sticky") }));
    for (const old of Voice.pushedOut(stacked)) old.element.remove();
    if (!sticky) window.setTimeout(() => toast.remove(), shown.kind === "warning" ? 7000 : 4500);
  }

  // Every message passes through here, whoever sends it; the kind decides where it is shown.
  function say(message) {
    lastMessage = message;
    const where = Voice.placement(message.kind);
    if (where.status) showStatus(message);
    if (where.toast) showToast(message, where.sticky);
  }

  // ---- the model: what the data module answered, the chosen flavor, the search text

  let root = null;
  let flavors = [];
  let selectedId = null;
  let query = "";
  // The update run going, one at a time (task 010): its id, its flavor, the progress slots of its rows.
  let run = null;
  let runs = 0;

  function selected() {
    return flavors.find((flavor) => flavor.id === selectedId) || null;
  }

  // Puts a flavor of an answer in place of the one with its id.
  function replaceFlavor(flavor) {
    if (!flavor) return;
    const index = flavors.findIndex((item) => item.id === flavor.id);
    if (index >= 0) flavors[index] = flavor;
    else flavors.push(flavor);
  }

  // ---- the flavor navigation

  function renderNav() {
    const list = $("flavor-list");
    list.replaceChildren();
    for (const flavor of Flavors.displayOrder(flavors)) {
      const item = document.createElement("li");
      const button = document.createElement("button");
      button.type = "button";
      button.className = "flavor-button";
      button.classList.toggle("selected", flavor.id === selectedId);
      button.setAttribute("aria-current", flavor.id === selectedId ? "page" : "false");
      const name = document.createElement("span");
      name.className = "flavor-name";
      name.textContent = flavor.name || Flavors.friendlyName(flavor.id);
      const folder = document.createElement("span");
      folder.className = "flavor-folder muted";
      folder.textContent = flavor.folder;
      const count = document.createElement("span");
      count.className = "count";
      count.textContent = String(flavor.addons.length);
      count.title = `${flavor.addons.length} addons`;
      button.append(name, folder, count);
      const updates = State.countUpdates(flavor.addons);
      if (updates > 0) {
        const due = document.createElement("span");
        due.className = "count updates";
        due.textContent = `${updates}↑`;
        due.title = `${updates} updates available`;
        button.append(due);
      }
      button.addEventListener("click", () => {
        selectedId = flavor.id;
        renderNav();
        renderFlavor();
      });
      item.append(button);
      list.append(item);
    }
  }

  // ---- the chosen flavor: its header, its rows, the legend

  function renderFlavor() {
    const flavor = selected();
    $("empty").hidden = Boolean(flavor);
    $("flavor").hidden = !flavor;
    if (!flavor) return;

    $("flavor-name").textContent = flavor.name || Flavors.friendlyName(flavor.id);
    const meta = [flavor.folder];
    // Where the build was read (task 019): `.build.info`, or the client exe when the file has no row.
    if (flavor.version) meta.push(`build ${flavor.version}${flavor.version_file ? ` from ${flavor.version_file}` : ""}`);
    else meta.push("build unknown");
    if (flavor.interface) meta.push(`interface ${flavor.interface}`);
    if (flavor.curse_client) meta.push("managed by the CurseForge app");
    $("flavor-meta").textContent = meta.join(" · ");

    const warnings = $("flavor-warnings");
    warnings.hidden = !(flavor.warnings && flavor.warnings.length);
    warnings.textContent = (flavor.warnings || []).join("; ");

    // The actions: the check and "update all", and the cancel in the place of "update all" while a run goes.
    // A flavor whose client was never started has no AddOns folder and nothing to check.
    const updates = State.countUpdates(flavor.addons);
    const noFolder = flavor.addons_folder === false;
    const button = $("update-all");
    button.textContent = updates > 0 ? `Update all (${updates})` : "Update all";
    button.disabled = Boolean(run) || noFolder;
    button.hidden = Boolean(run);
    button.title = noFolder ? "the client has not been started for this flavor yet" : "check every addon, then install every update";
    const check = $("check-updates");
    check.disabled = Boolean(run) || noFolder;
    check.title = noFolder ? button.title : "ask the sources about every addon; nothing is installed";
    const cancel = $("cancel-update");
    cancel.hidden = !run;
    cancel.title = run ? `stop the run in ${run.flavorName} before its next addon; the addon in flight finishes` : "";

    // The Forever column (header and cells) shows only when a row of this flavor carries a note.
    $("flavor").classList.toggle("without-forever", !State.hasForeverNotes(flavor.addons));

    renderRows(flavor);
    renderLegend();
  }

  function renderRows(flavor) {
    const list = $("addon-list");
    const rows = Search.filter(flavor.addons, query);
    list.replaceChildren();
    for (const addon of rows) list.append(renderRow(addon));
    const noMatch = $("no-match");
    noMatch.hidden = rows.length > 0;
    noMatch.textContent =
      flavor.addons_folder === false
        ? `no Interface\\AddOns folder in ${flavor.folder}: the client has not been started for this flavor yet`
        : flavor.addons.length === 0
          ? `no addons in ${flavor.folder}`
          : `no addon matches "${query.trim()}" in ${flavor.name}`;
  }

  function renderRow(addon) {
    const fragment = $("row-template").content.cloneNode(true);
    const row = fragment.querySelector(".row");
    const state = State.stateOf(addon);
    row.classList.add(State.classOf(state));
    row.classList.toggle("ignored", Boolean(addon.ignored));
    const slot = run && run.flavorId === selectedId ? run.progress[addon.folder] : null;
    row.classList.toggle("busy", Boolean(slot));
    row.dataset.folder = addon.folder;

    row.querySelector(".name").textContent = addon.name || addon.folder;
    row.querySelector(".folder").textContent = addon.name && addon.name !== addon.folder ? addon.folder : "";
    const missing = addon.missing || [];
    row.querySelector(".missing").textContent = missing.length ? `missing: ${missing.join(", ")}` : "";

    row.querySelector(".version-now").textContent = addon.version || "—";
    row.querySelector(".version-next").textContent = addon.update ? `→ ${addon.update.version}` : "";
    const note = row.querySelector(".version-note");
    note.textContent = Actions.noteOf(addon);
    note.title = addon.check ? addon.check.line : "";

    const badge = row.querySelector(".badge");
    badge.className = `badge ${State.sourceClass(addon.source)}`;
    badge.classList.toggle("guessed", Boolean(addon.guessed));
    badge.textContent = addon.source ? addon.source.kind : "";
    badge.title = addon.source ? `${addon.source.kind}: ${addon.source.id}${addon.guessed ? " (from the TOC, not adopted yet)" : ""}` : "";

    const pill = row.querySelector(".pill");
    pill.classList.add(State.classOf(state));
    pill.textContent = State.labelOf(state);

    const forever = row.querySelector(".forever");
    forever.className = "forever";
    if (addon.forever) {
      const info = addon.forever;
      forever.classList.add(`forever-${info.build}`);
      forever.classList.toggle("forever-stale", Boolean(info.stale));
      forever.textContent = info.note ? `${info.build}: ${info.note}` : info.build;
      const facts = [`Forever build: ${forever.textContent}`];
      if (info.verified) facts.push(`verified ${info.verified}${info.stale ? ", stale" : ""}`);
      if (info.source) facts.push(info.source);
      forever.title = facts.join("\n");
    } else {
      forever.textContent = "";
    }

    // The progress slot: a moving bar while the length is unknown, a filling one by the install's step.
    const progress = row.querySelector(".progress");
    progress.hidden = !slot;
    row.querySelector(".step").textContent = slot ? slot.label : "";
    if (slot) {
      progress.classList.toggle("determinate", slot.value !== null);
      progress.classList.toggle("failed", Boolean(slot.failed));
      if (slot.value !== null) progress.style.setProperty("--progress", `${Math.round(slot.value * 100)}%`);
    }

    renderMenu(row.querySelector(".row-menu"), addon);
    return fragment;
  }

  // The row menu (task 010): the items `actions.js` gives, each one a call of the data module.
  function renderMenu(details, addon) {
    const menu = details.querySelector(".menu");
    menu.replaceChildren();
    for (const item of Actions.menuItems(addon, Boolean(run))) {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = item.label;
      button.title = item.title;
      button.disabled = !item.enabled;
      button.addEventListener("click", () => {
        details.open = false;
        const flavor = selected();
        if (!flavor) return;
        if (item.action === "update") startRun("one", flavor, addon.folder);
        else markRow(flavor, addon, item);
      });
      menu.append(button);
    }
  }

  function renderLegend() {
    const legend = $("legend");
    legend.replaceChildren();
    for (const state of State.STATES) {
      const item = document.createElement("span");
      item.className = `legend-item ${State.classOf(state)}`;
      const dot = document.createElement("span");
      dot.className = "dot";
      item.append(dot, document.createTextNode(State.labelOf(state)));
      legend.append(item);
    }
  }

  // ---- the actions: search, the update check, "update all", a row's update, the cancel and the marks

  $("search").addEventListener("input", (event) => {
    query = event.target.value;
    const flavor = selected();
    if (flavor) renderRows(flavor);
  });

  // Says an answer's messages in order (the summary last) and draws the flavor it carries.
  function applyActionAnswer(answer) {
    if (answer && answer.flavor) replaceFlavor(answer.flavor);
    for (const message of (answer && answer.messages) || []) say(message);
  }

  // One run of the update actions (task 010): `check`, `all`, or `one` for a row's folder. Each event of the
  // run moves its row's progress slot, lays a check's answer over its row and says its line (progress in
  // the status bar; the stop before the rate limit and a failed update also as a toast). An event that
  // comes after the run's answer belongs to a finished run and is dropped.
  async function startRun(kind, flavor, folder) {
    if (run) return;
    runs += 1;
    const id = runs;
    run = { id, flavorId: flavor.id, flavorName: flavor.name, progress: {} };
    renderNav();
    renderFlavor();
    const progress = (event) => {
      if (!run || run.id !== id) return;
      run.progress = Actions.applyProgress(run.progress, event);
      if (event.check && event.folder) {
        const target = flavors.find((item) => item.id === run.flavorId);
        const index = target ? target.addons.findIndex((addon) => addon.folder === event.folder) : -1;
        if (index >= 0) target.addons[index] = Actions.applyCheck(target.addons[index], event.check);
      }
      if (event.message) say(event.message);
      if (selectedId === run.flavorId) renderFlavor();
    };
    try {
      const answer =
        kind === "check"
          ? await Data.checkUpdates(flavor.id, progress)
          : kind === "one"
            ? await Data.updateOne(flavor.id, folder, progress)
            : await Data.updateAll(flavor.id, progress);
      run = null;
      applyActionAnswer(answer);
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `the update in ${flavor.name} failed: ${error}` });
    } finally {
      run = null;
      renderNav();
      renderFlavor();
    }
  }

  $("check-updates").addEventListener("click", () => {
    const flavor = selected();
    if (flavor) startRun("check", flavor);
  });

  $("update-all").addEventListener("click", () => {
    const flavor = selected();
    if (flavor) startRun("all", flavor);
  });

  $("cancel-update").addEventListener("click", async () => {
    const button = $("cancel-update");
    button.disabled = true;
    try {
      say(await Data.cancelUpdate());
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `the cancel failed: ${error}` });
    } finally {
      button.disabled = false;
    }
  });

  // A row menu's mark (pin, unpin, ignore, unignore, the channel): the answer is the flavor as it is now.
  async function markRow(flavor, addon, item) {
    try {
      applyActionAnswer(await Data.mark(flavor.id, addon.folder, item.action, item.value));
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `${item.action} ${addon.name} failed: ${error}` });
    } finally {
      renderNav();
      renderFlavor();
    }
  }

  // ---- the game folder: the chooser, and the flavors of the folder it answers with

  // Draws an answer of `Data.flavors()` or of a chosen folder: the root, the flavors, the banner. The chosen
  // flavor stays when the new answer still has it.
  function applyAnswer(answer) {
    root = answer.root || null;
    flavors = answer.flavors || [];
    if (!flavors.some((flavor) => flavor.id === selectedId)) selectedId = Flavors.defaultId(flavors);
    $("choose-root").title = root ? `Game folder: ${root}` : "No game folder yet";
    if (answer.message) {
      say(answer.message);
      const shown = heard(answer.message);
      $("empty-murloc").textContent = shown.murloc;
      $("empty-plain").textContent = shown.plain;
    }
    renderNav();
    renderFlavor();
  }

  async function chooseRoot() {
    try {
      const choice = await Data.chooseRoot();
      if (choice && choice.flavors) applyAnswer(choice.flavors);
      else if (choice && choice.message) say(choice.message);
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `choosing the game folder failed: ${error}` });
    }
  }

  $("choose-root").addEventListener("click", chooseRoot);
  $("empty-choose").addEventListener("click", chooseRoot);

  // ---- copy diagnostics: the doctor's report (task 024) onto the clipboard

  async function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      try {
        await navigator.clipboard.writeText(text);
        return true;
      } catch (_) {
        // Falls through to the selection copy below.
      }
    }
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.className = "offscreen";
    document.body.append(area);
    area.select();
    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch (_) {
      copied = false;
    }
    area.remove();
    return copied;
  }

  $("copy-diagnostics").addEventListener("click", async () => {
    const button = $("copy-diagnostics");
    button.disabled = true;
    try {
      const report = await Data.diagnostics();
      const copied = report.text ? await copyText(report.text) : false;
      say(copied ? report.copied : report.failed);
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `the diagnostics failed: ${error}` });
    } finally {
      button.disabled = false;
    }
  });

  // ---- start: the version, then the flavors and the banner

  async function start() {
    try {
      await loadPreferences();
      const [version, answer] = await Promise.all([Data.version(), Data.flavors()]);
      $("version").textContent = `mrgl ${version}`;
      applyAnswer(answer);
    } catch (error) {
      // A failed answer still leaves a readable window: the plain line says what went wrong.
      say({ kind: "error", murloc: "", plain: `the window could not talk to mrgl: ${error}` });
      $("empty").hidden = false;
      $("empty-plain").textContent = `the window could not talk to mrgl: ${error}`;
    }
  }

  start();
})();
