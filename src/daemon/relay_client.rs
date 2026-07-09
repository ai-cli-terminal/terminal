#[cfg(feature = "remote")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "remote")]
use anyhow::{Context, Result};
#[cfg(feature = "remote")]
use serde::Deserialize;

#[cfg(feature = "remote")]
use super::companion_live::{COMPANION_LIVE_HTTP_MAX_BODY, COMPANION_LIVE_HTTP_MAX_HEADER};
#[cfg(feature = "remote")]
use super::device_listener::DeviceListenerHandle;
#[cfg(feature = "remote")]
use super::remote_gate::RemoteApprovalPlan;

#[cfg(feature = "remote")]
#[derive(Clone)]
pub(super) enum RemoteDaemonBridge {
    LiveListener(Arc<Mutex<DeviceListenerHandle>>),
    Relay(Arc<Mutex<Box<dyn RemoteRelayBridge>>>),
}

#[cfg(feature = "remote")]
pub(super) trait RemoteRelayBridge: Send {
    fn roundtrip(
        &mut self,
        plan: &RemoteApprovalPlan,
        request: crate::session::CompanionTransportMsg,
        timeout: std::time::Duration,
    ) -> Result<crate::session::CompanionTransportMsg>;
}

#[cfg(feature = "remote")]
pub struct CompanionRelayDaemonRuntime {
    setup: crate::remote_transport::CompanionRelaySelfHostedRuntimeSetup,
    endpoint: crate::remote_transport::CompanionRelayEndpoint,
    registered_session: bool,
}

#[cfg(feature = "remote")]
impl CompanionRelayDaemonRuntime {
    pub fn new(
        setup: crate::remote_transport::CompanionRelaySelfHostedRuntimeSetup,
    ) -> Result<Self> {
        setup.validate_metadata()?;
        let endpoint = crate::remote_transport::CompanionRelayEndpoint::daemon(
            setup.signed_session_ticket.ticket.session_id.clone(),
        )?;
        Ok(Self {
            setup,
            endpoint,
            registered_session: false,
        })
    }

    pub fn setup(&self) -> &crate::remote_transport::CompanionRelaySelfHostedRuntimeSetup {
        &self.setup
    }

    pub fn register_session(&mut self, timeout: std::time::Duration) -> Result<()> {
        relay_register_signed_ticket(&self.setup, timeout)?;
        self.registered_session = true;
        Ok(())
    }

    fn ensure_registered_session(&mut self, timeout: std::time::Duration) -> Result<()> {
        if self.registered_session {
            return Ok(());
        }
        self.register_session(timeout)
    }
}

#[cfg(feature = "remote")]
impl RemoteRelayBridge for CompanionRelayDaemonRuntime {
    fn roundtrip(
        &mut self,
        _plan: &RemoteApprovalPlan,
        request: crate::session::CompanionTransportMsg,
        timeout: std::time::Duration,
    ) -> Result<crate::session::CompanionTransportMsg> {
        self.ensure_registered_session(timeout)?;
        let mut socket = RelayWebSocketClient::connect(
            &self.setup.relay_endpoint_url,
            &self.setup.daemon_connect,
            timeout,
        )?;
        let frame = self.endpoint.next_frame(relay_now_ms(), &request)?;
        socket.send_text(&serde_json::to_string(&frame)?)?;

        let started = std::time::Instant::now();
        loop {
            let remaining = remaining_timeout(started, timeout)?;
            let text = socket.read_text(remaining)?;
            let envelope: RelayWebSocketEnvelope = serde_json::from_str(&text)
                .with_context(|| format!("relay websocket envelope JSON 파싱 실패: {text}"))?;
            match envelope.kind.as_str() {
                "connected" | "queued" => {}
                "frame" => {
                    let frame_json = envelope
                        .frame_json
                        .ok_or_else(|| anyhow::anyhow!("relay websocket frame_json 누락"))?;
                    let frame: crate::remote_transport::CompanionRelayFrame =
                        serde_json::from_str(&frame_json)?;
                    if let Some(message) = self.endpoint.accept_frame(frame, relay_now_ms())? {
                        return Ok(message);
                    }
                }
                "error" => {
                    return Err(anyhow::anyhow!(
                        "relay websocket 오류: {}",
                        envelope.message.unwrap_or_else(|| "unknown error".into())
                    ));
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "relay websocket envelope 타입 오류: {other}"
                    ))
                }
            }
        }
    }
}

#[cfg(feature = "remote")]
#[derive(Deserialize)]
struct RelayWebSocketEnvelope {
    kind: String,
    #[serde(default)]
    frame_json: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[cfg(feature = "remote")]
pub(super) fn relay_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(feature = "remote")]
fn remaining_timeout(
    started: std::time::Instant,
    timeout: std::time::Duration,
) -> Result<std::time::Duration> {
    match timeout.checked_sub(started.elapsed()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!("relay websocket 응답 시간 초과")),
    }
}

#[cfg(feature = "remote")]
fn relay_register_signed_ticket(
    setup: &crate::remote_transport::CompanionRelaySelfHostedRuntimeSetup,
    timeout: std::time::Duration,
) -> Result<()> {
    let ws = ParsedWsUrl::parse(&setup.relay_endpoint_url)?;
    let body = serde_json::to_string(&setup.signed_session_ticket)?;
    let status = http_post_json(&ws, "/sessions", &body, timeout)
        .context("relay session ticket 등록 실패")?;
    if !(200..300).contains(&status) {
        anyhow::bail!("relay session ticket 등록 HTTP 상태 오류: {status}");
    }
    Ok(())
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone)]
pub(super) struct ParsedWsUrl {
    pub(super) scheme: ParsedWsScheme,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) host_header: String,
    pub(super) path: String,
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParsedWsScheme {
    Ws,
    Wss,
}

#[cfg(feature = "remote")]
impl ParsedWsScheme {
    fn default_port(self) -> u16 {
        match self {
            Self::Ws => 80,
            Self::Wss => 443,
        }
    }
}

#[cfg(feature = "remote")]
impl ParsedWsUrl {
    pub(super) fn parse(url: &str) -> Result<Self> {
        let (scheme, rest) = if let Some(rest) = url.strip_prefix("wss://") {
            (ParsedWsScheme::Wss, rest)
        } else if let Some(rest) = url.strip_prefix("ws://") {
            (ParsedWsScheme::Ws, rest)
        } else {
            anyhow::bail!("relay endpoint URL은 ws:// 또는 wss:// 여야 합니다");
        };
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/".into()),
        };
        if authority.is_empty() {
            anyhow::bail!("relay endpoint host 누락");
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() => {
                let port = port
                    .parse::<u16>()
                    .context("relay endpoint port 형식 오류")?;
                (host.to_string(), port)
            }
            _ => (authority.to_string(), scheme.default_port()),
        };
        if scheme == ParsedWsScheme::Ws && !matches!(host.as_str(), "localhost" | "127.0.0.1") {
            anyhow::bail!("ws:// relay runtime은 localhost endpoint만 허용합니다");
        }
        let host_header = if authority.contains(':') {
            authority.to_string()
        } else {
            format!("{host}:{port}")
        };
        Ok(Self {
            scheme,
            host,
            port,
            host_header,
            path,
        })
    }

    fn relay_path(
        &self,
        connect: &crate::remote_transport::CompanionRelaySessionConnect,
    ) -> String {
        let separator = if self.path.contains('?') { '&' } else { '?' };
        format!(
            "{}{}session_id={}&role={}",
            self.path,
            separator,
            connect.session_id,
            connect.peer.id()
        )
    }
}

#[cfg(feature = "remote")]
enum RelayStream {
    Plain(std::net::TcpStream),
    #[cfg(feature = "tls")]
    Tls(
        Box<
            tokio_rustls::rustls::StreamOwned<
                tokio_rustls::rustls::ClientConnection,
                std::net::TcpStream,
            >,
        >,
    ),
}

#[cfg(feature = "remote")]
impl RelayStream {
    fn connect(ws: &ParsedWsUrl, timeout: std::time::Duration) -> Result<Self> {
        match ws.scheme {
            ParsedWsScheme::Ws => Ok(Self::Plain(relay_tcp_stream(&ws.host, ws.port, timeout)?)),
            ParsedWsScheme::Wss => relay_tls_stream(&ws.host, ws.port, timeout),
        }
    }

    fn set_read_timeout(&mut self, timeout: std::time::Duration) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.set_read_timeout(Some(timeout)),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => stream.sock.set_read_timeout(Some(timeout)),
        }
    }
}

#[cfg(feature = "remote")]
impl std::io::Read for RelayStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => std::io::Read::read(stream, buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => std::io::Read::read(stream, buf),
        }
    }
}

#[cfg(feature = "remote")]
impl std::io::Write for RelayStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => std::io::Write::write(stream, buf),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => std::io::Write::write(stream, buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => std::io::Write::flush(stream),
            #[cfg(feature = "tls")]
            Self::Tls(stream) => std::io::Write::flush(stream),
        }
    }
}

#[cfg(all(feature = "remote", feature = "tls"))]
fn relay_tls_stream(host: &str, port: u16, timeout: std::time::Duration) -> Result<RelayStream> {
    use std::sync::Arc;

    use tokio_rustls::rustls::pki_types::ServerName;
    use tokio_rustls::rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|error| anyhow::anyhow!("relay TLS config 오류: {error}"))?
    .with_root_certificates(root_store)
    .with_no_client_auth();
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|error| anyhow::anyhow!("relay TLS host 형식 오류: {host}: {error}"))?;
    let connection = ClientConnection::new(Arc::new(config), server_name)
        .map_err(|error| anyhow::anyhow!("relay TLS client 생성 실패: {error}"))?;
    let stream = relay_tcp_stream(host, port, timeout)?;
    Ok(RelayStream::Tls(Box::new(StreamOwned::new(
        connection, stream,
    ))))
}

#[cfg(all(feature = "remote", not(feature = "tls")))]
fn relay_tls_stream(_host: &str, _port: u16, _timeout: std::time::Duration) -> Result<RelayStream> {
    anyhow::bail!("wss relay runtime은 `tls` feature 빌드가 필요합니다")
}

#[cfg(feature = "remote")]
fn relay_tcp_stream(
    host: &str,
    port: u16,
    timeout: std::time::Duration,
) -> Result<std::net::TcpStream> {
    let stream = std::net::TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    Ok(stream)
}

#[cfg(feature = "remote")]
fn http_post_json(
    ws: &ParsedWsUrl,
    path: &str,
    body: &str,
    timeout: std::time::Duration,
) -> Result<u16> {
    use std::io::{Read, Write};

    let mut stream = RelayStream::connect(ws, timeout)?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        ws.host_header,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let header = String::from_utf8_lossy(&response);
    let status_line = header
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("relay HTTP 응답 누락"))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("relay HTTP status 누락"))?
        .parse::<u16>()
        .context("relay HTTP status 형식 오류")?;
    Ok(status)
}

#[cfg(feature = "remote")]
struct RelayWebSocketClient {
    stream: RelayStream,
}

#[cfg(feature = "remote")]
impl RelayWebSocketClient {
    fn connect(
        endpoint_url: &str,
        connect: &crate::remote_transport::CompanionRelaySessionConnect,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        use std::io::{Read, Write};

        let ws = ParsedWsUrl::parse(endpoint_url)?;
        let mut stream = RelayStream::connect(&ws, timeout)?;
        let mut nonce = [0_u8; 16];
        getrandom::getrandom(&mut nonce)?;
        let key = base64_encode(&nonce);
        let path = ws.relay_path(connect);
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            ws.host_header
        );
        stream.write_all(request.as_bytes())?;
        stream.flush()?;

        let mut header = Vec::new();
        let mut byte = [0_u8; 1];
        while !header.ends_with(b"\r\n\r\n") {
            if header.len() > COMPANION_LIVE_HTTP_MAX_HEADER {
                anyhow::bail!("relay websocket handshake header too large");
            }
            stream.read_exact(&mut byte)?;
            header.push(byte[0]);
        }
        let header_text = String::from_utf8_lossy(&header);
        let status_line = header_text
            .lines()
            .next()
            .ok_or_else(|| anyhow::anyhow!("relay websocket handshake 응답 누락"))?;
        if !status_line.contains(" 101 ") {
            anyhow::bail!("relay websocket handshake 실패: {status_line}");
        }
        let expected_accept = websocket_accept_key(&key);
        let accept_ok = header_text.lines().any(|line| {
            line.split_once(':')
                .map(|(name, value)| {
                    name.eq_ignore_ascii_case("Sec-WebSocket-Accept")
                        && value.trim() == expected_accept
                })
                .unwrap_or(false)
        });
        if !accept_ok {
            anyhow::bail!("relay websocket accept key 검증 실패");
        }

        let mut client = Self { stream };
        client.send_text(&serde_json::to_string(connect)?)?;
        let started = std::time::Instant::now();
        loop {
            let text = client.read_text(remaining_timeout(started, timeout)?)?;
            let envelope: RelayWebSocketEnvelope = serde_json::from_str(&text)?;
            match envelope.kind.as_str() {
                "connected" => return Ok(client),
                "error" => {
                    anyhow::bail!(
                        "relay websocket connect 오류: {}",
                        envelope.message.unwrap_or_else(|| "unknown error".into())
                    );
                }
                _ => {}
            }
        }
    }

    fn send_text(&mut self, text: &str) -> Result<()> {
        self.send_frame(0x1, text.as_bytes())
    }

    fn send_pong(&mut self, payload: &[u8]) -> Result<()> {
        self.send_frame(0xA, payload)
    }

    fn send_frame(&mut self, opcode: u8, payload: &[u8]) -> Result<()> {
        use std::io::Write;

        let mut frame = Vec::new();
        frame.push(0x80 | (opcode & 0x0F));
        let len = payload.len();
        if len <= 125 {
            frame.push(0x80 | u8::try_from(len)?);
        } else if len <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&u16::try_from(len)?.to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&u64::try_from(len)?.to_be_bytes());
        }
        let mut mask = [0_u8; 4];
        getrandom::getrandom(&mut mask)?;
        frame.extend_from_slice(&mask);
        for (idx, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[idx % mask.len()]);
        }
        self.stream.write_all(&frame)?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_text(&mut self, timeout: std::time::Duration) -> Result<String> {
        loop {
            match self.read_frame(timeout)? {
                WebSocketFrame::Text(text) => return Ok(text),
                WebSocketFrame::Ping(payload) => self.send_pong(&payload)?,
                WebSocketFrame::Pong | WebSocketFrame::Close => {}
            }
        }
    }

    fn read_frame(&mut self, timeout: std::time::Duration) -> Result<WebSocketFrame> {
        use std::io::Read;

        self.stream.set_read_timeout(timeout)?;
        let mut header = [0_u8; 2];
        self.stream.read_exact(&mut header)?;
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0F;
        if !fin {
            anyhow::bail!("relay websocket fragmented frames are not supported");
        }
        let masked = header[1] & 0x80 != 0;
        let mut len = u64::from(header[1] & 0x7F);
        if len == 126 {
            let mut ext = [0_u8; 2];
            self.stream.read_exact(&mut ext)?;
            len = u64::from(u16::from_be_bytes(ext));
        } else if len == 127 {
            let mut ext = [0_u8; 8];
            self.stream.read_exact(&mut ext)?;
            len = u64::from_be_bytes(ext);
        }
        if len > u64::try_from(COMPANION_LIVE_HTTP_MAX_BODY)? {
            anyhow::bail!("relay websocket message too large");
        }
        let mut mask = [0_u8; 4];
        if masked {
            self.stream.read_exact(&mut mask)?;
        }
        let mut payload = vec![0_u8; usize::try_from(len)?];
        self.stream.read_exact(&mut payload)?;
        if masked {
            for (idx, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[idx % mask.len()];
            }
        }
        match opcode {
            0x1 => Ok(WebSocketFrame::Text(String::from_utf8(payload)?)),
            0x8 => Ok(WebSocketFrame::Close),
            0x9 => Ok(WebSocketFrame::Ping(payload)),
            0xA => Ok(WebSocketFrame::Pong),
            _ => anyhow::bail!("unsupported relay websocket opcode: {opcode}"),
        }
    }
}

#[cfg(feature = "remote")]
enum WebSocketFrame {
    Text(String),
    Ping(Vec<u8>),
    Pong,
    Close,
}

#[cfg(feature = "remote")]
pub(super) fn websocket_accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64_encode(&hasher.finalize())
}

#[cfg(feature = "remote")]
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        let b0 = bytes[idx];
        let b1 = bytes.get(idx + 1).copied().unwrap_or(0);
        let b2 = bytes.get(idx + 2).copied().unwrap_or(0);
        out.push(TABLE[usize::from(b0 >> 2)] as char);
        out.push(TABLE[usize::from(((b0 & 0b0000_0011) << 4) | (b1 >> 4))] as char);
        if idx + 1 < bytes.len() {
            out.push(TABLE[usize::from(((b1 & 0b0000_1111) << 2) | (b2 >> 6))] as char);
        } else {
            out.push('=');
        }
        if idx + 2 < bytes.len() {
            out.push(TABLE[usize::from(b2 & 0b0011_1111)] as char);
        } else {
            out.push('=');
        }
        idx += 3;
    }
    out
}
