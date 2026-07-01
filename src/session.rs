//! 원격 승인 Noise 세션 메시지 + 왕복 (M1 slice 3, `remote` feature).
//!
//! M0.5 크립토(`remote.rs` snow transport·Ed25519)와 M1 s2 검증(`approval.rs`)을
//! **실제 Noise 암호문 위에서** 잇는다: 데몬이 ApprovalRequest를 암호화 송신 → 디바이스가
//! 복호·서명 회신 → 데몬이 복호·검증(NonceStore replay + validate). 전송 substrate
//! (소켓/Tailscale/relay)는 `write_message`/`read_message` 바이트를 실어 나르기만 하면
//! 되며, 본 모듈은 그 substrate와 무관한 직렬화·변환·서명 로직을 제공한다.

use std::io::{Read, Write};

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::approval::{PendingApproval, SignedResponse};
use crate::remote;

/// 프레임 길이 상한(DoS 가드). Noise 메시지 + json 승인은 이보다 훨씬 작다.
const MAX_FRAME: usize = 1 << 20; // 1 MiB

pub const COMPANION_TRANSPORT_PROTOCOL_VERSION: u32 = 1;

/// 데몬→디바이스 승인 요청(와이어). `[u8;N]`은 serde 한계로 `Vec<u8>`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequestMsg {
    pub approval_id: Vec<u8>,
    pub nonce: Vec<u8>,
    pub command_masked: String,
    pub context_hash: String,
    pub expires_at: u64,
    pub device_epoch: u64,
}

/// 디바이스→데몬 서명 응답(와이어).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalResponseMsg {
    pub approval_id: Vec<u8>,
    pub nonce: Vec<u8>,
    pub approve: bool,
    pub sig: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompanionTransportMsg {
    Hello {
        protocol_version: u32,
        device_id: String,
        noise_pubkey_hex: String,
        approval_pubkey_hex: String,
    },
    ApprovalRequest {
        request: ApprovalRequestMsg,
    },
    ApprovalResponse {
        response: ApprovalResponseMsg,
    },
    Ping {
        nonce: String,
    },
    Pong {
        nonce: String,
    },
    Error {
        message: String,
    },
}

/// serde_json 바이트로 직렬화(Noise 메시지 페이로드).
pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(msg)?)
}

pub fn approval_request_json(request: &ApprovalRequestMsg) -> Result<String> {
    Ok(serde_json::to_string(request)?)
}

pub fn companion_transport_json(message: &CompanionTransportMsg) -> Result<String> {
    validate_companion_transport_msg(message)?;
    Ok(serde_json::to_string(message)?)
}

pub fn parse_companion_transport_json(text: &str) -> Result<CompanionTransportMsg> {
    let message: CompanionTransportMsg = serde_json::from_str(text)?;
    validate_companion_transport_msg(&message)?;
    Ok(message)
}

pub fn validate_companion_transport_msg(message: &CompanionTransportMsg) -> Result<()> {
    match message {
        CompanionTransportMsg::Hello {
            protocol_version,
            device_id,
            noise_pubkey_hex,
            approval_pubkey_hex,
        } => {
            if *protocol_version != COMPANION_TRANSPORT_PROTOCOL_VERSION {
                bail!("지원하지 않는 companion transport protocol_version");
            }
            if !valid_device_id(device_id) {
                bail!("device_id 형식 오류");
            }
            if !is_hex_32(noise_pubkey_hex) {
                bail!("noise_pubkey_hex 형식 오류");
            }
            if !is_hex_32(approval_pubkey_hex) {
                bail!("approval_pubkey_hex 형식 오류");
            }
        }
        CompanionTransportMsg::ApprovalRequest { request } => {
            request.to_pending()?;
            if request.command_masked.is_empty() {
                bail!("command_masked 형식 오류");
            }
        }
        CompanionTransportMsg::ApprovalResponse { response } => {
            response.to_signed()?;
        }
        CompanionTransportMsg::Ping { nonce } | CompanionTransportMsg::Pong { nonce } => {
            if nonce.is_empty() || nonce.len() > 128 {
                bail!("heartbeat nonce 형식 오류");
            }
        }
        CompanionTransportMsg::Error { message } => {
            if message.trim().is_empty() {
                bail!("error message 형식 오류");
            }
        }
    }
    Ok(())
}

pub fn approval_url(request: &ApprovalRequestMsg) -> Result<String> {
    Ok(format!(
        "aiterminal://approve?approval={}",
        percent_encode(&approval_request_json(request)?)
    ))
}

pub fn approval_pwa_url(request: &ApprovalRequestMsg, base_url: &str) -> Result<String> {
    let base = base_url.trim();
    if base.is_empty() {
        bail!("PWA URL은 비어 있을 수 없음");
    }
    let separator = if base.contains('?') {
        if base.ends_with('?') || base.ends_with('&') {
            ""
        } else {
            "&"
        }
    } else {
        "?"
    };
    Ok(format!(
        "{base}{separator}approval={}",
        percent_encode(&approval_request_json(request)?)
    ))
}

pub fn approval_qr_text(request: &ApprovalRequestMsg) -> Result<String> {
    crate::qr::render_terminal_qr(&approval_url(request)?)
}

pub fn approval_pwa_qr_text(request: &ApprovalRequestMsg, base_url: &str) -> Result<String> {
    crate::qr::render_terminal_qr(&approval_pwa_url(request, base_url)?)
}

/// serde_json 바이트에서 역직렬화.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

impl ApprovalRequestMsg {
    /// 대기 항목 + 마스킹된 명령으로 와이어 요청을 만든다.
    pub fn from_pending(p: &PendingApproval, command_masked: &str) -> Self {
        Self {
            approval_id: p.approval_id.clone(),
            nonce: p.nonce.to_vec(),
            command_masked: command_masked.to_string(),
            context_hash: p.context_hash.clone(),
            expires_at: p.expires_at,
            device_epoch: p.device_epoch,
        }
    }

    /// URL/PWA 경계를 지난 승인 요청을 내부 검증용 pending 항목으로 되돌린다.
    pub fn to_pending(&self) -> Result<PendingApproval> {
        let nonce: [u8; 32] = self
            .nonce
            .as_slice()
            .try_into()
            .context("nonce 길이는 32바이트여야 함")?;
        Ok(PendingApproval {
            approval_id: self.approval_id.clone(),
            nonce,
            expires_at: self.expires_at,
            context_hash: self.context_hash.clone(),
            device_epoch: self.device_epoch,
        })
    }
}

impl ApprovalResponseMsg {
    /// 와이어 응답을 내부 [`SignedResponse`]로 변환(길이 검증).
    pub fn to_signed(&self) -> Result<SignedResponse> {
        let nonce: [u8; 32] = self
            .nonce
            .as_slice()
            .try_into()
            .context("nonce 길이는 32바이트여야 함")?;
        let sig: [u8; 64] = self
            .sig
            .as_slice()
            .try_into()
            .context("서명 길이는 64바이트여야 함")?;
        Ok(SignedResponse {
            approval_id: self.approval_id.clone(),
            nonce,
            approve: self.approve,
            sig,
        })
    }
}

/// 모의 디바이스: 요청에 Ed25519 서명해 응답을 만든다(PWA가 할 일의 in-repo 대역).
pub fn device_respond(
    req: &ApprovalRequestMsg,
    device_sk: &[u8; 32],
    approve: bool,
) -> Result<ApprovalResponseMsg> {
    let nonce: [u8; 32] = req
        .nonce
        .as_slice()
        .try_into()
        .context("nonce 길이는 32바이트여야 함")?;
    let sig = remote::sign_approval(device_sk, &req.approval_id, &nonce, approve);
    Ok(ApprovalResponseMsg {
        approval_id: req.approval_id.clone(),
        nonce: req.nonce.clone(),
        approve,
        sig: sig.to_vec(),
    })
}

/// 스트림에 한 프레임을 쓴다(`[u32 BE len][payload]`, M0.5 framing).
pub fn send_frame<W: Write>(w: &mut W, payload: &[u8]) -> Result<()> {
    let len = u32::try_from(payload.len()).context("프레임이 u32 범위 초과")?;
    w.write_all(&len.to_be_bytes())?;
    w.write_all(payload)?;
    w.flush()?;
    Ok(())
}

/// 스트림에서 한 프레임을 읽는다. 상한 초과 길이는 거부(DoS 가드).
pub fn recv_frame<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let mut lenb = [0u8; 4];
    r.read_exact(&mut lenb)?;
    let len = u32::from_be_bytes(lenb) as usize;
    if len > MAX_FRAME {
        bail!("프레임이 너무 큼: {len} > {MAX_FRAME}");
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

/// 모의 디바이스(initiator) 역할: XX handshake → transport → 요청 수신·복호 → 서명 →
/// 응답 송신. 수신한 요청을 반환한다(검사용). 전송 substrate는 제네릭 스트림.
pub fn run_device<S: Read + Write>(
    stream: &mut S,
    device_private: &[u8],
    device_sk: &[u8; 32],
    approve: bool,
) -> Result<ApprovalRequestMsg> {
    let pattern = remote::NOISE_PATTERN
        .parse()
        .map_err(|e| anyhow::anyhow!("noise pattern: {e:?}"))?;
    let mut hs = snow::Builder::new(pattern)
        .local_private_key(device_private)
        .build_initiator()
        .map_err(|e| anyhow::anyhow!("build_initiator: {e:?}"))?;
    let mut buf = vec![0u8; 65535];

    // XX: -> e ; <- e,ee,s,es ; -> s,se
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    send_frame(stream, &buf[..n])?;
    let m2 = recv_frame(stream)?;
    hs.read_message(&m2, &mut buf).map_err(noise_err)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    send_frame(stream, &buf[..n])?;

    let mut t = hs.into_transport_mode().map_err(noise_err)?;
    let ct = recv_frame(stream)?;
    let m = t.read_message(&ct, &mut buf).map_err(noise_err)?;
    let req: ApprovalRequestMsg = decode(&buf[..m])?;

    let resp = device_respond(&req, device_sk, approve)?;
    let n = t
        .write_message(&encode(&resp)?, &mut buf)
        .map_err(noise_err)?;
    send_frame(stream, &buf[..n])?;
    Ok(req)
}

/// 데몬(responder) 역할: XX handshake → transport → 요청 송신 → 응답 수신·복호 반환.
/// 검증(consume + validate)은 호출자(데몬)가 수행한다.
pub fn run_daemon_request<S: Read + Write>(
    stream: &mut S,
    daemon_private: &[u8],
    request: &ApprovalRequestMsg,
) -> Result<ApprovalResponseMsg> {
    let pattern = remote::NOISE_PATTERN
        .parse()
        .map_err(|e| anyhow::anyhow!("noise pattern: {e:?}"))?;
    let mut hs = snow::Builder::new(pattern)
        .local_private_key(daemon_private)
        .build_responder()
        .map_err(|e| anyhow::anyhow!("build_responder: {e:?}"))?;
    let mut buf = vec![0u8; 65535];

    let m1 = recv_frame(stream)?;
    hs.read_message(&m1, &mut buf).map_err(noise_err)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    send_frame(stream, &buf[..n])?;
    let m3 = recv_frame(stream)?;
    hs.read_message(&m3, &mut buf).map_err(noise_err)?;

    let mut t = hs.into_transport_mode().map_err(noise_err)?;
    let n = t
        .write_message(&encode(request)?, &mut buf)
        .map_err(noise_err)?;
    send_frame(stream, &buf[..n])?;
    let ct = recv_frame(stream)?;
    let m = t.read_message(&ct, &mut buf).map_err(noise_err)?;
    decode(&buf[..m])
}

/// 데몬 쪽 실제 Unix 소켓 listener에서 디바이스 연결 1건을 받아 승인 요청을 왕복한다.
/// RA-1의 최소 substrate: 상시 데몬 결선 전, `run_daemon_request`가 실제 listener 위에서
/// 동작함을 검증한다.
#[cfg(unix)]
pub fn run_daemon_listener_once(
    listener: &std::os::unix::net::UnixListener,
    daemon_private: &[u8],
    request: &ApprovalRequestMsg,
) -> Result<ApprovalResponseMsg> {
    let (mut stream, _) = listener.accept()?;
    run_daemon_request(&mut stream, daemon_private, request)
}

/// 디바이스 쪽 실제 Unix 소켓 클라이언트: path에 연결한 뒤 기존 device 역할을 수행한다.
#[cfg(unix)]
pub fn run_device_connect(
    path: &std::path::Path,
    device_private: &[u8],
    device_sk: &[u8; 32],
    approve: bool,
) -> Result<ApprovalRequestMsg> {
    let mut stream = std::os::unix::net::UnixStream::connect(path)?;
    run_device(&mut stream, device_private, device_sk, approve)
}

fn noise_err(e: snow::Error) -> anyhow::Error {
    anyhow::anyhow!("noise: {e:?}")
}

fn valid_device_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn is_hex_32(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn percent_encode(text: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::new();
    for &byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore};
    use ed25519_dalek::SigningKey;

    const PATTERN: &str = remote::NOISE_PATTERN;
    const DEVICE_SK: [u8; 32] = [3u8; 32];

    fn pending(nonce: [u8; 32]) -> PendingApproval {
        PendingApproval {
            approval_id: b"appr-1".to_vec(),
            nonce,
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        }
    }

    #[test]
    fn wire_roundtrip() {
        let p = pending([1u8; 32]);
        let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");
        let got: ApprovalRequestMsg = decode(&encode(&req).unwrap()).unwrap();
        assert_eq!(got, req);

        let resp = device_respond(&req, &DEVICE_SK, true).unwrap();
        let got: ApprovalResponseMsg = decode(&encode(&resp).unwrap()).unwrap();
        assert_eq!(got, resp);
    }

    #[test]
    fn approval_request_url_and_qr_are_pwa_ready() {
        let p = pending([1u8; 32]);
        let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");
        assert_eq!(req.to_pending().unwrap().approval_id, p.approval_id);
        assert_eq!(req.to_pending().unwrap().nonce, p.nonce);
        assert_eq!(req.to_pending().unwrap().context_hash, p.context_hash);
        let json = approval_request_json(&req).unwrap();
        assert!(json.contains(r#""command_masked":"rm -rf /data""#));
        assert!(json.contains(r#""context_hash":"ctx""#));

        let url = approval_url(&req).unwrap();
        assert!(url.starts_with("aiterminal://approve?approval="));
        assert!(url.contains("%7B%22approval_id%22%3A"));

        let pwa_url = approval_pwa_url(&req, "http://127.0.0.1:8787/index.html").unwrap();
        assert!(pwa_url.starts_with("http://127.0.0.1:8787/index.html?approval="));
        let pwa_url_with_query =
            approval_pwa_url(&req, "https://example.test/app?mode=approve").unwrap();
        assert!(pwa_url_with_query.starts_with("https://example.test/app?mode=approve&approval="));

        let qr = approval_qr_text(&req).unwrap();
        assert!(qr.lines().count() > 4);
        let pwa_qr = approval_pwa_qr_text(&req, "http://127.0.0.1:8787/index.html").unwrap();
        assert!(pwa_qr.lines().count() > 4);
    }

    #[test]
    fn pwa_response_json_validates_after_request_rebuild() {
        let device_pk = SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes();
        let p = pending([2u8; 32]);
        let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");
        let resp = device_respond(&req, &DEVICE_SK, true).unwrap();
        let pending = req.to_pending().unwrap();
        let signed = resp.to_signed().unwrap();
        let device = DeviceRecord {
            pubkey: device_pk,
            epoch: req.device_epoch,
        };
        let outcome =
            approval::validate(&pending, &device, p.expires_at, &req.context_hash, &signed);
        assert_eq!(outcome, ApprovalOutcome::Approved);
    }

    #[test]
    fn companion_transport_messages_are_typed_and_validated() {
        let p = pending([3u8; 32]);
        let req = ApprovalRequestMsg::from_pending(&p, "chmod -R 777 .");
        let resp = device_respond(&req, &DEVICE_SK, true).unwrap();

        let hello = CompanionTransportMsg::Hello {
            protocol_version: COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: "web-1234abcd".into(),
            noise_pubkey_hex: "a".repeat(64),
            approval_pubkey_hex: "b".repeat(64),
        };
        let json = companion_transport_json(&hello).unwrap();
        assert!(json.contains(r#""type":"hello""#), "{json}");
        assert_eq!(parse_companion_transport_json(&json).unwrap(), hello);

        let request_msg = CompanionTransportMsg::ApprovalRequest {
            request: req.clone(),
        };
        let parsed =
            parse_companion_transport_json(&companion_transport_json(&request_msg).unwrap())
                .unwrap();
        assert_eq!(parsed, request_msg);

        let response_msg = CompanionTransportMsg::ApprovalResponse { response: resp };
        let parsed =
            parse_companion_transport_json(&companion_transport_json(&response_msg).unwrap())
                .unwrap();
        assert_eq!(parsed, response_msg);

        let ping = CompanionTransportMsg::Ping {
            nonce: "ping-1".into(),
        };
        assert_eq!(
            parse_companion_transport_json(&companion_transport_json(&ping).unwrap()).unwrap(),
            ping
        );
    }

    #[test]
    fn companion_transport_rejects_malformed_messages() {
        let bad_version = CompanionTransportMsg::Hello {
            protocol_version: 2,
            device_id: "web-1234abcd".into(),
            noise_pubkey_hex: "a".repeat(64),
            approval_pubkey_hex: "b".repeat(64),
        };
        assert!(companion_transport_json(&bad_version).is_err());

        let bad_id = CompanionTransportMsg::Hello {
            protocol_version: COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: "bad id".into(),
            noise_pubkey_hex: "a".repeat(64),
            approval_pubkey_hex: "b".repeat(64),
        };
        assert!(companion_transport_json(&bad_id).is_err());

        let bad_key = CompanionTransportMsg::Hello {
            protocol_version: COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: "web-1234abcd".into(),
            noise_pubkey_hex: "not-hex".into(),
            approval_pubkey_hex: "b".repeat(64),
        };
        assert!(companion_transport_json(&bad_key).is_err());

        assert!(parse_companion_transport_json(r#"{"type":"ping","nonce":""}"#).is_err());
        assert!(parse_companion_transport_json(r#"{"type":"error","message":"   "}"#).is_err());
    }

    /// 실제 Noise 채널 위 승인 한 바퀴: handshake → 암호화 요청 → 서명 응답 → 복호 →
    /// consume(replay) + validate. approve/reject + replay 거부.
    fn run_roundtrip(approve: bool) -> (ApprovalOutcome, bool) {
        // XX handshake: device=initiator, daemon=responder.
        let dev_kp = remote::generate_static_keypair().unwrap();
        let dmn_kp = remote::generate_static_keypair().unwrap();
        let mut device = snow::Builder::new(PATTERN.parse().unwrap())
            .local_private_key(&dev_kp.private)
            .build_initiator()
            .unwrap();
        let mut daemon = snow::Builder::new(PATTERN.parse().unwrap())
            .local_private_key(&dmn_kp.private)
            .build_responder()
            .unwrap();
        let (mut b, mut o) = ([0u8; 4096], [0u8; 4096]);
        let n = device.write_message(&[], &mut b).unwrap();
        daemon.read_message(&b[..n], &mut o).unwrap();
        let n = daemon.write_message(&[], &mut b).unwrap();
        device.read_message(&b[..n], &mut o).unwrap();
        let n = device.write_message(&[], &mut b).unwrap();
        daemon.read_message(&b[..n], &mut o).unwrap();
        let mut dev_t = device.into_transport_mode().unwrap();
        let mut dmn_t = daemon.into_transport_mode().unwrap();

        // 데몬 상태.
        let device_pk = SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes();
        let nonce = [7u8; 32];
        let p = pending(nonce);
        let mut nonces = NonceStore::new();
        nonces.register(nonce, p.expires_at);

        // 데몬 → (암호화) → 디바이스.
        let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");
        let n = dmn_t.write_message(&encode(&req).unwrap(), &mut b).unwrap();
        let m = dev_t.read_message(&b[..n], &mut o).unwrap();
        let got_req: ApprovalRequestMsg = decode(&o[..m]).unwrap();
        assert_eq!(got_req.command_masked, "rm -rf /data");

        // 디바이스 서명 → (암호화) → 데몬.
        let resp = device_respond(&got_req, &DEVICE_SK, approve).unwrap();
        let n = dev_t
            .write_message(&encode(&resp).unwrap(), &mut b)
            .unwrap();
        let m = dmn_t.read_message(&b[..n], &mut o).unwrap();
        let got_resp: ApprovalResponseMsg = decode(&o[..m]).unwrap();

        // 데몬 검증: replay(consume) + validate.
        let first = nonces.consume(&nonce, 100);
        let device_rec = DeviceRecord {
            pubkey: device_pk,
            epoch: 1,
        };
        let outcome =
            approval::validate(&p, &device_rec, 100, "ctx", &got_resp.to_signed().unwrap());
        let replay_blocked = !nonces.consume(&nonce, 100);
        assert!(first, "최초 nonce 소비 성공");
        (outcome, replay_blocked)
    }

    #[test]
    fn end_to_end_approve_over_noise() {
        let (outcome, replay_blocked) = run_roundtrip(true);
        assert_eq!(outcome, ApprovalOutcome::Approved);
        assert!(replay_blocked, "동일 nonce 재사용은 거부되어야 함");
    }

    #[test]
    fn end_to_end_reject_over_noise() {
        let (outcome, _) = run_roundtrip(false);
        assert_eq!(outcome, ApprovalOutcome::Rejected);
    }

    #[test]
    fn frame_roundtrip_and_size_guard() {
        let mut buf: Vec<u8> = Vec::new();
        send_frame(&mut buf, b"hello-frame").unwrap();
        let mut cur = std::io::Cursor::new(buf);
        assert_eq!(recv_frame(&mut cur).unwrap(), b"hello-frame");
        // 과대 길이 헤더(~4GiB) 거부.
        let mut cur = std::io::Cursor::new(vec![0xffu8, 0xff, 0xff, 0xff]);
        assert!(recv_frame(&mut cur).is_err(), "상한 초과 프레임은 거부");
    }

    /// 실제 UnixStream 페어 위 handshake + 승인 왕복(전송 substrate 교체 검증).
    #[cfg(unix)]
    #[test]
    fn approval_roundtrip_over_unix_socket() {
        use std::os::unix::net::UnixStream;

        for approve in [true, false] {
            let (mut daemon_s, mut device_s) = UnixStream::pair().unwrap();
            let dev_kp = remote::generate_static_keypair().unwrap();
            let dmn_kp = remote::generate_static_keypair().unwrap();
            let device_pk = SigningKey::from_bytes(&DEVICE_SK)
                .verifying_key()
                .to_bytes();
            let nonce = [7u8; 32];
            let p = pending(nonce);

            let dev_priv = dev_kp.private.clone();
            let device_thread = std::thread::spawn(move || {
                run_device(&mut device_s, &dev_priv, &DEVICE_SK, approve).unwrap();
            });
            let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");
            let resp = run_daemon_request(&mut daemon_s, &dmn_kp.private, &req).unwrap();
            device_thread.join().unwrap();

            let mut nonces = NonceStore::new();
            nonces.register(nonce, p.expires_at);
            assert!(nonces.consume(&nonce, 100), "최초 nonce 소비");
            let device = DeviceRecord {
                pubkey: device_pk,
                epoch: 1,
            };
            let outcome = approval::validate(&p, &device, 100, "ctx", &resp.to_signed().unwrap());
            let expected = if approve {
                ApprovalOutcome::Approved
            } else {
                ApprovalOutcome::Rejected
            };
            assert_eq!(outcome, expected, "approve={approve}");
        }
    }

    /// RA-1 착수: 실제 UnixListener path에 디바이스가 연결해 handshake+승인 왕복을 끝낸다.
    #[cfg(unix)]
    #[test]
    fn approval_roundtrip_over_unix_listener() {
        use std::os::unix::net::UnixListener;

        let dir = std::env::temp_dir().join(format!(
            "ra_device_listener_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("device.sock");
        let listener = UnixListener::bind(&sock).unwrap();

        let dev_kp = remote::generate_static_keypair().unwrap();
        let dmn_kp = remote::generate_static_keypair().unwrap();
        let device_pk = SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes();
        let nonce = [9u8; 32];
        let p = pending(nonce);
        let req = ApprovalRequestMsg::from_pending(&p, "rm -rf /data");

        let daemon_private = dmn_kp.private.clone();
        let daemon_req = req.clone();
        let daemon_thread = std::thread::spawn(move || {
            run_daemon_listener_once(&listener, &daemon_private, &daemon_req).unwrap()
        });
        let got_req = run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, true).unwrap();
        assert_eq!(got_req, req);
        let resp = daemon_thread.join().unwrap();

        let mut nonces = NonceStore::new();
        nonces.register(nonce, p.expires_at);
        assert!(nonces.consume(&nonce, 100), "최초 nonce 소비");
        let device = DeviceRecord {
            pubkey: device_pk,
            epoch: 1,
        };
        let outcome = approval::validate(&p, &device, 100, "ctx", &resp.to_signed().unwrap());
        assert_eq!(outcome, ApprovalOutcome::Approved);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
