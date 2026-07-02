//! Remote companion transport catalog.
//!
//! This module is intentionally status/configuration plumbing only. Planned
//! modes are not selectable runtime transports until their security and
//! evidence gates exist.

use std::fmt;
use std::str::FromStr;

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
}
