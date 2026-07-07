//! Signed binary release manifest verification (P3-1 substrate).
//!
//! Release installers and update flows can use this module to require that
//! downloaded artifacts appear in an organization-signed manifest before they
//! are installed. This first slice is intentionally read-only: it verifies the
//! manifest and artifact hashes, while install/update enforcement is layered on
//! top in a later slice.

use crate::policy_d::{self, TrustAnchorSource};
use crate::trust::{
    verify_signed_manifest, SignedTrustManifest, TrustAnchor, TrustError, VerifiedTrustManifest,
};
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_BINARY_MANIFEST_SUBJECT: &str = "release/binary-manifest.json";
pub const DEFAULT_BINARY_MANIFEST_FILE: &str = "binary-manifest.json";
pub const DEFAULT_BINARY_MANIFEST_SIGNATURE_FILE: &str = "binary-manifest.manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct BinaryManifestDocument {
    #[serde(default)]
    pub artifacts: Vec<BinaryArtifactEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct BinaryArtifactEntry {
    pub name: String,
    pub sha256: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBinaryManifest {
    pub manifest: VerifiedTrustManifest,
    pub artifacts: Vec<VerifiedBinaryArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBinaryArtifact {
    pub name: String,
    pub sha256: String,
    pub version: Option<String>,
    pub platform: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBinaryManifest {
    pub verified: VerifiedBinaryManifest,
    pub payload_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
    pub anchor_source: TrustAnchorSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryManifestPaths {
    pub payload_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
    pub anchor_source: TrustAnchorSource,
    pub expected_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryManifestError {
    Trust(TrustError),
    Parse,
    SubjectMismatch { expected: String, actual: String },
    InvalidArtifactName { index: usize },
    InvalidArtifactHash { name: String },
    ArtifactMismatch { name: String, sha256: String },
    Io(String),
    Clock(String),
}

impl fmt::Display for BinaryManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryManifestError::Trust(e) => write!(f, "{e}"),
            BinaryManifestError::Parse => write!(f, "binary manifest payload is not valid JSON"),
            BinaryManifestError::SubjectMismatch { expected, actual } => write!(
                f,
                "binary manifest subject mismatch: expected {expected}, got {actual}"
            ),
            BinaryManifestError::InvalidArtifactName { index } => {
                write!(
                    f,
                    "binary manifest artifact at index {index} has an empty name"
                )
            }
            BinaryManifestError::InvalidArtifactHash { name } => write!(
                f,
                "binary manifest hash for {name} is not a SHA-256 hex digest"
            ),
            BinaryManifestError::ArtifactMismatch { name, sha256 } => write!(
                f,
                "artifact does not match signed binary manifest: name={name} sha256={sha256}"
            ),
            BinaryManifestError::Io(e) => write!(f, "binary manifest I/O failed: {e}"),
            BinaryManifestError::Clock(e) => write!(f, "binary manifest clock unavailable: {e}"),
        }
    }
}

impl std::error::Error for BinaryManifestError {}

impl From<TrustError> for BinaryManifestError {
    fn from(value: TrustError) -> Self {
        BinaryManifestError::Trust(value)
    }
}

impl VerifiedBinaryManifest {
    pub fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    pub fn allows_artifact(&self, name: &str, sha256: &str) -> bool {
        self.artifacts
            .iter()
            .any(|artifact| artifact.name == name && artifact.sha256.eq_ignore_ascii_case(sha256))
    }
}

pub fn default_binary_manifest_paths() -> Result<BinaryManifestPaths, BinaryManifestError> {
    let dir = crate::config::config_dir()
        .map_err(|e| BinaryManifestError::Io(format!("config dir unavailable: {e}")))?
        .join("release");
    let anchor_paths = policy_d::default_policy_d_paths()
        .map_err(|e| BinaryManifestError::Io(format!("organization anchor unavailable: {e}")))?;
    Ok(BinaryManifestPaths {
        payload_path: dir.join(DEFAULT_BINARY_MANIFEST_FILE),
        manifest_path: dir.join(DEFAULT_BINARY_MANIFEST_SIGNATURE_FILE),
        anchor_path: anchor_paths.anchor_path,
        anchor_source: anchor_paths.anchor_source,
        expected_subject: DEFAULT_BINARY_MANIFEST_SUBJECT.to_string(),
    })
}

pub fn verify_binary_manifest(
    manifest_payload: &[u8],
    signed_manifest: &SignedTrustManifest,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedBinaryManifest, BinaryManifestError> {
    let verified_manifest =
        verify_signed_manifest(signed_manifest, anchor, now_unix, Some(manifest_payload))?;
    if verified_manifest.manifest.subject != expected_subject {
        return Err(BinaryManifestError::SubjectMismatch {
            expected: expected_subject.to_string(),
            actual: verified_manifest.manifest.subject.clone(),
        });
    }

    let document = serde_json::from_slice::<BinaryManifestDocument>(manifest_payload)
        .map_err(|_| BinaryManifestError::Parse)?;
    let mut artifacts = Vec::with_capacity(document.artifacts.len());
    for (index, artifact) in document.artifacts.into_iter().enumerate() {
        if artifact.name.trim().is_empty() {
            return Err(BinaryManifestError::InvalidArtifactName { index });
        }
        if !is_sha256_hex(&artifact.sha256) {
            return Err(BinaryManifestError::InvalidArtifactHash {
                name: artifact.name,
            });
        }
        artifacts.push(VerifiedBinaryArtifact {
            name: artifact.name,
            sha256: artifact.sha256,
            version: artifact.version,
            platform: artifact.platform,
            kind: artifact.kind,
        });
    }

    Ok(VerifiedBinaryManifest {
        manifest: verified_manifest,
        artifacts,
    })
}

pub fn load_verified_binary_manifest_from_files(
    payload_path: &Path,
    manifest_path: &Path,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedBinaryManifest, BinaryManifestError> {
    let manifest_payload =
        std::fs::read(payload_path).map_err(|e| BinaryManifestError::Io(e.to_string()))?;
    let signed_text = std::fs::read_to_string(manifest_path)
        .map_err(|e| BinaryManifestError::Io(e.to_string()))?;
    let signed_manifest = serde_json::from_str::<SignedTrustManifest>(&signed_text)
        .map_err(|e| BinaryManifestError::Io(format!("manifest parse failed: {e}")))?;
    verify_binary_manifest(
        &manifest_payload,
        &signed_manifest,
        anchor,
        now_unix,
        expected_subject,
    )
}

pub fn load_default_binary_manifest_from_files(
    payload_path: &Path,
    manifest_path: &Path,
    now_unix: i64,
) -> Result<VerifiedBinaryManifest, BinaryManifestError> {
    let paths = default_binary_manifest_paths()?;
    let anchor = policy_d::load_trust_anchor_from(&paths.anchor_path)
        .map_err(|e| BinaryManifestError::Io(format!("anchor load failed: {e}")))?;
    load_verified_binary_manifest_from_files(
        payload_path,
        manifest_path,
        &anchor,
        now_unix,
        &paths.expected_subject,
    )
}

pub fn load_default_organization_binary_manifest(
    now_unix: i64,
) -> Result<Option<LoadedBinaryManifest>, BinaryManifestError> {
    let paths = default_binary_manifest_paths()?;
    let payload_exists = paths.payload_path.exists();
    let manifest_exists = paths.manifest_path.exists();
    let anchor_exists = paths.anchor_path.exists();

    if !payload_exists && !manifest_exists {
        return Ok(None);
    }
    if !payload_exists || !manifest_exists || !anchor_exists {
        return Err(BinaryManifestError::Io(format!(
            "incomplete binary manifest set: payload={} manifest={} anchor={}",
            payload_exists, manifest_exists, anchor_exists
        )));
    }

    let anchor = policy_d::load_trust_anchor_from(&paths.anchor_path)
        .map_err(|e| BinaryManifestError::Io(format!("anchor load failed: {e}")))?;
    let verified = load_verified_binary_manifest_from_files(
        &paths.payload_path,
        &paths.manifest_path,
        &anchor,
        now_unix,
        &paths.expected_subject,
    )?;
    Ok(Some(LoadedBinaryManifest {
        verified,
        payload_path: paths.payload_path,
        manifest_path: paths.manifest_path,
        anchor_path: paths.anchor_path,
        anchor_source: paths.anchor_source,
    }))
}

pub fn artifact_sha256_from_file(path: &Path) -> Result<String, BinaryManifestError> {
    let bytes = std::fs::read(path).map_err(|e| BinaryManifestError::Io(e.to_string()))?;
    Ok(crate::trust::sha256_hex(&bytes))
}

pub fn verify_artifact_against_manifest(
    verified: &VerifiedBinaryManifest,
    artifact_name: &str,
    artifact_path: &Path,
) -> Result<String, BinaryManifestError> {
    let sha256 = artifact_sha256_from_file(artifact_path)?;
    if !verified.allows_artifact(artifact_name, &sha256) {
        return Err(BinaryManifestError::ArtifactMismatch {
            name: artifact_name.to_string(),
            sha256,
        });
    }
    Ok(sha256)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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

    fn signed_binary_manifest(subject: &str, payload: &[u8]) -> (SignedTrustManifest, TrustAnchor) {
        let signing_key = SigningKey::from_bytes(&[14u8; 32]);
        let manifest = TrustManifest {
            manifest_id: "binary-manifest-001".to_string(),
            subject: subject.to_string(),
            version: 5,
            issued_at_unix: 1_700_000_000,
            expires_at_unix: 1_800_000_000,
            payload_sha256: sha256_hex(payload),
        };
        let signature = signing_key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let signed = SignedTrustManifest {
            key_id: "org-release-root".to_string(),
            manifest,
            signature: hex_encode(&signature.to_bytes()),
        };
        let anchor = TrustAnchor {
            key_id: signed.key_id.clone(),
            public_key_hex: hex_encode(&signing_key.verifying_key().to_bytes()),
            min_version: 5,
        };
        (signed, anchor)
    }

    #[test]
    fn verifies_signed_manifest_and_matches_artifact() {
        let artifact_payload = b"release artifact bytes";
        let artifact_hash = sha256_hex(artifact_payload);
        let payload = format!(
            "{{\"artifacts\":[{{\"name\":\"ai-linux-x86_64\",\"sha256\":\"{}\",\
             \"version\":\"0.3.4\",\"platform\":\"linux-x86_64\",\"kind\":\"cli\"}}]}}",
            artifact_hash
        );
        let (signed, anchor) =
            signed_binary_manifest(DEFAULT_BINARY_MANIFEST_SUBJECT, payload.as_bytes());

        let manifest = verify_binary_manifest(
            payload.as_bytes(),
            &signed,
            &anchor,
            1_750_000_000,
            DEFAULT_BINARY_MANIFEST_SUBJECT,
        )
        .unwrap();

        assert_eq!(manifest.artifact_count(), 1);
        assert!(manifest.allows_artifact("ai-linux-x86_64", &artifact_hash.to_uppercase()));
        assert!(!manifest.allows_artifact("ash-linux-x86_64", &artifact_hash));
    }

    #[test]
    fn rejects_modified_manifest_payload() {
        let payload = b"{\"artifacts\":[]}";
        let (signed, anchor) = signed_binary_manifest(DEFAULT_BINARY_MANIFEST_SUBJECT, payload);
        let modified_payload = format!(
            "{{\"artifacts\":[{{\"name\":\"ai\",\"sha256\":\"{}\"}}]}}",
            "a".repeat(64)
        );

        assert!(matches!(
            verify_binary_manifest(
                modified_payload.as_bytes(),
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_BINARY_MANIFEST_SUBJECT
            ),
            Err(BinaryManifestError::Trust(
                TrustError::PayloadDigestMismatch
            ))
        ));
    }

    #[test]
    fn rejects_wrong_subject() {
        let payload = b"{\"artifacts\":[]}";
        let (signed, anchor) = signed_binary_manifest("release/other.json", payload);

        assert!(matches!(
            verify_binary_manifest(
                payload,
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_BINARY_MANIFEST_SUBJECT
            ),
            Err(BinaryManifestError::SubjectMismatch { .. })
        ));
    }

    #[test]
    fn rejects_invalid_artifact_fields() {
        let bad_name = format!(
            "{{\"artifacts\":[{{\"name\":\"   \",\"sha256\":\"{}\"}}]}}",
            "a".repeat(64)
        );
        let (signed, anchor) =
            signed_binary_manifest(DEFAULT_BINARY_MANIFEST_SUBJECT, bad_name.as_bytes());
        assert!(matches!(
            verify_binary_manifest(
                bad_name.as_bytes(),
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_BINARY_MANIFEST_SUBJECT
            ),
            Err(BinaryManifestError::InvalidArtifactName { .. })
        ));

        let bad_hash = b"{\"artifacts\":[{\"name\":\"ai\",\"sha256\":\"nope\"}]}";
        let (signed, anchor) = signed_binary_manifest(DEFAULT_BINARY_MANIFEST_SUBJECT, bad_hash);
        assert!(matches!(
            verify_binary_manifest(
                bad_hash,
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_BINARY_MANIFEST_SUBJECT
            ),
            Err(BinaryManifestError::InvalidArtifactHash { .. })
        ));
    }
}
