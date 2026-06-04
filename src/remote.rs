//! 원격 승인 크립토 코어 (M0.5, `remote` feature).
//!
//! 검증 라이브러리만 사용한다(AKE를 직접 굴리지 않는다 — DESIGN M0.5):
//! - 핸드셰이크 + transport 암호화: **snow** `Noise_XX_25519_ChaChaPoly_BLAKE2s`
//!   (default resolver = 순수 Rust, `ring`/C 불필요).
//! - 승인 토큰 서명: **ed25519-dalek**(순수 Rust).
//!
//! 본 모듈은 프로토콜의 **크립토 프리미티브 래퍼**다. 데몬 프로세스·소켓·페어링·
//! nonce 저장소·context_hash 산출 등 상위 결선은 M1에서 추가한다.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Noise 패턴(§M0.5 확정). 양측 static key 상호 인증(TOFU 페어링에 적합).
pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// 데몬/디바이스의 Noise static keypair(X25519).
pub struct StaticKeypair {
    pub public: Vec<u8>,
    pub private: Vec<u8>,
}

/// Noise static keypair를 생성한다.
pub fn generate_static_keypair() -> Result<StaticKeypair> {
    let pattern = NOISE_PATTERN
        .parse()
        .map_err(|e| anyhow!("noise pattern: {e:?}"))?;
    let kp = snow::Builder::new(pattern)
        .generate_keypair()
        .map_err(|e| anyhow!("generate_keypair: {e:?}"))?;
    Ok(StaticKeypair {
        public: kp.public,
        private: kp.private,
    })
}

/// 승인 회신 서명 대상 바이트(`approval_id ‖ nonce ‖ decision`). 다른 승인/결정에
/// 서명이 재사용되지 않도록 세 값을 모두 바인딩한다.
pub fn approval_signing_bytes(approval_id: &[u8], nonce: &[u8], approve: bool) -> Vec<u8> {
    let mut b = Vec::with_capacity(approval_id.len() + nonce.len() + 1);
    b.extend_from_slice(approval_id);
    b.extend_from_slice(nonce);
    b.push(u8::from(approve));
    b
}

/// 디바이스가 승인 회신에 Ed25519 서명한다.
pub fn sign_approval(sk: &[u8; 32], approval_id: &[u8], nonce: &[u8], approve: bool) -> [u8; 64] {
    let signing = SigningKey::from_bytes(sk);
    let msg = approval_signing_bytes(approval_id, nonce, approve);
    signing.sign(&msg).to_bytes()
}

/// 데몬이 등록된 디바이스 pubkey로 승인 서명을 검증한다.
pub fn verify_approval(
    pk: &[u8; 32],
    approval_id: &[u8],
    nonce: &[u8],
    approve: bool,
    sig: &[u8; 64],
) -> bool {
    let Ok(verifying) = VerifyingKey::from_bytes(pk) else {
        return false;
    };
    let msg = approval_signing_bytes(approval_id, nonce, approve);
    verifying.verify(&msg, &Signature::from_bytes(sig)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// XX 핸드셰이크 완주 → 양측이 상대 static key를 학습(상호 인증) → transport 암복호 왕복 →
    /// 변조 암호문 거부. snow(순수 Rust) 검증.
    #[test]
    fn xx_handshake_mutual_auth_and_transport_roundtrip() {
        let pattern = || NOISE_PATTERN.parse().unwrap();
        let dev = generate_static_keypair().unwrap(); // initiator(디바이스)
        let dmn = generate_static_keypair().unwrap(); // responder(데몬)

        let mut initiator = snow::Builder::new(pattern())
            .local_private_key(&dev.private)
            .build_initiator()
            .unwrap();
        let mut responder = snow::Builder::new(pattern())
            .local_private_key(&dmn.private)
            .build_responder()
            .unwrap();

        // XX = 3 메시지: -> e ; <- e,ee,s,es ; -> s,se
        let (mut buf, mut out) = ([0u8; 1024], [0u8; 1024]);
        let n = initiator.write_message(&[], &mut buf).unwrap();
        responder.read_message(&buf[..n], &mut out).unwrap();
        let n = responder.write_message(&[], &mut buf).unwrap();
        initiator.read_message(&buf[..n], &mut out).unwrap();
        let n = initiator.write_message(&[], &mut buf).unwrap();
        responder.read_message(&buf[..n], &mut out).unwrap();

        // 상호 인증: 각 측이 상대 static pubkey를 학습했는지(transport 전환 전 캡처).
        assert_eq!(
            responder.get_remote_static(),
            Some(dev.public.as_slice()),
            "데몬이 디바이스 static key를 학습해야 함"
        );
        assert_eq!(
            initiator.get_remote_static(),
            Some(dmn.public.as_slice()),
            "디바이스가 데몬 static key를 학습해야 함(TOFU 앵커)"
        );

        let mut it = initiator.into_transport_mode().unwrap();
        let mut rt = responder.into_transport_mode().unwrap();

        // 앱 payload 암복호 왕복.
        let payload = br#"{"approval_id":"x","decision":"Approve"}"#;
        let n = it.write_message(payload, &mut buf).unwrap();
        let m = rt.read_message(&buf[..n], &mut out).unwrap();
        assert_eq!(&out[..m], payload, "transport 왕복 무손실");

        // 변조 암호문 거부(AEAD 무결성).
        let n = it.write_message(payload, &mut buf).unwrap();
        buf[0] ^= 0xff;
        assert!(
            rt.read_message(&buf[..n], &mut out).is_err(),
            "변조된 암호문은 거부되어야 함"
        );
    }

    /// Ed25519 승인 서명/검증 + 위조·다른 키·다른 결정 거부.
    #[test]
    fn ed25519_sign_verify_and_reject_forgery() {
        let sk = [7u8; 32];
        let pk = SigningKey::from_bytes(&sk).verifying_key().to_bytes();
        let approval_id = b"approval-001";
        let nonce = [9u8; 32];

        let sig = sign_approval(&sk, approval_id, &nonce, true);
        assert!(
            verify_approval(&pk, approval_id, &nonce, true, &sig),
            "정상 서명 검증"
        );

        // 결정 변조(Approve→Reject)는 거부.
        assert!(
            !verify_approval(&pk, approval_id, &nonce, false, &sig),
            "결정 바꾼 서명은 거부"
        );
        // 다른 키는 거부.
        let other_pk = SigningKey::from_bytes(&[8u8; 32])
            .verifying_key()
            .to_bytes();
        assert!(
            !verify_approval(&other_pk, approval_id, &nonce, true, &sig),
            "다른 pubkey 검증 실패"
        );
        // 서명 비트 변조는 거부.
        let mut bad = sig;
        bad[0] ^= 0xff;
        assert!(
            !verify_approval(&pk, approval_id, &nonce, true, &bad),
            "변조 서명 거부"
        );
    }
}
