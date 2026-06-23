# 중앙 실행 파이프라인 설계

> 작성일: 2026-06-03 · 그룹 C 키스톤 (W10/W11/W2 공통 토대)
> 정본 참조: `../document/docs/06-mvp-implementation-spec.md` §31.4(위험도)·§31.5(preview)·§31.6(undo)·§31.7(usage), §16.2(정상 복구), §5(실행 계층).

## 1. 배경 / 문제

명령 실행이 게이트 없이 분산되어 있고, 준비된 보안 조각들이 서로 연결되어 있지 않다.

- `ui.rs` TUI Enter → `pty::run_in_pty` → 출력 표시. **위험도·정책·preview·백업·usage 게이트를 전부 우회**한다.
- CLI에는 셸 명령 실행 커맨드 자체가 없다(`risk`/`preview`/`undo`는 개별 점검용).
- 독립적으로 존재하는 조각: `dispatch::dispatch`(위험도+정책 결정 산출), `preview::classify_preview`, `undo::create_backup`(상한 초과 시 `Refused`), `pty::run_in_pty`(동기 실행), `store`(record_command/usage/audit).

키스톤은 이 조각들을 하나의 오케스트레이터로 묶어, W10(undo 자동 백업 트리거)·W11(usage 기록)·W2(출력 스트리밍)의 공통 토대를 만든다.

## 2. 범위 (이번 증분)

**포함**: 게이트 배선(위험도→정책→preview→백업→실행→기록) + Executor/Confirmer/OutputSink 심 + `ai exec` CLI + TUI 재배선.

**제외(후속)**: W2 실제 async 스트리밍/backpressure(Executor impl 교체로 슬롯인), W9 실제 temp-copy diff 생성, Shell/Ai 단일 dispatcher 완전 통합, chmod/chown 권한 롤백.

## 3. 모듈 경계 & 심(seam)

순수 코어 + 얇은 CLI 패턴에 맞춰 I/O를 트레이트로 주입 → Windows에서 mock으로 단위 테스트 가능.

```rust
/// 실행 추상화 (W2 스트리밍 심). 지금은 동기 PtyExecutor, 후속에 StreamingExecutor.
pub trait Executor {
    /// 명령을 실행하고 출력을 sink로 흘려보낸 뒤 종료코드를 반환한다.
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32>;
}

/// 확인 게이트 주입. CLI=stdin/--yes, TUI=모달, 테스트=스크립트.
pub trait Confirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool;
}

/// 출력 싱크 (W2 스트리밍 심). CLI=stdout, TUI=append_output, 테스트=수집.
pub trait OutputSink {
    fn write(&mut self, chunk: &str);
}
```

- `PtyExecutor` = `run_in_pty` 래핑. 동기: 끝까지 읽어 한 번에 `sink.write` 호출. 트레이트 모양이 청크 스트리밍을 그대로 수용 → W2 후속은 이 impl만 교체.
- `MockExecutor` = 호출된 명령 기록 + 지정 출력/종료코드 반환(Windows 단위 테스트용, PTY 불필요).

### ConfirmRequest

확인 프롬프트에 노출할 정보(감사/설명용, RULES §2):

```rust
pub struct ConfirmRequest {
    pub command: String,
    pub level: RiskLevel,
    pub decision: Decision,      // Confirm | StrongConfirm
    pub factors: Vec<String>,    // 위험 요인 분해
    pub preview: PreviewPlan,    // 전략/대상 목록 (이번 증분은 표시까지)
    pub backup_files: Vec<String>, // 백업 예정 파일(있으면)
}
```

## 4. 파이프라인 단계 (순서 + 중단 규칙)

```rust
pub fn execute(
    command: &str,
    profile: &PolicyProfile,
    store: Option<&Store>,           // storage feature 시
    executor: &dyn Executor,
    confirmer: &mut dyn Confirmer,
    sink: &mut dyn OutputSink,
) -> ExecOutcome
```

1. **위험도** `risk::assess(command)` → **정책** `profile.decide(level)`.
2. **정책 게이트**:
   - `Block` → `ExecOutcome::Blocked { level, factors }` 반환, **실행 안 함**.
   - `Confirm` / `StrongConfirm` → 3·4단계 정보를 모아 `ConfirmRequest` 구성 → `confirmer.confirm()`. 거부 시 `ExecOutcome::Declined`.
   - `Allow` → 통과.
3. **미리보기** `preview::classify_preview(command)` → `ConfirmRequest`에 첨부. 이번 증분은 **전략/대상 목록 표시까지**(실제 diff 생성은 W9 후속 유지).
4. **실행 취소 백업 (W10 자동 트리거)**:
   - 트리거 조건: 명령이 **삭제**(rm/unlink/shred) 또는 **덮어쓰기/in-place 편집**(sed -i, `>`, cp/mv/tee/touch)이고, 대상 중 **기존 일반 파일**이 존재.
   - `undo::create_backup(undo_dir, files, UndoLimits::defaults())`:
     - `Created(id)` → `undo_id`를 결과에 보존.
     - `Refused(reason)` → `ExecOutcome::BackupRefused(reason)` 반환, **실행 중단**(§31.6 완료 기준 "백업 실패 시 위험 명령 중단").
   - chmod/chown 등 **권한 변경**은 undo가 내용만 백업하므로 **백업 생략 + 한계 고지**(조용한 미지원 금지, 가드레일 원칙).
5. **실행 (W2)**: `executor.run(command, sink)` → 종료코드. 출력은 sink로 흐른다.
6. **사후 기록 (W11/W12)**: storage 시 `store.record_command`(명령+위험도+종료코드) + `store.record_audit`(실행 이벤트). → `ExecOutcome::Ran { exit_code, undo_id }`.

### W11 위치 (명시)

셸 명령은 토큰 비용이 없다. W11 자동 usage는 이미 AI 경로(`gateway`, storage feature 시 자동 기록)에 존재하며, 파이프라인은 같은 `store`를 공유한다. 이번 증분의 W11/W12 기여는 **셸 실행을 store에 기록(명령+종료코드+audit)**하는 부분이다. 두 경로(Shell/Ai)를 단일 dispatcher 아래로 완전 통합하는 것은 얇은 후속 작업.

## 5. 결과 타입

```rust
pub enum ExecOutcome {
    Blocked { level: RiskLevel, factors: Vec<String> },
    Declined,
    BackupRefused(String),
    Ran { exit_code: i32, undo_id: Option<String> },
}
```

## 6. 진입 표면

- **CLI** `ai exec "<command>" [--yes] [--profile <p>]`:
  - 기본: 위험 명령(Confirm/StrongConfirm)에 stdin `y/N` 프롬프트(기본 No).
  - `--yes`: 자동 확인(AutoYes). `Block`은 어떤 플래그로도 우회 불가.
  - 종료코드 전파(`Ran`의 exit_code, 그 외는 비-0 + 사유 메시지).
- **TUI 재배선** (`ui.rs`): Enter → `run_in_pty` 직접 호출 제거 → `pipeline::execute`로 교체. Confirmer=TUI 모달(이번 증분은 간단한 inline 확인), Sink=`append_output`. 위험 명령이 TUI에서도 게이트를 거친다.

## 7. 테스트 전략 (TDD)

코어(전부 Windows에서 실행 가능, MockExecutor + 스크립트 Confirmer):
- Critical 명령 → `Blocked`, executor 미호출.
- High 명령 → 확인 거부 시 `Declined`(미실행) / 승인 시 `Ran`.
- 삭제 명령 → 백업 생성 후 실행(`undo_id` 존재).
- 백업 `Refused`(상한 0) → `BackupRefused`, executor 미호출.
- 종료코드 전파.
- `Allow` 명령 → 확인 없이 `Ran`.

WSL e2e:
- `ai exec "rm <file>"` → 백업 → `ai undo last` 복구 라운드트립.
- `ai exec "echo ..."` 종료코드/출력.

## 8. 부수 결정

- **master→main 정렬**: 기본 브랜치를 main으로 통일. CI(`push: [main]`)가 정상 발동하고 스펙/CONTRIBUTING과 일치. 검증/git 단계에서 처리.
- **WSL `/home/deepe` 홈 소실**: 주요 설정 전부 소실, 복구 불가로 확정. 미해결 결정에서 제거.

## 9. 영향 파일

- 신규: `src/pipeline.rs`.
- 수정: `src/main.rs`(`ai exec` 커맨드 + 핸들러), `src/ui.rs`(Enter 경로 재배선), `src/lib.rs`(모듈 등록).
- 재사용(변경 없음): `risk`, `policy`, `preview`, `undo`, `pty`, `store`.
