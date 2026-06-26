use agent_manager_desktop::mobile_bridge::{
    authenticated_device_id, build_xtunnel_start, BridgeBind, MobileAuthHeaders, PairingStore,
    PairingTime, StreamClientMessage, StreamServerMessage, XtunnelConfig,
};

#[test]
fn pairing_code_can_be_claimed_once_before_expiry() {
    let mut store = PairingStore::default();
    let code = store.issue_code(PairingTime::from_epoch_seconds(100));

    let device = store
        .claim_code(&code.code, "Pixel 9", PairingTime::from_epoch_seconds(160))
        .expect("pairing code should be valid");

    assert_eq!(device.name, "Pixel 9");
    assert!(device.token.starts_with("agm_"));
    assert!(store.authenticate(&device.id, &device.token));
    assert!(!store.authenticate(&device.id, "wrong-token"));
    assert!(store
        .claim_code(
            &code.code,
            "Other phone",
            PairingTime::from_epoch_seconds(170)
        )
        .is_err());
}

#[test]
fn expired_pairing_code_cannot_be_claimed() {
    let mut store = PairingStore::default();
    let code = store.issue_code(PairingTime::from_epoch_seconds(100));

    let error = store
        .claim_code(&code.code, "Pixel 9", PairingTime::from_epoch_seconds(701))
        .expect_err("code should be expired after ten minutes");

    assert_eq!(error.to_string(), "pairing code expired");
}

#[test]
fn revoked_device_token_no_longer_authenticates() {
    let mut store = PairingStore::default();
    let code = store.issue_code(PairingTime::from_epoch_seconds(100));
    let device = store
        .claim_code(&code.code, "Pixel 9", PairingTime::from_epoch_seconds(120))
        .unwrap();

    store.revoke_device(&device.id);

    assert!(!store.authenticate(&device.id, &device.token));
}

#[test]
fn xtunnel_start_command_targets_local_bridge_port_and_optional_auth_policy() {
    let command = build_xtunnel_start(&XtunnelConfig {
        slug: "linhmon".to_string(),
        server_selected_domain: "linhmon.1vn.app".to_string(),
        local_port: 17654,
        auth_mail: Some("dev@1ai.tech".to_string()),
        auth_org: Some("1ai.tech".to_string()),
    });

    assert_eq!(
        command.argv,
        vec![
            "xtunnel.cmd",
            "linhmon",
            "start",
            "17654",
            "--auth-mail",
            "dev@1ai.tech",
            "--auth-org",
            "1ai.tech"
        ]
    );
    assert_eq!(command.public_url, "https://linhmon.linhmon.1vn.app");
}

#[test]
fn bridge_bind_defaults_to_loopback_only() {
    assert_eq!(BridgeBind::default().to_string(), "127.0.0.1:17654");
}

#[test]
fn mobile_auth_requires_device_id_and_bearer_token() {
    let mut store = PairingStore::default();
    let code = store.issue_code(PairingTime::from_epoch_seconds(100));
    let device = store
        .claim_code(&code.code, "Pixel 9", PairingTime::from_epoch_seconds(120))
        .unwrap();

    assert_eq!(
        authenticated_device_id(
            &store,
            &MobileAuthHeaders {
                device_id: Some(device.id.clone()),
                authorization: Some(format!("Bearer {}", device.token)),
            },
        )
        .unwrap(),
        device.id
    );
    assert!(authenticated_device_id(
        &store,
        &MobileAuthHeaders {
            device_id: None,
            authorization: Some("Bearer token".to_string()),
        },
    )
    .is_err());
    assert!(authenticated_device_id(
        &store,
        &MobileAuthHeaders {
            device_id: Some(device.id),
            authorization: Some("Basic token".to_string()),
        },
    )
    .is_err());
}

#[test]
fn stream_protocol_uses_camel_case_tagged_messages() {
    let client: StreamClientMessage =
        serde_json::from_str(r#"{"type":"attachTerminal","runId":"run-1","cols":96,"rows":28}"#)
            .unwrap();

    assert_eq!(
        client,
        StreamClientMessage::AttachTerminal {
            run_id: "run-1".to_string(),
            cols: 96,
            rows: 28,
        }
    );
    assert_eq!(
        serde_json::to_string(&StreamServerMessage::TerminalSnapshot {
            run_id: "run-1".to_string(),
            data: "hello".to_string(),
        })
        .unwrap(),
        r#"{"type":"terminalSnapshot","runId":"run-1","data":"hello"}"#
    );
}
