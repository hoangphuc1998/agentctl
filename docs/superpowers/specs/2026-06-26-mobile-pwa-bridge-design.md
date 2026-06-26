# Mobile PWA Bridge Design

## Goal

Provide an Android-friendly mobile experience through Chrome instead of the native
WebView, avoiding Google OAuth's embedded-user-agent block while preserving the
existing Mobile Bridge pairing and tmux interaction model.

## Problem

The xTunnel URL `https://linhmon.linhmon.1vn.app/api/mobile/v1/health` works in
Chrome, proving the desktop Mobile Bridge, xTunnel route, and auth policy are
valid. The Android native app fails when AI HAY asks Google to authenticate
inside Android WebView, returning `Error 403: disallowed_useragent`. That means
the native app cannot reliably acquire the xTunnel session cookie when Google is
the required provider.

## Architecture

Serve a lightweight mobile web app from the Mobile Bridge at `/mobile`. Users
open `https://linhmon.linhmon.1vn.app/mobile` in Chrome, complete xTunnel/Google
auth in the browser, then interact with the same-origin bridge APIs:

- `POST /api/mobile/v1/pair/claim`
- `GET /api/mobile/v1/dashboard`
- `POST /api/mobile/v1/runs/:run_id/resume`
- `GET /api/mobile/v1/stream`

The app stores the paired device id/token in browser local storage. Normal HTTP
requests use the existing `Authorization` and `X-Agent-Manager-Device` headers.
Browser WebSockets cannot set custom headers, so the stream endpoint also accepts
the paired device id/token as query parameters for `/api/mobile/v1/stream`.

## User Flow

1. Start Mobile Bridge in the desktop app.
2. Start xTunnel with `xtunnel.cmd linhmon start 17654`.
3. Open `https://linhmon.linhmon.1vn.app/mobile` in Android Chrome.
4. Complete xTunnel/Google auth if prompted.
5. Enter the desktop pairing code.
6. View active/restorable runs, resume restorable runs, and send instructions to
   the selected tmux-backed agent pane.
7. Optionally add the page to the Android home screen from Chrome.

## Components

- `src-tauri/src/mobile_pwa.rs` serves static PWA assets and declares their
  content types.
- `src-tauri/src/mobile_bridge_server.rs` mounts `/mobile` asset routes and
  accepts stream auth query parameters for browser WebSocket compatibility.
- `src/App.tsx` surfaces the mobile web URL next to the xTunnel command.
- `README.md` documents the Chrome/PWA path as the preferred Android flow when
  Google auth is required.

## Security

xTunnel remains the outer access gate. The bridge pairing token remains the app
authorization layer for dashboard, resume, and terminal stream operations.
Stream token-in-query is limited to the WebSocket endpoint because browser
WebSocket APIs do not allow custom headers.

## Verification

Automated checks cover PWA asset routing, browser WebSocket query auth, desktop
mobile-panel URL guidance, and the existing bridge behavior. Manual verification
uses Chrome to load `/api/mobile/v1/health` and `/mobile` through xTunnel.
