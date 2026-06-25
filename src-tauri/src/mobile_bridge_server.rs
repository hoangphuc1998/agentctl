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
        Path, State,
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
    models::{host_tool_statuses, ActionResult, DashboardState},
    services::build_dashboard_state,
    terminal_plan::tmux_attach_command,
};

const TMUX_SESSION: &str = "agentctl";
const DEVICE_ID_HEADER: &str = "x-agent-manager-device";

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
    websocket: WebSocketUpgrade,
) -> Response {
    match authorize(&state, &headers) {
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
            let _ = pty_tx.blocking_send(StreamServerMessage::TerminalSnapshot {
                run_id: run_id.clone(),
                data: snapshot.visible_text,
            });
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
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    let data = String::from_utf8_lossy(&buffer[..size]).to_string();
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
