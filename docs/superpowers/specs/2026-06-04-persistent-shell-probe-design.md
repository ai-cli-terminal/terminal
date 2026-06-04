# FU-3 — 영속 PTY 셸 런처 (probe cwd 동기화, 바운디드 MVP) (설계)

> **작성일**: 2026-06-04 · **정본**: §30-1(Native Wrapper), §7.4(probe 동기화), §5.
> **계획**: `docs/superpowers/plans/2026-06-04-phase2-followups.md` FU-3. **범위: 바운디드 MVP(사용자 확정).**

## 문제

`ai exec`/`ai tui`는 명령마다 **새 PTY**에서 실행 → `cd`·`export` 등 셸 built-in 상태가 다음 명령에 유지되지 않는다. §30-1 Native Wrapper의 핵심은 **영속 셸** + 실행 후 **probe로 built-in 상태(cwd 등) 동기화**(§7.4)다.

## 범위 결정 (바운디드 MVP)

전체 인터셉트(자체 PTY에서 모든 입력을 가로채 분류·게이트·라인 에디팅)는 §30-1도 "구현 부담이 크다"고 명시 → **제외**. 라인 단위 위험도 게이트는 기존 `ai exec`/`ai tui`로 충분. FU-3 MVP는 **영속 세션 + probe cwd 동기화**라는 핵심 메커니즘에 집중한다.

## 설계 결정

### probe 프로토콜 (순수, 테스트 가능)
- **`wrapper::PROBE`**: `\u{1f}`(unit separator) 마커.
- **`probe_command(user_cmd)`**: 사용자 명령 실행 후 `\x1f$PWD\x1f`를 방출하는 셸 명령 생성 → 출력에서 cwd를 파싱·제거.
- **`parse_probe_cwds(output)`**: PTY 출력에서 마커쌍 사이 cwd 값들을 추출(순수). 마커 밖 텍스트는 무시.
- **`strip_probes(output)`**: 표시용으로 마커 구간을 제거한 출력.

### 영속 세션 (`PtySession` 재사용)
- `ai shell`이 `PtySession`으로 bash를 1회 띄우고 **계속 재사용** → `cd`가 다음 명령에 유지된다(영속성). 각 사용자 명령은 `probe_command`로 감싸 실행, 출력에서 probe를 파싱해 **세션 cwd를 동기화**(storage 시 `record_context_snapshot`/`update_session_cwd`).
- 라인 단위 REPL(전체 raw-mode 인터셉트 아님) — TTY 루프는 기존 `ui::run`과 동일하게 단위 테스트 비대상, WSL 스모크.

## 범위

- **포함**: `wrapper` 모듈(probe_command/parse_probe_cwds/strip_probes 순수 + 영속 세션 probe e2e) + `ai shell` 라인 REPL + 테스트.
- **제외(후속)**: 입력 인터셉트·분류·라인 에디팅·raw-mode passthrough, export/alias 등 cwd 외 상태 probe, 프롬프트 파싱.

## 수용 기준 (DoD, §30-1/§7.4)

1. `parse_probe_cwds`: 마커쌍 사이 cwd만 추출, 마커 밖/홀수 마커는 안전 처리. (단위)
2. `strip_probes`: 마커 구간 제거(표시용). (단위)
3. 영속 `PtySession`에서 `cd /tmp` 후 probe가 `/tmp` 보고, 다음 명령에도 cwd 유지(영속성). (WSL e2e)
4. `ai shell`이 영속 셸로 동작하고 cwd 동기화. (WSL 스모크)
5. default·`--features storage` 빌드 fmt/clippy(-D warnings)/test green.
