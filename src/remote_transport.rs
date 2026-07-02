//! Remote companion transport catalog.
//!
//! This module is intentionally status/configuration plumbing only. Planned
//! modes are not selectable runtime transports until their security and
//! evidence gates exist.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const COMPANION_RELAY_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_COMPANION_RELAY_FRAME_TTL_MS: u64 = 30_000;
pub const DEFAULT_COMPANION_RELAY_SESSION_TTL_MS: u64 = 5 * 60 * 1000;
const MAX_RELAY_SESSION_ID_LEN: usize = 96;
const MIN_RELAY_SESSION_TOKEN_LEN: usize = 32;
const MAX_RELAY_SESSION_TOKEN_LEN: usize = 128;
const MAX_RELAY_DEVICE_ID_LEN: usize = 96;
const MAX_RELAY_PAYLOAD_JSON_BYTES: usize = 1 << 20;
const COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID: &str = "websocket";

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
