# FU-4 / M0 — 원격 승인 셸 인터셉트 제어점 (검증 기반 설계)

> **작성일**: 2026-06-04 · **정본**: `../document/planning/builds/remote-approval/{DESIGN,CEO-PLAN,TEST-PLAN}.md`, §30-1(Native Wrapper), §30-13(원격 승인 위험도 경계), §31.4(위험도), §31.1(셸 hook).
> **상위 빌드**: "Trustworthy Remote Approval" — D(컨텍스트 정확도)가 C(폰 승인)를 신뢰 가능하게 만든다.
> **이 문서 범위: M0(인터셉트 제어점 증명) + 그 in-repo 착지.** 크립토·데몬 네트워킹·PWA·원격 왕복은 **M1+ 후속**.

## 왜 M0가 먼저인가

전 리뷰 체인(office-hours→Codex→eng→CEO→cross-model)의 만장일치 결론: **"인터셉트 제어점이 최대 feasibility 위험"**이고 **"크립토 전에 먼저 증명하라"**. Codex 지적: **"preexec 반환값으로 차단은 비이식적"** — 즉 zsh `preexec`/bash 단순 hook은 명령을 *관찰*만 할 뿐 *차단*하지 못한다. 차단 가능한 제어점이 셸별로 실재하는지가 빌드 전체의 전제다.

## 검증 결과 (2026-06-04, WSL spike — 추정 아님)

크립토 없이 로컬에서 "armed 상태에서 위험 명령을 **실행 전** 가로채 데몬에 묻고 결과로 실행/차단"을 bash·zsh 양쪽에서 실증했다. (스파이크: PTY 대화형 하니스 + Unix 소켓 mock 데몬)

| 항목 | 메커니즘 | 결과 |
|---|---|---|
| bash 대화형 차단 | `shopt -s extdebug` + `DEBUG` trap, trap이 **비0 반환 시 명령 실행 취소** | ✅ `rm -rf` 차단(대상 디렉터리 생존), 안전 명령 실행, 셸 생존 |
| zsh 대화형 차단 | `preexec`(불가) 대신 **ZLE `accept-line` 위젯** 오버라이드로 `$BUFFER` 검사 후 비움 | ✅ 동일 |
| 전체 게이트 루프 | hook → **Unix 소켓** → 데몬 → `ALLOW`/`BLOCK` 회신 → 차단/실행 | ✅ BLOCK_WORKS |
| IPC 왕복 지연 | Unix 소켓 connect+send+recv (순수 IPC, 프로세스 시작 제외) | mean **0.117ms** / p95 0.18ms |
| 비-armed hot-path 오버헤드 | trap 설치·`AI_ARMED` 미설정 시 즉시 return | **0.02ms/fire** (목표 ≤10ms) |
| 저하 경로 | 데몬 도달 불가 시 게이트 클라이언트 **exit 1 = fail-closed(차단)** | ✅ |

**핵심 비대칭(설계 반영 필수)**:
- **bash**: `DEBUG` trap이 보는 단위는 `$BASH_COMMAND` = **simple command 단위**. 파이프라인·`a && b`·서브셸·명령치환은 구성요소별로 trap이 여러 번 발화한다 → 게이트가 "한 줄"이 아니라 "조각"을 본다. (보수적 분류엔 유리하나, 표시·승인 단위는 라인이 자연스러움)
- **zsh**: ZLE 위젯은 **입력 라인 전체(`$BUFFER`)**를 본다 → 승인 단위가 깔끔. 대신 비대화형/스크립트 경로는 ZLE 미적용(인터셉트는 대화형 전용).
- 결론: **인터셉트는 대화형 셸 전용 가드레일**이며(위협모델과 일치: "자기 자신용 가드레일"), 비대화형/직접 binary 실행은 본질적으로 우회 가능(DESIGN Threat Model).

## 설계 결정 (M0 in-repo 착지)

기존 `src/shell.rs`는 이미 bash/zsh hook(preexec/precmd/chpwd)을 생성·설치(마커 래핑·idempotent·`command -v ai` 가드)한다. M0는 여기에 **인터셉트(차단) 변형**을 추가한다.

### 1. 인터셉트 hook 생성 (`shell.rs` 확장)
- **bash**: `shopt -s extdebug` + `DEBUG` trap 함수. armed(`AI_TERMINAL_ARMED` env 또는 상태파일) 일 때만, 내부 함수·`PROMPT_COMMAND`를 제외하고 `$BASH_COMMAND`를 게이트에 전달. 게이트가 차단이면 trap `return 1`.
- **zsh**: `ai-accept-line` 위젯 + `zle -N accept-line`. armed 일 때 `$BUFFER`를 게이트에 전달, 차단이면 `BUFFER=""` 후 `.accept-line`(아무것도 실행 안 함) + 사유 표시.
- 양쪽 모두 **기존 hook 마커 블록과 별개 마커**로 설치(독립 on/off), `bash -n`/`zsh -n` 문법 검증.

### 2. 게이트 진입점 (`ai __gate "<cmd>"`)
- 신규 내부 서브커맨드. **M0(로컬 결정)**: armed가 아니면 즉시 exit 0(통과). armed면 기존 `risk.rs`로 분류 → 정책(§30-13 경계)으로 allow/block 결정 → **exit code로 hook에 회신**(0=allow, 비0=block). 표준 출력에 사유.
- **M1(원격)**에서 이 자리가 "데몬에 블로킹 질의 → 폰 승인 왕복"으로 교체된다. M0는 그 **인터페이스(명령→exit code)와 fail-closed 계약**을 고정한다.
- 성능: armed 게이트만 중작업, 비-armed는 hook이 게이트를 **호출조차 안 함**(검증된 0.02ms hot-path).

### 3. armed 상태 (`ai remote arm` / `disarm`)
- M0 최소: 상태파일(예: `$XDG_RUNTIME_DIR/ai-terminal/armed` 또는 data_dir) 생성/삭제. hook은 이 존재 여부로 분기.
- TTL(`--for 30m|2h`, 캡 ≤4h, monotonic, 만료=fail-closed)은 **CEO 확장 스코프(M1-after-floor)** → M0는 단순 arm/disarm만.

### 4. 위험도 경계 (§30-13, M0부터 강제)
- 로컬 게이트 결정도 §30-13 정본을 따른다: 기본 **Medium까지 허용**, High는 opt-in 오버라이드, **Critical은 절대 차단**(로컬 안내). M1 원격 왕복도 동일 게이트를 공유(shared-core).

## 범위

- **포함(M0)**: `shell.rs` 인터셉트 hook(bash extdebug / zsh ZLE) 생성·설치·문법검증, `ai __gate`(로컬 risk 기반 allow/block + exit code 계약 + fail-closed), `ai remote arm/disarm`(상태파일), §30-13 경계 강제, 단위테스트 + WSL 대화형 e2e(차단/통과/비-armed 무개입).
- **제외(M1+)**: 데몬(tokio) 프로세스·Unix 소켓 서버, 크립토(Noise/X25519/AEAD/Ed25519), 페어링/QR, PWA, 원격 왕복, TTL/heartbeat/viz(#1·#2·#4), context_hash TOCTOU, replay/nonce, revoke. (spike에서 소켓 왕복·fail-closed는 *메커니즘만* 증명했고, 실데몬·E2E는 M1.)

## 위협 모델 (DESIGN 승계 — 제품 약속 레벨)
"자기 자신용 가드레일"이며 악성/탈취 로컬 유저 경계가 **아니다**. hook 비활성·다른 셸·직접 binary·PATH 변조·데몬 kill로 우회 가능(그래서 데몬-다운=일반 셸 정상이 허용). 인터셉트는 **대화형 셸 전용**(스파이크로 확인). README/문서에 "advisory, best-effort" 명시(CEO #8).

## 수용 기준 (DoD)

1. `ai init shell`(또는 신규 플래그)이 **인터셉트 hook**을 설치, `bash -n`/`zsh -n` 통과, 마커 idempotent·`--uninstall` 라운드트립. (단위)
2. **bash 대화형**: armed + `rm -rf <dir>` → 차단(대상 생존), 안전 명령 실행, 셸 생존. (WSL e2e)
3. **zsh 대화형**: 동일. (WSL e2e)
4. **비-armed**: hook이 게이트를 호출하지 않고 일반 셸과 동일 동작(오버헤드 무시 가능). (WSL e2e + 마이크로벤치)
5. `ai __gate`: Critical=항상 차단, Medium=허용, High=기본 차단·opt-in 허용(§30-13), 데몬/오류 시 fail-closed(차단). (단위)
6. default·`--features storage` 빌드 fmt/clippy(-D warnings)/test green.

## 검증된 spike 산출물 (참고)
PTY 대화형 하니스·extdebug/ZLE hook·Unix 소켓 mock 데몬·지연/오버헤드 측정 스크립트로 위 표를 산출(세션 임시 디렉터리, 레포 미포함). M1 데몬 설계 시 프로토콜 primitive는 DESIGN M0.5 확정(Noise_XX + X25519 + ChaCha20-Poly1305 + Ed25519).
