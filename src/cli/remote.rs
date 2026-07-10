#[cfg(feature = "remote")]
use crate::cli::gate::resolve_relay_deployment_selection;

#[cfg(all(feature = "remote", unix))]
pub(crate) fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok(format!(
        "unix://{}",
        ai_terminal::daemon::device_socket_path()?.display()
    ))
}

#[cfg(all(feature = "remote", not(unix)))]
pub(crate) fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok("unsupported://local-daemon-unavailable".into())
}

#[cfg(feature = "remote")]
pub(crate) fn run_remote_devices() -> anyhow::Result<()> {
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
pub(crate) fn run_remote_transport() -> anyhow::Result<()> {
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
pub(crate) fn run_remote_relay_setup(
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
pub(crate) fn run_remote_pair(
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
pub(crate) fn run_remote_approval_url(
    request_json: String,
    pwa_url: Option<String>,
) -> anyhow::Result<()> {
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
pub(crate) fn run_remote_approval_verify(
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
pub(crate) fn remote_approval_verify_device(
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
pub(crate) fn run_remote_approval_verify(
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
pub(crate) fn run_remote_approval_url(
    _request_json: String,
    _pwa_url: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote approval-url`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
pub(crate) fn run_remote_pair(
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
pub(crate) fn run_remote_devices() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote devices`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
pub(crate) fn run_remote_transport() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote transport`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
pub(crate) fn run_remote_relay_setup(
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
