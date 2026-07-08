use std::fmt;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::validate::valid_relay_session_id;
use super::{COMPANION_RELAY_PROTOCOL_VERSION, MAX_RELAY_PAYLOAD_JSON_BYTES};

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
