# Agent Manager

Agent Manager is a Linux desktop app for managing tmux-backed Codex and Claude
sessions in Git worktrees or directly in existing folders. It is the desktop
counterpart to the CLI/TUI in `../agent-manager`.

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

## Direct Folder Sessions

Choose `Folder` in the New Run modal to launch Codex or Claude directly in an
existing directory. Folder sessions can have independent names, so several
sessions can share the same directory without creating Git worktrees.

Folder mode does not run Git setup, merge, diff, worktree removal, or branch
deletion commands—even when the selected folder is a Git repository. `Stop`
hides the session after stopping its tmux window. `End` also forgets the
session, but always preserves the selected folder and every file in it.

Direct folder sessions are available in the desktop app only. The mobile
bridge continues to expose worktree runs.

## Standalone Codex Sessions

Agent Manager launches every Codex run as its own `codex` CLI process. It does
not start or connect runs to a shared `codex app-server`. This keeps each run's
memory and MCP subprocess lifecycle isolated, so stopping a run releases its
Codex resources without retaining them in one long-lived server.

The desktop reads thread IDs from Codex's local state database to preserve
exact resume behavior, including when several named sessions share a folder.
Run status is detected from the managed tmux pane and terminal output.

Status detection is provider-agnostic: tmux collects pane health, current
command, title, output activity time, and recent visible text, then a pure
evidence reducer selects the freshest, strongest signal. Codex and Claude have
separate prompt/work-marker profiles, and future structured provider events can
feed the same reducer without introducing a shared agent process. A numbered
Codex approval menu is treated as blocking input even when a preceding Running
marker is still fresh; the ordinary Codex composer remains subordinate to real
interruptible work. Claude's starred duration footer marks a completed response,
and prompts separated by Unicode whitespace still request input; a live Claude
spinner remains authoritative over both.

Runs created by older Agent Manager builds may remain connected to the legacy
`agentctl-codex` tmux service until they are stopped. After those runs have
ended, the unused legacy service can be removed with:

```bash
tmux kill-session -t agentctl-codex
```

## Restart Persistence

When tmux restart restore is enabled, Agent Manager rewrites saved managed panes
to resume their recorded Codex or Claude session before tmux-resurrect rebuilds
them. Codex's NVM-compatible login-shell resume wrapper is explicitly included
in tmux-resurrect's process matching. Snapshot discovery follows
tmux-resurrect's Linux convention: an existing legacy `~/.tmux/resurrect`
directory takes precedence; otherwise snapshots are loaded from
`$XDG_DATA_HOME/tmux/resurrect`, defaulting to `~/.local/share/tmux/resurrect`.

## Worktree File Snapshots

New runs copy non-ignored untracked files into the worktree before the agent
starts. The New Run modal also enables copying Git-ignored files by default so
local `.env` files and generated artifacts are available in the worktree.

The modal previews the ignored file count and size. Snapshots at or above
100 MiB or 10,000 files require confirmation. This is a one-time copy: later
changes in the source repository are not synchronized into the worktree.

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

4. Open the Chrome/PWA mobile UI on Android:

```text
https://linhmon.linhmon.1vn.app/mobile
```

5. Complete xTunnel sign-in in Chrome, tap `Pair Android` in the desktop panel,
   and enter that one-time code in the mobile page.

The Chrome/PWA path is preferred when Google sign-in is required because Google
blocks OAuth inside embedded Android WebView. The PWA stores the paired device
token in browser storage, shows recent worktree-run state, and sends
instructions into the selected tmux-backed agent pane over the bridge
WebSocket. It intentionally exposes only resume and terminal input controls on
mobile.

The native Android app remains in `android/`, but xTunnel login with Google may
not complete inside its WebView.

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
