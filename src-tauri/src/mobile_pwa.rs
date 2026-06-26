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
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  background: #f5f1e8;
  color: #101418;
}

body {
  margin: 0;
  min-height: 100vh;
  background:
    linear-gradient(135deg, rgba(25, 169, 116, 0.12), transparent 32rem),
    linear-gradient(315deg, rgba(230, 172, 0, 0.16), transparent 28rem),
    #f5f1e8;
}

button,
input,
textarea {
  font: inherit;
}

button {
  min-height: 42px;
  border: 0;
  border-radius: 7px;
  padding: 0 14px;
  background: #101418;
  color: #fffaf0;
  font-weight: 700;
}

button.secondary {
  background: #e7e0d2;
  color: #101418;
}

button.danger {
  background: #b42318;
}

button:disabled {
  opacity: 0.48;
}

input,
textarea {
  width: 100%;
  border: 1px solid #b8b1a2;
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.72);
  color: #101418;
  padding: 12px;
}

textarea {
  min-height: 84px;
  resize: vertical;
}

.shell {
  width: min(100%, 980px);
  margin: 0 auto;
  padding: max(18px, env(safe-area-inset-top)) 16px max(22px, env(safe-area-inset-bottom));
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
  color: #52605a;
  font-size: 0.78rem;
  font-weight: 800;
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
  font-size: clamp(1.6rem, 7vw, 2.6rem);
  line-height: 1;
}

.status-strip {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin: 0 0 14px;
}

.metric,
.panel,
.run-row,
.terminal,
.notice {
  border: 1px solid rgba(16, 20, 24, 0.16);
  border-radius: 8px;
  background: rgba(255, 252, 246, 0.78);
  box-shadow: 0 10px 30px rgba(16, 20, 24, 0.08);
}

.metric {
  padding: 10px;
}

.metric strong {
  display: block;
  font-size: 1.35rem;
}

.metric span {
  color: #52605a;
  font-size: 0.76rem;
  font-weight: 700;
}

.panel {
  padding: 16px;
  margin-bottom: 14px;
}

.pair-grid,
.composer {
  display: grid;
  gap: 10px;
}

.notice {
  padding: 12px 14px;
  margin-bottom: 14px;
  color: #7a2518;
  background: #fff1ed;
}

.layout {
  display: grid;
  grid-template-columns: minmax(0, 0.82fr) minmax(0, 1.18fr);
  gap: 14px;
}

.run-list {
  display: grid;
  gap: 8px;
}

.run-row {
  width: 100%;
  min-height: 74px;
  display: block;
  padding: 12px;
  text-align: left;
  background: rgba(255, 252, 246, 0.82);
  color: #101418;
}

.run-row.selected {
  border-color: #101418;
  background: #fffaf0;
}

.run-row strong {
  display: block;
  margin-bottom: 4px;
}

.run-row span,
.muted {
  color: #52605a;
  font-size: 0.84rem;
}

.badge {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  margin-top: 8px;
  border-radius: 999px;
  padding: 2px 8px;
  background: #dff5eb;
  color: #075f3e;
  font-size: 0.72rem;
  font-weight: 800;
}

.terminal {
  min-height: 42vh;
  max-height: 54vh;
  overflow: auto;
  padding: 12px;
  background: #101418;
  color: #dff5eb;
  font: 0.82rem/1.45 "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  white-space: pre-wrap;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.toolbar h2 {
  margin-bottom: 2px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

@media (max-width: 760px) {
  .topbar,
  .layout {
    display: grid;
    grid-template-columns: 1fr;
  }

  .status-strip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
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
  state.terminalStatus = "connecting";
  if (!run || !state.credentials) return;
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
  attachSelectedRun();
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
    <section class="topbar">
      <div>
        <p class="kicker">Mobile Bridge</p>
        <h1>Agent Manager</h1>
        <p class="muted">${escapeHtml(location.host)}</p>
      </div>
      <div class="actions">
        <button class="secondary" data-action="refresh" ${state.busy ? "disabled" : ""}>Refresh</button>
        <button class="secondary" data-action="disconnect">Disconnect</button>
      </div>
    </section>
    ${state.error ? `<div class="notice">${escapeHtml(state.error)}</div>` : ""}
    ${dashboard ? metricsTemplate(dashboard) : ""}
    <section class="layout">
      <div class="panel">
        <div class="toolbar">
          <div>
            <h2>Runs</h2>
            <p class="muted">${runs.length} available</p>
          </div>
        </div>
        <div class="run-list">
          ${runs.map((item) => runRowTemplate(item)).join("") || `<p class="muted">No active runs.</p>`}
        </div>
      </div>
      <div class="panel">
        ${run ? selectedRunTemplate(run) : `<h2>No run selected</h2><p class="muted">Create or resume a run from desktop.</p>`}
      </div>
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
    <button class="run-row${selected}" data-run-id="${escapeHtml(run.id)}">
      <strong>${escapeHtml(run.runName)}</strong>
      <span>${escapeHtml(run.repoName)} / ${escapeHtml(run.agent)} / ${escapeHtml(run.observedState)}</span>
      ${run.restorable ? `<span class="badge">restorable</span>` : ""}
    </button>
  `;
}

function selectedRunTemplate(run) {
  return `
    <div class="toolbar">
      <div>
        <h2>${escapeHtml(run.runName)}</h2>
        <p class="muted">${escapeHtml(run.repoName)} / ${escapeHtml(run.branch)}</p>
      </div>
      ${run.restorable ? `<button data-action="resume" ${state.busy ? "disabled" : ""}>Resume</button>` : ""}
    </div>
    <pre class="terminal" aria-label="Terminal output">${escapeHtml(terminalText())}</pre>
    <form class="composer" data-form="instruction">
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
}
