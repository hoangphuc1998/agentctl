use axum::{
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MobilePwaAsset {
    pub content_type: &'static str,
    pub body: &'static str,
}

pub fn asset_for_path(path: &str) -> Option<MobilePwaAsset> {
    match path.trim_end_matches('/') {
        "/mobile" => Some(MobilePwaAsset {
            content_type: "text/html; charset=utf-8",
            body: INDEX_HTML,
        }),
        "/mobile/app.js" => Some(MobilePwaAsset {
            content_type: "text/javascript; charset=utf-8",
            body: APP_JS,
        }),
        "/mobile/styles.css" => Some(MobilePwaAsset {
            content_type: "text/css; charset=utf-8",
            body: STYLES_CSS,
        }),
        "/mobile/manifest.webmanifest" => Some(MobilePwaAsset {
            content_type: "application/manifest+json; charset=utf-8",
            body: MANIFEST_JSON,
        }),
        "/mobile/sw.js" => Some(MobilePwaAsset {
            content_type: "text/javascript; charset=utf-8",
            body: SERVICE_WORKER_JS,
        }),
        "/mobile/icon.svg" => Some(MobilePwaAsset {
            content_type: "image/svg+xml; charset=utf-8",
            body: ICON_SVG,
        }),
        _ => None,
    }
}

pub async fn index() -> Response {
    asset_response("/mobile")
}

pub async fn app_js() -> Response {
    asset_response("/mobile/app.js")
}

pub async fn styles_css() -> Response {
    asset_response("/mobile/styles.css")
}

pub async fn manifest() -> Response {
    asset_response("/mobile/manifest.webmanifest")
}

pub async fn service_worker() -> Response {
    asset_response("/mobile/sw.js")
}

pub async fn icon_svg() -> Response {
    asset_response("/mobile/icon.svg")
}

fn asset_response(path: &str) -> Response {
    match asset_for_path(path) {
        Some(asset) => ([(CONTENT_TYPE, asset.content_type)], asset.body).into_response(),
        None => (StatusCode::NOT_FOUND, "mobile asset not found").into_response(),
    }
}

const INDEX_HTML: &str = r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="theme-color" content="#101418">
    <link rel="manifest" href="/mobile/manifest.webmanifest">
    <link rel="icon" href="/mobile/icon.svg" type="image/svg+xml">
    <link rel="stylesheet" href="/mobile/styles.css">
    <title>Agent Manager Mobile</title>
  </head>
  <body>
    <main id="app" class="shell" aria-live="polite"></main>
    <script type="module" src="/mobile/app.js"></script>
  </body>
</html>
"##;

const MANIFEST_JSON: &str = r##"{
  "name": "Agent Manager Mobile",
  "short_name": "Agents",
  "start_url": "/mobile",
  "scope": "/mobile",
  "display": "standalone",
  "background_color": "#f5f1e8",
  "theme_color": "#101418",
  "icons": [
    {
      "src": "/mobile/icon.svg",
      "sizes": "any",
      "type": "image/svg+xml",
      "purpose": "any maskable"
    }
  ]
}
"##;

const SERVICE_WORKER_JS: &str = r#"const CACHE_NAME = "agent-manager-mobile-v1";
const SHELL_ASSETS = [
  "/mobile",
  "/mobile/styles.css",
  "/mobile/app.js",
  "/mobile/manifest.webmanifest",
  "/mobile/icon.svg"
];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(SHELL_ASSETS)));
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)))
    )
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.startsWith("/api/mobile/")) return;
  if (url.pathname.startsWith("/mobile")) {
    event.respondWith(caches.match(event.request).then((cached) => cached || fetch(event.request)));
  }
});
"#;

const ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192" role="img" aria-label="Agent Manager">
  <rect width="192" height="192" rx="38" fill="#101418"/>
  <path d="M52 59h88v74H52z" fill="#f5f1e8"/>
  <path d="M69 82l18 14-18 14M96 116h35" fill="none" stroke="#101418" stroke-width="12" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M146 53l18 18-18 18-18-18z" fill="#19a974"/>
</svg>
"##;

const STYLES_CSS: &str = r#"* {
  box-sizing: border-box;
}

:root {
  color-scheme: light;
  font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
  background: #ede7dc;
  color: #101418;
  --bg: #ede7dc;
  --paper: #fffaf1;
  --paper-strong: #f7efe2;
  --ink: #101418;
  --muted: #5f6b62;
  --line: #d2c7b7;
  --line-strong: #b8ad9d;
  --terminal: #070b0e;
  --terminal-text: #ddf2df;
  --accent: #137a52;
  --accent-soft: #dff5eb;
  --danger: #b42318;
  --shadow: 0 16px 44px rgba(16, 20, 24, 0.14);
}

html,
body {
  min-height: 100%;
}

body {
  margin: 0;
  min-height: 100dvh;
  background:
    linear-gradient(135deg, rgba(19, 122, 82, 0.11), transparent 32rem),
    linear-gradient(315deg, rgba(178, 124, 34, 0.12), transparent 30rem),
    var(--bg);
}

button,
input,
textarea {
  font: inherit;
}

button {
  min-height: 44px;
  border: 0;
  border-radius: 7px;
  padding: 0 14px;
  background: var(--ink);
  color: #fffaf0;
  font-weight: 760;
  cursor: pointer;
}

button.secondary {
  background: #e3dacb;
  color: var(--ink);
}

button.danger {
  background: var(--danger);
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
}

button:focus-visible,
input:focus-visible,
textarea:focus-visible {
  outline: 2px solid rgba(19, 122, 82, 0.62);
  outline-offset: 2px;
}

input,
textarea {
  width: 100%;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  background: rgba(255, 250, 241, 0.92);
  color: var(--ink);
  padding: 12px;
}

textarea {
  min-height: 82px;
  resize: vertical;
}

.shell {
  width: min(100%, 1160px);
  min-height: 100dvh;
  margin: 0 auto;
  padding: max(12px, env(safe-area-inset-top)) 12px max(12px, env(safe-area-inset-bottom));
}

.topbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 0 18px;
}

.kicker {
  margin: 0 0 4px;
  color: var(--muted);
  font-size: 0.74rem;
  font-weight: 820;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

h1,
h2,
h3,
p {
  margin-top: 0;
}

h1 {
  margin-bottom: 4px;
  font-size: clamp(1.45rem, 6vw, 2.45rem);
  line-height: 1;
  letter-spacing: 0;
}

h2 {
  margin-bottom: 3px;
  font-size: 1.12rem;
  line-height: 1.15;
  letter-spacing: 0;
}

.muted {
  color: var(--muted);
  font-size: 0.84rem;
}

.notice {
  border: 1px solid rgba(180, 35, 24, 0.2);
  border-radius: 8px;
  background: #fff1ed;
  box-shadow: 0 10px 30px rgba(16, 20, 24, 0.08);
  color: #7a2518;
  padding: 12px 14px;
}

.panel,
.metric,
.run-row {
  border: 1px solid rgba(16, 20, 24, 0.14);
  border-radius: 8px;
  background: rgba(255, 250, 241, 0.86);
  box-shadow: 0 10px 30px rgba(16, 20, 24, 0.08);
}

.panel {
  padding: 16px;
}

.pair-grid {
  display: grid;
  gap: 10px;
}

.ready-shell {
  height: calc(100dvh - 24px);
  min-height: 520px;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 10px;
}

.ready-topbar {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  border: 1px solid rgba(16, 20, 24, 0.12);
  border-radius: 8px;
  background: rgba(255, 250, 241, 0.9);
  box-shadow: 0 10px 30px rgba(16, 20, 24, 0.08);
  padding: 9px;
}

.ready-title {
  min-width: 0;
}

.ready-title h1 {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ready-actions,
.actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.ready-actions {
  justify-content: flex-end;
}

.ready-actions .secondary {
  min-width: 44px;
  padding: 0 11px;
}

.drawer-toggle {
  min-width: 72px;
}

.ready-main {
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr;
}

.terminal-panel {
  min-width: 0;
  min-height: 0;
  height: 100%;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  overflow: hidden;
  border: 1px solid rgba(16, 20, 24, 0.14);
  border-radius: 8px;
  background: var(--paper);
  box-shadow: var(--shadow);
}

.terminal-header {
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  border-bottom: 1px solid var(--line);
  padding: 10px 12px;
}

.terminal-header h2,
.terminal-header .muted {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.terminal {
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: auto;
  margin: 0;
  border: 0;
  border-radius: 0;
  background: var(--terminal);
  color: var(--terminal-text);
  padding: 13px;
  font: 0.84rem/1.44 "Ubuntu Mono", "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  letter-spacing: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.composer-bar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: end;
  gap: 8px;
  border-top: 1px solid var(--line);
  background: rgba(247, 239, 226, 0.96);
  padding: 10px;
}

.composer-bar label {
  min-width: 0;
  display: grid;
  gap: 5px;
}

.composer-bar textarea {
  min-height: 54px;
  max-height: 30dvh;
}

.mobile-drawer {
  position: fixed;
  inset: 0 auto 0 0;
  z-index: 30;
  width: min(88vw, 360px);
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  gap: 12px;
  overflow: auto;
  transform: translateX(-105%);
  visibility: hidden;
  transition: transform 0.18s ease;
  border-right: 1px solid var(--line);
  background: rgba(247, 239, 226, 0.98);
  box-shadow: var(--shadow);
  padding: max(14px, env(safe-area-inset-top)) 12px max(14px, env(safe-area-inset-bottom));
}

.mobile-drawer.open {
  transform: translateX(0);
  visibility: visible;
}

.drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  background: rgba(16, 20, 24, 0.36);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.18s ease;
}

.drawer-backdrop.open {
  opacity: 1;
  pointer-events: auto;
}

.drawer-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.status-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
}

.metric {
  padding: 9px;
}

.metric strong {
  display: block;
  font-size: 1.28rem;
  line-height: 1;
}

.metric span {
  color: var(--muted);
  font-size: 0.72rem;
  font-weight: 760;
}

.run-list {
  min-width: 0;
  display: grid;
  gap: 8px;
}

.run-row {
  width: 100%;
  min-height: 76px;
  display: grid;
  gap: 6px;
  padding: 11px;
  text-align: left;
  background: rgba(255, 250, 241, 0.9);
  color: var(--ink);
}

.run-row.selected {
  border-color: var(--ink);
  background: #fffef9;
  box-shadow: 0 0 0 1px rgba(16, 20, 24, 0.1) inset;
}

.run-row strong,
.run-row span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.run-row span {
  color: var(--muted);
  font-size: 0.8rem;
}

.run-row-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.badge {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  width: fit-content;
  border-radius: 999px;
  padding: 2px 8px;
  background: var(--accent-soft);
  color: #075f3e;
  font-size: 0.72rem;
  font-weight: 820;
}

@media (min-width: 900px) {
  .ready-shell {
    min-height: 680px;
  }

  .ready-main {
    grid-template-columns: minmax(290px, 0.36fr) minmax(0, 1fr);
    gap: 12px;
  }

  .mobile-drawer {
    position: static;
    z-index: auto;
    width: auto;
    min-height: 0;
    transform: none;
    visibility: visible;
    border: 1px solid rgba(16, 20, 24, 0.14);
    border-radius: 8px;
    box-shadow: 0 10px 30px rgba(16, 20, 24, 0.08);
    padding: 14px;
  }

  .drawer-backdrop,
  .drawer-toggle,
  .mobile-drawer .drawer-header button {
    display: none;
  }
}

@media (max-width: 520px) {
  .shell {
    padding-inline: 8px;
  }

  .ready-shell {
    height: calc(100dvh - 16px);
    min-height: 480px;
    gap: 8px;
  }

  .ready-topbar {
    grid-template-columns: auto minmax(0, 1fr) auto;
    padding: 7px;
  }

  .ready-actions .secondary[data-action="disconnect"] {
    display: none;
  }

  .composer-bar {
    grid-template-columns: 1fr;
  }
}
"#;

const APP_JS: &str = r#"const STORAGE_KEY = "agent-manager-mobile-credentials";
const app = document.getElementById("app");

const state = {
  credentials: readCredentials(),
  dashboard: null,
  selectedRunId: null,
  terminalOutput: "",
  terminalId: null,
  terminalStatus: "idle",
  socket: null,
  drawerOpen: false,
  busy: false,
  error: ""
};

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/mobile/sw.js").catch(() => {});
}

render();
if (state.credentials) {
  loadDashboard();
}

function readCredentials() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "null");
  } catch {
    return null;
  }
}

function saveCredentials(credentials) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(credentials));
  state.credentials = credentials;
}

function clearCredentials() {
  localStorage.removeItem(STORAGE_KEY);
  closeStream();
  state.credentials = null;
  state.dashboard = null;
  state.selectedRunId = null;
  state.terminalOutput = "";
  state.terminalId = null;
  state.terminalStatus = "idle";
  state.drawerOpen = false;
  state.error = "";
  render();
}

function authHeaders() {
  return {
    "Authorization": `Bearer ${state.credentials.deviceToken}`,
    "X-Agent-Manager-Device": state.credentials.deviceId
  };
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(state.credentials ? authHeaders() : {}),
      ...(options.headers || {})
    }
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `HTTP ${response.status}`);
  }
  const text = await response.text();
  return text ? JSON.parse(text) : null;
}

async function pair(event) {
  event.preventDefault();
  const form = new FormData(event.currentTarget);
  const code = String(form.get("code") || "").trim();
  if (!code) return;
  state.busy = true;
  state.error = "";
  render();
  try {
    const paired = await api("/api/mobile/v1/pair/claim", {
      method: "POST",
      body: JSON.stringify({ code, deviceName: navigator.userAgent.includes("Android") ? "Android Chrome" : "Mobile Chrome" })
    });
    saveCredentials({
      baseUrl: location.origin,
      deviceId: paired.id,
      deviceToken: paired.token
    });
    await loadDashboard();
  } catch (error) {
    state.error = errorMessage(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadDashboard(preferredRunId = state.selectedRunId) {
  if (!state.credentials) return;
  state.busy = true;
  state.error = "";
  render();
  try {
    const dashboard = await api("/api/mobile/v1/dashboard");
    state.dashboard = dashboard;
    const runs = allRuns();
    state.selectedRunId = preferredRunId && runs.some((run) => run.id === preferredRunId)
      ? preferredRunId
      : dashboard.selectedRunId || (runs[0] && runs[0].id) || null;
    attachSelectedRun();
  } catch (error) {
    state.error = errorMessage(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function resumeSelectedRun() {
  const run = selectedRun();
  if (!run || !run.restorable) return;
  state.busy = true;
  state.error = "";
  render();
  try {
    await api(`/api/mobile/v1/runs/${encodeURIComponent(run.id)}/resume`, { method: "POST", body: "" });
    await loadDashboard(run.id);
  } catch (error) {
    state.error = errorMessage(error);
  } finally {
    state.busy = false;
    render();
  }
}

function attachSelectedRun() {
  const run = selectedRun();
  closeStream();
  state.terminalOutput = "";
  state.terminalId = null;
  if (!run || !state.credentials) {
    state.terminalStatus = "idle";
    return;
  }
  state.terminalStatus = "connecting";
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  const query = new URLSearchParams({
    deviceId: state.credentials.deviceId,
    token: state.credentials.deviceToken
  });
  const socket = new WebSocket(`${protocol}//${location.host}/api/mobile/v1/stream?${query}`);
  state.socket = socket;
  socket.addEventListener("open", () => {
    socket.send(JSON.stringify({ type: "attachTerminal", runId: run.id, cols: 96, rows: 28 }));
  });
  socket.addEventListener("message", (event) => handleStreamMessage(event.data));
  socket.addEventListener("error", () => {
    if (state.socket === socket) {
      state.terminalStatus = "closed";
      state.error = "Terminal stream error.";
      render();
    }
  });
  socket.addEventListener("close", () => {
    if (state.socket === socket) {
      state.socket = null;
      if (!state.terminalId) {
        state.terminalStatus = "closed";
        state.error = "Terminal stream closed before attach.";
        render();
      }
    }
  });
}

function handleStreamMessage(text) {
  const message = JSON.parse(text);
  if (message.type === "terminalAttached") {
    state.terminalId = message.terminalId;
    state.terminalStatus = "attached";
  }
  if (message.type === "terminalSnapshot") {
    state.terminalOutput = message.data || "";
    state.terminalStatus = "attached";
  }
  if (message.type === "terminalOutput") {
    state.terminalOutput += message.data || "";
    state.terminalStatus = "attached";
  }
  if (message.type === "error") {
    state.error = message.message || "Stream error";
  }
  render();
}

function sendInstruction(event) {
  event.preventDefault();
  const run = selectedRun();
  if (!run || !state.socket || !state.terminalId) return;
  const form = new FormData(event.currentTarget);
  const text = String(form.get("instruction") || "").trimEnd();
  if (!text) return;
  state.socket.send(JSON.stringify({ type: "terminalInput", terminalId: state.terminalId, data: `${text}\n` }));
  event.currentTarget.reset();
}

function closeStream() {
  if (state.socket) {
    const socket = state.socket;
    state.socket = null;
    if (state.terminalId) {
      socket.send(JSON.stringify({ type: "detachTerminal", terminalId: state.terminalId }));
    }
    socket.close();
  }
  state.terminalStatus = "idle";
}

function allRuns() {
  return state.dashboard ? state.dashboard.repos.flatMap((repo) => repo.runs) : [];
}

function selectedRun() {
  return allRuns().find((run) => run.id === state.selectedRunId) || allRuns()[0] || null;
}

function selectRun(id) {
  state.selectedRunId = id;
  state.drawerOpen = false;
  attachSelectedRun();
  render();
}

function toggleDrawer() {
  state.drawerOpen = !state.drawerOpen;
  render();
}

function closeDrawer() {
  state.drawerOpen = false;
  render();
}

function errorMessage(error) {
  return error && error.message ? error.message : String(error || "Unexpected error");
}

function render() {
  app.innerHTML = state.credentials ? readyTemplate() : pairingTemplate();
  bindEvents();
}

function pairingTemplate() {
  return `
    <section class="topbar">
      <div>
        <p class="kicker">Chrome PWA</p>
        <h1>Agent Manager</h1>
        <p class="muted">Pair this browser with the desktop Mobile Bridge.</p>
      </div>
      <button class="secondary" data-action="health">Check bridge</button>
    </section>
    ${state.error ? `<div class="notice">${escapeHtml(state.error)}</div>` : ""}
    <section class="panel">
      <h2>Pair browser</h2>
      <form class="pair-grid" data-form="pair">
        <label>
          <span class="muted">Desktop pairing code</span>
          <input name="code" autocomplete="one-time-code" inputmode="text" placeholder="ABCD1234" required>
        </label>
        <button ${state.busy ? "disabled" : ""}>${state.busy ? "Pairing..." : "Pair browser"}</button>
      </form>
    </section>
  `;
}

function readyTemplate() {
  const dashboard = state.dashboard;
  const run = selectedRun();
  const runs = allRuns();
  return `
    <section class="ready-shell">
      <header class="ready-topbar">
        <button
          class="drawer-toggle"
          data-action="toggle-drawer"
          aria-controls="run-drawer"
          aria-expanded="${state.drawerOpen ? "true" : "false"}"
        >Runs</button>
        <div class="ready-title">
          <p class="kicker">Mobile Bridge</p>
          <h1>${run ? escapeHtml(run.runName) : "Agent Manager"}</h1>
          <p class="muted">${run ? `${escapeHtml(run.repoName)} / ${escapeHtml(run.observedState)}` : escapeHtml(location.host)}</p>
        </div>
        <div class="ready-actions">
          <button class="secondary" data-action="refresh" ${state.busy ? "disabled" : ""}>Refresh</button>
          <button class="secondary" data-action="disconnect">Disconnect</button>
        </div>
      </header>
      ${state.error ? `<div class="notice">${escapeHtml(state.error)}</div>` : ""}
      <div class="ready-main">
        <aside
          id="run-drawer"
          class="mobile-drawer${state.drawerOpen ? " open" : ""}"
          aria-label="Runs drawer"
        >
          <div class="drawer-header">
            <div>
              <p class="kicker">Runs</p>
              <h2>Work queue</h2>
              <p class="muted">${runs.length} available</p>
            </div>
            <button class="secondary" data-action="close-drawer">Close</button>
          </div>
          ${dashboard ? metricsTemplate(dashboard) : ""}
          <div class="run-list">
            ${runs.map((item) => runRowTemplate(item)).join("") || `<p class="muted">No active runs.</p>`}
          </div>
        </aside>
        <section class="terminal-panel">
          ${run ? selectedRunTemplate(run) : `<div class="terminal-header"><div><h2>No run selected</h2><p class="muted">Create or resume a run from desktop.</p></div></div><pre class="terminal" aria-label="Terminal output">Waiting for terminal output...</pre>`}
        </section>
      </div>
      <div class="drawer-backdrop${state.drawerOpen ? " open" : ""}" data-action="close-drawer" aria-hidden="true"></div>
    </section>
  `;
}

function metricsTemplate(dashboard) {
  return `
    <section class="status-strip" aria-label="Run summary">
      <div class="metric"><strong>${dashboard.activeCount}</strong><span>active</span></div>
      <div class="metric"><strong>${dashboard.attentionCount}</strong><span>attention</span></div>
      <div class="metric"><strong>${dashboard.restorableCount}</strong><span>restorable</span></div>
    </section>
  `;
}

function runRowTemplate(run) {
  const selected = run.id === state.selectedRunId ? " selected" : "";
  return `
    <button class="run-row${selected}" data-run-id="${escapeHtml(run.id)}" aria-current="${selected ? "true" : "false"}">
      <strong>${escapeHtml(run.runName)}</strong>
      <span>${escapeHtml(run.repoName)}</span>
      <span class="run-row-meta">
        <span>${escapeHtml(run.agent)}</span>
        <span>${escapeHtml(run.observedState)}</span>
      </span>
      ${run.restorable ? `<span class="badge">restorable</span>` : ""}
    </button>
  `;
}

function selectedRunTemplate(run) {
  return `
    <div class="terminal-header">
      <div>
        <h2>${escapeHtml(run.runName)}</h2>
        <p class="muted">${escapeHtml(run.repoName)} / ${escapeHtml(run.branch)}</p>
      </div>
      ${run.restorable ? `<button data-action="resume" ${state.busy ? "disabled" : ""}>Resume</button>` : ""}
    </div>
    <pre class="terminal" aria-label="Terminal output">${escapeHtml(terminalText())}</pre>
    <form class="composer-bar" data-form="instruction">
      <label>
        <span class="muted">Instruction</span>
        <textarea name="instruction" placeholder="Send instructions to the selected agent"></textarea>
      </label>
      <button ${state.terminalId ? "" : "disabled"}>Send</button>
    </form>
  `;
}

function terminalText() {
  if (state.terminalOutput) return state.terminalOutput;
  if (state.terminalStatus === "connecting") return "Connecting terminal stream...";
  if (state.terminalStatus === "attached") return "Terminal attached. Waiting for output...";
  if (state.terminalStatus === "closed") return "Terminal stream closed before attach.";
  return "Waiting for terminal output...";
}

function bindEvents() {
  app.querySelector('[data-form="pair"]')?.addEventListener("submit", pair);
  app.querySelector('[data-form="instruction"]')?.addEventListener("submit", sendInstruction);
  app.querySelector('[data-action="refresh"]')?.addEventListener("click", () => loadDashboard());
  app.querySelector('[data-action="disconnect"]')?.addEventListener("click", clearCredentials);
  app.querySelector('[data-action="resume"]')?.addEventListener("click", resumeSelectedRun);
  app.querySelector('[data-action="toggle-drawer"]')?.addEventListener("click", toggleDrawer);
  app.querySelectorAll('[data-action="close-drawer"]').forEach((element) => {
    element.addEventListener("click", closeDrawer);
  });
  app.querySelector('[data-action="health"]')?.addEventListener("click", async () => {
    state.error = "";
    try {
      await api("/api/mobile/v1/health");
      state.error = "Bridge health check passed. Generate a pairing code on desktop.";
    } catch (error) {
      state.error = errorMessage(error);
    }
    render();
  });
  app.querySelectorAll("[data-run-id]").forEach((button) => {
    button.addEventListener("click", () => selectRun(button.getAttribute("data-run-id")));
  });
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_shell_links_manifest_styles_and_script() {
        let shell = asset_for_path("/mobile").expect("mobile shell should be served");

        assert_eq!(shell.content_type, "text/html; charset=utf-8");
        assert!(shell
            .body
            .contains(r#"href="/mobile/manifest.webmanifest""#));
        assert!(shell.body.contains(r#"href="/mobile/styles.css""#));
        assert!(shell.body.contains(r#"src="/mobile/app.js""#));
        assert!(shell.body.contains("Agent Manager Mobile"));
    }

    #[test]
    fn mobile_script_uses_bridge_pairing_dashboard_resume_and_stream_endpoints() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert_eq!(script.content_type, "text/javascript; charset=utf-8");
        assert!(script.body.contains("/api/mobile/v1/pair/claim"));
        assert!(script.body.contains("/api/mobile/v1/dashboard"));
        assert!(script.body.contains("/api/mobile/v1/runs/"));
        assert!(script.body.contains("/api/mobile/v1/stream?"));
        assert!(script.body.contains("deviceId"));
        assert!(script.body.contains("token"));
    }

    #[test]
    fn mobile_script_reports_terminal_stream_connection_states() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("Connecting terminal stream..."));
        assert!(script
            .body
            .contains("Terminal stream closed before attach."));
        assert!(script.body.contains("Terminal stream error."));
    }

    #[test]
    fn mobile_styles_make_phone_layout_terminal_first_with_drawer_navigation() {
        let styles = asset_for_path("/mobile/styles.css").expect("mobile styles should be served");

        assert!(styles.body.contains(".ready-shell"));
        assert!(styles
            .body
            .contains("grid-template-rows: auto minmax(0, 1fr);"));
        assert!(styles.body.contains(".terminal-panel"));
        assert!(styles.body.contains("height: 100%;"));
        assert!(styles.body.contains(".mobile-drawer"));
        assert!(styles.body.contains(".drawer-backdrop"));
        assert!(styles.body.contains("@media (min-width: 900px)"));
    }

    #[test]
    fn mobile_script_renders_accessible_drawer_and_terminal_first_content() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("drawerOpen: false"));
        assert!(script.body.contains(r#"data-action="toggle-drawer""#));
        assert!(script
            .body
            .contains(r#"aria-expanded="${state.drawerOpen ? "true" : "false"}""#));
        assert!(script
            .body
            .contains(r#"class="mobile-drawer${state.drawerOpen ? " open" : ""}""#));
        assert!(script.body.contains(r#"data-action="close-drawer""#));
        assert!(script.body.contains(r#"class="terminal-panel""#));
        assert!(script.body.contains(r#"class="composer-bar""#));
    }

    #[test]
    fn selecting_a_mobile_run_closes_the_drawer_before_attaching_terminal() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("state.drawerOpen = false;"));
        assert!(script.body.contains("attachSelectedRun();"));
        assert!(script.body.contains("function toggleDrawer()"));
    }
}
