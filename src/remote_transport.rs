//! Remote companion transport catalog.
//!
//! This module is intentionally status/configuration plumbing only. Planned
//! modes are not selectable runtime transports until their security and
//! evidence gates exist.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const COMPANION_RELAY_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_COMPANION_RELAY_FRAME_TTL_MS: u64 = 30_000;
pub const DEFAULT_COMPANION_RELAY_SESSION_TTL_MS: u64 = 5 * 60 * 1000;
pub const COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256: &str = "hmac-sha256";
pub const MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS: usize = 3;
pub const COMPANION_RELAY_TICKET_KEYRING_FILE: &str = "remote-relay-ticket-keys.json";
pub const COMPANION_RELAY_TICKET_KEYRING_VERSION: u32 = 1;
pub const COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED: &str = "self-hosted";
pub const COMPANION_RELAY_SETUP_TRANSPORT_MODE: &str = "relay";
const MAX_RELAY_SESSION_ID_LEN: usize = 96;
const MIN_RELAY_SESSION_TOKEN_LEN: usize = 32;
const MAX_RELAY_SESSION_TOKEN_LEN: usize = 128;
const MAX_RELAY_DEVICE_ID_LEN: usize = 96;
const MAX_RELAY_TICKET_KEY_ID_LEN: usize = 64;
const MAX_RELAY_ENDPOINT_URL_LEN: usize = 2048;
const MAX_RELAY_PAYLOAD_JSON_BYTES: usize = 1 << 20;
const MIN_RELAY_TICKET_HMAC_KEY_BYTES: usize = 32;
const COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID: &str = "websocket";

type RelayTicketHmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionTransportReadiness {
    Ready,
    Internal,
    Planned,
}

impl CompanionTransportReadiness {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Internal => "internal",
            Self::Planned => "planned",
        }
    }
}

impl fmt::Display for CompanionTransportReadiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionTransportMode {
    LiveLoopback,
    DeviceSocket,
    Relay,
    Tailscale,
    WebSocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionTransportDescriptor {
    pub mode: CompanionTransportMode,
    pub id: &'static str,
    pub readiness: CompanionTransportReadiness,
    pub role: &'static str,
}

pub const ALL_COMPANION_TRANSPORT_MODES: [CompanionTransportMode; 5] = [
    CompanionTransportMode::LiveLoopback,
    CompanionTransportMode::DeviceSocket,
    CompanionTransportMode::Relay,
    CompanionTransportMode::Tailscale,
    CompanionTransportMode::WebSocket,
];

impl CompanionTransportMode {
    pub const fn active_product() -> Self {
        Self::LiveLoopback
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::LiveLoopback => "live-loopback",
            Self::DeviceSocket => "device-sock",
            Self::Relay => "relay",
            Self::Tailscale => "tailscale",
            Self::WebSocket => "websocket",
        }
    }

    pub const fn readiness(self) -> CompanionTransportReadiness {
        match self {
            Self::LiveLoopback => CompanionTransportReadiness::Ready,
            Self::DeviceSocket => CompanionTransportReadiness::Internal,
            Self::Relay | Self::Tailscale | Self::WebSocket => CompanionTransportReadiness::Planned,
        }
    }

    pub const fn role(self) -> &'static str {
        match self {
            Self::LiveLoopback => "product default for same-host browser companion",
            Self::DeviceSocket => "Unix socket substrate and fallback candidate",
            Self::Relay => "M2 relay service path",
            Self::Tailscale => "direct private-network candidate",
            Self::WebSocket => "browser-friendly relay/session substrate",
        }
    }

    pub const fn descriptor(self) -> CompanionTransportDescriptor {
        CompanionTransportDescriptor {
            mode: self,
            id: self.id(),
            readiness: self.readiness(),
            role: self.role(),
        }
    }

    pub const fn is_product_default(self) -> bool {
        matches!(self, Self::LiveLoopback)
    }
}

impl fmt::Display for CompanionTransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for CompanionTransportMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "live-loopback" => Ok(Self::LiveLoopback),
            "device-sock" => Ok(Self::DeviceSocket),
            "relay" => Ok(Self::Relay),
            "tailscale" => Ok(Self::Tailscale),
            "websocket" => Ok(Self::WebSocket),
            other => Err(format!("unknown companion transport mode: {other}")),
        }
    }
}

pub fn active_product_mode() -> CompanionTransportMode {
    CompanionTransportMode::active_product()
}

pub fn all_modes() -> &'static [CompanionTransportMode] {
    &ALL_COMPANION_TRANSPORT_MODES
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompanionRelayPeer {
    Daemon,
    Companion,
}

impl CompanionRelayPeer {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Companion => "companion",
        }
    }
}

impl fmt::Display for CompanionRelayPeer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Relay routing wrapper for existing companion transport JSON.
///
/// Relay code may validate this metadata without understanding approval
/// semantics. Daemon/PWA endpoints remain responsible for decoding and
/// validating the `CompanionTransportMsg` payload.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayFrame {
    pub relay_protocol_version: u32,
    pub session_id: String,
    pub sender: CompanionRelayPeer,
    pub sequence: u64,
    pub sent_at_ms: u64,
    pub expires_at_ms: u64,
    pub payload_json: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayRouteEnvelope {
    pub relay_protocol_version: u32,
    pub session_id: String,
    pub sender: CompanionRelayPeer,
    pub sequence: u64,
    pub sent_at_ms: u64,
    pub expires_at_ms: u64,
    pub payload_json_bytes: usize,
}

impl CompanionRelayFrame {
    pub fn new(
        session_id: impl Into<String>,
        sender: CompanionRelayPeer,
        sequence: u64,
        sent_at_ms: u64,
        expires_at_ms: u64,
        payload_json: impl Into<String>,
    ) -> Result<Self> {
        let frame = Self {
            relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
            session_id: session_id.into(),
            sender,
            sequence,
            sent_at_ms,
            expires_at_ms,
            payload_json: payload_json.into(),
        };
        frame.validate_metadata()?;
        Ok(frame)
    }

    pub fn from_message(
        session_id: impl Into<String>,
        sender: CompanionRelayPeer,
        sequence: u64,
        sent_at_ms: u64,
        expires_at_ms: u64,
        message: &crate::session::CompanionTransportMsg,
    ) -> Result<Self> {
        Self::new(
            session_id,
            sender,
            sequence,
            sent_at_ms,
            expires_at_ms,
            crate::session::companion_transport_json(message)?,
        )
    }

    pub fn validate_metadata(&self) -> Result<()> {
        if self.relay_protocol_version != COMPANION_RELAY_PROTOCOL_VERSION {
            bail!("unsupported companion relay protocol version");
        }
        if !valid_relay_session_id(&self.session_id) {
            bail!("relay session_id format error");
        }
        if self.sequence == 0 {
            bail!("relay sequence must be positive");
        }
        if self.sent_at_ms == 0 {
            bail!("relay sent_at_ms must be positive");
        }
        if self.expires_at_ms <= self.sent_at_ms {
            bail!("relay expires_at_ms must be greater than sent_at_ms");
        }
        let payload_len = self.payload_json.len();
        if payload_len == 0 || payload_len > MAX_RELAY_PAYLOAD_JSON_BYTES {
            bail!("relay payload_json size error");
        }
        Ok(())
    }

    pub fn payload_message(&self) -> Result<crate::session::CompanionTransportMsg> {
        self.validate_metadata()?;
        crate::session::parse_companion_transport_json(&self.payload_json)
    }

    pub fn route_envelope(&self) -> Result<CompanionRelayRouteEnvelope> {
        self.validate_metadata()?;
        Ok(CompanionRelayRouteEnvelope {
            relay_protocol_version: self.relay_protocol_version,
            session_id: self.session_id.clone(),
            sender: self.sender,
            sequence: self.sequence,
            sent_at_ms: self.sent_at_ms,
            expires_at_ms: self.expires_at_ms,
            payload_json_bytes: self.payload_json.len(),
        })
    }
}

pub fn valid_relay_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELAY_SESSION_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

pub fn valid_relay_session_token(value: &str) -> bool {
    value.len() >= MIN_RELAY_SESSION_TOKEN_LEN
        && value.len() <= MAX_RELAY_SESSION_TOKEN_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-' | b'~'))
}

pub fn valid_relay_device_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELAY_DEVICE_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn valid_relay_pubkey_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_relay_ticket_mac_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_relay_ticket_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELAY_TICKET_KEY_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

pub fn valid_relay_websocket_endpoint_url(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_RELAY_ENDPOINT_URL_LEN
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return false;
    }

    if value.starts_with("wss://") {
        return value.len() > "wss://".len();
    }

    if let Some(rest) = value.strip_prefix("ws://") {
        let authority = rest.split(&['/', '?', '#'][..]).next().unwrap_or_default();
        let host = if authority.starts_with("[::1]") {
            "[::1]"
        } else {
            authority.split(':').next().unwrap_or_default()
        };
        return matches!(host, "127.0.0.1" | "localhost" | "[::1]");
    }

    false
}

fn validate_relay_ticket_hmac_key(secret: &[u8]) -> Result<()> {
    if secret.len() < MIN_RELAY_TICKET_HMAC_KEY_BYTES {
        bail!("relay ticket hmac key too short");
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySessionTicket {
    pub relay_protocol_version: u32,
    pub transport: String,
    pub session_id: String,
    pub session_token: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub daemon_pubkey_hex: String,
    pub companion_device_id: String,
    pub companion_noise_pubkey_hex: String,
    pub companion_approval_pubkey_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySessionTicketInput {
    pub session_id: String,
    pub session_token: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub daemon_pubkey_hex: String,
    pub companion_device_id: String,
    pub companion_noise_pubkey_hex: String,
    pub companion_approval_pubkey_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySignedSessionTicket {
    pub ticket: CompanionRelaySessionTicket,
    pub mac_alg: String,
    pub mac_hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayTicketIssuer {
    hmac_sha256_keys: Vec<CompanionRelayTicketHmacKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompanionRelayTicketHmacKey {
    key_id: Option<String>,
    secret: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayTicketHmacKeyRecord {
    pub key_id: String,
    pub secret: Vec<u8>,
    pub created_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_at_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayTicketKeyringRecord {
    pub version: u32,
    pub active_key_id: String,
    pub hmac_sha256_keys: Vec<CompanionRelayTicketHmacKeyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelaySelfHostedSetupInput {
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

impl CompanionRelaySessionTicket {
    pub fn websocket(input: CompanionRelaySessionTicketInput) -> Result<Self> {
        let ticket = Self {
            relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
            transport: COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID.into(),
            session_id: input.session_id,
            session_token: input.session_token,
            issued_at_ms: input.issued_at_ms,
            expires_at_ms: input.expires_at_ms,
            daemon_pubkey_hex: input.daemon_pubkey_hex,
            companion_device_id: input.companion_device_id,
            companion_noise_pubkey_hex: input.companion_noise_pubkey_hex,
            companion_approval_pubkey_hex: input.companion_approval_pubkey_hex,
        };
        ticket.validate_metadata()?;
        Ok(ticket)
    }

    pub fn validate_metadata(&self) -> Result<()> {
        if self.relay_protocol_version != COMPANION_RELAY_PROTOCOL_VERSION {
            bail!("unsupported companion relay protocol version");
        }
        if self.transport != COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID {
            bail!("relay transport must be websocket");
        }
        if !valid_relay_session_id(&self.session_id) {
            bail!("relay session_id format error");
        }
        if !valid_relay_session_token(&self.session_token) {
            bail!("relay session_token format error");
        }
        if self.issued_at_ms == 0 || self.expires_at_ms <= self.issued_at_ms {
            bail!("relay session expiry format error");
        }
        if self.expires_at_ms - self.issued_at_ms > DEFAULT_COMPANION_RELAY_SESSION_TTL_MS {
            bail!("relay session ttl too long");
        }
        if !valid_relay_pubkey_hex(&self.daemon_pubkey_hex) {
            bail!("relay daemon_pubkey_hex format error");
        }
        if !valid_relay_device_id(&self.companion_device_id) {
            bail!("relay companion device_id format error");
        }
        if !valid_relay_pubkey_hex(&self.companion_noise_pubkey_hex) {
            bail!("relay companion noise_pubkey_hex format error");
        }
        if !valid_relay_pubkey_hex(&self.companion_approval_pubkey_hex) {
            bail!("relay companion approval_pubkey_hex format error");
        }
        Ok(())
    }

    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }

    pub fn validate_connect(
        &self,
        connect: &CompanionRelaySessionConnect,
        now_ms: u64,
    ) -> Result<()> {
        self.validate_metadata()?;
        connect.validate_metadata()?;
        if now_ms == 0 {
            bail!("relay session now_ms format error");
        }
        if self.is_expired_at(now_ms) {
            bail!("relay session expired");
        }
        if connect.session_id != self.session_id {
            bail!("relay session_id mismatch");
        }
        if connect.session_token != self.session_token {
            bail!("relay session_token mismatch");
        }
        match connect.peer {
            CompanionRelayPeer::Daemon => {
                if connect.daemon_pubkey_hex.as_deref() != Some(self.daemon_pubkey_hex.as_str()) {
                    bail!("relay daemon pubkey mismatch");
                }
            }
            CompanionRelayPeer::Companion => {
                if connect.device_id.as_deref() != Some(self.companion_device_id.as_str()) {
                    bail!("relay companion device_id mismatch");
                }
                if connect.noise_pubkey_hex.as_deref()
                    != Some(self.companion_noise_pubkey_hex.as_str())
                {
                    bail!("relay companion noise pubkey mismatch");
                }
                if connect.approval_pubkey_hex.as_deref()
                    != Some(self.companion_approval_pubkey_hex.as_str())
                {
                    bail!("relay companion approval pubkey mismatch");
                }
            }
        }
        Ok(())
    }
}

pub fn relay_session_ticket_signing_payload(
    ticket: &CompanionRelaySessionTicket,
) -> Result<String> {
    ticket.validate_metadata()?;
    Ok(format!(
        concat!(
            "ai-terminal-relay-ticket-v1\n",
            "relay_protocol_version={}\n",
            "transport={}\n",
            "session_id={}\n",
            "session_token={}\n",
            "issued_at_ms={}\n",
            "expires_at_ms={}\n",
            "daemon_pubkey_hex={}\n",
            "companion_device_id={}\n",
            "companion_noise_pubkey_hex={}\n",
            "companion_approval_pubkey_hex={}\n"
        ),
        ticket.relay_protocol_version,
        ticket.transport,
        ticket.session_id,
        ticket.session_token,
        ticket.issued_at_ms,
        ticket.expires_at_ms,
        ticket.daemon_pubkey_hex,
        ticket.companion_device_id,
        ticket.companion_noise_pubkey_hex,
        ticket.companion_approval_pubkey_hex
    ))
}

pub fn relay_session_ticket_hmac_sha256_hex(
    ticket: &CompanionRelaySessionTicket,
    secret: &[u8],
) -> Result<String> {
    validate_relay_ticket_hmac_key(secret)?;
    let payload = relay_session_ticket_signing_payload(ticket)?;
    let mut mac = RelayTicketHmacSha256::new_from_slice(secret)
        .expect("HMAC-SHA256 accepts any non-empty key length");
    mac.update(payload.as_bytes());
    let bytes = mac.finalize().into_bytes();
    Ok(crate::pairing::hex_encode(bytes.as_slice()))
}

impl CompanionRelaySignedSessionTicket {
    pub fn hmac_sha256(ticket: CompanionRelaySessionTicket, secret: &[u8]) -> Result<Self> {
        Self::hmac_sha256_with_key_id(ticket, secret, None)
    }

    pub fn hmac_sha256_with_key_id(
        ticket: CompanionRelaySessionTicket,
        secret: &[u8],
        key_id: Option<String>,
    ) -> Result<Self> {
        if let Some(key_id) = key_id.as_deref() {
            if !valid_relay_ticket_key_id(key_id) {
                bail!("relay ticket key_id format error");
            }
        }
        let signed = Self {
            mac_hex: relay_session_ticket_hmac_sha256_hex(&ticket, secret)?,
            ticket,
            mac_alg: COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256.into(),
            key_id,
        };
        signed.validate_metadata()?;
        Ok(signed)
    }

    pub fn validate_metadata(&self) -> Result<()> {
        self.ticket.validate_metadata()?;
        if self.mac_alg != COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256 {
            bail!("relay ticket mac_alg format error");
        }
        if !valid_relay_ticket_mac_hex(&self.mac_hex) {
            bail!("relay ticket mac_hex format error");
        }
        if let Some(key_id) = self.key_id.as_deref() {
            if !valid_relay_ticket_key_id(key_id) {
                bail!("relay ticket key_id format error");
            }
        }
        Ok(())
    }

    pub fn validate_mac(&self, secret: &[u8]) -> Result<()> {
        validate_relay_ticket_hmac_key(secret)?;
        self.validate_metadata()?;
        let actual = crate::pairing::hex_decode(&self.mac_hex)?;
        let payload = relay_session_ticket_signing_payload(&self.ticket)?;
        let mut mac = RelayTicketHmacSha256::new_from_slice(secret)
            .expect("HMAC-SHA256 accepts any non-empty key length");
        mac.update(payload.as_bytes());
        if mac.verify_slice(&actual).is_err() {
            bail!("relay ticket mac mismatch");
        }
        Ok(())
    }

    pub fn validate_ticket(&self, secret: &[u8]) -> Result<&CompanionRelaySessionTicket> {
        self.validate_mac(secret)?;
        Ok(&self.ticket)
    }

    pub fn validate_connect(
        &self,
        connect: &CompanionRelaySessionConnect,
        now_ms: u64,
        secret: &[u8],
    ) -> Result<()> {
        self.validate_mac(secret)?;
        self.ticket.validate_connect(connect, now_ms)
    }
}

impl CompanionRelayTicketKeyringRecord {
    pub fn new(
        active_key_id: impl Into<String>,
        active_secret: Vec<u8>,
        now_ms: u64,
    ) -> Result<Self> {
        let record = Self {
            version: COMPANION_RELAY_TICKET_KEYRING_VERSION,
            active_key_id: active_key_id.into(),
            hmac_sha256_keys: vec![CompanionRelayTicketHmacKeyRecord {
                key_id: String::new(),
                secret: active_secret,
                created_at_ms: now_ms,
                retired_at_ms: None,
            }],
        };
        let mut record = record;
        record.hmac_sha256_keys[0].key_id = record.active_key_id.clone();
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != COMPANION_RELAY_TICKET_KEYRING_VERSION {
            bail!("relay ticket keyring version unsupported");
        }
        if !valid_relay_ticket_key_id(&self.active_key_id) {
            bail!("relay ticket active key_id format error");
        }
        if self.hmac_sha256_keys.is_empty() {
            bail!("relay ticket keyring empty");
        }
        if self.hmac_sha256_keys.len() > MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS {
            bail!("relay ticket hmac verify key count too high");
        }

        let mut active_seen = false;
        for (index, key) in self.hmac_sha256_keys.iter().enumerate() {
            if !valid_relay_ticket_key_id(&key.key_id) {
                bail!("relay ticket key_id format error");
            }
            if key.created_at_ms == 0 {
                bail!("relay ticket key created_at_ms format error");
            }
            validate_relay_ticket_hmac_key(&key.secret)?;
            if self.hmac_sha256_keys[..index]
                .iter()
                .any(|existing| existing.key_id == key.key_id)
            {
                bail!("relay ticket key_id duplicate");
            }
            if self.hmac_sha256_keys[..index]
                .iter()
                .any(|existing| existing.secret == key.secret)
            {
                bail!("relay ticket hmac key duplicate");
            }

            if key.key_id == self.active_key_id {
                if key.retired_at_ms.is_some() {
                    bail!("relay ticket active key retired");
                }
                active_seen = true;
            } else {
                let retired_at_ms = key
                    .retired_at_ms
                    .context("relay ticket previous key missing retired_at_ms")?;
                if retired_at_ms <= key.created_at_ms {
                    bail!("relay ticket previous key retired_at_ms format error");
                }
            }
        }

        if !active_seen {
            bail!("relay ticket active key missing");
        }
        Ok(())
    }

    pub fn active_key(&self) -> Option<&CompanionRelayTicketHmacKeyRecord> {
        self.hmac_sha256_keys
            .iter()
            .find(|key| key.key_id == self.active_key_id)
    }

    pub fn previous_keys(&self) -> impl Iterator<Item = &CompanionRelayTicketHmacKeyRecord> {
        self.hmac_sha256_keys
            .iter()
            .filter(|key| key.key_id != self.active_key_id)
    }

    pub fn issuer(&self) -> Result<CompanionRelayTicketIssuer> {
        CompanionRelayTicketIssuer::from_keyring_record(self)
    }

    pub fn rotate_hmac_key(
        &self,
        new_key_id: impl Into<String>,
        new_secret: Vec<u8>,
        now_ms: u64,
    ) -> Result<Self> {
        self.validate()?;
        let new_key_id = new_key_id.into();
        if self
            .hmac_sha256_keys
            .iter()
            .any(|key| key.key_id == new_key_id)
        {
            bail!("relay ticket key_id duplicate");
        }
        let mut hmac_sha256_keys = vec![CompanionRelayTicketHmacKeyRecord {
            key_id: new_key_id.clone(),
            secret: new_secret,
            created_at_ms: now_ms,
            retired_at_ms: None,
        }];

        let mut previous_active = self
            .active_key()
            .context("relay ticket active key missing")?
            .clone();
        previous_active.retired_at_ms = Some(now_ms);
        hmac_sha256_keys.push(previous_active);

        for previous in self.previous_keys() {
            if hmac_sha256_keys.len() >= MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS {
                break;
            }
            hmac_sha256_keys.push(previous.clone());
        }

        let rotated = Self {
            version: self.version,
            active_key_id: new_key_id,
            hmac_sha256_keys,
        };
        rotated.validate()?;
        Ok(rotated)
    }
}

pub fn companion_relay_ticket_keyring_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join(COMPANION_RELAY_TICKET_KEYRING_FILE))
}

pub fn load_companion_relay_ticket_keyring(
    path: &Path,
) -> Result<CompanionRelayTicketKeyringRecord> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("relay ticket keyring 읽기 실패: {}", path.display()))?;
    let record: CompanionRelayTicketKeyringRecord = serde_json::from_slice(&bytes)
        .with_context(|| format!("relay ticket keyring 파싱 실패: {}", path.display()))?;
    record.validate()?;
    Ok(record)
}

pub fn save_companion_relay_ticket_keyring(
    path: &Path,
    record: &CompanionRelayTicketKeyringRecord,
) -> Result<()> {
    record.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(record)?;
    std::fs::write(path, bytes)
        .with_context(|| format!("relay ticket keyring 쓰기 실패: {}", path.display()))
}

pub fn load_or_create_companion_relay_ticket_keyring(
    path: &Path,
) -> Result<CompanionRelayTicketKeyringRecord> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let record: CompanionRelayTicketKeyringRecord = serde_json::from_slice(&bytes)
                .with_context(|| format!("relay ticket keyring 파싱 실패: {}", path.display()))?;
            record.validate()?;
            Ok(record)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let record = new_companion_relay_ticket_keyring(relay_ticket_now_ms())?;
            save_companion_relay_ticket_keyring(path, &record)?;
            Ok(record)
        }
        Err(err) => {
            Err(err).with_context(|| format!("relay ticket keyring 읽기 실패: {}", path.display()))
        }
    }
}

pub fn new_companion_relay_ticket_keyring(
    now_ms: u64,
) -> Result<CompanionRelayTicketKeyringRecord> {
    let key_id = generate_relay_ticket_key_id(now_ms)?;
    let secret = generate_relay_ticket_hmac_secret()?;
    CompanionRelayTicketKeyringRecord::new(key_id, secret, now_ms)
}

fn generate_relay_ticket_hmac_secret() -> Result<Vec<u8>> {
    let mut secret = vec![0_u8; MIN_RELAY_TICKET_HMAC_KEY_BYTES];
    getrandom::getrandom(&mut secret)?;
    Ok(secret)
}

fn generate_relay_ticket_key_id(now_ms: u64) -> Result<String> {
    if now_ms == 0 {
        bail!("relay ticket key created_at_ms format error");
    }
    let mut random = [0_u8; 6];
    getrandom::getrandom(&mut random)?;
    Ok(format!(
        "relay-{now_ms:x}-{}",
        crate::pairing::hex_encode(&random)
    ))
}

fn relay_ticket_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

pub fn issue_self_hosted_relay_runtime_setup(
    keyring: &CompanionRelayTicketKeyringRecord,
    input: CompanionRelaySelfHostedSetupInput,
) -> Result<CompanionRelaySelfHostedRuntimeSetup> {
    keyring.validate()?;
    if !valid_relay_websocket_endpoint_url(&input.relay_endpoint_url) {
        bail!("relay endpoint URL must be wss:// or localhost ws://");
    }
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
    let operator_setup_text = format!(
        "Self-hosted relay endpoint {} is ready for device {} until {}.",
        input.relay_endpoint_url, companion_identity.device_id, expires_at_ms
    );
    let setup = CompanionRelaySelfHostedRuntimeSetup {
        relay_protocol_version: COMPANION_RELAY_PROTOCOL_VERSION,
        transport_mode: COMPANION_RELAY_SETUP_TRANSPORT_MODE.into(),
        deployment_mode: COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED.into(),
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
        if self.deployment_mode != COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED {
            bail!("relay setup deployment_mode format error");
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

impl CompanionRelayTicketIssuer {
    pub fn hmac_sha256(active_secret: Vec<u8>, previous_secrets: Vec<Vec<u8>>) -> Result<Self> {
        let verify_key_count = 1 + previous_secrets.len();
        if verify_key_count > MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS {
            bail!("relay ticket hmac verify key count too high");
        }

        validate_relay_ticket_hmac_key(&active_secret)?;
        let mut hmac_sha256_keys = Vec::with_capacity(verify_key_count);
        hmac_sha256_keys.push(CompanionRelayTicketHmacKey {
            key_id: None,
            secret: active_secret,
        });

        for secret in previous_secrets {
            validate_relay_ticket_hmac_key(&secret)?;
            if hmac_sha256_keys
                .iter()
                .any(|existing| existing.secret == secret)
            {
                bail!("relay ticket hmac key duplicate");
            }
            hmac_sha256_keys.push(CompanionRelayTicketHmacKey {
                key_id: None,
                secret,
            });
        }

        Ok(Self { hmac_sha256_keys })
    }

    pub fn hmac_sha256_with_key_ids(
        active_key_id: impl Into<String>,
        active_secret: Vec<u8>,
        previous_keys: Vec<(String, Vec<u8>)>,
    ) -> Result<Self> {
        let active_key_id = active_key_id.into();
        if !valid_relay_ticket_key_id(&active_key_id) {
            bail!("relay ticket active key_id format error");
        }
        let verify_key_count = 1 + previous_keys.len();
        if verify_key_count > MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS {
            bail!("relay ticket hmac verify key count too high");
        }

        validate_relay_ticket_hmac_key(&active_secret)?;
        let mut hmac_sha256_keys = Vec::with_capacity(verify_key_count);
        hmac_sha256_keys.push(CompanionRelayTicketHmacKey {
            key_id: Some(active_key_id),
            secret: active_secret,
        });

        for (key_id, secret) in previous_keys {
            if !valid_relay_ticket_key_id(&key_id) {
                bail!("relay ticket key_id format error");
            }
            validate_relay_ticket_hmac_key(&secret)?;
            if hmac_sha256_keys
                .iter()
                .any(|existing| existing.key_id.as_deref() == Some(key_id.as_str()))
            {
                bail!("relay ticket key_id duplicate");
            }
            if hmac_sha256_keys
                .iter()
                .any(|existing| existing.secret == secret)
            {
                bail!("relay ticket hmac key duplicate");
            }
            hmac_sha256_keys.push(CompanionRelayTicketHmacKey {
                key_id: Some(key_id),
                secret,
            });
        }

        Ok(Self { hmac_sha256_keys })
    }

    pub fn from_keyring_record(record: &CompanionRelayTicketKeyringRecord) -> Result<Self> {
        record.validate()?;
        let active = record
            .active_key()
            .context("relay ticket active key missing")?;
        let previous_keys = record
            .previous_keys()
            .map(|key| (key.key_id.clone(), key.secret.clone()))
            .collect();
        Self::hmac_sha256_with_key_ids(active.key_id.clone(), active.secret.clone(), previous_keys)
    }

    pub fn active_key_id(&self) -> Option<&str> {
        self.hmac_sha256_keys
            .first()
            .and_then(|key| key.key_id.as_deref())
    }

    pub fn verification_key_count(&self) -> usize {
        self.hmac_sha256_keys.len()
    }

    pub fn issue_websocket_ticket(
        &self,
        input: CompanionRelaySessionTicketInput,
    ) -> Result<CompanionRelaySignedSessionTicket> {
        let ticket = CompanionRelaySessionTicket::websocket(input)?;
        let active = &self.hmac_sha256_keys[0];
        CompanionRelaySignedSessionTicket::hmac_sha256_with_key_id(
            ticket,
            &active.secret,
            active.key_id.clone(),
        )
    }

    pub fn validate_ticket<'a>(
        &self,
        signed: &'a CompanionRelaySignedSessionTicket,
    ) -> Result<&'a CompanionRelaySessionTicket> {
        signed.validate_metadata()?;
        if let Some(key_id) = signed.key_id.as_deref() {
            let key = self
                .hmac_sha256_keys
                .iter()
                .find(|key| key.key_id.as_deref() == Some(key_id))
                .context("relay ticket key_id unknown")?;
            signed.validate_mac(&key.secret)?;
            return Ok(&signed.ticket);
        }
        for key in &self.hmac_sha256_keys {
            if signed.validate_mac(&key.secret).is_ok() {
                return Ok(&signed.ticket);
            }
        }
        bail!("relay ticket mac mismatch")
    }

    pub fn validate_connect(
        &self,
        signed: &CompanionRelaySignedSessionTicket,
        connect: &CompanionRelaySessionConnect,
        now_ms: u64,
    ) -> Result<()> {
        let ticket = self.validate_ticket(signed)?;
        ticket.validate_connect(connect, now_ms)
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

#[derive(Debug, Default)]
pub struct CompanionRelayLoopback {
    sessions: HashMap<String, CompanionRelaySessionQueue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionRelayLoopbackStats {
    pub session_count: usize,
    pub queued_frames: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayEndpoint {
    session_id: String,
    peer: CompanionRelayPeer,
    next_sequence: u64,
    frame_ttl_ms: u64,
}

#[derive(Debug, Default)]
struct CompanionRelaySessionQueue {
    daemon_to_companion: VecDeque<CompanionRelayFrame>,
    companion_to_daemon: VecDeque<CompanionRelayFrame>,
    last_daemon_sequence: u64,
    last_companion_sequence: u64,
}

impl CompanionRelayLoopback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, frame: CompanionRelayFrame) -> Result<()> {
        frame.validate_metadata()?;
        let session = self.sessions.entry(frame.session_id.clone()).or_default();
        session.enqueue(frame)
    }

    pub fn dequeue(
        &mut self,
        session_id: &str,
        recipient: CompanionRelayPeer,
        now_ms: u64,
    ) -> Result<Option<CompanionRelayFrame>> {
        if !valid_relay_session_id(session_id) {
            bail!("relay session_id format error");
        }

        let (frame, remove_session) = match self.sessions.get_mut(session_id) {
            Some(session) => {
                let frame = session.dequeue_for_recipient(recipient, now_ms);
                (frame, session.is_empty())
            }
            None => return Ok(None),
        };
        if remove_session {
            self.sessions.remove(session_id);
        }
        Ok(frame)
    }

    pub fn queued_for(&self, session_id: &str, recipient: CompanionRelayPeer) -> Result<usize> {
        if !valid_relay_session_id(session_id) {
            bail!("relay session_id format error");
        }
        Ok(self
            .sessions
            .get(session_id)
            .map(|session| session.queue_for_recipient(recipient).len())
            .unwrap_or(0))
    }

    pub fn stats(&self) -> CompanionRelayLoopbackStats {
        CompanionRelayLoopbackStats {
            session_count: self.sessions.len(),
            queued_frames: self
                .sessions
                .values()
                .map(CompanionRelaySessionQueue::queued_frames)
                .sum(),
        }
    }
}

impl CompanionRelayEndpoint {
    pub fn new(session_id: impl Into<String>, peer: CompanionRelayPeer) -> Result<Self> {
        Self::with_frame_ttl(session_id, peer, DEFAULT_COMPANION_RELAY_FRAME_TTL_MS)
    }

    pub fn with_frame_ttl(
        session_id: impl Into<String>,
        peer: CompanionRelayPeer,
        frame_ttl_ms: u64,
    ) -> Result<Self> {
        let session_id = session_id.into();
        if !valid_relay_session_id(&session_id) {
            bail!("relay session_id format error");
        }
        if frame_ttl_ms == 0 {
            bail!("relay frame_ttl_ms must be positive");
        }
        Ok(Self {
            session_id,
            peer,
            next_sequence: 1,
            frame_ttl_ms,
        })
    }

    pub fn daemon(session_id: impl Into<String>) -> Result<Self> {
        Self::new(session_id, CompanionRelayPeer::Daemon)
    }

    pub fn companion(session_id: impl Into<String>) -> Result<Self> {
        Self::new(session_id, CompanionRelayPeer::Companion)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn peer(&self) -> CompanionRelayPeer {
        self.peer
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn send_message(
        &mut self,
        relay: &mut CompanionRelayLoopback,
        now_ms: u64,
        message: &crate::session::CompanionTransportMsg,
    ) -> Result<u64> {
        let sequence = self.next_sequence;
        let expires_at_ms = match now_ms.checked_add(self.frame_ttl_ms) {
            Some(value) => value,
            None => bail!("relay frame expiry overflow"),
        };
        let frame = CompanionRelayFrame::from_message(
            self.session_id.clone(),
            self.peer,
            sequence,
            now_ms,
            expires_at_ms,
            message,
        )?;
        relay.enqueue(frame)?;
        self.next_sequence = match self.next_sequence.checked_add(1) {
            Some(value) => value,
            None => bail!("relay sequence overflow"),
        };
        Ok(sequence)
    }

    pub fn recv_message(
        &self,
        relay: &mut CompanionRelayLoopback,
        now_ms: u64,
    ) -> Result<Option<crate::session::CompanionTransportMsg>> {
        match relay.dequeue(&self.session_id, self.peer, now_ms)? {
            Some(frame) => Ok(Some(frame.payload_message()?)),
            None => Ok(None),
        }
    }
}

impl CompanionRelaySessionQueue {
    fn enqueue(&mut self, frame: CompanionRelayFrame) -> Result<()> {
        let last_sequence = match frame.sender {
            CompanionRelayPeer::Daemon => &mut self.last_daemon_sequence,
            CompanionRelayPeer::Companion => &mut self.last_companion_sequence,
        };
        if frame.sequence <= *last_sequence {
            bail!("relay sequence must increase for sender");
        }
        *last_sequence = frame.sequence;
        self.queue_for_sender_mut(frame.sender).push_back(frame);
        Ok(())
    }

    fn dequeue_for_recipient(
        &mut self,
        recipient: CompanionRelayPeer,
        now_ms: u64,
    ) -> Option<CompanionRelayFrame> {
        let queue = self.queue_for_recipient_mut(recipient);
        while let Some(frame) = queue.pop_front() {
            if now_ms < frame.expires_at_ms {
                return Some(frame);
            }
        }
        None
    }

    fn queued_frames(&self) -> usize {
        self.daemon_to_companion.len() + self.companion_to_daemon.len()
    }

    fn is_empty(&self) -> bool {
        self.queued_frames() == 0
    }

    fn queue_for_sender_mut(
        &mut self,
        sender: CompanionRelayPeer,
    ) -> &mut VecDeque<CompanionRelayFrame> {
        match sender {
            CompanionRelayPeer::Daemon => &mut self.daemon_to_companion,
            CompanionRelayPeer::Companion => &mut self.companion_to_daemon,
        }
    }

    fn queue_for_recipient_mut(
        &mut self,
        recipient: CompanionRelayPeer,
    ) -> &mut VecDeque<CompanionRelayFrame> {
        match recipient {
            CompanionRelayPeer::Daemon => &mut self.companion_to_daemon,
            CompanionRelayPeer::Companion => &mut self.daemon_to_companion,
        }
    }

    fn queue_for_recipient(&self, recipient: CompanionRelayPeer) -> &VecDeque<CompanionRelayFrame> {
        match recipient {
            CompanionRelayPeer::Daemon => &self.companion_to_daemon,
            CompanionRelayPeer::Companion => &self.daemon_to_companion,
        }
    }
}

#[cfg(test)]
mod tests {
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

    fn relay_ticket_input(
        issued_at_ms: u64,
        expires_at_ms: u64,
    ) -> CompanionRelaySessionTicketInput {
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
        let ticket =
            CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
        let secret = b"relay-ticket-secret-1234567890abcdef";
        let signed =
            CompanionRelaySignedSessionTicket::hmac_sha256(ticket.clone(), secret).unwrap();

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
        let ticket =
            CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
        let secret = b"relay-ticket-secret-1234567890abcdef";
        let signed =
            CompanionRelaySignedSessionTicket::hmac_sha256(ticket.clone(), secret).unwrap();

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
            CompanionRelaySignedSessionTicket::hmac_sha256(previous_ticket, &previous_secret)
                .unwrap();
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
            CompanionRelayTicketKeyringRecord::new("relay-active-v1", active_v1.clone(), 1000)
                .unwrap();
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
            CompanionRelayTicketKeyringRecord::new("bad key id", active_secret.clone(), 1000)
                .is_err()
        );
        assert!(
            CompanionRelayTicketKeyringRecord::new("relay-active", active_secret.clone(), 0)
                .is_err()
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
        let ticket =
            CompanionRelaySessionTicket::websocket(relay_ticket_input(1000, 2000)).unwrap();
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
            CompanionRelayEndpoint::with_frame_ttl("session-a", CompanionRelayPeer::Daemon, 0)
                .is_err()
        );
    }

    #[test]
    fn relay_endpoint_applies_frame_ttl() {
        let mut relay = CompanionRelayLoopback::new();
        let mut daemon =
            CompanionRelayEndpoint::with_frame_ttl("session-a", CompanionRelayPeer::Daemon, 5)
                .unwrap();
        let companion = CompanionRelayEndpoint::companion("session-a").unwrap();
        let message = crate::session::CompanionTransportMsg::Ping {
            nonce: "expires".into(),
        };

        daemon.send_message(&mut relay, 100, &message).unwrap();
        assert!(companion.recv_message(&mut relay, 105).unwrap().is_none());
        assert_eq!(relay.stats().queued_frames, 0);
    }
}
