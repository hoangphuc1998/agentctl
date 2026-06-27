use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use agentctl_core::{
    app::{App, AppConfig, SystemCommandRunner},
    registry::SqliteRegistry,
    tmux::Tmux,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    error::DesktopError,
    mobile_bridge::{
        authenticated_device_id, build_xtunnel_start, default_xtunnel_config, BridgeBind,
        MobileAuthHeaders, MobileBridgeStatus, PairedDevice, PairingStore, PairingTime,
        StreamClientMessage, StreamServerMessage,
    },
    mobile_pwa,
    models::{host_tool_statuses, ActionResult, DashboardState},
    services::build_dashboard_state,
    terminal_plan::tmux_attach_command,
};

const TMUX_SESSION: &str = "agentctl";
const DEVICE_ID_HEADER: &str = "x-agent-manager-device";
#[cfg(test)]
const MOBILE_PWA_ROUTE_PATHS: &[&str] = &[
    "/mobile",
    "/mobile/",
    "/mobile/app.js",
    "/mobile/styles.css",
    "/mobile/manifest.webmanifest",
    "/mobile/sw.js",
    "/mobile/icon.svg",
];

#[derive(Clone)]
pub struct BridgeServerState {
    pub registry_path: PathBuf,
    pub pairing: Arc<Mutex<PairingStore>>,
}

#[derive(Default)]
pub struct MobileBridgeRuntime {
    shutdown: Option<oneshot::Sender<()>>,
    bind: BridgeBind,
}

impl MobileBridgeRuntime {
    pub fn is_running(&self) -> bool {
        self.shutdown.is_some()
    }

    pub fn start(&mut self, state: BridgeServerState, bind: BridgeBind) -> Result<(), String> {
        if self.shutdown.is_some() {
            return Ok(());
        }

        let listener = TcpListener::bind(bind.to_string()).map_err(|err| err.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|err| err.to_string())?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let router = mobile_bridge_router(state);
        tauri::async_runtime::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(listener) {
                Ok(listener) => listener,
                Err(err) => {
                    eprintln!("mobile bridge listener failed to start: {err}");
                    return;
                }
            };
            let server = axum::serve(listener, router).with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
            if let Err(err) = server.await {
                eprintln!("mobile bridge server stopped with error: {err}");
            }
        });
        self.shutdown = Some(shutdown_tx);
        self.bind = bind;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }

    pub fn status(&self, pairing: &PairingStore) -> MobileBridgeStatus {
        let command = build_xtunnel_start(&default_xtunnel_config(self.bind.port));
        MobileBridgeStatus {
            enabled: self.is_running(),
            bind: self.bind.to_string(),
            public_url: command.public_url,
            paired_devices: pairing.devices(),
            xtunnel_start_command: command.argv,
        }
    }
}

pub fn mobile_bridge_router(state: BridgeServerState) -> Router {
    Router::new()
        .route("/mobile", get(mobile_pwa::index))
        .route("/mobile/", get(mobile_pwa::index))
        .route("/mobile/app.js", get(mobile_pwa::app_js))
        .route("/mobile/styles.css", get(mobile_pwa::styles_css))
        .route("/mobile/manifest.webmanifest", get(mobile_pwa::manifest))
        .route("/mobile/sw.js", get(mobile_pwa::service_worker))
        .route("/mobile/icon.svg", get(mobile_pwa::icon_svg))
        .route("/api/mobile/v1/health", get(health))
        .route("/api/mobile/v1/pair/claim", post(claim_pairing_code))
        .route("/api/mobile/v1/dashboard", get(dashboard))
        .route("/api/mobile/v1/runs/:run_id/resume", post(resume_run))
        .route("/api/mobile/v1/stream", get(stream))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "agent-manager-mobile-bridge",
    })
}

async fn claim_pairing_code(
    State(state): State<BridgeServerState>,
    Json(payload): Json<PairClaimRequest>,
) -> Result<Json<PairedDevice>, BridgeApiError> {
    let mut store = state
        .pairing
        .lock()
        .map_err(|_| BridgeApiError::internal("pairing lock poisoned"))?;
    Ok(Json(store.claim_code(
        &payload.code,
        payload.device_name,
        PairingTime::now(),
    )?))
}

async fn dashboard(
    State(state): State<BridgeServerState>,
    headers: HeaderMap,
) -> Result<Json<DashboardState>, BridgeApiError> {
    authorize(&state, &headers)?;
    Ok(Json(build_mobile_dashboard(&state)?))
}

async fn resume_run(
    State(state): State<BridgeServerState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Result<Json<ActionResult>, BridgeApiError> {
    authorize(&state, &headers)?;
    let id =
        Uuid::parse_str(&run_id).map_err(|err| BridgeApiError::bad_request(err.to_string()))?;
    let registry = registry(&state)?;
    let mut app = App::new(registry, SystemCommandRunner, AppConfig::from_environment());
    let run = app
        .restore_run(id)
        .map_err(|err| BridgeApiError::internal(err.to_string()))?;
    Ok(Json(ActionResult {
        message: run
            .as_ref()
            .map(|run| format!("Resumed `{}`.", run.run_name))
            .unwrap_or_else(|| format!("Run not found: {run_id}")),
        run: run.map(Into::into),
    }))
}

async fn stream(
    State(state): State<BridgeServerState>,
    headers: HeaderMap,
    Query(query): Query<StreamAuthQuery>,
    websocket: WebSocketUpgrade,
) -> Response {
    match authorize_stream(&state, &headers, &query) {
        Ok(_) => websocket
            .on_upgrade(move |socket| handle_stream(socket, state))
            .into_response(),
        Err(error) => error.into_response(),
    }
}

async fn handle_stream(socket: WebSocket, state: BridgeServerState) {
    let (mut sender, mut receiver) = socket.split();
    let (pty_tx, mut pty_rx) = mpsc::channel::<StreamServerMessage>(128);
    let mut terminals: HashMap<String, MobilePtySession> = HashMap::new();

    loop {
        tokio::select! {
            next = receiver.next() => {
                let Some(Ok(message)) = next else {
                    break;
                };
                if let Message::Text(text) = message {
                    if let Some(response) = handle_client_message(
                        &state,
                        &mut terminals,
                        pty_tx.clone(),
                        &text,
                    ) {
                        let _ = sender.send(Message::Text(to_json(&response))).await;
                    }
                }
            }
            Some(message) = pty_rx.recv() => {
                if sender.send(Message::Text(to_json(&message))).await.is_err() {
                    break;
                }
            }
        }
    }
}

fn handle_client_message(
    state: &BridgeServerState,
    terminals: &mut HashMap<String, MobilePtySession>,
    pty_tx: mpsc::Sender<StreamServerMessage>,
    text: &str,
) -> Option<StreamServerMessage> {
    match serde_json::from_str::<StreamClientMessage>(text) {
        Ok(StreamClientMessage::SubscribeDashboard) => Some(match build_mobile_dashboard(state) {
            Ok(dashboard) => StreamServerMessage::DashboardState { dashboard },
            Err(err) => StreamServerMessage::Error {
                message: err.message,
            },
        }),
        Ok(StreamClientMessage::AttachTerminal { run_id, cols, rows }) => {
            attach_terminal(state, terminals, pty_tx, run_id, cols, rows)
        }
        Ok(StreamClientMessage::TerminalInput { terminal_id, data }) => {
            if let Some(session) = terminals.get_mut(&terminal_id) {
                if let Err(err) = session.input(&data) {
                    return Some(StreamServerMessage::Error {
                        message: err.to_string(),
                    });
                }
            }
            None
        }
        Ok(StreamClientMessage::TerminalResize {
            terminal_id,
            cols,
            rows,
        }) => {
            if let Some(session) = terminals.get(&terminal_id) {
                if let Err(err) = session.resize(cols, rows) {
                    return Some(StreamServerMessage::Error {
                        message: err.to_string(),
                    });
                }
            }
            None
        }
        Ok(StreamClientMessage::DetachTerminal { terminal_id }) => {
            terminals.remove(&terminal_id);
            None
        }
        Err(err) => Some(StreamServerMessage::Error {
            message: err.to_string(),
        }),
    }
}

fn attach_terminal(
    state: &BridgeServerState,
    terminals: &mut HashMap<String, MobilePtySession>,
    pty_tx: mpsc::Sender<StreamServerMessage>,
    run_id: String,
    cols: u16,
    rows: u16,
) -> Option<StreamServerMessage> {
    let registry = match registry(state) {
        Ok(registry) => registry,
        Err(err) => {
            return Some(StreamServerMessage::Error {
                message: err.message,
            })
        }
    };
    let id = match Uuid::parse_str(&run_id) {
        Ok(id) => id,
        Err(err) => {
            return Some(StreamServerMessage::Error {
                message: err.to_string(),
            })
        }
    };
    let run = match registry.get_run(id) {
        Ok(Some(run)) => run,
        Ok(None) => {
            return Some(StreamServerMessage::Error {
                message: format!("run not found: {run_id}"),
            })
        }
        Err(err) => {
            return Some(StreamServerMessage::Error {
                message: err.to_string(),
            })
        }
    };
    if let Some(window) = run.tmux_window.as_deref() {
        let session = run.tmux_session.as_deref().unwrap_or(TMUX_SESSION);
        if let Ok(snapshot) = Tmux::new(session).snapshot_window(window) {
            let _ = queue_terminal_snapshot(&pty_tx, run_id.clone(), snapshot.visible_text);
        }
    }
    match MobilePtySession::start(&run, cols, rows, pty_tx) {
        Ok(session) => {
            let terminal_id = session.terminal_id.clone();
            terminals.insert(terminal_id.clone(), session);
            Some(StreamServerMessage::TerminalAttached {
                terminal_id,
                run_id,
            })
        }
        Err(err) => Some(StreamServerMessage::Error { message: err }),
    }
}

fn queue_terminal_snapshot(
    output: &mpsc::Sender<StreamServerMessage>,
    run_id: String,
    data: String,
) -> Result<(), mpsc::error::TrySendError<StreamServerMessage>> {
    output.try_send(StreamServerMessage::TerminalSnapshot { run_id, data })
}

struct MobilePtySession {
    terminal_id: String,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl MobilePtySession {
    fn start(
        run: &agentctl_core::domain::RunRecord,
        cols: u16,
        rows: u16,
        output: mpsc::Sender<StreamServerMessage>,
    ) -> Result<Self, String> {
        let plan = tmux_attach_command(run, TMUX_SESSION)?;
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| err.to_string())?;
        let mut command = CommandBuilder::new(plan.program);
        command.args(plan.args);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| err.to_string())?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| err.to_string())?;
        let writer = pair.master.take_writer().map_err(|err| err.to_string())?;
        let terminal_id = Uuid::new_v4().to_string();
        let run_id = run.id.to_string();
        spawn_mobile_reader(terminal_id.clone(), run_id, reader, output);
        Ok(Self {
            terminal_id,
            master: pair.master,
            writer,
            child,
        })
    }

    fn input(&mut self, data: &str) -> std::io::Result<()> {
        self.writer.write_all(data.as_bytes())?;
        self.writer.flush()
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| err.to_string())
    }
}

impl Drop for MobilePtySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn spawn_mobile_reader(
    terminal_id: String,
    run_id: String,
    mut reader: Box<dyn Read + Send>,
    output: mpsc::Sender<StreamServerMessage>,
) {
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        let mut sanitizer = MobileTerminalTextSanitizer::default();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    let raw = String::from_utf8_lossy(&buffer[..size]);
                    let data = sanitizer.sanitize_chunk(&raw);
                    if data.is_empty() {
                        continue;
                    }
                    if output
                        .blocking_send(StreamServerMessage::TerminalOutput {
                            terminal_id: terminal_id.clone(),
                            run_id: run_id.clone(),
                            data,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = output.blocking_send(StreamServerMessage::TerminalClosed {
            terminal_id,
            run_id,
        });
    });
}

#[derive(Default)]
struct MobileTerminalTextSanitizer {
    state: TerminalControlState,
    pending_cr: bool,
}

#[derive(Default)]
enum TerminalControlState {
    #[default]
    Ground,
    Escape,
    EscapeIntermediate,
    Charset,
    Csi,
    Osc,
    OscEscape,
    ControlString,
    ControlStringEscape,
}

impl MobileTerminalTextSanitizer {
    fn sanitize_chunk(&mut self, data: &str) -> String {
        let mut output = String::with_capacity(data.len());
        for ch in data.chars() {
            self.sanitize_char(ch, &mut output);
        }
        output
    }

    fn sanitize_char(&mut self, ch: char, output: &mut String) {
        match self.state {
            TerminalControlState::Ground => self.sanitize_ground_char(ch, output),
            TerminalControlState::Escape => self.sanitize_escape_char(ch),
            TerminalControlState::EscapeIntermediate => self.sanitize_escape_intermediate_char(ch),
            TerminalControlState::Charset => {
                self.state = TerminalControlState::Ground;
            }
            TerminalControlState::Csi => self.sanitize_csi_char(ch),
            TerminalControlState::Osc => self.sanitize_osc_char(ch),
            TerminalControlState::OscEscape => self.sanitize_osc_escape_char(ch),
            TerminalControlState::ControlString => self.sanitize_control_string_char(ch),
            TerminalControlState::ControlStringEscape => {
                self.sanitize_control_string_escape_char(ch);
            }
        }
    }

    fn sanitize_ground_char(&mut self, ch: char, output: &mut String) {
        if self.pending_cr {
            self.pending_cr = false;
            if ch == '\n' {
                return;
            }
        }

        match ch {
            '\x1b' => self.state = TerminalControlState::Escape,
            '\u{009b}' => self.state = TerminalControlState::Csi,
            '\u{009d}' => self.state = TerminalControlState::Osc,
            '\u{0090}' | '\u{0098}' | '\u{009e}' | '\u{009f}' => {
                self.state = TerminalControlState::ControlString;
            }
            '\r' => {
                output.push('\n');
                self.pending_cr = true;
            }
            '\n' | '\t' => output.push(ch),
            ch if is_c0_control(ch) || is_c1_control(ch) => {}
            _ => output.push(ch),
        }
    }

    fn sanitize_escape_char(&mut self, ch: char) {
        self.state = match ch {
            '[' => TerminalControlState::Csi,
            ']' => TerminalControlState::Osc,
            'P' | 'X' | '^' | '_' => TerminalControlState::ControlString,
            '(' | ')' | '*' | '+' | '-' | '.' | '/' => TerminalControlState::Charset,
            '\x1b' => TerminalControlState::Escape,
            ch if is_escape_intermediate(ch) => TerminalControlState::EscapeIntermediate,
            _ => TerminalControlState::Ground,
        };
    }

    fn sanitize_escape_intermediate_char(&mut self, ch: char) {
        if ch == '\x1b' {
            self.state = TerminalControlState::Escape;
        } else if is_escape_final(ch) {
            self.state = TerminalControlState::Ground;
        }
    }

    fn sanitize_csi_char(&mut self, ch: char) {
        if ch == '\x1b' {
            self.state = TerminalControlState::Escape;
        } else if is_csi_final(ch) {
            self.state = TerminalControlState::Ground;
        }
    }

    fn sanitize_osc_char(&mut self, ch: char) {
        match ch {
            '\x07' => self.state = TerminalControlState::Ground,
            '\x1b' => self.state = TerminalControlState::OscEscape,
            _ => {}
        }
    }

    fn sanitize_osc_escape_char(&mut self, ch: char) {
        self.state = match ch {
            '\\' => TerminalControlState::Ground,
            '\x1b' => TerminalControlState::OscEscape,
            _ => TerminalControlState::Osc,
        };
    }

    fn sanitize_control_string_char(&mut self, ch: char) {
        if ch == '\x1b' {
            self.state = TerminalControlState::ControlStringEscape;
        }
    }

    fn sanitize_control_string_escape_char(&mut self, ch: char) {
        self.state = match ch {
            '\\' => TerminalControlState::Ground,
            '\x1b' => TerminalControlState::ControlStringEscape,
            _ => TerminalControlState::ControlString,
        };
    }
}

fn is_c0_control(ch: char) -> bool {
    matches!(ch, '\u{0000}'..='\u{001f}' | '\u{007f}')
}

fn is_c1_control(ch: char) -> bool {
    matches!(ch, '\u{0080}'..='\u{009f}')
}

fn is_escape_intermediate(ch: char) -> bool {
    matches!(ch, '\u{0020}'..='\u{002f}')
}

fn is_escape_final(ch: char) -> bool {
    matches!(ch, '\u{0030}'..='\u{007e}')
}

fn is_csi_final(ch: char) -> bool {
    matches!(ch, '\u{0040}'..='\u{007e}')
}

fn authorize(state: &BridgeServerState, headers: &HeaderMap) -> Result<String, BridgeApiError> {
    let auth = MobileAuthHeaders {
        device_id: header_value(headers, DEVICE_ID_HEADER),
        authorization: header_value(headers, "authorization"),
    };
    let store = state
        .pairing
        .lock()
        .map_err(|_| BridgeApiError::internal("pairing lock poisoned"))?;
    authenticated_device_id(&store, &auth).map_err(|_| BridgeApiError::unauthorized())
}

fn authorize_stream(
    state: &BridgeServerState,
    headers: &HeaderMap,
    query: &StreamAuthQuery,
) -> Result<String, BridgeApiError> {
    let auth = stream_auth_headers(headers, query);
    let store = state
        .pairing
        .lock()
        .map_err(|_| BridgeApiError::internal("pairing lock poisoned"))?;
    authenticated_device_id(&store, &auth).map_err(|_| BridgeApiError::unauthorized())
}

fn stream_auth_headers(headers: &HeaderMap, query: &StreamAuthQuery) -> MobileAuthHeaders {
    MobileAuthHeaders {
        device_id: header_value(headers, DEVICE_ID_HEADER).or_else(|| {
            query
                .device_id
                .as_ref()
                .filter(|value| !value.is_empty())
                .cloned()
        }),
        authorization: header_value(headers, "authorization").or_else(|| {
            query
                .token
                .as_ref()
                .filter(|value| !value.is_empty())
                .map(|token| format!("Bearer {token}"))
        }),
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn build_mobile_dashboard(state: &BridgeServerState) -> Result<DashboardState, BridgeApiError> {
    let registry = registry(state)?;
    let active = registry
        .list_active_runs()
        .map_err(|err| BridgeApiError::internal(err.to_string()))?;
    let active_repo_path = registry
        .active_repo_path()
        .map_err(|err| BridgeApiError::internal(err.to_string()))?;
    Ok(build_dashboard_state(
        active,
        active_repo_path,
        host_tool_statuses(),
        None,
    ))
}

fn registry(state: &BridgeServerState) -> Result<SqliteRegistry, BridgeApiError> {
    SqliteRegistry::open(&state.registry_path)
        .map_err(|err| BridgeApiError::internal(err.to_string()))
}

fn to_json(message: &StreamServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|err| {
        serde_json::json!({
            "type": "error",
            "message": err.to_string(),
        })
        .to_string()
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairClaimRequest {
    code: String,
    device_name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamAuthQuery {
    device_id: Option<String>,
    token: Option<String>,
}

#[cfg(test)]
fn mobile_pwa_route_paths() -> &'static [&'static str] {
    MOBILE_PWA_ROUTE_PATHS
}

struct BridgeApiError {
    status: StatusCode,
    message: String,
}

impl BridgeApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "mobile bridge authentication required".to_string(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<crate::mobile_bridge::PairingError> for BridgeApiError {
    fn from(value: crate::mobile_bridge::PairingError) -> Self {
        Self::bad_request(value.to_string())
    }
}

impl IntoResponse for BridgeApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message,
            })),
        )
            .into_response()
    }
}

impl From<BridgeApiError> for DesktopError {
    fn from(value: BridgeApiError) -> Self {
        Self::Message(value.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_query_auth_can_authenticate_browser_websocket_clients() {
        let mut store = PairingStore::default();
        let code = store.issue_code(PairingTime::from_epoch_seconds(100));
        let device = store
            .claim_code(
                &code.code,
                "Chrome PWA",
                PairingTime::from_epoch_seconds(120),
            )
            .unwrap();
        let query = StreamAuthQuery {
            device_id: Some(device.id.clone()),
            token: Some(device.token),
        };

        let auth = stream_auth_headers(&HeaderMap::new(), &query);

        assert_eq!(authenticated_device_id(&store, &auth).unwrap(), device.id);
    }

    #[test]
    fn bridge_declares_mobile_pwa_asset_routes() {
        assert_eq!(
            mobile_pwa_route_paths(),
            &[
                "/mobile",
                "/mobile/",
                "/mobile/app.js",
                "/mobile/styles.css",
                "/mobile/manifest.webmanifest",
                "/mobile/sw.js",
                "/mobile/icon.svg",
            ],
        );
    }

    #[test]
    fn terminal_snapshot_queue_is_safe_inside_async_runtime() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let (tx, mut rx) = mpsc::channel(1);

            queue_terminal_snapshot(&tx, "run-1".to_string(), "pane text".to_string()).unwrap();

            assert_eq!(
                rx.recv().await.unwrap(),
                StreamServerMessage::TerminalSnapshot {
                    run_id: "run-1".to_string(),
                    data: "pane text".to_string(),
                }
            );
        });
    }

    #[test]
    fn mobile_terminal_output_drops_vt_control_sequences() {
        let mut sanitizer = MobileTerminalTextSanitizer::default();

        assert_eq!(
            sanitizer.sanitize_chunk("\x1b[39mfeat\x1b[49m\x1b[2m\r\n\x1b(B\x1b[Kqueued\x1b[0m"),
            "feat\nqueued"
        );
        assert_eq!(sanitizer.sanitize_chunk("\x1b[3"), "");
        assert_eq!(sanitizer.sanitize_chunk("8;5;151mgreen"), "green");
        assert_eq!(sanitizer.sanitize_chunk("\x1b]0;window title"), "");
        assert_eq!(sanitizer.sanitize_chunk("\x07done"), "done");
    }
}
