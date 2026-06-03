# 설계: Gateway 시맨틱 캐시 2차 조회 + cache source 플래그

> 날짜: 2026-06-03 · 핸드오프 백로그 ② · 관련: P2-3 응답 캐시, P2-9 시맨틱 캐시, P2-1 Gateway

## 문제

`SemanticCache`(`src/cache.rs` — 단어 집합 Jaccard 유사도·임계값·TTL, `get_similar`)와 exact
`ResponseCache`(해시 키·TTL)가 둘 다 구현돼 있으나, `Gateway::ask`는 **exact 캐시만** 조회한다.
유사하지만 글자가 다른 프롬프트는 매번 백엔드를 호출 → 비용·지연 낭비. 백로그
"gateway에 시맨틱 캐시 2차 조회 결합"이 이 항목이다.

## 범위

- **포함**: `Gateway::ask`에 exact 미스 후 시맨틱 2차 조회 결합, 시맨틱 히트의 exact 승격,
  `CacheSource` 플래그를 `ai ask`·`ai dispatch` 표시까지 파급.
- **제외**(비목표):
  - config로 threshold/TTL 노출(exact 캐시도 Gateway에서 하드코딩 — 별도 후속).
  - 임베딩 기반 유사도(현 Jaccard 휴리스틱 유지).
  - 시맨틱 캐시 영속화(in-memory 유지).
  - TUI 배지 표시(`render_output`은 `..`로 흡수, 후속).

## 캐시 흐름 (`src/gateway.rs`)

`Gateway`에 `semantic: Mutex<SemanticCache>` 필드 추가. `new`에서
`SemanticCache::new(86_400, 0.85)`로 초기화 — TTL 24h는 exact와 동일, 임계값 0.85는
보수적(잘못된 근사 답변 제공 위험 최소화). exact TTL을 Gateway에서 하드코딩하는 기존
패턴과 일치(둘 다 config 노출은 후속).

`ask`의 캐시 단계(기존 3·4단계)를 다음으로 교체한다:

1. 마스킹(fail-closed) — 변경 없음.
2. 토큰 윈도 truncate — 변경 없음. 결과 텍스트 = `sent`, `input_tokens` 산출.
3. **exact 조회**: `ResponseCache::key(&sent)` → `cache.get`. 히트 시
   `Answered { source: CacheSource::Exact, .. }` 반환.
4. **시맨틱 2차 조회(신규)**: `semantic.get_similar(&sent, now)`. 히트 시:
   - 그 답을 exact 캐시에 **승격 저장**: `cache.put(key.clone(), text.clone(), now)`
     (다음번 동일 `sent`는 exact 히트).
   - `Answered { source: CacheSource::Semantic, .. }` 반환.
5. **백엔드 생성**: `backend.generate(&sent).await?`. 응답을 exact + semantic 양쪽에 저장:
   `cache.put(key, text, now)`, `semantic.put(sent.clone(), text, now)`.
   `Answered { source: CacheSource::Backend, .. }` 반환.

시맨틱 키는 마스킹된 `sent`를 사용한다(RULES §2: 마스킹된 컨텍스트만 저장 — secret 미저장).
락은 exact 캐시와 동일하게 값 복제 후 즉시 해제(백엔드 호출 중 락 미보유).

## CacheSource 플래그

`src/cache.rs`:
```rust
/// 응답 출처(캐시 계층 식별 — telemetry/표시용).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    Backend,
    Exact,
    Semantic,
}
```

파급:
- `gateway.rs`: `GatewayOutcome::Answered`에 `source: CacheSource` 필드 추가.
- `responder.rs`: `finish`가 gateway의 `source`를 `AiOutcome::Answered`로 전달.
- `dispatch.rs`: `AiOutcome::Answered`에 `source: CacheSource` 필드 추가(공유 enum 재사용).
  `MockAi`는 `CacheSource::Backend` 반환. `..` 패턴 매칭 사이트(run_dispatch/ui)는 무영향이나
  정확 일치 비교 테스트는 필드 추가.
- `main.rs`:
  - `ai ask`: Answered 표시 시 캐시 배지 출력 — `Exact`→`(cache: exact)`,
    `Semantic`→`(cache: semantic ~근사)`, `Backend`→무배지.
  - `ai dispatch`: 토큰 요약 라인에 동일 배지(헬퍼로 라벨 산출).

## 테스트

- **gateway**(`#[tokio::test]`):
  - 시맨틱 히트: 백엔드 호출을 세는 backend로 `"alpha beta gamma"` 요청(백엔드 1회, exact+semantic 저장)
    후 단어 재배열 `"gamma alpha beta"` 요청 → exact 미스(문자열 다름→해시 다름), 시맨틱 유사도 1.0
    ≥ 0.85 → 히트. 백엔드 여전히 1회, `source == Semantic`.
  - 승격: 위 직후 `"gamma alpha beta"` 재요청 → 이제 exact 히트, `source == Exact`, 백엔드 1회 유지.
  - source 기본: 첫 요청 `source == Backend`; 동일 문자열 2회째 `source == Exact`.
  - 기존 마스킹/fail-closed/취소·타임아웃 테스트 유지(필드 추가에 맞춰 패턴 보정).
- **responder**: `finish_answered_writes_to_sink`가 `source`를 전달·보존하는지(필드 추가).
- **dispatch**: `AiOutcome::Answered` 정확 일치 테스트에 `source` 필드 반영.
- **main**: 배지 헬퍼 단위 테스트(Backend→빈 문자열, Exact/Semantic→라벨).

## 비목표 (재확인)

- config threshold/TTL, 임베딩 유사도, 시맨틱 영속화, TUI 배지 — 모두 후속.
