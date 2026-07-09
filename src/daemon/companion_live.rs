#[cfg(feature = "remote")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "remote")]
use anyhow::{Context, Result};

#[cfg(feature = "remote")]
use super::device_listener::{DeviceListenerHandle, DeviceListenerRequest};
#[cfg(feature = "remote")]
use super::remote_gate::now_secs;

#[cfg(feature = "remote")]
pub(super) const COMPANION_LIVE_HTTP_MAX_BODY: usize = 1 << 20;

#[cfg(feature = "remote")]
pub(super) const COMPANION_LIVE_HTTP_MAX_HEADER: usize = 16 * 1024;

#[cfg(feature = "remote")]
const COMPANION_LIVE_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(feature = "remote")]
pub struct CompanionLiveEndpointHandle {
    pub addr: std::net::SocketAddr,
    pub base_url: String,
    pub message_url: String,
    pub events_url: String,
    pub listener: DeviceListenerHandle,
}

#[cfg(feature = "remote")]
#[derive(Clone)]
struct CompanionLiveState {
    registry: crate::device_registry::DeviceRegistry,
    device_id: Option<String>,
    base_url: String,
    message_url: String,
    events_url: String,
    request_rx: Arc<Mutex<std::sync::mpsc::Receiver<DeviceListenerRequest>>>,
    pending: Arc<Mutex<Option<CompanionLivePendingApproval>>>,
    connected_device_id: Arc<Mutex<Option<String>>>,
    poll_timeout: std::time::Duration,
}

#[cfg(feature = "remote")]
struct CompanionLivePendingApproval {
    request: crate::session::ApprovalRequestMsg,
    response_tx: std::sync::mpsc::Sender<Result<crate::session::ApprovalResponseMsg>>,
}

#[cfg(feature = "remote")]
struct CompanionHttpRequest {
    method: String,
    path: String,
    body: String,
}

#[cfg(feature = "remote")]
struct CompanionHttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: String,
}

/// Browser-compatible local companion endpoint. This is intentionally small and
/// dependency-free: HTTP POST carries `CompanionTransportMsg` JSON, and a minimal
/// SSE endpoint proves browser-readable event framing before the full approval
/// bridge is attached.
#[cfg(feature = "remote")]
pub fn spawn_companion_live_endpoint(
    registry: crate::device_registry::DeviceRegistry,
    device_id: Option<String>,
) -> Result<CompanionLiveEndpointHandle> {
    spawn_companion_live_endpoint_with_timeout(registry, device_id, COMPANION_LIVE_HTTP_TIMEOUT)
}

#[cfg(feature = "remote")]
pub(super) fn spawn_companion_live_endpoint_with_timeout(
    registry: crate::device_registry::DeviceRegistry,
    device_id: Option<String>,
    timeout: std::time::Duration,
) -> Result<CompanionLiveEndpointHandle> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .context("PWA live companion endpoint bind 실패")?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");
    let message_url = format!("{base_url}/message");
    let events_url = format!("{base_url}/events");
    let (request_tx, request_rx) = std::sync::mpsc::channel::<DeviceListenerRequest>();
    let state = CompanionLiveState {
        registry,
        device_id,
        base_url: base_url.clone(),
        message_url: message_url.clone(),
        events_url: events_url.clone(),
        request_rx: Arc::new(Mutex::new(request_rx)),
        pending: Arc::new(Mutex::new(None)),
        connected_device_id: Arc::new(Mutex::new(None)),
        poll_timeout: timeout,
    };
    let thread = std::thread::spawn(move || {
        let state = std::sync::Arc::new(state);
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                break;
            };
            let state = state.clone();
            std::thread::spawn(move || {
                let _ = handle_companion_live_connection(&mut stream, &state, timeout);
            });
        }
    });

    Ok(CompanionLiveEndpointHandle {
        addr,
        base_url,
        message_url,
        events_url,
        listener: DeviceListenerHandle { request_tx, thread },
    })
}

#[cfg(feature = "remote")]
fn handle_companion_live_connection(
    stream: &mut std::net::TcpStream,
    state: &CompanionLiveState,
    timeout: std::time::Duration,
) -> Result<()> {
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let request = match read_companion_http_request(stream) {
        Ok(request) => request,
        Err(err) if is_timeout_error(&err) => {
            return write_companion_http_response(
                stream,
                companion_error_response(408, "Request Timeout", "live companion request timeout"),
            );
        }
        Err(_) => {
            return write_companion_http_response(
                stream,
                companion_error_response(400, "Bad Request", "malformed live companion request"),
            );
        }
    };
    let response = companion_live_response(state, request);
    write_companion_http_response(stream, response)
}

#[cfg(feature = "remote")]
fn read_companion_http_request(stream: &mut std::net::TcpStream) -> Result<CompanionHttpRequest> {
    use std::io::Read;

    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    let header_end = loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            anyhow::bail!("connection closed before HTTP header");
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() > COMPANION_LIVE_HTTP_MAX_HEADER + COMPANION_LIVE_HTTP_MAX_BODY {
            anyhow::bail!("HTTP request too large");
        }
        if let Some(pos) = find_http_header_end(&bytes) {
            break pos;
        }
        if bytes.len() > COMPANION_LIVE_HTTP_MAX_HEADER {
            anyhow::bail!("HTTP header too large");
        }
    };

    let header = std::str::from_utf8(&bytes[..header_end])?;
    let mut lines = header.split("\r\n");
    let request_line = lines.next().context("missing HTTP request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("missing HTTP method")?.to_string();
    let path = parts.next().context("missing HTTP path")?.to_string();
    let _version = parts.next().context("missing HTTP version")?;

    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse::<usize>()?;
            if content_length > COMPANION_LIVE_HTTP_MAX_BODY {
                anyhow::bail!("HTTP body too large");
            }
        }
    }

    let body_start = header_end + 4;
    let mut body = bytes[body_start..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            anyhow::bail!("connection closed before HTTP body");
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(content_length);
    let body = String::from_utf8(body)?;

    Ok(CompanionHttpRequest { method, path, body })
}

#[cfg(feature = "remote")]
fn find_http_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(feature = "remote")]
fn companion_live_response(
    state: &CompanionLiveState,
    request: CompanionHttpRequest,
) -> CompanionHttpResponse {
    let path = request.path.split('?').next().unwrap_or(&request.path);
    match (request.method.as_str(), path) {
        ("OPTIONS", _) => CompanionHttpResponse {
            status: 204,
            reason: "No Content",
            content_type: "text/plain; charset=utf-8",
            body: String::new(),
        },
        ("GET", "/") | ("GET", "/health") => companion_json_response(
            200,
            "OK",
            serde_json::json!({
                "status": "ready",
                "protocol_version": crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
                "base_url": state.base_url,
                "message_url": state.message_url,
                "events_url": state.events_url,
            })
            .to_string(),
        ),
        ("GET", "/events") => companion_events_response(state),
        ("POST", "/message") => companion_message_response(state, &request.body),
        _ => companion_error_response(404, "Not Found", "unknown live companion endpoint"),
    }
}

#[cfg(feature = "remote")]
fn companion_events_response(state: &CompanionLiveState) -> CompanionHttpResponse {
    if state
        .connected_device_id
        .lock()
        .map(|connected| connected.is_none())
        .unwrap_or(true)
    {
        return companion_error_response(409, "Conflict", "companion hello required");
    }

    let message = match next_live_approval_request(state) {
        Ok(Some(request)) => crate::session::CompanionTransportMsg::ApprovalRequest { request },
        Ok(None) => crate::session::CompanionTransportMsg::Ping {
            nonce: "ready".into(),
        },
        Err(err) => {
            return companion_error_response(500, "Internal Server Error", &err.to_string());
        }
    };

    companion_sse_response(state, message)
}

#[cfg(feature = "remote")]
fn companion_message_response(state: &CompanionLiveState, body: &str) -> CompanionHttpResponse {
    let message = match crate::session::parse_companion_transport_json(body) {
        Ok(message) => message,
        Err(err) => {
            return companion_error_response(
                400,
                "Bad Request",
                &format!("malformed live companion message: {err}"),
            );
        }
    };

    match companion_live_reply(state, message) {
        Ok(reply) => companion_envelope_response(200, "OK", reply),
        Err((status, reason, message)) => companion_error_response(status, reason, &message),
    }
}

#[cfg(feature = "remote")]
fn companion_live_reply(
    state: &CompanionLiveState,
    message: crate::session::CompanionTransportMsg,
) -> std::result::Result<crate::session::CompanionTransportMsg, (u16, &'static str, String)> {
    match message {
        crate::session::CompanionTransportMsg::Hello {
            device_id,
            noise_pubkey_hex,
            approval_pubkey_hex,
            ..
        } => {
            validate_companion_hello(state, &device_id, &noise_pubkey_hex, &approval_pubkey_hex)
                .map_err(|err| (403, "Forbidden", format!("companion hello rejected: {err}")))?;
            match state.connected_device_id.lock() {
                Ok(mut connected) => {
                    *connected = Some(device_id.clone());
                }
                Err(_) => {
                    return Err((
                        500,
                        "Internal Server Error",
                        "live companion session lock failed".into(),
                    ));
                }
            }
            Ok(crate::session::CompanionTransportMsg::Pong {
                nonce: format!("hello:{device_id}"),
            })
        }
        crate::session::CompanionTransportMsg::Ping { nonce } => {
            Ok(crate::session::CompanionTransportMsg::Pong { nonce })
        }
        crate::session::CompanionTransportMsg::ApprovalResponse { response } => {
            complete_live_approval_response(state, response).map_err(|err| {
                (
                    409,
                    "Conflict",
                    format!("approval_response bridge failed: {err}"),
                )
            })?;
            Ok(crate::session::CompanionTransportMsg::Pong {
                nonce: "approval_response".into(),
            })
        }
        crate::session::CompanionTransportMsg::ApprovalRequest { .. } => Err((
            400,
            "Bad Request",
            "browser companions must not send approval_request messages".into(),
        )),
        crate::session::CompanionTransportMsg::Pong { .. }
        | crate::session::CompanionTransportMsg::Error { .. } => Err((
            400,
            "Bad Request",
            "unsupported client live companion message".into(),
        )),
    }
}

#[cfg(feature = "remote")]
fn next_live_approval_request(
    state: &CompanionLiveState,
) -> Result<Option<crate::session::ApprovalRequestMsg>> {
    if let Some(pending) = state
        .pending
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|pending| pending.request.clone()))
    {
        return Ok(Some(pending));
    }

    loop {
        let item = match state.request_rx.lock() {
            Ok(rx) => match rx.recv_timeout(state.poll_timeout) {
                Ok(item) => item,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return Ok(None),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("live approval request channel closed")
                }
            },
            Err(_) => anyhow::bail!("live approval request lock failed"),
        };

        if item.request.expires_at <= now_secs() {
            let _ = item.response_tx.send(Err(anyhow::anyhow!(
                "live approval expired before delivery"
            )));
            continue;
        }

        let request = item.request.clone();
        match state.pending.lock() {
            Ok(mut pending) => {
                *pending = Some(CompanionLivePendingApproval {
                    request: item.request,
                    response_tx: item.response_tx,
                });
            }
            Err(_) => anyhow::bail!("live pending approval lock failed"),
        }
        return Ok(Some(request));
    }
}

#[cfg(feature = "remote")]
fn complete_live_approval_response(
    state: &CompanionLiveState,
    response: crate::session::ApprovalResponseMsg,
) -> Result<()> {
    let pending = match state.pending.lock() {
        Ok(mut pending) => pending.take(),
        Err(_) => anyhow::bail!("live pending approval lock failed"),
    };
    let Some(pending) = pending else {
        anyhow::bail!("no pending live approval");
    };

    if pending.request.approval_id != response.approval_id
        || pending.request.nonce != response.nonce
    {
        let mut guard = state
            .pending
            .lock()
            .map_err(|_| anyhow::anyhow!("live pending approval lock failed"))?;
        *guard = Some(pending);
        anyhow::bail!("approval_response does not match pending request");
    }

    pending
        .response_tx
        .send(Ok(response))
        .map_err(|_| anyhow::anyhow!("gate waiter is no longer waiting for live approval"))?;
    Ok(())
}

#[cfg(feature = "remote")]
fn companion_sse_response(
    state: &CompanionLiveState,
    message: crate::session::CompanionTransportMsg,
) -> CompanionHttpResponse {
    let body = match crate::session::companion_transport_json(&message) {
        Ok(message) => format!(
            "event: message\ndata: {message}\n\n: message_url={}\n\n",
            state.message_url
        ),
        Err(_) => {
            "event: message\ndata: {\"type\":\"error\",\"message\":\"event encode failure\"}\n\n"
                .to_string()
        }
    };
    CompanionHttpResponse {
        status: 200,
        reason: "OK",
        content_type: "text/event-stream; charset=utf-8",
        body,
    }
}

#[cfg(feature = "remote")]
fn validate_companion_hello(
    state: &CompanionLiveState,
    device_id: &str,
    noise_pubkey_hex: &str,
    approval_pubkey_hex: &str,
) -> Result<()> {
    let device = state
        .registry
        .select_device(state.device_id.as_deref())
        .context("live companion device selection failed")?;
    if device.id != device_id {
        anyhow::bail!("device_id mismatch");
    }
    if !crate::pairing::hex_encode(&device.noise_pubkey).eq_ignore_ascii_case(noise_pubkey_hex) {
        anyhow::bail!("noise_pubkey mismatch");
    }
    if !crate::pairing::hex_encode(&device.approval_pubkey)
        .eq_ignore_ascii_case(approval_pubkey_hex)
    {
        anyhow::bail!("approval_pubkey mismatch");
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn companion_envelope_response(
    status: u16,
    reason: &'static str,
    message: crate::session::CompanionTransportMsg,
) -> CompanionHttpResponse {
    match crate::session::companion_transport_json(&message) {
        Ok(body) => companion_json_response(status, reason, body),
        Err(err) => companion_error_response(500, "Internal Server Error", &err.to_string()),
    }
}

#[cfg(feature = "remote")]
fn companion_json_response(
    status: u16,
    reason: &'static str,
    body: String,
) -> CompanionHttpResponse {
    CompanionHttpResponse {
        status,
        reason,
        content_type: "application/json; charset=utf-8",
        body,
    }
}

#[cfg(feature = "remote")]
fn companion_error_response(
    status: u16,
    reason: &'static str,
    message: &str,
) -> CompanionHttpResponse {
    companion_envelope_response(
        status,
        reason,
        crate::session::CompanionTransportMsg::Error {
            message: message.to_string(),
        },
    )
}

#[cfg(feature = "remote")]
fn write_companion_http_response(
    stream: &mut std::net::TcpStream,
    response: CompanionHttpResponse,
) -> Result<()> {
    use std::io::Write;

    let body = response.body.as_bytes();
    let headers = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: content-type\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         X-Content-Type-Options: nosniff\r\n\
         \r\n",
        response.status,
        response.reason,
        response.content_type,
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

#[cfg(feature = "remote")]
fn is_timeout_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .map(|io| {
                matches!(
                    io.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                )
            })
            .unwrap_or(false)
    })
}
