use std::path::PathBuf;

use super::validate::valid_relay_ticket_mac_hex;

use super::*;

#[test]
fn active_product_mode_is_live_loopback() {
    let mode = active_product_mode();
    assert_eq!(mode, CompanionTransportMode::LiveLoopback);
    assert_eq!(mode.id(), "live-loopback");
    assert!(mode.is_product_default());
    assert_eq!(mode.readiness(), CompanionTransportReadiness::Ready);
}

#[test]
fn mode_ids_parse_roundtrip() {
    for mode in all_modes() {
        assert_eq!(mode.id().parse::<CompanionTransportMode>().unwrap(), *mode);
        assert_eq!(mode.to_string(), mode.id());
    }
}

#[test]
fn catalog_keeps_future_modes_non_ready() {
    let planned = [
        CompanionTransportMode::Relay,
        CompanionTransportMode::Tailscale,
        CompanionTransportMode::WebSocket,
    ];
    for mode in planned {
        assert_eq!(mode.readiness(), CompanionTransportReadiness::Planned);
        assert!(!mode.is_product_default());
    }
    assert_eq!(
        CompanionTransportMode::DeviceSocket.readiness(),
        CompanionTransportReadiness::Internal
    );
}

#[test]
fn descriptor_contains_stable_metadata() {
    let descriptor = CompanionTransportMode::Relay.descriptor();
    assert_eq!(descriptor.id, "relay");
    assert_eq!(descriptor.readiness.id(), "planned");
    assert!(descriptor.role.contains("M2"));
}

fn relay_ticket_input(issued_at_ms: u64, expires_at_ms: u64) -> CompanionRelaySessionTicketInput {
    CompanionRelaySessionTicketInput {
        session_id: "relay-ws-session-1".into(),
        session_token: "token_1234567890abcdef1234567890abcdef".into(),
        issued_at_ms,
        expires_at_ms,
        daemon_pubkey_hex: "a".repeat(64),
        companion_device_id: "web-1234abcd".into(),
        companion_noise_pubkey_hex: "b".repeat(64),
        companion_approval_pubkey_hex: "c".repeat(64),
    }
}

fn relay_keyring_path(tag: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "ai_relay_ticket_keyring_{}_{}_{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join(COMPANION_RELAY_TICKET_KEYRING_FILE)
}

#[test]
fn relay_frame_wraps_transport_message_without_mutating_payload() {
    let message = crate::session::CompanionTransportMsg::Ping {
        nonce: "relay-ping-1".into(),
    };
    let frame = CompanionRelayFrame::from_message(
        "relay-session-1",
        CompanionRelayPeer::Companion,
        1,
        10,
        20,
        &message,
    )
    .unwrap();

    assert_eq!(
        frame.relay_protocol_version,
        COMPANION_RELAY_PROTOCOL_VERSION
    );
    assert_eq!(frame.sender.to_string(), "companion");
    assert_eq!(frame.payload_message().unwrap(), message);

    let encoded = serde_json::to_string(&frame).unwrap();
    let decoded: CompanionRelayFrame = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, frame);
}

#[test]
fn relay_frame_rejects_bad_metadata() {
    let payload = r#"{"type":"ping","nonce":"relay-ping-1"}"#;
    assert!(CompanionRelayFrame::new(
        "bad session",
        CompanionRelayPeer::Daemon,
        1,
        10,
        20,
        payload
    )
    .is_err());
    assert!(CompanionRelayFrame::new(
        "relay-session-1",
        CompanionRelayPeer::Daemon,
        0,
        10,
        20,
        payload
    )
    .is_err());
    assert!(CompanionRelayFrame::new(
        "relay-session-1",
        CompanionRelayPeer::Daemon,
        1,
        10,
        10,
        payload
    )
    .is_err());
    assert!(CompanionRelayFrame::new(
        "relay-session-1",
        CompanionRelayPeer::Daemon,
        1,
        10,
        20,
        "x".repeat(MAX_RELAY_PAYLOAD_JSON_BYTES + 1),
    )
    .is_err());
}

#[test]
fn relay_frame_decodes_payload_at_endpoint_boundary() {
    let invalid_payload = r#"{"type":"ping","nonce":""}"#;
    let frame = CompanionRelayFrame::new(
        "relay-session-1",
        CompanionRelayPeer::Daemon,
        1,
        10,
        20,
        invalid_payload,
    )
    .unwrap();

    frame.validate_metadata().unwrap();
    assert!(frame.payload_message().is_err());
}

#[test]
fn relay_route_envelope_exposes_metadata_without_decoding_payload() {
    let invalid_payload = r#"{"type":"ping","nonce":""}"#;
    let frame = CompanionRelayFrame::new(
        "relay-session-1",
        CompanionRelayPeer::Daemon,
        7,
        100,
        200,
        invalid_payload,
    )
    .unwrap();

    let route = frame.route_envelope().unwrap();
    assert_eq!(
        route.relay_protocol_version,
        COMPANION_RELAY_PROTOCOL_VERSION
    );
    assert_eq!(route.session_id, "relay-session-1");
    assert_eq!(route.sender, CompanionRelayPeer::Daemon);
    assert_eq!(route.sequence, 7);
    assert_eq!(route.payload_json_bytes, invalid_payload.len());

    let encoded = serde_json::to_string(&route).unwrap();
    assert!(encoded.contains("payload_json_bytes"));
    assert!(!encoded.contains("payload_json\":"));
    assert!(!encoded.contains("nonce"));
    assert!(frame.payload_message().is_err());
}

#[test]
fn relay_session_id_validation_is_stable_ascii() {
    assert!(valid_relay_session_id("relay-session_1:daemon.web"));
    assert!(!valid_relay_session_id(""));
    assert!(!valid_relay_session_id("relay session"));
    assert!(!valid_relay_session_id(
        &"a".repeat(MAX_RELAY_SESSION_ID_LEN + 1)
    ));
}

#[test]
fn relay_session_ticket_binds_websocket_peers() {
    let ticket = CompanionRelaySessionTicket::websocket(relay_ticket_input(
        1000,
        1000 + DEFAULT_COMPANION_RELAY_SESSION_TTL_MS,
    ))
    .unwrap();
    let daemon = CompanionRelaySessionConnect::daemon(&ticket).unwrap();
    let companion = CompanionRelaySessionConnect::companion(&ticket).unwrap();

    ticket.validate_connect(&daemon, 2000).unwrap();
    ticket.validate_connect(&companion, 2000).unwrap();
    assert!(!ticket.is_expired_at(2000));
    assert!(ticket.is_expired_at(ticket.expires_at_ms));

    let encoded = serde_json::to_string(&companion).unwrap();
    let decoded: CompanionRelaySessionConnect = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, companion);
}

#[test]
fn relay_signed_session_ticket_binds_hmac_payload() {
    let ticket = CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
    let secret = b"relay-ticket-secret-1234567890abcdef";
    let signed = CompanionRelaySignedSessionTicket::hmac_sha256(ticket.clone(), secret).unwrap();

    assert_eq!(
            relay_session_ticket_signing_payload(&ticket).unwrap(),
            concat!(
                "ai-terminal-relay-ticket-v1\n",
                "relay_protocol_version=1\n",
                "transport=websocket\n",
                "session_id=relay-ws-session-1\n",
                "session_token=token_1234567890abcdef1234567890abcdef\n",
                "issued_at_ms=1000\n",
                "expires_at_ms=2000\n",
                "daemon_pubkey_hex=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
                "companion_device_id=web-1234abcd\n",
                "companion_noise_pubkey_hex=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
                "companion_approval_pubkey_hex=cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\n"
            )
        );
    assert_eq!(signed.mac_alg, COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256);
    assert!(valid_relay_ticket_mac_hex(&signed.mac_hex));
    signed.validate_mac(secret).unwrap();

    let companion = CompanionRelaySessionConnect::companion(&ticket).unwrap();
    signed.validate_connect(&companion, 1500, secret).unwrap();

    let encoded = serde_json::to_string(&signed).unwrap();
    let decoded: CompanionRelaySignedSessionTicket = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, signed);
}

#[test]
fn relay_signed_session_ticket_fails_closed_on_tamper_or_bad_secret() {
    let ticket = CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
    let secret = b"relay-ticket-secret-1234567890abcdef";
    let signed = CompanionRelaySignedSessionTicket::hmac_sha256(ticket.clone(), secret).unwrap();

    assert!(CompanionRelaySignedSessionTicket::hmac_sha256(ticket.clone(), b"short").is_err());
    assert!(signed
        .validate_mac(b"wrong-ticket-secret-1234567890abcdef")
        .is_err());

    let wrong_mac = CompanionRelaySignedSessionTicket {
        mac_hex: "0".repeat(64),
        ..signed.clone()
    };
    assert!(wrong_mac.validate_mac(secret).is_err());

    let tampered_ticket = CompanionRelaySignedSessionTicket {
        ticket: CompanionRelaySessionTicket {
            session_token: "tampered_1234567890abcdef1234567890abcdef".into(),
            ..ticket
        },
        ..signed
    };
    assert!(tampered_ticket.validate_mac(secret).is_err());
}

#[test]
fn relay_ticket_issuer_signs_with_active_key_and_validates_connect() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let issuer = CompanionRelayTicketIssuer::hmac_sha256(active_secret.clone(), vec![])
        .expect("issuer should accept active hmac key");

    assert_eq!(issuer.verification_key_count(), 1);
    let signed = issuer
        .issue_websocket_ticket(relay_ticket_input(1000, 2000))
        .unwrap();
    signed.validate_mac(&active_secret).unwrap();

    let companion = CompanionRelaySessionConnect::companion(&signed.ticket).unwrap();
    issuer.validate_connect(&signed, &companion, 1500).unwrap();
    assert!(issuer.validate_connect(&signed, &companion, 2000).is_err());
}

#[test]
fn relay_ticket_issuer_retains_previous_key_and_rejects_retired_key() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let previous_secret = b"relay-ticket-previous-secret-1234567890".to_vec();
    let retired_secret = b"relay-ticket-retired-secret-1234567890".to_vec();
    let previous_ticket =
        CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
    let previous_signed =
        CompanionRelaySignedSessionTicket::hmac_sha256(previous_ticket, &previous_secret).unwrap();
    let retired_signed = CompanionRelaySignedSessionTicket::hmac_sha256(
        CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap(),
        &retired_secret,
    )
    .unwrap();

    let rotated = CompanionRelayTicketIssuer::hmac_sha256(
        active_secret.clone(),
        vec![previous_secret.clone()],
    )
    .unwrap();

    assert_eq!(rotated.verification_key_count(), 2);
    assert!(previous_signed.validate_mac(&active_secret).is_err());
    rotated.validate_ticket(&previous_signed).unwrap();
    assert!(rotated.validate_ticket(&retired_signed).is_err());

    let new_signed = rotated
        .issue_websocket_ticket(relay_ticket_input(2000, 3000))
        .unwrap();
    new_signed.validate_mac(&active_secret).unwrap();
    assert!(new_signed.validate_mac(&previous_secret).is_err());
}

#[test]
fn relay_ticket_issuer_policy_rejects_bad_key_state() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let previous_a = b"relay-ticket-previous-a-secret-1234567890".to_vec();
    let previous_b = b"relay-ticket-previous-b-secret-1234567890".to_vec();
    let previous_c = b"relay-ticket-previous-c-secret-1234567890".to_vec();

    assert!(CompanionRelayTicketIssuer::hmac_sha256(b"short".to_vec(), vec![]).is_err());
    assert!(CompanionRelayTicketIssuer::hmac_sha256(
        active_secret.clone(),
        vec![active_secret.clone()],
    )
    .is_err());
    assert!(CompanionRelayTicketIssuer::hmac_sha256(
        active_secret,
        vec![previous_a, previous_b, previous_c],
    )
    .is_err());
}

#[test]
fn relay_ticket_keyring_persists_and_builds_keyed_issuer() {
    let path = relay_keyring_path("roundtrip");
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let record =
        CompanionRelayTicketKeyringRecord::new("relay-active-1", active_secret.clone(), 1000)
            .unwrap();
    save_companion_relay_ticket_keyring(&path, &record).unwrap();

    let loaded = load_companion_relay_ticket_keyring(&path).unwrap();
    assert_eq!(loaded, record);
    let issuer = loaded.issuer().unwrap();
    assert_eq!(issuer.active_key_id(), Some("relay-active-1"));
    assert_eq!(issuer.verification_key_count(), 1);

    let signed = issuer
        .issue_websocket_ticket(relay_ticket_input(1200, 1800))
        .unwrap();
    assert_eq!(signed.key_id.as_deref(), Some("relay-active-1"));
    signed.validate_mac(&active_secret).unwrap();
    issuer.validate_ticket(&signed).unwrap();

    let encoded = serde_json::to_string(&signed).unwrap();
    assert!(encoded.contains("\"key_id\":\"relay-active-1\""));

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn relay_ticket_key_id_migration_keeps_legacy_tickets_valid() {
    let active_v1 = b"relay-ticket-active-v1-secret-123456".to_vec();
    let active_v2 = b"relay-ticket-active-v2-secret-123456".to_vec();
    let first =
        CompanionRelayTicketKeyringRecord::new("relay-active-v1", active_v1.clone(), 1000).unwrap();
    let rotated = first
        .rotate_hmac_key("relay-active-v2", active_v2.clone(), 2000)
        .unwrap();
    assert_eq!(rotated.active_key_id, "relay-active-v2");
    assert_eq!(rotated.hmac_sha256_keys.len(), 2);
    assert_eq!(
        rotated
            .previous_keys()
            .next()
            .and_then(|key| key.retired_at_ms),
        Some(2000)
    );

    let issuer = rotated.issuer().unwrap();
    let legacy_ticket =
        CompanionRelaySessionTicket::websocket(relay_ticket_input(1200, 1800)).unwrap();
    let legacy_signed =
        CompanionRelaySignedSessionTicket::hmac_sha256(legacy_ticket, &active_v1).unwrap();
    assert_eq!(legacy_signed.key_id, None);
    issuer.validate_ticket(&legacy_signed).unwrap();

    let keyed_previous_ticket =
        CompanionRelaySessionTicket::websocket(relay_ticket_input(1200, 1800)).unwrap();
    let keyed_previous = CompanionRelaySignedSessionTicket::hmac_sha256_with_key_id(
        keyed_previous_ticket,
        &active_v1,
        Some("relay-active-v1".into()),
    )
    .unwrap();
    issuer.validate_ticket(&keyed_previous).unwrap();

    let new_signed = issuer
        .issue_websocket_ticket(relay_ticket_input(2200, 2800))
        .unwrap();
    assert_eq!(new_signed.key_id.as_deref(), Some("relay-active-v2"));
    new_signed.validate_mac(&active_v2).unwrap();
    assert!(new_signed.validate_mac(&active_v1).is_err());

    let bad_key_id = CompanionRelaySignedSessionTicket {
        key_id: Some("relay-missing".into()),
        ..new_signed
    };
    assert!(issuer.validate_ticket(&bad_key_id).is_err());
}

#[test]
fn relay_ticket_keyring_load_or_create_is_stable() {
    let path = relay_keyring_path("create");
    let first = load_or_create_companion_relay_ticket_keyring(&path).unwrap();
    first.validate().unwrap();
    assert_eq!(first.version, COMPANION_RELAY_TICKET_KEYRING_VERSION);
    assert_eq!(first.hmac_sha256_keys.len(), 1);
    assert_eq!(
        first.active_key().unwrap().secret.len(),
        MIN_RELAY_TICKET_HMAC_KEY_BYTES
    );

    let second = load_or_create_companion_relay_ticket_keyring(&path).unwrap();
    assert_eq!(second, first);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn relay_ticket_keyring_policy_rejects_bad_persistent_state() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    assert!(
        CompanionRelayTicketKeyringRecord::new("bad key id", active_secret.clone(), 1000).is_err()
    );
    assert!(
        CompanionRelayTicketKeyringRecord::new("relay-active", active_secret.clone(), 0).is_err()
    );

    let mut duplicate =
        CompanionRelayTicketKeyringRecord::new("relay-active", active_secret.clone(), 1000)
            .unwrap();
    duplicate
        .hmac_sha256_keys
        .push(CompanionRelayTicketHmacKeyRecord {
            key_id: "relay-active".into(),
            secret: b"relay-ticket-previous-secret-1234567890".to_vec(),
            created_at_ms: 1000,
            retired_at_ms: Some(2000),
        });
    assert!(duplicate.validate().is_err());

    let mut missing_retired =
        CompanionRelayTicketKeyringRecord::new("relay-active", active_secret, 1000).unwrap();
    missing_retired
        .hmac_sha256_keys
        .push(CompanionRelayTicketHmacKeyRecord {
            key_id: "relay-previous".into(),
            secret: b"relay-ticket-previous-secret-1234567890".to_vec(),
            created_at_ms: 1000,
            retired_at_ms: None,
        });
    assert!(missing_retired.validate().is_err());
}

fn relay_setup_input() -> CompanionRelaySelfHostedSetupInput {
    CompanionRelaySelfHostedSetupInput {
        deployment_mode: None,
        private_network_name: None,
        relay_endpoint_url: "wss://relay.example.test/session".into(),
        daemon_pubkey: vec![0xaa; 32],
        companion_device_id: "web-1234abcd".into(),
        companion_noise_pubkey: vec![0xbb; 32],
        companion_approval_pubkey: [0xcc; 32],
        issued_at_ms: 1200,
        ttl_ms: 1000,
        session_id: Some("relay-runtime-session-1".into()),
        session_token: Some("token_relay_runtime_setup_1234567890abcdef".into()),
    }
}

#[test]
fn relay_self_hosted_runtime_setup_uses_persisted_keyring() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let keyring =
        CompanionRelayTicketKeyringRecord::new("relay-active-1", active_secret.clone(), 1000)
            .unwrap();
    let setup = issue_self_hosted_relay_runtime_setup(&keyring, relay_setup_input()).unwrap();

    assert_eq!(setup.transport_mode, COMPANION_RELAY_SETUP_TRANSPORT_MODE);
    assert_eq!(
        setup.deployment_mode,
        COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED
    );
    assert_eq!(setup.relay_endpoint_url, "wss://relay.example.test/session");
    assert_eq!(
        setup.signed_session_ticket.key_id.as_deref(),
        Some("relay-active-1")
    );
    assert_eq!(setup.signed_session_ticket.ticket.expires_at_ms, 2200);
    assert_eq!(setup.companion_identity.device_id, "web-1234abcd");
    assert_eq!(setup.companion_identity.noise_pubkey_hex, "bb".repeat(32));
    assert_eq!(
        setup.companion_identity.approval_pubkey_hex,
        "cc".repeat(32)
    );
    setup.validate_metadata().unwrap();
    setup
        .signed_session_ticket
        .validate_mac(&active_secret)
        .unwrap();
    keyring
        .issuer()
        .unwrap()
        .validate_connect(&setup.signed_session_ticket, &setup.companion_connect, 1300)
        .unwrap();
    keyring
        .issuer()
        .unwrap()
        .validate_connect(&setup.signed_session_ticket, &setup.daemon_connect, 1300)
        .unwrap();

    let encoded = serde_json::to_string(&setup).unwrap();
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("hmac_sha256_keys"));
    assert!(encoded.contains("\"signedSessionTicket\""));
    assert!(encoded.contains("\"relayEndpointUrl\""));
    assert!(encoded.contains("\"deviceId\":\"web-1234abcd\""));
    assert!(encoded.contains("\"key_id\":\"relay-active-1\""));
}

#[test]
fn relay_private_network_runtime_setup_emits_guarded_contract() {
    let active_secret = b"relay-ticket-active-secret-1234567890".to_vec();
    let keyring =
        CompanionRelayTicketKeyringRecord::new("relay-active-1", active_secret.clone(), 1000)
            .unwrap();
    let mut input = relay_setup_input();
    input.deployment_mode = Some(COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK.into());
    input.private_network_name = Some("tailnet-dev".into());
    input.relay_endpoint_url = "wss://relay.tailnet.example/relay".into();
    let setup = issue_self_hosted_relay_runtime_setup(&keyring, input).unwrap();

    assert_eq!(
        setup.deployment_mode,
        COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK
    );
    assert_eq!(setup.private_network_name.as_deref(), Some("tailnet-dev"));
    assert_eq!(
        setup.relay_endpoint_url,
        "wss://relay.tailnet.example/relay"
    );
    assert!(setup
        .operator_setup_text
        .contains("Private-network relay tailnet-dev endpoint"));
    setup.validate_metadata().unwrap();
    setup
        .signed_session_ticket
        .validate_mac(&active_secret)
        .unwrap();

    let encoded = serde_json::to_string(&setup).unwrap();
    assert!(encoded.contains("\"deploymentMode\":\"private-network\""));
    assert!(encoded.contains("\"privateNetworkName\":\"tailnet-dev\""));
    assert!(!encoded.contains("secret"));

    let mut public_ws = relay_setup_input();
    public_ws.deployment_mode = Some(COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK.into());
    public_ws.private_network_name = Some("tailnet-dev".into());
    public_ws.relay_endpoint_url = "ws://relay.example.test/relay".into();
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, public_ws).is_err());

    let mut missing_name = relay_setup_input();
    missing_name.deployment_mode = Some(COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK.into());
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, missing_name).is_err());

    let mut self_hosted_with_name = relay_setup_input();
    self_hosted_with_name.private_network_name = Some("tailnet-dev".into());
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, self_hosted_with_name).is_err());
}

#[test]
fn relay_self_hosted_runtime_setup_rejects_bad_inputs() {
    let keyring = CompanionRelayTicketKeyringRecord::new(
        "relay-active-1",
        b"relay-ticket-active-secret-1234567890".to_vec(),
        1000,
    )
    .unwrap();

    let mut bad_endpoint = relay_setup_input();
    bad_endpoint.relay_endpoint_url = "https://relay.example.test/session".into();
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, bad_endpoint).is_err());

    let mut bad_ttl = relay_setup_input();
    bad_ttl.ttl_ms = DEFAULT_COMPANION_RELAY_SESSION_TTL_MS + 1;
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, bad_ttl).is_err());

    let mut bad_identity = relay_setup_input();
    bad_identity.companion_device_id = "bad device".into();
    assert!(issue_self_hosted_relay_runtime_setup(&keyring, bad_identity).is_err());
}

#[test]
fn relay_session_ticket_fails_closed_on_mismatch_or_expiry() {
    let ticket = CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
    let daemon = CompanionRelaySessionConnect::daemon(&ticket).unwrap();
    let companion = CompanionRelaySessionConnect::companion(&ticket).unwrap();

    assert!(ticket.validate_connect(&daemon, 2000).is_err());
    assert!(ticket.validate_connect(&companion, 2000).is_err());

    let wrong_token = CompanionRelaySessionConnect {
        session_token: "wrong_1234567890abcdef1234567890abcdef".into(),
        ..companion.clone()
    };
    assert!(ticket.validate_connect(&wrong_token, 1500).is_err());

    let wrong_device = CompanionRelaySessionConnect {
        device_id: Some("web-other".into()),
        ..companion
    };
    assert!(ticket.validate_connect(&wrong_device, 1500).is_err());

    let wrong_daemon = CompanionRelaySessionConnect {
        daemon_pubkey_hex: Some("d".repeat(64)),
        ..daemon
    };
    assert!(ticket.validate_connect(&wrong_daemon, 1500).is_err());
}

#[test]
fn relay_session_ticket_rejects_bad_metadata() {
    assert!(!valid_relay_session_token("short"));
    assert!(valid_relay_session_token(
        "token_1234567890abcdef1234567890abcdef"
    ));
    let mut bad_session = relay_ticket_input(1000, 2000);
    bad_session.session_id = "bad session".into();
    assert!(CompanionRelaySessionTicket::websocket(bad_session).is_err());

    let mut bad_token = relay_ticket_input(1000, 2000);
    bad_token.session_token = "short".into();
    assert!(CompanionRelaySessionTicket::websocket(bad_token).is_err());

    let long_ttl = relay_ticket_input(1000, 2000 + DEFAULT_COMPANION_RELAY_SESSION_TTL_MS);
    assert!(CompanionRelaySessionTicket::websocket(long_ttl).is_err());

    let mut bad_pubkey = relay_ticket_input(1000, 2000);
    bad_pubkey.daemon_pubkey_hex = "not-hex".into();
    assert!(CompanionRelaySessionTicket::websocket(bad_pubkey).is_err());
}

#[test]
fn relay_loopback_routes_frames_by_session_and_recipient() {
    let mut relay = CompanionRelayLoopback::new();
    let ping = crate::session::CompanionTransportMsg::Ping {
        nonce: "daemon-to-companion".into(),
    };
    let pong = crate::session::CompanionTransportMsg::Pong {
        nonce: "companion-to-daemon".into(),
    };

    relay
        .enqueue(
            CompanionRelayFrame::from_message(
                "session-a",
                CompanionRelayPeer::Daemon,
                1,
                10,
                100,
                &ping,
            )
            .unwrap(),
        )
        .unwrap();
    relay
        .enqueue(
            CompanionRelayFrame::from_message(
                "session-a",
                CompanionRelayPeer::Companion,
                1,
                11,
                100,
                &pong,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(
        relay
            .queued_for("session-a", CompanionRelayPeer::Companion)
            .unwrap(),
        1
    );
    assert_eq!(
        relay
            .queued_for("session-a", CompanionRelayPeer::Daemon)
            .unwrap(),
        1
    );
    assert_eq!(relay.stats().queued_frames, 2);

    let companion_frame = relay
        .dequeue("session-a", CompanionRelayPeer::Companion, 20)
        .unwrap()
        .unwrap();
    assert_eq!(companion_frame.payload_message().unwrap(), ping);

    let daemon_frame = relay
        .dequeue("session-a", CompanionRelayPeer::Daemon, 20)
        .unwrap()
        .unwrap();
    assert_eq!(daemon_frame.payload_message().unwrap(), pong);
    assert_eq!(relay.stats().session_count, 0);
}

#[test]
fn relay_loopback_keeps_sessions_isolated() {
    let mut relay = CompanionRelayLoopback::new();
    let message = crate::session::CompanionTransportMsg::Ping {
        nonce: "session-isolation".into(),
    };
    for (session_id, sequence) in [("session-a", 1), ("session-b", 1)] {
        relay
            .enqueue(
                CompanionRelayFrame::from_message(
                    session_id,
                    CompanionRelayPeer::Daemon,
                    sequence,
                    10,
                    100,
                    &message,
                )
                .unwrap(),
            )
            .unwrap();
    }

    assert!(relay
        .dequeue("session-b", CompanionRelayPeer::Companion, 20)
        .unwrap()
        .is_some());
    assert_eq!(
        relay
            .queued_for("session-a", CompanionRelayPeer::Companion)
            .unwrap(),
        1
    );
    assert_eq!(
        relay
            .queued_for("session-b", CompanionRelayPeer::Companion)
            .unwrap(),
        0
    );
}

#[test]
fn relay_loopback_rejects_duplicate_sender_sequence() {
    let mut relay = CompanionRelayLoopback::new();
    let message = crate::session::CompanionTransportMsg::Ping {
        nonce: "sequence".into(),
    };
    let frame = |sequence| {
        CompanionRelayFrame::from_message(
            "session-a",
            CompanionRelayPeer::Daemon,
            sequence,
            10 + sequence,
            100 + sequence,
            &message,
        )
        .unwrap()
    };

    relay.enqueue(frame(2)).unwrap();
    assert!(relay.enqueue(frame(2)).is_err());
    assert!(relay.enqueue(frame(1)).is_err());
    relay.enqueue(frame(3)).unwrap();
    assert_eq!(
        relay
            .queued_for("session-a", CompanionRelayPeer::Companion)
            .unwrap(),
        2
    );
}

#[test]
fn relay_loopback_drops_expired_frames_on_dequeue() {
    let mut relay = CompanionRelayLoopback::new();
    let expired = crate::session::CompanionTransportMsg::Ping {
        nonce: "expired".into(),
    };
    let fresh = crate::session::CompanionTransportMsg::Ping {
        nonce: "fresh".into(),
    };

    relay
        .enqueue(
            CompanionRelayFrame::from_message(
                "session-a",
                CompanionRelayPeer::Daemon,
                1,
                10,
                20,
                &expired,
            )
            .unwrap(),
        )
        .unwrap();
    relay
        .enqueue(
            CompanionRelayFrame::from_message(
                "session-a",
                CompanionRelayPeer::Daemon,
                2,
                21,
                100,
                &fresh,
            )
            .unwrap(),
        )
        .unwrap();

    let frame = relay
        .dequeue("session-a", CompanionRelayPeer::Companion, 30)
        .unwrap()
        .unwrap();
    assert_eq!(frame.payload_message().unwrap(), fresh);
    assert_eq!(relay.stats().queued_frames, 0);
}

#[test]
fn relay_endpoint_exchanges_ping_pong_messages() {
    let mut relay = CompanionRelayLoopback::new();
    let mut daemon = CompanionRelayEndpoint::daemon("session-a").unwrap();
    let mut companion = CompanionRelayEndpoint::companion("session-a").unwrap();
    let ping = crate::session::CompanionTransportMsg::Ping { nonce: "p1".into() };
    let pong = crate::session::CompanionTransportMsg::Pong { nonce: "p1".into() };

    assert_eq!(daemon.send_message(&mut relay, 100, &ping).unwrap(), 1);
    assert_eq!(daemon.next_sequence(), 2);
    assert_eq!(
        companion.recv_message(&mut relay, 101).unwrap().unwrap(),
        ping
    );
    assert!(daemon.recv_message(&mut relay, 102).unwrap().is_none());

    assert_eq!(companion.send_message(&mut relay, 103, &pong).unwrap(), 1);
    assert_eq!(daemon.recv_message(&mut relay, 104).unwrap().unwrap(), pong);
    assert_eq!(relay.stats().queued_frames, 0);
}

#[test]
fn relay_endpoint_preserves_approval_payloads() {
    let mut relay = CompanionRelayLoopback::new();
    let mut daemon = CompanionRelayEndpoint::daemon("approval-session").unwrap();
    let mut companion = CompanionRelayEndpoint::companion("approval-session").unwrap();
    let request = crate::session::ApprovalRequestMsg {
        approval_id: b"appr-1".to_vec(),
        nonce: vec![7u8; 32],
        command_masked: "rm -rf /data".into(),
        context_hash: "ctx".into(),
        expires_at: 9999,
        device_epoch: 1,
    };
    let request_message = crate::session::CompanionTransportMsg::ApprovalRequest {
        request: request.clone(),
    };

    daemon
        .send_message(&mut relay, 100, &request_message)
        .unwrap();
    assert_eq!(
        companion.recv_message(&mut relay, 101).unwrap().unwrap(),
        request_message
    );

    let response = crate::session::ApprovalResponseMsg {
        approval_id: request.approval_id,
        nonce: request.nonce,
        approve: false,
        sig: vec![3u8; 64],
    };
    let response_message = crate::session::CompanionTransportMsg::ApprovalResponse { response };
    companion
        .send_message(&mut relay, 102, &response_message)
        .unwrap();
    assert_eq!(
        daemon.recv_message(&mut relay, 103).unwrap().unwrap(),
        response_message
    );
}

#[test]
fn relay_endpoint_rejects_bad_session_and_ttl() {
    assert!(CompanionRelayEndpoint::new("bad session", CompanionRelayPeer::Daemon).is_err());
    assert!(
        CompanionRelayEndpoint::with_frame_ttl("session-a", CompanionRelayPeer::Daemon, 0).is_err()
    );
}

#[test]
fn relay_endpoint_applies_frame_ttl() {
    let mut relay = CompanionRelayLoopback::new();
    let mut daemon =
        CompanionRelayEndpoint::with_frame_ttl("session-a", CompanionRelayPeer::Daemon, 5).unwrap();
    let companion = CompanionRelayEndpoint::companion("session-a").unwrap();
    let message = crate::session::CompanionTransportMsg::Ping {
        nonce: "expires".into(),
    };

    daemon.send_message(&mut relay, 100, &message).unwrap();
    assert!(companion.recv_message(&mut relay, 105).unwrap().is_none());
    assert_eq!(relay.stats().queued_frames, 0);
}
