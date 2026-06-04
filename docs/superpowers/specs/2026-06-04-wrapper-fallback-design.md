# WI-4 — Native Wrapper fallback (설계)

> **작성일**: 2026-06-04 · **정본**: §30-1/§29.1(Hook 기본 + Wrapper fallback), §31.1.
> **계획**: `docs/superpowers/plans/2026-06-04-phase1-usability-gaps.md` WI-4.

## 문제

§30-1 확정: **Hook 기반 기본 + Native Wrapper fallback**. hook 미설치/비호환 셸에서는 컨텍스트(명령·cwd)가 전혀 수집되지 않는다. 현재 hook 경로(`__hook preexec/precmd/chpwd`)만 `commands`/세션 cwd를 기록하고, wrapper 경로(`ai exec`/`ai dispatch`)는 audit만 남길 뿐 명령을 `commands`에 기록하지 않는다. hook이 없는 환경에서 AI 컨텍스트가 비게 된다.

## 범위 결정 (over-design 방지)

§30-1의 Native Wrapper는 "자체 PTY에서 셸을 직접 띄우고 명령을 intercept, probe로 동기화"를 포함하나, **영속 인터랙티브 셸 런처(PTY 프롬프트 파싱)는 무겁고 기존 `ai tui`/PTY 런타임 및 WI-5와 중복**이다. 따라서 **Phase 2로 명시 이연**한다.

WI-4(MVP)는 fallback의 **핵심 가치 = hook 없이도 컨텍스트 수집**에 집중한 수직 슬라이스:

1. **통합 모드 해석**(`shell.rs`, 순수): `ConfiguredMode { Hook, Wrapper, Auto }` + `IntegrationMode { Hook, Wrapper }` + `resolve_integration_mode(configured, hook_active) -> IntegrationMode`. `Auto` → hook_active면 Hook, 아니면 Wrapper(fallback).
2. **hook 활성 감지**(env 마커): hook 스크립트가 `export AI_TERMINAL_HOOK=1`을 설정. `shell::hook_active(env_get) -> bool`이 이 마커를 확인(DI로 테스트 가능). 마커 부재 = hook 미작동 = wrapper 모드. (toml 설정 로더는 신설하지 않음 — YAGNI; 마커가 곧 트리거.)
3. **wrapper 모드 컨텍스트 수집(기존 경로 활용)**: 조사 결과 `ai exec`/`ai dispatch`의 `Ran` 경로는 이미 `record_exec`로 명령 + cwd + exit를 `commands`에 기록한다(`source="exec"/"dispatch"`, 위험도 동반). 즉 **wrapper 모드의 데이터 수집은 이미 기능**한다 → 별도 wrapper 기록을 추가하면 중복(over-design)이므로 추가하지 않는다. WI-4는 이 경로가 fallback임을 *인지·표시*하는 데 집중한다.
4. **가시성**: `ai doctor`가 유효 통합 모드(Hook 활성 / Wrapper fallback)를 표시하고, wrapper일 때 `ai exec` 사용·hook 설치를 안내.

## 설계 결정

- **중복 회피(YAGNI)**: `record_exec`가 이미 실행 명령을 기록하므로 wrapper 전용 기록 경로를 신설하지 않는다. WI-4는 모드 감지·해석·표시만 추가한다.
- **감지는 env 마커로**: hook이 `AI_TERMINAL_HOOK=1`을 export → 현재 셸에 마커가 있으면 hook 작동 중. 별도 hook-health 심층 검증(활성 셸 수·마지막 인터셉트 등)은 T-RA5로 이연.

## 범위

- **포함**: 모드 해석·hook 활성 감지·hook 마커·wrapper 컨텍스트 기록·doctor 표시 + 테스트.
- **제외(Phase 2)**: 영속 PTY 셸 런처(프롬프트 파싱·probe 동기화), toml `[shell_integration]` 로더, fish/sh.

## 수용 기준 (DoD, §30-1/§31.1)

1. `resolve_integration_mode`: Hook→Hook, Wrapper→Wrapper, Auto+active→Hook, Auto+!active→Wrapper. (단위)
2. `hook_active`: `AI_TERMINAL_HOOK=1` 있으면 true, 없으면 false. (단위)
3. hook 스크립트(bash/zsh)가 `AI_TERMINAL_HOOK` 마커를 export하고 `bash -n`/`zsh -n` 통과. (단위/WSL)
4. `ai exec`의 `Ran` 경로가 명령 + cwd를 `commands`에 기록(기존 `record_exec`, wrapper 데이터 수집 충족). (기존 동작 확인)
5. `ai doctor`가 유효 통합 모드를 표시하고 wrapper일 때 안내. (수동 확인)
6. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green.
