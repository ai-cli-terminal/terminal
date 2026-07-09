/// `ai __gate` 본체. armed 상태를 읽어 게이트 결정 → exit code 반환.
/// armed면 데몬(Unix 소켓)에 질의하고, 데몬 도달 불가 시 로컬 `decide_gate`로 폴백한다
/// (데몬 다운은 보안 경계가 아니라 자기-가드레일 — DESIGN Threat Model). armed 경로
/// 접근 실패는 fail-closed(차단=1).
pub(crate) fn run_gate(command: &str) -> i32 {
    use ai_terminal::gate::{self, GateDecision};

    let path = match gate::armed_path() {
        Ok(p) => p,
        Err(_) => return 1, // 경로 불명 = fail-closed
    };
    let (armed, allow_high) = match gate::load_arm_state(&path) {
        Some(st) => (true, st.allow_high),
        None => return 0, // 비-armed: 게이트 미개입(hot-path)
    };

    // armed: 데몬에 질의(unix). 도달 불가 시 로컬 결정으로 폴백.
    #[cfg(unix)]
    {
        use ai_terminal::daemon;
        if let Ok(sock) = daemon::socket_path() {
            let req = daemon::GateRequest {
                command: command.to_string(),
                context_origin: Some(ai_terminal::context::RemoteContextOrigin::gather()),
            };
            if let Ok(reply) = daemon::query_with_context(&sock, &req) {
                if reply.is_allow() {
                    return 0;
                }
                eprintln!("AI 게이트 차단(데몬): {}", reply.reason);
                return 1;
            }
        }
    }

    match gate::decide_gate(command, armed, allow_high) {
        GateDecision::Allow => 0,
        GateDecision::Block { reason } => {
            eprintln!("AI 게이트 차단: {reason}");
            1
        }
    }
}

/// `ai remote daemon` 본체. Unix 소켓 게이트 데몬을 포그라운드 실행한다(Ctrl-C 종료).
pub(crate) fn run_gate_daemon(
    device_id: Option<String>,
    transport: String,
    relay_endpoint_url: Option<String>,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    relay_ttl_seconds: u64,
) -> anyhow::Result<()> {
    #[cfg(any(not(feature = "remote"), not(unix)))]
    let _ = (
        &device_id,
        &transport,
        &relay_endpoint_url,
        &relay_deployment_mode,
        &private_network_name,
        relay_ttl_seconds,
    );

    #[cfg(unix)]
    {
        use ai_terminal::daemon;
        let sock = daemon::socket_path()?;
        println!("원격 게이트 데몬 시작: {} (Ctrl-C 종료)", sock.display());
        let rt = tokio::runtime::Runtime::new()?;
        #[cfg(feature = "remote")]
        {
            let daemon_transport = resolve_daemon_transport_selection(
                &transport,
                relay_endpoint_url.as_deref(),
                &relay_deployment_mode,
                private_network_name.as_deref(),
                relay_ttl_seconds,
            )?;
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            match device_id.as_deref() {
                Some(id) => {
                    registry.select_device(Some(id))?;
                    println!("원격 승인 디바이스 선택: {id}");
                }
                None if registry.devices.len() == 1 => {
                    println!("원격 승인 디바이스 선택: {}", registry.devices[0].id);
                }
                None => {
                    println!(
                        "원격 승인 디바이스 선택: <ambiguous> (registered={}, 필요 시 --device-id 사용)",
                        registry.devices.len()
                    );
                }
            }
            let transport_mode = ai_terminal::remote_transport::active_product_mode();
            let relay_keyring_path =
                ai_terminal::remote_transport::companion_relay_ticket_keyring_path()?;
            let relay_keyring =
                ai_terminal::remote_transport::load_or_create_companion_relay_ticket_keyring(
                    &relay_keyring_path,
                )?;
            let relay_issuer = relay_keyring.issuer()?;
            let relay_setup = if daemon_transport.mode
                == ai_terminal::remote_transport::CompanionTransportMode::Relay
            {
                let device = registry.select_device(device_id.as_deref())?;
                let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
                let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
                let setup = ai_terminal::remote_transport::issue_self_hosted_relay_runtime_setup(
                    &relay_keyring,
                    ai_terminal::remote_transport::CompanionRelaySelfHostedSetupInput {
                        relay_endpoint_url: daemon_transport
                            .relay_endpoint_url
                            .clone()
                            .expect("relay endpoint URL must be validated for relay mode"),
                        deployment_mode: Some(daemon_transport.relay_deployment_mode.clone()),
                        private_network_name: daemon_transport.private_network_name.clone(),
                        daemon_pubkey: daemon_key.public.clone(),
                        companion_device_id: device.id.clone(),
                        companion_noise_pubkey: device.noise_pubkey.clone(),
                        companion_approval_pubkey: device.approval_pubkey,
                        issued_at_ms: ai_terminal::pairing::now_ms(),
                        ttl_ms: daemon_transport.relay_ttl_ms,
                        session_id: None,
                        session_token: None,
                    },
                )?;
                Some((daemon_key_path, setup))
            } else {
                None
            };
            println!("PWA transport mode : {}", transport_mode.id());
            println!("PWA requested transport: {}", daemon_transport.mode.id());
            println!("PWA relay keyring  : {}", relay_keyring_path.display());
            println!(
                "PWA relay ticket key: {}",
                relay_issuer.active_key_id().unwrap_or("<legacy>")
            );
            if let Some((daemon_key_path, setup)) = relay_setup {
                let mut relay_runtime = daemon::CompanionRelayDaemonRuntime::new(setup.clone())?;
                relay_runtime.register_session(std::time::Duration::from_secs(5))?;
                println!("PWA relay runtime  : enabled");
                println!("PWA relay daemon key: {}", daemon_key_path.display());
                println!("PWA relay deployment: {}", setup.deployment_mode);
                if let Some(private_network_name) = setup.private_network_name.as_deref() {
                    println!("PWA private network: {}", private_network_name);
                }
                println!("PWA relay endpoint : {}", setup.relay_endpoint_url);
                println!(
                    "PWA relay device   : {}",
                    setup.companion_identity.device_id
                );
                println!(
                    "PWA relay expires  : {}",
                    setup.signed_session_ticket.ticket.expires_at_ms
                );
                println!("PWA relay ticket registered: true");
                println!("PWA relay setup json: {}", serde_json::to_string(&setup)?);
                rt.block_on(daemon::serve_with_remote_relay(
                    &sock,
                    registry,
                    relay_runtime,
                    device_id,
                ))
            } else {
                println!("PWA relay runtime  : disabled");
                let live_endpoint =
                    daemon::spawn_companion_live_endpoint(registry.clone(), device_id.clone())?;
                println!("PWA live endpoint  : {}", live_endpoint.base_url);
                println!("PWA message endpoint: {}", live_endpoint.message_url);
                println!("PWA events endpoint : {}", live_endpoint.events_url);
                let listener = live_endpoint.listener;
                rt.block_on(daemon::serve_with_remote(
                    &sock, registry, listener, device_id,
                ))
            }
        }
        #[cfg(not(feature = "remote"))]
        {
            rt.block_on(daemon::serve(&sock))
        }
    }
    #[cfg(not(unix))]
    {
        println!("게이트 데몬은 Unix 전용입니다.");
        Ok(())
    }
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaemonTransportSelection {
    mode: ai_terminal::remote_transport::CompanionTransportMode,
    relay_endpoint_url: Option<String>,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    relay_ttl_ms: u64,
}

#[cfg(feature = "remote")]
pub(crate) fn resolve_daemon_transport_selection(
    transport: &str,
    relay_endpoint_url: Option<&str>,
    relay_deployment_mode: &str,
    private_network_name: Option<&str>,
    relay_ttl_seconds: u64,
) -> anyhow::Result<DaemonTransportSelection> {
    use std::str::FromStr;

    let mode = ai_terminal::remote_transport::CompanionTransportMode::from_str(transport)
        .map_err(anyhow::Error::msg)?;
    match mode {
        ai_terminal::remote_transport::CompanionTransportMode::LiveLoopback => {
            if relay_endpoint_url.is_some() {
                anyhow::bail!("--relay-endpoint-url은 --transport relay에서만 사용할 수 있습니다");
            }
            if relay_deployment_mode
                != ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED
            {
                anyhow::bail!(
                    "--relay-deployment-mode은 --transport relay에서만 self-hosted 외 값을 사용할 수 있습니다"
                );
            }
            if private_network_name
                .map(str::trim)
                .is_some_and(|name| !name.is_empty())
            {
                anyhow::bail!(
                    "--private-network-name은 --transport relay에서만 사용할 수 있습니다"
                );
            }
            Ok(DaemonTransportSelection {
                mode,
                relay_endpoint_url: None,
                relay_deployment_mode:
                    ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED
                        .into(),
                private_network_name: None,
                relay_ttl_ms: 0,
            })
        }
        ai_terminal::remote_transport::CompanionTransportMode::Relay => {
            if relay_ttl_seconds == 0 {
                anyhow::bail!("relay setup TTL은 1초 이상이어야 합니다");
            }
            let relay_ttl_ms = relay_ttl_seconds
                .checked_mul(1000)
                .ok_or_else(|| anyhow::anyhow!("relay setup TTL overflow"))?;
            let relay_endpoint_url = relay_endpoint_url
                .filter(|url| !url.trim().is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("--transport relay에는 --relay-endpoint-url이 필요합니다")
                })?
                .to_string();
            if !ai_terminal::remote_transport::valid_relay_websocket_endpoint_url(
                &relay_endpoint_url,
            ) {
                anyhow::bail!("relay endpoint URL must be wss:// or localhost ws://");
            }
            let (relay_deployment_mode, private_network_name) =
                resolve_relay_deployment_selection(relay_deployment_mode, private_network_name)?;
            Ok(DaemonTransportSelection {
                mode,
                relay_endpoint_url: Some(relay_endpoint_url),
                relay_deployment_mode,
                private_network_name,
                relay_ttl_ms,
            })
        }
        ai_terminal::remote_transport::CompanionTransportMode::DeviceSocket
        | ai_terminal::remote_transport::CompanionTransportMode::Tailscale
        | ai_terminal::remote_transport::CompanionTransportMode::WebSocket => {
            anyhow::bail!("{mode} transport는 daemon runtime에서 아직 선택할 수 없습니다")
        }
    }
}

#[cfg(feature = "remote")]
pub(crate) fn resolve_relay_deployment_selection(
    relay_deployment_mode: &str,
    private_network_name: Option<&str>,
) -> anyhow::Result<(String, Option<String>)> {
    match relay_deployment_mode {
        ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED => {
            if private_network_name
                .map(str::trim)
                .is_some_and(|name| !name.is_empty())
            {
                anyhow::bail!(
                    "--private-network-name은 --relay-deployment-mode private-network에서만 사용할 수 있습니다"
                );
            }
            Ok((
                ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED.into(),
                None,
            ))
        }
        ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK => {
            let name = private_network_name
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "--relay-deployment-mode private-network에는 --private-network-name이 필요합니다"
                    )
                })?;
            if !ai_terminal::remote_transport::valid_relay_private_network_name(name) {
                anyhow::bail!("private-network relay name format error");
            }
            Ok((
                ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK
                    .into(),
                Some(name.to_string()),
            ))
        }
        _ => anyhow::bail!("relay deployment mode는 self-hosted 또는 private-network여야 합니다"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "remote")]
    #[test]
    fn daemon_transport_selection_validates_relay_inputs() {
        let live =
            resolve_daemon_transport_selection("live-loopback", None, "self-hosted", None, 300)
                .unwrap();
        assert_eq!(
            live.mode,
            ai_terminal::remote_transport::CompanionTransportMode::LiveLoopback
        );
        assert!(live.relay_endpoint_url.is_none());
        assert_eq!(live.relay_deployment_mode, "self-hosted");
        assert!(live.private_network_name.is_none());

        let relay = resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .unwrap();
        assert_eq!(
            relay.mode,
            ai_terminal::remote_transport::CompanionTransportMode::Relay
        );
        assert_eq!(
            relay.relay_endpoint_url.as_deref(),
            Some("wss://relay.example.test/session")
        );
        assert_eq!(relay.relay_deployment_mode, "self-hosted");
        assert!(relay.private_network_name.is_none());
        assert_eq!(relay.relay_ttl_ms, 120_000);

        let private_network = resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.tailnet.example/relay"),
            "private-network",
            Some("tailnet-dev"),
            120,
        )
        .unwrap();
        assert_eq!(private_network.relay_deployment_mode, "private-network");
        assert_eq!(
            private_network.private_network_name.as_deref(),
            Some("tailnet-dev")
        );

        assert!(
            resolve_daemon_transport_selection("relay", None, "self-hosted", None, 120).is_err()
        );
        assert!(resolve_daemon_transport_selection(
            "relay",
            Some("ws://relay.example.test/relay"),
            "private-network",
            Some("tailnet-dev"),
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.tailnet.example/relay"),
            "private-network",
            None,
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "live-loopback",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "websocket",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .is_err());
    }
}
