# Phase 2 후속 작업 계획 (FU-1~4)

> **작성일**: 2026-06-04 · WI-1~5(Phase 1 실사용 갭) 완료 후속.
> 의존성·위험도 순으로 진행: 자립적 리팩터 → 실행형 preview → PTY 런처 → remote-approval.

## FU-1 — 리팩터 부채 (캐시 LRU + `cmd_parse` 공용화) · 위험 낮음
- **캐시 용량 상한**: `ResponseCache`(HashMap)·`SemanticCache`(Vec) 둘 다 무한 증가(`cache.rs` line 111 TODO). 용량 상한 + 오래된 항목 축출 추가.
- **`cmd_parse` 공용화**: `program_token`이 `preview.rs`·`pipeline.rs`에 중복, 래퍼 스킵(`sudo|doas|env|nohup|nice`+`VAR=`)이 verify/risk/preview/pipeline 4곳에 흩어짐 → `cmdparse` 모듈로 단일화(동작 보존 리팩터).

## FU-2 — 실행형 preview 샌드박스 (§31.11) · 위험 중, Linux
- W9 안전 preview는 실행 없는 diff(cp/mv)·content-at-risk(rm)만. `sed -i`/formatter 등 **실행 필요** diff는 미지원.
- MVP: **tmpdir 백엔드**(대상 임시 복사 → 임시본에서 실행 → diff). bubblewrap/gVisor는 후속. Linux/WSL 검증.

## FU-3 — 영속 PTY 셸 런처 (Native Wrapper 완성) · 위험 큼, Linux
- WI-4는 모드 감지만. 인터랙티브 완성형: `ai shell`이 자체 PTY에서 셸을 띄우고 입력을 가로채 분류·게이트·실행, cwd 등 probe 동기화(§30-1). 프롬프트 파싱 주의.

## FU-4 — remote-approval 빌드 (T-RA1~5) · P2-P3
- `planning/builds/remote-approval/`(DESIGN/TEST-PLAN/CEO-PLAN) 기반. M1 데모 green 이후 재평가 전제. 이벤트 버스 일반화는 YAGNI 주의(T-RA4).

## 진행 상태
- [x] FU-1 — 리팩터 부채 (완료 2026-06-04 — 캐시 2종 용량 상한+축출, `cmdparse` 모듈로 program_token/래퍼-스킵 단일화)
- [x] FU-2 — 실행형 preview 샌드박스 (완료 2026-06-04 — `sandbox` tmpdir 백엔드, sed -i 등 in-place 편집 실제 diff, 원본 미수정, Unix 한정; WSL 검증)
- [ ] FU-3 — 영속 PTY 셸 런처
- [ ] FU-4 — remote-approval 빌드
