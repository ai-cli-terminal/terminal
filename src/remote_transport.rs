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

#[derive(Debug, Default)]
pub struct CompanionRelayLoopback {
    sessions: HashMap<String, CompanionRelaySessionQueue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionRelayLoopbackStats {
    pub session_count: usize,
    pub queued_frames: usize,
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
}
