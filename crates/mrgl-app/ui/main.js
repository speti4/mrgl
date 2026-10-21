// The view (task 017): the flavor navigation, the addon list, the search, "update all", the voice's two
// places and the theme. It reads `window.Data` and draws what it gets; it does not know whether the rows
// are made up (`mock-data.js`) or the core's answers (task 009), and it never composes a murloc line.
(function () {
  "use strict";

  const Voice = window.Voice;
  const State = window.State;
  const Search = window.Search;
  const Theme = window.Theme;
  const Flavors = window.Flavors;
  const Data = window.Data;

  const $ = (id) => document.getElementById(id);

  // ---- preferences: the theme and the serious switch, remembered per machine until the shell has settings

  const store = {
    get(key) {
      try {
        return window.localStorage.getItem(key);
      } catch (_) {
        return null;
      }
    },
    set(key, value) {
      try {
        window.localStorage.setItem(key, value);
      } catch (_) {
        // A webview without storage still has a theme for this run.
      }
    },
  };

  const systemDark = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  let theme = Theme.normalize(store.get("theme"));
  let serious = store.get("serious") === "1";

  function applyTheme() {
    const dark = Boolean(systemDark && systemDark.matches);
    document.documentElement.dataset.theme = Theme.resolve(theme, dark);
    $("theme-toggle").textContent = Theme.label(theme, dark);
  }

  $("theme-toggle").addEventListener("click", () => {
    theme = Theme.next(theme);
    store.set("theme", theme);
    applyTheme();
  });
  if (systemDark && systemDark.addEventListener) systemDark.addEventListener("change", applyTheme);
  applyTheme();

  const seriousToggle = $("serious-toggle");
  seriousToggle.checked = serious;
  seriousToggle.addEventListener("change", () => {
    serious = seriousToggle.checked;
    store.set("serious", serious ? "1" : "0");
    if (lastMessage) showStatus(lastMessage);
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
  let busy = false;

  function selected() {
    return flavors.find((flavor) => flavor.id === selectedId) || null;
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
    if (flavor.version) meta.push(`build ${flavor.version}`);
    if (flavor.interface) meta.push(`interface ${flavor.interface}`);
    if (flavor.curse_client) meta.push("managed by the CurseForge app");
    $("flavor-meta").textContent = meta.join(" · ");

    const warnings = $("flavor-warnings");
    warnings.hidden = !(flavor.warnings && flavor.warnings.length);
    warnings.textContent = (flavor.warnings || []).join("; ");

    const updates = State.countUpdates(flavor.addons);
    const button = $("update-all");
    button.textContent = updates > 0 ? `Update all (${updates})` : "Update all";
    button.disabled = busy;

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
      flavor.addons.length === 0 ? `no addons in ${flavor.folder}` : `no addon matches "${query.trim()}" in ${flavor.name}`;
  }

  function renderRow(addon) {
    const fragment = $("row-template").content.cloneNode(true);
    const row = fragment.querySelector(".row");
    const state = State.stateOf(addon);
    row.classList.add(State.classOf(state));
    row.classList.toggle("ignored", Boolean(addon.ignored));
    row.classList.toggle("busy", busy && Boolean(addon.update) && !addon.pinned && !addon.ignored);
    row.dataset.folder = addon.folder;

    row.querySelector(".name").textContent = addon.name || addon.folder;
    row.querySelector(".folder").textContent = addon.name && addon.name !== addon.folder ? addon.folder : "";

    row.querySelector(".version-now").textContent = addon.version || "—";
    row.querySelector(".version-next").textContent = addon.update ? `→ ${addon.update.version}` : "";
    const note = [];
    if (addon.pinned) note.push(`pinned at ${addon.pinned}`);
    if (addon.ignored) note.push("ignored");
    row.querySelector(".version-note").textContent = note.join(", ");

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
      forever.classList.add(`forever-${addon.forever.build}`);
      forever.textContent = addon.forever.note ? `${addon.forever.build}: ${addon.forever.note}` : addon.forever.build;
      forever.title = `Forever build: ${forever.textContent}`;
    } else {
      forever.textContent = "";
    }

    row.querySelector(".progress").hidden = !row.classList.contains("busy");
    return fragment;
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

  // ---- the actions: search and "update all"

  $("search").addEventListener("input", (event) => {
    query = event.target.value;
    const flavor = selected();
    if (flavor) renderRows(flavor);
  });

  $("update-all").addEventListener("click", async () => {
    const flavor = selected();
    if (!flavor || busy) return;
    busy = true;
    renderFlavor();
    try {
      await Data.updateAll(flavor.id, (message) => {
        say(message);
        renderNav();
        renderFlavor();
      });
    } catch (error) {
      say({ kind: "error", murloc: "", plain: `update all failed in ${flavor.name}: ${error}` });
    } finally {
      busy = false;
      renderNav();
      renderFlavor();
    }
  });

  // ---- start: the version, then the flavors and the banner

  async function start() {
    try {
      const [version, answer] = await Promise.all([Data.version(), Data.flavors()]);
      $("version").textContent = `mrgl ${version}`;
      root = answer.root;
      flavors = answer.flavors || [];
      selectedId = Flavors.defaultId(flavors);
      if (answer.message) {
        say(answer.message);
        const shown = heard(answer.message);
        $("empty-murloc").textContent = shown.murloc;
        $("empty-plain").textContent = shown.plain;
      }
      renderNav();
      renderFlavor();
    } catch (error) {
      // A failed answer still leaves a readable window: the plain line says what went wrong.
      say({ kind: "error", murloc: "", plain: `the window could not talk to mrgl: ${error}` });
      $("empty").hidden = false;
      $("empty-plain").textContent = `the window could not talk to mrgl: ${error}`;
    }
  }

  start();
})();
