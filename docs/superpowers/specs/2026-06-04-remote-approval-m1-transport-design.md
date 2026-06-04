# FU-4 / M1 (slice 4a) — Noise 세션 전송 substrate (설계)

> **작성일**: 2026-06-04 · **선행**: M1 s3(`session.rs` 와이어/왕복 로직, 인메모리 버퍼).
> **이 슬라이스 범위: slice-3 승인 왕복을 실제 스트림(소켓) 위에서 완주**(framing + 역할 함수). 게이트 플로우 결선·페어링·PWA는 후속.

## 왜

s3는 Noise 왕복을 **인메모리 버퍼**로 검증했다. 실제 데몬↔폰은 스트림(소켓/Tailscale)을 거친다. Noise 메시지는 가변 길이 바이트라 스트림에 **framing**이 필요하다. 이 슬라이스는 framing + handshake/승인을 스트림 위에서 도는 역할 함수로 만들고, **실제 `UnixStream` 페어**로 완주를 증명한다(substrate가 바뀌어도 s3 로직 불변임을 보인다).

## 설계 결정 (`session.rs` 확장, `remote` feature)

### framing (M0.5: `[u32 BE len][payload]`)
- `send_frame<W: Write>(w, payload)`: 4바이트 BE 길이 + payload, flush.
- `recv_frame<R: Read>(r) -> Vec<u8>`: 길이 읽고 정확히 그만큼 읽음. 상한(1 MiB) 초과 시 에러(DoS 가드).
- 제네릭 `Read`/`Write` → 소켓·페어·TCP 무관(portable). 전송 substrate 교체점.

### 역할 함수 (제네릭 스트림)
- `run_device<S: Read+Write>(stream, device_private, device_sk, approve) -> ApprovalRequestMsg`:
  initiator handshake(3 메시지, frame 단위) → transport → 요청 수신·복호 → `device_respond` 서명 → 응답 송신. (모의 디바이스 = PWA in-repo 대역)
- `run_daemon_request<S: Read+Write>(stream, daemon_private, request) -> ApprovalResponseMsg`:
  responder handshake → transport → 요청 송신 → 응답 수신·복호 반환. **검증(consume+validate)은 호출자**(데몬)가 수행.

handshake 순서(XX): device가 msg1 송신 → daemon이 msg1 수신·msg2 송신 → device가 msg2 수신·msg3 송신 → daemon msg3 수신. 블로킹 스트림 + 각 역할 별도 스레드로 결정적.

## 범위
- **포함**: `send_frame`/`recv_frame` + `run_device`/`run_daemon_request` + 실제 `UnixStream::pair` 위 end-to-end 테스트(handshake+승인 왕복 → daemon이 `validate`=Approved; reject 변형).
- **제외(후속)**: 실제 데몬 프로세스에서 디바이스 연결 수락(별도 리스너)·페어링/디바이스 등록·데몬 게이트 플로우 결선(armed High opt-in → 승인 트리거)·PWA·relay(M2).

## 수용 기준 (DoD)
1. `send_frame`/`recv_frame` 왕복 무손실 + 과대 길이 거부. (단위)
2. **실제 `UnixStream` 페어**: device 스레드 ↔ daemon 스레드가 handshake + 승인 왕복 완주 → daemon `validate`=Approved(reject→Rejected). (통합, unix)
3. `--features remote`·`"storage tls remote"` fmt/clippy/test green, default 불변.
