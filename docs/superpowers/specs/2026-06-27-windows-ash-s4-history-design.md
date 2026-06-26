# S4 — `ash` File-Backed History 설계

> **작성일**: 2026-06-27
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S4).
> **상위**: `2026-06-26-windows-ash-completion-scoping-design.md` (#2 history). 선행: S1(config), S3(line editor, reedline).
> **참고**: reedline `FileBackedHistory`(`/nushell/reedline`), `crate::mask::Masker`.

## 1. 목표

`ash`의 reedline 라인에디터에 **세션 간 영속 history**(파일 저장/로드, ↑↓ 회상)를 추가한다. 다음을 만족한다:

- 기본 경로 파일에 저장/로드, capacity는 config(S1 `history_limit`).
- **민감명령 저장 제외**: 명령 텍스트에 secret/PII가 탐지되면(`mask`) history 파일에 쓰지 않는다.
- **손상 파일 fail-soft 복구**: history 파일 로드 실패 시 메모리 history로 폴백 + 경고(세션 비중단).
- **동시 best-effort append**: 여러 ash 인스턴스 동시 실행은 reedline `FileBackedHistory`의 파일 sync 시맨틱에 맡긴다(커스텀 락 없음).

S3의 in-session 메모리 history를 **파일 영속**으로 격상하는 슬라이스다. SQLite `ai history`(감사 로그, storage feature)와는 무관하다.

## 2. 경계 제약

- 전부 데스크톱 모듈 `src/line_editor.rs`(`cfg(not(target_os="android"))`)에 둔다. reedline/mask/config는 데스크톱 모듈 — 사용 OK.
- `shellcore`·android cdylib 빌드 불변(`cargo check --lib --target aarch64-linux-android` green). `LineReader` 트레이트 시그니처 불변(S3).

## 3. 민감 판정 (순수)

```rust
/// 명령 텍스트에 secret/PII가 탐지되면 true → history 저장 제외.
pub(crate) fn is_sensitive_command(cmd: &str) -> bool {
    !crate::mask::Masker::baseline().mask(cmd).redactions.is_empty()
}
```
- `MaskOutcome.redactions`(발동 규칙 이름들)이 비어있지 않으면 민감. API key/password/token/PII 등.
- 순수 함수 — 단위 테스트 대상.

## 4. `FilteringHistory` — 제외 래퍼

reedline은 제출 라인을 자동으로 history에 추가한다. 민감명령 제외는 reedline `History` 트레이트를 구현한 래퍼가 `save`에서 거른다.

```rust
struct FilteringHistory {
    inner: reedline::FileBackedHistory,
}
impl reedline::History for FilteringHistory {
    // save(item): item의 command_line이 is_sensitive_command이면 저장 건너뛰고
    //   원본 item을 그대로 반환(추가 안 함). 아니면 inner.save(item)에 위임.
    // 그 외 모든 트레이트 메서드(load/count/search/update/delete/clear/sync/session 등)는
    //   inner로 단순 위임한다.
}
```
- **트레이트 표면은 reedline 버전 의존** → 플랜에서 컴파일러 가이드로 정확히 위임(S3 `Prompt` 패턴 동일). `LineReader`/`ReadOutcome`는 바꾸지 않는다.
- `save`의 정확한 시그니처/반환은 해석된 버전에 맞춘다(예: `fn save(&mut self, h: HistoryItem) -> Result<HistoryItem>`). 민감 시 **inner를 호출하지 않고** 입력 item을 그대로 반환(또는 버전이 요구하는 무해한 성공값).

## 5. `ReedlineReader` 구성 변경

S3의 `ReedlineReader::new() -> Result<Self>`를 history 주입형으로:

```rust
impl ReedlineReader {
    /// capacity=0이면 파일 영속 없이 메모리 history만(persistence 비활성).
    pub fn with_history(path: PathBuf, capacity: usize) -> anyhow::Result<Self>;
}
```
동작:
1. `capacity == 0` → reedline 기본(메모리) history로 생성(파일 없음). 반환 Ok.
2. `FileBackedHistory::with_file(capacity, path)`:
   - `Ok(fbh)` → `FilteringHistory { inner: fbh }`로 감싸 `Reedline::create().with_history(Box::new(...))`.
   - `Err(e)` → **경고 출력 후 메모리 history 폴백**(`FileBackedHistory::default()` 또는 무-history). 세션 비중단.

## 6. ash 결선

`src/bin/ash.rs`에서 TTY 분기 시 `ReedlineReader::with_history` 호출:
- **path** = `ai_terminal::config::config_dir()?.join("ash_history")`(config_dir 실패 시 임시/홈 폴백, fail-soft).
- **capacity** = `loaded.config.general.history_limit`(이미 로드됨).
- `with_history` 실패는 기존처럼 `StdinLineReader`로 폴백.

## 7. 에러 처리 (fail-soft 전면)

- history 파일 손상/권한 오류 → 메모리 history + 경고, 세션 지속.
- config_dir 해석 실패 → 합리적 기본 경로 폴백.
- 어떤 history 오류도 `ash`를 종료시키지 않는다.

## 8. 테스트

단위(`src/line_editor.rs`):
- `is_sensitive_command`: secret 포함 명령(예: `export TOKEN=ghp_...`/고엔트로피)→true; 평범한 명령(`ls -al`/`echo hi`)→false.
- `FilteringHistory.save` 필터링: 임시 파일 history에 민감 명령 + 일반 명령을 `save`한 뒤, 재로드(또는 inner count/search)로 **일반 명령만 영속됨**을 확인. (reedline `HistoryItem` 구성은 해석된 버전에 맞춰)
- capacity 0 → 파일 미생성(메모리 경로) 확인 가능 범위.

e2e(WSL): 인터랙티브 raw가 아니므로 직접 스크립트 어려움 — 가능하면 두 번째 ash 세션이 이전 세션의 비민감 명령을 history 파일에서 로드하는지 파일 수준으로 확인(예: history 파일 내용 grep). android 경계 check.

수동(인터랙티브): 실제 터미널에서 명령 실행 후 새 세션에서 ↑로 회상되는지, secret 든 명령은 회상/파일에 없는지 확인.

## 9. 수용 기준

1. ash가 비민감 명령을 `<config_dir>/ash_history`에 저장하고, 새 세션이 ↑↓로 회상한다.
2. secret/PII가 탐지된 명령은 history 파일에 저장되지 않는다.
3. 손상/오류 history 파일은 메모리 폴백 + 경고로 처리(세션 비중단).
4. capacity는 config `history_limit`를 따른다(0이면 영속 비활성).
5. `shellcore`/android cdylib 빌드 불변.
6. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green.

## 10. 비목표

- history 경로 커스터마이즈 config 필드(기본 경로만), dedup, history 검색 UI, 시간窓/TTL, `ai history`(SQLite) 통합, 동시성 커스텀 락(reedline best-effort에 의존).
