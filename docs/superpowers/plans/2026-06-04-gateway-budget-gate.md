# WI-1 — Gateway 예산 게이트 구현 계획 (TDD)

> **설계**: `docs/superpowers/specs/2026-06-04-gateway-budget-gate-design.md`.
> Red → Green → Refactor. 각 단계 테스트 먼저 실패 확인.

## Task 1 — `usage::estimate_cost` (순수 함수)
- **RED**: `usage.rs` 테스트 — `estimate_cost(1000, 500)`가 양수 비용 + `CostSource::Estimated`. `estimate_cost(0,0)==0.0`.
- **GREEN**: 단가 테이블 상수(입력/출력 per-token) → `(f64, CostSource::Estimated)`.
- **검증**: `cargo test --lib usage`.

## Task 2 — Gateway 예산 게이트 (주입식)
- **RED**: `gateway.rs` 테스트 3종 —
  1. `with_budget(spent=2.0, defaults)` + 새 프롬프트 → `Blocked`, 백엔드 미호출(Counting=0).
  2. `with_budget(spent=2.0, ...)`라도 **캐시된** 프롬프트 → `Answered`(백엔드 미호출, 차단 안 함).
  3. `with_budget(spent=0.5, ...)` → `Answered`(정상).
- **GREEN**: `BudgetSnapshot{spent_usd, cfg}` + `Gateway.budget: Option<_>` + `with_budget(..)`; `ask`에서 캐시 미스 후 backend 직전 `evaluate`→Block 시 `Blocked`.
- **검증**: `cargo test --lib gateway`.

## Task 3 — `ai ask` 와이어링 (main.rs)
- estimated 비용 기록(0.0 제거): `usage::estimate_cost(in,out)` → `record_usage(...cost...)`. ollama는 0.
- estimated 배지: 출력에 `(cost ~ $X.XXXX, estimated)` 추가.
- 예산 주입(`--features storage`): `store.total_cost(None)` → `gw.with_budget(spent, BudgetConfig::defaults())`. ollama 백엔드는 비용 0이라 주입해도 무해.
- **검증**: `cargo build` + `--features storage` 빌드.

## Task 4 — storage 통합 테스트
- **RED/GREEN**: `tests/integration.rs`(`#[cfg(feature="storage")]`) — usage $2 기록 후 budget snapshot으로 게이트웨이 `ask` → `Blocked`.
- **검증**: `cargo test --features storage`.

## Task 5 — 최종 검증
- default + `--features storage`: `cargo test`, `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`.
- `docs/HISTORY.md`·`docs/TASK.md`(W11)·plans 진행상태 갱신.
