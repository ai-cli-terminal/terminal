use anyhow::{bail, Result};

use super::{
    MAX_RELAY_DEVICE_ID_LEN, MAX_RELAY_ENDPOINT_URL_LEN, MAX_RELAY_PRIVATE_NETWORK_NAME_LEN,
    MAX_RELAY_SESSION_ID_LEN, MAX_RELAY_SESSION_TOKEN_LEN, MAX_RELAY_TICKET_KEY_ID_LEN,
    MIN_RELAY_SESSION_TOKEN_LEN, MIN_RELAY_TICKET_HMAC_KEY_BYTES,
};

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

pub fn valid_relay_private_network_name(value: &str) -> bool {
    value.len() >= 3
        && value.len() <= MAX_RELAY_PRIVATE_NETWORK_NAME_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

pub(super) fn valid_relay_pubkey_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(super) fn valid_relay_ticket_mac_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(super) fn valid_relay_ticket_key_id(value: &str) -> bool {
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

pub(super) fn validate_relay_ticket_hmac_key(secret: &[u8]) -> Result<()> {
    if secret.len() < MIN_RELAY_TICKET_HMAC_KEY_BYTES {
        bail!("relay ticket hmac key too short");
    }
    Ok(())
}
