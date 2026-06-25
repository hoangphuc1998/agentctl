# Session Progress Log

## Current State

**Last Updated:** 2026-06-25 18:11 +07
**Session ID:** correct-xtunnel-bridge-url
**Active Feature:** feat-030 - Correct xTunnel Bridge URL

## Status

### What is Done

- [x] Updated the Mobile Bridge public URL to `https://linhmon.linhmon.1vn.app`.
- [x] Preserved the local xTunnel command as `xtunnel.cmd linhmon start 17654`.
- [x] Modeled xTunnel URL generation as slug `linhmon` plus server-selected domain `linhmon.1vn.app`.
- [x] Updated Android's default pairing URL and README bridge setup text.
- [x] Rebuilt the Android debug APK after the URL change.

### What is In Progress

- [x] URL correction implementation is complete.
- [x] Focused verification is complete.
- [x] Standard post-change `./init.sh` verification is complete.

### What is Next

1. Commit the completed URL correction.

## Blockers / Risks

- [x] No unresolved blockers.
- [ ] `https://linhmon.1vn.app/` remains documented as the xTunnel docs site; the bridge connection URL is `https://linhmon.linhmon.1vn.app`.

## Decisions Made

- Keep the xTunnel slug as `linhmon`; only the server-selected domain portion changes the public bridge URL.
- Keep the local bridge bind and command at `127.0.0.1:17654` / `xtunnel.cmd linhmon start 17654`.
- Update Android defaults so fresh installs open the corrected bridge URL without manual editing.

## Files Modified This Session

- `src-tauri/src/mobile_bridge.rs` - Adds server-selected domain to xTunnel config and generates `https://linhmon.linhmon.1vn.app`.
- `src-tauri/tests/mobile_bridge.rs` - Updates URL expectations and fixture config.
- `android/app/src/main/java/com/example/agentmanagermobile/ui/main/MainScreenViewModel.kt` - Updates Android default bridge URL.
- `android/app/src/test/java/com/example/agentmanagermobile/ui/main/MainScreenViewModelTest.kt` - Adds default URL coverage and updates fixtures.
- `src/App.test.tsx` - Updates desktop bridge status fixture.
- `README.md` - Updates Android bridge connection URL.
- `feature_list.json` - Adds completed feat-030 evidence.
- `progress.md` - Records this session status and evidence.

## Evidence of Completion

- [x] Baseline: `./init.sh` exited 0 before the change.
- [x] RED: `cargo test -p agent-manager-desktop --test mobile_bridge xtunnel_start_command_targets_local_bridge_port_and_optional_auth_policy` failed because the old public URL was `https://linhmon.1vn.app`.
- [x] RED: `./gradlew testDebugUnitTest --tests com.example.agentmanagermobile.ui.main.MainScreenViewModelTest.defaultsPairingToTheXtunnelBridgeUrlWhenNoCredentialsExist` failed because Android still defaulted to `https://linhmon.1vn.app`.
- [x] GREEN: `cargo test -p agent-manager-desktop --test mobile_bridge` exited 0 with 7 tests passing.
- [x] GREEN: `npm test -- src/App.test.tsx -t "starts the mobile bridge"` exited 0.
- [x] GREEN: `./gradlew testDebugUnitTest --tests com.example.agentmanagermobile.ui.main.MainScreenViewModelTest` exited 0.
- [x] Android APK build: `./gradlew assembleDebug` exited 0.
- [x] Standard verification: `./init.sh` exited 0 with npm test, npm run build, and cargo test passing.
