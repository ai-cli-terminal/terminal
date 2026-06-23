# WI-1 — Gateway 예산 게이트 + estimated 비용 (설계)

> **작성일**: 2026-06-04 · **정본**: §31.7(usage/cost), §13 `[ai.usage]`.
> **계획**: `docs/superpowers/plans/2026-06-04-phase1-usability-gaps.md` WI-1.

## 문제

`gateway::ask`는 마스킹·토큰윈도·캐시는 거치지만 **백엔드(원격 AI) 호출 전 예산을 평가하지 않는다.** `usage::evaluate`(session $2 / month $30, warn 80% / block 100%)는 순수 함수로 존재하나 어디서도 호출되지 않는다. 또한 `ai ask`는 usage를 기록하되 **비용을 `0.0`으로 하드코딩**해 지출이 누적되지 않는다.

결과: §31.7 수용 기준 "예산 100% 도달 시 원격 AI 차단", "부정확 비용 estimated 표기"가 미충족.

## 설계 결정

### 1) 예산 게이트는 게이트웨이의 백엔드 호출 직전에 둔다
- **위치**: `ask`에서 exact·semantic 캐시 **미스 이후**, `backend.generate()` **직전**.
- **근거**: 캐시 히트·로컬 결과는 원격 비용이 0 → 예산과 무관하게 허용해야 한다. 예산은 *원격 전송*을 막는 것(§31.7). 캐시 뒤에 두면 "예산 초과여도 캐시된 답은 제공"이라는 올바른 동작이 자연히 성립한다.
- **차단 시**: `GatewayOutcome::Blocked("예산 초과: ${spent:.2} / ${limit:.2} (원격 AI 차단)")` 반환. 백엔드 미호출.

### 2) 게이트웨이는 storage에 의존하지 않는다 — 예산 스냅샷 주입
- 기본 빌드는 C-free(`storage` feature 게이트). 게이트웨이가 `Store`를 직접 참조하면 경계가 깨진다.
- **주입**: `Gateway`에 `budget: Option<BudgetSnapshot>` 필드. `BudgetSnapshot { spent_usd: f64, cfg: BudgetConfig }`.
  - `None`(기본) → 예산 미적용(현행 동작 보존).
  - `Some(_)` → `usage::evaluate`로 Block 판정 시 차단.
- 호출측(`main.rs`)이 `store.total_cost(None)`로 spent를 읽어 주입(`--features storage`). default 빌드는 영속 지출을 모르므로 미주입.
- **빌더**: `Gateway::with_budget(self, spent_usd, cfg) -> Gateway`.

### 3) 예산 한도는 session($2)을 바인딩 제약으로 쓴다
- 기존 `ai usage` CLI가 `total_cost(None)`를 `session_usd`에 평가하는 것과 일관. 월 시간창(monthly window) 추적은 후속(현재 total_cost는 전체 합계).

### 4) estimated 비용 추정 — `usage::estimate_cost`
- **추가**: `usage::estimate_cost(input_tokens, output_tokens) -> (f64, CostSource)`.
  - MVP 기본 단가 테이블(예: $0.000_003/입력토큰, $0.000_015/출력토큰 — 가정값, provider 미보고 시).
  - provider가 비용을 보고하지 않으므로 `CostSource::Estimated` 반환.
- `ai ask`가 응답 후 이 값으로 `record_usage`(0.0 하드코딩 제거) + "(estimated)" 배지 표시.
- **로컬 백엔드(ollama)**: 비용 0 + `CostSource::PricingTable`이 아니라 0 기록 → 로컬 사용은 지출 누적 안 함(§31.7 "로컬 LLM 비용 0"). 따라서 예산 스냅샷을 모든 백엔드에 주입해도 로컬-only 사용은 절대 차단되지 않는다.

## 범위

- **포함**: 게이트웨이 예산 게이트(주입식) · `usage::estimate_cost` · `ai ask` 와이어링(estimated 기록·배지) · storage 통합테스트(초과 시 차단).
- **제외(후속)**: 월 시간창 추적, provider-reported 실비용, `ai ask` 외 경로(dispatch responder)의 예산 게이트 — 동일 패턴으로 후속.

## 수용 기준 (완료 기준, §31.7)

1. 예산 스냅샷이 block 임계(spent ≥ session_usd) → `ask`가 백엔드 호출 없이 `Blocked` 반환. (게이트웨이 단위 테스트)
2. 캐시 히트는 예산 초과여도 답을 반환(백엔드 미호출이므로). (단위 테스트)
3. `estimate_cost`가 토큰 수에 비례한 양수 비용 + `CostSource::Estimated` 반환. (단위 테스트)
4. `--features storage`: 누적 지출이 $2 이상이면 `ai ask`가 원격 차단 문구 출력. (통합 테스트)
5. 모든 AI 요청 usage 기록(비용 0.0 하드코딩 제거), estimated 배지 표시.
6. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green.
