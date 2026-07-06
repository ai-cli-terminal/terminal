# PRODUCT PACKAGING — 역할, 버전, 플랫폼 문구

> 기준: v0.3.4 이후 `terminal/` repo의 독립 `ash` 피벗. 이전
> `../document/` v3.3 문서는 Phase 1 안전/AI 설계의 역사적 기준으로 남기되,
> 제품 정체성과 플랫폼 약속은 `docs/superpowers/specs/`와 이 문서를 우선한다.

## 1. 제품 표면

| 표면 | 역할 | 사용자에게 말할 것 |
|---|---|---|
| `ai-terminal.exe` | Windows GUI 제품 표면 | 더블클릭 가능한 독립 GUI 터미널. 내부에서 bundled `ash.exe`를 PTY/ConPTY로 실행한다. |
| `ash` / `ash.exe` | 독립 구조화 셸 런타임 | 데스크톱과 모바일 트랙이 공유하는 로컬 터미널 코어. 안전 게이트, 라인 에디터, AI 라우팅을 셸 안에 둔다. |
| `ai` / `ai.exe` | 호환 CLI/helper | `doctor`, `exec`, `risk`, `policy`, install/diagnostics, 기존 wrapper 흐름을 유지하는 CLI. GUI 앱도, 독립 셸도 아니다. |
| Android mobile ash app | 모바일 로컬 터미널 본체 | Android 기기 안의 app-private workspace와 Rust `shellcore`/Termux opt-in bridge를 쓰는 로컬 터미널. |
| iOS/iPadOS ash app | TODO/deferred research | macOS/Xcode/iOS project 환경 전까지 구현 대상이 아니다. 약속은 self-contained `shellcore`와 제한적 local workspace뿐이다. |
| PWA companion | 승인/페어링/모니터링/demo | 로컬 터미널 대체물이 아니다. desktop/mobile `ash`를 돕는 companion surface다. |

## 2. 버전 정책

- Repo release version은 `VERSION`과 `Cargo.toml`의 package version이 기준이다.
- 공개 릴리즈의 `ai`, `ash`, Windows GUI, Android artifact는 같은 repo release
  version을 공유한다.
- 플랫폼별 준비 상태는 별도 product version을 만들지 않고 release notes, README,
  `docs/releases/`, `docs/TASK.md`에 명시한다.
- Android는 같은 semver를 `versionName`으로 쓰고, F-Droid/release gate는
  semver-derived `versionCode`를 검증한다.
- iOS/iPadOS는 TestFlight artifact가 생기기 전까지 public release version 약속을
  하지 않는다.

## 3. v3.3에서 terminal pivot으로의 migration note

`../document/` v3.3은 "AI CLI 통합 리눅스 터미널"을 기준으로 안전한 AI 보조,
프라이버시, 정책, preview/undo, 사용량 기록, provider boundary를 정의했다. 이
repo는 그 안전 코어를 보존하면서 제품 표면을 다음처럼 재정렬했다.

| v3.3 framing | terminal pivot |
|---|---|
| `ai` 단일 CLI/터미널 중심 | `ai` CLI/helper + `ash` 독립 셸 + Windows GUI `ai-terminal.exe` 병행 |
| Linux/bash/zsh hook 우선 | Linux/WSL/Windows/Android가 공유하는 독립 local terminal runtime 우선 |
| 모바일은 remote approval companion 중심 | Android는 local terminal 본체, PWA는 approve/pair/monitor/demo companion |
| 완전 Linux terminal 표현 | 플랫폼별 capability를 명시. Android/iOS/PWA는 Linux userland promise를 과장하지 않음 |

충돌이 생기면 다음 순서로 해석한다.

1. 안전/AI 정책과 프라이버시 원칙은 v3.3 설계를 유지한다.
2. 제품 이름, 플랫폼 지원, 모바일/PWA 역할은 `terminal/`의 플랫폼 피벗 문서를
   우선한다.
3. iOS/iPadOS는 self-contained `shellcore` research로만 말하며 Linux terminal,
   package manager, Termux-equivalent userland, downloaded code, arbitrary process
   실행을 약속하지 않는다.

## 4. Mobile/PWA identity and copy separation

사용자 문구:

- Mobile ash app = local terminal.
- PWA companion = approve/pair/monitor/demo.

Identity boundary:

- RA device identity는 approve/pair/monitor companion identity다.
- Android/iOS mobile terminal body identity와 RA device identity를 자동으로
  결합하지 않는다.
- mobile terminal runtime이 stable contract를 갖기 전까지 PWA identity를 모바일
  터미널 본체 계정, workspace, signing identity, local shell identity로 재사용하지
  않는다.
- PWA는 mobile local terminal이 준비되지 않은 플랫폼의 대체 제품으로 설명하지
  않는다.

## 5. 현재 배포와 목표 매트릭스

README의 "현재 배포" 표는 사용자가 지금 받을 수 있는 artifact만 설명한다. README의
"목표 매트릭스" 표는 구현 방향과 연구/deferred 항목을 설명한다. 두 표를 섞지 않는다.
