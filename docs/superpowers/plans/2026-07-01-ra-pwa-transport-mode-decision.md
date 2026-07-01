# 2026-07-01 RA/PWA Transport Mode Decision

## 목적

RA/PWA companion의 local live path가 P4b browser/operator evidence와 monitoring
view까지 통과했으므로, daemon의 기본 transport mode를 명확히 고정한다. 이 문서는
native `device.sock` substrate를 user-facing fallback/flag로 노출할지 여부를 결정하고
다음 작업자가 같은 결정을 다시 반복하지 않게 한다.

## 결정

- Product default transport는 **live-loopback**이다.
- `ai remote daemon`은 remote feature build에서 `127.0.0.1:<ephemeral>`
  HTTP/SSE endpoint를 열고 PWA companion이 `/events`와 `/message`로 승인 요청/응답을
  왕복한다.
- Native `device.sock` substrate는 코드와 테스트에 남기되, 지금은 user-facing
  `--transport device-sock` flag로 공개하지 않는다.
- Relay/Tailscale/WebSocket transport는 M2/enterprise 후속에서 별도 설계로 다룬다.

## 근거

- Browser/PWA flow는 이미 `ai remote daemon --device-id <id>`와 실제 High-risk
  gate approve/reject를 통과하는 repeatable smoke를 갖고 있다.
- PWA companion은 browser sandbox 안에서 동작하므로 native Unix socket에 직접 붙을 수
  없다. 지금 fallback flag를 공개하면 실제 사용자 UX는 live path와 분리되고, 별도
  evidence 없는 운영 경로가 생긴다.
- 기존 native `device.sock` listener는 RA-1/RA-3 substrate와 regression coverage로
  가치가 있다. 삭제하지 않고 내부 fallback 후보로 유지하는 편이 relay/M2 비교에도
  유리하다.
- Live loopback은 기존 `DeviceListenerHandle`, nonce consume, signature validation,
  replay/timeout fail-closed 경계를 재사용하므로 보안 경계를 새로 만들지 않는다.

## 구현 범위

- `ai remote daemon` startup output에 `PWA transport mode : live-loopback`을 출력한다.
- P4b browser evidence smoke는 daemon output에서 transport mode를 검증하고 evidence
  JSON에 `transportMode`를 기록한다.
- 남은 작업 문서에서는 transport mode decision을 완료로 내리고 다음 우선순위를
  release body/MSI/Android signing/buildserver/relay 후속으로 재정렬한다.

## 비범위

- `--transport` CLI flag 추가.
- Native `device.sock` product UX 문서화.
- Relay/M2 transport 구현.
- PWA private key export 또는 browser 보안 경계 완화.

## 완료 기준

- [x] Transport decision 문서 추가.
- [x] Daemon output이 active transport mode를 표시.
- [x] Browser evidence smoke가 `live-loopback` mode를 assert하고 evidence에 기록.
- [x] `docs/HANDOFF.md`, `docs/HISTORY.md`, `docs/TASK.md`,
  `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`,
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-companion-next.md` 갱신.

## 다음 작업

1. v0.3.3 GitHub release body 보강. 태그/자산은 수정하지 않고 사용자용 설치/자산 설명만
   채운다.
2. Windows-native Rust/MSVC/WiX host에서 MSI packaging preflight를 재검토한다.
3. Android signing secrets 등록 및 fdroidserver build/buildserver evidence를 확보한다.
4. RA/PWA relay/M2 transport는 local live loopback path를 유지한 상태에서 별도 설계로
   착수한다.
