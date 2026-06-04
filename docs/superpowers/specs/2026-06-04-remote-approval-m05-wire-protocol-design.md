# FU-4 / M0.5 — 원격 승인 와이어 프로토콜 (검증 기반 설계)

> **작성일**: 2026-06-04 · **정본**: `../document/planning/builds/remote-approval/{DESIGN,CEO-PLAN,TEST-PLAN}.md`(M0.5/M1), §28(보안 모델), §30-12/13.
> **선행**: M0 인터셉트 제어점 완료(`2026-06-04-remote-approval-m0-intercept*`). 게이트 결정(`gate.rs decide_gate`)은 원격 경로도 공유한다(shared-core).
> **이 문서 범위: 와이어 프로토콜 정의(핸드셰이크·payload·서명·replay·revoke·framing) + 크립토 스택 확정.** 데몬 프로세스/소켓 서버·PWA = M1, relay = M2.

## 왜 M0.5인가 (정본)

DESIGN M0.5: "1페이지가 아니라 제대로. **핸드셰이크는 Noise(예: Noise_XX) 검증 패턴 사용**(AKE 직접 굴리지 않음), hand-roll은 앱 레이어(payload·nonce·approval_id·expires_at·device_epoch·게이트)에 한정. identity/transcript 바인딩·key confirmation·replay window·메시지 순서·디바이스 교체 명시. primitive: X25519+ChaCha20-Poly1305+Ed25519(검증 라이브러리)."

## 크립토 스택 (검증됨 — context7 `/mcginty/snow`, 2026-06-04)

| 역할 | 라이브러리 | 비고 |
|---|---|---|
| 핸드셰이크 + transport 암호화(E2E) | **snow** `Noise_XX_25519_ChaChaPoly_BLAKE2s`, **default resolver(순수 Rust)** | `ring`(C) 불필요 — `Builder::new(pattern)` 기본 resolver가 순수 Rust. 평판 High. |
| 승인 토큰 서명 | **ed25519-dalek** | 순수 Rust. 디바이스가 승인 회신에 서명, 데몬이 등록 pubkey로 검증. |
| 난수(키 생성·nonce) | `getrandom`(OS) | C 없음. |

- **전부 C-free** → storage/tls(C 필요)와 달리 C 컴파일러 의존이 없다. 다만 코어 `ai` 바이너리 경량화·빌드 매트릭스 명시를 위해 **`remote` feature로 게이트**한다(storage/tls와 동일 패턴): `remote = ["dep:snow", "dep:ed25519-dalek"]`.
- snow API(검증): `Builder::new("Noise_XX_25519_ChaChaPoly_BLAKE2s".parse()?)` → `build_initiator()`/`build_responder()` → `write_message`/`read_message`로 핸드셰이크 → `into_transport_mode()`(reliable transport, nonce 내부 관리) → transport `write_message`/`read_message`로 앱 데이터 암복호.

## Noise_XX 선택 근거

XX = 양측 **static key를 핸드셰이크 중 상호 교환·인증**(mutual auth). TOFU 페어링(데몬↔디바이스 양방향 인증)에 적합. NN(익명)·NK/KN(한쪽만 known)은 상호 인증 부족. 핸드셰이크 해시가 transcript를 바인딩(MITM·재생 방어 토대), 3번째 메시지가 key confirmation.

## 페어링 (TOFU, §28)

- **QR payload**(폰이 스캔): `{ daemon_static_pubkey, pairing_code, transport_addr, protocol_version }`. 폰은 `daemon_static_pubkey`를 **신뢰앵커**로 고정(TOFU).
- 핸드셰이크(XX) 중 디바이스 static key가 데몬에 전달 → 데몬은 **최초 1회** `pairing_code` 검증 후 디바이스 pubkey를 등록(TOFU). 이미 페어링된 상태에서 신규 페어링은 **명시적 확인 없이 거부**(동시/재페어링 거부).
- 양방향: 폰은 QR의 daemon pubkey로 데몬을 인증, 데몬은 pairing_code로 폰을 인증.

## 앱 레이어 메시지 (Noise transport 위 — 이미 암호화/인증됨)

프레이밍: `[u32 BE length][payload]`. payload는 M1에서 `serde_json`(가독·디버그 우선), 추후 CBOR 최적화 가능(프로토콜 버전 필드로 협상).

```
ApprovalRequest {
  protocol_version: u16,
  approval_id: Uuid,          // 승인 1건 식별
  nonce: [u8; 32],            // 1회용(데몬 발행, 데몬이 소비)
  command_masked: String,     // §31.8 best-effort 마스킹 적용
  context: { cwd, git_branch, host, container, ... },  // §31.10
  risk: { score: u8, level, factors: [{label, delta}] },// §31.4, 데몬 서명 facts(PWA 재계산 금지)
  expires_at: u64,            // unix epoch sec, 만료 후 거부
  context_hash: String,       // 실행 직전 재검증용(env allowlist 해시 + realpath 타깃)
  device_epoch: u64,          // revoke 단조 카운터
}

ApprovalResponse {
  approval_id: Uuid,
  nonce: [u8; 32],            // 요청 nonce 그대로 반향
  decision: Approve | Reject,
  signature: [u8; 64],        // Ed25519(서명 대상 = canonical(approval_id‖nonce‖decision))
}
```

## 보안 불변식 (M1 데몬이 강제, ship 게이트 테스트 대상)

1. **E2E**: payload 평문은 Noise transport로만 흐른다. relay(M2)는 암호문만 본다.
2. **replay 차단**: 데몬이 `nonce`를 1회용 소비(소비 셋 + `expires_at`). 재사용·만료 토큰 거부.
3. **서명 검증**: `ApprovalResponse.signature`를 등록된 디바이스 Ed25519 pubkey로 검증. `approval_id‖nonce‖decision` 바인딩(다른 승인에 재사용 불가).
4. **TOCTOU**: 승인 회신 수신 후 **실행 직전** `context_hash` 재계산·비교. 드리프트(cwd/branch/타깃 변경) 시 거부 + 재승인. ("전부는 못 묶음" 명시 — env allowlist 해시 + realpath 타깃 한정.)
5. **revoke**: `device_epoch` 단조 증가. `ai remote revoke <device>` 시 epoch++. `device_epoch < current`인 디바이스의 서명 거부(대기 중 revoke → 거부).
6. **위험도 경계(§30-13)**: 원격 승인은 **`gate.rs decide_gate` shared-core 재사용** — Critical 절대 불가(로컬 전용), High opt-in, Medium 허용. 폰은 데몬 서명 risk facts만 표시(재계산 금지).
7. **마스킹**: `command_masked`/`context`는 payload 생성 시 §31.8 best-effort 마스킹. private key 블록은 원격 차단.
8. **저하 경로(fail-closed)**: 폰 timeout/도달 불가 = **차단**(자동 로컬 승인 없음, "disarm 후 로컬 실행" 안내). 데몬 다운 = hook no-op(일반 셸 정상, 위협모델상 허용).

## 메시지 순서 / 세션

- M1 reliable transport(localhost/Unix/Tailscale TCP) → snow `into_transport_mode()`(nonce 내부 순증, 순서 보장). 손상/순서뒤바뀜 = 세션 폐기·재핸드셰이크.
- 디바이스 교체 = 새 페어링(새 static key + epoch 리셋 정책은 M1에서 확정).

## 키 관리 (§28.4)

- 디바이스 키: PWA WebCrypto **non-extractable**(IndexedDB). 분실/캐시삭제 복구 = **revoke + 재페어링**(백업 없음).
- 데몬 static key 유출: 회전(새 키 생성 + 전 디바이스 재페어링). relay 채널 토큰 유출 시 토큰 회전 후 구 자격 거부.

## feature gate / 의존성

```toml
[features]
remote = ["dep:snow", "dep:ed25519-dalek"]

[dependencies]
snow = { version = "0.9", optional = true }            # 순수 Rust default resolver
ed25519-dalek = { version = "2", optional = true }
uuid = { version = "1", features = ["v4"], optional = true }  # approval_id (remote에 포함)
```
(정확한 버전은 spike에서 고정. C-free 확인 → default 빌드 불변, `--features remote`로만 컴파일.)

## 수용 기준 (DoD — M0.5 + 크립토 코어 KAT 슬라이스)

1. **핸드셰이크 roundtrip**: XX initiator↔responder가 static key 교환·완료, transport 모드 전환. (단위)
2. **transport 암복호 roundtrip**: 앱 payload가 암호화→복호화로 무손실 왕복, **변조된 암호문 거부**. (단위)
3. **서명**: Ed25519 서명/검증 통과, **위조·다른 키 서명 거부**. (단위)
4. **C-free 확인**: `--features remote` 빌드가 C 컴파일러 없이(default resolver) 성공. (WSL)
5. default 빌드 불변(remote 미포함), `--features remote`·`"storage tls remote"` fmt/clippy/test green.

## 제외 (M1+)
데몬 프로세스·Unix/TCP 소켓 서버·armed 연동(M0 게이트↔원격 왕복 결선)·페어링 CLI/QR 생성·PWA·relay(M2)·TTL/heartbeat/viz·실제 nonce 저장소·context_hash 산출 로직. 본 스펙은 **프로토콜 정의 + 크립토 코어(검증된 라이브러리 roundtrip)**까지.
