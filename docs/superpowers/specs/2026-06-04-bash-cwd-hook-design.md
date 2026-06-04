# WI-3 — bash cwd hook 연동 (설계)

> **작성일**: 2026-06-04 · **정본**: §31.1(셸 hook), §31.10(컨텍스트).
> **계획**: `docs/superpowers/plans/2026-06-04-phase1-usability-gaps.md` WI-3.

## 문제

zsh는 native `chpwd` hook으로 디렉터리 변경 시 `ai __hook chpwd "cwd=$PWD"`를 호출해 세션 cwd·git branch를 갱신한다. **bash는 native `chpwd`가 없어** cwd 변경이 세션에 반영되지 않는다. 현재 bash `precmd` 핸들러(`record_hook_precmd`)는 `exit=`만 처리하고 함께 받은 `cwd=`는 무시한다. 결과: bash에서 `cd`/`git switch` 후 `ai context`/AI 컨텍스트의 cwd·branch가 갱신되지 않는다(§31.10 미충족).

## 설계 결정

### bash hook에서 chpwd를 에뮬레이트한다(셸 측)
- **이유**: `chpwd` 핸들러(`record_hook_chpwd`)는 이미 cwd+git branch 스냅샷을 올바르게 남긴다 → **핸들러 재사용**. precmd마다 DB에서 직전 cwd를 읽어 비교하면 prompt마다 불필요한 디스크 I/O가 발생 → 셸 메모리 변수로 비교하는 편이 가볍고 정확.
- **방법**(`BASH_HOOK`): 셸 변수 `__ai_last_pwd`로 직전 PWD를 보관. `precmd`에서 `$PWD != $__ai_last_pwd`이면 변수 갱신 후 `ai __hook chpwd "cwd=$PWD"` 호출. 초기값은 빈 문자열 → 첫 prompt에서 초기 cwd를 1회 기록(startup 컨텍스트 확보).
- **exit 코드 보존**: precmd 시작에서 `local __ai_ec=$?`로 캡처, chpwd 에뮬레이션은 그 뒤에 수행, 마지막에 `return $__ai_ec`로 원래 종료 코드 반환(가드 `|| true`가 $?를 바꿔도 무해).
- **불변식 유지**(§31.1): 모든 외부 호출은 `command -v ai` 가드 + `>/dev/null 2>&1 || true`로 감싼다 → hook 실패가 셸 사용을 막지 않는다.

### 핸들러는 변경하지 않는다
- `record_hook_chpwd`는 그대로. bash가 zsh와 동일한 `chpwd` 이벤트를 보내므로 양 셸이 단일 경로로 수렴.

## 범위

- **포함**: `BASH_HOOK` chpwd 에뮬레이션 + 단위 테스트(스크립트 내용·문법) + WSL 동작 검증.
- **제외(후속)**: Native Wrapper fallback(WI-4), `subshell`/`pushd` 깊은 추적.

## 수용 기준 (DoD, §31.1/§31.10)

1. `hook_script(Bash)`가 PWD 변화 감지 로직(`__ai_last_pwd` 추적 + `__hook chpwd` 호출)을 포함. (단위 테스트)
2. 생성된 bash hook이 `bash -n` 문법 검사를 통과. (기존 `generated_hooks_pass_syntax_check`, WSL)
3. WSL e2e: bash에서 `cd` 후 `ai context`/세션 cwd가 갱신, hook 실패가 셸을 중단하지 않음.
4. exit 코드 보존(`precmd`가 직전 `$?`를 그대로 반환).
5. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green.
