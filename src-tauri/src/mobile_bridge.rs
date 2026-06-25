use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const PAIRING_CODE_TTL_SECONDS: i64 = 10 * 60;
const TOKEN_PREFIX: &str = "agm_";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PairingTime {
    epoch_seconds: i64,
}

impl PairingTime {
    pub fn now() -> Self {
        Self {
            epoch_seconds: chrono::Utc::now().timestamp(),
        }
    }

    pub fn from_epoch_seconds(epoch_seconds: i64) -> Self {
        Self { epoch_seconds }
    }

    fn plus_seconds(self, seconds: i64) -> Self {
        Self {
            epoch_seconds: self.epoch_seconds + seconds,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    pub code: String,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub id: String,
    pub name: String,
    pub token: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDeviceView {
    pub id: String,
    pub name: String,
    pub paired_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileBridgeStatus {
    pub enabled: bool,
    pub bind: String,
    pub public_url: String,
    pub paired_devices: Vec<PairedDeviceView>,
    pub xtunnel_start_command: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PairingCodeRecord {
    expires_at: PairingTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PairedDeviceRecord {
    id: String,
    name: String,
    token_hash: String,
    paired_at: i64,
    revoked: bool,
}

impl PairedDeviceRecord {
    fn view(&self) -> PairedDeviceView {
        PairedDeviceView {
            id: self.id.clone(),
            name: self.name.clone(),
            paired_at: self.paired_at,
        }
    }
}

#[derive(Default)]
pub struct PairingStore {
    codes: HashMap<String, PairingCodeRecord>,
    devices: HashMap<String, PairedDeviceRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PairingError {
    InvalidCode,
    ExpiredCode,
}

impl fmt::Display for PairingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode => formatter.write_str("pairing code invalid"),
            Self::ExpiredCode => formatter.write_str("pairing code expired"),
        }
    }
}

impl std::error::Error for PairingError {}

impl PairingStore {
    pub fn issue_code(&mut self, now: PairingTime) -> PairingCode {
        let code = new_pairing_code();
        let expires_at = now.plus_seconds(PAIRING_CODE_TTL_SECONDS);
        self.codes
            .insert(code.clone(), PairingCodeRecord { expires_at });
        PairingCode {
            code,
            expires_at: expires_at.epoch_seconds,
        }
    }

    pub fn claim_code(
        &mut self,
        code: &str,
        device_name: impl Into<String>,
        now: PairingTime,
    ) -> Result<PairedDevice, PairingError> {
        let Some(record) = self.codes.remove(code) else {
            return Err(PairingError::InvalidCode);
        };
        if now > record.expires_at {
            return Err(PairingError::ExpiredCode);
        }

        let id = Uuid::new_v4().to_string();
        let token = new_device_token();
        let name = device_name.into();
        self.devices.insert(
            id.clone(),
            PairedDeviceRecord {
                id: id.clone(),
                name: name.clone(),
                token_hash: token_hash(&token),
                paired_at: now.epoch_seconds,
                revoked: false,
            },
        );
        Ok(PairedDevice { id, name, token })
    }

    pub fn authenticate(&self, device_id: &str, token: &str) -> bool {
        self.devices
            .get(device_id)
            .filter(|device| !device.revoked)
            .map(|device| device.token_hash == token_hash(token))
            .unwrap_or(false)
    }

    pub fn revoke_device(&mut self, device_id: &str) {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.revoked = true;
        }
    }

    pub fn devices(&self) -> Vec<PairedDeviceView> {
        let mut devices = self
            .devices
            .values()
            .filter(|device| !device.revoked)
            .map(PairedDeviceRecord::view)
            .collect::<Vec<_>>();
        devices.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        devices
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeBind {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for BridgeBind {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 17654,
        }
    }
}

impl fmt::Display for BridgeBind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        SocketAddr::new(self.host, self.port).fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XtunnelConfig {
    pub slug: String,
    pub local_port: u16,
    pub auth_mail: Option<String>,
    pub auth_org: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XtunnelCommand {
    pub argv: Vec<String>,
    pub public_url: String,
}

impl XtunnelCommand {
    pub fn shell_command(&self) -> String {
        self.argv.join(" ")
    }
}

pub fn default_xtunnel_config(port: u16) -> XtunnelConfig {
    XtunnelConfig {
        slug: "linhmon".to_string(),
        local_port: port,
        auth_mail: None,
        auth_org: None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MobileAuthHeaders {
    pub device_id: Option<String>,
    pub authorization: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StreamClientMessage {
    SubscribeDashboard,
    AttachTerminal {
        run_id: String,
        cols: u16,
        rows: u16,
    },
    TerminalInput {
        terminal_id: String,
        data: String,
    },
    TerminalResize {
        terminal_id: String,
        cols: u16,
        rows: u16,
    },
    DetachTerminal {
        terminal_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StreamServerMessage {
    DashboardState {
        dashboard: crate::models::DashboardState,
    },
    AgentAttention {
        event: crate::models::AgentAttentionEvent,
    },
    TerminalAttached {
        terminal_id: String,
        run_id: String,
    },
    TerminalSnapshot {
        run_id: String,
        data: String,
    },
    TerminalOutput {
        terminal_id: String,
        run_id: String,
        data: String,
    },
    TerminalClosed {
        terminal_id: String,
        run_id: String,
    },
    Error {
        message: String,
    },
}

pub fn authenticated_device_id(
    store: &PairingStore,
    headers: &MobileAuthHeaders,
) -> Result<String, PairingError> {
    let device_id = headers
        .device_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(PairingError::InvalidCode)?;
    let token = headers
        .authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or(PairingError::InvalidCode)?;
    if store.authenticate(device_id, token) {
        Ok(device_id.to_string())
    } else {
        Err(PairingError::InvalidCode)
    }
}

pub fn build_xtunnel_start(config: &XtunnelConfig) -> XtunnelCommand {
    let mut argv = vec![
        "xtunnel.cmd".to_string(),
        config.slug.clone(),
        "start".to_string(),
        config.local_port.to_string(),
    ];
    if let Some(auth_mail) = config.auth_mail.as_ref().filter(|value| !value.is_empty()) {
        argv.push("--auth-mail".to_string());
        argv.push(auth_mail.clone());
    }
    if let Some(auth_org) = config.auth_org.as_ref().filter(|value| !value.is_empty()) {
        argv.push("--auth-org".to_string());
        argv.push(auth_org.clone());
    }

    XtunnelCommand {
        argv,
        public_url: format!("https://{}.1vn.app", config.slug),
    }
}

fn new_pairing_code() -> String {
    Uuid::new_v4()
        .as_simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>()
        .to_ascii_uppercase()
}

fn new_device_token() -> String {
    format!(
        "{TOKEN_PREFIX}{}{}",
        Uuid::new_v4().as_simple(),
        Uuid::new_v4().as_simple()
    )
}

fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
