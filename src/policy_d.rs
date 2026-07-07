//! Signed `policy.d` verification (P3-1 substrate).
//!
//! Organization policy is allowed to override the user's active profile only
//! after a readonly policy payload is bound to a valid trust-channel manifest.

use crate::policy::PolicyProfile;
use crate::trust::{
    verify_signed_manifest, SignedTrustManifest, TrustAnchor, TrustError, VerifiedTrustManifest,
};
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PolicyDDocument {
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPolicyD {
    pub profile: PolicyProfile,
    pub manifest: VerifiedTrustManifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDError {
    Trust(TrustError),
    Utf8,
    Parse,
    SubjectMismatch { expected: String, actual: String },
    UnknownProfile(String),
    PolicyFileNotReadonly,
    Io(String),
}

impl fmt::Display for PolicyDError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyDError::Trust(e) => write!(f, "{e}"),
            PolicyDError::Utf8 => write!(f, "policy.d payload is not UTF-8"),
            PolicyDError::Parse => write!(f, "policy.d payload is not valid TOML"),
            PolicyDError::SubjectMismatch { expected, actual } => write!(
                f,
                "policy.d manifest subject mismatch: expected {expected}, got {actual}"
            ),
            PolicyDError::UnknownProfile(profile) => {
                write!(f, "policy.d references unknown profile: {profile}")
            }
            PolicyDError::PolicyFileNotReadonly => write!(f, "policy.d file is not readonly"),
            PolicyDError::Io(e) => write!(f, "policy.d I/O failed: {e}"),
        }
    }
}

impl std::error::Error for PolicyDError {}

impl From<TrustError> for PolicyDError {
    fn from(value: TrustError) -> Self {
        PolicyDError::Trust(value)
    }
}

pub fn parse_policy_d(payload: &[u8]) -> Result<PolicyDDocument, PolicyDError> {
    let text = std::str::from_utf8(payload).map_err(|_| PolicyDError::Utf8)?;
    toml::from_str::<PolicyDDocument>(text).map_err(|_| PolicyDError::Parse)
}

pub fn verify_policy_d(
    policy_payload: &[u8],
    signed_manifest: &SignedTrustManifest,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedPolicyD, PolicyDError> {
    let verified_manifest =
        verify_signed_manifest(signed_manifest, anchor, now_unix, Some(policy_payload))?;
    if verified_manifest.manifest.subject != expected_subject {
        return Err(PolicyDError::SubjectMismatch {
            expected: expected_subject.to_string(),
            actual: verified_manifest.manifest.subject.clone(),
        });
    }

    let document = parse_policy_d(policy_payload)?;
    let profile = PolicyProfile::by_name(&document.profile)
        .ok_or_else(|| PolicyDError::UnknownProfile(document.profile.clone()))?;

    Ok(VerifiedPolicyD {
        profile,
        manifest: verified_manifest,
    })
}

pub fn load_verified_policy_d_from_files(
    policy_path: &Path,
    manifest_path: &Path,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedPolicyD, PolicyDError> {
    let metadata = std::fs::metadata(policy_path).map_err(|e| PolicyDError::Io(e.to_string()))?;
    if !metadata.permissions().readonly() {
        return Err(PolicyDError::PolicyFileNotReadonly);
    }

    let policy_payload =
        std::fs::read(policy_path).map_err(|e| PolicyDError::Io(e.to_string()))?;
    let manifest_text =
        std::fs::read_to_string(manifest_path).map_err(|e| PolicyDError::Io(e.to_string()))?;
    let signed_manifest = serde_json::from_str::<SignedTrustManifest>(&manifest_text)
        .map_err(|e| PolicyDError::Io(format!("manifest parse failed: {e}")))?;
    verify_policy_d(
        &policy_payload,
        &signed_manifest,
        anchor,
        now_unix,
        expected_subject,
    )
}

pub fn effective_profile(
    user_profile: PolicyProfile,
    organization_policy: Option<&VerifiedPolicyD>,
) -> PolicyProfile {
    organization_policy
        .map(|policy| policy.profile.clone())
        .unwrap_or(user_profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::{manifest_signing_bytes, sha256_hex, TrustManifest};
    use ed25519_dalek::{Signer, SigningKey};

    fn hex_encode(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }

    fn fixture(subject: &str, payload: &[u8]) -> (SignedTrustManifest, TrustAnchor) {
        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        let manifest = TrustManifest {
            manifest_id: "policy-manifest-001".to_string(),
            subject: subject.to_string(),
            version: 7,
            issued_at_unix: 1_700_000_000,
            expires_at_unix: 1_800_000_000,
            payload_sha256: sha256_hex(payload),
        };
        let signature = signing_key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let signed = SignedTrustManifest {
            key_id: "org-policy-root".to_string(),
            manifest,
            signature: hex_encode(&signature.to_bytes()),
        };
        let anchor = TrustAnchor {
            key_id: signed.key_id.clone(),
            public_key_hex: hex_encode(&signing_key.verifying_key().to_bytes()),
            min_version: 7,
        };
        (signed, anchor)
    }

    #[test]
    fn verified_policy_overrides_user_profile() {
        let payload = b"profile = \"paranoid\"\n";
        let (signed, anchor) = fixture("policy.d/org.toml", payload);

        let verified =
            verify_policy_d(payload, &signed, &anchor, 1_750_000_000, "policy.d/org.toml")
                .unwrap();
        let effective = effective_profile(PolicyProfile::balanced(), Some(&verified));

        assert_eq!(verified.profile.name, "paranoid");
        assert_eq!(effective.name, "paranoid");
    }

    #[test]
    fn rejects_manifest_for_different_subject() {
        let payload = b"profile = \"paranoid\"\n";
        let (signed, anchor) = fixture("policy.d/other.toml", payload);

        assert!(matches!(
            verify_policy_d(payload, &signed, &anchor, 1_750_000_000, "policy.d/org.toml"),
            Err(PolicyDError::SubjectMismatch { .. })
        ));
    }

    #[test]
    fn rejects_unsigned_or_modified_policy_payload() {
        let payload = b"profile = \"paranoid\"\n";
        let (signed, anchor) = fixture("policy.d/org.toml", payload);

        assert!(matches!(
            verify_policy_d(
                b"profile = \"balanced\"\n",
                &signed,
                &anchor,
                1_750_000_000,
                "policy.d/org.toml"
            ),
            Err(PolicyDError::Trust(TrustError::PayloadDigestMismatch))
        ));
    }

    #[test]
    fn rejects_unknown_policy_profile() {
        let payload = b"profile = \"root-only\"\n";
        let (signed, anchor) = fixture("policy.d/org.toml", payload);

        assert_eq!(
            verify_policy_d(payload, &signed, &anchor, 1_750_000_000, "policy.d/org.toml")
                .unwrap_err(),
            PolicyDError::UnknownProfile("root-only".to_string())
        );
    }
}
