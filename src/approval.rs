//! 원격 승인 검증 상태머신 + nonce 저장소 (M1 slice 2, `remote` feature).
//!
//! 디바이스가 보낸 **서명된 승인 응답**을 대기 중 승인·디바이스 등록정보에 대해 검증한다.
//! 보안-핵심(ship 게이트): replay(nonce 1회용)·TOCTOU(context_hash 재검증)·revoke
//! (device_epoch 단조)·서명(Ed25519, `remote::verify_approval`)·만료. 폰/네트워크와
//! 독립적으로 단위 검증 가능하다. Noise 전송 결선·페어링·context_hash 산출은 후속.

use std::collections::HashMap;

use crate::remote;

/// 데몬이 승인 요청 발행 시 보관하는 대기 항목.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub approval_id: Vec<u8>,
    pub nonce: [u8; 32],
    pub expires_at: u64,
    pub context_hash: String,
    pub device_epoch: u64,
}

/// 등록된 디바이스(현재 epoch).
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub pubkey: [u8; 32],
    pub epoch: u64,
}

/// 디바이스가 보낸 서명 응답.
#[derive(Debug, Clone)]
pub struct SignedResponse {
    pub approval_id: Vec<u8>,
    pub nonce: [u8; 32],
    pub approve: bool,
    pub sig: [u8; 64],
}

/// 검증 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Approved,
    Rejected,
    Invalid(InvalidReason),
}

/// 거부(Invalid) 사유 — 음성 케이스 테스트에서 정확히 구분한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidReason {
    ApprovalIdMismatch,
    NonceMismatch,
    Expired,
    Revoked,
    BadSignature,
    ContextDrift,
}

/// 서명 응답을 대기 항목·디바이스·현재 컨텍스트에 대해 검증한다(순수, 보수적).
/// 실패는 즉시 `Invalid(reason)`. nonce 1회용(replay)은 [`NonceStore`]가 별도로 강제한다.
pub fn validate(
    pending: &PendingApproval,
    device: &DeviceRecord,
    now: u64,
    current_context_hash: &str,
    resp: &SignedResponse,
) -> ApprovalOutcome {
    use ApprovalOutcome::Invalid;
    use InvalidReason::*;

    if resp.approval_id != pending.approval_id {
        return Invalid(ApprovalIdMismatch);
    }
    if resp.nonce != pending.nonce {
        return Invalid(NonceMismatch);
    }
    if now > pending.expires_at {
        return Invalid(Expired);
    }
    // 대기 중 revoke: 발행 시점 epoch이 현재 등록 epoch보다 낮으면 무효.
    if pending.device_epoch < device.epoch {
        return Invalid(Revoked);
    }
    if !remote::verify_approval(
        &device.pubkey,
        &resp.approval_id,
        &resp.nonce,
        resp.approve,
        &resp.sig,
    ) {
        return Invalid(BadSignature);
    }
    // TOCTOU: 실행 직전 컨텍스트가 발행 시점과 다르면 거부(재승인 필요).
    if current_context_hash != pending.context_hash {
        return Invalid(ContextDrift);
    }
    if resp.approve {
        ApprovalOutcome::Approved
    } else {
        ApprovalOutcome::Rejected
    }
}

/// 1회용 nonce 저장소(replay 차단). 요청 발행 시 `register`, 응답 도착 시 `consume`.
#[derive(Debug, Default)]
pub struct NonceStore {
    entries: HashMap<[u8; 32], u64>, // nonce -> expires_at
}

impl NonceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 발행한 nonce를 만료시각과 함께 등록한다.
    pub fn register(&mut self, nonce: [u8; 32], expires_at: u64) {
        self.entries.insert(nonce, expires_at);
    }

    /// nonce를 소비한다. 존재 + 미만료면 **제거 후 true**(1회용). 없거나 만료면 false
    /// (replay/unknown/expired). 동일 nonce 두 번째 호출은 false.
    pub fn consume(&mut self, nonce: &[u8; 32], now: u64) -> bool {
        match self.entries.get(nonce) {
            Some(&exp) if now <= exp => {
                self.entries.remove(nonce);
                true
            }
            Some(_) => {
                // 만료된 항목은 정리.
                self.entries.remove(nonce);
                false
            }
            None => false,
        }
    }

    /// 만료된 nonce를 정리한다.
    pub fn prune(&mut self, now: u64) {
        self.entries.retain(|_, &mut exp| now <= exp);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 32바이트 1회용 nonce를 생성한다(OS 난수, C-free).
pub fn gen_nonce() -> [u8; 32] {
    let mut n = [0u8; 32];
    getrandom::getrandom(&mut n).expect("getrandom");
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::sign_approval;
    use ed25519_dalek::SigningKey;

    const SK: [u8; 32] = [3u8; 32];

    fn device() -> (DeviceRecord, [u8; 32]) {
        let pk = SigningKey::from_bytes(&SK).verifying_key().to_bytes();
        (
            DeviceRecord {
                pubkey: pk,
                epoch: 1,
            },
            SK,
        )
    }

    fn pending(nonce: [u8; 32]) -> PendingApproval {
        PendingApproval {
            approval_id: b"appr-1".to_vec(),
            nonce,
            expires_at: 1000,
            context_hash: "ctx-A".into(),
            device_epoch: 1,
        }
    }

    fn signed(p: &PendingApproval, approve: bool, sk: &[u8; 32]) -> SignedResponse {
        let sig = sign_approval(sk, &p.approval_id, &p.nonce, approve);
        SignedResponse {
            approval_id: p.approval_id.clone(),
            nonce: p.nonce,
            approve,
            sig,
        }
    }

    #[test]
    fn happy_approve_and_reject() {
        let (dev, sk) = device();
        let p = pending([1u8; 32]);
        let r = signed(&p, true, &sk);
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Approved
        );
        let r = signed(&p, false, &sk);
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Rejected
        );
    }

    #[test]
    fn expired_is_invalid() {
        let (dev, sk) = device();
        let p = pending([1u8; 32]);
        let r = signed(&p, true, &sk);
        assert_eq!(
            validate(&p, &dev, 2000, "ctx-A", &r),
            ApprovalOutcome::Invalid(InvalidReason::Expired)
        );
    }

    #[test]
    fn revoked_epoch_is_invalid() {
        let (mut dev, sk) = device();
        dev.epoch = 2; // revoke → epoch 상승
        let p = pending([1u8; 32]); // 발행 시점 epoch=1 < 2
        let r = signed(&p, true, &sk);
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Invalid(InvalidReason::Revoked)
        );
    }

    #[test]
    fn bad_signature_is_invalid() {
        let (dev, _sk) = device();
        let p = pending([1u8; 32]);
        // 다른 키로 서명.
        let r = signed(&p, true, &[9u8; 32]);
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Invalid(InvalidReason::BadSignature)
        );
    }

    #[test]
    fn context_drift_is_invalid() {
        let (dev, sk) = device();
        let p = pending([1u8; 32]);
        let r = signed(&p, true, &sk);
        // 실행 직전 컨텍스트가 발행 시점과 다름(TOCTOU).
        assert_eq!(
            validate(&p, &dev, 500, "ctx-B", &r),
            ApprovalOutcome::Invalid(InvalidReason::ContextDrift)
        );
    }

    #[test]
    fn id_and_nonce_mismatch_invalid() {
        let (dev, sk) = device();
        let p = pending([1u8; 32]);
        let mut r = signed(&p, true, &sk);
        r.approval_id = b"other".to_vec();
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Invalid(InvalidReason::ApprovalIdMismatch)
        );
        let mut r = signed(&p, true, &sk);
        r.nonce = [2u8; 32];
        assert_eq!(
            validate(&p, &dev, 500, "ctx-A", &r),
            ApprovalOutcome::Invalid(InvalidReason::NonceMismatch)
        );
    }

    #[test]
    fn nonce_store_blocks_replay() {
        let mut store = NonceStore::new();
        let nonce = [5u8; 32];
        store.register(nonce, 1000);
        assert!(store.consume(&nonce, 500), "최초 소비는 성공");
        assert!(!store.consume(&nonce, 500), "재사용(replay)은 거부");
    }

    #[test]
    fn nonce_store_expired_and_prune() {
        let mut store = NonceStore::new();
        store.register([6u8; 32], 100);
        assert!(!store.consume(&[6u8; 32], 200), "만료 nonce는 거부");
        store.register([7u8; 32], 100);
        store.prune(200);
        assert!(store.is_empty(), "prune이 만료 항목을 정리");
    }

    #[test]
    fn gen_nonce_is_random_32() {
        let a = gen_nonce();
        let b = gen_nonce();
        assert_ne!(a, b, "두 nonce가 달라야 함(난수)");
    }
}
