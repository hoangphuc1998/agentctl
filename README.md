# Agent Manager

Agent Manager is a Linux desktop app for managing `agentctl` Codex and Claude
runs in git worktrees. It is the desktop counterpart to the CLI/TUI in
`../agent-manager`.

The app uses Tauri, a copied Rust core, SQLite registry compatibility with the
CLI, and an embedded xterm terminal attached to tmux-backed agent windows.

## Development

Install Linux/Tauri build dependencies on Ubuntu 22.04:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libgdk-pixbuf-2.0-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libdbus-1-dev
```

```bash
npm install
npm run tauri:dev
```

## Verification

```bash
./init.sh
```

## Android Companion

The Android companion app lives in `android/`. It connects to the desktop app
through the local Mobile Bridge and xTunnel public HTTPS endpoint.

Desktop setup:

1. Install xTunnel from the internal docs at `https://linhmon.1vn.app/`.
   The documented setup syncs scripts from `tunnel-edge.1ai.lab` into
   `/xserver/bin` and adds `XCRIPTS_HOME=/xserver/bin` to `PATH`.
2. Start the Mobile Bridge from the desktop left panel.
3. Expose the bridge with xTunnel:

```bash
xtunnel.cmd linhmon start 17654
```

4. Tap `Pair Android` in the desktop panel and enter that one-time code in the
   Android app at `https://linhmon.linhmon.1vn.app`.

The Android app uses an in-app WebView for xTunnel authentication, stores the
paired device token in encrypted preferences, shows recent run state, and sends
instructions into the selected tmux-backed agent pane over the bridge WebSocket.
It intentionally exposes only resume and terminal input controls on mobile.

Build the Android debug APK:

```bash
cd android
./gradlew assembleDebug
```

## Packaging

```bash
npm run tauri:build
```

The first Linux packaging targets are AppImage and Debian packages.

If `npm run tauri:build` reports missing `gdk-3.0`, `gdk-pixbuf-2.0`,
`webkit2gtk-4.1`, `dbus-1`, `openssl`, or packaging commands such as `curl`,
`wget`, or `file`, install the Ubuntu packages above and retry.

To build one package type at a time:

```bash
npm run tauri:build:deb
npm run tauri:build:appimage
```
