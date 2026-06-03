# WORKFLOW — 개발 흐름

> **정본**: `../document/planning/09_Git_규칙_정의서.md` · `11_테스트_전략서.md` · `12_코드_리뷰_규칙.md` · `10_환경_설정_템플릿.md`.
> 본 문서는 일상 개발 루프에 필요한 명령·규칙을 압축한다.

---

## 1. 개발 루프 (한눈에)

```text
이슈 생성 → 브랜치 분기(main) → TDD 구현 → fmt/clippy/test 로컬 통과
→ PR(템플릿) → CI green + 리뷰 승인 → Squash merge → 브랜치 삭제
```

## 2. 빌드/검증 명령 (§10.4)

```bash
cargo build                                   # 개발 빌드
cargo build --release                         # 릴리스 빌드
cargo run -- doctor                           # ai CLI 실행 (진단)
cargo test                                    # 테스트
cargo fmt --all -- --check                    # 포맷 검사
cargo clippy --all-targets -- -D warnings     # 린트(경고=에러)
cargo audit                                   # 의존성 취약점
cargo deny check                              # 라이선스/공급망 (선택)
```

> **현재 개발 환경**: Rust 툴체인은 WSL(Ubuntu)에만 설치돼 있어 위 cargo 명령은 WSL에서 실행한다(`CARGO_TARGET_DIR`를 `/mnt/d` 밖으로 분리해 산출물 충돌·지연 회피). 기본 빌드는 C-free이며 `storage`(rusqlite/SQLite)·`tls`(tokio-rustls/ring)는 C 컴파일러가 필요해 feature로 게이트한다 — 검증은 default·`--features storage`·`--features tls`(필요 시 `"storage tls"`)로 한다. PTY·샌드박스 등 Linux 전용 동작은 WSL 또는 Linux CI에서 검증한다.

## 3. 브랜치 전략 — GitHub Flow (§1)

- **`main`**: 항상 배포 가능. 직접 push 금지(branch protection). 모든 변경은 PR로만.
- **작업 브랜치**: `<type>/<이슈번호>-<kebab-설명>`, 머지 후 삭제.

| 접두어 | 용도 | 예시 |
|---|---|---|
| `feat/` | 신규 기능 | `feat/142-shell-hook-install` |
| `fix/` | 버그 수정 | `fix/151-mask-fail-closed` |
| `docs/` | 문서만 | `docs/160-config-reference` |
| `refactor/` | 동작 불변 구조 개선 | `refactor/170-provider-capability-map` |
| `test/` | 테스트 | `test/175-golden-dangerous-commands` |
| `chore/` `build/` `ci/` | 빌드·의존성·CI | `chore/180-bump-tokio` |

- 릴리스 태깅: Phase별 SemVer 서명 태그(`git tag -s v1.0.0`). Phase 1=`v1.x` … Phase 4=`v4.x`.

## 4. 커밋 — Conventional Commits (§2)

```text
<type>(<scope>): <subject>   # 명령형, 72자 이내, 마침표 없음

<body: 왜 중심. 보안 변경은 위협/완화 1줄 이상>

<footer: Closes #142, BREAKING CHANGE: ...>
```

- **type**: `feat`(minor) · `fix`(patch) · `docs` · `refactor` · `perf` · `test` · `build` · `ci` · `chore` · `security` · `revert`. Breaking은 `feat!`/`fix!` 또는 footer `BREAKING CHANGE:`.
- **scope(도메인)**: `shell` · `ai` · `policy` · `mask` · `guard` · `store` · `skill` · `mcp` · `remote`.

예:
```text
feat(shell): ai init shell에 --dry-run/--diff/--uninstall 추가

Hook 기반 통합을 기본값으로 하되 rc 파일은 자동 수정하지 않는다(§29.1, §31.1).

Closes #142
```

## 5. PR & 머지 게이트 (§3)

PR 본문 템플릿: 무엇을/왜(설계 §인용) · 변경 사항 · 설계 근거/상호참조 · 테스트(단위·Golden Set·수용 기준) · **보안 체크**(보안 민감 변경 필수) · 스크린샷.

| 변경 유형 | 필수 리뷰어 | Merge 조건 |
|---|---|---|
| 일반 | 코드 오너 1 | CI 통과 + 1 Approve |
| 보안 민감(`security` 라벨) | 코드 오너 1 + **SECADMIN 1** | CI 통과 + 2 Approve(SECADMIN 필수) |
| 릴리스 | Maintainer | CI 통과 + Maintainer Approve + 태깅 검증 |

공통 게이트: `main` 직접 push 금지 · 필수 CI(fmt/clippy -D warnings/test/audit/golden) 통과 · 미해결 코멘트 0 · **Squash merge**(1 PR = 1 logical commit) · 머지 후 브랜치 삭제 · `unsafe` 추가/변경 PR은 SECADMIN 강제.

**보안 민감 범위(SECADMIN 강제)**: 마스킹·정책 엔진/프로파일·위험도 스코어링·Guardrails/preview·샌드박스·서명/자동 업데이트·플러그인 권한 경계·원격 승인.

## 6. CI 파이프라인 (`.github/workflows/ci.yml`)

PR/push 시: `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test` → `cargo audit`. CI는 **외부 AI 호출 금지(mock provider)**, golden set·속성 기반 검증은 `temperature=0` 결정성 강제(§22.6, §29.13).

## 7. 테스트 전략 (§11 요약)

테스트 피라미드: 단위(파서·위험도 분류기·마스킹) → 통합(정책·preview·스토리지) → e2e(셸 호환성). 핵심 모듈 커버리지 **≥80%**. LLM 비결정성은 golden set + 속성 기반 + N회 샘플링 안정성으로 회귀.

## 8. 릴리스 (§4)

1. `main` 최신화 + CI green
2. `git tag -s vX.Y.Z -m "..."` → `git push origin vX.Y.Z`
3. 서명 바이너리 + 체크섬 배포, 자동 업데이트는 서명 검증 후만(`verify_signature=true`), 다운그레이드 차단(`allow_downgrade=false`).
4. CHANGELOG는 Conventional Commits 기반, `security` 커밋은 별도 섹션.

## 9. 디렉터리 레이아웃 (런타임, §10.5)

```text
~/.config/ai-terminal/config.toml   # 설정 정본(§13)
~/.local/share/ai-terminal/
  ai-terminal.db  sessions/  logs/  cache/  locks/  undo/  usage.jsonl  hook.sock
```

## 10. Superpowers 설계·구현 플로우 (실제 적용)

복잡한 기능은 다음 흐름으로 진행한다(산출물은 repo의 `docs/superpowers/`에 보존):

1. **brainstorm** — 의도·제약·범위·핵심 설계 결정 합의.
2. **설계 문서(spec)** — `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`.
3. **구현 계획(plan)** — `docs/superpowers/plans/YYYY-MM-DD-<topic>.md` (TDD 단계별 Task, 정확한 파일/코드/검증 명령).
4. **subagent-driven 구현** — Task별: 구현 → spec 준수 리뷰 → 코드 품질 리뷰 → 수정 루프(리뷰 통과까지).
5. **최종 홀리스틱 리뷰** → 검증(단위 + WSL e2e, clippy/fmt clean, default·storage 전체 통과).
6. **통합** — 피처 브랜치 FF 병합 → `main` push(push가 CI 발동).

> **현재 병합 관행**: §5의 PR + branch protection + Squash merge는 설계 정본의 **목표값**이다. 현 repo는 단독 운영 단계라 로컬에서 피처 브랜치를 `main`에 **FF 병합 후 직접 push**(push가 CI 발동)하는 방식을 쓴다. PR 게이트·SECADMIN·branch protection은 협업/공개 도입 시 활성화한다.
>
> **검증 주의(WSL)**: `wsl.exe -- bash -lc '...'`에 멀티라인 전달 금지(CRLF). 종료코드는 일부 셸 경유 환경에서 `$?` 확장이 무력화될 수 있어 `cmd && echo OK || echo FAIL` 제어흐름으로 확인한다. DB 조회는 `python3` 표준 sqlite3 사용(passwordless sudo 아님).
