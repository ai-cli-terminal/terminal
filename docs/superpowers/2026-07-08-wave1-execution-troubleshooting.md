# Wave 1 리팩토링 실행 트러블슈팅 기록 (2026-07-08)

스펙 `specs/2026-07-08-workspace-refactoring-review-design.md` / 플랜
`plans/2026-07-08-workspace-refactoring-wave1.md`의 6개 태스크를 subagent-driven
방식으로 실행하며 마주친 문제와 해결을 시간순·주제별로 정리한다. 다음 세션·다음
Wave가 같은 함정을 반복하지 않도록 하는 것이 목적이다.

실행 요약: Task 1(루트 정리, 커밋 없음) + Task 2~6(각 독립 브랜치·PR #81~#85).
전 태스크 개별 리뷰 통과, 5개 브랜치 origin/main 위로 충돌 없이 통합 검증.

---

## 1. 서브에이전트 최종 메시지 유실 (반복 발생, 최다 빈도)

**증상**: Explore·general-purpose 서브에이전트가 작업은 정상 수행하고도 최종
메시지로 `"Complete."` / `"완료."` / `"Done."` 두 단어만 반환. 보고서 본문이 통째로
유실됨. Rust 코어 매핑 에이전트는 SendMessage로 3회 재요청해도 계속 두 단어만 반환.

**원인**: 서브에이전트의 final assistant 메시지가 간헐적으로 유실되는 하니스 특성.
작업 자체는 완료됐으나 컨트롤러에 전달되는 요약이 비어버린다.

**해결/예방**:
- 모든 후속 서브에이전트에 **"보고서를 지정 파일에 쓰고, 최종 메시지엔 상태 한 줄만
  반환하라"**를 명시. 파일을 정본으로 삼는다(리포트/리뷰 파일 handoff).
- SDD 스킬의 File Handoffs 패턴과 정확히 일치 — 이 패턴이 없었으면 Task 4~6의
  대용량 리포트가 전부 유실됐을 것.

**교훈**: 서브에이전트 산출물은 처음부터 파일로 받는다. 최종 메시지는 status 신호로만.

---

## 2. 로컬 `main` 드리프트 → PR #81 diff 오염 (P1, 정정함)

**증상**: Task 2 브랜치(`refactor/wave1-architecture-doc`)를 로컬 `main`에서 분기해
PR #81을 열었더니, diff에 ARCHITECTURE.md·README 외에 무관한 11개 파일 388줄
(product packaging 문서, HANDOFF/HISTORY/PRD 등)이 섞여 들어감.

**원인**: 로컬 `main`(`990883b`)이 origin/main(`41137fa`)보다 **2커밋 앞서** 있었다.
`codex/product-packaging-companion-docs`(PR #68, GitHub에는 아직 미머지)의 커밋
2개가 로컬 `main`에만 병합돼 있던 상태. "main에서 분기"가 곧 오염된 베이스를 뜻했다.

**해결**:
- `git rebase --onto origin/main 990883b refactor/wave1-architecture-doc` 후
  `git push --force-with-lease`. PR #81 diff가 2파일 77줄로 정상화(GitHub 전파에
  ~8초 지연 후 확인).
- **이후 모든 태스크(3~6)는 `origin/main`에서 분기**하도록 지시 변경.
- Task 3 구현자는 이 사실을 스스로 감지(로컬 main이 origin/main보다 앞섬)하고
  origin/main 베이스로 선회 후 보고 — 올바른 판단이었다.

**미해결(사용자 결정 대기)**: 로컬 `main`이 origin/main보다 앞선 드리프트 자체는
이 작업 스코프 밖이라 손대지 않음. origin/main으로 리셋 권고(부록 A 브랜치 위생).

**교훈**: 공유 레포에서 "main에서 분기" 전에 `git rev-list origin/main..main`으로
로컬 main 드리프트를 항상 확인한다. 리팩토링 브랜치는 origin 기준으로 분기.

---

## 3. 서브에이전트 `rm` 권한 차단 (Task 1)

**증상**: Task 1 구현자(haiku)가 루트 `.git`·`android`·`ai-terminal` 디렉터리 삭제를
시도하자 자동 보안 시스템이 `.git` 삭제를 "비가역 파괴"로 차단. PowerShell 우회도
동일 차단. 파일 3개(test_audit_step*.sh, HANDOFF.md)는 삭제 성공, 디렉터리 3개만 BLOCKED.

**원인**: 서브에이전트 실행 환경의 안전장치가 `.git` 삭제를 위험 동작으로 판정.
브리프의 "루트는 git 밖" 전제와 실제 `.git`(껍데기) 존재가 표면적으로 모순돼 보였음.

**해결**: 구현자가 정확히 BLOCKED로 보고 → 컨트롤러가 사용자 승인 하에 직접 삭제
후 리포트에 완료 추록. Step 5 검증(`ROOT_CLEAN_OK`) 통과.

**교훈**: 비가역·고위험 파일 조작은 서브에이전트가 막힐 수 있다. 계획 단계에서
"삭제 목록 제시 → 확인 게이트 → 컨트롤러 직접 실행"으로 설계하면 매끄럽다.

---

## 4. SDD 스크립트의 cwd 요구 (task-brief / review-package)

**증상**: `task-brief`·`review-package` 스크립트가 `fatal: not a git repository`로 실패.

**원인**: 두 스크립트는 내부에서 `git rev-parse`를 호출하므로 **git 레포 안에서**
실행돼야 한다. 세션 cwd가 git 밖 워크스페이스 루트(`terminal-project/`)였음.

**해결**: `cd /d/workspace/terminal-project/terminal-wave1-wt && bash <스크립트>`.
review-package는 BASE/HEAD 해석도 레포 안에서 이뤄지므로 반드시 cwd를 레포로.

**교훈**: SDD 헬퍼 스크립트 호출은 항상 대상 레포(또는 워크트리) 안에서.

---

## 5. `git mv` 대상 디렉터리 자동생성 안됨 (Task 5)

**증상**: `git mv src/daemon.rs src/daemon/mod.rs`가 대상 디렉터리 `src/daemon/`
부재 시 실패(git-for-windows).

**해결**: `mkdir -p src/daemon` 선행 후 `git mv`.

**트레이드오프(Task 6)**: Task 6은 이 문제 + 심볼 인터리브 때문에 `git mv` 대신
`sed`로 심볼별 추출 후 원본 삭제 방식을 택함. 결과 트리는 동일하나 git이 rename으로
인식 못할 수 있어 `git log --follow` 이력 연결이 끊길 수 있음. 대신 줄 번호 산술
전수검증(2546 = 추출분 + 잔류분)과 심볼 카운트 대조로 내용 보존을 별도 입증.

**교훈**: 디렉터리 모듈 분할 시 `mkdir -p` 선행. 연속 구간이면 `git mv`로 rename
이력 보존, 심볼이 인터리브면 sed 추출 + 산술 검증(이력 연결은 포기).

---

## 6. WSL `/tmp` 로그 유실 (Task 5)

**증상**: `cargo test` 로그를 `/tmp`에 남겼는데 baseline 테스트명 목록이 이후 사라짐.

**원인**: WSL 인스턴스가 세션 중 재시작되면서 `/tmp`(WSL 네임스페이스) 초기화.
(기존 메모리에도 있던 "git-bash /tmp ≠ WSL /tmp" 함정의 연장선.)

**해결**: 로그를 `/mnt/d/...`(영구 D 드라이브 경로)에 남기고 baseline을 재대조.

**교훈**: WSL 검증 로그·중간 산출물은 `/mnt/d` 영구 경로에. `/tmp`는 휘발.

---

## 7. 플랜 결함: 브리프 mod.rs의 cfg 게이팅 누락 (Task 5)

**증상**: Task 5 브리프가 제시한 `mod.rs`의 `pub use` 코드에
`#[cfg(feature = "remote")]`가 없어, 그대로 적용 시 무피처 `cargo build`가
unresolved-import로 하드 실패.

**원인**: `daemon` 모듈은 `#[cfg(all(unix, not(android)))]`로만 게이팅돼 있고
`remote` feature 전체를 감싸지 않는다. 재수출 대상 심볼 다수는 개별적으로
`#[cfg(feature = "remote")]` 정의라, 재수출부에도 같은 cfg를 붙여야 무피처 빌드가
성립. 플랜 작성 시 이 비대칭을 놓침.

**해결**: 구현자가 원본 정의부의 cfg를 재수출부에 그대로 반영(동작 변경 아님).
`serve::{...}` 재수출을 무조건부 3개 + remote 조건부 2개로 분리.

**대조 사례(Task 6)**: `remote_transport` 모듈은 lib.rs 선언 자체가
`#[cfg(feature = "remote")]`라 모듈 내부엔 추가 게이팅 불필요. **두 모듈의 게이팅
구조 차이**가 함정이었다 — Task 6 브리프엔 이 힌트를 미리 넣어 매끄럽게 통과.

**교훈**: 모듈을 디렉터리로 분할할 때 lib.rs의 모듈 선언 cfg와 내부 심볼 cfg의
관계를 먼저 확인. 모듈 선언이 feature를 안 감싸면 재수출부에 개별 cfg 필요.

---

## 8. 워크트리 fresh checkout — `npm ci` 필요 (Task 4)

**증상**: desktop baseline 빌드(`npm run build`) 전 `node_modules` 부재.

**원인**: `git worktree add`로 만든 워크트리는 빌드 산출물·의존성을 공유하지 않음.

**해결**: baseline 빌드 전 `cd desktop && npm ci`(21 packages, ~15s).

**교훈**: 워크트리에서 JS 태스크는 `npm ci` 선행. Rust는 `CARGO_TARGET_DIR` 공유로
회피(메모리의 빌드 환경 규칙).

---

## 9. 단일 워크트리 순차 공유 (Task 2~6)

**상황**: 6개 태스크가 같은 워크트리(`terminal-wave1-wt`)를 순차 재사용. 각 태스크가
이전 태스크의 브랜치 위에서 시작 → 명시적으로 `git switch -c <새브랜치> origin/main`
으로 전환.

**리스크**: 병렬 실행 시 체크아웃 충돌 위험. 이번엔 순차라 안전했고, 각 구현자에게
"현재 브랜치가 무엇이든 clean이면 origin/main으로 전환"을 지시.

**부수 사례**: 최종 리뷰·백그라운드 리뷰어에게는 "워크트리 소스에 접근 말고 diff
파일만 읽어라"를 지시 — 다른 태스크가 체크아웃을 바꿔도 리뷰 정본(diff 파일)은 불변.

**교훈**: 단일 워크트리 순차 실행은 안전하나, 리뷰어는 워크트리 라이브 상태가 아닌
고정 diff 파일을 봐야 한다. 진짜 병렬이 필요하면 태스크별 워크트리 분리.

---

## 10. GitHub API 전파 지연

**증상**: `git push --force-with-lease` 직후 `gh pr view`가 옛 커밋 목록(4커밋)을 반환.

**해결**: ~8초 후 재조회하니 정정된 2커밋으로 갱신됨.

**교훈**: force-push 후 PR 상태 재확인은 짧은 지연을 두고.

---

## 부록: subagent-scope 규칙 준수 확인

글로벌 규칙(`subagent-scope.md`)에 따라 매 태스크 후 컨트롤러가 직접 검증:
`git log`로 커밋 범위, `merge-base`로 분기점, 트레일러(`Co-Authored-By`) 실측,
인접 레포 오염 스팟체크. **인접 레포 오염 0건**(전 작업이 워크트리 내부,
`git -C` 절대경로 사용). 서브에이전트 cwd 리셋으로 인한 엉뚱한 레포 실행 사고 없음.
