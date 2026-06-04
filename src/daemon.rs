//! 원격 승인 로컬 게이트 데몬 (M1 slice 1, unix 전용).
//!
//! 셸 hook(`ai __gate`)이 Unix 소켓으로 **블로킹 질의** → 데몬이 게이트 결정
//! (`gate::decide_gate` shared-core, §30-13) 회신. hook↔데몬은 **같은 머신의 신뢰된
//! 로컬 IPC**다(phone↔데몬의 Noise E2E[M0.5]와 다른 채널). 컨텍스트 스냅샷·폰 왕복·
//! nonce 소비·페어링은 M1 후속 슬라이스에서 데몬에 결합한다.
//!
//! 프레이밍: 개행 구분 JSON(연결당 1요청/1회신).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::gate::{self, GateDecision};

/// 게이트 질의 요청.
#[derive(Serialize, Deserialize, Debug)]
pub struct GateRequest {
    pub command: String,
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

/// 데몬을 실행한다: 소켓 바인드 → accept 루프(연결마다 1요청 처리). 무한 루프.
pub async fn serve(path: &Path) -> Result<()> {
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
        tokio::spawn(async move {
            let _ = handle_conn(stream).await;
        });
    }
}

/// 단일 연결 처리: 요청 한 줄을 읽어 결정 회신.
async fn handle_conn(stream: tokio::net::UnixStream) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let reply = match serde_json::from_str::<GateRequest>(line.trim()) {
        Ok(req) => decide_request(&req.command),
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
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let req = serde_json::to_string(&GateRequest {
        command: command.to_string(),
    })?;
    stream.write_all(req.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(serde_json::from_str(line.trim())?)
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
}
