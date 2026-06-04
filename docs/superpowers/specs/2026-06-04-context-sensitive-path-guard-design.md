# WI-2 — `.env`/민감 경로 컨텍스트 제외 가드 (설계)

> **작성일**: 2026-06-04 · **정본**: §31.8(마스킹·원격 적격성), §31.10(컨텍스트).
> **계획**: `docs/superpowers/plans/2026-06-04-phase1-usability-gaps.md` WI-2.

## 문제

`mask::is_sensitive_path`(`.env`/`.env.*`/`id_rsa`/`credentials`/`.pem`/`.key`)는 있으나 **컨텍스트 경계에서 사용되지 않는다.** 현재 `context::gather`는 `.git/HEAD`만 읽어 파일 본문을 수집하지 않지만, Phase 2의 파일 본문 수집기(Semantic File Index 컨텍스트화 등)가 추가되면 `.env`/`.pem` 본문이 원격 AI로 유출될 면이 열린다. §31.8 "`.env` 원격 컨텍스트 제외"가 구조적으로 보장되지 않는다.

## 설계 결정

### 컨텍스트 경계에 fail-closed 게이트를 둔다
- **추가(`context.rs`)**:
  - `allow_file_in_context(path) -> bool` — 원격 AI 컨텍스트에 파일을 포함해도 되는지. 민감 경로면 `false`. `mask::is_sensitive_path`에 위임(단일 진실원).
  - `filter_context_paths<I, S>(paths) -> Vec<String>` — 후보 파일 경로에서 민감 경로를 제거하고 순서를 보존해 반환.
- **계약**: *향후 모든 파일 본문 수집기는 원격 전송 전 이 게이트를 반드시 통과시킨다.* 게이트는 경로 기준(본문 스캔 이전 1차 방어) → 본문은 추가로 `mask`(2차 방어, secret/PII). 즉 경로 게이트(WI-2) + 본문 마스킹(기존)으로 이중 방어.
- **fail-closed**: 판정 불가/모호 시 제외(보수적). `is_sensitive_path`는 파일명 기준 결정적.

### 단일 진실원 위임
- 민감 패턴은 `mask::is_sensitive_path` 한 곳에서만 정의. context는 의미를 부여(컨텍스트 포함 여부)할 뿐 패턴을 중복 정의하지 않는다.

## 범위

- **포함**: `allow_file_in_context` · `filter_context_paths` 공개 API + 테스트(음성 케이스 포함).
- **제외(후속)**: 실제 파일 본문 수집기(Phase 2 — 이 게이트를 통과시키는 소비자) · 디렉터리 단위 제외 규칙(`.ssh/` 등 확장).

## 수용 기준 (DoD, §31.8)

1. `allow_file_in_context`가 `.env`/`.env.local`/`*.pem`/`*.key`/`id_rsa`/`credentials`를 `false`, 일반 소스(`main.rs`/`README.md`)를 `true`로 판정. (단위 테스트)
2. `filter_context_paths`가 혼합 목록에서 민감 경로를 제거하고 안전 경로 순서를 보존. (단위 테스트)
3. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green.
