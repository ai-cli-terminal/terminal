# Termux-compatible opt-in bridge design spike — PM-3F

> **작성일**: 2026-06-25
> **범위**: Android 외부 명령 adapter 후보인 Termux-compatible opt-in bridge.
> **관련 문서**: `2026-06-24-android-external-command-strategy.md`, `2026-06-24-android-stream-cancel-contract.md`, `2026-06-23-platform-execution-contract.md`, `../plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

## 1. 결론

Termux-compatible bridge는 채택 가능한 다음 spike지만, Android MVP 기본값은 계속 `shellcore-only`다.

초기 구현은 두 단계로 나눈다.

| 단계 | 목적 | 판단 |
|---|---|---|
| T0 `run-command-probe` | Termux 설치, permission, `allow-external-apps`, final stdout/stderr/exit code를 확인한다. | 먼저 구현한다. `ShellStreamEvent`에는 completion 후 chunk로만 변환한다. |
| T1 `ash-termux-helper` | incremental stdout/stderr, cancel token, workspace staging을 제공하는 작은 Termux-side helper를 둔다. | T0 smoke 후 구현한다. 실제 userland adapter는 이 단계부터 켠다. |

이유는 Termux의 documented `RUN_COMMAND` intent가 third-party app에서 Termux context command를 실행하고 Java caller로 result를 돌려줄 수 있지만, 기본 결과 전달은 완료 후 bundle/result 형태다. 따라서 `ShellRunHandle.cancel()`과 incremental stream을 제품 계약으로 내세우려면 앱 쪽 intent 호출만으로 충분하지 않고, bridge helper가 job id, event log, cancel token을 관리해야 한다.

## 2. 비목표

- Termux private app data를 직접 탐색하거나 mount하지 않는다.
- Termux package manager를 `ash` 앱이 대신 실행하거나 업데이트하지 않는다.
- 앱의 PATH에 Termux binary를 섞지 않는다.
- "Android에서 모든 POSIX 명령이 된다"는 약속을 하지 않는다.
- Play Store 호환성을 Termux-compatible bridge의 전제 조건으로 두지 않는다. 배포 채널별 문구와 동작은 별도 정책 검토가 필요하다.

## 3. Opt-in UX와 capability

기본 상태:

```text
capability = core / private
external commands = disabled
```

사용자가 bridge를 켤 때만 다음 상태로 이동한다.

| 상태 | 감지/조건 | UI 표시 |
|---|---|---|
| `unavailable` | Termux-compatible package를 찾지 못함 | `external runtime unavailable` |
| `installed` | package는 있으나 permission 또는 property 미설정 | setup action 표시 |
| `authorized` | `com.termux.permission.RUN_COMMAND` permission과 Termux `allow-external-apps=true` 준비 | probe 실행 가능 |
| `ready` | T0 smoke 통과, optional T1 helper handshake 통과 | `external / opt-in` |

Android `targetSdkVersion >= 30`에서는 package visibility 선언이 필요하다. 따라서 구현 slice는 manifest에 Termux package query와 `com.termux.permission.RUN_COMMAND` permission을 명시하고, permission이 없거나 사용자가 거부한 경우에도 앱은 `shellcore-only`로 정상 동작해야 한다.

## 4. Adapter 경계

Kotlin 쪽에는 complete-result `ShellBridge`와 stream wrapper가 이미 있다. Termux bridge는 `NativeShellBridge`를 바꾸지 않고 외부 명령 전용 adapter로 붙인다.

```text
TerminalViewModel
  -> ShellWorker.submitStreaming
  -> AndroidShellAdapter
     -> NativeShellBridge for shellcore-only evaluation
     -> TermuxBridgeAdapter for opted-in external command
        -> RUN_COMMAND intent / helper protocol
        -> ShellStreamEvent
```

Rust `MobileShell`은 계속 `Engine::pure()`다. 첫 구현은 Rust `ExternalRunner`를 Android에서 바로 교체하지 않는다. Kotlin worker가 "pure shellcore failure가 external execution disabled이고 bridge가 ready일 때" 외부 command path로 재시도하는 형태로 시작한다. 이 방식은 기존 JNI 안전 경계를 흔들지 않으며, bridge 실패 시 pure result를 그대로 설명할 수 있다.

후속 정리 단계에서만 Rust `ExternalRunner` trait에 Android adapter를 연결한다.

## 5. T0 `run-command-probe`

T0는 Termux의 documented `RUN_COMMAND` intent만 사용한다.

필수 입력:

| 항목 | 값 |
|---|---|
| action | `com.termux.RUN_COMMAND` |
| service | Termux `RunCommandService` |
| command path | Termux prefix의 `sh` 또는 `env` 절대 경로 |
| background | `true` |
| pending intent | 앱의 result receiver service |

T0 smoke:

| Smoke | Command | 기대 |
|---|---|---|
| availability | `echo ASH_TERMUX_OK` | stdout marker와 exit `0` |
| cwd | `pwd` | Termux home 또는 configured workdir |
| non-zero | `sh -c 'exit 7'` | exit `7` |
| stderr | `sh -c 'echo ERR >&2; exit 2'` | stderr `ERR`, exit `2` |

T0 event 변환:

```text
Started(input, state)
  -> Stdout(final_stdout)?
  -> Stderr(final_stderr)?
  -> Finished(result_with_exit_code)
```

제약:

- PendingIntent 결과는 완료 후 도착하므로 실제 incremental stream이 아니다.
- foreground terminal session transcript는 stdout/stderr 분리가 약하다. T0는 background command만 사용한다.
- Android binder/result 크기 제한 때문에 큰 output은 T0에서 지원하지 않는다.
- cancel은 T0에서 best-effort UI cancel만 제공한다. 이미 시작한 Termux command의 실제 interrupt를 보장하지 않는다.

따라서 T0가 통과해도 `has_pty=false`, `can_spawn=true`, `has_userland=true`, `can_write_workspace=false`, `can_network=bridge-dependent`로만 표시한다.

## 6. T1 `ash-termux-helper`

T1부터 제품 adapter로 볼 수 있다. 사용자가 Termux 안에 작은 helper script를 설치한다.

```text
~/.ash-termux-bridge/
  helper.sh
  jobs/<job-id>/
    request.json
    events.ndjson
    cancel
    exit.json
```

앱은 user-selected shared directory 또는 app export로 `request.json`을 만든다. Termux helper는 request를 읽고 command를 child process group으로 실행한다. helper는 stdout/stderr를 line 또는 byte chunk 단위 NDJSON으로 기록하고, 앱은 해당 event file을 polling 또는 SAF observer로 읽어 `ShellStreamEvent`로 변환한다.

Event schema:

```json
{"seq":1,"type":"started","job_id":"...","pid":12345}
{"seq":2,"type":"stdout","text":"hello\n"}
{"seq":3,"type":"stderr","text":"warn\n"}
{"seq":4,"type":"finished","exit_code":0}
{"seq":5,"type":"cancelled","reason":"user"}
```

Cancel:

1. `ShellRunHandle.cancel()`이 shared job dir에 `cancel` file을 만든다.
2. helper는 `cancel` file을 감지하면 child process group에 `SIGINT`를 보낸다.
3. timeout 뒤에도 종료되지 않으면 `SIGTERM`, 마지막으로 `SIGKILL`을 시도한다.
4. adapter는 helper의 final event를 `Cancelled(state)` 또는 timeout error `Finished`로 변환한다.

T1 capability:

| Capability | 값 |
|---|---|
| `can_spawn` | 예, opt-in |
| `has_pty` | 아니오, T1은 background process stream |
| `has_userland` | 예, Termux package 상태에 따름 |
| `can_write_workspace` | shared staging dir에 한정 |
| `can_network` | Termux command 권한/패키지에 따름. UI에 별도 badge 필요 |

## 7. Workspace와 파일 교환

App-private `ash-workspace`는 Termux가 직접 읽지 못한다고 가정한다. bridge workspace는 명시 staging boundary다.

| 데이터 | 경로 |
|---|---|
| 앱 기본 작업 | app-private `ash-workspace` |
| Termux input file | 사용자가 선택한 shared staging directory로 export |
| Termux output file | shared staging directory에서 import |
| transcript/result | small output은 PendingIntent, large/stream output은 T1 event files |

초기 UX는 자동 동기화가 아니라 명시 복사다.

```text
Import -> app-private workspace
Run external -> selected files exported to bridge staging
Collect -> bridge output imported back to app workspace
```

이 경계는 느리지만 안전하다. Android SAF와 Termux storage permission 상태가 기기별로 다를 수 있으므로, direct shared path를 제품 기본값으로 숨기지 않는다.

## 8. Command 해석

`ash`는 shell string을 합성하지 않는 원칙을 유지한다.

T0에서는 smoke와 compatibility 검증만 하기 때문에 controlled `sh -c` 사용을 허용한다. 사용자 command 실행은 T1 helper에서 argv 배열을 JSON으로 전달한다.

```json
{
  "argv": ["grep", "-n", "needle", "file.txt"],
  "cwd": "bridge://workspace",
  "env": {
    "TERM": "xterm-256color"
  },
  "timeout_ms": 30000
}
```

helper는 PATH lookup을 Termux context에서 수행하되, 앱은 command name, argv, cwd, env allowlist를 audit log에 구조화해서 남긴다. Secret/path masking은 앱에서 event를 렌더링하거나 log에 쓰기 전에 적용한다.

## 9. Failure Mode

| 실패 | 사용자 메시지 | 동작 |
|---|---|---|
| Termux 미설치 | `external runtime unavailable` | shellcore-only 유지 |
| permission 없음 | `Termux bridge permission required` | setup action 제공 |
| `allow-external-apps` false | `Termux external apps disabled` | setup guide 제공 |
| PendingIntent timeout | `external runtime did not respond` | running 상태 해제 |
| output truncated | `external output truncated by bridge` | T1 helper 권장 |
| helper missing | `Termux bridge helper not installed` | T0만 사용 |
| cancel timeout | `external command did not stop after cancel` | timeout final event |

## 10. 구현 순서

1. `TermuxBridgeAdapter` interface와 fake adapter unit test를 추가한다.
2. Android manifest에 package visibility와 `RUN_COMMAND` permission을 추가한다.
3. T0 availability/probe flow를 구현한다.
4. T0 smoke를 JVM fake와 instrumentation manual-gated test로 나눈다.
5. T1 helper protocol 문서와 bootstrap UX를 추가한다.
6. T1 event reader가 `ShellStreamEvent` order와 cancel contract를 만족하게 한다.
7. smoke를 `echo`, `pwd`, long-running stdout, cancel, non-zero exit, stderr, large output으로 확장한다.

## 11. 수용 기준

- Android MVP 문구는 계속 `shellcore-only`다.
- Termux bridge는 사용자가 켠 뒤에만 capability가 활성화된다.
- `RUN_COMMAND` T0는 final result adapter임을 UI와 문서가 숨기지 않는다.
- 실제 incremental stream/cancel은 T1 helper 없이는 ready로 표시하지 않는다.
- app-private workspace와 Termux private data를 직접 연결하지 않는다.
- bridge smoke는 `ShellStreamEvent` 순서와 `ShellRunHandle.cancel()` 계약을 검증한다.

## 12. 외부 참조

- Termux app wiki `RUN_COMMAND Intent`: setup permission, `allow-external-apps`, command/result extras, result size/truncation behavior.
- Termux app README installation section: supported install sources, signature/source mixing caveat, Google Play branch caveat, Android process limitations.
