# 플랫폼 목표 매트릭스 — 독립 셸 + 모바일 로컬 터미널 피벗

> **작성일**: 2026-06-23
> **결정**: 제품 정체성은 "기존 bash/zsh를 감싼 AI 보조 도구"가 아니라 **모든 지원 플랫폼에서 돌아가는 독립 로컬 터미널**이다. 모바일도 단순 승인 PWA가 아니라, 장기 목표를 **온디바이스 로컬 터미널**로 전환한다.
> **관계**: `2026-06-05-independent-shell-s0-core-design.md`의 독립 셸 피벗을 플랫폼별 실행 목표로 확장한다. 기존 RA/PWA 설계는 "모바일 제품의 본체"가 아니라 승인/동기화 companion 기능으로 재배치한다.
> **실행 workflow**: `../plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

---

## 1. 정체성

`ash`(가칭)는 플랫폼마다 다른 기존 셸을 래핑하는 얇은 도구가 아니다. 같은 언어 코어(`shellcore`)와 같은 안전/AI 라이브러리(`ai_terminal`)를 공유하는 **로컬 우선 터미널 런타임**이다.

핵심 원칙:

- **로컬 우선**: 사용자가 명령을 입력하고 결과를 보는 1차 터미널은 해당 기기에서 돈다.
- **공유 코어**: 파서/평가기/구조화 값 모델/AI 안전 게이트는 데스크톱과 모바일이 같은 Rust 코어를 쓴다.
- **플랫폼별 실행 어댑터**: 외부 명령 실행, PTY/ConPTY, 파일시스템, 패키지/유저랜드는 플랫폼별 어댑터로 분리한다.
- **원격 승인/PWA는 보조 기능**: 폰 승인 UI는 계속 가치가 있지만, 모바일의 최종 목표를 대체하지 않는다.

---

## 2. 플랫폼 목표 매트릭스

| 플랫폼 | 티어 | 목표 | 로컬 터미널 의미 | 실행/PTY 전략 | 배포 | 현재 상태 |
|---|---:|---|---|---|---|---|
| Linux desktop/server | P0 | `ash` 1급 로컬 터미널 | POSIX 유저랜드 위에서 직접 외부 명령 실행 | PTY + process group + Linux guardrails | GitHub Release `ai-linux-x86_64`, 이후 패키지 | `shellcore`/`ash` 시작, 기존 `ai` 기능 재사용 가능 |
| WSL | P0 | Windows 사용자의 Linux 로컬 터미널 | WSL distro 내부에서 Linux와 동일한 `ash` 실행 | WSL bash/PTY 검증 경로 유지 | Windows 문서에서 WSL 설치 경로 제공 | 테스트/개발 주 경로 |
| Windows native | P0 | `ash.exe` 1급 로컬 터미널 | Windows PATH/PATHEXT, `.exe/.cmd/.bat/.ps1` 실행 | ConPTY + shell adapter(`cmd`/PowerShell/직접 spawn) | `install.ps1`, GitHub Release `ai-windows-x86_64.exe` | `ai.exe` 배포/스모크 있음, `ash.exe` 제품화는 후속 |
| Git Bash/MSYS on Windows | P1 | Windows 위 POSIX 호환 프로파일 | MSYS path conversion과 POSIX toolchain을 명시적으로 다룸 | `ash.exe` native 모드와 MSYS bridge 모드 분리, bridge는 명시 opt-in | Windows 설치 후 optional profile | profile 계약 정의됨, bridge runner는 후속 |
| PowerShell host | P1 | 설치/호스트 셸 + 외부 실행 bridge | `ash` 문법과 PowerShell 문법을 섞지 않음. PowerShell은 실행 대상/호스트로 취급 | `.ps1` 실행 정책, quoting, exit code adapter | `install.ps1`, profile helper | 설치 스크립트만 있음 |
| Android | P1 | **모바일 로컬 터미널 1차 타깃** | 기기 안에서 `ash` UI + 로컬 파일/프로세스/패키지 환경을 제공 | Rust core + Android UI + 별도 worker process. 첫 slice는 shellcore-only, Termux/bundled userland는 후속 비교 | APK/F-Droid 우선, Play Store는 정책 검토 후 | Rust `MobileShell` pure core boundary 착수 |
| iPadOS/iOS | P2 / research | 제한적 로컬 터미널 또는 교육/샌드박스형 터미널 | App Store 제약 안에서 self-contained shellcore, 파일 컨테이너, 허용된 interpreter 범위만 | 네이티브 프로세스/다운로드 코드/외부 유저랜드 제약 검증 필요 | TestFlight 우선, App Store는 별도 정책 게이트 | 신규 방향, 고위험 |
| Web/PWA | P2 | companion 또는 sandbox demo | 로컬 OS 명령 실행은 목표 아님. 승인/모니터링/학습용 구조화 셸 데모 | WASM shellcore, no native process | 정적 배포 | 기존 PWA 승인 목업 있음 |
| Remote host / SSH | P2 | 로컬 `ash`에서 원격 런타임 연결 | 명령은 원격 host에서 실행되지만 사용자의 터미널 세션은 로컬에 남음 | SSH/k8s/container adapter, context stack | 데스크톱/모바일 공통 기능 | 기존 문서에 원격 executor 개념 있음 |

---

## 3. 모바일 전환

기존 모바일 방향은 "위험 명령을 폰 PWA에서 승인"하는 companion 경험이었다. 새 방향은 다음처럼 재정렬한다.

```text
기존:
  Desktop terminal/daemon -> PWA approve/reject

새 방향:
  Desktop ash       -> local terminal
  Android ash       -> local terminal
  iOS/iPadOS ash    -> constrained local terminal / research
  PWA               -> approval, monitor, pairing, web demo
```

Android는 모바일 로컬 터미널의 1차 타깃이다. Android 공식 런타임은 앱별 Linux process와 별도 worker process 구성을 허용하므로, UI thread와 터미널/명령 실행 worker를 분리하는 구조를 먼저 검증한다.

iOS/iPadOS는 목표에서 제외하지 않는다. 다만 Apple App Review 2.5.2가 앱 bundle/container와 동적 코드 실행에 강한 제약을 두므로, iOS는 "일반 Linux 유저랜드 터미널"로 약속하지 않는다. 먼저 self-contained `shellcore` + 파일 컨테이너 + 교육/개발용 명령 subset으로 성립하는지 검증한다.

---

## 4. 제품 산출물

| 산출물 | 역할 | 플랫폼 |
|---|---|---|
| `ai` | 기존 CLI/도구 명령, release continuity | Linux/WSL/Windows |
| `ash` | 독립 셸 런타임, 최종 제품 코어 | Linux/WSL/Windows/Android/iOS research |
| `ai_terminal` lib | 안전/AI/스토리지/원격승인 공통 라이브러리 | 전체 |
| Mobile app | `ash` UI + local terminal runtime | Android P1, iOS P2 |
| PWA | 승인/페어링/모니터링 companion, web demo | Web/mobile browser |

`ai`와 `ash`는 당분간 병행한다. 새 플랫폼 작업은 `ash`를 중심으로 하고, `ai`는 설치/진단/정책/기존 CLI 호환 역할을 유지한다.

---

## 5. 단계 재정렬

| 단계 | 플랫폼 의미 |
|---|---|
| S0 | 공통 shellcore MVP. 데스크톱/모바일 모두 같은 언어 코어를 쓴다는 증명 |
| S1 | 구조화 데이터 명령. 모바일 좁은 화면에서도 가치가 큰 테이블/레코드 흐름 |
| S2 | 라인에디터/히스토리/config/리다이렉트. 데스크톱과 모바일 UI adapter 분리 시작 |
| S3 | AI/안전 게이트를 `ash` 실행 경로에 결선 |
| S4 | 플랫폼 adapter: Windows shell bridge, Android app, iOS research, remote approval companion |

RA(remote approval)는 S4의 companion 기능으로 이동한다. 모바일 로컬 터미널을 막지 않으며, 오히려 모바일 `ash`와 데스크톱 `ash` 사이의 승인/동기화 기능으로 재사용한다.

---

## 6. 명시적 비목표

- 모바일에서 루팅/탈옥을 요구하지 않는다.
- iOS에서 완전한 Linux 배포판 실행을 약속하지 않는다.
- PowerShell 문법 호환 셸을 만들지 않는다. PowerShell은 host/adapter다.
- Git Bash/MSYS를 Windows native와 섞어 암묵 처리하지 않는다. 별도 profile로 두고, MSYS bridge는 명시 opt-in으로만 켠다.
- PWA를 모바일 로컬 터미널의 대체물로 취급하지 않는다.

---

## 7. 다음 작업

1. `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`의 PM-1~PM-6 순서로 구현 slice를 진행한다.
2. `ash` 플랫폼 매트릭스 테스트를 추가한다: Linux/WSL, Windows native, Android spike, iOS research.
3. Windows `ash.exe` 스모크를 릴리즈/CI에 추가한다.
4. Android 스파이크: Rust `shellcore` 호출, terminal UI 입력/출력, worker process, local file access, 외부 명령 전략(Termux 호환 vs bundled userland)을 비교한다.
5. iOS 스파이크: self-contained `shellcore` REPL, 파일 컨테이너, App Review 2.5.2/2.5.4 제약에서 가능한 명령 subset을 검증한다.
6. README와 `docs/TASK.md`는 이 문서를 플랫폼 정본으로 참조한다.

---

## 8. 외부 정책 체크

- Apple App Review 2.5.2: 앱은 self-contained여야 하며 container 밖 read/write와 기능을 바꾸는 코드 다운로드/실행에 제약이 있다. iOS 로컬 터미널은 이 제약 때문에 P2/research로 둔다. https://developer.apple.com/app-store/review/guidelines/#software-requirements
- Android 앱은 기본적으로 앱 process/main thread에서 시작하지만, manifest로 component별 process 분리가 가능하다. Android 터미널은 UI와 명령 실행 worker/process 분리를 스파이크한다. https://developer.android.com/guide/components/processes-and-threads
