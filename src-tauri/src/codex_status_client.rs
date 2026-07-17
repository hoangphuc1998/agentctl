use std::time::Duration;

use agentctl_core::{
    codex_status::{
        parse_thread_list_response, CodexThreadSnapshot, CODEX_THREAD_LIST_RESPONSE_ID,
    },
    commands::CODEX_APP_SERVER_URL,
};
use serde_json::{json, Value};
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

const RPC_TIMEOUT: Duration = Duration::from_secs(2);

pub fn load_codex_thread_statuses() -> Result<Vec<CodexThreadSnapshot>, String> {
    let (mut socket, _) = connect(CODEX_APP_SERVER_URL)
        .map_err(|err| format!("failed to connect to Codex app-server: {err}"))?;
    configure_timeout(&mut socket)?;

    send_json(
        &mut socket,
        json!({
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "agentctl",
                    "title": "Agent Manager",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
    )?;
    wait_for_response(&mut socket, 1)?;
    send_json(&mut socket, json!({ "method": "initialized" }))?;
    send_json(
        &mut socket,
        json!({
            "id": CODEX_THREAD_LIST_RESPONSE_ID,
            "method": "thread/list",
            "params": {
                "limit": 100,
                "useStateDbOnly": true,
                "sortKey": "updated_at",
                "sortDirection": "desc"
            }
        }),
    )?;

    loop {
        let message = read_text(&mut socket)?;
        if let Some(threads) = parse_thread_list_response(&message)? {
            let _ = socket.close(None);
            return Ok(threads);
        }
    }
}

fn configure_timeout(
    socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>,
) -> Result<(), String> {
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_read_timeout(Some(RPC_TIMEOUT))
            .map_err(|err| format!("failed to configure Codex status timeout: {err}"))?;
        stream
            .set_write_timeout(Some(RPC_TIMEOUT))
            .map_err(|err| format!("failed to configure Codex status timeout: {err}"))?;
    }
    Ok(())
}

fn send_json(
    socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    value: Value,
) -> Result<(), String> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|err| format!("failed to send Codex status request: {err}"))
}

fn wait_for_response(
    socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    response_id: u64,
) -> Result<(), String> {
    loop {
        let message = read_text(socket)?;
        let value: Value = serde_json::from_str(&message)
            .map_err(|err| format!("invalid Codex initialize response: {err}"))?;
        if value.get("id").and_then(Value::as_u64) == Some(response_id) {
            if let Some(error) = value.get("error") {
                return Err(format!("Codex initialize failed: {error}"));
            }
            return Ok(());
        }
    }
}

fn read_text(
    socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>,
) -> Result<String, String> {
    loop {
        match socket.read() {
            Ok(Message::Text(text)) => return Ok(text.to_string()),
            Ok(Message::Binary(bytes)) => {
                return String::from_utf8(bytes.to_vec())
                    .map_err(|err| format!("invalid Codex status response encoding: {err}"));
            }
            Ok(Message::Close(frame)) => {
                return Err(format!(
                    "Codex app-server closed the status connection: {frame:?}"
                ));
            }
            Ok(_) => {}
            Err(err) => return Err(format!("failed to read Codex status response: {err}")),
        }
    }
}
