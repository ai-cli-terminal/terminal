//! Remote companion transport catalog.
//!
//! This module is intentionally status/configuration plumbing only. Planned
//! modes are not selectable runtime transports until their security and
//! evidence gates exist.

pub const COMPANION_RELAY_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_COMPANION_RELAY_FRAME_TTL_MS: u64 = 30_000;
pub const DEFAULT_COMPANION_RELAY_SESSION_TTL_MS: u64 = 5 * 60 * 1000;
pub const COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256: &str = "hmac-sha256";
pub const MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS: usize = 3;
pub const COMPANION_RELAY_TICKET_KEYRING_FILE: &str = "remote-relay-ticket-keys.json";
pub const COMPANION_RELAY_TICKET_KEYRING_VERSION: u32 = 1;
pub const COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED: &str = "self-hosted";
pub const COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK: &str = "private-network";
pub const COMPANION_RELAY_SETUP_TRANSPORT_MODE: &str = "relay";
const MAX_RELAY_SESSION_ID_LEN: usize = 96;
const MIN_RELAY_SESSION_TOKEN_LEN: usize = 32;
const MAX_RELAY_SESSION_TOKEN_LEN: usize = 128;
const MAX_RELAY_DEVICE_ID_LEN: usize = 96;
const MAX_RELAY_PRIVATE_NETWORK_NAME_LEN: usize = 96;
const MAX_RELAY_TICKET_KEY_ID_LEN: usize = 64;
const MAX_RELAY_ENDPOINT_URL_LEN: usize = 2048;
const MAX_RELAY_PAYLOAD_JSON_BYTES: usize = 1 << 20;
const MIN_RELAY_TICKET_HMAC_KEY_BYTES: usize = 32;
const COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID: &str = "websocket";

mod frame;
mod loopback;
mod mode;
mod setup;
#[cfg(test)]
mod tests;
mod ticket;
mod validate;

pub use frame::{CompanionRelayFrame, CompanionRelayPeer, CompanionRelayRouteEnvelope};
pub use loopback::{CompanionRelayEndpoint, CompanionRelayLoopback, CompanionRelayLoopbackStats};
pub use mode::{
    active_product_mode, all_modes, CompanionTransportDescriptor, CompanionTransportMode,
    CompanionTransportReadiness, ALL_COMPANION_TRANSPORT_MODES,
};
pub use setup::{
    issue_self_hosted_relay_runtime_setup, CompanionRelayCompanionIdentity,
    CompanionRelaySelfHostedRuntimeSetup, CompanionRelaySelfHostedSetupInput,
    CompanionRelaySessionConnect,
};
pub use ticket::{
    companion_relay_ticket_keyring_path, load_companion_relay_ticket_keyring,
    load_or_create_companion_relay_ticket_keyring, new_companion_relay_ticket_keyring,
    relay_session_ticket_hmac_sha256_hex, relay_session_ticket_signing_payload,
    save_companion_relay_ticket_keyring, CompanionRelaySessionTicket,
    CompanionRelaySessionTicketInput, CompanionRelaySignedSessionTicket,
    CompanionRelayTicketHmacKeyRecord, CompanionRelayTicketIssuer,
    CompanionRelayTicketKeyringRecord,
};
pub use validate::{
    valid_relay_device_id, valid_relay_private_network_name, valid_relay_session_id,
    valid_relay_session_token, valid_relay_websocket_endpoint_url,
};
