use std::path::Path;

use anyhow::{Context, Result};

#[cfg(feature = "remote")]
use super::device_listener::DeviceListenerHandle;
#[cfg(feature = "remote")]
use super::relay_client::CompanionRelayDaemonRuntime;
use super::remote_gate::DaemonRuntime;
use super::{GateReply, GateRequest};

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

/// remote-enabled daemon over an explicit relay runtime. This remains opt-in
/// behind `--transport relay`; the product default still uses live-loopback.
#[cfg(feature = "remote")]
pub async fn serve_with_remote_relay(
    path: &Path,
    registry: crate::device_registry::DeviceRegistry,
    relay: CompanionRelayDaemonRuntime,
    device_id: Option<String>,
) -> Result<()> {
    serve_with_runtime(
        path,
        DaemonRuntime::remote_relay(registry, relay, device_id),
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
