use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::frame::CompanionRelayPeer;
use super::setup::CompanionRelaySessionConnect;
use super::validate::{
    valid_relay_device_id, valid_relay_pubkey_hex, valid_relay_session_id,
    valid_relay_session_token, valid_relay_ticket_key_id, valid_relay_ticket_mac_hex,
    validate_relay_ticket_hmac_key,
};
use super::{
    COMPANION_RELAY_PROTOCOL_VERSION, COMPANION_RELAY_TICKET_KEYRING_FILE,
    COMPANION_RELAY_TICKET_KEYRING_VERSION, COMPANION_RELAY_TICKET_MAC_ALG_HMAC_SHA256,
    COMPANION_RELAY_WEBSOCKET_TRANSPORT_ID, DEFAULT_COMPANION_RELAY_SESSION_TTL_MS,
    MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS, MIN_RELAY_TICKET_HMAC_KEY_BYTES,
};

type RelayTicketHmacSha256 = Hmac<Sha256>;

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
