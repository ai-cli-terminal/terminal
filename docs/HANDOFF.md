# HANDOFF — ai-cli-terminal (2026-06-23)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, 그리고 `docs/superpowers/` 아래 spec/plan 문서다. 이 파일은
재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태

작업 repo는 `D:\workspace\terminal-project\terminal`이고 브랜치는 `main`이다.
상위 workspace에는 `.github`, `document`, `terminal` 하위 repo가 있다. 2026-06-23
세션에서 하위 repo pull을 완료했고, `terminal`은 `origin/main` 기준 최신 상태에서
문서 정렬 작업을 진행했다.

현재 제품 방향은 기존 "bash/zsh 위 AI 보조 레이어"가 아니라 **플랫폼별 독립
로컬 터미널 `ash`**다. 모바일도 PWA 승인 화면이 아니라 **온디바이스 로컬
터미널**을 장기 목표로 둔다.

## 2. 이번 세션 산출

| 파일 | 내용 |
|---|---|
| `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md` | Linux/WSL/Windows/Git Bash/PowerShell/Android/iOS/PWA/remote host별 목표 매트릭스. Android는 P1 모바일 로컬 터미널, iOS/iPadOS는 P2 research, PWA는 companion으로 재배치 |
| `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md` | PM-0~PM-6 세부 workflow. 다음 구현자는 이 문서의 PM-1부터 집으면 된다 |
| `docs/TASK.md` | 현재 진행 상태 표 추가. `ai`, Phase 1/2 안전 코어, RA 기반, `ash`/`shellcore`, Windows/Android/iOS/PWA 상태와 다음 gap 정리 |
| `docs/WORKFLOW.md` | 플랫폼/모바일 로컬 터미널 Task Workflow 추가. 플랫폼별 필수 증거와 완료 처리 규칙 정의 |
| `README.md` | 플랫폼 목표 매트릭스와 workflow 문서 진입점 추가. 현재 상태에 Android/iOS/PWA companion 재배치 반영 |
| `docs/superpowers/specs/2026-06-05-independent-shell-s0-core-design.md` | `shellcore`를 Linux/WSL/Windows와 모바일 로컬 터미널에서 공유한다는 방향 반영 |
| `docs/superpowers/specs/2026-06-05-phase3-roadmap-design.md` | Phase 3 순서를 R0 → PM(platform/ash/mobile) → RA companion → P3로 재정렬 |

검증:

- `git diff --check` 통과
- 새 문서 경로 `Test-Path` 확인
- `rg`로 Android/iOS/PWA companion/모바일 로컬 터미널 참조 확인
- 코드 변경 없음. 이번 세션의 문서 작업에는 cargo 테스트를 돌리지 않았다

## 3. 중요한 결정

- `ash`가 최종 제품 코어다. `ai`는 기존 CLI/정책/진단/릴리즈 연속성을 유지한다.
- 모바일 제품 본체는 PWA가 아니다. Android는 로컬 터미널 P1, iOS/iPadOS는 제한적 로컬 터미널 research다.
- RA/PWA는 S4 companion이다. 승인, 페어링, 모니터링, 웹 데모 역할로 유지한다.
- PowerShell은 `ash` 문법에 섞지 않는다. Windows에서는 host/execution target/adapter로 다룬다.
- Git Bash/MSYS는 Windows native와 암묵적으로 섞지 않고 별도 profile로 둔다.
- iOS는 완전한 Linux 터미널을 약속하지 않는다. self-contained `shellcore`, 파일 컨테이너, 허용 명령 subset을 먼저 검증한다.

## 4. 다음 세션 첫 작업

정본 workflow: `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

1. **PM-1A — `shellcore` core purity audit**
   - `src/shellcore/*`에서 desktop-only 의존성을 조사한다.
   - pure evaluator와 외부 process execution을 분리할 위치를 정한다.
   - `external::run`을 trait-backed adapter로 바꿀지, desktop runner로 feature-gate할지 결정한다.

2. **PM-1B — Platform execution contract**
   - command resolution, argv quoting, cwd/workspace, env policy, stdout/stderr stream, exit code, capability flags를 문서화한다.
   - Windows, Android, iOS, PWA가 각각 어떤 capability를 구현하는지 표로 만든다.

3. **PM-1C — Shared smoke tests**
   - pure `shellcore` 테스트와 `ash` smoke를 분리한다.
   - 기준 smoke:
     ```bash
     printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash
     ```
   - 기대: `size 200` 행만 출력.

이 세 작업이 끝나면 PM-2 Windows native `ash.exe` adapter/ConPTY/CI smoke로 넘어간다.

## 5. 재개 명령

```powershell
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:/workspace/terminal-project/terminal status --short --branch
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:/workspace/terminal-project/terminal log -3 --oneline
rg -n "PM-1|Platform execution contract|Android|iOS|PWA companion" D:\workspace\terminal-project\terminal\docs
```

WSL cargo 검증 표준형:

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```

주의:

- Windows sandbox 사용자 때문에 git 명령에 `-c safe.directory=D:/workspace/terminal-project/terminal`가 필요할 수 있다.
- WSL `bash -lc`에 멀티라인 문자열을 넣지 않는다.
- `git add -A` 대신 의도한 파일만 stage한다.
- 상위 workspace의 `document` repo에는 `.omc/` untracked가 남아 있을 수 있다. 이번 커밋 범위가 아니다.
