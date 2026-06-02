# MVP 진입 결정 (§31.12 체크리스트 · §31.13 확정값)

> 작성일 2026-06-02. 설계 정본 `../document/docs/06-mvp-implementation-spec.md` §31.12·§31.13.
> M1~M4의 **로컬 결정성 핵심**을 모두 구현·검증했다. 실제 AI provider 연동(원격 호출·
> 자동 usage 기록·end-to-end 응답)은 Phase 2(Model Gateway)에서 결합한다.

## §31.12 9개 영역 체크리스트

| 영역 | 상태 | 구현 |
|---|---|---|
| **Shell** | ✅ | Hook 생성(bash/zsh)·rc dry-run/diff/uninstall·Wrapper fallback 설계(`shell.rs`). WSL `bash -n`/`zsh -n` 검증 |
| **Storage** | ✅ | WAL + 7테이블 + 파일 락 + stale 회수 + 동시성 무손상(`store.rs`, `lock.rs`) |
| **Policy & Risk** | ✅ | 0~100 deterministic 스코어링·balanced/paranoid·Critical 차단 100%(`risk.rs`, `policy.rs`) |
| **Preview & Undo** | ✅ | preview 전략 분류·best-effort 롤백·백업 상한(`preview.rs`, `undo.rs`) |
| **Usage** | ✅ | usage event·예산 평가(80%/100%)(`usage.rs`, `store.record_usage`) |
| **Privacy** | ✅ | Secret/PII 마스킹·fail-closed·env secret 미저장(`mask.rs`, `context.rs`) |
| **Provider** | ✅ | capability map + 명시적 fallback + mock(`provider.rs`, `tokenwin.rs`) |
| **Context Sync** | ✅ | cwd/shell/git 추적·env allow/deny·mismatch refresh(`context.rs`) |
| **Guardrails** | ✅ | baseline + 플랫폼 capability matrix + `ai doctor --guardrails`(`guardrails.rs`) |

## §31.13 최종 확정값 (구현 반영)

```text
Shell    : Hook default + Native Wrapper fallback; rc dry-run/diff/uninstall  ✅
Storage  : SQLite WAL + file lock + stale lock cleanup (ai-terminal.db)        ✅
Policy   : balanced default + paranoid; local policy wins                      ✅
Risk     : rule-based 0~100; Critical blocked; AI classification advisory      ✅(AI 보조 신호는 P2)
Preview  : strategy per command; dry-run first; temp-copy diff(분류까지)        ◑(실제 diff 생성 P2)
Undo     : best-effort file rollback; 500MB cap; 7-day TTL                     ✅(자동 트리거 P2)
Usage    : usage event + budget; estimated when unavailable                    ✅(자동 기록 P2)
Privacy  : Secret/PII masking on; block remote AI on masking failure           ✅
Provider : minimal interface + capability map                                 ✅(HTTP 어댑터 P2)
Context  : cwd/exit/git/shell required; env allowlist only                     ✅(hook 자동적용 P2)
Guardrails: static + preview + timeout baseline; platform matrix              ✅(동적 감시 P2)
```

## KPI 검증 (§3)

| 지표 | 목표 | 현재 |
|---|---|---|
| 위험도 점수 결정성 | deterministic | ✅ 통합 테스트(50회 동일) |
| Critical 명령 차단 | 100% | ✅ 두 프로파일 모두 Block |
| Secret/PII 마스킹 누락 | 0건 | ✅ 무유출 속성 테스트 |
| 핵심 모듈 테스트 | ≥80% 커버리지 | ◑ 테스트 114+통합 4(커버리지 도구 측정은 후속) |
| 입력 지연 ≤10ms / 라우팅 ≤100ms / 응답 ≤3s | — | ◑ 실행·provider 연동(P2) 후 측정 |

## 결론

설계 §31.12의 9개 영역과 §31.13 확정값을 **로컬 결정성 범위에서 모두 구현**했고,
크로스플랫폼(Windows/WSL)에서 테스트·clippy·fmt가 통과한다. AI provider에 의존하는
원격 경로(자동 usage·preview diff 실행·hook 자동 적용·동적 guardrails)는 Phase 2에서
결합한다. 본 저장소는 **MVP+ 핵심 골격 완료** 상태다.
