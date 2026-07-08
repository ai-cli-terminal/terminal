use super::*;

#[cfg(feature = "remote")]
use super::companion_live::spawn_companion_live_endpoint_with_timeout;
#[cfg(feature = "remote")]
use super::relay_client::{
    relay_now_ms, websocket_accept_key, ParsedWsScheme, ParsedWsUrl, RemoteDaemonBridge,
    RemoteRelayBridge,
};
#[cfg(feature = "remote")]
use super::remote_gate::{now_secs, RemoteDaemonState};

#[test]
fn decide_with_enforces_boundary() {
    // §30-13: armed Critical=block, armed Low=allow, 비-armed=allow.
    assert!(!decide_with("rm -rf /", true, true).is_allow());
    assert!(decide_with("ls -al", true, false).is_allow());
    assert!(decide_with("rm -rf /", false, false).is_allow());
}

#[test]
fn reply_helpers() {
    assert!(GateReply::allow().is_allow());
    assert!(!GateReply::block("x".into()).is_allow());
}

#[test]
fn gate_request_accepts_legacy_json_without_context() {
    let req: GateRequest = serde_json::from_str(r#"{"command":"ls -al"}"#).unwrap();
    assert_eq!(req.command, "ls -al");
    assert!(req.context_origin.is_none());
}

#[cfg(feature = "remote")]
fn registry_with_device() -> crate::device_registry::DeviceRegistry {
    use ed25519_dalek::SigningKey;

    let approval_pubkey = SigningKey::from_bytes(&[3u8; 32])
        .verifying_key()
        .to_bytes();
    let mut registry = crate::device_registry::DeviceRegistry::default();
    registry
        .register_device("phone-1", vec![9u8; 32], approval_pubkey, 1234)
        .unwrap();
    registry
}

#[cfg(feature = "remote")]
fn plan_gate<'a>(
    registry: &crate::device_registry::DeviceRegistry,
    command: &'a str,
    armed: bool,
    allow_high: bool,
    device_id: Option<&'a str>,
    now: u64,
) -> Result<RemoteGateStep> {
    plan_remote_gate(
        registry,
        RemoteGatePlanInput {
            command,
            armed,
            allow_high,
            device_id,
            now,
            ttl: 60,
            context_hash: "ctx",
        },
    )
}

#[cfg(feature = "remote")]
fn companion_http_request(
    addr: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: &str,
) -> (u16, String) {
    use std::io::{Read, Write};

    let mut stream = std::net::TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\n\
             Host: {addr}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or_default()
        .to_string();
    (status, body)
}

#[cfg(feature = "remote")]
fn companion_hello_json(device: &crate::device_registry::RegisteredDevice) -> String {
    crate::session::companion_transport_json(&crate::session::CompanionTransportMsg::Hello {
        protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
        device_id: device.id.clone(),
        noise_pubkey_hex: crate::pairing::hex_encode(&device.noise_pubkey),
        approval_pubkey_hex: crate::pairing::hex_encode(&device.approval_pubkey),
    })
    .unwrap()
}

#[cfg(feature = "remote")]
fn companion_sse_message(body: &str) -> crate::session::CompanionTransportMsg {
    let data = body
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .expect("SSE body must include data line");
    crate::session::parse_companion_transport_json(data).unwrap()
}

#[cfg(feature = "remote")]
#[test]
fn companion_live_endpoint_serves_descriptor_events_and_ping() {
    let registry = registry_with_device();
    let device = registry.get("phone-1").unwrap().clone();
    let handle = spawn_companion_live_endpoint_with_timeout(
        registry,
        None,
        std::time::Duration::from_secs(1),
    )
    .unwrap();

    let (status, body) = companion_http_request(handle.addr, "GET", "/health", "");
    assert_eq!(status, 200);
    let descriptor: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(descriptor["status"], "ready");
    assert_eq!(
        descriptor["protocol_version"],
        crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION
    );
    assert_eq!(descriptor["message_url"], handle.message_url);
    assert_eq!(descriptor["events_url"], handle.events_url);

    let (status, body) = companion_http_request(
        handle.addr,
        "POST",
        "/message",
        &companion_hello_json(&device),
    );
    assert_eq!(status, 200);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "hello:phone-1"
    ));

    let (status, body) = companion_http_request(handle.addr, "GET", "/events", "");
    assert_eq!(status, 200);
    assert!(body.contains("event: message"), "{body}");
    assert!(body.contains(r#""type":"ping""#), "{body}");

    let ping =
        crate::session::companion_transport_json(&crate::session::CompanionTransportMsg::Ping {
            nonce: "p1".into(),
        })
        .unwrap();
    let (status, body) = companion_http_request(handle.addr, "POST", "/message", &ping);
    assert_eq!(status, 200);
    assert_eq!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Pong { nonce: "p1".into() }
    );
}

#[cfg(feature = "remote")]
#[test]
fn companion_live_endpoint_validates_registered_hello() {
    let registry = registry_with_device();
    let device = registry.get("phone-1").unwrap().clone();
    let handle = spawn_companion_live_endpoint_with_timeout(
        registry,
        Some("phone-1".into()),
        std::time::Duration::from_secs(1),
    )
    .unwrap();

    let hello =
        crate::session::companion_transport_json(&crate::session::CompanionTransportMsg::Hello {
            protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: device.id,
            noise_pubkey_hex: crate::pairing::hex_encode(&device.noise_pubkey),
            approval_pubkey_hex: crate::pairing::hex_encode(&device.approval_pubkey),
        })
        .unwrap();
    let (status, body) = companion_http_request(handle.addr, "POST", "/message", &hello);
    assert_eq!(status, 200);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "hello:phone-1"
    ));

    let rejected =
        crate::session::companion_transport_json(&crate::session::CompanionTransportMsg::Hello {
            protocol_version: crate::session::COMPANION_TRANSPORT_PROTOCOL_VERSION,
            device_id: "phone-1".into(),
            noise_pubkey_hex: "a".repeat(64),
            approval_pubkey_hex: "b".repeat(64),
        })
        .unwrap();
    let (status, body) = companion_http_request(handle.addr, "POST", "/message", &rejected);
    assert_eq!(status, 403);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Error { message } if message.contains("hello rejected")
    ));
}

#[cfg(feature = "remote")]
#[test]
fn companion_live_bridge_roundtrip_allows_gate_request() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let registry = registry_with_device();
    let device = registry.get("phone-1").unwrap().clone();
    let registry_for_gate = registry.clone();
    let handle = spawn_companion_live_endpoint_with_timeout(
        registry,
        Some("phone-1".into()),
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let addr = handle.addr;

    let (status, _) =
        companion_http_request(addr, "POST", "/message", &companion_hello_json(&device));
    assert_eq!(status, 200);

    let live_listener = handle.listener;
    let now = now_secs();
    let gate_thread = std::thread::spawn(move || {
        decide_with_remote_listener(
            &registry_for_gate,
            &live_listener,
            RemoteGateRun {
                command: "chmod -R 777 .",
                armed: true,
                allow_high: true,
                now,
                ttl: 60,
                device_id: Some("phone-1"),
                issued_context_hash: "ctx",
                context_origin: None,
                current_context_hash: Some("ctx"),
                response_timeout: std::time::Duration::from_secs(5),
            },
        )
    });

    let (status, body) = companion_http_request(addr, "GET", "/events", "");
    assert_eq!(status, 200);
    let request = match companion_sse_message(&body) {
        crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
        other => panic!("expected approval_request SSE, got {other:?}"),
    };
    assert_eq!(request.command_masked, "chmod -R 777 .");

    let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
    let response_json = crate::session::companion_transport_json(
        &crate::session::CompanionTransportMsg::ApprovalResponse { response },
    )
    .unwrap();
    let (status, body) = companion_http_request(addr, "POST", "/message", &response_json);
    assert_eq!(status, 200);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Pong { nonce } if nonce == "approval_response"
    ));

    let reply = gate_thread.join().unwrap();
    assert!(reply.is_allow(), "{reply:?}");
}

#[cfg(feature = "remote")]
#[test]
fn companion_live_bridge_rejects_unmatched_response_without_losing_pending_request() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let registry = registry_with_device();
    let device = registry.get("phone-1").unwrap().clone();
    let registry_for_gate = registry.clone();
    let handle = spawn_companion_live_endpoint_with_timeout(
        registry,
        Some("phone-1".into()),
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let addr = handle.addr;
    let (status, _) =
        companion_http_request(addr, "POST", "/message", &companion_hello_json(&device));
    assert_eq!(status, 200);

    let live_listener = handle.listener;
    let now = now_secs();
    let gate_thread = std::thread::spawn(move || {
        decide_with_remote_listener(
            &registry_for_gate,
            &live_listener,
            RemoteGateRun {
                command: "chmod -R 777 .",
                armed: true,
                allow_high: true,
                now,
                ttl: 60,
                device_id: Some("phone-1"),
                issued_context_hash: "ctx",
                context_origin: None,
                current_context_hash: Some("ctx"),
                response_timeout: std::time::Duration::from_secs(5),
            },
        )
    });

    let (status, body) = companion_http_request(addr, "GET", "/events", "");
    assert_eq!(status, 200);
    let request = match companion_sse_message(&body) {
        crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
        other => panic!("expected approval_request SSE, got {other:?}"),
    };

    let mut wrong_response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
    wrong_response.approval_id = b"other-approval".to_vec();
    let wrong_json = crate::session::companion_transport_json(
        &crate::session::CompanionTransportMsg::ApprovalResponse {
            response: wrong_response,
        },
    )
    .unwrap();
    let (status, body) = companion_http_request(addr, "POST", "/message", &wrong_json);
    assert_eq!(status, 409);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Error { message } if message.contains("does not match")
    ));

    let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
    let response_json = crate::session::companion_transport_json(
        &crate::session::CompanionTransportMsg::ApprovalResponse { response },
    )
    .unwrap();
    let (status, _) = companion_http_request(addr, "POST", "/message", &response_json);
    assert_eq!(status, 200);

    let reply = gate_thread.join().unwrap();
    assert!(reply.is_allow(), "{reply:?}");
}

#[cfg(feature = "remote")]
#[test]
fn companion_live_endpoint_rejects_malformed_unknown_and_incomplete_requests() {
    use std::io::{Read, Write};

    let registry = registry_with_device();
    let handle = spawn_companion_live_endpoint_with_timeout(
        registry,
        None,
        std::time::Duration::from_millis(50),
    )
    .unwrap();

    let (status, body) = companion_http_request(handle.addr, "POST", "/message", "{");
    assert_eq!(status, 400);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Error { .. }
    ));

    let (status, body) = companion_http_request(handle.addr, "GET", "/missing", "");
    assert_eq!(status, 404);
    assert!(matches!(
        crate::session::parse_companion_transport_json(&body).unwrap(),
        crate::session::CompanionTransportMsg::Error { .. }
    ));

    let mut stream = std::net::TcpStream::connect(handle.addr).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let request = format!(
        "POST /message HTTP/1.1\r\nHost: {}\r\nContent-Length: 32\r\n\r\n{{",
        handle.addr
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 408"), "{response}");
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_plans_only_high_opt_in() {
    let registry = registry_with_device();

    assert!(matches!(
        plan_gate(&registry, "ls -al", true, true, None, 100).unwrap(),
        RemoteGateStep::Local(reply) if reply.is_allow()
    ));
    assert!(matches!(
        plan_gate(&registry, "rm -rf /", true, true, None, 100).unwrap(),
        RemoteGateStep::Local(reply) if !reply.is_allow()
    ));
    assert!(matches!(
        plan_gate(&registry, "chmod -R 777 .", true, false, None, 100).unwrap(),
        RemoteGateStep::Local(reply) if !reply.is_allow()
    ));

    let step = plan_gate(&registry, "chmod -R 777 .", true, true, None, 100).unwrap();
    match step {
        RemoteGateStep::NeedsRemote(plan) => {
            assert_eq!(plan.device_id, "phone-1");
            assert_eq!(plan.pending.expires_at, 160);
            assert_eq!(plan.pending.context_hash, "ctx");
            assert_eq!(plan.pending.device_epoch, 1);
            assert_eq!(plan.request.command_masked, "chmod -R 777 .");
        }
        other => panic!("expected remote approval plan, got {other:?}"),
    }
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_requires_single_registered_device() {
    let empty = crate::device_registry::DeviceRegistry::default();
    assert!(
        plan_gate(&empty, "chmod -R 777 .", true, true, None, 100).is_err(),
        "등록 디바이스 없으면 fail-closed 경계"
    );

    let mut multi = registry_with_device();
    multi
        .register_device("phone-2", vec![10u8; 32], [4u8; 32], 1235)
        .unwrap();
    assert!(
        plan_gate(&multi, "chmod -R 777 .", true, true, None, 100).is_err(),
        "여러 디바이스면 명시 선택 전까지 ambiguous"
    );

    let selected = plan_gate(&multi, "chmod -R 777 .", true, true, Some("phone-2"), 100).unwrap();
    match selected {
        RemoteGateStep::NeedsRemote(plan) => {
            assert_eq!(plan.device_id, "phone-2");
            assert_eq!(plan.pending.device_epoch, 1);
        }
        other => panic!("expected selected remote approval plan, got {other:?}"),
    }
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_response_approve_reject_replay_and_timeout() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let registry = registry_with_device();

    let plan = match plan_gate(&registry, "chmod -R 777 .", true, true, None, 100).unwrap() {
        RemoteGateStep::NeedsRemote(plan) => plan,
        other => panic!("expected remote approval plan, got {other:?}"),
    };
    let approve = crate::session::device_respond(&plan.request, &DEVICE_SK, true).unwrap();
    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(plan.pending.nonce, plan.pending.expires_at);
    let reply = finish_remote_gate_response(&registry, &plan, &mut nonces, 120, "ctx", &approve);
    assert!(reply.is_allow(), "{reply:?}");
    let replay = finish_remote_gate_response(&registry, &plan, &mut nonces, 120, "ctx", &approve);
    assert!(!replay.is_allow(), "동일 nonce replay는 차단");

    let reject_plan = match plan_gate(&registry, "chmod -R 777 .", true, true, None, 200).unwrap() {
        RemoteGateStep::NeedsRemote(plan) => plan,
        other => panic!("expected remote approval plan, got {other:?}"),
    };
    let reject = crate::session::device_respond(&reject_plan.request, &DEVICE_SK, false).unwrap();
    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(reject_plan.pending.nonce, reject_plan.pending.expires_at);
    let reply =
        finish_remote_gate_response(&registry, &reject_plan, &mut nonces, 220, "ctx", &reject);
    assert!(!reply.is_allow());
    assert!(reply.reason.contains("거부"), "{reply:?}");

    let drift_plan = match plan_gate(&registry, "chmod -R 777 .", true, true, None, 300).unwrap() {
        RemoteGateStep::NeedsRemote(plan) => plan,
        other => panic!("expected remote approval plan, got {other:?}"),
    };
    let drift = crate::session::device_respond(&drift_plan.request, &DEVICE_SK, true).unwrap();
    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(drift_plan.pending.nonce, drift_plan.pending.expires_at);
    let reply = finish_remote_gate_response(
        &registry,
        &drift_plan,
        &mut nonces,
        320,
        "ctx-drift",
        &drift,
    );
    assert!(!reply.is_allow());
    assert!(reply.reason.contains("검증 실패"), "{reply:?}");

    let timeout = remote_timeout_reply();
    assert!(!timeout.is_allow());
    assert!(timeout.reason.contains("시간 초과"), "{timeout:?}");
}

#[cfg(feature = "remote")]
fn relay_gate_run<'a>(command: &'a str, now: u64) -> RemoteGateRun<'a> {
    RemoteGateRun {
        command,
        armed: true,
        allow_high: true,
        now,
        ttl: 60,
        device_id: None,
        issued_context_hash: "ctx",
        context_origin: None,
        current_context_hash: Some("ctx"),
        response_timeout: std::time::Duration::from_secs(5),
    }
}

#[cfg(feature = "remote")]
fn relay_loopback_roundtrip(
    relay: &mut crate::remote_transport::CompanionRelayLoopback,
    daemon_endpoint: &mut crate::remote_transport::CompanionRelayEndpoint,
    companion_endpoint: &mut crate::remote_transport::CompanionRelayEndpoint,
    now_ms: &mut u64,
    message: crate::session::CompanionTransportMsg,
    approve: bool,
) -> Result<crate::session::CompanionTransportMsg> {
    const DEVICE_SK: [u8; 32] = [3u8; 32];

    daemon_endpoint.send_message(relay, *now_ms, &message)?;
    *now_ms += 1;
    let companion_message = companion_endpoint
        .recv_message(relay, *now_ms)?
        .ok_or_else(|| anyhow::anyhow!("missing relay companion request"))?;
    assert_eq!(companion_message, message);
    let request = match companion_message {
        crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
        other => anyhow::bail!("unexpected relay companion request: {other:?}"),
    };

    let response = crate::session::device_respond(&request, &DEVICE_SK, approve)?;
    let response_message = crate::session::CompanionTransportMsg::ApprovalResponse { response };
    *now_ms += 1;
    companion_endpoint.send_message(relay, *now_ms, &response_message)?;
    *now_ms += 1;
    daemon_endpoint
        .recv_message(relay, *now_ms)?
        .ok_or_else(|| anyhow::anyhow!("missing relay daemon response"))
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_relay_bridge_loopback_allows_approved_high_command() {
    let registry = registry_with_device();
    let mut relay = crate::remote_transport::CompanionRelayLoopback::new();
    let mut daemon_endpoint =
        crate::remote_transport::CompanionRelayEndpoint::daemon("relay-gate-session").unwrap();
    let mut companion_endpoint =
        crate::remote_transport::CompanionRelayEndpoint::companion("relay-gate-session").unwrap();
    let mut now_ms = 10_000;

    let reply = decide_with_remote_relay_bridge(
        &registry,
        relay_gate_run("chmod -R 777 .", 100),
        |plan, message| {
            assert_eq!(plan.device_id, "phone-1");
            relay_loopback_roundtrip(
                &mut relay,
                &mut daemon_endpoint,
                &mut companion_endpoint,
                &mut now_ms,
                message,
                true,
            )
        },
    );

    assert!(reply.is_allow(), "{reply:?}");
    assert_eq!(daemon_endpoint.next_sequence(), 2);
    assert_eq!(companion_endpoint.next_sequence(), 2);
    assert_eq!(relay.stats().queued_frames, 0);
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_relay_bridge_rejects_declined_response() {
    let registry = registry_with_device();
    let mut relay = crate::remote_transport::CompanionRelayLoopback::new();
    let mut daemon_endpoint =
        crate::remote_transport::CompanionRelayEndpoint::daemon("relay-gate-reject").unwrap();
    let mut companion_endpoint =
        crate::remote_transport::CompanionRelayEndpoint::companion("relay-gate-reject").unwrap();
    let mut now_ms = 20_000;

    let reply = decide_with_remote_relay_bridge(
        &registry,
        relay_gate_run("chmod -R 777 .", 200),
        |_plan, message| {
            relay_loopback_roundtrip(
                &mut relay,
                &mut daemon_endpoint,
                &mut companion_endpoint,
                &mut now_ms,
                message,
                false,
            )
        },
    );

    assert!(!reply.is_allow(), "{reply:?}");
    assert!(reply.reason.contains("거부"), "{reply:?}");
    assert_eq!(relay.stats().queued_frames, 0);
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_relay_bridge_skips_bridge_for_local_decision() {
    let registry = registry_with_device();
    let mut called = false;

    let reply =
        decide_with_remote_relay_bridge(&registry, relay_gate_run("ls -al", 100), |_plan, _| {
            called = true;
            Ok(crate::session::CompanionTransportMsg::Ping {
                nonce: "unexpected".into(),
            })
        });

    assert!(reply.is_allow(), "{reply:?}");
    assert!(!called, "local gate decisions must not call relay bridge");
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_relay_bridge_rejects_invalid_response_type() {
    let registry = registry_with_device();

    let reply = decide_with_remote_relay_bridge(
        &registry,
        relay_gate_run("chmod -R 777 .", 100),
        |_plan, _message| {
            Ok(crate::session::CompanionTransportMsg::Ping {
                nonce: "not-approval-response".into(),
            })
        },
    );

    assert!(!reply.is_allow(), "{reply:?}");
    assert!(reply.reason.contains("응답 타입"), "{reply:?}");
}

#[cfg(feature = "remote")]
struct MockRuntimeRelayBridge {
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    approve: bool,
}

#[cfg(feature = "remote")]
impl RemoteRelayBridge for MockRuntimeRelayBridge {
    fn roundtrip(
        &mut self,
        _plan: &RemoteApprovalPlan,
        request: crate::session::CompanionTransportMsg,
        _timeout: std::time::Duration,
    ) -> Result<crate::session::CompanionTransportMsg> {
        const DEVICE_SK: [u8; 32] = [3u8; 32];

        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match request {
            crate::session::CompanionTransportMsg::ApprovalRequest { request } => {
                let response = crate::session::device_respond(&request, &DEVICE_SK, self.approve)?;
                Ok(crate::session::CompanionTransportMsg::ApprovalResponse { response })
            }
            other => anyhow::bail!("unexpected relay request: {other:?}"),
        }
    }
}

#[cfg(feature = "remote")]
fn relay_runtime_state(
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    approve: bool,
) -> RemoteDaemonState {
    RemoteDaemonState {
        registry: registry_with_device(),
        device_id: None,
        bridge: RemoteDaemonBridge::Relay(std::sync::Arc::new(std::sync::Mutex::new(Box::new(
            MockRuntimeRelayBridge { calls, approve },
        )))),
        approval_ttl: 60,
        response_timeout: std::time::Duration::from_secs(5),
    }
}

#[cfg(feature = "remote")]
fn relay_runtime_gate_request(command: &str) -> GateRequest {
    GateRequest {
        command: command.to_string(),
        context_origin: Some(crate::context::RemoteContextOrigin::gather()),
    }
}

#[cfg(feature = "remote")]
#[test]
fn relay_daemon_runtime_uses_relay_bridge_for_high_command() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let state = relay_runtime_state(calls.clone(), true);

    let reply = state.decide_with_arm(&relay_runtime_gate_request("chmod -R 777 ."), true, true);

    assert!(reply.is_allow(), "{reply:?}");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[cfg(feature = "remote")]
#[test]
fn relay_daemon_runtime_skips_relay_for_local_decision() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let state = relay_runtime_state(calls.clone(), true);

    let reply = state.decide_with_arm(&relay_runtime_gate_request("ls -al"), true, true);

    assert!(reply.is_allow(), "{reply:?}");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[cfg(feature = "remote")]
fn test_read_http_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let header = String::from_utf8_lossy(&bytes).to_string();
    let content_len = header
        .lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
        })
        .unwrap_or(0);
    if content_len > 0 {
        let mut body = vec![0_u8; content_len];
        stream.read_exact(&mut body).unwrap();
        format!("{header}{}", String::from_utf8_lossy(&body))
    } else {
        header
    }
}

#[cfg(feature = "remote")]
fn test_write_ws_text(stream: &mut std::net::TcpStream, text: &str) {
    use std::io::Write;

    let payload = text.as_bytes();
    let mut frame = Vec::new();
    frame.push(0x81);
    if payload.len() <= 125 {
        frame.push(u8::try_from(payload.len()).unwrap());
    } else {
        frame.push(126);
        frame.extend_from_slice(&u16::try_from(payload.len()).unwrap().to_be_bytes());
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame).unwrap();
    stream.flush().unwrap();
}

#[cfg(feature = "remote")]
fn test_read_ws_text(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;

    let mut header = [0_u8; 2];
    stream.read_exact(&mut header).unwrap();
    assert_eq!(header[0] & 0x0F, 0x1);
    let masked = header[1] & 0x80 != 0;
    let mut len = usize::from(header[1] & 0x7F);
    if len == 126 {
        let mut ext = [0_u8; 2];
        stream.read_exact(&mut ext).unwrap();
        len = usize::from(u16::from_be_bytes(ext));
    }
    let mut mask = [0_u8; 4];
    if masked {
        stream.read_exact(&mut mask).unwrap();
    }
    let mut payload = vec![0_u8; len];
    stream.read_exact(&mut payload).unwrap();
    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % mask.len()];
        }
    }
    String::from_utf8(payload).unwrap()
}

#[cfg(feature = "remote")]
fn relay_runtime_setup_for_url(
    relay_endpoint_url: String,
) -> crate::remote_transport::CompanionRelaySelfHostedRuntimeSetup {
    use ed25519_dalek::SigningKey;

    let approval_pubkey = SigningKey::from_bytes(&[3u8; 32])
        .verifying_key()
        .to_bytes();
    let keyring = crate::remote_transport::CompanionRelayTicketKeyringRecord::new(
        "relay-active-test",
        vec![7u8; 32],
        1000,
    )
    .unwrap();
    crate::remote_transport::issue_self_hosted_relay_runtime_setup(
        &keyring,
        crate::remote_transport::CompanionRelaySelfHostedSetupInput {
            deployment_mode: None,
            private_network_name: None,
            relay_endpoint_url,
            daemon_pubkey: vec![8u8; 32],
            companion_device_id: "phone-1".into(),
            companion_noise_pubkey: vec![9u8; 32],
            companion_approval_pubkey: approval_pubkey,
            issued_at_ms: 1000,
            ttl_ms: 60_000,
            session_id: Some("relay-runtime-test".into()),
            session_token: Some("token_relay_runtime_test_1234567890abcdef".into()),
        },
    )
    .unwrap()
}

#[cfg(feature = "remote")]
#[test]
fn relay_daemon_runtime_parses_wss_and_keeps_public_ws_blocked() {
    let hosted = ParsedWsUrl::parse("wss://relay.example.test/relay").unwrap();
    assert_eq!(hosted.scheme, ParsedWsScheme::Wss);
    assert_eq!(hosted.host, "relay.example.test");
    assert_eq!(hosted.port, 443);
    assert_eq!(hosted.host_header, "relay.example.test:443");
    assert_eq!(hosted.path, "/relay");

    let hosted_with_port =
        ParsedWsUrl::parse("wss://relay.example.test:8443/relay?region=test").unwrap();
    assert_eq!(hosted_with_port.scheme, ParsedWsScheme::Wss);
    assert_eq!(hosted_with_port.host, "relay.example.test");
    assert_eq!(hosted_with_port.port, 8443);
    assert_eq!(hosted_with_port.host_header, "relay.example.test:8443");
    assert_eq!(hosted_with_port.path, "/relay?region=test");

    let local = ParsedWsUrl::parse("ws://127.0.0.1:49152/relay").unwrap();
    assert_eq!(local.scheme, ParsedWsScheme::Ws);
    assert_eq!(local.host, "127.0.0.1");
    assert_eq!(local.port, 49152);

    assert!(ParsedWsUrl::parse("ws://relay.example.test/relay").is_err());
    assert!(ParsedWsUrl::parse("https://relay.example.test/relay").is_err());
}

#[cfg(feature = "remote")]
#[test]
fn relay_daemon_runtime_websocket_roundtrip_allows_approved_high_command() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let setup = relay_runtime_setup_for_url(format!("ws://{addr}/relay"));
    let server_setup = setup.clone();
    let server = std::thread::spawn(move || {
        use std::io::Write;

        let (mut register_stream, _) = listener.accept().unwrap();
        let register_request = test_read_http_request(&mut register_stream);
        assert!(
            register_request.starts_with("POST /sessions "),
            "{register_request}"
        );
        assert!(
            register_request.contains("relay-runtime-test"),
            "{register_request}"
        );
        register_stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 24\r\nConnection: close\r\n\r\n{\"status\":\"registered\"}\n",
                )
                .unwrap();
        drop(register_stream);

        let (mut ws_stream, _) = listener.accept().unwrap();
        let handshake = test_read_http_request(&mut ws_stream);
        assert!(
            handshake.starts_with("GET /relay?session_id=relay-runtime-test&role=daemon "),
            "{handshake}"
        );
        let key = handshake
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("sec-websocket-key")
                        .then(|| value.trim().to_string())
                })
            })
            .unwrap();
        let accept = websocket_accept_key(&key);
        ws_stream
                .write_all(
                    format!(
                        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .unwrap();

        let connect_json = test_read_ws_text(&mut ws_stream);
        let connect: crate::remote_transport::CompanionRelaySessionConnect =
            serde_json::from_str(&connect_json).unwrap();
        assert_eq!(
            connect.peer,
            crate::remote_transport::CompanionRelayPeer::Daemon
        );
        test_write_ws_text(
            &mut ws_stream,
            r#"{"kind":"connected","session_id":"relay-runtime-test","peer":"daemon"}"#,
        );

        let frame_json = test_read_ws_text(&mut ws_stream);
        let request_frame: crate::remote_transport::CompanionRelayFrame =
            serde_json::from_str(&frame_json).unwrap();
        let request_message = request_frame.payload_message().unwrap();
        let request = match request_message {
            crate::session::CompanionTransportMsg::ApprovalRequest { request } => request,
            other => panic!("expected approval request, got {other:?}"),
        };
        let response = crate::session::device_respond(&request, &DEVICE_SK, true).unwrap();
        let response_message = crate::session::CompanionTransportMsg::ApprovalResponse { response };
        let now_ms = relay_now_ms();
        let response_frame = crate::remote_transport::CompanionRelayFrame::from_message(
            server_setup.signed_session_ticket.ticket.session_id,
            crate::remote_transport::CompanionRelayPeer::Companion,
            1,
            now_ms,
            now_ms + 30_000,
            &response_message,
        )
        .unwrap();
        let envelope = serde_json::json!({
            "kind": "frame",
            "route": response_frame.route_envelope().unwrap(),
            "frame_json": serde_json::to_string(&response_frame).unwrap(),
        });
        test_write_ws_text(&mut ws_stream, &serde_json::to_string(&envelope).unwrap());
    });

    let registry = registry_with_device();
    let mut runtime = CompanionRelayDaemonRuntime::new(setup).unwrap();
    let reply = decide_with_remote_relay_bridge(
        &registry,
        relay_gate_run("chmod -R 777 .", 100),
        |plan, request| runtime.roundtrip(plan, request, std::time::Duration::from_secs(5)),
    );

    assert!(reply.is_allow(), "{reply:?}");
    server.join().unwrap();
}

#[cfg(feature = "remote")]
#[test]
fn remote_gate_listener_roundtrip_allows_approved_high_command() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let registry = registry_with_device();
    let dir = std::env::temp_dir().join(format!(
        "ra_gate_listener_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sock = dir.join("device.sock");
    let dev_kp = crate::remote::generate_static_keypair().unwrap();
    let dmn_kp = crate::remote::generate_static_keypair().unwrap();
    let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();
    assert!(sock.exists(), "device listener must bind before returning");

    let sock_for_device = sock.clone();
    let device_thread = std::thread::spawn(move || {
        crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
            .unwrap()
    });
    let command = "chmod -R 777 .";
    let origin = crate::context::RemoteContextOrigin {
        cwd: std::env::current_dir().unwrap().display().to_string(),
        env: vec![
            ("PATH".into(), "hash-test".into()),
            ("SHELL".into(), "/bin/bash".into()),
            ("USER".into(), "alice".into()),
            ("HOSTNAME".into(), "host".into()),
        ],
    };
    let issued_context_hash = crate::context::remote_context_hash_for_origin(command, &origin);
    let reply = decide_with_remote_listener(
        &registry,
        &handle,
        RemoteGateRun {
            command,
            armed: true,
            allow_high: true,
            now: 100,
            ttl: 60,
            device_id: None,
            issued_context_hash: &issued_context_hash,
            context_origin: Some(&origin),
            current_context_hash: None,
            response_timeout: std::time::Duration::from_secs(5),
        },
    );
    assert!(reply.is_allow(), "{reply:?}");
    let got_req = device_thread.join().unwrap();
    assert_eq!(got_req.command_masked, "chmod -R 777 .");

    drop(handle.request_tx);
    handle.thread.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// serve↔query IPC 왕복: 데몬 태스크 기동 → 클라이언트 질의 → 회신이 로컬 결정과 일치.
/// (결정 내용 정확성은 decide_with 단위테스트, 여기선 소켓/프레이밍/직렬화 왕복 검증.)
#[test]
fn serve_query_roundtrip_matches_local_decision() {
    use std::time::Duration;

    let dir = std::env::temp_dir().join(format!("ra_daemon_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("gate.sock");
    let sock_srv = sock.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let handle = rt.spawn(async move {
        let _ = serve(&sock_srv).await;
    });
    // 소켓 등장 대기.
    for _ in 0..200 {
        if sock.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(sock.exists(), "데몬이 소켓을 바인드해야 함");

    for cmd in ["rm -rf /", "ls -al"] {
        let via_socket = query(&sock, cmd).unwrap();
        let local = decide_request(cmd);
        assert_eq!(via_socket, local, "IPC 회신이 로컬 결정과 일치해야: {cmd}");
    }

    handle.abort();
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "remote")]
#[test]
fn device_listener_once_roundtrip() {
    use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
    use ed25519_dalek::SigningKey;

    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let dir = std::env::temp_dir().join(format!(
        "ra_daemon_device_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sock = dir.join("device.sock");
    let dev_kp = crate::remote::generate_static_keypair().unwrap();
    let dmn_kp = crate::remote::generate_static_keypair().unwrap();
    let pending = PendingApproval {
        approval_id: b"appr-daemon-1".to_vec(),
        nonce: [11u8; 32],
        expires_at: 9999,
        context_hash: "ctx".into(),
        device_epoch: 1,
    };
    let req = crate::session::ApprovalRequestMsg::from_pending(&pending, "rm -rf /data");

    let daemon_private = dmn_kp.private.clone();
    let daemon_req = req.clone();
    let sock_for_device = sock.clone();
    let device_thread = std::thread::spawn(move || {
        // listener bind와 accept 준비 시간을 짧게 폴링한다.
        for _ in 0..200 {
            if sock_for_device.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
            .unwrap()
    });
    let resp = serve_device_once(&sock, &daemon_private, &daemon_req).unwrap();
    let got_req = device_thread.join().unwrap();
    assert_eq!(got_req, req);

    let mut nonces = NonceStore::new();
    nonces.register(pending.nonce, pending.expires_at);
    assert!(nonces.consume(&pending.nonce, 100), "최초 nonce 소비");
    let device = DeviceRecord {
        pubkey: SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes(),
        epoch: 1,
    };
    let outcome = approval::validate(&pending, &device, 100, "ctx", &resp.to_signed().unwrap());
    assert_eq!(outcome, ApprovalOutcome::Approved);

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "remote")]
#[test]
fn device_listener_loop_handles_multiple_connections() {
    use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
    use ed25519_dalek::SigningKey;

    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let dir = std::env::temp_dir().join(format!(
        "ra_daemon_device_loop_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sock = dir.join("device.sock");
    let dev_kp = crate::remote::generate_static_keypair().unwrap();
    let dmn_kp = crate::remote::generate_static_keypair().unwrap();
    let p1 = PendingApproval {
        approval_id: b"appr-loop-1".to_vec(),
        nonce: [21u8; 32],
        expires_at: 9999,
        context_hash: "ctx".into(),
        device_epoch: 1,
    };
    let p2 = PendingApproval {
        approval_id: b"appr-loop-2".to_vec(),
        nonce: [22u8; 32],
        expires_at: 9999,
        context_hash: "ctx".into(),
        device_epoch: 1,
    };
    let req1 = crate::session::ApprovalRequestMsg::from_pending(&p1, "rm -rf build");
    let req2 = crate::session::ApprovalRequestMsg::from_pending(&p2, "sudo systemctl restart x");

    let daemon_private = dmn_kp.private.clone();
    let daemon_sock = sock.clone();
    let daemon_thread = std::thread::spawn(move || {
        let mut requests = vec![req1.clone(), req2.clone()].into_iter();
        let mut responses = Vec::new();
        serve_device_loop(
            &daemon_sock,
            &daemon_private,
            || requests.next(),
            |resp| {
                responses.push(resp.unwrap());
                responses.len() < 2
            },
        )
        .unwrap();
        responses
    });

    for _ in 0..200 {
        if sock.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let got1 =
        crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, true).unwrap();
    let got2 =
        crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, false).unwrap();
    assert_eq!(got1.approval_id, p1.approval_id);
    assert_eq!(got2.approval_id, p2.approval_id);
    let responses = daemon_thread.join().unwrap();
    assert_eq!(responses.len(), 2);

    let device = DeviceRecord {
        pubkey: SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes(),
        epoch: 1,
    };
    let mut nonces = NonceStore::new();
    nonces.register(p1.nonce, p1.expires_at);
    nonces.register(p2.nonce, p2.expires_at);
    assert!(nonces.consume(&p1.nonce, 100));
    assert!(nonces.consume(&p2.nonce, 100));
    assert_eq!(
        approval::validate(&p1, &device, 100, "ctx", &responses[0].to_signed().unwrap()),
        ApprovalOutcome::Approved
    );
    assert_eq!(
        approval::validate(&p2, &device, 100, "ctx", &responses[1].to_signed().unwrap()),
        ApprovalOutcome::Rejected
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "remote")]
#[test]
fn spawned_device_listener_handles_queued_request() {
    use crate::approval::{self, ApprovalOutcome, DeviceRecord, NonceStore, PendingApproval};
    use ed25519_dalek::SigningKey;

    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let dir = std::env::temp_dir().join(format!(
        "ra_daemon_spawned_device_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sock = dir.join("device.sock");
    let dev_kp = crate::remote::generate_static_keypair().unwrap();
    let dmn_kp = crate::remote::generate_static_keypair().unwrap();
    let pending = PendingApproval {
        approval_id: b"appr-spawned-1".to_vec(),
        nonce: [31u8; 32],
        expires_at: 9999,
        context_hash: "ctx".into(),
        device_epoch: 1,
    };
    let req = crate::session::ApprovalRequestMsg::from_pending(&pending, "rm -rf build");

    let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();
    assert!(sock.exists(), "spawned listener must bind device.sock");
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    handle
        .request_tx
        .send(DeviceListenerRequest {
            request: req.clone(),
            response_tx,
            accept_timeout: std::time::Duration::from_secs(5),
        })
        .unwrap();
    let got_req =
        crate::session::run_device_connect(&sock, &dev_kp.private, &DEVICE_SK, true).unwrap();
    assert_eq!(got_req, req);
    let resp = response_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap()
        .unwrap();
    drop(handle.request_tx);
    handle.thread.join().unwrap();

    let mut nonces = NonceStore::new();
    nonces.register(pending.nonce, pending.expires_at);
    assert!(nonces.consume(&pending.nonce, 100), "최초 nonce 소비");
    let device = DeviceRecord {
        pubkey: SigningKey::from_bytes(&DEVICE_SK)
            .verifying_key()
            .to_bytes(),
        epoch: 1,
    };
    let outcome = approval::validate(&pending, &device, 100, "ctx", &resp.to_signed().unwrap());
    assert_eq!(outcome, ApprovalOutcome::Approved);

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "remote")]
#[test]
fn device_listener_timeout_does_not_poison_next_request() {
    const DEVICE_SK: [u8; 32] = [3u8; 32];
    let registry = registry_with_device();
    let dir = std::env::temp_dir().join(format!(
        "ra_daemon_timeout_recover_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let sock = dir.join("device.sock");
    let dev_kp = crate::remote::generate_static_keypair().unwrap();
    let dmn_kp = crate::remote::generate_static_keypair().unwrap();
    let handle = spawn_device_listener(sock.clone(), dmn_kp.private).unwrap();

    let timeout = decide_with_remote_listener(
        &registry,
        &handle,
        RemoteGateRun {
            command: "chmod -R 777 .",
            armed: true,
            allow_high: true,
            now: 100,
            ttl: 60,
            device_id: None,
            issued_context_hash: "ctx",
            context_origin: None,
            current_context_hash: Some("ctx"),
            response_timeout: std::time::Duration::from_millis(50),
        },
    );
    assert!(!timeout.is_allow(), "{timeout:?}");

    std::thread::sleep(std::time::Duration::from_millis(150));
    let sock_for_device = sock.clone();
    let device_thread = std::thread::spawn(move || {
        crate::session::run_device_connect(&sock_for_device, &dev_kp.private, &DEVICE_SK, true)
            .unwrap()
    });
    let recovered = decide_with_remote_listener(
        &registry,
        &handle,
        RemoteGateRun {
            command: "chmod -R 777 .",
            armed: true,
            allow_high: true,
            now: 200,
            ttl: 60,
            device_id: None,
            issued_context_hash: "ctx",
            context_origin: None,
            current_context_hash: Some("ctx"),
            response_timeout: std::time::Duration::from_secs(5),
        },
    );
    assert!(recovered.is_allow(), "{recovered:?}");
    let _ = device_thread.join().unwrap();

    drop(handle.request_tx);
    handle.thread.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
