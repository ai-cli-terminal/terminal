# PRD — AI CLI 통합 리눅스 터미널 (구현 working-set)

> **정본**: `../../document/` (설계 v3.3, "MVP spec finalized — ready to build").
> 본 문서는 정본을 **압축·링크**한 구현 작업용 요약이다. 값이 충돌하면 항상 `../document/docs/`의 §번호 정본을 따른다.
> 주요 출처: `00-overview-architecture.md`(§0~§5,§32) · `06-mvp-implementation-spec.md`(§31) · `planning/01_프로젝트_계획서.md` · `planning/07_요구사항_정의서.md`

| 항목 | 값 |
|---|---|
| 제품명 | AI CLI 통합 리눅스 터미널 (`ai` CLI) |
| 설계 버전 | v3.3 (2026-06-01) |
| 구현 repo 부트스트랩 | 2026-06-02 |
| 기술 스택 | Rust · ratatui · crossterm · tokio · portable-pty · rusqlite(SQLite WAL) |
| 대상 플랫폼 | Linux(우선) / WSL / macOS |

---

## 1. 한 줄 정의

일반 리눅스 터미널과 **완전 호환**되는 실행 환경을 유지하면서, AI 명령 생성·설명·디버깅·로그 분석·자동화를 **안전하게** 결합하는 단일 바이너리 터미널.

> 설계 철학: **"AI는 명령 실행자가 아니라 의사결정 보조자."**

## 2. 해결하는 문제 (배경)

1. **잘못된 cwd/컨텍스트 기반 위험 명령 제안** — AI가 셸 상태를 추적 못해 의도와 다른 경로에서 `rm -rf`·광범위 `chmod` 실행.
2. **AI 장애의 터미널 전파** — AI가 느려지거나 죽으면 일상 셸 작업까지 멈춤.
3. **민감정보 유출** — 명령·로그·env의 Secret/PII가 마스킹 없이 원격 LLM으로 전송.

## 3. 핵심 목표 (기능, §2.1 / 계획서 §1.2)

① 일반 리눅스 명령 실행 · ② `ai "..."` 인라인 호출 · ③ 자연어→명령 변환 · ④ 실행 전 설명·위험도(0~100) 분석 · ⑤ 결과 요약 · ⑥ 에러 분석(`ai explain last-error`) · ⑦ 자동화 스크립트 생성(자동 실행 금지) · ⑧ 컨텍스트 기반 추천 · ⑨ 재현 가능한 세션 로그(`ai-terminal.db`) · ⑩ 권한 확인 게이트 · ⑪ preview/dry-run/diff · ⑫ 정적 정책 검사 + 동적 가드레일.

비기능 목표: 높은 셸 호환성 · 낮은 입력 지연(일반·AI 경로 분리) · Secret/PII 마스킹 필수 · 로컬 우선·오프라인 기본 동작 · **AI 실패 시 끊김 없는 터미널 사용** · PTY 상태와 AI 컨텍스트 상시 동기화.

## 4. 대상 사용자 (페르소나)

| 역할 | 관심사 | 대표 기능 |
|---|---|---|
| **DEV** 개발자 | 빠른 작성·디버깅, 낮은 지연 | 자연어→명령, 에러 분석, preview/diff, undo, 히스토리 |
| **OPS** 운영자 | 안전한 운영, 위험 명령 통제 | 로그 분석, 위험도 분석, 강한 확인 게이트, 감사 로그 |
| **연구자** | 반복 실험 자동화, 재현성 | 자동화 스크립트, 사용량·토큰 추적, 작업 로그 |
| **보안 관리자** | 정책 강제, 마스킹, 감사 | 정책 프로파일, Secret/PII 마스킹, 감사 이벤트 |

## 5. 범위

### 5.1 MVP+ 포함 (Phase 1, §25.1 · 계획서 §1.4)

PTY+bash/zsh 호환 · `ai "..."` 인라인/자연어 변환/설명 · 실행 전 확인+Critical 차단 · **Ctrl+C AI 취소와 정상 복구** · **AI 타임아웃** · **Secret/PII 마스킹** · **토큰 윈도 기본 기능** · **`ai preview`+diff** · **컨텍스트 일관성 기본 기능** · **정책 프로파일(balanced/paranoid)** · **alias 충돌 감지** · 에러 분석/세션 히스토리/감사 로그/사용량·토큰 추적.

### 5.2 MVP 제외 (의도적 — "무엇을 안 할 것인가"가 핵심 결정, §30)

시맨틱/벡터 인덱스(P2) · MCP 연동(P2) · 원격 릴레이·승인(P3) · 로컬 LLM 고도화(P2) · 조직 정책 배포(P3) · 스킬 서명/마켓플레이스(P3) · gVisor/Firecracker(P3~4) · eBPF 감시(P3) · 데몬 아키텍처(P2).

### 5.3 MVP 확정값 (§31.13)

```text
셸       : Hook 기본 + Native Wrapper fallback; rc dry-run/diff/uninstall 필수
저장소   : SQLite WAL + file lock + stale lock cleanup (ai-terminal.db)
정책     : balanced 기본 + paranoid 필수; 로컬 정책 우선
위험     : 규칙 기반 0~100 점수; Critical 차단; AI 분류는 보조 신호만 사용
미리보기 : 가능한 파일 수정에는 필수; dry-run 우선; temp copy 기반 diff
실행 취소: best-effort 파일 롤백만 지원; 500MB 상한; 7일 TTL
사용량   : AI 요청마다 usage event 기록; provider 데이터가 없으면 추정값 사용
프라이버시: Secret/PII 마스킹 기본 활성화; 마스킹 실패 시 원격 AI 차단
공급자   : 최소 인터페이스 + capability map 필수
컨텍스트 : cwd/exit code/git state/shell/hostname 필수; env allowlist만 허용
가드레일 : 정적 분석 + preview + timeout baseline; platform capability matrix
```

## 6. 성공 지표 (KPI, 계획서 §3)

| 분류 | 지표 | 목표 |
|---|---|---|
| 성능 | 일반 명령 입력 지연 | ≤ 10ms |
| 성능 | AI 라우팅 지연 | ≤ 100ms |
| 성능 | 짧은 AI 응답 | ≤ 3s |
| 보안 | Secret/PII 마스킹 누락 | 0건 (fail-closed) |
| 보안 | Critical 명령 차단 | 100% |
| 보안 | 위험도 점수 결정성 | deterministic |
| 품질 | 핵심 모듈 테스트 커버리지 | ≥ 80% |

위험도 등급: Low 0~24 / Medium 25~49 / High 50~79 / Critical 80~100.
AI 타임아웃 단계: 5s / 15s / 60s / 180s. 예산: 세션 $2 / 월 $30 (80% 경고, 100% 차단).

## 7. 아키텍처 한눈에 (§5)

5계층 + Phase 3 Remote Gateway. 데이터 흐름 2경로 완전 분리:

- **일반 셸 경로**: `입력 → 분류기 → 정책 엔진 → PTY 실행 → (가드레일 동적 감시) → 출력` (AI 계층 미경유 → 최소 지연)
- **AI 경로**: `입력 → 분류기 → 컨텍스트 동기화 → 제로 트러스트 파이프라인 → 토큰 윈도 → 모델 게이트웨이 → 에이전트 파이프라인 → 제안/설명/preview → 사용자 확인 → (선택 시) PTY 실행`

핵심 도메인 7개: 1) 터미널 코어 2) AI 서비스 3) 보안과 정책 4) 컨텍스트 일관성 관리자 5) 실행 가드레일 엔진 6) 저장소와 감사 7) Subsystems(Skill§26/MCP§27/Remote§28). **MVP는 1~6 구현, 7은 Phase 2+.**

## 8. 상세 참조 색인

| 주제 | 정본 |
|---|---|
| 원칙 20개 / 시나리오 / 아키텍처 | `../document/docs/00-overview-architecture.md` |
| 컴포넌트·Guardrails·컨텍스트·권한·호환성 | `../document/docs/01-core-design.md` |
| 보안·마스킹·정책 | `../document/docs/02-security-policy.md` |
| MVP 스키마·점수표·룰셋·수용 기준 | `../document/docs/06-mvp-implementation-spec.md` §31 |
| 요구사항 FR/NFR | `../document/planning/07_요구사항_정의서.md` |
| 설정·디렉터리·툴체인 | `../document/planning/10_환경_설정_템플릿.md` |
