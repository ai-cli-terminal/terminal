# FU-2 — 실행형 preview 샌드박스 (tmpdir 백엔드) (설계)

> **작성일**: 2026-06-04 · **정본**: §31.5(preview/diff), §31.11(guardrails/sandbox), §15(tmpdir=MVP).
> **계획**: `docs/superpowers/plans/2026-06-04-phase2-followups.md` FU-2.

## 문제

W9 안전 preview는 **실행 없는** diff(cp/mv 덮어쓰기)·content-at-risk(rm/truncate)만 보여준다. `sed -i`·`perl -i`·포매터(`prettier --write`/`black`/`gofmt -w`/`ruff --fix`) 등 **실행이 필요한 in-place 편집**은 "샌드박스 후속으로 보류" 안내만 한다(§31.5 수용 기준의 diff 미충족).

## 설계 결정

### tmpdir 백엔드(§15: MVP) — 원본 미수정
- **흐름**: 대상 파일을 임시 디렉터리로 복사 → 명령의 대상 경로 토큰을 임시본 경로로 치환 → 임시 cwd에서 실행 → 임시본(편집 후) vs 원본 diff(`diff::unified_diff`) → 임시 정리. **원본 파일은 절대 수정하지 않는다.**
- bubblewrap/gVisor/container 격리는 후속(§31.11) — MVP는 tmpdir + 대상 한정 실행.
- 실행 대상은 `is_in_place_edit`로 이미 분류된 **알려진 in-place 편집 집합**으로 한정(임의 명령 실행 아님). cwd=tempdir로 부수효과 면을 줄인다.

### 순수/부수효과 분리(테스트 가능)
- `sandbox::in_place_targets(command) -> Vec<String>`: `cmdparse::args_after_program`로 인자만 추려 **기존 일반 파일**만 대상(플래그/스크립트/`=` 제외).
- `sandbox::rewrite_path(command, from, to) -> String`: 명령 문자열에서 대상 경로 토큰을 임시 경로로 치환(토큰 단위, 순수).
- `sandbox::preview_in_place(command) -> Result<Vec<PreviewRender>>`: 위 둘 + 복사·실행·diff 오케스트레이션(부수효과, WSL 검증).

### 안전장치
- 대상별 크기 상한(기존 `MAX_DIFF_BYTES`/cells 재사용)으로 거대 파일 회피. 실행 타임아웃(기존 PTY 경로). 임시 정리는 RAII/`Drop` 또는 명시.
- 대상이 없거나 모호하면 기존 "보류" 안내로 강등(fail-safe).

## 범위

- **포함**: `sandbox` 모듈(in_place_targets/rewrite_path/preview_in_place) + `render_temp_copy`의 in-place 분기를 실제 diff로 + `ai preview` 출력 연결 + 테스트.
- **제외(후속)**: bubblewrap/gVisor/container 격리, 다중 파일 원자성, 비결정 명령, 네트워크 차단.

## 수용 기준 (DoD, §31.5)

1. `in_place_targets`: `sed -i 's/a/b/' f.txt`(기존 파일)에서 `f.txt`를 대상으로, 플래그/스크립트/`=`는 제외. (단위, temp 파일)
2. `rewrite_path`: 명령의 대상 토큰만 임시 경로로 치환(다른 토큰 불변). (단위)
3. `preview_in_place`: `sed -i`가 원본 미수정 + 실제 unified diff 생성(WSL). 대상 없음/모호 시 보류 안내.
4. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green. WSL e2e(원본 불변·diff 정확).
