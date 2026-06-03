# 설계: 비-Ran 명령 결과 audit 기록

> 날짜: 2026-06-03 · 핸드오프 백로그 ① · 관련: 그룹 C 중앙 실행 파이프라인, W12 감사

## 문제

`run_exec`/`run_dispatch`(둘 다 `src/main.rs`)는 `ExecOutcome::Ran` 경로에서만 audit를
남긴다(`record_exec` → `record_audit("command_executed", ...)`). 나머지 세 결과는 모두
보안상 의미 있는 사건인데 감사 로그에 흔적이 없다:

- `Blocked` — 정책상 Critical 등급 명령 차단.
- `Declined` — 사용자가 위험 명령 확인을 거부.
- `BackupRefused` — undo 백업이 상한 등으로 거부되어 위험 명령이 중단됨.

현재 이 세 경로는 `eprintln!` 후 `std::process::exit(1)`만 하고 끝난다. 차단/거부된
위험 명령이 무엇이었는지 사후에 추적할 수 없다.

## 범위

- **포함**: CLI `run_exec`·`run_dispatch`의 비-Ran 셸 arm.
- **제외**:
  - TUI(`src/ui.rs`)는 현재 audit를 전혀 기록하지 않는다 → 별도 관심사, 핸드오프 범위 밖.
  - `src/pipeline.rs`는 의도적으로 storage-free(기록은 호출측에서 수행) → 그대로 유지.
  - 기존 `command_executed` payload 형식은 변경하지 않는다.
  - commands 테이블에 비실행 명령을 기록하지 않는다(audit_events만).

## 구조

프로젝트의 "순수 코어 + I/O 주입" 패턴을 따른다. 순수 매퍼를 단위 테스트하고,
얇은 I/O 헬퍼가 storage 기록·stderr 출력·프로세스 종료를 담당한다.

### 1. 순수 매퍼 (`main.rs`, 항상 컴파일·단위 테스트 가능)

```rust
struct AuditRecord {
    event_type: &'static str,
    level: String,
    payload_json: String,
}

/// 비-Ran 결과를 audit 레코드로 변환한다. Ran은 record_exec가 처리하므로 None.
fn shell_outcome_audit(command: &str, source: &str, outcome: &ExecOutcome) -> Option<AuditRecord>
```

- 명령 텍스트는 `mask::Masker::baseline().mask(command).text`로 마스킹 후 payload에 포함.
- payload는 `serde_json`(이미 의존성)으로 안전 구성 → 명령 내 따옴표/역슬래시 이스케이프.
- `event_type` 매핑:
  - `ExecOutcome::Blocked { .. }` → `"command_blocked"`
  - `ExecOutcome::Declined` → `"command_declined"`
  - `ExecOutcome::BackupRefused(_)` → `"command_backup_refused"`
  - `ExecOutcome::Ran { .. }` → `None`
- `level`:
  - `Blocked`는 variant가 들고 있는 `level` 사용.
  - `Declined`/`BackupRefused`는 variant에 level이 없으므로 `risk::assess(command).level` 재산출(결정적·저비용).
- payload extra 필드:
  - 공통: `command`(마스킹), `source`.
  - `Blocked`: `factors`(variant의 factors 문자열 배열).
  - `BackupRefused`: `reason`(variant의 사유 문자열).

### 2. 수렴 I/O 헬퍼 (`main.rs`, 두 호출자 공용)

`run_exec`와 `run_dispatch`의 셸 arm 처리 로직이 동일하므로 공통 헬퍼로 추출한다(이전
리뷰에서 YAGNI로 보류했던 중복 제거 항목이, audit 추가로 중복이 커지면서 정당화됨).

```rust
/// 셸 실행 결과를 마무리한다: audit 기록 + 사용자 안내 + 프로세스 종료(항상 발산).
fn finish_shell_outcome(command: &str, source: &str, outcome: ExecOutcome) -> !
```

- `Ran { exit_code, undo_id }` → (기존대로) undo 안내 + `record_exec(command, exit_code, source)` + `exit(exit_code)`.
- 비-Ran → `shell_outcome_audit`로 얻은 레코드를 storage-gated 경로로 `record_audit`에
  저장한 뒤, 기존 `eprintln!` 메시지(차단됨/취소/백업 거부) 출력 + `exit(1)`.
- 호출:
  - `run_exec`: `finish_shell_outcome(command, "exec", outcome)`
  - `run_dispatch`의 `Handled::Shell(outcome)`: `finish_shell_outcome(input, "dispatch", outcome)`
  - `run_dispatch`의 `Handled::Ai(..)`/`Handled::Empty`는 기존 처리 유지.

storage 미사용 빌드에서는 순수 매퍼는 그대로 동작하고(레코드 생성), 실제 `record_audit`
호출만 `#[cfg(feature = "storage")]`로 게이트되어 드롭된다.

## 페이로드 예시

```json
// event_type=command_blocked, level=Critical
{"command":"rm -rf /","source":"exec","factors":["재귀 삭제 (+30)","루트 경로 (+50)"]}

// event_type=command_declined, level=High
{"command":"sudo systemctl restart nginx","source":"dispatch"}

// event_type=command_backup_refused, level=High
{"command":"rm /home/u/big.bin","source":"exec","reason":"파일 크기 초과(20MB)"}
```

## 테스트

- **단위**(`main.rs` `#[cfg(test)]`):
  - `Ran` → `None`.
  - 각 비-Ran → 올바른 `event_type`·`level`.
  - `Blocked` payload에 `factors`, `BackupRefused` payload에 `reason` 포함.
  - **마스킹 검증**: secret 토큰을 포함한 명령(예: GitHub 토큰)을 넣었을 때 payload에
    원문 secret이 잔존하지 않음.
- **e2e**(WSL, `--features storage`):
  - `ai exec "rm -rf /"` → audit_events에 `command_blocked` 행 1건(마스킹된 command).
  - `ai exec "sudo systemctl restart nginx"`에 `n` 입력 → `command_declined` 행.
  - python3 표준 sqlite3로 audit_events 조회(sudo/apt 불필요).

## 비목표

- `command_executed`(Ran) payload 형식 변경.
- TUI audit 기록.
- commands 테이블에 비실행 명령 기록.
- audit 보존/회전 정책(별도).
