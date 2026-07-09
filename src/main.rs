//! AI CLI 통합 리눅스 터미널 — `ai` 진입점 (스켈레톤).
//!
//! 설계 정본: `../document/`(v3.3). 본 골격은 M0 부트스트랩 단계의 최소 구현으로,
//! CLI 표면(§9, §31)을 잡아두고 이후 마일스톤(M1~M4)에서 채워 넣는다.
//!
//! 불변식(자세히는 `docs/RULES.md`):
//! - AI 기능 장애가 일반 셸 사용을 막아서는 안 된다(§3-3).
//! - AI 생성 명령은 자동 실행하지 않는다(§3-11).
//! - 로컬 정책/위험도 평가가 먼저 수행된다(§3-9).

mod cli;

use std::path::PathBuf;

use ai_terminal::config;
use ai_terminal::context;
use ai_terminal::dispatch;
use ai_terminal::gateway;
use ai_terminal::guardrails;
use ai_terminal::index;
use ai_terminal::intent;
use ai_terminal::mcp;
use ai_terminal::planner;
use ai_terminal::shell;
use ai_terminal::skill;
use ai_terminal::ui;
use ai_terminal::undo;
#[cfg(feature = "storage")]
use ai_terminal::usage;
use ai_terminal::verify_agent;
use clap::Parser;
use cli::command::{Cli, Command, InitMode, InitTarget, PolicyAction, RemoteAction};
use cli::hooks::{plan_init_shell, resolve_rc, resolve_shell};
#[cfg(feature = "storage")]
use cli::hooks::{record_hook_chpwd, record_hook_precmd, record_hook_preexec};
use cli::inspect::{
    describe_profile, format_mask, format_preview, format_risk, resolve_profile, run_explain,
};
use cli::io::{AutoYes, StdinConfirmer, StdoutSink};

/// `ai __gate` 본체. armed 상태를 읽어 게이트 결정 → exit code 반환.
/// armed면 데몬(Unix 소켓)에 질의하고, 데몬 도달 불가 시 로컬 `decide_gate`로 폴백한다
/// (데몬 다운은 보안 경계가 아니라 자기-가드레일 — DESIGN Threat Model). armed 경로
/// 접근 실패는 fail-closed(차단=1).
fn run_gate(command: &str) -> i32 {
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
fn run_gate_daemon(
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
struct DaemonTransportSelection {
    mode: ai_terminal::remote_transport::CompanionTransportMode,
    relay_endpoint_url: Option<String>,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    relay_ttl_ms: u64,
}

#[cfg(feature = "remote")]
fn resolve_daemon_transport_selection(
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
fn resolve_relay_deployment_selection(
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

#[cfg(all(feature = "remote", unix))]
fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok(format!(
        "unix://{}",
        ai_terminal::daemon::device_socket_path()?.display()
    ))
}

#[cfg(all(feature = "remote", not(unix)))]
fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok("unsupported://local-daemon-unavailable".into())
}

#[cfg(feature = "remote")]
fn run_remote_devices() -> anyhow::Result<()> {
    let registry_path = ai_terminal::device_registry::registry_path()?;
    let registry = ai_terminal::device_registry::DeviceRegistry::load(&registry_path)?;
    println!("등록 원격 디바이스");
    println!("registry_file       : {}", registry_path.display());
    println!("count               : {}", registry.devices.len());
    if registry.devices.is_empty() {
        println!("(none)");
        return Ok(());
    }

    for device in &registry.devices {
        println!("- device_id         : {}", device.id);
        println!("  epoch             : {}", device.epoch);
        println!("  paired_at_ms      : {}", device.paired_at_ms);
        println!(
            "  noise_pubkey_hex  : {}",
            ai_terminal::pairing::hex_encode(&device.noise_pubkey)
        );
        println!(
            "  approval_pubkey_hex: {}",
            ai_terminal::pairing::hex_encode(&device.approval_pubkey)
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_transport() -> anyhow::Result<()> {
    let active = ai_terminal::remote_transport::active_product_mode();
    println!("원격 companion transport");
    println!("active_product_mode : {}", active.id());
    println!("user_selectable     : false");
    println!("modes:");
    for mode in ai_terminal::remote_transport::all_modes() {
        let descriptor = mode.descriptor();
        let marker = if descriptor.mode.is_product_default() {
            " (active)"
        } else {
            ""
        };
        println!(
            "- {:<13} {:<8} {}{}",
            descriptor.id, descriptor.readiness, descriptor.role, marker
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_relay_setup(
    relay_endpoint_url: String,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    device_id: Option<String>,
    ttl_seconds: u64,
) -> anyhow::Result<()> {
    if ttl_seconds == 0 {
        anyhow::bail!("relay setup TTL은 1초 이상이어야 합니다");
    }
    let ttl_ms = ttl_seconds
        .checked_mul(1000)
        .ok_or_else(|| anyhow::anyhow!("relay setup TTL overflow"))?;
    let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
    let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
    let registry_path = ai_terminal::device_registry::registry_path()?;
    let registry = ai_terminal::device_registry::DeviceRegistry::load(&registry_path)?;
    let device = registry.select_device(device_id.as_deref())?;
    let keyring_path = ai_terminal::remote_transport::companion_relay_ticket_keyring_path()?;
    let keyring = ai_terminal::remote_transport::load_or_create_companion_relay_ticket_keyring(
        &keyring_path,
    )?;
    if !ai_terminal::remote_transport::valid_relay_websocket_endpoint_url(&relay_endpoint_url) {
        anyhow::bail!("relay endpoint URL must be wss:// or localhost ws://");
    }
    let (relay_deployment_mode, private_network_name) = resolve_relay_deployment_selection(
        &relay_deployment_mode,
        private_network_name.as_deref(),
    )?;
    let setup = ai_terminal::remote_transport::issue_self_hosted_relay_runtime_setup(
        &keyring,
        ai_terminal::remote_transport::CompanionRelaySelfHostedSetupInput {
            deployment_mode: Some(relay_deployment_mode),
            private_network_name,
            relay_endpoint_url,
            daemon_pubkey: daemon_key.public.clone(),
            companion_device_id: device.id.clone(),
            companion_noise_pubkey: device.noise_pubkey.clone(),
            companion_approval_pubkey: device.approval_pubkey,
            issued_at_ms: ai_terminal::pairing::now_ms(),
            ttl_ms,
            session_id: None,
            session_token: None,
        },
    )?;

    println!("원격 relay self-hosted setup");
    println!("registry_file       : {}", registry_path.display());
    println!("daemon_key_file     : {}", daemon_key_path.display());
    println!("relay_keyring_file  : {}", keyring_path.display());
    println!("transport_mode      : {}", setup.transport_mode);
    println!("deployment_mode     : {}", setup.deployment_mode);
    if let Some(private_network_name) = setup.private_network_name.as_deref() {
        println!("private_network_name: {}", private_network_name);
    }
    println!("relay_endpoint_url  : {}", setup.relay_endpoint_url);
    println!(
        "device_id           : {}",
        setup.companion_identity.device_id
    );
    println!(
        "relay_ticket_key_id : {}",
        setup
            .signed_session_ticket
            .key_id
            .as_deref()
            .unwrap_or("<legacy>")
    );
    println!(
        "expires_at_ms       : {}",
        setup.signed_session_ticket.ticket.expires_at_ms
    );
    println!("relay_setup_json    : {}", serde_json::to_string(&setup)?);
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_pair(
    device_id: Option<String>,
    code: Option<String>,
    noise_pubkey_hex: Option<String>,
    approval_pubkey_hex: Option<String>,
    ttl_seconds: u64,
    pwa_url: Option<String>,
) -> anyhow::Result<()> {
    let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
    let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
    let pairing_path = ai_terminal::pairing::pairing_path()?;
    let registry_path = ai_terminal::device_registry::registry_path()?;

    match (device_id, code, noise_pubkey_hex, approval_pubkey_hex) {
        (None, None, None, None) => {
            let session =
                ai_terminal::pairing::start_pairing(&pairing_path, &daemon_key, ttl_seconds)?;
            let transport_addr = remote_pair_transport_addr()?;
            let payload = ai_terminal::pairing::pairing_payload(&session, &transport_addr);
            println!("원격 디바이스 페어링 시작");
            println!("code              : {}", session.code);
            println!(
                "daemon_pubkey_hex : {}",
                ai_terminal::pairing::hex_encode(&session.daemon_pubkey)
            );
            println!("transport_addr    : {}", transport_addr);
            println!("expires_at_ms     : {}", session.expires_at_ms);
            println!("pairing_file      : {}", pairing_path.display());
            println!(
                "pair_payload_json : {}",
                ai_terminal::pairing::pairing_payload_json(&payload)?
            );
            println!(
                "pair_url          : {}",
                ai_terminal::pairing::pairing_url(&payload)?
            );
            println!("pair_qr:");
            println!("{}", ai_terminal::pairing::pairing_qr_text(&payload)?);
            if let Some(pwa_url) = pwa_url {
                println!(
                    "pwa_pair_url      : {}",
                    ai_terminal::pairing::pairing_pwa_url(&payload, &pwa_url)?
                );
                println!("pwa_pair_qr:");
                println!(
                    "{}",
                    ai_terminal::pairing::pairing_pwa_qr_text(&payload, &pwa_url)?
                );
            }
        }
        (Some(device_id), Some(code), Some(noise_pubkey_hex), Some(approval_pubkey_hex)) => {
            let noise_pubkey = ai_terminal::pairing::hex_decode(&noise_pubkey_hex)?;
            let approval_pubkey = ai_terminal::pairing::hex_decode_32(&approval_pubkey_hex)?;
            let registered = ai_terminal::pairing::complete_pairing(
                &pairing_path,
                &registry_path,
                &device_id,
                &code,
                noise_pubkey,
                approval_pubkey,
            )?;
            println!("원격 디바이스 등록 완료");
            println!("device_id         : {}", registered.id);
            println!("epoch             : {}", registered.epoch);
            println!("registry_file     : {}", registry_path.display());
        }
        _ => {
            anyhow::bail!(
                "pair complete에는 --device-id, --code, --noise-pubkey-hex, --approval-pubkey-hex가 모두 필요합니다"
            );
        }
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_approval_url(request_json: String, pwa_url: Option<String>) -> anyhow::Result<()> {
    let request: ai_terminal::session::ApprovalRequestMsg = serde_json::from_str(&request_json)?;
    println!("원격 승인 요청 URL 생성");
    println!(
        "approval_request_json : {}",
        ai_terminal::session::approval_request_json(&request)?
    );
    println!(
        "approval_url          : {}",
        ai_terminal::session::approval_url(&request)?
    );
    println!("approval_qr:");
    println!("{}", ai_terminal::session::approval_qr_text(&request)?);
    if let Some(pwa_url) = pwa_url {
        println!(
            "pwa_approval_url      : {}",
            ai_terminal::session::approval_pwa_url(&request, &pwa_url)?
        );
        println!("pwa_approval_qr:");
        println!(
            "{}",
            ai_terminal::session::approval_pwa_qr_text(&request, &pwa_url)?
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_approval_verify(
    request_json: String,
    response_json: String,
    device_id: Option<String>,
    approval_pubkey_hex: Option<String>,
    device_epoch: Option<u64>,
    now: Option<u64>,
    context_hash: Option<String>,
) -> anyhow::Result<()> {
    let request: ai_terminal::session::ApprovalRequestMsg = serde_json::from_str(&request_json)?;
    let response: ai_terminal::session::ApprovalResponseMsg = serde_json::from_str(&response_json)?;
    let pending = request.to_pending()?;
    let signed = response.to_signed()?;
    let (device_label, device) =
        remote_approval_verify_device(&request, device_id, approval_pubkey_hex, device_epoch)?;
    let now = now.unwrap_or_else(now_secs);
    let context_hash = context_hash.unwrap_or_else(|| request.context_hash.clone());
    let outcome = ai_terminal::approval::validate(&pending, &device, now, &context_hash, &signed);
    println!("원격 승인 응답 검증");
    println!("device_id        : {device_label}");
    println!("approval_outcome : {outcome:?}");
    Ok(())
}

#[cfg(feature = "remote")]
fn remote_approval_verify_device(
    request: &ai_terminal::session::ApprovalRequestMsg,
    device_id: Option<String>,
    approval_pubkey_hex: Option<String>,
    device_epoch: Option<u64>,
) -> anyhow::Result<(String, ai_terminal::approval::DeviceRecord)> {
    match (device_id, approval_pubkey_hex) {
        (Some(_), Some(_)) => {
            anyhow::bail!("--device-id와 --approval-pubkey-hex는 동시에 지정할 수 없습니다")
        }
        (Some(id), None) => {
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            let device = registry
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("등록되지 않은 디바이스: {id}"))?;
            Ok((device.id.clone(), device.to_approval_record()))
        }
        (None, Some(hex)) => Ok((
            "(direct-pubkey)".into(),
            ai_terminal::approval::DeviceRecord {
                pubkey: ai_terminal::pairing::hex_decode_32(&hex)?,
                epoch: device_epoch.unwrap_or(request.device_epoch),
            },
        )),
        (None, None) => {
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            let device = registry.single_device().ok_or_else(|| {
                anyhow::anyhow!("--device-id 또는 --approval-pubkey-hex가 필요합니다")
            })?;
            Ok((device.id.clone(), device.to_approval_record()))
        }
    }
}

#[cfg(not(feature = "remote"))]
fn run_remote_approval_verify(
    _request_json: String,
    _response_json: String,
    _device_id: Option<String>,
    _approval_pubkey_hex: Option<String>,
    _device_epoch: Option<u64>,
    _now: Option<u64>,
    _context_hash: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote approval-verify`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_approval_url(_request_json: String, _pwa_url: Option<String>) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote approval-url`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_pair(
    _device_id: Option<String>,
    _code: Option<String>,
    _noise_pubkey_hex: Option<String>,
    _approval_pubkey_hex: Option<String>,
    _ttl_seconds: u64,
    _pwa_url: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote pair`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_devices() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote devices`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_transport() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote transport`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_relay_setup(
    _relay_endpoint_url: String,
    _relay_deployment_mode: String,
    _private_network_name: Option<String>,
    _device_id: Option<String>,
    _ttl_seconds: u64,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote relay-setup`은 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(feature = "remote")]
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 현재 콘솔에 attach된 프로세스 수(Windows). 비-Windows·감지 실패 시 None.
#[cfg(windows)]
fn console_process_count() -> Option<u32> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleProcessList(lpdwProcessList: *mut u32, dwProcessCount: u32) -> u32;
    }
    let mut buf = [0u32; 4];
    // SAFETY: 유효한 버퍼 포인터와 길이로 호출. 반환값은 콘솔에 attach된 프로세스 수(0=실패).
    let n = unsafe { GetConsoleProcessList(buf.as_mut_ptr(), buf.len() as u32) };
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

#[cfg(not(windows))]
fn console_process_count() -> Option<u32> {
    None
}

/// 탐색기 더블클릭으로 자기 콘솔을 단독 점유해 실행됐는지 추정한다(순수).
/// attach 프로세스가 자기 자신 1개뿐이면 double-click; 터미널 실행은 부모 셸도 attach 되어
/// 2 이상. None(비-Windows/감지 실패)은 false(보수적 — 일시정지하지 않음).
fn is_double_click_launch(console_process_count: Option<u32>) -> bool {
    console_process_count == Some(1)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor { guardrails }) => run_doctor(guardrails),
        Some(Command::Risk { command, profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            print!("{}", format_risk(&command, &p));
            Ok(())
        }
        Some(Command::Policy { action }) => match action {
            PolicyAction::Show { profile } => {
                let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
                print!("{}", describe_profile(&p));
                Ok(())
            }
            PolicyAction::Set { profile } => {
                let p = resolve_profile(&profile)?;
                config::set_active_profile(p.name)?;
                println!("활성 정책 프로파일을 '{}'(으)로 설정했습니다.", p.name);
                Ok(())
            }
        },
        Some(Command::ShellHook { shell }) => {
            let sh = resolve_shell(Some(&shell))?;
            print!("{}", shell::hook_script(sh));
            Ok(())
        }
        Some(Command::Tui { profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            ui::run(p.name)
        }
        Some(Command::Shell {}) => run_persistent_shell(),
        Some(Command::Remote { action }) => {
            use ai_terminal::gate;
            let path = gate::armed_path()?;
            match action {
                RemoteAction::Arm { allow_high } => {
                    gate::arm_at(&path, allow_high)?;
                    println!(
                        "원격 게이트 armed{}.",
                        if allow_high {
                            " (High opt-in 허용)"
                        } else {
                            ""
                        }
                    );
                }
                RemoteAction::Disarm {} => {
                    gate::disarm_at(&path)?;
                    println!("원격 게이트 disarmed.");
                }
                RemoteAction::Status {} => match gate::load_arm_state(&path) {
                    Some(st) => println!(
                        "armed (allow_high={}). 위험 명령이 인터셉트됩니다.",
                        st.allow_high
                    ),
                    None => println!("disarmed. 인터셉트 미개입."),
                },
                RemoteAction::Daemon {
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                } => run_gate_daemon(
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                )?,
                RemoteAction::Devices {} => run_remote_devices()?,
                RemoteAction::Transport {} => run_remote_transport()?,
                RemoteAction::RelaySetup {
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                } => run_remote_relay_setup(
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                )?,
                RemoteAction::Pair {
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                } => run_remote_pair(
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                )?,
                RemoteAction::ApprovalUrl {
                    request_json,
                    pwa_url,
                } => run_remote_approval_url(request_json, pwa_url)?,
                RemoteAction::ApprovalVerify {
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                } => run_remote_approval_verify(
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                )?,
            }
            Ok(())
        }
        Some(Command::Mask { text }) => {
            print!("{}", format_mask(&text));
            Ok(())
        }
        Some(Command::Preview { command }) => {
            print!("{}", format_preview(&command));
            Ok(())
        }
        Some(Command::Verify { command }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            let v = verify_agent::verify_command(&command, &profile);
            println!("binary   : {:?}", v.binary);
            println!("risk     : {:?} -> {:?}", v.risk, v.decision);
            println!("safe     : {}", v.safe_to_suggest);
            if !v.issues.is_empty() {
                println!("issues   :");
                for i in &v.issues {
                    println!("  - {i}");
                }
            }
            Ok(())
        }
        Some(Command::Classify { input }) => {
            println!("{:?}", intent::classify(&input));
            Ok(())
        }
        Some(Command::Plan { request }) => {
            for (i, step) in planner::plan(&request).steps.iter().enumerate() {
                let cmd = step.command.as_deref().unwrap_or("(AI 위임)");
                println!("{}. {} — {}", i + 1, step.description, cmd);
            }
            Ok(())
        }
        Some(Command::Index { query, root }) => {
            let idx = index::FileIndex::build(&root);
            let results = idx.search(&query, 10);
            if results.is_empty() {
                println!("(매칭 파일 없음, {} 파일 인덱싱)", idx.len());
            }
            for (path, score) in results {
                println!("  {score:>3}  {}", path.display());
            }
            Ok(())
        }
        Some(Command::Route { input }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            match dispatch::dispatch(&input, &profile) {
                dispatch::Route::Empty => println!("(빈 입력)"),
                dispatch::Route::Shell {
                    command,
                    risk,
                    decision,
                } => {
                    println!("route    : Shell");
                    println!("command  : {command}");
                    println!("risk     : {risk:?} -> {decision:?}");
                }
                dispatch::Route::Ai { prompt } => {
                    println!("route    : AI");
                    println!("prompt   : {prompt}");
                }
            }
            Ok(())
        }
        Some(Command::Ask {
            prompt,
            backend,
            model,
            ollama_url,
            openai_url,
        }) => {
            let ai_cfg = config::Ai {
                provider: backend.clone(),
                model: model.clone(),
                ollama_url: ollama_url.clone(),
                openai_url: openai_url.clone(),
            };
            let cap = ai_terminal::provider::Provider::mock().models[0].clone();
            let gw = match ai_cfg.provider.as_str() {
                "ollama" => {
                    let b = ai_terminal::ollama::OllamaBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.ollama_url,
                        &ai_cfg.model,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                "openai" => {
                    let api_key = std::env::var("OPENAI_API_KEY").ok();
                    let b = ai_terminal::openai::OpenAiBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.openai_url,
                        &ai_cfg.model,
                        api_key,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                _ => gateway::Gateway::mock(),
            };
            // 예산 게이트(§31.7) — storage가 있으면 누적 지출을 읽어 주입한다. 초과 시
            // 게이트웨이가 원격 백엔드 호출 전에 차단한다. default 빌드는 영속 지출을
            // 모르므로 미적용(현행 동작 보존).
            #[cfg(feature = "storage")]
            let gw = match ai_terminal::store::Store::open_default() {
                Ok(store) => {
                    let spent = store.total_cost(None).unwrap_or(0.0);
                    gw.with_budget(spent, usage::BudgetConfig::defaults())
                }
                Err(_) => gw,
            };
            let ctx = context::gather();
            // AI 호출을 타임아웃·Ctrl+C 취소와 함께 실행한다(§16.2, Graceful Recovery).
            let ctx_str = format!("cwd={}", ctx.cwd);
            let timeout = ai_terminal::aitask::Timeouts::defaults().request;
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let result = rt.block_on(async {
                let cancel = std::sync::Arc::new(tokio::sync::Notify::new());
                ai_terminal::aitask::cancel_on_ctrl_c(cancel.clone());
                gw.ask_cancellable(&prompt, &ctx_str, timeout, cancel).await
            });
            match result {
                Ok(gateway::GatewayOutcome::Answered {
                    text,
                    input_tokens,
                    output_tokens,
                    source,
                }) => {
                    println!("{text}");
                    let usage = ai_terminal::ai_usage::summarize(
                        &ai_cfg,
                        source,
                        input_tokens,
                        output_tokens,
                    );
                    let cost_badge = if usage.estimated { " estimated" } else { "" };
                    println!(
                        "(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                        usage.cost_usd,
                        cache_badge(source)
                    );
                    #[cfg(feature = "storage")]
                    let _ = ai_terminal::ai_usage::record(&usage, None);
                }
                Ok(gateway::GatewayOutcome::Blocked(reason)) => {
                    println!("[차단] 원격 전송 불가(fail-closed): {reason}");
                }
                Err(e) => {
                    // AI 장애는 셸로 전파되지 않는다(§3-3). 친절히 고지하고 정상 종료.
                    println!("[AI 사용 불가] {e}");
                }
            }
            Ok(())
        }
        Some(Command::Skill { query }) => {
            let mut paths = vec![PathBuf::from("./.ai-terminal/skills")];
            if let Ok(cd) = config::config_dir() {
                paths.push(cd.join("skills"));
            }
            let skills = skill::discover(&paths);
            let shown: Vec<&skill::Skill> = match &query {
                Some(q) => skill::match_skills(&skills, q, 5),
                None => skills.iter().collect(),
            };
            if shown.is_empty() {
                println!("(스킬 없음 — {:?})", paths);
            }
            for s in shown {
                println!("- {} — {}", s.name, s.description);
            }
            Ok(())
        }
        Some(Command::Mcp { config: cfg }) => {
            let path = match cfg {
                Some(p) => p,
                None => config::config_dir()?.join("mcp.json"),
            };
            match std::fs::read_to_string(&path) {
                Ok(json) => {
                    let servers = mcp::parse_servers(&json)?;
                    if servers.is_empty() {
                        println!("(등록된 MCP 서버 없음)");
                    }
                    for s in &servers {
                        println!("- {} : {} {}", s.name, s.command, s.args.join(" "));
                    }
                    println!("(auto_connect=false — 부작용 도구는 컨센트·감사 필요, §27)");
                }
                Err(_) => println!("mcp.json 없음: {}", path.display()),
            }
            Ok(())
        }
        Some(Command::Context {}) => {
            let c = context::gather();
            println!("cwd      : {}", c.cwd);
            println!("shell    : {}", c.shell);
            println!("user     : {}", c.user);
            println!("hostname : {}", c.hostname);
            println!("git      : {}", c.git_branch.as_deref().unwrap_or("-"));
            Ok(())
        }
        Some(Command::Explain {
            command,
            exit,
            stderr,
            last_error,
        }) => run_explain(command, exit, stderr, last_error),
        Some(Command::Undo { target }) => {
            if target != "last" {
                anyhow::bail!("지원하지 않는 undo 대상: {target} (last만 지원)");
            }
            let dir = undo::default_undo_dir()?;
            match undo::latest(&dir) {
                Some(id) => {
                    let n = undo::restore(&dir, &id)?;
                    println!("복구 완료: {n}개 파일 ({id})");
                }
                None => println!("복구할 백업이 없습니다."),
            }
            Ok(())
        }
        Some(Command::Init { target }) => match target {
            InitTarget::Shell {
                shell,
                rc,
                dry_run,
                diff,
                uninstall,
            } => {
                let sh = resolve_shell(shell.as_deref())?;
                let path = resolve_rc(rc, sh)?;
                let old = std::fs::read_to_string(&path).unwrap_or_default();
                let mode = if uninstall {
                    InitMode::Uninstall
                } else if diff {
                    InitMode::Diff
                } else if dry_run {
                    InitMode::DryRun
                } else {
                    InitMode::Install
                };
                let plan = plan_init_shell(&old, sh, mode, &path.display().to_string());
                if plan.write {
                    std::fs::write(&path, &plan.new_content)?;
                }
                print!("{}", plan.message);
                Ok(())
            }
        },
        #[cfg(feature = "storage")]
        Some(Command::Usage {}) => {
            let store = ai_terminal::store::Store::open_default()?;
            let spent = store.total_cost(None)?;
            let cfg = usage::BudgetConfig::defaults();
            let action = usage::evaluate(spent, cfg.session_usd, cfg.warn_pct, cfg.block_pct);
            println!("usage    : ${spent:.4} 사용");
            println!(
                "budget   : 세션 ${:.2} / 월 ${:.2} (경고 {}% / 차단 {}%)",
                cfg.session_usd, cfg.monthly_usd, cfg.warn_pct, cfg.block_pct
            );
            println!("status   : {action:?}");
            Ok(())
        }
        #[cfg(feature = "storage")]
        Some(Command::History { limit }) => {
            let store = ai_terminal::store::Store::open_default()?;
            let rows = store.recent_commands(limit)?;
            if rows.is_empty() {
                println!("(아직 기록된 명령이 없습니다)");
            }
            for r in rows.iter().rev() {
                let lvl = r.risk_level.as_deref().unwrap_or("-");
                println!("[{lvl:<8}] {}", r.command_text);
            }
            Ok(())
        }
        Some(Command::Exec {
            command,
            yes,
            profile,
        }) => run_exec(&command, yes, profile),
        Some(Command::Dispatch {
            input,
            yes,
            profile,
        }) => run_dispatch(&input, yes, profile),
        Some(Command::Gate { command }) => {
            let cmd = command.join(" ");
            let code = run_gate(&cmd);
            std::process::exit(code);
        }
        Some(Command::Hook { event, rest }) => {
            // hook 실패가 셸을 막지 않도록 항상 Ok 반환(best-effort).
            #[cfg(feature = "storage")]
            match event.as_str() {
                // preexec: 명령을 위험도와 함께 기록(종료 코드 미정).
                "preexec" => {
                    if let Err(e) = record_hook_preexec(&rest) {
                        tracing::debug!("hook preexec record failed (ignored): {e}");
                    }
                }
                // precmd: 직전 명령의 실제 종료 코드를 반영.
                "precmd" => {
                    if let Err(e) = record_hook_precmd(&rest) {
                        tracing::debug!("hook precmd record failed (ignored): {e}");
                    }
                }
                // chpwd: 작업 디렉터리 변경 → 세션 cwd + git branch 컨텍스트 스냅샷.
                "chpwd" => {
                    if let Err(e) = record_hook_chpwd(&rest) {
                        tracing::debug!("hook chpwd record failed (ignored): {e}");
                    }
                }
                _ => {}
            }
            tracing::trace!(event, ?rest, "shell hook event");
            Ok(())
        }
        None => {
            // TODO(M1): 인터랙티브 터미널(REPL/TUI) 진입. 현재는 사용법 안내.
            println!(
                "ai {} — `ai doctor` 로 환경 진단, `ai --help` 로 사용법 확인.",
                env!("CARGO_PKG_VERSION")
            );
            // 탐색기 더블클릭(자기 콘솔 단독 점유)이면 콘솔이 즉시 닫혀 안내를 못 본다.
            // 이 도구는 CLI다 — 사용법을 보여주고 Enter 입력까지 창을 유지한다(터미널 실행엔 무영향).
            if is_double_click_launch(console_process_count()) {
                use std::io::Write;
                println!();
                println!("이 파일은 AI Terminal GUI 앱이 아니라 명령줄(CLI) 도구 `ai.exe`입니다.");
                println!("Windows GUI 터미널은 릴리즈의 `ai-terminal-windows-*.zip` 또는 설치 파일을 받아 `ai-terminal.exe`를 실행하세요.");
                println!();
                println!("CLI로 사용할 때는 더블클릭이 아니라 터미널에서 실행하세요:");
                println!("  ai doctor                 # 환경 진단");
                println!("  ai risk \"rm -rf /tmp/x\"   # 위험도 평가");
                println!("  ai --help                 # 전체 명령");
                println!(
                    "CLI PATH 등록: scripts/install.ps1 · 문서: https://github.com/ai-cli-terminal/terminal"
                );
                print!("\n계속하려면 Enter 키를 누르세요... ");
                let _ = std::io::stdout().flush();
                let mut _line = String::new();
                let _ = std::io::stdin().read_line(&mut _line);
            }
            Ok(())
        }
    }
}

/// `ai shell` — 영속 PTY 셸(Native Wrapper, FU-3). 하나의 `PtySession`을 재사용해 `cd`가
/// 다음 명령에 유지되며(영속성), 각 명령 뒤 probe로 cwd를 동기화한다(§7.4). 라인 단위
/// REPL이며 입력 인터셉트·분류는 범위 외(라인 게이트는 `ai exec`/`ai tui`).
fn run_persistent_shell() -> anyhow::Result<()> {
    use std::io::{BufRead, Write};

    use ai_terminal::pty::PtySession;
    use ai_terminal::wrapper::{self, PROBE};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    // 라인 에디터가 probe 마커(\x1f)를 가로채면 cwd 동기화가 멈추므로 셸별 안전 인자를 준다.
    let shell_args = wrapper::session_shell_args(&shell);
    let shell_arg_refs: Vec<&str> = shell_args.iter().map(String::as_str).collect();
    let mut session = PtySession::spawn(&shell, &shell_arg_refs)?;
    println!("ai shell — 영속 셸 (exit/quit/Ctrl-D 종료). cwd는 probe로 동기화됩니다.");

    let stdin = std::io::stdin();
    let mut last_cwd = String::new();
    loop {
        print!("ai> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF(Ctrl-D)
        }
        let cmd = line.trim_end();
        if cmd == "exit" || cmd == "quit" {
            break;
        }
        if cmd.trim().is_empty() {
            continue;
        }

        session.write_input(&wrapper::probe_command(cmd))?;
        // probe 쌍(이번 명령의 cwd 방출)을 볼 때까지 출력을 모은다. 인터랙티브 echo로
        // 마커가 더 보일 수 있으나, 마지막 파싱 cwd가 실제값이다.
        let mut acc = String::new();
        for _ in 0..2000 {
            acc.push_str(&session.read_chunk()?);
            if acc.matches(PROBE).count() >= 2 {
                break;
            }
        }
        print!("{}", wrapper::strip_probes(&acc));
        let _ = std::io::stdout().flush();

        if let Some(cwd) = wrapper::parse_probe_cwds(&acc).into_iter().last() {
            if cwd != last_cwd && cwd.starts_with('/') {
                last_cwd = cwd.clone();
                #[cfg(feature = "storage")]
                sync_wrapper_cwd(&cwd);
            }
        }
    }
    let _ = session.kill();
    Ok(())
}

/// probe로 관측한 cwd를 세션 컨텍스트에 동기화한다(§7.4, storage feature).
#[cfg(feature = "storage")]
fn sync_wrapper_cwd(cwd: &str) {
    use ai_terminal::store::{NewContext, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let session_id = "sess-default";
    let _ = store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.to_string(),
            policy_profile: config::get_active_profile(),
        },
    );
    let branch = ai_terminal::context::git_branch(std::path::Path::new(cwd));
    let _ = store.update_session_cwd(session_id, cwd);
    let _ = store.record_context_snapshot(&NewContext {
        session_id: session_id.into(),
        context_type: "wrapper_probe".into(),
        cwd: Some(cwd.to_string()),
        git_branch: branch,
    });
}

fn run_exec(command: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut sink = StdoutSink;
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };

    let outcome = pipeline::execute(command, &cfg, &executor, confirmer.as_mut(), &mut sink)?;
    flush_stdout();
    finish_shell_outcome(command, "exec", outcome)
}

fn run_dispatch(input: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::dispatch::{self, AiOutcome, Handled, Handlers};
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };
    let mut ai = ai_terminal::responder::GatewayResponder::mock()?;
    let mut sink = StdoutSink;

    let mut h = Handlers {
        executor: &executor,
        confirmer: confirmer.as_mut(),
        ai: &mut ai,
        sink: &mut sink,
    };
    let handled = dispatch::run(input, &prof, &cfg, &mut h)?;
    flush_stdout();

    match handled {
        Handled::Empty => Ok(()),
        Handled::Shell(outcome) => finish_shell_outcome(input, "dispatch", outcome),
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            source,
            ..
        }) => {
            let usage = ai_terminal::ai_usage::summarize(
                &config::Ai {
                    provider: "mock".into(),
                    model: "mock-model".into(),
                    ..Default::default()
                },
                source,
                input_tokens,
                output_tokens,
            );
            #[cfg(feature = "storage")]
            let _ = ai_terminal::ai_usage::record(&usage, None);
            // 답변 본문은 이미 sink(stdout)로 출력됨. 토큰 요약만 덧붙인다.
            let cost_badge = if usage.estimated { " estimated" } else { "" };
            println!(
                "\n(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                usage.cost_usd,
                cache_badge(source)
            );
            Ok(())
        }
        Handled::Ai(AiOutcome::Blocked(r)) => {
            println!("[차단] 원격 전송 불가(fail-closed): {r}");
            Ok(())
        }
        Handled::Ai(AiOutcome::Unavailable(e)) => {
            println!("[AI 사용 불가] {e}");
            Ok(())
        }
    }
}

fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// 셸 실행 결과를 마무리한다: audit 기록 + 사용자 안내 + 프로세스 종료(항상 발산).
/// `run_exec`·`run_dispatch`가 공유한다. `command`는 기록/안내에 쓸 명령 텍스트.
fn finish_shell_outcome(
    command: &str,
    source: &str,
    outcome: ai_terminal::pipeline::ExecOutcome,
) -> ! {
    use ai_terminal::pipeline::ExecOutcome;

    if let ExecOutcome::Ran { exit_code, undo_id } = &outcome {
        if let Some(id) = undo_id {
            eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
        }
        ai_terminal::shell_audit::record_ran_command(command, *exit_code, source);
        std::process::exit(*exit_code);
    }

    // 비-Ran: audit 기록 후 안내 + exit 1.
    if let Some(rec) = ai_terminal::shell_audit::shell_outcome_audit(command, source, &outcome) {
        ai_terminal::shell_audit::record_outcome_audit(&rec);
    }
    match outcome {
        ExecOutcome::Blocked { level, factors } => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
        }
        ExecOutcome::Declined => eprintln!("실행을 취소했습니다."),
        ExecOutcome::BackupRefused(r) => eprintln!("백업 거부로 실행 중단: {r}"),
        ExecOutcome::Ran { .. } => unreachable!("Ran 은 위에서 처리됨"),
    }
    std::process::exit(1);
}

/// 캐시 출처 배지(Backend는 무배지). `ai ask`·`ai dispatch` 공용.
fn cache_badge(source: ai_terminal::cache::CacheSource) -> &'static str {
    use ai_terminal::cache::CacheSource;
    match source {
        CacheSource::Backend => "",
        CacheSource::Exact => " [cache: exact]",
        CacheSource::Semantic => " [cache: semantic ~근사]",
    }
}

/// `ai doctor`용 config 진단 텍스트(순수 포매터).
fn format_config_diagnostics(loaded: &config::LoadedConfig) -> String {
    use std::fmt::Write as _;
    let source = match &loaded.source {
        config::ConfigSource::File(p) => format!("file: {}", p.display()),
        config::ConfigSource::Default => "default (no file)".to_string(),
    };
    let shell = loaded
        .config
        .general
        .default_shell
        .as_deref()
        .unwrap_or("<unset>");
    let mut out = String::new();
    let _ = writeln!(out, "config: {source}");
    let _ = writeln!(
        out,
        "  general.history_limit = {}",
        loaded.config.general.history_limit
    );
    let _ = write!(out, "  general.default_shell = {shell}");
    if let Some(w) = &loaded.warning {
        let _ = write!(out, "\n  warning: {w}");
    }
    out
}

/// `ai doctor` — 현재 환경/플랫폼 capability를 표시한다.
///
/// MVP에서는 정적 분석·preview·timeout 등 baseline guardrails를 모든 플랫폼에서
/// 보장하고, 동적 감시(seccomp/cgroups 등)는 플랫폼별로 다르다(§31.11).
fn run_doctor(show_guardrails: bool) -> anyhow::Result<()> {
    println!("AI Terminal doctor");
    println!("  version : {}", env!("CARGO_PKG_VERSION"));
    println!("{}", format_config_diagnostics(&config::load()));
    println!("  os      : {}", std::env::consts::OS);
    println!("  arch    : {}", std::env::consts::ARCH);

    // 통합 모드(§30-1): hook 마커가 현재 셸에 있으면 hook, 아니면 wrapper fallback.
    let hook_on = shell::hook_active(|k| std::env::var(k).ok());
    let mode = shell::resolve_integration_mode(shell::ConfiguredMode::Auto, hook_on);
    match mode {
        shell::IntegrationMode::Hook => println!("  shell   : hook 통합 활성"),
        shell::IntegrationMode::Wrapper => {
            println!("  shell   : wrapper fallback (hook 미감지)");
            println!(
                "            명령을 `ai exec \"<cmd>\"`로 실행하면 컨텍스트가 기록됩니다. hook 설치: `ai init shell`."
            );
        }
    }

    if show_guardrails {
        let platform = guardrails::detect();
        println!("\nplatform : {platform:?}  (정본 §31.11)");
        println!("baseline guardrails (모든 플랫폼):");
        for g in guardrails::baseline() {
            println!("  - {g}");
        }
        println!("platform-specific (동적 감시):");
        for c in guardrails::capabilities(platform) {
            println!("  - {:<28} {:?}", c.name, c.support);
        }
        if guardrails::dynamic_monitoring_limited(platform) {
            println!(
                "\n[!] 동적 감시가 제한되는 플랫폼입니다. High 이상 명령 확인을 강화합니다(§31.11)."
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_launch_only_when_sole_console_process() {
        // 자기 콘솔 단독 점유(1) = 더블클릭. 터미널 실행(부모 셸 attach, 2+)·감지 실패(None)는 아님.
        assert!(is_double_click_launch(Some(1)));
        assert!(!is_double_click_launch(Some(2)));
        assert!(!is_double_click_launch(Some(5)));
        assert!(!is_double_click_launch(None));
    }

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

    #[test]
    fn parses_exec_command() {
        let cli = Cli::parse_from(["ai", "exec", "rm -rf build", "--yes"]);
        match cli.command {
            Some(Command::Exec {
                command,
                yes,
                profile,
            }) => {
                assert_eq!(command, "rm -rf build");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Exec, got {other:?}"),
        }
    }

    #[test]
    fn parses_dispatch_command() {
        let cli = Cli::parse_from(["ai", "dispatch", "ls -al", "--yes"]);
        match cli.command {
            Some(Command::Dispatch {
                input,
                yes,
                profile,
            }) => {
                assert_eq!(input, "ls -al");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Dispatch, got {other:?}"),
        }
    }

    #[test]
    fn cache_badge_labels() {
        use ai_terminal::cache::CacheSource;
        assert_eq!(cache_badge(CacheSource::Backend), "");
        assert!(cache_badge(CacheSource::Exact).contains("exact"));
        assert!(cache_badge(CacheSource::Semantic).contains("semantic"));
    }

    #[test]
    fn config_diagnostics_show_file_source_and_values() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config {
                general: ai_terminal::config::General {
                    default_shell: Some("/bin/bash".to_string()),
                    history_limit: 123,
                },
                ai: Default::default(),
            },
            source: ai_terminal::config::ConfigSource::File(std::path::PathBuf::from(
                "/cfg/config.toml",
            )),
            warning: None,
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("file: /cfg/config.toml"), "{out}");
        assert!(out.contains("general.history_limit = 123"), "{out}");
        assert!(out.contains("general.default_shell = /bin/bash"), "{out}");
    }

    #[test]
    fn config_diagnostics_show_default_and_warning() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config::default(),
            source: ai_terminal::config::ConfigSource::Default,
            warning: Some("boom".to_string()),
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("default (no file)"), "{out}");
        assert!(out.contains("general.default_shell = <unset>"), "{out}");
        assert!(out.contains("warning: boom"), "{out}");
    }
}
