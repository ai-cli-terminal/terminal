//! Remote companion transport catalog.
//!
//! This module is intentionally status/configuration plumbing only. Planned
//! modes are not selectable runtime transports until their security and
//! evidence gates exist.

use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const COMPANION_RELAY_PROTOCOL_VERSION: u32 = 1;
const MAX_RELAY_SESSION_ID_LEN: usize = 96;
const MAX_RELAY_PAYLOAD_JSON_BYTES: usize = 1 << 20;

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
}

pub fn valid_relay_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RELAY_SESSION_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
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
    fn relay_session_id_validation_is_stable_ascii() {
        assert!(valid_relay_session_id("relay-session_1:daemon.web"));
        assert!(!valid_relay_session_id(""));
        assert!(!valid_relay_session_id("relay session"));
        assert!(!valid_relay_session_id(
            &"a".repeat(MAX_RELAY_SESSION_ID_LEN + 1)
        ));
    }
}
