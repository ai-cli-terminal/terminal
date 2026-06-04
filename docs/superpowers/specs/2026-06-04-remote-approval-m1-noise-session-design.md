# FU-4 / M1 (slice 3) — Noise 세션 승인 왕복 (설계)

> **작성일**: 2026-06-04 · **정본**: M0.5 와이어 프로토콜, `../document/.../DESIGN.md`(M1 한 바퀴).
> **선행**: M0.5(`remote.rs` snow handshake/transport·Ed25519), M1 s2(`approval.rs` validate/NonceStore).
> **이 슬라이스 범위: 와이어 메시지 직렬화 + Noise transport 위 승인 왕복(요청 암호화 송신 → 서명 응답 수신 → 검증)을 in-repo end-to-end 실증.** 실제 소켓/Tailscale 전송·페어링·PWA는 후속(전송 substrate만 교체).

## 왜 지금

M0.5(크립토)·M1s2(검증)가 따로 검증됐다. 이 슬라이스는 둘을 **실제 Noise 암호문 위에서 잇는다** — "데몬이 ApprovalRequest를 암호화 송신 → 디바이스가 복호·서명 회신 → 데몬이 복호·검증"의 한 바퀴를 메모리 내 Noise 채널로 증명한다. 남는 건 전송 substrate(소켓/relay)와 UI(PWA)뿐임을 보인다.

## 설계 결정 (`session.rs`, `remote` feature)

### 와이어 메시지 (serde_json, M0.5 framing)
```
ApprovalRequestMsg {
  approval_id: Vec<u8>, nonce: Vec<u8>,    // serde 배열 한계 회피 위해 Vec
  command_masked: String, context_hash: String,
  expires_at: u64, device_epoch: u64,
}
ApprovalResponseMsg { approval_id: Vec<u8>, nonce: Vec<u8>, approve: bool, sig: Vec<u8> }
```
- `[u8;64]` 서명은 serde 기본 미지원 → 와이어는 `Vec<u8>`, 내부 변환 시 `[u8;32]`/`[u8;64]`로 `try_into`(길이 검증).
- `encode/decode`(serde_json bytes). Noise transport 메시지 1건 = 승인 메시지 1건(≤64KiB).

### 변환 (와이어 ↔ 내부 [M1s2])
- `ApprovalRequestMsg::from_pending(&PendingApproval, command_masked)`.
- `ApprovalResponseMsg::to_signed() -> SignedResponse`(길이 검증).
- `device_respond(req, device_sk, approve) -> ApprovalResponseMsg`(모의 디바이스: `remote::sign_approval`).

### 왕복 (Noise transport 위)
1. handshake(XX, M0.5) 완료 → daemon=responder·device=initiator transport 모드.
2. daemon: `encode(request)` → `write_message`(암호화) → device.
3. device: `read_message`(복호) → `decode` → `device_respond`(서명) → `encode` → `write_message` → daemon.
4. daemon: `read_message` → `decode` → `NonceStore.consume`(replay) → `approval::validate`(서명·만료·revoke·TOCTOU) → Approved/Rejected/Invalid.

전송 substrate(소켓/Tailscale/relay)는 `write_message`/`read_message` 바이트를 실어 나르기만 하면 된다 — 이 슬라이스는 그 substrate를 메모리 버퍼로 대체해 크립토+검증 파이프라인을 격리 검증한다.

## 범위
- **포함**: `session.rs`(와이어 타입 + encode/decode + 변환 + device_respond) + e2e 테스트(handshake → 암호화 요청 → 서명 응답 → 복호 → consume+validate = Approved; reject 변형; replay 거부).
- **제외(후속)**: 실제 소켓/Tailscale 전송에 실어 보내기(데몬↔폰), 페어링/QR·디바이스 등록 영속화, context_hash 산출(§31.10), 데몬 게이트 플로우 결선(armed Medium → 승인 대기), PWA, relay(M2).

## 수용 기준 (DoD)
1. 와이어 직렬화 roundtrip: encode→decode 무손실(요청/응답). (단위)
2. **end-to-end**: XX handshake → daemon 암호화 요청 → device 복호·서명 → daemon 복호 → `consume`+`validate` = `Approved`. (통합)
3. reject 변형: device가 approve=false 서명 → daemon `Rejected`. (통합)
4. **replay**: 동일 nonce 2차 consume false. (통합)
5. `--features remote`·`"storage tls remote"` fmt/clippy/test green, default 불변.
