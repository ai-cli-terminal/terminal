# HANDOFF — ai-cli-terminal (2026-06-04)

다음 세션 이관 문서. 권위 기록은 `docs/HISTORY.md`·`docs/TASK.md`, Claude 메모리
(`terminal-build-env`, `terminal-project-state`). 이 파일은 세션 요약 + 재개 가이드.

## 1. 현재 상태 (main = origin/main 동기, 작업트리 clean)

org `ai-cli-terminal`: `terminal`(구현) · `document`(설계 정본 v3.3) · `.github`(org 공통).
브랜치는 **main 단일**(master→main 통일 완료, 2026-06-03). CI는 main push + PR에서 동작.

## 2. 이번 세션(2026-06-04) 산출 — 전부 main 푸시·검증 green

| 커밋 | 내용 |
|------|------|
| `3c957cb` | **fix(shell)**: 영속 PTY 셸 readline이 probe 마커(`\x1f`=undo) 가로채 무한 행 수정 → bash `--noediting`. (FU-3 e2e 재확인 중 발견·수정) |
| `c5893a9`±| **FU-4 / M0**: 셸 인터셉트 제어점(bash extdebug DEBUG trap·zsh ZLE 위젯) + `gate.rs`(decide_gate §30-13)·`ai __gate`·`ai remote arm/disarm/status` |
| `5771e42` | **FU-4 / M0.5**: 와이어 프로토콜 + 크립토 코어(`remote.rs`: snow Noise_XX 순수 Rust C-free + ed25519-dalek). `remote` feature |
| `9c743ea` | **FU-4 / M1 s1**: 로컬 게이트 데몬(`daemon.rs` tokio Unix 소켓) + `ai __gate`↔데몬 결선 + 로컬 폴백 |
| `9ff8842` | **FU-4 / M1 s2**: 승인 검증 상태머신(`approval.rs`: validate + NonceStore) — replay·TOCTOU·revoke·서명·만료 음성 케이스 |
| `1c672cf` | **FU-4 / M1 s3**: Noise 세션 승인 왕복(`session.rs`) — 실제 암호문 위 end-to-end(인메모리) |
| `2719555` | **FU-4 / M1 s4a**: 전송 substrate(`session.rs` framing + 역할 함수) — 실제 `UnixStream` 위 승인 왕복 |

핵심: **원격 승인의 모든 부품이 준비됨** — 크립토(snow/dalek)·게이트(decide_gate)·검증(validate)·데몬(소켓)·세션 왕복·전송 substrate. 전부 검증된 라이브러리/메커니즘 기반(context7 snow 확인, WSL spike 실증).

## 3. 빌드·검증 환경 (메모리 `terminal-build-env` 참조)

- Rust는 **WSL(Ubuntu)에만**. `CARGO_TARGET_DIR=$HOME/targets/ai-terminal`로 분리.
  ```
  wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; \
    export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
  ```
- **feature 매트릭스**: 기본(C-free) / `storage`(SQLite, C) / `tls`(ring, C) / **`remote`(snow+dalek+getrandom, 순수 Rust C-free)**. 검증은 default + `"storage tls remote"` + clippy(`-D warnings`) + `cargo fmt`.
- 현재 규모: default 230 테스트 / `"storage tls remote"` 263.
- **WSL 함정**: 멀티라인 `bash -lc` 금지·`/mnt` 경로는 PowerShell 경유 또는 스크립트 파일(git-bash pathconv) · 종료코드는 `&&/||`로만 검증 · `rm -rf $HOME` 금지. 무한 행 테스트는 워치독 + `timeout 60 cargo test <mod>:: -- --test-threads=1`로 범인 격리.
- **gh 인증**: WSL에 `VelkaressiaBlutkrone` 로그인됨(HTTPS). PR/merge 가능.
- **커밋 주의**: `git add -A` 금지(로컬 `.omc/` 오커밋 위험 — 이미 .gitignore 처리). 명시적 파일 add.

## 4. 다음 작업 — M1 slice 4b (정본 = `docs/TASK.md`)

원격 승인 부품 조립 단계. 데몬에서 다음을 결선:
1. **디바이스 연결 리스너**: 데몬이 디바이스(폰)용 별도 소켓/TCP 리스너를 띄워 `session::run_daemon_request`를 호스팅(현재는 함수만 존재).
2. **페어링 CLI/QR**: `daemon_pubkey`를 신뢰앵커로, `pairing_code`로 디바이스 인증 + `DeviceRecord`(pubkey+epoch) 등록 영속화(TOFU, 동시 페어링 거부).
3. **게이트 플로우 결선**: armed High(opt-in) 명령 → 데몬이 등록 디바이스로 승인 왕복 트리거 → `consume`+`validate` 결과로 통과/차단. **fail-closed timeout**(폰 무응답=차단). gate에 `NeedsApproval` 밴드 추가 검토(현재 decide_gate는 Allow/Block만).
4. **데몬 컨텍스트 스냅샷**(§31.10) + `context_hash` 산출(env allowlist 해시 + realpath 타깃).
5. 이후: **PWA**(/approve·/pair, 웹 — 별도 큰 작업) → relay(M2) → 확장(#1 viz·#2 heartbeat·#4 arm TTL).

잔여 후속: bubblewrap/gVisor 격리, 영속 셸 입력 인터셉트, monthly 예산 시간창.

## 5. 빠른 재개 체크

```
git -C D:\workspace\terminal-project\terminal status            # clean (.omc만 untracked), main
gh pr list -R ai-cli-terminal/terminal                          # open PR 확인
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; \
  export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote" 2>&1 | tail -3'
```

설계/계획 문서: `docs/superpowers/specs/`·`docs/superpowers/plans/`의 `2026-06-04-remote-approval-*`.
