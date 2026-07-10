//! Trust channel manifest verification core (P3-1 substrate).
//!
//! This module intentionally stays storage- and OS-agnostic. Higher layers can
//! load anchors from an OS trust store, MDM profile, or readonly policy path and
//! then call this deterministic verifier.

use ed25519_dalek::Signer as _;
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustManifest {
    pub manifest_id: String,
    pub subject: String,
    pub version: u64,
    pub issued_at_unix: i64,
    pub expires_at_unix: i64,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTrustManifest {
    pub key_id: String,
    pub manifest: TrustManifest,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustAnchor {
    pub key_id: String,
    pub public_key_hex: String,
    pub min_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedTrustManifest {
    pub key_id: String,
    pub manifest: TrustManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    AnchorMismatch,
    InvalidValidityWindow,
    NotYetValid,
    Expired,
    Rollback,
    InvalidHex(&'static str),
    InvalidPublicKey,
    InvalidSignature,
    PayloadDigestMismatch,
    Serialization,
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustError::AnchorMismatch => write!(f, "trust manifest key id does not match anchor"),
            TrustError::InvalidValidityWindow => {
                write!(f, "trust manifest validity window is invalid")
            }
            TrustError::NotYetValid => write!(f, "trust manifest is not yet valid"),
            TrustError::Expired => write!(f, "trust manifest is expired"),
            TrustError::Rollback => write!(f, "trust manifest version is below anchor minimum"),
            TrustError::InvalidHex(field) => write!(f, "trust manifest has invalid hex in {field}"),
            TrustError::InvalidPublicKey => write!(f, "trust anchor public key is invalid"),
            TrustError::InvalidSignature => write!(f, "trust manifest signature is invalid"),
            TrustError::PayloadDigestMismatch => {
                write!(f, "trust manifest payload digest mismatch")
            }
            TrustError::Serialization => write!(f, "trust manifest serialization failed"),
        }
    }
}

impl std::error::Error for TrustError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_encode(&digest)
}

pub fn manifest_signing_bytes(manifest: &TrustManifest) -> Result<Vec<u8>, TrustError> {
    serde_json::to_vec(manifest).map_err(|_| TrustError::Serialization)
}

pub fn verify_signed_manifest(
    signed: &SignedTrustManifest,
    anchor: &TrustAnchor,
    now_unix: i64,
    payload: Option<&[u8]>,
) -> Result<VerifiedTrustManifest, TrustError> {
    if signed.key_id != anchor.key_id {
        return Err(TrustError::AnchorMismatch);
    }
    if signed.manifest.expires_at_unix <= signed.manifest.issued_at_unix {
        return Err(TrustError::InvalidValidityWindow);
    }
    if now_unix < signed.manifest.issued_at_unix {
        return Err(TrustError::NotYetValid);
    }
    if now_unix > signed.manifest.expires_at_unix {
        return Err(TrustError::Expired);
    }
    if signed.manifest.version < anchor.min_version {
        return Err(TrustError::Rollback);
    }

    let expected_payload_digest =
        decode_hex_exact::<32>(&signed.manifest.payload_sha256, "manifest.payload_sha256")?;
    if let Some(payload) = payload {
        let actual_payload_digest: [u8; 32] = Sha256::digest(payload).into();
        if expected_payload_digest != actual_payload_digest {
            return Err(TrustError::PayloadDigestMismatch);
        }
    }

    let public_key = decode_hex_exact::<32>(&anchor.public_key_hex, "anchor.public_key_hex")?;
    let signature = decode_hex_exact::<64>(&signed.signature, "signature")?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| TrustError::InvalidPublicKey)?;
    let signing_bytes = manifest_signing_bytes(&signed.manifest)?;
    verifying_key
        .verify(&signing_bytes, &Signature::from_bytes(&signature))
        .map_err(|_| TrustError::InvalidSignature)?;

    Ok(VerifiedTrustManifest {
        key_id: signed.key_id.clone(),
        manifest: signed.manifest.clone(),
    })
}

pub fn sign_manifest(
    manifest: TrustManifest,
    key_id: String,
    private_key_hex: &str,
) -> Result<SignedTrustManifest, TrustError> {
    let private_key = decode_hex_exact::<32>(private_key_hex, "private_key")?;
    let signing_key = SigningKey::from_bytes(&private_key);
    let signature = signing_key.sign(&manifest_signing_bytes(&manifest)?);
    Ok(SignedTrustManifest {
        key_id,
        manifest,
        signature: hex_encode(&signature.to_bytes()),
    })
}

fn decode_hex_exact<const N: usize>(hex: &str, field: &'static str) -> Result<[u8; N], TrustError> {
    if hex.len() != N * 2 {
        return Err(TrustError::InvalidHex(field));
    }
    let mut out = [0u8; N];
    let bytes = hex.as_bytes();
    for i in 0..N {
        let hi = hex_nibble(bytes[i * 2]).ok_or(TrustError::InvalidHex(field))?;
        let lo = hex_nibble(bytes[i * 2 + 1]).ok_or(TrustError::InvalidHex(field))?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn fixture(payload: &[u8]) -> (SignedTrustManifest, TrustAnchor) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let manifest = TrustManifest {
            manifest_id: "manifest-001".to_string(),
            subject: "policy.d/org.toml".to_string(),
            version: 42,
            issued_at_unix: 1_700_000_000,
            expires_at_unix: 1_800_000_000,
            payload_sha256: sha256_hex(payload),
        };
        let signature = signing_key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let signed = SignedTrustManifest {
            key_id: "org-root-2026".to_string(),
            manifest,
            signature: hex_encode(&signature.to_bytes()),
        };
        let anchor = TrustAnchor {
            key_id: signed.key_id.clone(),
            public_key_hex: hex_encode(&signing_key.verifying_key().to_bytes()),
            min_version: 40,
        };
        (signed, anchor)
    }

    #[test]
    fn verifies_signature_window_version_and_payload_digest() {
        let payload = b"profile = 'paranoid'";
        let (signed, anchor) = fixture(payload);

        let verified =
            verify_signed_manifest(&signed, &anchor, 1_750_000_000, Some(payload)).unwrap();

        assert_eq!(verified.key_id, "org-root-2026");
        assert_eq!(verified.manifest.subject, "policy.d/org.toml");
    }

    #[test]
    fn signs_manifest_with_private_key_hex() {
        let payload = b"{\"artifacts\":[]}";
        let signing_key = SigningKey::from_bytes(&[8u8; 32]);
        let manifest = TrustManifest {
            manifest_id: "binary-manifest-001".to_string(),
            subject: "release/binary-manifest.json".to_string(),
            version: 10,
            issued_at_unix: 1_700_000_000,
            expires_at_unix: 1_800_000_000,
            payload_sha256: sha256_hex(payload),
        };
        let signed = sign_manifest(
            manifest,
            "release-root".to_string(),
            &hex_encode(&signing_key.to_bytes()),
        )
        .unwrap();
        let anchor = TrustAnchor {
            key_id: signed.key_id.clone(),
            public_key_hex: hex_encode(&signing_key.verifying_key().to_bytes()),
            min_version: 10,
        };

        let verified =
            verify_signed_manifest(&signed, &anchor, 1_750_000_000, Some(payload)).unwrap();

        assert_eq!(verified.key_id, "release-root");
        assert_eq!(verified.manifest.subject, "release/binary-manifest.json");
    }

    #[test]
    fn rejects_forged_manifest() {
        let payload = b"profile = 'paranoid'";
        let (mut signed, anchor) = fixture(payload);
        signed.manifest.subject = "policy.d/attacker.toml".to_string();

        assert_eq!(
            verify_signed_manifest(&signed, &anchor, 1_750_000_000, Some(payload)).unwrap_err(),
            TrustError::InvalidSignature
        );
    }

    #[test]
    fn rejects_expired_manifest() {
        let payload = b"profile = 'paranoid'";
        let (signed, anchor) = fixture(payload);

        assert_eq!(
            verify_signed_manifest(&signed, &anchor, 1_900_000_000, Some(payload)).unwrap_err(),
            TrustError::Expired
        );
    }

    #[test]
    fn rejects_rollback_below_anchor_min_version() {
        let payload = b"profile = 'paranoid'";
        let (signed, mut anchor) = fixture(payload);
        anchor.min_version = 43;

        assert_eq!(
            verify_signed_manifest(&signed, &anchor, 1_750_000_000, Some(payload)).unwrap_err(),
            TrustError::Rollback
        );
    }

    #[test]
    fn rejects_payload_digest_mismatch() {
        let payload = b"profile = 'paranoid'";
        let (signed, anchor) = fixture(payload);

        assert_eq!(
            verify_signed_manifest(
                &signed,
                &anchor,
                1_750_000_000,
                Some(b"profile = 'balanced'")
            )
            .unwrap_err(),
            TrustError::PayloadDigestMismatch
        );
    }
}
