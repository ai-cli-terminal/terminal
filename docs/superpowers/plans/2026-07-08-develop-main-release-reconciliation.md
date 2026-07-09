# develop→main 릴리스 정비 계획 (2026-07-08)

develop이 축적한 managed relay 작업을 main에 릴리스하고, 양방향 분기(develop 56커밋 /
main 6커밋)와 Android reader 중복을 정리해 develop 경유 2단계 PR 흐름을 정상화한다.

> 이 문서는 **계획**이다. §4 결정 지점을 사용자가 확정한 뒤 §5 실행에 들어간다.
> git 히스토리 정비라 코드 구현 플랜과 형식이 다르다.

## 1. 현황 (2026-07-08 조사)

- **분기점**: `a66a9e9` "Release follow-up and managed relay evidence closeout".
  이후 main과 develop이 갈라졌다.
- **main 고유 6커밋** (v0.3.4 릴리스, develop 미포함): Android reader main버전
  (`047fba6`) + hardening(`60ac71c`, #64) + iOS C ABI 3커밋(#65~67, `mobile_ffi`/
  json bridge/pure shell boundary) + iOS 경계 문서.
- **develop 고유 56커밋** (main 미릴리스): managed relay 37 feat(operator setup·
  endpoint delivery·quota metering·control plane·support/abuse 등 대부분) + relay
  docs 16 + test 3 + chore 2. **Android reader develop버전**(`698fa2d`)도 포함.
- **Android reader 중복(핵심 충돌원)**: `047fba6`(main)과 `698fa2d`(develop)가
  동일 파일·동일 줄수를 도입 — 같은 작업이 양쪽에 다른 커밋으로. 이후 main은 iOS
  hardening으로, develop은 다른 방향으로 발전해 `WorkspaceDocuments.kt`가 241줄 분기.
- **충돌 파일 10개** (develop←main 머지 기준): `CHANGELOG.md`, Android 소스 4개
  (`TerminalViewModel.kt`·`WorkspaceDocuments.kt`·`WorkspaceDocumentsTest.kt`·
  `android/README.md`), 문서 4개(`docs/HANDOFF.md`·`HISTORY.md`·`TASK.md`·
  `TROUBLESHOOTING.md`), `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`.
- **VERSION**: main·develop 둘 다 `0.3.4` (develop이 버전 미갱신). 최신 태그 `v0.3.4`.
- **trust 스택(#69~80)**: codex/* 체인, main 기반, **develop 미포함 — 독립**.
  릴리스 정비와 분리 가능(정비 후 rebase).
- **Wave1 6 PR(#81~86) + PR #87**: 전부 develop base로 대기 중.

## 2. 목표

1. develop의 managed relay 작업을 **v0.4.0**으로 main에 릴리스.
2. main의 iOS C ABI 작업(#65~67)을 릴리스 결과물에 **보존**.
3. Android reader 중복을 **단일 정본**으로 해소.
4. develop이 main을 다시 **상위집합으로 포함**(develop 경유 흐름 정상화).
5. Wave1 6 PR + PR #87을 이 릴리스에 포함.

## 3. 비목표

- managed relay·iOS 기능의 동작 변경. 정비는 히스토리·병합만.
- trust 스택(#69~80) 통합 — 별도. 릴리스로 main 전진 후 그 위로 rebase(§5 Phase 6).
- develop의 56커밋 취사선택 — 전부 릴리스 대상(managed relay는 완결된 evidence 체인).

## 4. 결정 지점 (2026-07-08 사용자 확정 — 권장안 전체 채택)

아래 권장안(D1~D5)을 사용자가 확정했다. 실행 세션은 이 확정값을 전제로 §5를 수행한다.

| # | 결정 | 확정값 | 근거 |
|---|---|---|---|
| D1 | Android reader 정본 | **main 버전** ✓ | main이 hardening(#64)까지 후속 반영, 더 최신. develop 고유 변경분만 별도 확인해 추가 |
| D2 | 릴리스 버전 | **v0.4.0** ✓ | managed relay = 신규 minor feature 다수. patch 아님 |
| D3 | 정비 방향 | **A: 격리 release 브랜치** ✓ | main/develop 직접 훼손 없이 충돌 해결. 검증 후 양쪽 반영 |
| D4 | Wave1/PR#87 포함 순서 | **릴리스 전 develop 머지** ✓ | develop base라 develop에 먼저 머지하면 릴리스에 자연 포함 |
| D5 | trust 스택 타이밍 | **릴리스 후 rebase** ✓ | 독립 스택. main 전진 후 codex/* 를 새 main 위로 |

## 5. 실행 계획 (권장안 A = 격리 release 브랜치 기준)

### Phase 0 — 준비
- 백업 태그: `backup/pre-release-v0.4.0/{main,develop}` = 현재 origin tip.
  (기존 `backup/pre-develop-migration/*` 10개도 유지)
- 격리 워크트리 `terminal-release-wt`를 origin/develop 기반으로 생성
  (Wave1 워크트리와 별개, 메인 워킹트리 codex 브랜치 불변).

### Phase 1 — Wave1 + PR #87을 develop에 머지 (D4)
- CI green 확인 후 PR #81~86, #87을 순서대로 develop에 머지(GitHub merge).
  전부 develop base·충돌 없음(검증 완료)이라 순수 진행.
- 로컬 develop을 origin/develop과 동기화.
- 결과: develop = 기존 develop + Wave1 분할 + product-packaging 문서.

### Phase 2 — release 브랜치에서 main 흡수 (D3)
- `release/v0.4.0`을 develop tip에서 분기.
- `git merge origin/main` → 10개 파일 충돌. **§6 방침대로 수동 해결**.
- 해결 후 `mobile_ffi`·iOS json bridge·pure shell boundary(#65~67)가 트리에
  존재하는지 확인(main 고유 코드 흡수 검증). `lib.rs`에 `pub mod mobile_ffi;`
  복원 확인.

### Phase 3 — 버전 범프 + CHANGELOG (D2)
- `VERSION`·`Cargo.toml`·`Cargo.lock`·`desktop/package.json` → `0.4.0`.
- `CHANGELOG.md` `[0.4.0]` 섹션: managed relay, Wave1 리팩토링(구조 분할),
  product-packaging 문서, iOS C ABI 흡수를 요약.
- README/HANDOFF의 버전·상태 문구 갱신.

### Phase 4 — 통합 검증
- WSL: `cargo fmt --all -- --check` / `clippy --all-targets --features "storage tls remote" -D warnings` /
  `test --features "storage tls remote"` / `build` / `check --lib --target aarch64-linux-android`.
  판정은 파이프 없이 `&& echo PASS || echo FAIL`.
- Android: `gradle -p android :app:testDebugUnitTest`(Android reader 병합 검증 — WorkspaceDocumentsTest).
- 릴리스 자산 스모크는 태그 발행 후 GitHub Actions가 담당.

### Phase 5 — 릴리스 (D3)
- `release/v0.4.0` → `develop` PR 머지(정비 결과를 develop에 back-merge — develop이
  main 흡수분을 갖도록).
- `develop` → `main` 릴리스 PR 머지(2단계 PR 규칙). CI green 필수.
- `v0.4.0` annotated 태그 push → release.yml이 ai/ash/desktop 자산 발행.
- main = develop 동기(0 커밋차) 확인.

### Phase 6 — trust 스택 재정렬 (D5, 후속)
- main이 v0.4.0으로 전진했으므로 codex/trust-* 스택을 새 main 위로 rebase.
- 스택 최하단(#69)부터 순차 rebase·force-push. 별도 세션 가능.

## 6. 충돌 해결 방침 (Phase 2, 파일별)

| 파일 | 방침 |
|---|---|
| `WorkspaceDocuments.kt`·`TerminalViewModel.kt`·`WorkspaceDocumentsTest.kt`·`android/README.md` | **main 버전 기준**(D1). develop 고유 후속 변경이 있으면 `git log -p origin/main..origin/develop -- <file>`로 확인해 비-중복 변경만 추가. gradle test로 검증 |
| `CHANGELOG.md` | 양쪽 엔트리 **합집합**. v0.4.0 섹션으로 재구성 |
| `docs/HANDOFF.md` | develop 버전 기준(더 최신 relay 재개점) + main의 iOS 재개점 문단 추가 |
| `docs/HISTORY.md` | 시간순 **병합**(양쪽 세션 로그 보존) |
| `docs/TASK.md` | 완료/대기 카운트 재계산, 양쪽 완료 항목 합산 |
| `docs/TROUBLESHOOTING.md` | 양쪽 항목 합집합 |
| `docs/superpowers/plans/2026-07-01-remaining-work-priority.md` | develop 버전(더 최신 relay 우선순위) 채택, main 고유 항목 확인 |

원칙: **코드는 main(iOS 후속 포함)**, **relay 문서는 develop(최신)**, **로그류는 합집합**.
각 해결은 "왜 이 버전"을 커밋 메시지 또는 PR 본문에 기록.

## 7. 리스크·롤백

| 리스크 | 대응 |
|---|---|
| Android reader 병합 오류로 기능 회귀 | gradle `WorkspaceDocumentsTest` green 필수, 실패 시 Phase 2 재작업 |
| iOS 코드(#65~67) 흡수 누락 | Phase 2 말미 `mobile_ffi`·bridge 존재 + `cargo check` android 타깃으로 검증 |
| develop/main 직접 훼손 | 격리 `release/v0.4.0`에서만 충돌 해결(A안). main/develop은 PR 머지로만 전진 |
| 정비 중 새 커밋이 develop/main에 유입 | Phase 0 백업 태그 + 착수 시 tip 재확인, 짧은 수명 |
| 롤백 | 태그 push 전이면 release 브랜치 폐기. push 후면 hotfix. 백업 태그로 origin 복원(force, 사용자 승인) |

## 8. 예상 규모·순서

- Phase 1(Wave1 머지)은 기계적(충돌 없음). Phase 2(충돌 해결)가 핵심 수작업 —
  Android 4파일 + 문서 5파일, 반나절 규모. Phase 3~5는 절차적.
- **선행 조건**: Wave1 6 PR CI가 develop base에서 green이어야 Phase 1 진행.
  현재 base 변경 직후라 CI 재실행 확인 필요.
- trust 스택(Phase 6)은 릴리스와 디커플. 릴리스 완료 후 언제든.

## 9. 대안 (기각/보류)

- **B. develop→main 직접 머지**: main 6커밋(iOS)이 충돌·유실 위험. develop에 iOS를
  cherry-pick 후 진행해야 해서 A와 수고 비슷하나 안전성 낮음. 기각.
- **C. main→develop 직접 동기화 후 develop→main**: A와 유사하나 develop을 직접
  충돌 해결 대상으로 삼아, 실패 시 develop(공유 브랜치) 오염 위험. A(격리 브랜치)가 안전.
