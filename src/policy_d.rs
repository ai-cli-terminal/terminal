//! Signed `policy.d` verification (P3-1 substrate).
//!
//! Organization policy is allowed to override the user's active profile only
//! after a readonly policy payload is bound to a valid trust-channel manifest.

use crate::policy::PolicyProfile;
use crate::trust::{
    verify_signed_manifest, SignedTrustManifest, TrustAnchor, TrustError, VerifiedTrustManifest,
};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_POLICY_SUBJECT: &str = "policy.d/org.toml";
pub const DEFAULT_POLICY_FILE: &str = "org.toml";
pub const DEFAULT_POLICY_MANIFEST_FILE: &str = "org.toml.manifest.json";
pub const DEFAULT_POLICY_ANCHOR_FILE: &str = "org-root.json";

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
pub struct LoadedPolicyD {
    pub verified: VerifiedPolicyD,
    pub policy_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    UserActiveProfile {
        profile: String,
    },
    OrganizationPolicy {
        subject: String,
        version: u64,
        manifest_id: String,
        policy_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePolicy {
    pub profile: PolicyProfile,
    pub source: PolicySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDPaths {
    pub policy_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
    pub expected_subject: String,
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
    Clock(String),
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
            PolicyDError::Clock(e) => write!(f, "policy.d clock unavailable: {e}"),
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

pub fn default_policy_d_paths() -> Result<PolicyDPaths, PolicyDError> {
    let dir = crate::config::config_dir()
        .map_err(|e| PolicyDError::Io(format!("config dir unavailable: {e}")))?
        .join("policy.d");
    Ok(PolicyDPaths {
        policy_path: dir.join(DEFAULT_POLICY_FILE),
        manifest_path: dir.join(DEFAULT_POLICY_MANIFEST_FILE),
        anchor_path: dir.join(DEFAULT_POLICY_ANCHOR_FILE),
        expected_subject: DEFAULT_POLICY_SUBJECT.to_string(),
    })
}

pub fn current_unix_time() -> Result<i64, PolicyDError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| PolicyDError::Clock(e.to_string()))?;
    i64::try_from(duration.as_secs()).map_err(|e| PolicyDError::Clock(e.to_string()))
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

    let policy_payload = std::fs::read(policy_path).map_err(|e| PolicyDError::Io(e.to_string()))?;
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

pub fn load_trust_anchor_from(path: &Path) -> Result<TrustAnchor, PolicyDError> {
    let text = std::fs::read_to_string(path).map_err(|e| PolicyDError::Io(e.to_string()))?;
    serde_json::from_str::<TrustAnchor>(&text)
        .map_err(|e| PolicyDError::Io(format!("anchor parse failed: {e}")))
}

pub fn load_default_organization_policy(
    now_unix: i64,
) -> Result<Option<LoadedPolicyD>, PolicyDError> {
    let paths = default_policy_d_paths()?;
    let policy_exists = paths.policy_path.exists();
    let manifest_exists = paths.manifest_path.exists();
    let anchor_exists = paths.anchor_path.exists();

    if !policy_exists && !manifest_exists && !anchor_exists {
        return Ok(None);
    }
    if !policy_exists || !manifest_exists || !anchor_exists {
        return Err(PolicyDError::Io(format!(
            "incomplete policy.d set: policy={} manifest={} anchor={}",
            policy_exists, manifest_exists, anchor_exists
        )));
    }

    let anchor = load_trust_anchor_from(&paths.anchor_path)?;
    let verified = load_verified_policy_d_from_files(
        &paths.policy_path,
        &paths.manifest_path,
        &anchor,
        now_unix,
        &paths.expected_subject,
    )?;
    Ok(Some(LoadedPolicyD {
        verified,
        policy_path: paths.policy_path,
        manifest_path: paths.manifest_path,
        anchor_path: paths.anchor_path,
    }))
}

pub fn effective_profile(
    user_profile: PolicyProfile,
    organization_policy: Option<&VerifiedPolicyD>,
) -> PolicyProfile {
    organization_policy
        .map(|policy| policy.profile.clone())
        .unwrap_or(user_profile)
}

pub fn resolve_effective_profile(
    user_profile: PolicyProfile,
) -> Result<EffectivePolicy, PolicyDError> {
    match load_default_organization_policy(current_unix_time()?)? {
        Some(loaded) => Ok(EffectivePolicy {
            profile: loaded.verified.profile.clone(),
            source: PolicySource::OrganizationPolicy {
                subject: loaded.verified.manifest.manifest.subject.clone(),
                version: loaded.verified.manifest.manifest.version,
                manifest_id: loaded.verified.manifest.manifest.manifest_id.clone(),
                policy_path: loaded.policy_path,
            },
        }),
        None => Ok(EffectivePolicy {
            profile: user_profile.clone(),
            source: PolicySource::UserActiveProfile {
                profile: user_profile.name.to_string(),
            },
        }),
    }
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

        let verified = verify_policy_d(
            payload,
            &signed,
            &anchor,
            1_750_000_000,
            "policy.d/org.toml",
        )
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
            verify_policy_d(
                payload,
                &signed,
                &anchor,
                1_750_000_000,
                "policy.d/org.toml"
            ),
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
            verify_policy_d(
                payload,
                &signed,
                &anchor,
                1_750_000_000,
                "policy.d/org.toml"
            )
            .unwrap_err(),
            PolicyDError::UnknownProfile("root-only".to_string())
        );
    }

    #[test]
    fn effective_policy_records_user_source_without_org_policy() {
        let user = PolicyProfile::balanced();
        let effective = EffectivePolicy {
            profile: user.clone(),
            source: PolicySource::UserActiveProfile {
                profile: user.name.to_string(),
            },
        };

        assert_eq!(effective.profile.name, "balanced");
        assert_eq!(
            effective.source,
            PolicySource::UserActiveProfile {
                profile: "balanced".to_string()
            }
        );
    }
}
