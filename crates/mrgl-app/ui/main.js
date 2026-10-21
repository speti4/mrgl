// The empty window: the banner from Rust in the voice's two lines, and the version in the status bar.
(async function () {
  "use strict";
  const invoke = window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__.core.invoke : null;
  if (!invoke) return;
  const murloc = document.getElementById("murloc");
  const plain = document.getElementById("plain");
  try {
    const [version, banner] = await Promise.all([invoke("app_version"), invoke("banner")]);
    document.getElementById("version").textContent = `mrgl ${version}`;
    const lines = window.Voice.lines(banner);
    const [first, second] = lines.length === 2 ? lines : ["", banner.plain];
    murloc.textContent = first;
    plain.textContent = second;
    document.title = window.Voice.oneLine(banner);
  } catch (error) {
    // A failed command still leaves a readable window: the plain line says what went wrong.
    murloc.textContent = "";
    plain.textContent = `the window could not talk to mrgl: ${error}`;
  }
})();
