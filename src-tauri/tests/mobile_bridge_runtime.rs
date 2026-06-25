#![cfg(feature = "tauri-app")]

use std::{
    net::{IpAddr, Ipv4Addr},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
};

use agent_manager_desktop::{
    mobile_bridge::{BridgeBind, PairingStore},
    mobile_bridge_server::{BridgeServerState, MobileBridgeRuntime},
};

#[test]
fn starting_mobile_bridge_from_sync_context_does_not_panic() {
    let state = BridgeServerState {
        registry_path: "unused.sqlite3".into(),
        pairing: Arc::new(Mutex::new(PairingStore::default())),
    };
    let bind = BridgeBind {
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port: 0,
    };

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut runtime = MobileBridgeRuntime::default();
        let started = runtime.start(state, bind);
        runtime.stop();
        started
    }));

    assert!(result.is_ok(), "mobile bridge start panicked");
    let start_result = result.unwrap();
    assert!(
        start_result.is_ok(),
        "mobile bridge start returned error: {start_result:?}"
    );
}
