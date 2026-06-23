# FU-4 / M1 (slice 2) — 승인 검증 상태머신 + nonce 저장소 (설계)

> **작성일**: 2026-06-04 · **정본**: `../document/planning/builds/remote-approval/{DESIGN,TEST-PLAN}.md`(보안-핵심 ship 게이트), M0.5 와이어 프로토콜.
> **선행**: M0.5 크립토 코어(`remote.rs` sign/verify), M1 slice1 데몬(`daemon.rs`).
> **이 슬라이스 범위: 디바이스 승인 응답을 검증하는 보안-핵심 로직**(replay·TOCTOU·revoke·서명·만료)을 폰/네트워크 없이 Rust로 구현·테스트. Noise 전송 결선·페어링·PWA는 후속.

## 왜 지금 (TEST-PLAN: ship 게이트)

TEST-PLAN "Critical Paths(보안-핵심, ship 게이트 — proptest + known-answer + 음성 케이스)": **revoke / replay / TOCTOU 거부 = 세 보안 음성 케이스 모두 통과 필수**, 서명 검증, 위험도 경계. 이 로직은 폰/네트워크와 **독립적으로 단위 검증** 가능하므로(전송보다 먼저) 위험을 가장 크게 줄인다.

## 설계 결정 (`approval.rs`, `remote` feature)

### 타입
```
PendingApproval {          // 데몬이 요청 발행 시 보관
  approval_id: Vec<u8>,
  nonce: [u8; 32],
  expires_at: u64,         // unix sec
  context_hash: String,    // 발행 시점 컨텍스트(불투명; 산출은 slice3 §31.10)
  device_epoch: u64,       // 발행 시점 디바이스 epoch
}
DeviceRecord { pubkey: [u8; 32], epoch: u64 }   // 등록 디바이스(현재 epoch)
SignedResponse { approval_id: Vec<u8>, nonce: [u8; 32], approve: bool, sig: [u8; 64] }
ApprovalOutcome { Approved, Rejected, Invalid(InvalidReason) }
InvalidReason { ApprovalIdMismatch, NonceMismatch, Expired, Revoked, BadSignature, ContextDrift }
```

### `validate(pending, device, now, current_context_hash, resp) -> ApprovalOutcome` (순수)
검사 순서(보수적, 실패는 즉시 Invalid):
1. `resp.approval_id == pending.approval_id` 아니면 `ApprovalIdMismatch`.
2. `resp.nonce == pending.nonce` 아니면 `NonceMismatch`.
3. `now > pending.expires_at` 이면 `Expired`.
4. `pending.device_epoch < device.epoch` 이면 `Revoked`(대기 중 revoke → 단조 epoch).
5. `remote::verify_approval(device.pubkey, resp.approval_id, resp.nonce, resp.approve, resp.sig)` 실패면 `BadSignature`.
6. `current_context_hash != pending.context_hash` 이면 `ContextDrift`(TOCTOU, 실행 직전 재검증).
7. 통과: `resp.approve ? Approved : Rejected`.

### `NonceStore` (replay 차단, 상태)
- `register(nonce, expires_at)`: 요청 발행 시 등록.
- `consume(nonce, now) -> bool`: 존재 + 미만료면 **제거 후 true**(1회용). 없거나 만료면 false(=replay/unknown/expired). 두 번째 동일 nonce → false.
- `prune(now)`: 만료 nonce 정리.
- 데몬 플로우: 응답 도착 → `consume`(false면 즉시 거부) → `validate`.

### `gen_nonce() -> [u8;32]`
- `getrandom`(OS, C-free)로 32바이트 난수. `remote` feature 의존에 `getrandom` 추가.

## 범위
- **포함**: `approval.rs`(위 타입 + `validate` + `NonceStore` + `gen_nonce`) + 보안 음성 케이스 단위테스트(happy approve/reject, replay, expired, revoked, bad sig, context drift, id/nonce mismatch).
- **제외(후속)**: Noise 전송으로 요청/응답 실제 송수신(slice3), 페어링/디바이스 등록 영속화, context_hash 산출(§31.10), 데몬 게이트 플로우에 승인 결선(Medium 명령 → 승인 대기), PWA.

## 수용 기준 (완료 기준)
1. happy: 유효 서명·nonce·미만료·동일 context → `Approved`(approve=false → `Rejected`). (단위)
2. **replay**: `NonceStore.consume` 두 번째 호출 false. (단위)
3. **expired**: `now > expires_at` → `Expired`. (단위)
4. **revoke**: `pending.device_epoch < device.epoch` → `Revoked`. (단위)
5. **signature**: 위조/다른키/다른결정 서명 → `BadSignature`. (단위, remote.rs 연동)
6. **TOCTOU**: context_hash 변경 → `ContextDrift`. (단위)
7. id/nonce mismatch 거부. (단위)
8. `--features remote`·`"storage tls remote"` fmt/clippy/test green, default 불변.
