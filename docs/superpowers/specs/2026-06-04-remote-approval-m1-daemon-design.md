# FU-4 / M1 (slice 1) — 로컬 게이트 데몬 + hook 결선 (설계)

> **작성일**: 2026-06-04 · **정본**: `../document/planning/builds/remote-approval/DESIGN.md`(M1), §30-1/§30-13.
> **선행**: M0(인터셉트·`ai __gate`·`gate.rs`), M0.5(크립토 코어 `remote.rs`).
> **이 슬라이스 범위: 영속 데몬(tokio Unix 소켓) + `ai __gate`가 데몬에 블로킹 질의 → 게이트 결정 회신 + fail-safe fallback.** phone Noise 왕복·컨텍스트 스냅샷·페어링·PWA·nonce 저장소 = M1 후속 슬라이스.

## 왜 데몬인가 (DESIGN M1)

원격 승인의 한 바퀴는 "armed + 게이트 명령 → (블로킹 IPC) → Host 데몬 → 결정 → 실행/차단". 데몬은 장수 프로세스로 (a) 컨텍스트 스냅샷 유지, (b) 향후 폰 왕복 수행, (c) nonce 소비, (d) 디바이스 레지스트리 보유의 자리다. M0는 `ai __gate`가 **로컬** 결정만 했다. 이 슬라이스는 그 결정 지점을 **데몬으로 옮길 수 있는 IPC 경로**를 깐다(향후 폰 왕복이 끼어들 seam).

## 설계 결정

### 로컬 IPC 프로토콜 (hook ↔ 데몬, **신뢰 경계 내부** — Noise 아님)
- hook↔데몬은 같은 머신의 신뢰된 로컬 IPC다. phone↔데몬의 Noise E2E(M0.5)와 **다른 채널**이다.
- 전송: **Unix 도메인 소켓**(`tokio::net::UnixListener`). 경로 `<config_dir>/gate.sock`.
- 프레이밍: **개행 구분 JSON**(연결당 1요청/1회신 후 종료).
  - 요청: `{"command": "<cmd>"}`
  - 회신: `{"decision": "allow"|"block", "reason": "<설명>"}`

### 결정 로직 (shared-core 재사용)
- 데몬은 요청마다 `gate::load_arm_state` + `gate::decide_gate`(§30-13)로 결정 → 회신. M1-floor(폰 없음)는 데몬이 로컬과 동일 로직으로 결정(폰 왕복이 끼어들 자리). **Critical 불가·High opt-in·Medium 허용** 경계 동일.

### `ai __gate` 결선 + fail-safe
1. **armed 아님** → 즉시 통과(exit 0). 데몬 접촉 없음(hot-path 보존).
2. **armed** → 데몬 소켓에 동기 연결(`std::os::unix::net::UnixStream`, 짧은 timeout) → 질의 → 회신대로 통과/차단.
3. **데몬 도달 불가**(미실행/소켓 없음) → **로컬 `decide_gate`로 폴백**(M0 동작 보존). 근거: 데몬 다운은 보안 경계가 아니라 자기-가드레일이며(DESIGN Threat Model), 셸을 막지 않으면서도 로컬 게이트로 보호를 유지한다. (폰 timeout=차단인 §저하경로와 구분 — 그건 M1 폰 왕복 슬라이스에서.)

### `ai remote daemon`
- 데몬을 포그라운드 실행(tokio). 소켓 바인드 → accept 루프. SIGINT로 종료. (백그라운드화/서비스 등록은 후속.)
- 기존 소켓 파일이 stale면 정리 후 바인드.

### 플랫폼
- Unix 소켓이라 데몬·클라이언트 경로는 **`#[cfg(unix)]`**. 비-unix는 `ai remote daemon` 미지원, `ai __gate`는 로컬 결정만(현행). hook 자체가 bash/zsh(unix)라 일관.
- core 의존만 사용(tokio·serde·gate). 크립토(snow/dalek, `remote` feature)는 폰 왕복 슬라이스에서 결합 → 이 슬라이스는 **feature 무관, default 빌드(unix)** 에 포함.

## 범위

- **포함**: `daemon.rs`(소켓 프로토콜 타입 + `decide_request` + `serve`(tokio) + `query`(sync client) + `socket_path`), `ai remote daemon` 커맨드, `ai __gate` 데몬 질의+폴백 결선, 단위(decide_request) + 통합(serve↔query roundtrip) 테스트.
- **제외(후속)**: phone Noise 왕복·페어링/QR·PWA·컨텍스트 스냅샷(§31.10) 데몬 보유·nonce 소비·context_hash·revoke·TTL/heartbeat. 데몬 백그라운드화/자동기동.

## 수용 기준 (완료 기준)

1. `daemon::decide_request("rm -rf /")`가 armed 시 block, 비-armed 시 allow(§30-13). (단위)
2. `serve`↔`query` 통합: 임시 소켓에 데몬 태스크 기동 → 클라이언트 질의 → allow/block 정확 회신. (통합, unix)
3. `ai __gate`: armed + 데몬 실행 중 → 데몬 결정 사용. armed + 데몬 없음 → 로컬 폴백(M0 동작). 비-armed → 통과. (통합/수동)
4. `ai remote daemon`이 소켓 바인드·accept, stale 소켓 정리. (수동 e2e)
5. default(unix) + `--features "storage tls remote"` fmt/clippy/test green. 비-unix 빌드 깨지지 않음(cfg 가드).
