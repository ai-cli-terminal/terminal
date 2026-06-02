# RULES — 구현·보안·코딩 규칙

> **정본**: `../document/docs/00-overview-architecture.md` §3(20대 원칙) · `02-security-policy.md` · `06-mvp-implementation-spec.md` §31 · `../document/planning/12_코드_리뷰_규칙.md`.
> 본 문서는 설계·구현·리뷰의 판단 기준이다. 충돌 시 정본 §번호가 우선한다.

---

## 1. 최상위 원칙 (절대 양보 불가)

1. **일반 쉘 호환성이 AI 기능보다 우선한다.**
2. **AI는 명령 실행자가 아니라 보조자**이며, 최종 실행 권한은 사용자에게 있다.
3. **AI 기능 장애는 터미널 장애로 전파되지 않는다.** (AI 경로와 일반 셸 경로 완전 분리)
4. PTY 상태와 AI 컨텍스트는 항상 동기화된다.
5. **AI 생성 명령은 자동 실행하지 않는다.** (`auto_execute=false` 고정)
6. 파일 변경 명령은 preview·dry-run·diff를 우선 제공한다.
7. Secret/PII 마스킹은 원격 AI 호출 전 필수이며, **마스킹 실패 시 원격 호출을 차단한다(fail-closed)**.

> 전체 20개 원칙: `../document/docs/00-overview-architecture.md` §3.

## 2. 보안 불변식 (코드/리뷰에서 반드시 보장)

- **마스킹 우회 경로 없음.** 원격 전송 전 파이프라인(Raw → Secret → PII → Masking → Validation → Remote Eligibility)을 반드시 통과. `.env`는 기본 원격 컨텍스트 제외, private key block 감지 시 원격 차단. (§31.8)
- **원문 secret은 디스크에 저장하지 않는다.** 로그·캐시·세션에는 마스킹된 값만. 캐시는 at-rest 암호화.
- **위험도 점수는 deterministic.** 동일 명령·환경 → 동일 0~100 점수. **로컬 규칙 점수가 AI 분류보다 항상 우선.** (§31.4)
- **Critical(80~100)은 실행되지 않는다.** balanced/paranoid 모두 차단.
- **정책 엔진·Zero-Trust 파이프라인은 우회 불가.** 스킬·MCP·플러그인도 동일 위험도 분류·확인·감사 경계 안에서만 동작.
- **sudo AI 명령 금지** (`allow_sudo_ai_commands=false`).
- 감사 로그는 추적성을 높이되 **민감 정보는 저장하지 않는다.**
- env 수집은 allowlist 기반, denylist(`.*TOKEN.*`/`.*SECRET.*`/`.*KEY.*`/`.*PASSWORD.*`) 적용, PATH는 hash-only. (§31.10)

## 3. 위험도 등급 ↔ 정책 액션 (§31.4)

| 등급 | 점수 | balanced | paranoid |
|---|--:|---|---|
| Low | 0~24 | 허용 | 확인 |
| Medium | 25~49 | 확인 | 확인 |
| High | 50~79 | 강한 확인 + sandbox/preview | 기본 차단 또는 강한 확인 |
| Critical | 80~100 | 차단 | 차단 |

## 4. Rust 코딩 규칙

- **포맷**: `rustfmt`(`rustfmt.toml`: edition 2021, max_width 100, imports_granularity=Module, group_imports=StdExternalCrate). CI에서 `cargo fmt --check` 강제.
- **린트**: `cargo clippy --all-targets -- -D warnings` (경고=에러). CI 강제.
- **`unsafe` 최소화.** 불가피하면 `// SAFETY:` 주석 필수 + **SECADMIN 리뷰 강제**.
- **에러 처리**: `unwrap()`/`expect()`는 테스트·초기화 외 금지. 라이브러리 경계는 `Result` 전파(`anyhow`/`thiserror` 권장).
- **AI 경로 격리**: AI 호출은 비동기(tokio)로, 타임아웃(5/15/60/180s)과 Ctrl+C 취소를 항상 동반. AI 실패가 셸 경로를 막지 않도록 패닉 전파 금지.
- **플랫폼 분기**: Linux 전용 기능(seccomp·cgroups·bubblewrap 등)은 `#[cfg(target_os = "linux")]`로 분리하고, 미지원 플랫폼은 **조용히 실패하지 말고 명시적으로 고지**(`ai doctor --guardrails`). (§31.11)
- **결정성**: 위험도 분류기·마스킹 등 보안 핵심 로직은 순수 함수로 작성해 단위 테스트와 golden set으로 고정.

## 5. 테스트 규칙

- 보안 핵심(파서·위험도 분류기·마스킹)은 **단위 테스트 필수**, 커버리지 ≥80%.
- 위험 명령 분류는 **golden set 회귀** 유지(`ls -al`=Low … `rm -rf /`=Critical).
- LLM 비결정성은 속성 기반 검증 + N회 샘플링 안정성으로 검증, CI는 `temperature=0` + **외부 AI 호출 금지(mock)**.

## 6. 리뷰 체크리스트 (PR 보안 섹션, §12)

- [ ] 마스킹 우회 경로 없음
- [ ] 정책 엔진/Zero-Trust 파이프라인 우회 불가
- [ ] AI 생성 명령 자동 실행 없음
- [ ] 시크릿이 로그/컨텍스트/캐시에 미포함
- [ ] 위험도 점수 deterministic
- [ ] 수용 기준(§31.x) 인용·충족
- [ ] `unsafe` 추가 시 `// SAFETY:` + SECADMIN 승인

## 7. 문서 규칙

- 설계 사실은 **재작성하지 말고 `../document/` §번호로 링크**한다(중복 최소화, 단일 정본).
- 구현 진행은 `docs/TASK.md` 체크박스, 결정·변경은 `docs/HISTORY.md`에 날짜 역순 기록.
- 커밋·PR·브랜치 규칙은 `docs/WORKFLOW.md`를 따른다.
