# Android 외부 명령 전략 비교 — PM-3E

> **작성일**: 2026-06-24
> **범위**: 플랫폼/모바일 로컬 터미널 workflow의 PM-3E.
> **관련 문서**: `2026-06-23-android-local-terminal-spike.md`, `2026-06-23-platform-execution-contract.md`, `2026-06-24-android-stream-cancel-contract.md`.

## 1. 결론

Android MVP는 계속 `shellcore-only`다. 지금 제품 약속은 "Android에서 로컬 `ash` 구조화 셸 코어를 평가한다"이지, "Android에서 완전한 Linux userland를 제공한다"가 아니다.

다음 외부 명령 spike는 **Termux-compatible opt-in bridge**로 진행한다. 단, Termux를 같은 process의 PATH처럼 직접 mount하거나 실행하지 않는다. 별도 앱/런타임과의 명시적 연동으로 취급하고, 사용자가 설치와 권한을 선택한 경우에만 capability를 켠다.

`bundled minimal userland`는 보류한다. 앱이 바이너리 집합을 직접 포함하면 reproducible UX는 좋아지지만, ABI별 패키징, 보안 업데이트, 라이선스, 배포 크기, 정책 리뷰 책임이 바로 제품 책임이 된다.

## 2. 공통 제약

외부 명령 어댑터는 어떤 선택지든 다음 계약을 깨면 안 된다.

- workspace root는 app-private `ash-workspace`가 기본이다.
- user-selected document는 direct mount가 아니라 import/export copy boundary를 지난다.
- `ShellStreamEvent` 이벤트 순서는 UI가 소비하는 정본이다.
- `ShellRunHandle.cancel()`은 adapter가 붙는 순간 실제 interrupt 또는 timeout 의미를 가져야 한다.
- stdout/stderr/exit code/cancel reason은 UI text 렌더링 전에 구조화 이벤트로 남아야 한다.
- secrets/path masking 경계는 Android FFI, logs, future companion boundary를 넘어갈 때 유지되어야 한다.

## 3. 선택지 비교

| 전략 | 설명 | 장점 | 비용/위험 | 판단 |
|---|---|---|---|---|
| `shellcore-only` | Rust `MobileShell`의 pure evaluator와 안전한 builtin만 제공한다. 외부 process spawn은 차단한다. | 현재 구현과 테스트가 작고 명확하다. Android 앱 샌드박스와 workspace 경계가 단순하다. Play/F-Droid 문구를 과장하지 않아도 된다. | `git`, `python`, `grep` 같은 기대 명령은 동작하지 않는다. "터미널" 기대와 실제 기능 차이를 UI에서 정직하게 보여줘야 한다. | MVP 기본값으로 유지 |
| Termux-compatible opt-in bridge | 사용자가 별도로 설치한 Termux 호환 런타임과 명시적으로 연동한다. | 실제 Android userland 생태계를 재사용한다. 패키지 업데이트 책임을 앱이 모두 떠안지 않는다. 고급 사용자는 더 빠르게 실용 명령을 얻는다. | 다른 앱 UID/샌드박스라 직접 PATH처럼 실행할 수 없다. bridge API, intent, 권한, 파일 교환 UX가 필요하다. latency와 stream/cancel 의미가 런타임 경계에 의존한다. | 다음 spike |
| Bundled minimal userland | 앱 APK/AAB가 BusyBox류 또는 curated binaries를 함께 배포한다. | self-contained라 설치 UX가 예측 가능하다. 같은 app UID 아래에서 process control과 workspace guard를 직접 설계할 수 있다. | 4 ABI 패키징, CVE 업데이트, 라이선스 고지, binary provenance, 크기 증가, 정책 리뷰 부담이 생긴다. 명령 subset도 사용자가 기대하는 Linux와 계속 어긋난다. | 보류 |
| Remote/SSH adapter | Android UI는 로컬 terminal이지만 명령은 사용자가 선택한 원격 host에서 실행한다. | 실제 개발 환경과 호환성이 높다. 모바일 기기 userland 한계를 우회한다. | "온디바이스 로컬 터미널"과 다른 제품 약속이다. 네트워크/인증/키 관리/감사 경계가 PM-5 이후 범위다. | PM-3E 범위 밖 |

## 4. Termux-compatible bridge의 경계

Termux-compatible은 direct integration이 아니라 explicit bridge다.

```text
Android ash app
  -> ShellStreamEvent adapter
  -> explicit bridge boundary
  -> user-installed Termux-compatible runtime
```

초기 spike의 목표는 다음이다.

- runtime availability를 감지하고 UI capability를 `core`에서 `external: opt-in`으로 바꿀 수 있는지 확인한다.
- workspace 파일은 import/export 또는 사용자가 명시 선택한 공유 위치를 통해 전달한다.
- bridge가 stdout/stderr chunk, exit code, cancel/timeout을 `ShellStreamEvent`로 변환할 수 있는지 확인한다.
- direct app-private path 접근을 요구하지 않는다.
- bridge 실패는 `external runtime unavailable` 같은 명확한 오류로 표시한다.

초기 spike의 비목표:

- Termux package manager를 앱이 대신 관리하지 않는다.
- Termux private app data를 직접 탐색하거나 실행하지 않는다.
- "모든 POSIX 명령이 된다"는 호환성 문구를 쓰지 않는다.

## 5. Bundled minimal userland 보류 조건

Bundled userland는 다음 조건이 충족될 때만 재검토한다.

- Termux-compatible bridge가 stream/cancel 또는 설치 UX에서 제품적으로 불충분하다는 근거가 있다.
- 포함할 명령 subset이 명확하다. 예: `cat`, `grep`, `sed`, `awk`, `tar`, `git` 중 무엇을 왜 포함할지.
- ABI별 빌드/서명/provenance/CVE update workflow가 CI에 들어간다.
- 라이선스 고지와 소스 제공 의무를 릴리즈 프로세스가 처리한다.
- app-private workspace 밖 접근이 불가능하다는 테스트가 있다.

## 6. 구현 순서

1. MVP 상태는 `shellcore-only`로 유지한다.
2. Android UI에는 external capability를 숨기거나 disabled 상태로 둔다.
3. Termux-compatible bridge design spike를 별도 문서로 연다. PM-3F 결과는 `2026-06-25-termux-compatible-opt-in-bridge-design.md`다.
4. bridge prototype은 먼저 T0 `RUN_COMMAND`로 `echo`, `pwd`, stderr, non-zero exit code final result를 smoke한다.
5. T0가 통과하면 T1 helper로 long-running output, cancel, large output, workspace sharing 모델을 붙인다.
6. 그 뒤에야 bundled minimal userland를 다시 비교한다.

## 7. 수용 기준

- PM-3 문서와 README가 Android MVP를 `shellcore-only`로 계속 설명한다.
- 다음 구현 후보가 Termux-compatible opt-in bridge임을 명시한다.
- bundled minimal userland가 지금 당장 구현 대상이 아닌 이유를 보안/배포/정책/라이선스 관점에서 남긴다.
- future adapter가 `ShellStreamEvent`와 `ShellRunHandle.cancel()` 계약을 구현해야 함을 문서화한다.
