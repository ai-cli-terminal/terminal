# Phase 3 로드맵 + remote-approval 완주 + 현 상태 릴리즈 — 설계

> **작성일**: 2026-06-05 · 브레인스토밍 산출.
> **정본 근거**: `../../../../document/docs/05-roadmap-enhancements-decisions.md` §25.3·§29·§30, `../../../../document/planning/17_스케줄.md` §4, `../../../../document/planning/builds/remote-approval/`(DESIGN/TEST-PLAN/CEO-PLAN), `../../TODOS.md`(T-RA1~5), `../plans/2026-06-04-phase2-followups.md`(FU-4).
> **상태**: 계획. 구현 착수 시 각 마일스톤은 `writing-plans`로 슬라이스별 계획 문서를 생성한다.

---

## 0. 배경과 범위

Phase 1(MVP+, M1~M4)·Phase 2(P2-1~12 + 후속 FU-1~3)는 착지 완료. Phase 2 후속으로 진행 중이던 **FU-4 remote-approval**은 M0·M0.5·M1 slice 1~4a까지 와 있다(HEAD `6737ece`). Phase 3~4는 설계 정본(§25.3·§30·스케줄 §4)에 **요약 불릿**으로만 존재했다.

본 설계는 다음 3종을 정의한다.

1. **R0 — 현 상태 릴리즈 (v0.2.0)**: 지금 동작하는 기능을 Linux x86_64 + Windows 네이티브에서 설치·실행 가능한 배포물로.
2. **RA — remote-approval 완주**: M1 4b(게이트 결선) → 실데몬 프로세스 + 디바이스 등록 → **PWA(실폰 승인)** 까지. relay(M2)는 제외(완주 후 재평가).
3. **Phase 3 본체 (P3-1~3)**: 조직 정책·트러스트 채널·중앙 감사·MCP 확장·고격리. Phase 4는 요약 유지.

**순서(가치 우선)**: R0 → RA → P3-1 → P3-2 → P3-3. 근거: (1) 동작하는 바이너리를 즉시 배포해 피드백 루프를 연다, (2) 중반까지 온 remote-approval을 마무리해 thesis(컨텍스트 정확도) 차별화를 완성한다, (3) 조직/엔터프라이즈 기능은 이 둘이 안정된 위에 얹는다.

**플랫폼 현실**: Rust는 WSL/Linux 중심으로 검증돼 왔다. feature는 C-free(`default`, `remote`=snow+dalek)와 C 의존(`storage`=SQLite, `tls`=ring)으로 나뉜다. 동적 감시(seccomp/cgroups/eBPF)·gVisor는 Linux 우선이며 Windows 네이티브는 capability matrix로 "미지원"을 명시한다(§30-8, 조용한 실패 금지).

---

## 1. R0 — 현 상태 릴리즈 (v0.2.0)

목표: 지금 동작하는 기능을 **Linux x86_64 + Windows 네이티브**에서 설치해 쓸 수 있는 배포물로. macOS·`.deb`/`.rpm`은 범위 외.

| WI | 작업 | DoD |
|----|------|-----|
| **R0-1** | feature 매트릭스 빌드 확정: `default`+`remote`(C-free) 양 플랫폼 우선 / `storage`+`tls`(C 툴체인)는 Windows MSVC 빌드 검증 | Linux·Windows에서 각 feature 조합 release 빌드 green, 실패 조합 명시 |
| **R0-2** | Windows 네이티브 실사용 검증 — PTY(portable-pty ConPTY), 셸 hook 비대상(bash/zsh 부재)→wrapper 모드 안내, 경로/`\r\n` 처리 | `ai doctor`가 Windows에서 유효 모드(wrapper) 표시, 핵심 명령(`risk`/`mask`/`preview`/`ask`) 동작 |
| **R0-3** | 버전·릴리즈 메타: `Cargo.toml` 0.2.0, `CHANGELOG.md`, `VERSION` | 버전 단조 증가(§29.11), CHANGELOG에 Phase 1~2 + remote M0~M1 요약 |
| **R0-4** | 배포 스크립트: Linux `install.sh`(curl\|sh, PATH 안내) + Windows 설치(`install.ps1` 또는 zip+PATH 안내) | 깨끗한 환경에서 설치→`ai --version` 동작(양 플랫폼) |
| **R0-5** | 크로스빌드 CI: GitHub Actions 릴리즈 워크플로 — `ubuntu-latest`(x86_64-gnu) + `windows-latest`(x86_64-msvc), 아티팩트 업로드 + **SHA256 체크섬** | 태그 push 시 GitHub Release에 바이너리+체크섬 자동 첨부 |
| **R0-6** | 릴리즈 노트 + 설치 문서(`README` 설치 절, feature 빌드 안내) | 사용자가 문서만 보고 설치 가능 |

**경계**: 서명 바이너리(§29.11 full)는 P3-1 trust channel로 이연한다. R0는 SHA256 체크섬까지(서명 인프라는 조직 정책과 동일 채널에서 구축).

---

## 2. RA — remote-approval 완주 (M1 4b → PWA)

현재 부품(크립토 `remote.rs`·게이트 `gate.rs`·검증 `approval.rs`·데몬 `daemon.rs`·세션 왕복/전송 substrate `session.rs`)은 모두 존재한다. 남은 것은 **실제 데몬 프로세스에서의 조립 + 폰 UX**다.

| WI | 작업 | DoD |
|----|------|-----|
| **RA-1** | **디바이스 연결 리스너**: 데몬이 디바이스용 소켓/TCP 리스너로 `session::run_daemon_request`를 호스팅(현재 함수만 존재) | 외부 디바이스 핸들러가 실제 리스너 위에서 handshake+승인 왕복 |
| **RA-2** | **페어링 CLI/QR**: `daemon_pubkey` 신뢰앵커 + `pairing_code`로 디바이스 인증, `DeviceRecord`(pubkey+epoch) 등록 영속화(TOFU, 동시 페어링 거부) | `ai remote pair` → QR/코드 발급, 디바이스 등록·재페어링 거부 |
| **RA-3** | **게이트 플로우 결선**: armed High(opt-in) → 데몬이 등록 디바이스로 승인 왕복 트리거 → `consume`+`validate` 결과로 통과/차단. **fail-closed timeout**(폰 무응답=차단). `decide_gate`에 `NeedsApproval` 밴드 추가 검토 | armed High 명령이 폰 승인 시 통과/거부·타임아웃 시 차단 e2e. **← "M1 데모 green" 체크포인트** |
| **RA-4** | **데몬 컨텍스트 스냅샷**(§31.10) + `context_hash` 산출(env allowlist 해시 + realpath 타깃) | TOCTOU 재검증이 실제 컨텍스트 해시로 동작 |
| **RA-5** | **PWA**(`/approve`·`/pair`): `pwa-approval-mockup.html` 기반 실제 앱 + Noise 클라이언트(WASM 또는 경량) + 로컬/Tailscale 직결 | 실폰 브라우저에서 페어링→승인/거부, 터미널 반영 |
| **RA-6** | 확장: arm TTL(#4 자동 disarm) + heartbeat 최소판(#2) + 승인 상태 표시(#1) | armed 만료 자동 해제, 데몬 도달 heartbeat 표시 |

**경계/불변식**: relay(M2)와 deferred T-RA1~5(결과 승인·거부 사유·히스토리·이벤트 버스·심층 hook-health)는 RA 완주 후 재평가(설계 전제 유지). PWA·전송은 §28 정본의 보안 불변식을 따른다 — E2E(릴레이가 있어도 복호 불가)·device key identity·pairing code·device revoke·key rotation·replay 방지(1회용 nonce)·approval expiration·signed approval token. 위험도 경계는 §30-13(High/Critical 기본 차단, Medium opt-in+강한 확인)을 강제한다.

---

## 3. Phase 3 본체

### P3-1 — 트러스트 채널 + 조직 정책

§30-7·§30-9·§29.11의 핵심: **정책·플러그인·스킬·바이너리가 동일 trust channel을 공유**(서명 코드 중복 방지).

| WI | 작업 | DoD |
|----|------|-----|
| **P3-1-1** | 공통 trust channel 코어: ed25519 manifest 검증(name/version/publisher/permissions/risk_level/signature), 공개키 앵커(OS trust store/MDM 배포) | 서명 검증 단일 모듈, 위조·만료·다운그레이드 거부 |
| **P3-1-2** | signed `policy.d`: 조직 정책 서명 필수, version monotonic, `issued_at`/`expires_at`, **readonly·최우선**(사용자 정책 위) | 미서명·rollback 정책 거부, 조직>사용자 우선순위 e2e |
| **P3-1-3** | 스킬 서명 + 조직 스킬 레지스트리(§26.6): 외부 스킬 기본 비활성·명시 enable, update/revoke·감사 통합 | 미서명 외부 스킬 차단, revoke 즉시 반영 |
| **P3-1-4** | 바이너리 서명(§29.11 full, R0 이연분): 서명 배포 + 자동 업데이트는 서명 검증 후만, 다운그레이드 방지 | 릴리즈 아티팩트 서명, 검증 실패 시 설치 거부 |

### P3-2 — 중앙 감사 + 팀 프로파일 + 엔터프라이즈 마스킹

| WI | 작업 | DoD |
|----|------|-----|
| **P3-2-1** | 중앙 감사 로그 내보내기: 기존 `audit_events`(append-only)를 조직 수집처로 export(OTLP/syslog/파일), 명령 내용 미전송 옵션(§29.5 정합) | 감사 이벤트 외부 export, 민감정보 미포함 검증 |
| **P3-2-2** | 팀별 프로파일: `balanced`/`paranoid` 위에 조직 정의 프로파일 레이어(P3-1 policy.d로 배포) | 조직 프로파일 적용·사용자 오버라이드 경계 |
| **P3-2-3** | 엔터프라이즈 마스킹 규칙: 조직 커스텀 패턴(policy.d 배포), 기존 `mask` 파이프라인 확장 | 조직 규칙 로드·적용, 기본 규칙과 병합 |
| **P3-2-4** | Debug Bundle: 진단 수집(설정·로그·플랫폼·guardrail capability), **마스킹 강제** 후 묶음 | `ai doctor --bundle` 생성물에 secret 미잔존 |

### P3-3 — MCP 확장 + 고격리/가드레일

| WI | 작업 | DoD |
|----|------|-----|
| **P3-3-1** | MCP mutate/external 컨센트: 미선언=write/external 보수 분류(§30-11), privileged 기본 차단/강한 확인 | mutate 도구 컨센트 게이트, 로컬 정책>서버 선언 |
| **P3-3-2** | MCP OAuth(§30-10): OS keyring 저장(평문 금지), scoped token, `ai mcp login/logout/status/rotate-token` | 토큰 keyring 저장·silent refresh·재인증 |
| **P3-3-3** | Guardrails 동적 감시 고도화(§30-8): seccomp/cgroups(Linux 우선), eBPF(Phase 3 한정), capability matrix 갱신 | Linux 동적 감시 동작, WSL/Win 제한 명시(조용한 실패 금지) |
| **P3-3-4** | gVisor 샌드박스: 실행형 preview/위험 명령 격리 백엔드(FU-2 tmpdir→gVisor 승격), 플랫폼 가용성 고지 | gVisor 가용 시 격리 실행, 미가용 시 tmpdir 폴백 |

**플랫폼 경계**: P3-3-3/4의 동적 감시·gVisor는 Linux 우선. Windows 네이티브는 capability matrix로 "미지원" 명시(§30-8). 미가용 플랫폼은 기존 baseline(정적 분석·preview·timeout)으로 폴백하고 High+ 확인을 강화한다.

---

## 4. Phase 4 (요약 — 추후 구체화)

Phase 3 안정화 후 회고를 거쳐 상세화한다. 주요 산출물(§25.3·스케줄 §4):

Cross-Session Knowledge, State Snapshot & Restore, Multi-agent workflow, Long-running task planner, IDE 연동, 웹 대시보드, Voice Input, Firecracker 고격리, **관리형 relay·멀티 디바이스**(RA에서 제외한 relay M2의 완성형이 여기로 합류).

---

## 5. 의존성·시퀀싱 요약

```text
R0 (릴리즈) ──► RA (remote-approval 완주) ──► P3-1 (트러스트 채널)
                                              │
                          P3-1 ──► P3-2 (감사/프로파일/마스킹)
                          P3-1 ──► P3-3 (MCP/고격리)   ※ P3-2·P3-3는 P3-1 위에서 병렬 가능
```

- **R0**는 독립적(현 코드 기준). 가장 먼저, 즉시 가치.
- **RA**는 기존 remote 부품 위 조립 — 외부 의존 없음. RA-3가 데모 체크포인트.
- **P3-1**(trust channel)은 P3-1-4(바이너리 서명)·P3-2·P3-3·스킬/MCP 서명의 공통 토대 → Phase 3 본체의 첫 블록.
- **P3-2·P3-3**은 P3-1 위에서 상호 독립(병렬 가능).
- relay(M2)·T-RA1~5는 RA 완주 후 별도 재평가 → 일부는 Phase 4 관리형 relay로 흡수.

## 6. 비목표 (이번 범위 밖)

- relay(M2) 자체호스팅/관리형, T-RA1~5(결과 승인 등) — RA 완주 후 재평가.
- macOS 릴리즈, `.deb`/`.rpm` 패키징.
- Phase 4 상세 주차 계획.
- 월 예산 시간창·provider 실비용 등 Phase 1/2 잔여 `[ ]`(별도 백로그 유지).
