//! 원격 승인 등록 디바이스 저장소 (RA-2 slice 1, `remote` feature).
//!
//! 페어링 CLI/QR이 아직 붙기 전, daemon이 신뢰할 디바이스를 파일로 영속화하고
//! 승인 응답을 등록 레코드 기준으로 검증하는 작은 경계다. Noise static pubkey는
//! 연결 상대 식별용, Ed25519 approval pubkey는 승인 응답 서명 검증용으로 분리한다.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::approval::{self, ApprovalOutcome, DeviceRecord, PendingApproval};
use crate::session::ApprovalResponseMsg;

pub const REGISTRY_FILE: &str = "remote-devices.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredDevice {
    pub id: String,
    pub noise_pubkey: Vec<u8>,
    pub approval_pubkey: [u8; 32],
    pub epoch: u64,
    pub paired_at_ms: u64,
}

impl RegisteredDevice {
    pub fn to_approval_record(&self) -> DeviceRecord {
        DeviceRecord {
            pubkey: self.approval_pubkey,
            epoch: self.epoch,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceRegistry {
    pub devices: Vec<RegisteredDevice>,
}

impl DeviceRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)
                .with_context(|| format!("등록 디바이스 파일 파싱 실패: {}", path.display()))?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err)
                .with_context(|| format!("등록 디바이스 파일 읽기 실패: {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, bytes)
            .with_context(|| format!("등록 디바이스 파일 쓰기 실패: {}", path.display()))
    }

    pub fn get(&self, id: &str) -> Option<&RegisteredDevice> {
        self.devices.iter().find(|device| device.id == id)
    }

    pub fn single_device(&self) -> Option<&RegisteredDevice> {
        if self.devices.len() == 1 {
            self.devices.first()
        } else {
            None
        }
    }

    pub fn select_device(&self, id: Option<&str>) -> Result<&RegisteredDevice> {
        match id.map(str::trim).filter(|value| !value.is_empty()) {
            Some(id) => self
                .get(id)
                .with_context(|| format!("등록되지 않은 디바이스: {id}")),
            None if self.devices.len() == 1 => Ok(&self.devices[0]),
            None if self.devices.is_empty() => {
                bail!("등록 디바이스가 없음; `ai remote pair`로 먼저 등록하세요")
            }
            None => bail!(
                "등록 디바이스가 여러 개입니다; `ai remote daemon --device-id <id>`로 선택하세요"
            ),
        }
    }

    pub fn register_device(
        &mut self,
        id: impl Into<String>,
        noise_pubkey: Vec<u8>,
        approval_pubkey: [u8; 32],
        paired_at_ms: u64,
    ) -> Result<&RegisteredDevice> {
        let id = id.into();
        if id.trim().is_empty() {
            bail!("디바이스 id는 비어 있을 수 없음");
        }
        if noise_pubkey.is_empty() {
            bail!("Noise pubkey는 비어 있을 수 없음");
        }
        if self.devices.iter().any(|device| device.id == id) {
            bail!("이미 등록된 디바이스 id: {id}");
        }
        if self
            .devices
            .iter()
            .any(|device| device.noise_pubkey == noise_pubkey)
        {
            bail!("이미 등록된 Noise pubkey");
        }
        if self
            .devices
            .iter()
            .any(|device| device.approval_pubkey == approval_pubkey)
        {
            bail!("이미 등록된 approval pubkey");
        }

        self.devices.push(RegisteredDevice {
            id,
            noise_pubkey,
            approval_pubkey,
            epoch: 1,
            paired_at_ms,
        });
        Ok(self.devices.last().expect("device was just pushed"))
    }
}

pub fn registry_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join(REGISTRY_FILE))
}

pub fn validate_registered_response(
    registry: &DeviceRegistry,
    device_id: &str,
    pending: &PendingApproval,
    now: u64,
    current_context_hash: &str,
    response: &ApprovalResponseMsg,
) -> Result<ApprovalOutcome> {
    let device = registry
        .get(device_id)
        .with_context(|| format!("등록되지 않은 디바이스: {device_id}"))?;
    Ok(approval::validate(
        pending,
        &device.to_approval_record(),
        now,
        current_context_hash,
        &response.to_signed()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::PendingApproval;
    use ed25519_dalek::SigningKey;

    const DEVICE_SK: [u8; 32] = [3u8; 32];

    fn temp_registry_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ai_remote_registry_{}_{}_{}.json",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn approval_pubkey() -> [u8; 32] {
        SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes()
    }

    fn pending() -> PendingApproval {
        PendingApproval {
            approval_id: b"appr-registry-1".to_vec(),
            nonce: [41u8; 32],
            expires_at: 9999,
            context_hash: "ctx".into(),
            device_epoch: 1,
        }
    }

    #[test]
    fn missing_registry_loads_empty() {
        let path = temp_registry_path("missing");
        let registry = DeviceRegistry::load(&path).unwrap();
        assert!(registry.devices.is_empty());
    }

    #[test]
    fn register_save_load_roundtrip() {
        let path = temp_registry_path("roundtrip");
        let mut registry = DeviceRegistry::default();
        registry
            .register_device("phone-1", vec![9u8; 32], approval_pubkey(), 1234)
            .unwrap();
        registry.save(&path).unwrap();

        let loaded = DeviceRegistry::load(&path).unwrap();
        assert_eq!(loaded, registry);
        let device = loaded.single_device().unwrap();
        assert_eq!(device.id, "phone-1");
        assert_eq!(device.epoch, 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn duplicate_identity_is_rejected() {
        let mut registry = DeviceRegistry::default();
        let approval_pk = approval_pubkey();
        registry
            .register_device("phone-1", vec![9u8; 32], approval_pk, 1234)
            .unwrap();

        assert!(
            registry
                .register_device("phone-1", vec![10u8; 32], [4u8; 32], 1235)
                .is_err(),
            "id 중복은 거부"
        );
        assert!(
            registry
                .register_device("phone-2", vec![9u8; 32], [5u8; 32], 1235)
                .is_err(),
            "Noise pubkey 중복은 거부"
        );
        assert!(
            registry
                .register_device("phone-2", vec![10u8; 32], approval_pk, 1235)
                .is_err(),
            "approval pubkey 중복은 거부"
        );
    }

    #[test]
    fn select_device_uses_single_or_explicit_device() {
        let mut registry = DeviceRegistry::default();
        assert!(registry.select_device(None).is_err(), "empty registry");

        registry
            .register_device("phone-1", vec![9u8; 32], approval_pubkey(), 1234)
            .unwrap();
        assert_eq!(registry.select_device(None).unwrap().id, "phone-1");
        assert_eq!(
            registry.select_device(Some("phone-1")).unwrap().id,
            "phone-1"
        );
        assert!(registry.select_device(Some("missing")).is_err());

        registry
            .register_device("phone-2", vec![10u8; 32], [4u8; 32], 1235)
            .unwrap();
        assert!(
            registry.select_device(None).is_err(),
            "multi-device registry requires explicit selection"
        );
        assert_eq!(
            registry.select_device(Some("phone-2")).unwrap().id,
            "phone-2"
        );
    }

    #[test]
    fn validates_response_against_registered_device() {
        let mut registry = DeviceRegistry::default();
        registry
            .register_device("phone-1", vec![9u8; 32], approval_pubkey(), 1234)
            .unwrap();
        let pending = pending();
        let request = crate::session::ApprovalRequestMsg::from_pending(&pending, "rm -rf build");
        let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();

        let outcome =
            validate_registered_response(&registry, "phone-1", &pending, 100, "ctx", &response)
                .unwrap();
        assert_eq!(outcome, ApprovalOutcome::Approved);
        assert!(
            validate_registered_response(&registry, "missing", &pending, 100, "ctx", &response)
                .is_err(),
            "미등록 디바이스는 fail-closed"
        );
    }
}
