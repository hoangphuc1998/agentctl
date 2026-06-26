# Mobile PWA Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve a Chrome-friendly mobile PWA from the Mobile Bridge so Android users can pair, inspect runs, and send tmux instructions without blocked WebView Google OAuth.

**Architecture:** Add a small static PWA served by Axum under `/mobile`, reuse existing bridge JSON APIs, and adapt only the WebSocket stream endpoint to accept browser-compatible query auth. The desktop UI and README expose the new `/mobile` URL.

**Tech Stack:** Rust/Axum Mobile Bridge, vanilla browser JavaScript for the PWA shell, existing React/Tauri desktop UI, Vitest, Cargo tests.

---

### Task 1: Mobile PWA Assets

**Files:**
- Create: `src-tauri/src/mobile_pwa.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing asset tests**

Add tests in `src-tauri/src/mobile_pwa.rs` proving `/mobile` serves an HTML shell with manifest/script links and app JS references the pairing, dashboard, stream, and resume endpoints.

- [ ] **Step 2: Run red test**

Run: `cargo test -p agent-manager-desktop --features tauri-app mobile_pwa`

Expected: fail because `mobile_pwa` does not exist or asset functions are missing.

- [ ] **Step 3: Implement PWA assets**

Create a focused `mobile_pwa` module with `asset_for_path`, route handlers, HTML, CSS, manifest, service worker, icon SVG, and browser JS.

- [ ] **Step 4: Run green test**

Run: `cargo test -p agent-manager-desktop --features tauri-app mobile_pwa`

Expected: PWA asset tests pass.

### Task 2: Bridge Routes and Browser WebSocket Auth

**Files:**
- Modify: `src-tauri/src/mobile_bridge_server.rs`

- [ ] **Step 1: Write failing route/auth tests**

Add feature-gated tests proving stream query auth authenticates a paired device and the router can expose PWA asset routes.

- [ ] **Step 2: Run red test**

Run: `cargo test -p agent-manager-desktop --features tauri-app mobile_bridge_server`

Expected: fail because stream query auth and `/mobile` routes are not implemented.

- [ ] **Step 3: Implement routes and auth mapping**

Mount `/mobile`, `/mobile/`, `/mobile/app.js`, `/mobile/styles.css`, `/mobile/manifest.webmanifest`, `/mobile/sw.js`, and `/mobile/icon.svg`. Add a `StreamAuthQuery` type and map `deviceId` plus `token` query values into the existing mobile auth path.

- [ ] **Step 4: Run green test**

Run: `cargo test -p agent-manager-desktop --features tauri-app mobile_bridge_server`

Expected: route/auth tests pass.

### Task 3: Desktop Guidance and Documentation

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `README.md`

- [ ] **Step 1: Write failing desktop guidance test**

Update the mobile bridge panel test to expect `https://linhmon.linhmon.1vn.app/mobile` after the bridge starts.

- [ ] **Step 2: Run red test**

Run: `npm test -- src/App.test.tsx -t "starts the mobile bridge"`

Expected: fail because the panel does not show the mobile PWA URL.

- [ ] **Step 3: Implement guidance and docs**

Render the `/mobile` URL in `MobileBridgePanel` and document the Chrome/PWA flow in `README.md`.

- [ ] **Step 4: Run green test**

Run: `npm test -- src/App.test.tsx -t "starts the mobile bridge"`

Expected: targeted desktop test passes.

### Task 4: Final Verification and Handoff

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [ ] **Step 1: Run focused checks**

Run:

```bash
cargo test -p agent-manager-desktop --features tauri-app mobile_pwa
cargo test -p agent-manager-desktop --features tauri-app mobile_bridge_server
npm test -- src/App.test.tsx -t "starts the mobile bridge"
```

- [ ] **Step 2: Run full checks**

Run:

```bash
npm test
npm run build
cargo test -p agent-manager-desktop --features tauri-app --test mobile_bridge_runtime
cargo check -p agent-manager-desktop --features tauri-app
./init.sh
```

- [ ] **Step 3: Update trackers**

Add completed feature evidence for the mobile PWA bridge and record verification in `progress.md`.

- [ ] **Step 4: Commit**

Run:

```bash
git add docs/superpowers/specs/2026-06-26-mobile-pwa-bridge-design.md docs/superpowers/plans/2026-06-26-mobile-pwa-bridge.md src-tauri/src/mobile_pwa.rs src-tauri/src/lib.rs src-tauri/src/mobile_bridge_server.rs src/App.tsx src/App.test.tsx README.md feature_list.json progress.md
git commit -m "feat: serve mobile bridge pwa"
```
