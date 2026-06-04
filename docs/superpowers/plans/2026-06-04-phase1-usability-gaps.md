# Phase 1 실사용 갭 클로징 — 작업 계획

> **작성일**: 2026-06-04 · **방향**: 실사용 MVP 완성(Phase 1 갭)
> **정본**: `../../../document/docs/06-mvp-implementation-spec.md` §31, `docs/TASK.md`(M1~M4).
> 본 문서는 v0.1.0 이후 "로컬 결정성 골격"을 **실제 머신에서 end-to-end 동작**하도록 메우는 5개 작업 항목(WI)을 정의한다.

## 배경

v0.1.0 시점 구현은 Phase 1(M1~M4) + Phase 2(P2-1~P2-12) 골격이 거의 완성되고 테스트 214개가 통과하지만, 다음이 비어 있다:

- `gateway::ask`가 백엔드 호출 전 **예산 평가(`usage::evaluate`)를 적용하지 않음** (로직은 `usage.rs`에 존재, 미연결).
- `context::gather`가 `.git/HEAD`만 읽음 → 향후 파일 본문 수집 시 `.env` 등 **민감 경로 제외 가드 부재**.
- bash는 native `chpwd` 없음 → **bash cwd 변경 미반영**(zsh만 동작).
- hook 주입 불가 환경의 **Native Wrapper fallback 경로 없음**.
- TUI가 동기 `PtyExecutor`로 실행 → **실행 중 라이브 스트리밍·중단 불가**(CLI `run_in_pty_streaming`은 존재).

## 시퀀스 & 의존성

```
WI-1 예산 게이트 ─┐
WI-2 .env 가드   ─┤ (크로스플랫폼·Windows 검증, 독립)
                  │
WI-3 bash cwd ───┤ (WSL 검증, 독립)
WI-4 Wrapper ────┘ (WSL 검증, WI-3 셸 코드 재사용)
WI-5 TUI 중단 ──── (WSL/PTY 검증, run_in_pty_streaming 재사용)
```

원칙: 위험 낮고 Windows에서 검증 가능한 e2e 계약(WI-1, WI-2) → Linux 검증이 필요한 셸/PTY 항목(WI-3~5) 순.

---

## WI-1 — Gateway 예산 게이트 + estimated 배지 (W11 후속) · **1순위**

- **갭**: `gateway::ask`가 백엔드 호출 전 예산을 막지 않음. `usage::evaluate`(session $2 / month $30, warn 80% / block 100%) 미연결.
- **접근**: `ask`의 백엔드 호출 직전에 `evaluate` 삽입 → `Block`이면 `GatewayOutcome::Blocked`, `Warn`이면 응답 경고 배지. 실제 provider 응답 시 `estimated` 토큰/비용 배지(`TokenSource::Estimated`).
- **DoD(§31.7)**: 예산 100% 시 원격 AI 차단, 모든 AI 요청 usage 기록, 부정확 비용 estimated 표기. `--features storage` 통합테스트.
- **Effort**: S (CC ~1h) · **검증**: Windows(default + storage).

## WI-2 — `.env`/민감 경로 컨텍스트 제외 가드 (W7/W13 후속)

- **갭**: 향후 컨텍스트 수집기가 파일 본문을 실으면 `.env`/`.pem`/`.key`가 원격 노출. `mask::is_sensitive_path`는 있으나 수집 경로에 미적용.
- **접근**: 컨텍스트 빌더에 `is_sensitive_path` 사전 필터 + 음성 테스트(.env 경로는 컨텍스트/프롬프트에 절대 미포함). fail-closed.
- **DoD(§31.8)**: `.env` 원격 컨텍스트 제외 검증. **보안 민감 → SECADMIN 범위.**
- **Effort**: S (CC ~1h) · **검증**: Windows.

## WI-3 — bash cwd hook 연동 (W3 후속)

- **갭**: bash는 native `chpwd` 없음 → cwd 변경 미반영(zsh만).
- **접근**: `precmd`에서 직전 기록 cwd와 `$PWD` 비교 → 변경 시 `record_context_snapshot`/`update_session_cwd`. `shell.rs` hook 생성기 + `__hook` 분기.
- **DoD(§31.1/§31.10)**: bash에서 `cd` 후 컨텍스트 cwd·git_branch 갱신, hook 실패가 셸 비중단. `bash -n` + WSL e2e.
- **Effort**: M (CC ~2h) · **검증**: WSL.

## WI-4 — Native Wrapper fallback (W3 후속)

- **갭**: hook 주입 불가 환경(제한 셸/비대화형)의 fallback 경로 없음.
- **접근**: `ai`를 셸 wrapper로 두고 명령을 가로채 컨텍스트 기록 후 위임하는 모드. `ai init shell --mode wrapper`(rc 자동수정 금지 유지). WI-3 컨텍스트 헬퍼 재사용.
- **DoD(§31.1)**: hook 미가용 시 wrapper로 동등 컨텍스트 수집, 일반 명령 투명 통과.
- **Effort**: L (CC ~half-day) · **검증**: WSL.

## WI-5 — TUI mid-exec 중단 + 라이브 스트리밍 (W2 후속)

- **갭**: TUI가 동기 `PtyExecutor`로 실행 → 실행 중 출력 스트리밍·중단 불가.
- **접근**: `run_in_pty_streaming`(bounded mpsc + ctrl_c select)을 TUI 이벤트 루프에 결합 — 실행 중 청크를 `append_output`로 라이브 표시, 키 이벤트로 중단(exit 130). `TestBackend` + WSL.
- **DoD(§31.5 UX)**: 장기 명령 출력 라이브 표시, 실행 중 중단 동작.
- **Effort**: M (CC ~2-3h) · **검증**: WSL(PTY).

---

## 진행 상태

- [x] WI-1 — Gateway 예산 게이트 (완료 2026-06-04 — `with_budget` 주입식, `estimate_cost`, storage 통합테스트)
- [x] WI-2 — `.env` 컨텍스트 제외 가드 (완료 2026-06-04 — `context::allow_file_in_context`/`filter_context_paths`, `mask::is_sensitive_path` 위임)
- [x] WI-3 — bash cwd hook (완료 2026-06-04 — BASH_HOOK chpwd 에뮬레이션, WSL e2e: cd→세션 cwd 갱신)
- [ ] WI-4 — Native Wrapper fallback
- [ ] WI-5 — TUI mid-exec 중단

> 각 WI는 `docs/superpowers/{specs,plans}/`에 개별 spec/plan을 남기고, 완료 시 `docs/HISTORY.md`·`docs/TASK.md`를 갱신한다.
