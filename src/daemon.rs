//! 원격 승인 로컬 게이트 데몬 (M1 slice 1, unix 전용).
//!
//! 셸 hook(`ai __gate`)이 Unix 소켓으로 **블로킹 질의** → 데몬이 게이트 결정
//! (`gate::decide_gate` shared-core, §30-13) 회신. hook↔데몬은 **같은 머신의 신뢰된
//! 로컬 IPC**다(phone↔데몬의 Noise E2E[M0.5]와 다른 채널). 컨텍스트 스냅샷·폰 왕복·
//! nonce 소비·페어링은 M1 후속 슬라이스에서 데몬에 결합한다.
//!
//! 프레이밍: 개행 구분 JSON(연결당 1요청/1회신).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::gate::{self, GateDecision};

/// 게이트 질의 요청.
#[derive(Serialize, Deserialize, Debug)]
pub struct GateRequest {
    pub command: String,
    #[serde(default)]
    pub context_origin: Option<crate::context::RemoteContextOrigin>,
}

/// 게이트 결정 회신.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GateReply {
    pub decision: String, // "allow" | "block"
    pub reason: String,
}

impl GateReply {
    pub fn allow() -> Self {
        Self {
            decision: "allow".into(),
            reason: String::new(),
        }
    }
    pub fn block(reason: String) -> Self {
        Self {
            decision: "block".into(),
            reason,
        }
    }
    pub fn is_allow(&self) -> bool {
        self.decision == "allow"
    }
}

/// 게이트 소켓 경로: `<config_dir>/gate.sock`.
pub fn socket_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("gate.sock"))
}

/// 디바이스 연결 소켓 경로: `<config_dir>/device.sock`.
/// RA-1 device listener가 사용하는 daemon-owned endpoint다.
#[cfg(feature = "remote")]
pub fn device_socket_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("device.sock"))
}

#[cfg(feature = "remote")]
const COMPANION_LIVE_HTTP_MAX_BODY: usize = 1 << 20;

#[cfg(feature = "remote")]
const COMPANION_LIVE_HTTP_MAX_HEADER: usize = 16 * 1024;

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
fn spawn_companion_live_endpoint_with_timeout(
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

/// armed/allow_high가 주어졌을 때의 게이트 회신(순수, shared-core).
pub fn decide_with(command: &str, armed: bool, allow_high: bool) -> GateReply {
    match gate::decide_gate(command, armed, allow_high) {
        GateDecision::Allow => GateReply::allow(),
        GateDecision::Block { reason } => GateReply::block(reason),
    }
}

/// 현재 armed 상태를 읽어 게이트 회신을 만든다(데몬 핸들러용).
pub fn decide_request(command: &str) -> GateReply {
    let (armed, allow_high) = gate::armed_path()
        .ok()
        .and_then(|p| gate::load_arm_state(&p))
        .map(|s| (true, s.allow_high))
        .unwrap_or((false, false));
    decide_with(command, armed, allow_high)
}

#[derive(Clone)]
struct DaemonRuntime {
    #[cfg(feature = "remote")]
    remote: Option<RemoteDaemonState>,
}

#[cfg(feature = "remote")]
#[derive(Clone)]
struct RemoteDaemonState {
    registry: crate::device_registry::DeviceRegistry,
    device_id: Option<String>,
    listener: Arc<Mutex<DeviceListenerHandle>>,
    approval_ttl: u64,
    response_timeout: std::time::Duration,
}

impl DaemonRuntime {
    fn local() -> Self {
        Self {
            #[cfg(feature = "remote")]
            remote: None,
        }
    }

    #[cfg(feature = "remote")]
    fn remote(
        registry: crate::device_registry::DeviceRegistry,
        listener: DeviceListenerHandle,
        device_id: Option<String>,
    ) -> Self {
        Self {
            remote: Some(RemoteDaemonState {
                registry,
                device_id,
                listener: Arc::new(Mutex::new(listener)),
                approval_ttl: 60,
                response_timeout: std::time::Duration::from_secs(30),
            }),
        }
    }

    fn decide(&self, req: &GateRequest) -> GateReply {
        let (armed, allow_high) = gate::armed_path()
            .ok()
            .and_then(|p| gate::load_arm_state(&p))
            .map(|s| (true, s.allow_high))
            .unwrap_or((false, false));

        #[cfg(feature = "remote")]
        if let Some(remote) = &self.remote {
            return remote.decide_with_arm(req, armed, allow_high);
        }

        decide_with(&req.command, armed, allow_high)
    }
}

#[cfg(feature = "remote")]
impl RemoteDaemonState {
    fn decide_with_arm(&self, req: &GateRequest, armed: bool, allow_high: bool) -> GateReply {
        let origin = req
            .context_origin
            .clone()
            .unwrap_or_else(crate::context::RemoteContextOrigin::gather);
        let issued_context_hash =
            crate::context::remote_context_hash_for_origin(&req.command, &origin);
        let Ok(listener) = self.listener.lock() else {
            return GateReply::block("원격 디바이스 리스너 lock 실패".into());
        };

        decide_with_remote_listener(
            &self.registry,
            &listener,
            RemoteGateRun {
                command: &req.command,
                armed,
                allow_high,
                now: now_secs(),
                ttl: self.approval_ttl,
                device_id: self.device_id.as_deref(),
                issued_context_hash: &issued_context_hash,
                context_origin: Some(&origin),
                current_context_hash: None,
                response_timeout: self.response_timeout,
            },
        )
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// RA-3 gate-flow가 로컬 결정으로 끝나는지, 등록 디바이스 승인 왕복이 필요한지.
#[cfg(feature = "remote")]
#[derive(Debug)]
pub enum RemoteGateStep {
    Local(GateReply),
    NeedsRemote(RemoteApprovalPlan),
}

/// 등록 디바이스로 보낼 pending approval 요청.
#[cfg(feature = "remote")]
#[derive(Debug)]
pub struct RemoteApprovalPlan {
    pub device_id: String,
    pub pending: crate::approval::PendingApproval,
    pub request: crate::session::ApprovalRequestMsg,
}

#[cfg(feature = "remote")]
pub struct RemoteGatePlanInput<'a> {
    pub command: &'a str,
    pub armed: bool,
    pub allow_high: bool,
    pub device_id: Option<&'a str>,
    pub now: u64,
    pub ttl: u64,
    pub context_hash: &'a str,
}

/// RA-3 첫 결선 경계: armed + High opt-in 명령은 즉시 allow하지 않고 registered
/// device 승인 요청으로 승격한다. Low/Medium은 로컬 allow, Critical은 로컬 block,
/// High without opt-in은 기존 opt-in 안내 block을 유지한다.
#[cfg(feature = "remote")]
pub fn plan_remote_gate(
    registry: &crate::device_registry::DeviceRegistry,
    input: RemoteGatePlanInput<'_>,
) -> Result<RemoteGateStep> {
    use crate::risk::RiskLevel;

    if !input.armed {
        return Ok(RemoteGateStep::Local(GateReply::allow()));
    }

    let assessment = crate::risk::assess(input.command);
    match assessment.level {
        RiskLevel::Low | RiskLevel::Medium => Ok(RemoteGateStep::Local(GateReply::allow())),
        RiskLevel::Critical => Ok(RemoteGateStep::Local(decide_with(
            input.command,
            true,
            input.allow_high,
        ))),
        RiskLevel::High if !input.allow_high => Ok(RemoteGateStep::Local(decide_with(
            input.command,
            true,
            false,
        ))),
        RiskLevel::High => {
            let device = registry
                .select_device(input.device_id)
                .context("원격 승인 디바이스 선택 실패")?;
            let nonce = crate::approval::gen_nonce();
            let approval_id = crate::approval::gen_nonce().to_vec();
            let pending = crate::approval::PendingApproval {
                approval_id,
                nonce,
                expires_at: input.now.saturating_add(input.ttl),
                context_hash: input.context_hash.to_string(),
                device_epoch: device.epoch,
            };
            let command_masked = crate::mask::Masker::baseline().mask(input.command).text;
            let request =
                crate::session::ApprovalRequestMsg::from_pending(&pending, &command_masked);
            Ok(RemoteGateStep::NeedsRemote(RemoteApprovalPlan {
                device_id: device.id.clone(),
                pending,
                request,
            }))
        }
    }
}

/// RA-3 응답 접기: listener에서 받은 서명 응답을 등록 디바이스/nonce/context에 대해
/// 검증하고 최종 gate reply로 변환한다. nonce 미소비, 미등록 디바이스, 서명/TOCTOU
/// 실패는 모두 fail-closed block이다.
#[cfg(feature = "remote")]
pub fn finish_remote_gate_response(
    registry: &crate::device_registry::DeviceRegistry,
    plan: &RemoteApprovalPlan,
    nonces: &mut crate::approval::NonceStore,
    now: u64,
    current_context_hash: &str,
    response: &crate::session::ApprovalResponseMsg,
) -> GateReply {
    if !nonces.consume(&plan.pending.nonce, now) {
        return GateReply::block("원격 승인 nonce가 없거나 만료됨(fail-closed)".into());
    }

    match crate::device_registry::validate_registered_response(
        registry,
        &plan.device_id,
        &plan.pending,
        now,
        current_context_hash,
        response,
    ) {
        Ok(crate::approval::ApprovalOutcome::Approved) => GateReply::allow(),
        Ok(crate::approval::ApprovalOutcome::Rejected) => {
            GateReply::block("원격 디바이스가 실행을 거부함".into())
        }
        Ok(crate::approval::ApprovalOutcome::Invalid(reason)) => {
            GateReply::block(format!("원격 승인 검증 실패: {reason:?}"))
        }
        Err(err) => GateReply::block(format!("원격 승인 처리 실패: {err}")),
    }
}

#[cfg(feature = "remote")]
pub fn remote_timeout_reply() -> GateReply {
    GateReply::block("원격 승인 시간 초과(fail-closed)".into())
}

#[cfg(feature = "remote")]
pub struct RemoteGateRun<'a> {
    pub command: &'a str,
    pub armed: bool,
    pub allow_high: bool,
    pub now: u64,
    pub ttl: u64,
    pub device_id: Option<&'a str>,
    pub issued_context_hash: &'a str,
    pub context_origin: Option<&'a crate::context::RemoteContextOrigin>,
    pub current_context_hash: Option<&'a str>,
    pub response_timeout: std::time::Duration,
}

/// RA-3 queue-backed listener 결선: High opt-in 명령이면 device listener에 승인 요청을
/// 보내고 응답을 기다려 최종 GateReply로 접는다. `current_context_hash`가 None이면
/// 응답 검증 직전에 현재 컨텍스트를 재계산해 TOCTOU drift를 fail-closed 처리한다.
#[cfg(feature = "remote")]
pub fn decide_with_remote_listener(
    registry: &crate::device_registry::DeviceRegistry,
    listener: &DeviceListenerHandle,
    run: RemoteGateRun<'_>,
) -> GateReply {
    let step = match plan_remote_gate(
        registry,
        RemoteGatePlanInput {
            command: run.command,
            armed: run.armed,
            allow_high: run.allow_high,
            device_id: run.device_id,
            now: run.now,
            ttl: run.ttl,
            context_hash: run.issued_context_hash,
        },
    ) {
        Ok(step) => step,
        Err(err) => return GateReply::block(format!("원격 승인 계획 실패: {err}")),
    };
    let plan = match step {
        RemoteGateStep::Local(reply) => return reply,
        RemoteGateStep::NeedsRemote(plan) => plan,
    };

    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(plan.pending.nonce, plan.pending.expires_at);
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    let request = DeviceListenerRequest {
        request: plan.request.clone(),
        response_tx,
        accept_timeout: run.response_timeout,
    };
    if listener.request_tx.send(request).is_err() {
        return GateReply::block("원격 디바이스 리스너에 승인 요청 전송 실패".into());
    }

    match response_rx.recv_timeout(run.response_timeout) {
        Ok(Ok(response)) => {
            let recomputed_context_hash = run
                .current_context_hash
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    run.context_origin.map_or_else(
                        || crate::context::remote_context_hash(run.command),
                        |origin| {
                            crate::context::remote_context_hash_for_origin(run.command, origin)
                        },
                    )
                });
            finish_remote_gate_response(
                registry,
                &plan,
                &mut nonces,
                run.now,
                &recomputed_context_hash,
                &response,
            )
        }
        Ok(Err(err)) => GateReply::block(format!("원격 디바이스 승인 왕복 실패: {err}")),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => remote_timeout_reply(),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            GateReply::block("원격 디바이스 리스너 응답 채널 종료".into())
        }
    }
}

/// 데몬을 실행한다: 소켓 바인드 → accept 루프(연결마다 1요청 처리). 무한 루프.
pub async fn serve(path: &Path) -> Result<()> {
    serve_with_runtime(path, DaemonRuntime::local()).await
}

/// remote-enabled daemon: gate socket 요청에서 High opt-in을 등록 디바이스 승인 왕복으로
/// 처리한다. registry는 시작 시점 스냅샷이며, 페어링 변경은 daemon 재시작 후 반영된다.
#[cfg(feature = "remote")]
pub async fn serve_with_remote(
    path: &Path,
    registry: crate::device_registry::DeviceRegistry,
    device_listener: DeviceListenerHandle,
    device_id: Option<String>,
) -> Result<()> {
    serve_with_runtime(
        path,
        DaemonRuntime::remote(registry, device_listener, device_id),
    )
    .await
}

async fn serve_with_runtime(path: &Path, runtime: DaemonRuntime) -> Result<()> {
    // stale 소켓 정리(단일 데몬 가정).
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener = tokio::net::UnixListener::bind(path)
        .with_context(|| format!("게이트 소켓 바인드 실패: {}", path.display()))?;
    loop {
        let (stream, _) = listener.accept().await?;
        let runtime = runtime.clone();
        tokio::spawn(async move {
            let _ = handle_conn(stream, runtime).await;
        });
    }
}

/// 단일 연결 처리: 요청 한 줄을 읽어 결정 회신.
async fn handle_conn(stream: tokio::net::UnixStream, runtime: DaemonRuntime) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let reply = match serde_json::from_str::<GateRequest>(line.trim()) {
        Ok(req) => runtime.decide(&req),
        // 잘못된 요청 = fail-closed(차단). hook은 비0으로 명령을 취소한다.
        Err(_) => GateReply::block("잘못된 게이트 요청(fail-closed)".into()),
    };
    let mut out = serde_json::to_string(&reply)?;
    out.push('\n');
    write_half.write_all(out.as_bytes()).await?;
    write_half.flush().await?;
    Ok(())
}

/// 클라이언트(동기): 소켓에 연결해 명령을 질의한다. 연결/IO 실패는 `Err`
/// (호출자 `ai __gate`가 로컬 폴백을 결정한다).
pub fn query(path: &Path, command: &str) -> Result<GateReply> {
    query_with_context(
        path,
        &GateRequest {
            command: command.to_string(),
            context_origin: None,
        },
    )
}

/// 클라이언트(동기): origin context 포함 질의. `ai __gate`는 셸에서 상속한 cwd/env를
/// 이 요청에 싣고, daemon은 그 origin을 기준으로 context_hash를 계산한다.
pub fn query_with_context(path: &Path, request: &GateRequest) -> Result<GateReply> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let req = serde_json::to_string(request)?;
    stream.write_all(req.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(serde_json::from_str(line.trim())?)
}

#[cfg(feature = "remote")]
fn bind_device_listener(path: &Path) -> Result<std::os::unix::net::UnixListener> {
    use std::os::unix::net::UnixListener;

    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    UnixListener::bind(path)
        .with_context(|| format!("디바이스 소켓 바인드 실패: {}", path.display()))
}

/// RA-1 device listener 최소 결선: daemon module이 Unix listener를 바인드하고,
/// 연결 1건에 대해 Noise 승인 요청/응답을 왕복한다. 페어링, device record 영속화,
/// gate-flow 결선은 후속 RA-2/RA-3에서 붙인다.
#[cfg(feature = "remote")]
pub fn serve_device_once(
    path: &Path,
    daemon_private: &[u8],
    request: &crate::session::ApprovalRequestMsg,
) -> Result<crate::session::ApprovalResponseMsg> {
    let mut pending = Some(request.clone());
    let mut response = None;
    serve_device_loop(
        path,
        daemon_private,
        || pending.take(),
        |resp| {
            response = Some(resp);
            false
        },
    )?;
    response
        .expect("one-shot device listener must produce one response")
        .context("디바이스 승인 왕복 실패")
}

/// 백그라운드 device listener 핸들. RA-3에서 gate-flow가 `request_tx`로 승인 요청을
/// 보낸다. 응답은 요청마다 별도 channel로 되돌려 stale response가 다음 gate를
/// 오염시키지 않게 한다.
#[cfg(feature = "remote")]
pub struct DeviceListenerHandle {
    pub request_tx: std::sync::mpsc::Sender<DeviceListenerRequest>,
    pub thread: std::thread::JoinHandle<()>,
}

#[cfg(feature = "remote")]
pub struct DeviceListenerRequest {
    pub request: crate::session::ApprovalRequestMsg,
    pub response_tx: std::sync::mpsc::Sender<Result<crate::session::ApprovalResponseMsg>>,
    pub accept_timeout: std::time::Duration,
}

/// daemon-owned `device.sock` listener를 백그라운드 스레드로 시작한다.
/// 반환 시점에는 소켓이 이미 bind되어 있다. 요청 채널이 닫히면 listener도 종료된다.
#[cfg(feature = "remote")]
pub fn spawn_device_listener(
    path: PathBuf,
    daemon_private: Vec<u8>,
) -> Result<DeviceListenerHandle> {
    let listener = bind_device_listener(&path)?;
    let (request_tx, request_rx) = std::sync::mpsc::channel::<DeviceListenerRequest>();
    let thread = std::thread::spawn(move || {
        while let Ok(item) = request_rx.recv() {
            let response = run_daemon_listener_once_with_timeout(
                &listener,
                &daemon_private,
                &item.request,
                item.accept_timeout,
            );
            let _ = item.response_tx.send(response);
        }
    });
    Ok(DeviceListenerHandle { request_tx, thread })
}

#[cfg(feature = "remote")]
fn run_daemon_listener_once_with_timeout(
    listener: &std::os::unix::net::UnixListener,
    daemon_private: &[u8],
    request: &crate::session::ApprovalRequestMsg,
    timeout: std::time::Duration,
) -> Result<crate::session::ApprovalResponseMsg> {
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    listener.set_nonblocking(true)?;
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                return crate::session::run_daemon_request(&mut stream, daemon_private, request)
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    anyhow::bail!("디바이스 연결 시간 초과");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(err.into()),
        }
    }
}

/// RA-1 반복 device listener skeleton. `next_request`가 `Some`을 반환할 때마다
/// 다음 device 연결을 수락하고 해당 승인 요청을 `session::run_daemon_request`로 왕복한다.
/// `handle_response`가 `false`를 반환하면 정상 종료한다.
///
/// 다음 단계에서 `next_request`는 gate-flow pending approval queue로, `handle_response`는
/// nonce consume + approval validation + gate reply wakeup으로 대체된다.
#[cfg(feature = "remote")]
pub fn serve_device_loop<N, H>(
    path: &Path,
    daemon_private: &[u8],
    mut next_request: N,
    mut handle_response: H,
) -> Result<()>
where
    N: FnMut() -> Option<crate::session::ApprovalRequestMsg>,
    H: FnMut(Result<crate::session::ApprovalResponseMsg>) -> bool,
{
    let listener = bind_device_listener(path)?;

    while let Some(request) = next_request() {
        let response =
            crate::session::run_daemon_listener_once(&listener, daemon_private, &request);
        if !handle_response(response) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_with_enforces_boundary() {
        // §30-13: armed Critical=block, armed Low=allow, 비-armed=allow.
        assert!(!decide_with("rm -rf /", true, true).is_allow());
        assert!(decide_with("ls -al", true, false).is_allow());
        assert!(decide_with("rm -rf /", false, false).is_allow());
    }

    #[test]
    fn reply_helpers() {
        assert!(GateReply::allow().is_allow());
        assert!(!GateReply::block("x".into()).is_allow());
    }

    #[test]
    fn gate_request_accepts_legacy_json_without_context() {
        let req: GateRequest = serde_json::from_str(r#"{"command":"ls -al"}"#).unwrap();
        assert_eq!(req.command, "ls -al");
        assert!(req.context_origin.is_none());
    }

    #[cfg(feature = "remote")]
    fn registry_with_device() -> crate::device_registry::DeviceRegistry {
        use ed25519_dalek::SigningKey;

        let approval_pubkey = SigningKey::from_bytes(&[3u8; 32])
            .verifying_key()
            .to_bytes();
        let mut registry = crate::device_registry::DeviceRegistry::default();
        registry
            .register_device("phone-1", vec![9u8; 32], approval_pubkey, 1234)
            .unwrap();
        registry
    }

    #[cfg(feature = "remote")]
    fn plan_gate<'a>(
        registry: &crate::device_registry::DeviceRegistry,
        command: &'a str,
        armed: bool,
        allow_high: bool,
        device_id: Option<&'a str>,
        now: u64,
    ) -> Result<RemoteGateStep> {
        plan_remote_gate(
            registry,
            RemoteGatePlanInput {
                command,
                armed,
                allow_high,
                device_id,
                now,
                ttl: 60,
                context_hash: "ctx",
            },
        )
    }

    #[cfg(feature = "remote")]
    fn companion_http_request(
        addr: std::net::SocketAddr,
        method: &str,
        path: &str,
        body: &str,
    ) -> (u16, String) {
        use std::io::{Read, Write};

        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\n\
             Host: {addr}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {body}",
            body.as_bytes().len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let body = response
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or_default()
            .to_string();
        (status, body)
    }

    #[cfg(feature = "remote")]
    fn companion_hello_json(device: &crate::device_registry::RegisteredDevice) -> String {
        crate::session::companion_transport_json(&crate::session::CompanionTransportMsg::Hello {
            protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: device.id.clone(),
            noise_pubkey_hex: crate::pairing::hex_encode(&device.noise_pubkey),
            approval_pubkey_hex: crate::pairing::hex_encode(&device.approval_pubkey),
        })
        .unwrap()
    }

    #[cfg(feature = "remote")]
    fn companion_sse_message(body: &str) -> crate::session::CompanionTransportMsg {
        let data = body
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("SSE body must include data line");
        crate::session::parse_companion_transport_json(data).unwrap()
    }

    #[cfg(feature = "remote")]
    #[test]
    fn companion_live_endpoint_serves_descriptor_events_and_ping() {
        let registry = registry_with_device();
        let device = registry.get("phone-1").unwrap().clone();
        let handle = spawn_companion_live_endpoint_with_timeout(
            registry,
            None,
            std::time::Duration::from_secs(1),
        )
        .unwrap();

        let (status, body) = companion_http_request(handle.addr, "GET", "/health", "");
        assert_eq!(status, 200);
        let descriptor: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(descriptor["status"], "ready");
        assert_eq!(
            descriptor["protocol_version"],
            crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION
        );
        assert_eq!(descriptor["message_url"], handle.message_url);
        assert_eq!(descriptor["events_url"], handle.events_url);

        let (status, body) = companion_http_request(
            handle.addr,
            "POST",
            "/message",
            &companion_hello_json(&device),
        );
        assert_eq!(status, 200);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "hello:phone-1"
        ));

        let (status, body) = companion_http_request(handle.addr, "GET", "/events", "");
        assert_eq!(status, 200);
        assert!(body.contains("event: message"), "{body}");
        assert!(body.contains(r#""type":"ping""#), "{body}");

        let ping = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::Ping { nonce: "p1".into() },
        )
        .unwrap();
        let (status, body) = companion_http_request(handle.addr, "POST", "/message", &ping);
        assert_eq!(status, 200);
        assert_eq!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Pong { nonce: "p1".into() }
        );
    }

    #[cfg(feature = "remote")]
    #[test]
    fn companion_live_endpoint_validates_registered_hello() {
        let registry = registry_with_device();
        let device = registry.get("phone-1").unwrap().clone();
        let handle = spawn_companion_live_endpoint_with_timeout(
            registry,
            Some("phone-1".into()),
            std::time::Duration::from_secs(1),
        )
        .unwrap();

        let hello = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::Hello {
                protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
                device_id: device.id,
                noise_pubkey_hex: crate::pairing::hex_encode(&device.noise_pubkey),
                approval_pubkey_hex: crate::pairing::hex_encode(&device.approval_pubkey),
            },
        )
        .unwrap();
        let (status, body) = companion_http_request(handle.addr, "POST", "/message", &hello);
        assert_eq!(status, 200);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "hello:phone-1"
        ));

        let rejected = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::Hello {
                protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
                device_id: "phone-1".into(),
                noise_pubkey_hex: "a".repeat(64),
                approval_pubkey_hex: "b".repeat(64),
            },
        )
        .unwrap();
        let (status, body) = companion_http_request(handle.addr, "POST", "/message", &rejected);
        assert_eq!(status, 403);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Error { message } if message.contains("hello rejected")
        ));
    }

    #[cfg(feature = "remote")]
    #[test]
    fn companion_live_bridge_roundtrip_allows_gate_request() {
        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let registry = registry_with_device();
        let device = registry.get("phone-1").unwrap().clone();
        let registry_for_gate = registry.clone();
        let handle = spawn_companion_live_endpoint_with_timeout(
            registry,
            Some("phone-1".into()),
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let addr = handle.addr;

        let (status, _) =
            companion_http_request(addr, "POST", "/message", &companion_hello_json(&device));
        assert_eq!(status, 200);

        let live_listener = handle.listener;
        let now = now_secs();
        let gate_thread = std::thread::spawn(move || {
            decide_with_remote_listener(
                &registry_for_gate,
                &live_listener,
                RemoteGateRun {
                    command: "chmod -R 777 .",
                    armed: true,
                    allow_high: true,
                    now,
                    ttl: 60,
                    device_id: Some("phone-1"),
                    issued_context_hash: "ctx",
                    context_origin: None,
                    current_context_hash: Some("ctx"),
                    response_timeout: std::time::Duration::from_secs(5),
                },
            )
        });

        let (status, body) = companion_http_request(addr, "GET", "/events", "");
        assert_eq!(status, 200);
        let request = match companion_sse_message(&body) {
            crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
            other => panic!("expected approval_request SSE, got {other:?}"),
        };
        assert_eq!(request.command_masked, "chmod -R 777 .");

        let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
        let response_json = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::ApprovalResponse { response },
        )
        .unwrap();
        let (status, body) = companion_http_request(addr, "POST", "/message", &response_json);
        assert_eq!(status, 200);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "approval_response"
        ));

        let reply = gate_thread.join().unwrap();
        assert!(reply.is_allow(), "{reply:?}");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn companion_live_bridge_rejects_unmatched_response_without_losing_pending_request() {
        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let registry = registry_with_device();
        let device = registry.get("phone-1").unwrap().clone();
        let registry_for_gate = registry.clone();
        let handle = spawn_companion_live_endpoint_with_timeout(
            registry,
            Some("phone-1".into()),
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let addr = handle.addr;
        let (status, _) =
            companion_http_request(addr, "POST", "/message", &companion_hello_json(&device));
        assert_eq!(status, 200);

        let live_listener = handle.listener;
        let now = now_secs();
        let gate_thread = std::thread::spawn(move || {
            decide_with_remote_listener(
                &registry_for_gate,
                &live_listener,
                RemoteGateRun {
                    command: "chmod -R 777 .",
                    armed: true,
                    allow_high: true,
                    now,
                    ttl: 60,
                    device_id: Some("phone-1"),
                    issued_context_hash: "ctx",
                    context_origin: None,
                    current_context_hash: Some("ctx"),
                    response_timeout: std::time::Duration::from_secs(5),
                },
            )
        });

        let (status, body) = companion_http_request(addr, "GET", "/events", "");
        assert_eq!(status, 200);
        let request = match companion_sse_message(&body) {
            crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
            other => panic!("expected approval_request SSE, got {other:?}"),
        };

        let mut wrong_response =
            crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
        wrong_response.approval_id = b"other-approval".to_vec();
        let wrong_json = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::ApprovalResponse {
                response: wrong_response,
            },
        )
        .unwrap();
        let (status, body) = companion_http_request(addr, "POST", "/message", &wrong_json);
        assert_eq!(status, 409);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Error { message } if message.contains("does not match")
        ));

        let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
        let response_json = crate::session::companion_transport_json(
            &crate::session::CompanionTransportMsg::ApprovalResponse { response },
        )
        .unwrap();
        let (status, _) = companion_http_request(addr, "POST", "/message", &response_json);
        assert_eq!(status, 200);

        let reply = gate_thread.join().unwrap();
        assert!(reply.is_allow(), "{reply:?}");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn companion_live_endpoint_rejects_malformed_unknown_and_incomplete_requests() {
        use std::io::{Read, Write};

        let registry = registry_with_device();
        let handle = spawn_companion_live_endpoint_with_timeout(
            registry,
            None,
            std::time::Duration::from_millis(50),
        )
        .unwrap();

        let (status, body) = companion_http_request(handle.addr, "POST", "/message", "{");
        assert_eq!(status, 400);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Error { .. }
        ));

        let (status, body) = companion_http_request(handle.addr, "GET", "/missing", "");
        assert_eq!(status, 404);
        assert!(matches!(
            crate::session::parse_companion_transport_json(&body).unwrap(),
            crate::session::CompanionTransportMsg::Error { .. }
        ));

        let mut stream = std::net::TcpStream::connect(handle.addr).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let request = format!(
            "POST /message HTTP/1.1\r\nHost: {}\r\nContent-Length: 32\r\n\r\n{{",
            handle.addr
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 408"), "{response}");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn remote_gate_plans_only_high_opt_in() {
        let registry = registry_with_device();

        assert!(matches!(
            plan_gate(&registry, "ls -al", true, true, None, 100).unwrap(),
            RemoteGateStep::Local(reply) if reply.is_allow()
        ));
        assert!(matches!(
            plan_gate(&registry, "rm -rf /", true, true, None, 100).unwrap(),
            RemoteGateStep::Local(reply) if !reply.is_allow()
        ));
        assert!(matches!(
            plan_gate(&registry, "chmod -R 777 .", true, false, None, 100).unwrap(),
            RemoteGateStep::Local(reply) if !reply.is_allow()
        ));

        let step = plan_gate(&registry, "chmod -R 777 .", true, true, None, 100).unwrap();
        match step {
            RemoteGateStep::NeedsRemote(plan) => {
                assert_eq!(plan.device_id, "phone-1");
                assert_eq!(plan.pending.expires_at, 160);
                assert_eq!(plan.pending.context_hash, "ctx");
                assert_eq!(plan.pending.device_epoch, 1);
                assert_eq!(plan.request.command_masked, "chmod -R 777 .");
            }
            other => panic!("expected remote approval plan, got {other:?}"),
        }
    }

    #[cfg(feature = "remote")]
    #[test]
    fn remote_gate_requires_single_registered_device() {
        let empty = crate::device_registry::DeviceRegistry::default();
        assert!(
            plan_gate(&empty, "chmod -R 777 .", true, true, None, 100).is_err(),
            "등록 디바이스 없으면 fail-closed 경계"
        );

        let mut multi = registry_with_device();
        multi
            .register_device("phone-2", vec![10u8; 32], [4u8; 32], 1235)
            .unwrap();
        assert!(
            plan_gate(&multi, "chmod -R 777 .", true, true, None, 100).is_err(),
            "여러 디바이스면 명시 선택 전까지 ambiguous"
        );

        let selected =
            plan_gate(&multi, "chmod -R 777 .", true, true, Some("phone-2"), 100).unwrap();
        match selected {
            RemoteGateStep::NeedsRemote(plan) => {
                assert_eq!(plan.device_id, "phone-2");
                assert_eq!(plan.pending.device_epoch, 1);
            }
            other => panic!("expected selected remote approval plan, got {other:?}"),
        }
    }

    #[cfg(feature = "remote")]
    #[test]
    fn remote_gate_response_approve_reject_replay_and_timeout() {
        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let registry = registry_with_device();

        let plan = match plan_gate(&registry, "chmod -R 777 .", true, true, None, 100).unwrap() {
            RemoteGateStep::NeedsRemote(plan) => plan,
            other => panic!("expected remote approval plan, got {other:?}"),
        };
        let approve = crate::session::device_respond(&plan.request, &DEVICE_SK, true).unwrap();
        let mut nonces = crate::approval::NonceStore::new();
        nonces.register(plan.pending.nonce, plan.pending.expires_at);
        let reply =
            finish_remote_gate_response(&registry, &plan, &mut nonces, 120, "ctx", &approve);
        assert!(reply.is_allow(), "{reply:?}");
        let replay =
            finish_remote_gate_response(&registry, &plan, &mut nonces, 120, "ctx", &approve);
        assert!(!replay.is_allow(), "동일 nonce replay는 차단");

        let reject_plan =
            match plan_gate(&registry, "chmod -R 777 .", true, true, None, 200).unwrap() {
                RemoteGateStep::NeedsRemote(plan) => plan,
                other => panic!("expected remote approval plan, got {other:?}"),
            };
        let reject =
            crate::session::device_respond(&reject_plan.request, &DEVICE_SK, false).unwrap();
        let mut nonces = crate::approval::NonceStore::new();
        nonces.register(reject_plan.pending.nonce, reject_plan.pending.expires_at);
        let reply =
            finish_remote_gate_response(&registry, &reject_plan, &mut nonces, 220, "ctx", &reject);
        assert!(!reply.is_allow());
        assert!(reply.reason.contains("거부"), "{reply:?}");

        let drift_plan =
            match plan_gate(&registry, "chmod -R 777 .", true, true, None, 300).unwrap() {
                RemoteGateStep::NeedsRemote(plan) => plan,
                other => panic!("expected remote approval plan, got {other:?}"),
            };
        let drift = crate::session::device_respond(&drift_plan.request, &DEVICE_SK, true).unwrap();
        let mut nonces = crate::approval::NonceStore::new();
        nonces.register(drift_plan.pending.nonce, drift_plan.pending.expires_at);
        let reply = finish_remote_gate_response(
            &registry,
            &drift_plan,
            &mut nonces,
            320,
            "ctx-drift",
            &drift,
        );
        assert!(!reply.is_allow());
        assert!(reply.reason.contains("검증 실패"), "{reply:?}");

        let timeout = remote_timeout_reply();
        assert!(!timeout.is_allow());
        assert!(timeout.reason.contains("시간 초과"), "{timeout:?}");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn remote_gate_listener_roundtrip_allows_approved_high_command() {
        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let registry = registry_with_device();
        let dir = std::env::temp_dir().join(format!(
            "ra_gate_listener_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sock = dir.join("device.sock");
        let dev_kp = crate::remote::generate_static_keypair().unwrap();
        let dmn_kp = crate::remote::generate_static_keypair().unwrap();
        let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();
        assert!(sock.exists(), "device listener must bind before returning");

        let sock_for_device = sock.clone();
        let device_thread = std::thread::spawn(move || {
            crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
                .unwrap()
        });
        let command = "chmod -R 777 .";
        let origin = crate::context::RemoteContextOrigin {
            cwd: std::env::current_dir().unwrap().display().to_string(),
            env: vec![
                ("PATH".into(), "hash-test".into()),
                ("SHELL".into(), "/bin/bash".into()),
                ("USER".into(), "alice".into()),
                ("HOSTNAME".into(), "host".into()),
            ],
        };
        let issued_context_hash = crate::context::remote_context_hash_for_origin(command, &origin);
        let reply = decide_with_remote_listener(
            &registry,
            &handle,
            RemoteGateRun {
                command,
                armed: true,
                allow_high: true,
                now: 100,
                ttl: 60,
                device_id: None,
                issued_context_hash: &issued_context_hash,
                context_origin: Some(&origin),
                current_context_hash: None,
                response_timeout: std::time::Duration::from_secs(5),
            },
        );
        assert!(reply.is_allow(), "{reply:?}");
        let got_req = device_thread.join().unwrap();
        assert_eq!(got_req.command_masked, "chmod -R 777 .");

        drop(handle.request_tx);
        handle.thread.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// serve↔query IPC 왕복: 데몬 태스크 기동 → 클라이언트 질의 → 회신이 로컬 결정과 일치.
    /// (결정 내용 정확성은 decide_with 단위테스트, 여기선 소켓/프레이밍/직렬화 왕복 검증.)
    #[test]
    fn serve_query_roundtrip_matches_local_decision() {
        use std::time::Duration;

        let dir = std::env::temp_dir().join(format!("ra_daemon_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("gate.sock");
        let sock_srv = sock.clone();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.spawn(async move {
            let _ = serve(&sock_srv).await;
        });
        // 소켓 등장 대기.
        for _ in 0..200 {
            if sock.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(sock.exists(), "데몬이 소켓을 바인드해야 함");

        for cmd in ["rm -rf /", "ls -al"] {
            let via_socket = query(&sock, cmd).unwrap();
            let local = decide_request(cmd);
            assert_eq!(via_socket, local, "IPC 회신이 로컬 결정과 일치해야: {cmd}");
        }

        handle.abort();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn device_listener_once_roundtrip() {
        use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
        use ed25519_dalek::SigningKey;

        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let dir = std::env::temp_dir().join(format!(
            "ra_daemon_device_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sock = dir.join("device.sock");
        let dev_kp = crate::remote::generate_static_keypair().unwrap();
        let dmn_kp = crate::remote::generate_static_keypair().unwrap();
        let pending = PendingApproval {
            approval_id: b"appr-daemon-1".to_vec(),
            nonce: [11u8; 32],
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        };
        let req = crate::session::ApprovalRequestMsg::from_pending(&pending, "rm -rf /data");

        let daemon_private = dmn_kp.private.clone();
        let daemon_req = req.clone();
        let sock_for_device = sock.clone();
        let device_thread = std::thread::spawn(move || {
            // listener bind와 accept 준비 시간을 짧게 폴링한다.
            for _ in 0..200 {
                if sock_for_device.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
                .unwrap()
        });
        let resp = serve_device_once(&sock, &daemon_private, &daemon_req).unwrap();
        let got_req = device_thread.join().unwrap();
        assert_eq!(got_req, req);

        let mut nonces = NonceStore::new();
        nonces.register(pending.nonce, pending.expires_at);
        assert!(nonces.consume(&pending.nonce, 100), "최초 nonce 소비");
        let device = DeviceRecord {
            pubkey: SigningKey::from_bytes(&DEVICE_SK)
                .verifying_key()
                .to_bytes(),
            epoch: 1,
        };
        let outcome = approval::validate(&pending, &device, 100, "ctx", &resp.to_signed().unwrap());
        assert_eq!(outcome, ApprovalOutcome::Approved);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn device_listener_loop_handles_multiple_connections() {
        use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
        use ed25519_dalek::SigningKey;

        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let dir = std::env::temp_dir().join(format!(
            "ra_daemon_device_loop_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sock = dir.join("device.sock");
        let dev_kp = crate::remote::generate_static_keypair().unwrap();
        let dmn_kp = crate::remote::generate_static_keypair().unwrap();
        let p1 = PendingApproval {
            approval_id: b"appr-loop-1".to_vec(),
            nonce: [21u8; 32],
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        };
        let p2 = PendingApproval {
            approval_id: b"appr-loop-2".to_vec(),
            nonce: [22u8; 32],
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        };
        let req1 = crate::session::ApprovalRequestMsg::from_pending(&p1, "rm -rf build");
        let req2 =
            crate::session::ApprovalRequestMsg::from_pending(&p2, "sudo systemctl restart x");

        let daemon_private = dmn_kp.private.clone();
        let daemon_sock = sock.clone();
        let daemon_thread = std::thread::spawn(move || {
            let mut requests = vec![req1.clone(), req2.clone()].into_iter();
            let mut responses = Vec::new();
            serve_device_loop(
                &daemon_sock,
                &daemon_private,
                || requests.next(),
                |resp| {
                    responses.push(resp.unwrap());
                    responses.len() < 2
                },
            )
            .unwrap();
            responses
        });

        for _ in 0..200 {
            if sock.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let got1 =
            crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, true).unwrap();
        let got2 =
            crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, false).unwrap();
        assert_eq!(got1.approval_id, p1.approval_id);
        assert_eq!(got2.approval_id, p2.approval_id);
        let responses = daemon_thread.join().unwrap();
        assert_eq!(responses.len(), 2);

        let device = DeviceRecord {
            pubkey: SigningKey::from_bytes(&DEVICE_SK)
                .verifying_key()
                .to_bytes(),
            epoch: 1,
        };
        let mut nonces = NonceStore::new();
        nonces.register(p1.nonce, p1.expires_at);
        nonces.register(p2.nonce, p2.expires_at);
        assert!(nonces.consume(&p1.nonce, 100));
        assert!(nonces.consume(&p2.nonce, 100));
        assert_eq!(
            approval::validate(&p1, &device, 100, "ctx", &responses[0].to_signed().unwrap()),
            ApprovalOutcome::Approved
        );
        assert_eq!(
            approval::validate(&p2, &device, 100, "ctx", &responses[1].to_signed().unwrap()),
            ApprovalOutcome::Rejected
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn spawned_device_listener_handles_queued_request() {
        use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
        use ed25519_dalek::SigningKey;

        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let dir = std::env::temp_dir().join(format!(
            "ra_daemon_spawned_device_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sock = dir.join("device.sock");
        let dev_kp = crate::remote::generate_static_keypair().unwrap();
        let dmn_kp = crate::remote::generate_static_keypair().unwrap();
        let pending = PendingApproval {
            approval_id: b"appr-spawned-1".to_vec(),
            nonce: [31u8; 32],
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        };
        let req = crate::session::ApprovalRequestMsg::from_pending(&pending, "rm -rf build");

        let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();
        assert!(sock.exists(), "spawned listener must bind device.sock");
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        handle
            .request_tx
            .send(DeviceListenerRequest {
                request: req.clone(),
                response_tx,
                accept_timeout: std::time::Duration::from_secs(5),
            })
            .unwrap();
        let got_req =
            crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, true).unwrap();
        assert_eq!(got_req, req);
        let resp = response_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        drop(handle.request_tx);
        handle.thread.join().unwrap();

        let mut nonces = NonceStore::new();
        nonces.register(pending.nonce, pending.expires_at);
        assert!(nonces.consume(&pending.nonce, 100), "최초 nonce 소비");
        let device = DeviceRecord {
            pubkey: SigningKey::from_bytes(&DEVICE_SK)
                .verifying_key()
                .to_bytes(),
            epoch: 1,
        };
        let outcome = approval::validate(&pending, &device, 100, "ctx", &resp.to_signed().unwrap());
        assert_eq!(outcome, ApprovalOutcome::Approved);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn device_listener_timeout_does_not_poison_next_request() {
        const DEVICE_SK: [u8; 32] = [3u8; 32];
        let registry = registry_with_device();
        let dir = std::env::temp_dir().join(format!(
            "ra_daemon_timeout_recover_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sock = dir.join("device.sock");
        let dev_kp = crate::remote::generate_static_keypair().unwrap();
        let dmn_kp = crate::remote::generate_static_keypair().unwrap();
        let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();

        let timeout = decide_with_remote_listener(
            &registry,
            &handle,
            RemoteGateRun {
                command: "chmod -R 777 .",
                armed: true,
                allow_high: true,
                now: 100,
                ttl: 60,
                device_id: None,
                issued_context_hash: "ctx",
                context_origin: None,
                current_context_hash: Some("ctx"),
                response_timeout: std::time::Duration::from_millis(50),
            },
        );
        assert!(!timeout.is_allow(), "{timeout:?}");

        std::thread::sleep(std::time::Duration::from_millis(150));
        let sock_for_device = sock.clone();
        let device_thread = std::thread::spawn(move || {
            crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
                .unwrap()
        });
        let recovered = decide_with_remote_listener(
            &registry,
            &handle,
            RemoteGateRun {
                command: "chmod -R 777 .",
                armed: true,
                allow_high: true,
                now: 200,
                ttl: 60,
                device_id: None,
                issued_context_hash: "ctx",
                context_origin: None,
                current_context_hash: Some("ctx"),
                response_timeout: std::time::Duration::from_secs(5),
            },
        );
        assert!(recovered.is_allow(), "{recovered:?}");
        let _ = device_thread.join().unwrap();

        drop(handle.request_tx);
        handle.thread.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
