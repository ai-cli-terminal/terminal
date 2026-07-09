use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::frame::CompanionRelayPeer;
use super::ticket::{
    CompanionRelaySessionTicket, CompanionRelaySessionTicketInput,
    CompanionRelaySignedSessionTicket, CompanionRelayTicketKeyringRecord,
};
use super::validate::{
    valid_relay_device_id, valid_relay_private_network_name, valid_relay_pubkey_hex,
    valid_relay_session_id, valid_relay_session_token, valid_relay_websocket_endpoint_url,
};
use super::{
    COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK, COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED,
    COMPANION_RELAY_PROTOCOL_VERSION, COMPANION_RELAY_SETUP_TRANSPORT_MODE,
    DEFAULT_COMPANION_RELAY_SESSION_TTL_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySelfHostedSetupInput {
    pub deployment_mode: Option<String>,
    pub private_network_name: Option<String>,
    pub relay_endpoint_url: String,
    pub daemon_pubkey: Vec<u8>,
    pub companion_device_id: String,
    pub companion_noise_pubkey: Vec<u8>,
    pub companion_approval_pubkey: [u8; 32],
    pub issued_at_ms: u64,
    pub ttl_ms: u64,
    pub session_id: Option<String>,
    pub session_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompanionRelayCompanionIdentity {
    pub device_id: String,
    pub noise_pubkey_hex: String,
    pub approval_pubkey_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompanionRelaySelfHostedRuntimeSetup {
    pub relay_protocol_version: u32,
    pub transport_mode: String,
    pub deployment_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_network_name: Option<String>,
    pub relay_endpoint_url: String,
    pub signed_session_ticket: CompanionRelaySignedSessionTicket,
    pub daemon_connect: CompanionRelaySessionConnect,
    pub companion_connect: CompanionRelaySessionConnect,
    pub companion_identity: CompanionRelayCompanionIdentity,
    pub operator_setup_text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySessionConnect {
    pub relay_protocol_version: u32,
    pub session_id: String,
    pub peer: CompanionRelayPeer,
    pub session_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daemon_pubkey_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_pubkey_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_pubkey_hex: Option<String>,
}
pub fn issue_self_hosted_relay_runtime_setup(
    keyring: &CompanionRelayTicketKeyringRecord,
    input: CompanionRelaySelfHostedSetupInput,
) -> Result<CompanionRelaySelfHostedRuntimeSetup> {
    keyring.validate()?;
    if !valid_relay_websocket_endpoint_url(&input.relay_endpoint_url) {
        bail!("relay endpoint URL must be wss:// or localhost ws://");
    }
    let deployment_mode = input
        .deployment_mode
        .as_deref()
        .unwrap_or(COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED);
    let private_network_name = match deployment_mode {
        COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED => {
            if input
                .private_network_name
                .as_deref()
                .map(str::trim)
                .is_some_and(|name| !name.is_empty())
            {
                bail!("self-hosted relay setup must not include private_network_name");
            }
            None
        }
        COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK => {
            let name = input
                .private_network_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| anyhow::anyhow!("private-network relay setup requires name"))?;
            if !valid_relay_private_network_name(name) {
                bail!("private-network relay name format error");
            }
            Some(name.to_string())
        }
        _ => bail!("relay setup deployment_mode format error"),
    };
    if input.issued_at_ms == 0 {
        bail!("relay session issued_at_ms format error");
    }
    if input.ttl_ms == 0 || input.ttl_ms > DEFAULT_COMPANION_RELAY_SESSION_TTL_MS {
        bail!("relay session ttl format error");
    }
    let expires_at_ms = input
        .issued_at_ms
        .checked_add(input.ttl_ms)
        .context("relay session expiry overflow")?;
    let session_id = match input.session_id {
        Some(session_id) => session_id,
        None => generate_relay_session_id(input.issued_at_ms)?,
    };
    let session_token = match input.session_token {
        Some(session_token) => session_token,
        None => generate_relay_session_token()?,
    };
    let companion_identity = CompanionRelayCompanionIdentity {
        device_id: input.companion_device_id.clone(),
        noise_pubkey_hex: crate::pairing::hex_encode(&input.companion_noise_pubkey),
        approval_pubkey_hex: crate::pairing::hex_encode(&input.companion_approval_pubkey),
    };
    let signed_session_ticket =
        keyring
            .issuer()?
            .issue_websocket_ticket(CompanionRelaySessionTicketInput {
                session_id,
                session_token,
                issued_at_ms: input.issued_at_ms,
                expires_at_ms,
                daemon_pubkey_hex: crate::pairing::hex_encode(&input.daemon_pubkey),
                companion_device_id: companion_identity.device_id.clone(),
                companion_noise_pubkey_hex: companion_identity.noise_pubkey_hex.clone(),
                companion_approval_pubkey_hex: companion_identity.approval_pubkey_hex.clone(),
            })?;
    let daemon_connect = CompanionRelaySessionConnect::daemon(&signed_session_ticket.ticket)?;
    let companion_connect = CompanionRelaySessionConnect::companion(&signed_session_ticket.ticket)?;
    let operator_setup_text = match private_network_name.as_deref() {
        Some(name) => format!(
            "Private-network relay {name} endpoint {} is ready for device {} until {}.",
            input.relay_endpoint_url, companion_identity.device_id, expires_at_ms
        ),
        None => format!(
            "Self-hosted relay endpoint {} is ready for device {} until {}.",
            input.relay_endpoint_url, companion_identity.device_id, expires_at_ms
        ),
    };
    let setup = CompanionRelaySelfHostedRuntimeSetup {
        relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
        transport_mode: COMPANION_RELAY_SETUP_TRANSPORT_MODE.into(),
        deployment_mode: deployment_mode.into(),
        private_network_name,
        relay_endpoint_url: input.relay_endpoint_url,
        signed_session_ticket,
        daemon_connect,
        companion_connect,
        companion_identity,
        operator_setup_text,
    };
    setup.validate_metadata()?;
    Ok(setup)
}

fn generate_relay_session_id(now_ms: u64) -> Result<String> {
    if now_ms == 0 {
        bail!("relay session issued_at_ms format error");
    }
    let mut random = [0_u8; 6];
    getrandom::getrandom(&mut random)?;
    Ok(format!(
        "relay-session-{now_ms:x}-{}",
        crate::pairing::hex_encode(&random)
    ))
}

fn generate_relay_session_token() -> Result<String> {
    let mut random = [0_u8; 32];
    getrandom::getrandom(&mut random)?;
    Ok(format!("token_{}", crate::pairing::hex_encode(&random)))
}
impl CompanionRelaySelfHostedRuntimeSetup {
    pub fn validate_metadata(&self) -> Result<()> {
        if self.relay_protocol_version != COMPANION_RELAY_PROTOCOL_VERSION {
            bail!("unsupported companion relay protocol version");
        }
        if self.transport_mode != COMPANION_RELAY_SETUP_TRANSPORT_MODE {
            bail!("relay setup transport_mode format error");
        }
        match self.deployment_mode.as_str() {
            COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED => {
                if self.private_network_name.is_some() {
                    bail!("self-hosted relay setup must not include private_network_name");
                }
            }
            COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK => {
                let private_network_name = self
                    .private_network_name
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("private-network relay setup requires name"))?;
                if !valid_relay_private_network_name(private_network_name) {
                    bail!("private-network relay name format error");
                }
            }
            _ => bail!("relay setup deployment_mode format error"),
        }
        if !valid_relay_websocket_endpoint_url(&self.relay_endpoint_url) {
            bail!("relay endpoint URL must be wss:// or localhost ws://");
        }
        self.signed_session_ticket.validate_metadata()?;
        self.daemon_connect.validate_metadata()?;
        self.companion_connect.validate_metadata()?;
        self.signed_session_ticket.ticket.validate_connect(
            &self.daemon_connect,
            self.signed_session_ticket.ticket.issued_at_ms,
        )?;
        self.signed_session_ticket.ticket.validate_connect(
            &self.companion_connect,
            self.signed_session_ticket.ticket.issued_at_ms,
        )?;
        if self.companion_identity.device_id
            != self.signed_session_ticket.ticket.companion_device_id
            || self.companion_identity.noise_pubkey_hex
                != self.signed_session_ticket.ticket.companion_noise_pubkey_hex
            || self.companion_identity.approval_pubkey_hex
                != self
                    .signed_session_ticket
                    .ticket
                    .companion_approval_pubkey_hex
        {
            bail!("relay setup companion identity mismatch");
        }
        if self.operator_setup_text.trim().len() < 12 {
            bail!("relay setup operator text missing");
        }
        Ok(())
    }
}
impl CompanionRelaySessionConnect {
    pub fn daemon(ticket: &CompanionRelaySessionTicket) -> Result<Self> {
        let connect = Self {
            relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
            session_id: ticket.session_id.clone(),
            peer: CompanionRelayPeer::Daemon,
            session_token: ticket.session_token.clone(),
            daemon_pubkey_hex: Some(ticket.daemon_pubkey_hex.clone()),
            device_id: None,
            noise_pubkey_hex: None,
            approval_pubkey_hex: None,
        };
        connect.validate_metadata()?;
        Ok(connect)
    }

    pub fn companion(ticket: &CompanionRelaySessionTicket) -> Result<Self> {
        let connect = Self {
            relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
            session_id: ticket.session_id.clone(),
            peer: CompanionRelayPeer::Companion,
            session_token: ticket.session_token.clone(),
            daemon_pubkey_hex: None,
            device_id: Some(ticket.companion_device_id.clone()),
            noise_pubkey_hex: Some(ticket.companion_noise_pubkey_hex.clone()),
            approval_pubkey_hex: Some(ticket.companion_approval_pubkey_hex.clone()),
        };
        connect.validate_metadata()?;
        Ok(connect)
    }

    pub fn validate_metadata(&self) -> Result<()> {
        if self.relay_protocol_version != COMPANION_RELAY_PROTOCOL_VERSION {
            bail!("unsupported companion relay protocol version");
        }
        if !valid_relay_session_id(&self.session_id) {
            bail!("relay session_id format error");
        }
        if !valid_relay_session_token(&self.session_token) {
            bail!("relay session_token format error");
        }
        match self.peer {
            CompanionRelayPeer::Daemon => {
                if self
                    .daemon_pubkey_hex
                    .as_deref()
                    .map(valid_relay_pubkey_hex)
                    != Some(true)
                {
                    bail!("relay daemon_pubkey_hex format error");
                }
                if self.device_id.is_some()
                    || self.noise_pubkey_hex.is_some()
                    || self.approval_pubkey_hex.is_some()
                {
                    bail!("relay daemon connect contains companion fields");
                }
            }
            CompanionRelayPeer::Companion => {
                if self.device_id.as_deref().map(valid_relay_device_id) != Some(true) {
                    bail!("relay companion device_id format error");
                }
                if self.noise_pubkey_hex.as_deref().map(valid_relay_pubkey_hex) != Some(true) {
                    bail!("relay companion noise_pubkey_hex format error");
                }
                if self
                    .approval_pubkey_hex
                    .as_deref()
                    .map(valid_relay_pubkey_hex)
                    != Some(true)
                {
                    bail!("relay companion approval_pubkey_hex format error");
                }
                if self.daemon_pubkey_hex.is_some() {
                    bail!("relay companion connect contains daemon field");
                }
            }
        }
        Ok(())
    }
}
