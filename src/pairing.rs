//! 원격 승인 페어링 상태 (RA-2 slice 2, `remote` feature).
//!
//! 실제 QR/PWA UX 전 단계의 로컬 상태 머신이다. daemon Noise key를 영속화하고,
//! 한 번에 하나의 pending pairing code만 허용한 뒤, 디바이스가 제공한 Noise static
//! pubkey와 Ed25519 approval pubkey를 `device_registry`에 등록한다.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::device_registry::{DeviceRegistry, RegisteredDevice};

pub const DAEMON_KEY_FILE: &str = "remote-daemon-key.json";
pub const PAIRING_FILE: &str = "remote-pairing.json";
pub const PAIRING_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonKeyRecord {
    pub public: Vec<u8>,
    pub private: Vec<u8>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingSession {
    pub code: String,
    pub daemon_pubkey: Vec<u8>,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingPayload {
    pub protocol_version: u32,
    pub pairing_code: String,
    pub daemon_pubkey_hex: String,
    pub transport_addr: String,
    pub expires_at_ms: u64,
}

pub fn daemon_key_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join(DAEMON_KEY_FILE))
}

pub fn pairing_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join(PAIRING_FILE))
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

pub fn load_or_create_daemon_key(path: &Path) -> Result<DaemonKeyRecord> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let record: DaemonKeyRecord = serde_json::from_slice(&bytes)
                .with_context(|| format!("daemon key 파일 파싱 실패: {}", path.display()))?;
            validate_daemon_key(&record)?;
            Ok(record)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let key = crate::remote::generate_static_keypair()?;
            let record = DaemonKeyRecord {
                public: key.public,
                private: key.private,
                created_at_ms: now_ms(),
            };
            save_daemon_key(path, &record)?;
            Ok(record)
        }
        Err(err) => Err(err).with_context(|| format!("daemon key 읽기 실패: {}", path.display())),
    }
}

pub fn save_daemon_key(path: &Path, record: &DaemonKeyRecord) -> Result<()> {
    validate_daemon_key(record)?;
    write_json(path, record)
}

pub fn start_pairing(
    pairing_path: &Path,
    daemon_key: &DaemonKeyRecord,
    ttl_seconds: u64,
) -> Result<PairingSession> {
    let code = generate_pairing_code()?;
    start_pairing_with_code(pairing_path, daemon_key, now_ms(), ttl_seconds, code)
}

pub fn complete_pairing(
    pairing_path: &Path,
    registry_path: &Path,
    device_id: &str,
    code: &str,
    noise_pubkey: Vec<u8>,
    approval_pubkey: [u8; 32],
) -> Result<RegisteredDevice> {
    complete_pairing_at(
        pairing_path,
        registry_path,
        device_id,
        code,
        noise_pubkey,
        approval_pubkey,
        now_ms(),
    )
}

pub fn pairing_payload(session: &PairingSession, transport_addr: &str) -> PairingPayload {
    PairingPayload {
        protocol_version: PAIRING_PROTOCOL_VERSION,
        pairing_code: session.code.clone(),
        daemon_pubkey_hex: hex_encode(&session.daemon_pubkey),
        transport_addr: transport_addr.to_string(),
        expires_at_ms: session.expires_at_ms,
    }
}

pub fn pairing_payload_json(payload: &PairingPayload) -> Result<String> {
    Ok(serde_json::to_string(payload)?)
}

pub fn pairing_url(payload: &PairingPayload) -> Result<String> {
    Ok(format!(
        "aiterminal://pair?payload={}",
        percent_encode(&pairing_payload_json(payload)?)
    ))
}

pub fn pairing_pwa_url(payload: &PairingPayload, base_url: &str) -> Result<String> {
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
        "{base}{separator}payload={}",
        percent_encode(&pairing_payload_json(payload)?)
    ))
}

pub fn pairing_qr_text(payload: &PairingPayload) -> Result<String> {
    crate::qr::render_terminal_qr(&pairing_url(payload)?)
}

pub fn pairing_pwa_qr_text(payload: &PairingPayload, base_url: &str) -> Result<String> {
    crate::qr::render_terminal_qr(&pairing_pwa_url(payload, base_url)?)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn hex_decode(text: &str) -> Result<Vec<u8>> {
    let clean = text.trim();
    if clean.len() % 2 != 0 {
        bail!("hex 길이는 짝수여야 함");
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    let bytes = clean.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = hex_value(pair[0])?;
        let lo = hex_value(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

pub fn hex_decode_32(text: &str) -> Result<[u8; 32]> {
    let bytes = hex_decode(text)?;
    bytes
        .as_slice()
        .try_into()
        .context("hex pubkey는 32바이트여야 함")
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

fn start_pairing_with_code(
    path: &Path,
    daemon_key: &DaemonKeyRecord,
    now_ms: u64,
    ttl_seconds: u64,
    code: String,
) -> Result<PairingSession> {
    validate_daemon_key(daemon_key)?;
    validate_code(&code)?;
    if ttl_seconds == 0 {
        bail!("pairing TTL은 1초 이상이어야 함");
    }
    if let Some(existing) = load_pairing(path)? {
        if now_ms <= existing.expires_at_ms {
            bail!("이미 진행 중인 페어링이 있음");
        }
    }

    let ttl_ms = ttl_seconds.saturating_mul(1000);
    let session = PairingSession {
        code,
        daemon_pubkey: daemon_key.public.clone(),
        created_at_ms: now_ms,
        expires_at_ms: now_ms.saturating_add(ttl_ms),
    };
    write_json(path, &session)?;
    Ok(session)
}

fn complete_pairing_at(
    pairing_path: &Path,
    registry_path: &Path,
    device_id: &str,
    code: &str,
    noise_pubkey: Vec<u8>,
    approval_pubkey: [u8; 32],
    now_ms: u64,
) -> Result<RegisteredDevice> {
    validate_code(code)?;
    let session = load_pairing(pairing_path)?.context("진행 중인 페어링이 없음")?;
    if now_ms > session.expires_at_ms {
        bail!("페어링 코드가 만료됨");
    }
    if session.code != code {
        bail!("페어링 코드가 일치하지 않음");
    }

    let mut registry = DeviceRegistry::load(registry_path)?;
    let registered = registry
        .register_device(device_id, noise_pubkey, approval_pubkey, now_ms)?
        .clone();
    registry.save(registry_path)?;
    let _ = std::fs::remove_file(pairing_path);
    Ok(registered)
}

fn load_pairing(path: &Path) -> Result<Option<PairingSession>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
            format!("pairing 파일 파싱 실패: {}", path.display())
        })?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("pairing 파일 읽기 실패: {}", path.display())),
    }
}

fn generate_pairing_code() -> Result<String> {
    let mut bytes = [0u8; 4];
    getrandom::getrandom(&mut bytes)?;
    let value = u32::from_be_bytes(bytes) % 1_000_000;
    Ok(format!("{value:06}"))
}

fn validate_daemon_key(record: &DaemonKeyRecord) -> Result<()> {
    if record.public.is_empty() {
        bail!("daemon public key가 비어 있음");
    }
    if record.private.is_empty() {
        bail!("daemon private key가 비어 있음");
    }
    Ok(())
}

fn validate_code(code: &str) -> Result<()> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        bail!("pairing code는 6자리 숫자여야 함");
    }
    Ok(())
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("유효하지 않은 hex 문자"),
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("파일 쓰기 실패: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn temp_path(tag: &str, name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "ai_pairing_{}_{}_{}",
                std::process::id(),
                tag,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join(name)
    }

    fn daemon_key() -> DaemonKeyRecord {
        DaemonKeyRecord {
            public: vec![1u8; 32],
            private: vec![2u8; 32],
            created_at_ms: 10,
        }
    }

    fn approval_pubkey() -> [u8; 32] {
        SigningKey::from_bytes(&[3u8; 32])
            .verifying_key()
            .to_bytes()
    }

    #[test]
    fn daemon_key_load_or_create_roundtrip() {
        let path = temp_path("key", DAEMON_KEY_FILE);
        let first = load_or_create_daemon_key(&path).unwrap();
        assert!(!first.public.is_empty());
        assert!(!first.private.is_empty());

        let second = load_or_create_daemon_key(&path).unwrap();
        assert_eq!(second, first);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn start_pairing_rejects_concurrent_unexpired_session() {
        let path = temp_path("concurrent", PAIRING_FILE);
        let key = daemon_key();
        let first = start_pairing_with_code(&path, &key, 1000, 60, "123456".into()).unwrap();
        assert_eq!(first.daemon_pubkey, key.public);

        let err = start_pairing_with_code(&path, &key, 2000, 60, "234567".into()).unwrap_err();
        assert!(err.to_string().contains("진행 중"), "{err}");

        let second = start_pairing_with_code(&path, &key, 62_000, 60, "234567".into()).unwrap();
        assert_eq!(second.code, "234567");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn complete_pairing_registers_device_and_clears_pending() {
        let pair_path = temp_path("complete", PAIRING_FILE);
        let reg_path = pair_path.with_file_name(crate::device_registry::REGISTRY_FILE);
        let key = daemon_key();
        start_pairing_with_code(&pair_path, &key, 1000, 60, "123456".into()).unwrap();

        let registered = complete_pairing_at(
            &pair_path,
            &reg_path,
            "phone-1",
            "123456",
            vec![9u8; 32],
            approval_pubkey(),
            2000,
        )
        .unwrap();
        assert_eq!(registered.id, "phone-1");
        assert!(!pair_path.exists(), "완료된 pending pairing은 제거");

        let registry = DeviceRegistry::load(&reg_path).unwrap();
        assert_eq!(registry.devices.len(), 1);
        assert_eq!(registry.devices[0].noise_pubkey, vec![9u8; 32]);

        let _ = std::fs::remove_dir_all(pair_path.parent().unwrap());
    }

    #[test]
    fn pairing_payload_and_url_are_pwa_ready() {
        let session = PairingSession {
            code: "123456".into(),
            daemon_pubkey: vec![0xab, 0xcd],
            created_at_ms: 1000,
            expires_at_ms: 61_000,
        };
        let payload = pairing_payload(&session, "unix:///tmp/ai-terminal/device.sock");
        assert_eq!(payload.protocol_version, PAIRING_PROTOCOL_VERSION);
        assert_eq!(payload.pairing_code, "123456");
        assert_eq!(payload.daemon_pubkey_hex, "abcd");
        assert_eq!(
            payload.transport_addr,
            "unix:///tmp/ai-terminal/device.sock"
        );

        let json = pairing_payload_json(&payload).unwrap();
        assert!(json.contains(r#""protocol_version":1"#));
        assert!(json.contains(r#""pairing_code":"123456""#));
        assert!(json.contains(r#""daemon_pubkey_hex":"abcd""#));
        assert!(json.contains(r#""transport_addr":"unix:///tmp/ai-terminal/device.sock""#));

        let url = pairing_url(&payload).unwrap();
        assert!(url.starts_with("aiterminal://pair?payload="));
        assert!(
            url.contains("%7B%22protocol_version%22%3A1"),
            "payload JSON should be percent-encoded: {url}"
        );
        assert!(url.contains("123456"));

        let pwa_url = pairing_pwa_url(&payload, "http://127.0.0.1:8787/index.html").unwrap();
        assert!(pwa_url.starts_with("http://127.0.0.1:8787/index.html?payload="));
        let pwa_url_with_query =
            pairing_pwa_url(&payload, "https://example.test/app?mode=pair").unwrap();
        assert!(pwa_url_with_query.starts_with("https://example.test/app?mode=pair&payload="));

        let qr = pairing_qr_text(&payload).unwrap();
        assert!(qr.lines().count() > 4);
        assert!(qr.contains('█') || qr.contains('▀') || qr.contains('▄'));
        let pwa_qr = pairing_pwa_qr_text(&payload, "http://127.0.0.1:8787/index.html").unwrap();
        assert!(pwa_qr.lines().count() > 4);
    }

    #[test]
    fn complete_pairing_is_fail_closed_for_bad_or_expired_code() {
        let pair_path = temp_path("bad", PAIRING_FILE);
        let reg_path = pair_path.with_file_name(crate::device_registry::REGISTRY_FILE);
        let key = daemon_key();
        start_pairing_with_code(&pair_path, &key, 1000, 1, "123456".into()).unwrap();

        assert!(
            complete_pairing_at(
                &pair_path,
                &reg_path,
                "phone-1",
                "000000",
                vec![9u8; 32],
                approval_pubkey(),
                1500,
            )
            .is_err(),
            "wrong code must fail"
        );
        assert!(
            complete_pairing_at(
                &pair_path,
                &reg_path,
                "phone-1",
                "123456",
                vec![9u8; 32],
                approval_pubkey(),
                3000,
            )
            .is_err(),
            "expired code must fail"
        );

        let _ = std::fs::remove_dir_all(pair_path.parent().unwrap());
    }

    #[test]
    fn hex_helpers_roundtrip_and_validate_length() {
        let bytes = [0xabu8; 32];
        let encoded = hex_encode(&bytes);
        assert_eq!(hex_decode_32(&encoded).unwrap(), bytes);
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode_32("00").is_err());
    }
}
