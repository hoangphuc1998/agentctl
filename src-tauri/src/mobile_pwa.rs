use axum::{
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        StatusCode,
    },
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
        "/mobile/reset" => Some(MobilePwaAsset {
            content_type: "text/html; charset=utf-8",
            body: RESET_HTML,
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

pub async fn reset() -> Response {
    asset_response("/mobile/reset")
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

macro_rules! mobile_pwa_asset_version {
    () => {
        "v6"
    };
}

pub const MOBILE_PWA_ASSET_VERSION: &str = mobile_pwa_asset_version!();

fn asset_response(path: &str) -> Response {
    match asset_for_path(path) {
        Some(asset) => (
            [
                (CONTENT_TYPE, asset.content_type),
                (CACHE_CONTROL, "no-store"),
            ],
            asset.body,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "mobile asset not found").into_response(),
    }
}

const INDEX_HTML: &str = concat!(
    r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="theme-color" content="#101418">
    <link rel="manifest" href="/mobile/manifest.webmanifest">
    <link rel="icon" href="/mobile/icon.svg" type="image/svg+xml">
    <link rel="stylesheet" href="/mobile/styles.css?v="##,
    mobile_pwa_asset_version!(),
    r##"">
    <title>Agent Manager Mobile</title>
  </head>
  <body>
    <main id="app" class="shell" aria-live="polite" data-mobile-version=""##,
    mobile_pwa_asset_version!(),
    r##""></main>
    <script type="module" src="/mobile/app.js?v="##,
    mobile_pwa_asset_version!(),
    r##""></script>
  </body>
</html>
"##
);

const RESET_HTML: &str = concat!(
    r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="theme-color" content="#101418">
    <title>Resetting Agent Manager Mobile</title>
    <style>
      * { box-sizing: border-box; }
      :root {
        font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, sans-serif;
        background: #e7eceb;
        color: #101418;
      }
      body {
        min-height: 100dvh;
        display: grid;
        place-items: center;
        margin: 0;
        padding: 18px;
      }
      main {
        width: min(480px, 100%);
        border: 1px solid #c4cfcb;
        border-radius: 8px;
        background: #fbf8f1;
        padding: 20px;
        box-shadow: 0 16px 44px rgba(16, 20, 24, 0.14);
      }
      .kicker {
        margin: 0 0 6px;
        color: #59666c;
        font-size: 0.72rem;
        font-weight: 820;
        letter-spacing: 0.08em;
        text-transform: uppercase;
      }
      h1 {
        margin: 0 0 10px;
        font-size: 2rem;
        line-height: 1;
      }
      p {
        margin: 0;
        color: #59666c;
        line-height: 1.45;
      }
    </style>
  </head>
  <body>
    <main data-mobile-version=""##,
    mobile_pwa_asset_version!(),
    r##"">
      <p class="kicker">Mobile Bridge</p>
      <h1>Resetting Agent Manager Mobile</h1>
      <p id="status">Clearing installed PWA assets and service worker...</p>
    </main>
    <script>
      (async () => {
        const status = document.getElementById("status");
        try {
          if ("serviceWorker" in navigator) {
            const registrations = await navigator.serviceWorker.getRegistrations();
            await Promise.all(registrations.map((registration) => registration.unregister()));
          }
          if ("caches" in window) {
            const keys = await caches.keys();
            await Promise.all(keys.map((key) => caches.delete(key)));
          }
          status.textContent = "Reset complete. Reloading the mobile bridge...";
        } catch (error) {
          status.textContent = `Reset hit an error, reloading anyway: ${error && error.message ? error.message : error}`;
        } finally {
          location.replace("/mobile?resetComplete=1&v="##,
    mobile_pwa_asset_version!(),
    r##"");
        }
      })();
    </script>
  </body>
</html>
"##
);

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

const SERVICE_WORKER_JS: &str = concat!(
    r#"const CACHE_NAME = "agent-manager-mobile-"#,
    mobile_pwa_asset_version!(),
    r#"";
const SHELL_ASSETS = [
  "/mobile",
  "/mobile/reset",
  "/mobile/styles.css?v="#,
    mobile_pwa_asset_version!(),
    r#"",
  "/mobile/app.js?v="#,
    mobile_pwa_asset_version!(),
    r#"",
  "/mobile/manifest.webmanifest",
  "/mobile/icon.svg"
];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(SHELL_ASSETS)));
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys();
    await Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)));
    await self.clients.claim();
    const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    await Promise.all(clients.map((client) => {
      const url = new URL(client.url);
      return url.pathname.startsWith("/mobile") ? client.navigate(client.url) : undefined;
    }));
  })());
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.pathname.startsWith("/api/mobile/")) return;
  if (url.pathname.startsWith("/mobile")) {
    event.respondWith(
      caches.open(CACHE_NAME).then((cache) =>
        fetch(event.request).then((response) => {
          if (response.ok) {
            cache.put(event.request, response.clone());
          }
          return response;
        }).catch(() =>
          caches.match(event.request).then((cached) => cached || Response.error())
        )
      )
    );
  }
});
"#
);

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
  background: #e7eceb;
  color: #101418;
  --bg: #e7eceb;
  --paper: #fbf8f1;
  --paper-strong: #eaf0ed;
  --ink: #101418;
  --muted: #59666c;
  --line: #c4cfcb;
  --line-strong: #a9b7b2;
  --terminal: #070b0e;
  --terminal-text: #ddf2df;
  --accent: #0f766e;
  --accent-soft: #d9f3ee;
  --accent-cool: #2f5d9f;
  --accent-warm: #b26b2d;
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
    linear-gradient(135deg, rgba(15, 118, 110, 0.13), transparent 32rem),
    linear-gradient(315deg, rgba(47, 93, 159, 0.11), transparent 30rem),
    linear-gradient(180deg, rgba(255, 255, 255, 0.62), transparent 18rem),
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
  background: #dbe5e2;
  color: var(--ink);
}

a {
  color: var(--accent);
  font-weight: 760;
  text-decoration-thickness: 0.08em;
  text-underline-offset: 0.18em;
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

.pwa-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 7px;
  margin: 4px 0 0;
}

.asset-version {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border: 1px solid rgba(89, 102, 108, 0.28);
  border-radius: 999px;
  background: rgba(234, 240, 237, 0.82);
  color: var(--muted);
  padding: 2px 8px;
  font-size: 0.72rem;
  font-weight: 760;
  white-space: nowrap;
}

.reset-link {
  font-size: 0.78rem;
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
  background: linear-gradient(180deg, #fbf8f1, #eef3ef);
  contain: layout paint style;
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

.terminal-status-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.stream-status {
  min-height: 26px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid rgba(15, 118, 110, 0.3);
  border-radius: 999px;
  background: rgba(217, 243, 238, 0.9);
  color: #075f58;
  padding: 3px 9px;
  font-size: 0.72rem;
  font-weight: 820;
  white-space: nowrap;
}

.stream-status::before {
  content: "";
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: currentColor;
  box-shadow: 0 0 0 3px rgba(15, 118, 110, 0.13);
}

.stream-status.idle,
.stream-status.closed {
  border-color: rgba(89, 102, 108, 0.32);
  background: rgba(234, 240, 237, 0.92);
  color: var(--muted);
}

.mini-button {
  min-width: 54px;
  min-height: 38px;
  display: inline-grid;
  place-items: center;
  border: 1px solid rgba(16, 20, 24, 0.12);
  background: #f5f1e8;
  color: var(--ink);
  padding: 0 10px;
  font-size: 0.78rem;
}

.terminal {
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: auto;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
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

.terminal::selection {
  background: rgba(76, 154, 255, 0.32);
  color: #f7fffb;
}

.composer-bar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: end;
  gap: 8px;
  border-top: 1px solid var(--line);
  background: rgba(234, 240, 237, 0.96);
  padding: 10px;
}

.composer-bar:focus-within {
  border-top-color: rgba(15, 118, 110, 0.55);
  box-shadow: 0 -1px 0 rgba(15, 118, 110, 0.2);
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

.composer-actions {
  display: flex;
  align-items: end;
  gap: 8px;
}

.choice-panel {
  border-top: 1px solid var(--line);
  background: rgba(234, 240, 237, 0.98);
  padding: 10px;
}

.choice-list {
  display: grid;
  gap: 8px;
}

.choice-button,
.key-button {
  min-height: 46px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(15, 118, 110, 0.28);
  background: #f7fff9;
  color: var(--ink);
  font-weight: 700;
}

.choice-button {
  justify-content: flex-start;
  text-align: left;
  padding: 10px 12px;
}

.key-bar {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 7px;
}

.key-button {
  padding: 0 8px;
}

.choice-panel-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
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

  .composer-actions {
    justify-content: flex-end;
  }
}
"#;

const APP_JS: &str = r#"const STORAGE_KEY = "agent-manager-mobile-credentials";
const TAIL_LOCK_THRESHOLD = 48;
const MOBILE_PWA_VERSION = "v6";
const app = document.getElementById("app");

const state = {
  credentials: readCredentials(),
  dashboard: null,
  selectedRunId: null,
  terminalOutput: "",
  terminalId: null,
  attachedRunId: null,
  terminalStatus: "idle",
  composerOverridePromptSignature: "",
  pendingTerminalOutput: "",
  pendingTerminalReplace: false,
  pendingTerminalControlRefresh: false,
  terminalFlushScheduled: false,
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
  state.attachedRunId = null;
  state.composerOverridePromptSignature = "";
  state.pendingTerminalOutput = "";
  state.pendingTerminalReplace = false;
  state.pendingTerminalControlRefresh = false;
  state.terminalFlushScheduled = false;
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
  render({ preserveTerminalScroll: true });
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
    render({ preserveTerminalScroll: true });
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
  if (selectedRunAlreadyAttached(run)) return;
  closeStream();
  state.terminalOutput = "";
  state.terminalId = null;
  state.attachedRunId = null;
  state.composerOverridePromptSignature = "";
  state.pendingTerminalOutput = "";
  state.pendingTerminalReplace = false;
  state.pendingTerminalControlRefresh = false;
  if (!run || !state.credentials) {
    state.terminalStatus = "idle";
    return;
  }
  state.attachedRunId = run.id;
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

function selectedRunAlreadyAttached(run) {
  if (!run || !state.socket) return false;
  if (state.attachedRunId !== run.id) return false;
  return state.socket.readyState === WebSocket.CONNECTING || state.socket.readyState === WebSocket.OPEN;
}

function handleStreamMessage(text) {
  const message = JSON.parse(text);
  if (message.type === "terminalAttached") {
    state.terminalId = message.terminalId;
    state.terminalStatus = "attached";
    render({ preserveTerminalScroll: true });
    return;
  }
  if (message.type === "terminalSnapshot") {
    state.terminalStatus = "attached";
    setTerminalOutput(message.data || "");
    return;
  }
  if (message.type === "terminalOutput") {
    state.terminalStatus = "attached";
    queueTerminalOutput(message.data || "");
    return;
  }
  if (message.type === "terminalClosed") {
    state.terminalId = null;
    state.terminalStatus = "closed";
    render({ preserveTerminalScroll: true });
    return;
  }
  if (message.type === "error") {
    state.error = message.message || "Stream error";
    render({ preserveTerminalScroll: true });
  }
}

function sendInstruction(event) {
  event.preventDefault();
  const form = new FormData(event.currentTarget);
  const text = String(form.get("instruction") || "").trimEnd();
  if (!text) return;
  sendTerminalInput(`${text}\n`);
  event.currentTarget.reset();
}

function sendTerminalInput(data) {
  if (!selectedRun() || !state.socket || !state.terminalId || !data) return;
  state.socket.send(JSON.stringify({ type: "terminalInput", terminalId: state.terminalId, data }));
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
  state.attachedRunId = null;
  state.terminalStatus = "idle";
}

function terminalOutputElement() {
  return app.querySelector("[data-terminal-output]");
}

function setTerminalOutput(data) {
  const previousPromptSignature = terminalPromptSignature(state.terminalOutput);
  state.pendingTerminalOutput = "";
  state.pendingTerminalReplace = false;
  state.pendingTerminalControlRefresh = false;
  state.terminalOutput = data;
  if (terminalControlsChanged(previousPromptSignature)) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const terminal = terminalOutputElement();
  if (!terminal) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const shouldFollow = shouldFollowTerminalTail(terminal);
  terminal.textContent = terminalText();
  updateTerminalScroll(terminal, shouldFollow);
  updateSendButton();
}

function appendTerminalOutput(data) {
  if (!data) return;
  const previousPromptSignature = terminalPromptSignature(state.terminalOutput);
  const hadOutput = Boolean(state.terminalOutput);
  state.terminalOutput += data;
  if (terminalControlsChanged(previousPromptSignature)) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const terminal = terminalOutputElement();
  if (!terminal) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const shouldFollow = shouldFollowTerminalTail(terminal);
  if (!hadOutput) terminal.textContent = "";
  terminal.append(document.createTextNode(data));
  updateTerminalScroll(terminal, shouldFollow);
  updateSendButton();
}

function queueTerminalOutput(data) {
  if (!data) return;
  const previousPromptSignature = terminalPromptSignature(state.terminalOutput);
  const hadOutput = Boolean(state.terminalOutput);
  state.terminalOutput += data;
  state.pendingTerminalOutput += data;
  state.pendingTerminalReplace = state.pendingTerminalReplace || !hadOutput;
  state.pendingTerminalControlRefresh = state.pendingTerminalControlRefresh || terminalControlsChanged(previousPromptSignature);
  if (!state.terminalFlushScheduled) {
    state.terminalFlushScheduled = true;
    requestAnimationFrame(flushTerminalOutput);
  }
}

function flushTerminalOutput() {
  state.terminalFlushScheduled = false;
  const data = state.pendingTerminalOutput;
  const replace = state.pendingTerminalReplace;
  const refreshControls = state.pendingTerminalControlRefresh;
  state.pendingTerminalOutput = "";
  state.pendingTerminalReplace = false;
  state.pendingTerminalControlRefresh = false;
  if (!data) return;
  if (refreshControls) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const terminal = terminalOutputElement();
  if (!terminal) {
    render({ preserveTerminalScroll: true });
    return;
  }
  const shouldFollow = shouldFollowTerminalTail(terminal);
  if (replace) terminal.textContent = "";
  terminal.append(document.createTextNode(data));
  updateTerminalScroll(terminal, shouldFollow);
  updateSendButton();
}

function shouldFollowTerminalTail(terminal) {
  return terminal.scrollHeight - terminal.clientHeight - terminal.scrollTop <= TAIL_LOCK_THRESHOLD;
}

function updateTerminalScroll(terminal, shouldFollow) {
  if (shouldFollow) {
    terminal.scrollTop = terminal.scrollHeight;
  }
}

function updateSendButton() {
  const sendButton = app.querySelector("[data-send-instruction]");
  if (sendButton) sendButton.disabled = !state.terminalId;
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

function captureTerminalScroll() {
  const terminal = terminalOutputElement();
  if (!terminal) return null;
  return {
    runId: state.selectedRunId,
    top: terminal.scrollTop,
    shouldFollow: shouldFollowTerminalTail(terminal)
  };
}

function restoreTerminalScroll(snapshot) {
  if (!snapshot || snapshot.runId !== state.selectedRunId) return;
  const terminal = terminalOutputElement();
  if (!terminal) return;
  updateTerminalScroll(terminal, snapshot.shouldFollow);
  if (!snapshot.shouldFollow) {
    terminal.scrollTop = snapshot.top;
  }
}

function render(options = {}) {
  const terminalScroll = options.preserveTerminalScroll ? captureTerminalScroll() : null;
  app.innerHTML = state.credentials ? readyTemplate() : pairingTemplate();
  bindEvents();
  restoreTerminalScroll(terminalScroll);
}

function pairingTemplate() {
  return `
    <section class="topbar">
      <div>
        <p class="kicker">Chrome PWA</p>
        <h1>Agent Manager</h1>
        <p class="muted">Pair this browser with the desktop Mobile Bridge.</p>
        ${pwaMetaTemplate()}
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
          ${pwaMetaTemplate()}
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
          ${run ? selectedRunTemplate(run) : `<div class="terminal-header"><div><h2>No run selected</h2><p class="muted">Create or resume a run from desktop.</p></div></div><pre class="terminal" data-terminal-output aria-label="Terminal output">Waiting for terminal output...</pre>`}
        </section>
      </div>
      <div class="drawer-backdrop${state.drawerOpen ? " open" : ""}" data-action="close-drawer" aria-hidden="true"></div>
    </section>
  `;
}

function pwaMetaTemplate() {
  return `
    <p class="pwa-meta">
      <span class="asset-version" data-mobile-version="${escapeHtml(MOBILE_PWA_VERSION)}">PWA ${escapeHtml(MOBILE_PWA_VERSION)}</span>
      <a class="reset-link" href="/mobile/reset">Reset PWA</a>
    </p>
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

function analyzeTerminalPrompt(text) {
  const lines = recentNonEmptyLines(text);
  const recentText = lines.join("\n");
  const choices = choicesFromOptionRows(parseChoiceRows(lines));
  if (choices.length >= 2) {
    return { mode: "choice", choices: withCancelChoice(choices, recentText) };
  }
  if (looksInteractivePrompt(recentText)) {
    return { mode: "fallbackKeys", choices: [] };
  }
  return { mode: "normal", choices: [] };
}

function recentNonEmptyLines(text) {
  return String(text || "")
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+$/, ""))
    .filter((line) => line.trim())
    .slice(-28);
}

function parseChoiceRows(lines) {
  const rows = lines.map((line) => optionRowFromLine(line));
  const selectedIndex = rows.findIndex((row) => row && row.selected);
  if (selectedIndex >= 0) {
    return contiguousOptionRows(rows, selectedIndex);
  }
  return longestOptionRun(rows);
}

function optionRowFromLine(line) {
  const match = String(line || "").match(/^\s*(?:(?<marker>[\u276f\u203a>▸▶➜→»])\s*)?(?:\[(?<bracketToken>\d{1,2}|[A-Za-z])\]|(?<token>\d{1,2}|[A-Za-z])[.)])\s+(?<label>.+?)\s*$/u);
  if (!match || !match.groups) return null;
  const rawLabel = match.groups.label.trim();
  const shortcut = shortcutFromLabel(rawLabel);
  return {
    selected: Boolean(match.groups.marker),
    token: match.groups.bracketToken || match.groups.token,
    label: cleanChoiceLabel(rawLabel),
    shortcut
  };
}

function contiguousOptionRows(rows, selectedIndex) {
  let start = selectedIndex;
  while (start > 0 && rows[start - 1]) start -= 1;
  let end = selectedIndex;
  while (end + 1 < rows.length && rows[end + 1]) end += 1;
  return rows.slice(start, end + 1).filter(Boolean);
}

function longestOptionRun(rows) {
  let best = [];
  let current = [];
  for (const row of rows) {
    if (row) {
      current.push(row);
      if (current.length > best.length) best = current;
      continue;
    }
    current = [];
  }
  return best;
}

function choicesFromOptionRows(rows) {
  if (rows.length < 2) return [];
  const selectedIndex = rows.findIndex((row) => row.selected);
  return rows.map((row, index) => ({
    label: row.label,
    input: optionRowInput(row, selectedIndex, index)
  }));
}

function optionRowInput(row, selectedIndex, index) {
  if (simpleShortcutInput(row.shortcut)) return simpleShortcutInput(row.shortcut);
  if (selectedIndex >= 0) return cursorChoiceInput(selectedIndex, index);
  return `${row.token}\n`;
}

function shortcutFromLabel(label) {
  const match = label.match(/\(([^()]+)\)\s*$/);
  return match ? match[1].trim().toLowerCase() : "";
}

function simpleShortcutInput(shortcut) {
  if (/^[a-z0-9]$/.test(shortcut)) return shortcut;
  if (shortcut === "esc" || shortcut === "escape") return "\x1b";
  return "";
}

function cleanChoiceLabel(label) {
  return label
    .replace(/^(?:\d{1,2}|[A-Za-z])[.)]\s+/, "")
    .replace(/\s+\((?:[a-z0-9]|esc|escape)\)\s*$/i, "");
}

function cursorChoiceInput(selectedIndex, index) {
  if (index > selectedIndex) return repeatKey("\x1b[B", index - selectedIndex) + "\r";
  if (index < selectedIndex) return repeatKey("\x1b[A", selectedIndex - index) + "\r";
  return "\r";
}

function repeatKey(key, count) {
  return Array.from({ length: count }, () => key).join("");
}

function withCancelChoice(choices, text) {
  if (!/\besc\b|\bcancel\b/i.test(text)) return choices;
  if (choices.some((choice) => choice.input === "\x1b" || /^cancel$/i.test(choice.label))) return choices;
  return [...choices, { label: "Cancel", input: "\x1b" }];
}

function looksInteractivePrompt(text) {
  return /\b(choose|select|approve|confirm|press enter|press esc|esc to cancel|use (the )?(arrow|up|down)|\[y\/n\]|\(y\/n\))\b/i.test(text);
}

function terminalPromptSignature(text) {
  const prompt = analyzeTerminalPrompt(text);
  return promptSignature(prompt);
}

function promptSignature(prompt) {
  return JSON.stringify({
    mode: prompt.mode,
    choices: prompt.choices.map((choice) => [choice.label, choice.input])
  });
}

function terminalControlsChanged(previousPromptSignature) {
  return terminalPromptSignature(state.terminalOutput) !== previousPromptSignature;
}

function shouldShowComposerOverride(prompt) {
  return prompt.mode !== "normal" && state.composerOverridePromptSignature === promptSignature(prompt);
}

function showComposerForCurrentPrompt() {
  const prompt = analyzeTerminalPrompt(state.terminalOutput);
  if (prompt.mode === "normal") return;
  state.composerOverridePromptSignature = promptSignature(prompt);
  render({ preserveTerminalScroll: true });
}

function showPromptControls() {
  state.composerOverridePromptSignature = "";
  render({ preserveTerminalScroll: true });
}

function selectedRunTemplate(run) {
  const prompt = state.terminalId ? analyzeTerminalPrompt(state.terminalOutput) : { mode: "normal", choices: [] };
  return `
    <div class="terminal-header">
      <div>
        <h2>${escapeHtml(run.runName)}</h2>
        <p class="muted">${escapeHtml(run.repoName)} / ${escapeHtml(run.branch)}</p>
      </div>
      <div class="terminal-status-row">
        ${statusPillTemplate()}
        ${prompt.mode === "normal" ? `<button class="secondary mini-button" data-action="focus-composer" aria-label="Focus instruction composer" title="Focus instruction composer">Input</button>` : ""}
        ${run.restorable ? `<button data-action="resume" ${state.busy ? "disabled" : ""}>Resume</button>` : ""}
      </div>
    </div>
    <pre class="terminal" data-terminal-output aria-label="Terminal output">${escapeHtml(terminalText())}</pre>
    ${controlPanelTemplate(prompt)}
  `;
}

function controlPanelTemplate(prompt) {
  if (shouldShowComposerOverride(prompt)) {
    return normalComposerTemplate({ showPromptControls: true });
  }
  if (prompt.mode === "choice") {
    return choiceModeTemplate(prompt);
  }
  if (prompt.mode === "fallbackKeys") {
    return fallbackKeyModeTemplate();
  }
  return normalComposerTemplate();
}

function statusPillTemplate() {
  const status = state.terminalId ? "live" : state.terminalStatus;
  const label = status === "attached" ? "live" : status;
  return `<span class="stream-status ${escapeHtml(status)}" aria-live="polite">${escapeHtml(label)}</span>`;
}

function choiceModeTemplate(prompt) {
  const disabled = state.terminalId ? "" : "disabled";
  return `
    <div class="choice-panel" data-terminal-controls data-choice-mode aria-label="Terminal choices">
      <div class="choice-list">
        ${prompt.choices.map((choice) => `
          <button class="choice-button" data-terminal-choice-input="${terminalInputAttribute(choice.input)}" ${disabled}>
            ${escapeHtml(choice.label)}
          </button>
        `).join("")}
      </div>
      <div class="choice-panel-actions">
        <button type="button" class="secondary mini-button" data-action="show-composer">Textbox</button>
      </div>
    </div>
  `;
}

function fallbackKeyModeTemplate() {
  const keys = [
    { label: "Up", data: "\x1b[A" },
    { label: "Down", data: "\x1b[B" },
    { label: "Enter", data: "\r" },
    { label: "Esc", data: "\x1b" },
    { label: "Tab", data: "\t" }
  ];
  const disabled = state.terminalId ? "" : "disabled";
  return `
    <div class="choice-panel key-panel" data-terminal-controls aria-label="Terminal keys">
      <div class="key-bar">
        ${keys.map((key) => `
          <button class="key-button" data-terminal-key="${terminalInputAttribute(key.data)}" ${disabled}>
            ${escapeHtml(key.label)}
          </button>
        `).join("")}
      </div>
      <div class="choice-panel-actions">
        <button type="button" class="secondary mini-button" data-action="show-composer">Textbox</button>
      </div>
    </div>
  `;
}

function normalComposerTemplate(options = {}) {
  return `
    <form class="composer-bar" data-terminal-controls data-form="instruction">
      <label>
        <span class="muted">Instruction</span>
        <textarea name="instruction" placeholder="Send instructions to the selected agent"></textarea>
      </label>
      <div class="composer-actions">
        ${options.showPromptControls ? `<button type="button" class="secondary mini-button" data-action="show-prompt-controls">Options</button>` : ""}
        <button data-send-instruction ${state.terminalId ? "" : "disabled"}>Send</button>
      </div>
    </form>
  `;
}

function terminalInputAttribute(data) {
  return escapeHtml(encodeURIComponent(data));
}

function terminalInputFromAttribute(encoded) {
  try {
    return decodeURIComponent(encoded || "");
  } catch {
    return "";
  }
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
  app.querySelectorAll("[data-terminal-choice-input]").forEach((button) => {
    button.addEventListener("click", () => {
      sendTerminalInput(terminalInputFromAttribute(button.getAttribute("data-terminal-choice-input")));
    });
  });
  app.querySelectorAll("[data-terminal-key]").forEach((button) => {
    button.addEventListener("click", () => {
      sendTerminalInput(terminalInputFromAttribute(button.getAttribute("data-terminal-key")));
    });
  });
  app.querySelector('[data-action="show-composer"]')?.addEventListener("click", showComposerForCurrentPrompt);
  app.querySelector('[data-action="show-prompt-controls"]')?.addEventListener("click", showPromptControls);
  app.querySelector('[data-action="refresh"]')?.addEventListener("click", () => loadDashboard());
  app.querySelector('[data-action="disconnect"]')?.addEventListener("click", clearCredentials);
  app.querySelector('[data-action="resume"]')?.addEventListener("click", resumeSelectedRun);
  app.querySelector('[data-action="focus-composer"]')?.addEventListener("click", () => {
    app.querySelector('[name="instruction"]')?.focus();
  });
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
        assert_eq!(MOBILE_PWA_ASSET_VERSION, "v6");
        assert!(shell
            .body
            .contains(r#"href="/mobile/manifest.webmanifest""#));
        assert!(shell.body.contains(r#"href="/mobile/styles.css?v=v6""#));
        assert!(shell.body.contains(r#"src="/mobile/app.js?v=v6""#));
        assert!(shell.body.contains(r#"data-mobile-version="v6""#));
        assert!(shell.body.contains("Agent Manager Mobile"));
    }

    #[test]
    fn mobile_reset_asset_clears_installed_pwa_state_before_reloading() {
        let reset = asset_for_path("/mobile/reset").expect("mobile reset should be served");

        assert_eq!(reset.content_type, "text/html; charset=utf-8");
        assert!(reset.body.contains("Resetting Agent Manager Mobile"));
        assert!(reset
            .body
            .contains("navigator.serviceWorker.getRegistrations()"));
        assert!(reset.body.contains("registration.unregister()"));
        assert!(reset.body.contains("caches.keys()"));
        assert!(reset.body.contains("caches.delete(key)"));
        assert!(reset
            .body
            .contains(r#"location.replace("/mobile?resetComplete=1&v=v6")"#));
    }

    #[test]
    fn mobile_asset_responses_disable_browser_http_cache() {
        let response = asset_response("/mobile/app.js");

        assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    }

    #[test]
    fn mobile_script_uses_bridge_pairing_dashboard_resume_and_stream_endpoints() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert_eq!(script.content_type, "text/javascript; charset=utf-8");
        assert!(script.body.contains(r#"const MOBILE_PWA_VERSION = "v6";"#));
        assert!(script.body.contains(r#"href="/mobile/reset""#));
        assert!(script
            .body
            .contains("PWA ${escapeHtml(MOBILE_PWA_VERSION)}"));
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

    #[test]
    fn mobile_script_does_not_reattach_same_running_terminal_on_dashboard_refresh() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("attachedRunId: null"));
        assert!(script
            .body
            .contains("function selectedRunAlreadyAttached(run)"));
        assert!(script
            .body
            .contains("if (selectedRunAlreadyAttached(run)) return;"));
    }

    #[test]
    fn mobile_script_updates_terminal_output_without_full_page_render() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains(r#"data-terminal-output"#));
        assert!(script.body.contains("function setTerminalOutput(data)"));
        assert!(script.body.contains("function appendTerminalOutput(data)"));
        assert!(script
            .body
            .contains("const hadOutput = Boolean(state.terminalOutput);"));
        assert!(script
            .body
            .contains("if (!hadOutput) terminal.textContent = \"\";"));
        assert!(script
            .body
            .contains("queueTerminalOutput(message.data || \"\");"));
    }

    #[test]
    fn mobile_script_follows_tail_unless_user_scrolled_up() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("const TAIL_LOCK_THRESHOLD = 48;"));
        assert!(script
            .body
            .contains("function shouldFollowTerminalTail(terminal)"));
        assert!(script
            .body
            .contains("function updateTerminalScroll(terminal, shouldFollow)"));
        assert!(script
            .body
            .contains("terminal.scrollTop = terminal.scrollHeight;"));
    }

    #[test]
    fn mobile_script_handles_terminal_closed_without_clearing_output() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("message.type === \"terminalClosed\""));
        assert!(script.body.contains("state.terminalId = null;"));
        assert!(script.body.contains("state.terminalStatus = \"closed\";"));
    }

    #[test]
    fn mobile_script_batches_live_terminal_output_before_touching_dom() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("pendingTerminalOutput: \"\""));
        assert!(script.body.contains("function queueTerminalOutput(data)"));
        assert!(script
            .body
            .contains("requestAnimationFrame(flushTerminalOutput)"));
        assert!(script
            .body
            .contains("queueTerminalOutput(message.data || \"\");"));
    }

    #[test]
    fn mobile_script_preserves_terminal_scroll_across_structural_renders() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function render(options = {})"));
        assert!(script.body.contains("function captureTerminalScroll()"));
        assert!(script
            .body
            .contains("function restoreTerminalScroll(snapshot)"));
        assert!(script
            .body
            .contains("render({ preserveTerminalScroll: true });"));
    }

    #[test]
    fn mobile_ui_surfaces_stream_status_and_operator_controls() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");
        let styles = asset_for_path("/mobile/styles.css").expect("mobile styles should be served");

        assert!(script.body.contains("function statusPillTemplate()"));
        assert!(script.body.contains(r#"class="stream-status"#));
        assert!(script.body.contains(r#"aria-live="polite""#));
        assert!(script.body.contains(r#"data-action="focus-composer""#));
        assert!(styles.body.contains(".stream-status"));
        assert!(styles.body.contains(".terminal-panel"));
        assert!(styles.body.contains("contain: layout paint style;"));
        assert!(styles.body.contains(".composer-bar:focus-within"));
    }

    #[test]
    fn mobile_script_detects_choice_prompts_and_replaces_composer() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function analyzeTerminalPrompt(text)"));
        assert!(script
            .body
            .contains("function controlPanelTemplate(prompt)"));
        assert!(script.body.contains("function choiceModeTemplate(prompt)"));
        assert!(script.body.contains("return choiceModeTemplate(prompt);"));
        assert!(script.body.contains(r#"data-terminal-choice-input"#));
    }

    #[test]
    fn mobile_choice_mode_can_reveal_textbox_for_current_prompt() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");
        let styles = asset_for_path("/mobile/styles.css").expect("mobile styles should be served");

        assert!(script.body.contains("composerOverridePromptSignature"));
        assert!(script
            .body
            .contains("function showComposerForCurrentPrompt()"));
        assert!(script.body.contains("function showPromptControls()"));
        assert!(script
            .body
            .contains("function shouldShowComposerOverride(prompt)"));
        assert!(script.body.contains(r#"data-action="show-composer""#));
        assert!(script
            .body
            .contains(r#"data-action="show-prompt-controls""#));
        assert!(script
            .body
            .contains("return normalComposerTemplate({ showPromptControls: true });"));
        assert!(styles.body.contains(".choice-panel-actions"));
        assert!(styles.body.contains(".composer-actions"));
    }

    #[test]
    fn mobile_script_maps_option_rows_to_terminal_input() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function parseChoiceRows(lines)"));
        assert!(script.body.contains("function optionRowFromLine(line)"));
        assert!(script.body.contains("function choicesFromOptionRows(rows)"));
        assert!(script
            .body
            .contains("function optionRowInput(row, selectedIndex, index)"));
        assert!(script.body.contains(r#"[\u276f\u203a>▸▶➜→»]"#));
        assert!(script.body.contains(r#"return `${row.token}\n`;"#));
        assert!(script
            .body
            .contains(r#"repeatKey("\x1b[B", index - selectedIndex)"#));
        assert!(script
            .body
            .contains(r#"repeatKey("\x1b[A", selectedIndex - index)"#));
        assert!(script.body.contains(r#"+ "\r""#));
    }

    #[test]
    fn mobile_script_maps_lettered_choices_to_terminal_input() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains(r#"\d{1,2}|[A-Za-z]"#));
        assert!(script.body.contains("return `${row.token}\\n`;"));
    }

    #[test]
    fn mobile_script_parses_selected_options_with_shortcut_hints() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script.body.contains("function cleanChoiceLabel(label)"));
        assert!(script.body.contains("function shortcutFromLabel(label)"));
        assert!(script
            .body
            .contains("function simpleShortcutInput(shortcut)"));
        assert!(script.body.contains(
            r#"if (simpleShortcutInput(row.shortcut)) return simpleShortcutInput(row.shortcut);"#
        ));
        assert!(script.body.contains(r#"shortcut === "esc""#));
    }

    #[test]
    fn mobile_script_renders_fallback_terminal_keys_for_uncertain_prompts() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");
        let styles = asset_for_path("/mobile/styles.css").expect("mobile styles should be served");

        assert!(script.body.contains("function fallbackKeyModeTemplate()"));
        assert!(script.body.contains("data-terminal-key"));
        assert!(script.body.contains(r#"{ label: "Up", data: "\x1b[A" }"#));
        assert!(script.body.contains(r#"{ label: "Down", data: "\x1b[B" }"#));
        assert!(script.body.contains(r#"{ label: "Enter", data: "\r" }"#));
        assert!(script.body.contains(r#"{ label: "Esc", data: "\x1b" }"#));
        assert!(script.body.contains(r#"{ label: "Tab", data: "\t" }"#));
        assert!(styles.body.contains(".choice-panel"));
        assert!(styles.body.contains(".key-button"));
    }

    #[test]
    fn mobile_script_encodes_terminal_input_attributes_before_sending() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script
            .body
            .contains("function terminalInputAttribute(data)"));
        assert!(script
            .body
            .contains("function terminalInputFromAttribute(encoded)"));
        assert!(script.body.contains("encodeURIComponent(data)"));
        assert!(script.body.contains("decodeURIComponent(encoded || \"\")"));
    }

    #[test]
    fn mobile_script_limits_interactive_hint_detection_to_recent_lines() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script
            .body
            .contains(r#"const recentText = lines.join("\n");"#));
        assert!(script
            .body
            .contains("withCancelChoice(choices, recentText)"));
        assert!(script.body.contains("looksInteractivePrompt(recentText)"));
    }

    #[test]
    fn mobile_script_keeps_normal_instruction_composer_for_regular_terminal_text() {
        let script = asset_for_path("/mobile/app.js").expect("mobile script should be served");

        assert!(script
            .body
            .contains("function normalComposerTemplate(options = {})"));
        assert!(script.body.contains(r#"<textarea name="instruction""#));
        assert!(script.body.contains("sendTerminalInput(`${text}\\n`)"));
    }

    #[test]
    fn mobile_service_worker_cache_is_bumped_for_choice_mode_assets() {
        let service_worker =
            asset_for_path("/mobile/sw.js").expect("mobile service worker should be served");

        assert!(service_worker
            .body
            .contains(r#"const CACHE_NAME = "agent-manager-mobile-v6";"#));
        assert!(service_worker.body.contains(r#""/mobile/styles.css?v=v6""#));
        assert!(service_worker.body.contains(r#""/mobile/app.js?v=v6""#));
        assert!(service_worker.body.contains(r#""/mobile/reset""#));
    }

    #[test]
    fn mobile_service_worker_fetches_fresh_mobile_assets_before_cache() {
        let service_worker =
            asset_for_path("/mobile/sw.js").expect("mobile service worker should be served");

        assert!(service_worker
            .body
            .contains("fetch(event.request).then((response) => {"));
        assert!(service_worker
            .body
            .contains("cache.put(event.request, response.clone())"));
        assert!(service_worker.body.contains("cached || Response.error()"));
        assert!(service_worker.body.contains("client.navigate(client.url)"));
    }
}
