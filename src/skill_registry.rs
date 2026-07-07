//! Signed organization skill registry (P3-1 substrate).
//!
//! A signed registry can restrict discovered skills to organization-approved
//! content hashes. Without a registry, existing local skill discovery behavior
//! is unchanged.

use crate::policy_d::{self, TrustAnchorSource};
use crate::skill::Skill;
use crate::trust::{
    verify_signed_manifest, SignedTrustManifest, TrustAnchor, TrustError, VerifiedTrustManifest,
};
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_SKILL_REGISTRY_SUBJECT: &str = "skills/org-registry.json";
pub const DEFAULT_SKILL_REGISTRY_FILE: &str = "org-registry.json";
pub const DEFAULT_SKILL_REGISTRY_MANIFEST_FILE: &str = "org-registry.manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct SkillRegistryDocument {
    #[serde(default)]
    pub skills: Vec<SkillRegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct SkillRegistryEntry {
    pub name: String,
    pub skill_sha256: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSkillRegistry {
    pub manifest: VerifiedTrustManifest,
    pub entries: Vec<VerifiedSkillRegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSkillRegistryEntry {
    pub name: String,
    pub skill_sha256: String,
    pub status: SkillRegistryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRegistryStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSkillRegistry {
    pub verified: VerifiedSkillRegistry,
    pub registry_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
    pub anchor_source: TrustAnchorSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRegistryPaths {
    pub registry_path: PathBuf,
    pub manifest_path: PathBuf,
    pub anchor_path: PathBuf,
    pub anchor_source: TrustAnchorSource,
    pub expected_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRegistryUpdateCandidate {
    pub verified: VerifiedSkillRegistry,
    pub registry_payload: Vec<u8>,
    pub manifest_payload: Vec<u8>,
    pub target_paths: SkillRegistryPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillRegistryError {
    Trust(TrustError),
    Parse,
    SubjectMismatch { expected: String, actual: String },
    UnknownStatus(String),
    InvalidSkillHash { name: String },
    Io(String),
    Clock(String),
}

impl fmt::Display for SkillRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillRegistryError::Trust(e) => write!(f, "{e}"),
            SkillRegistryError::Parse => write!(f, "skill registry payload is not valid JSON"),
            SkillRegistryError::SubjectMismatch { expected, actual } => write!(
                f,
                "skill registry manifest subject mismatch: expected {expected}, got {actual}"
            ),
            SkillRegistryError::UnknownStatus(status) => {
                write!(f, "skill registry references unknown status: {status}")
            }
            SkillRegistryError::InvalidSkillHash { name } => {
                write!(
                    f,
                    "skill registry hash for {name} is not a SHA-256 hex digest"
                )
            }
            SkillRegistryError::Io(e) => write!(f, "skill registry I/O failed: {e}"),
            SkillRegistryError::Clock(e) => write!(f, "skill registry clock unavailable: {e}"),
        }
    }
}

impl std::error::Error for SkillRegistryError {}

impl From<TrustError> for SkillRegistryError {
    fn from(value: TrustError) -> Self {
        SkillRegistryError::Trust(value)
    }
}

impl SkillRegistryStatus {
    fn parse(raw: &str) -> Result<Self, SkillRegistryError> {
        match raw {
            "active" => Ok(SkillRegistryStatus::Active),
            "revoked" => Ok(SkillRegistryStatus::Revoked),
            other => Err(SkillRegistryError::UnknownStatus(other.to_string())),
        }
    }
}

impl VerifiedSkillRegistry {
    pub fn allows_skill(&self, skill: &Skill) -> bool {
        let hash = skill_content_sha256(skill);
        self.entries.iter().any(|entry| {
            entry.status == SkillRegistryStatus::Active
                && entry.name == skill.name
                && entry.skill_sha256 == hash
        })
    }

    pub fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == SkillRegistryStatus::Active)
            .count()
    }

    pub fn revoked_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == SkillRegistryStatus::Revoked)
            .count()
    }

    pub fn revoked_skill_names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.status == SkillRegistryStatus::Revoked)
            .map(|entry| entry.name.as_str())
            .collect()
    }
}

pub fn skill_content_sha256(skill: &Skill) -> String {
    crate::trust::sha256_hex(skill.raw.as_bytes())
}

pub fn default_skill_registry_paths() -> Result<SkillRegistryPaths, SkillRegistryError> {
    let dir = crate::config::config_dir()
        .map_err(|e| SkillRegistryError::Io(format!("config dir unavailable: {e}")))?
        .join("skills");
    let anchor_paths = policy_d::default_policy_d_paths()
        .map_err(|e| SkillRegistryError::Io(format!("organization anchor unavailable: {e}")))?;
    Ok(SkillRegistryPaths {
        registry_path: dir.join(DEFAULT_SKILL_REGISTRY_FILE),
        manifest_path: dir.join(DEFAULT_SKILL_REGISTRY_MANIFEST_FILE),
        anchor_path: anchor_paths.anchor_path,
        anchor_source: anchor_paths.anchor_source,
        expected_subject: DEFAULT_SKILL_REGISTRY_SUBJECT.to_string(),
    })
}

pub fn verify_skill_registry(
    registry_payload: &[u8],
    signed_manifest: &SignedTrustManifest,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedSkillRegistry, SkillRegistryError> {
    let verified_manifest =
        verify_signed_manifest(signed_manifest, anchor, now_unix, Some(registry_payload))?;
    if verified_manifest.manifest.subject != expected_subject {
        return Err(SkillRegistryError::SubjectMismatch {
            expected: expected_subject.to_string(),
            actual: verified_manifest.manifest.subject.clone(),
        });
    }

    let document = serde_json::from_slice::<SkillRegistryDocument>(registry_payload)
        .map_err(|_| SkillRegistryError::Parse)?;
    let mut entries = Vec::with_capacity(document.skills.len());
    for entry in document.skills {
        if !is_sha256_hex(&entry.skill_sha256) {
            return Err(SkillRegistryError::InvalidSkillHash { name: entry.name });
        }
        entries.push(VerifiedSkillRegistryEntry {
            name: entry.name,
            skill_sha256: entry.skill_sha256,
            status: SkillRegistryStatus::parse(&entry.status)?,
        });
    }

    Ok(VerifiedSkillRegistry {
        manifest: verified_manifest,
        entries,
    })
}

pub fn load_verified_skill_registry_from_files(
    registry_path: &Path,
    manifest_path: &Path,
    anchor: &TrustAnchor,
    now_unix: i64,
    expected_subject: &str,
) -> Result<VerifiedSkillRegistry, SkillRegistryError> {
    let registry_payload =
        std::fs::read(registry_path).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    let manifest_text = std::fs::read_to_string(manifest_path)
        .map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    let signed_manifest = serde_json::from_str::<SignedTrustManifest>(&manifest_text)
        .map_err(|e| SkillRegistryError::Io(format!("manifest parse failed: {e}")))?;
    verify_skill_registry(
        &registry_payload,
        &signed_manifest,
        anchor,
        now_unix,
        expected_subject,
    )
}

pub fn load_default_skill_registry_update_candidate(
    registry_path: &Path,
    manifest_path: &Path,
    now_unix: i64,
) -> Result<SkillRegistryUpdateCandidate, SkillRegistryError> {
    let target_paths = default_skill_registry_paths()?;
    let anchor = policy_d::load_trust_anchor_from(&target_paths.anchor_path)
        .map_err(|e| SkillRegistryError::Io(format!("anchor load failed: {e}")))?;
    let registry_payload =
        std::fs::read(registry_path).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    let manifest_payload =
        std::fs::read(manifest_path).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    let signed_manifest = serde_json::from_slice::<SignedTrustManifest>(&manifest_payload)
        .map_err(|e| SkillRegistryError::Io(format!("manifest parse failed: {e}")))?;
    let verified = verify_skill_registry(
        &registry_payload,
        &signed_manifest,
        &anchor,
        now_unix,
        &target_paths.expected_subject,
    )?;
    Ok(SkillRegistryUpdateCandidate {
        verified,
        registry_payload,
        manifest_payload,
        target_paths,
    })
}

pub fn install_default_skill_registry_update(
    candidate: SkillRegistryUpdateCandidate,
) -> Result<LoadedSkillRegistry, SkillRegistryError> {
    write_file_atomic(
        &candidate.target_paths.registry_path,
        &candidate.registry_payload,
    )?;
    write_file_atomic(
        &candidate.target_paths.manifest_path,
        &candidate.manifest_payload,
    )?;
    Ok(LoadedSkillRegistry {
        verified: candidate.verified,
        registry_path: candidate.target_paths.registry_path,
        manifest_path: candidate.target_paths.manifest_path,
        anchor_path: candidate.target_paths.anchor_path,
        anchor_source: candidate.target_paths.anchor_source,
    })
}

pub fn install_default_skill_registry_from_files(
    registry_path: &Path,
    manifest_path: &Path,
    now_unix: i64,
) -> Result<LoadedSkillRegistry, SkillRegistryError> {
    let candidate =
        load_default_skill_registry_update_candidate(registry_path, manifest_path, now_unix)?;
    install_default_skill_registry_update(candidate)
}

pub fn load_default_organization_skill_registry(
    now_unix: i64,
) -> Result<Option<LoadedSkillRegistry>, SkillRegistryError> {
    let paths = default_skill_registry_paths()?;
    let registry_exists = paths.registry_path.exists();
    let manifest_exists = paths.manifest_path.exists();
    let anchor_exists = paths.anchor_path.exists();

    if !registry_exists && !manifest_exists {
        return Ok(None);
    }
    if !registry_exists || !manifest_exists || !anchor_exists {
        return Err(SkillRegistryError::Io(format!(
            "incomplete skill registry set: registry={} manifest={} anchor={}",
            registry_exists, manifest_exists, anchor_exists
        )));
    }

    let anchor = policy_d::load_trust_anchor_from(&paths.anchor_path)
        .map_err(|e| SkillRegistryError::Io(format!("anchor load failed: {e}")))?;
    let verified = load_verified_skill_registry_from_files(
        &paths.registry_path,
        &paths.manifest_path,
        &anchor,
        now_unix,
        &paths.expected_subject,
    )?;
    Ok(Some(LoadedSkillRegistry {
        verified,
        registry_path: paths.registry_path,
        manifest_path: paths.manifest_path,
        anchor_path: paths.anchor_path,
        anchor_source: paths.anchor_source,
    }))
}

#[cfg(any(feature = "storage", test))]
pub fn skill_registry_audit_payload(
    loaded: &LoadedSkillRegistry,
    discovered_count: Option<usize>,
    allowed_count: Option<usize>,
) -> String {
    let mut payload = serde_json::json!({
        "subject": loaded.verified.manifest.manifest.subject,
        "version": loaded.verified.manifest.manifest.version,
        "manifest_id": loaded.verified.manifest.manifest.manifest_id,
        "key_id": loaded.verified.manifest.key_id,
        "entry_count": loaded.verified.entries.len(),
        "active_count": loaded.verified.active_count(),
        "revoked_count": loaded.verified.revoked_count(),
        "anchor_source": loaded.anchor_source.as_str(),
    });
    if let Some(discovered_count) = discovered_count {
        payload["discovered_count"] = serde_json::json!(discovered_count);
    }
    if let Some(allowed_count) = allowed_count {
        payload["allowed_count"] = serde_json::json!(allowed_count);
    }
    payload.to_string()
}

#[cfg(feature = "storage")]
pub fn record_skill_registry_audit(
    event_type: &str,
    loaded: &LoadedSkillRegistry,
    discovered_count: Option<usize>,
    allowed_count: Option<usize>,
) {
    let Ok(store) = crate::store::Store::open_default() else {
        return;
    };
    let payload = skill_registry_audit_payload(loaded, discovered_count, allowed_count);
    let _ = store.record_audit(event_type, None, None, &payload);
}

#[cfg(not(feature = "storage"))]
pub fn record_skill_registry_audit(
    _event_type: &str,
    _loaded: &LoadedSkillRegistry,
    _discovered_count: Option<usize>,
    _allowed_count: Option<usize>,
) {
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), SkillRegistryError> {
    let parent = path
        .parent()
        .ok_or_else(|| SkillRegistryError::Io("target path has no parent".to_string()))?;
    std::fs::create_dir_all(parent).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| SkillRegistryError::Io("target path has no file name".to_string()))?;
    let tmp_path = parent.join(format!(".{}.tmp", file_name.to_string_lossy()));
    std::fs::write(&tmp_path, bytes).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(first_err) if path.exists() => {
            std::fs::remove_file(path).map_err(|e| SkillRegistryError::Io(e.to_string()))?;
            std::fs::rename(&tmp_path, path)
                .map_err(|e| SkillRegistryError::Io(format!("{first_err}; retry failed: {e}")))
        }
        Err(e) => Err(SkillRegistryError::Io(e.to_string())),
    }
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

    fn signed_registry(payload: &[u8]) -> (SignedTrustManifest, TrustAnchor) {
        let signing_key = SigningKey::from_bytes(&[12u8; 32]);
        let manifest = TrustManifest {
            manifest_id: "skill-registry-001".to_string(),
            subject: DEFAULT_SKILL_REGISTRY_SUBJECT.to_string(),
            version: 3,
            issued_at_unix: 1_700_000_000,
            expires_at_unix: 1_800_000_000,
            payload_sha256: sha256_hex(payload),
        };
        let signature = signing_key.sign(&manifest_signing_bytes(&manifest).unwrap());
        let signed = SignedTrustManifest {
            key_id: "org-skill-root".to_string(),
            manifest,
            signature: hex_encode(&signature.to_bytes()),
        };
        let anchor = TrustAnchor {
            key_id: signed.key_id.clone(),
            public_key_hex: hex_encode(&signing_key.verifying_key().to_bytes()),
            min_version: 3,
        };
        (signed, anchor)
    }

    fn skill(name: &str, raw: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: String::new(),
            path: PathBuf::from("SKILL.md"),
            body: raw.to_string(),
            raw: raw.to_string(),
        }
    }

    #[test]
    fn verifies_signed_registry_and_allows_matching_skill() {
        let skill = skill("deploy", "---\nname: deploy\n---\nship it");
        let payload = format!(
            "{{\"skills\":[{{\"name\":\"deploy\",\"skill_sha256\":\"{}\",\"status\":\"active\"}}]}}",
            skill_content_sha256(&skill)
        );
        let (signed, anchor) = signed_registry(payload.as_bytes());

        let registry = verify_skill_registry(
            payload.as_bytes(),
            &signed,
            &anchor,
            1_750_000_000,
            DEFAULT_SKILL_REGISTRY_SUBJECT,
        )
        .unwrap();

        assert!(registry.allows_skill(&skill));
        assert!(!registry.allows_skill(&skill("deploy", "tampered")));
    }

    #[test]
    fn revoked_skill_is_not_allowed() {
        let skill = skill("deploy", "ship it");
        let payload = format!(
            "{{\"skills\":[{{\"name\":\"deploy\",\"skill_sha256\":\"{}\",\"status\":\"revoked\"}}]}}",
            skill_content_sha256(&skill)
        );
        let (signed, anchor) = signed_registry(payload.as_bytes());
        let registry = verify_skill_registry(
            payload.as_bytes(),
            &signed,
            &anchor,
            1_750_000_000,
            DEFAULT_SKILL_REGISTRY_SUBJECT,
        )
        .unwrap();

        assert!(!registry.allows_skill(&skill));
        assert_eq!(registry.revoked_skill_names(), vec!["deploy"]);
        assert_eq!(registry.active_count(), 0);
        assert_eq!(registry.revoked_count(), 1);
    }

    #[test]
    fn audit_payload_uses_metadata_counts_without_paths() {
        let skill = skill("deploy", "ship it");
        let payload = format!(
            "{{\"skills\":[\
             {{\"name\":\"deploy\",\"skill_sha256\":\"{}\",\"status\":\"active\"}},\
             {{\"name\":\"old-deploy\",\"skill_sha256\":\"{}\",\"status\":\"revoked\"}}\
             ]}}",
            skill_content_sha256(&skill),
            "a".repeat(64)
        );
        let (signed, anchor) = signed_registry(payload.as_bytes());
        let verified = verify_skill_registry(
            payload.as_bytes(),
            &signed,
            &anchor,
            1_750_000_000,
            DEFAULT_SKILL_REGISTRY_SUBJECT,
        )
        .unwrap();
        let loaded = LoadedSkillRegistry {
            verified,
            registry_path: PathBuf::from("/secret/org-registry.json"),
            manifest_path: PathBuf::from("/secret/org-registry.manifest.json"),
            anchor_path: PathBuf::from("/secret/org-root.json"),
            anchor_source: TrustAnchorSource::ManagedPath,
        };

        let audit = skill_registry_audit_payload(&loaded, Some(9), Some(1));
        assert!(audit.contains("\"entry_count\":2"), "{audit}");
        assert!(audit.contains("\"active_count\":1"), "{audit}");
        assert!(audit.contains("\"revoked_count\":1"), "{audit}");
        assert!(audit.contains("\"discovered_count\":9"), "{audit}");
        assert!(audit.contains("\"allowed_count\":1"), "{audit}");
        assert!(!audit.contains("/secret"), "{audit}");
    }

    #[test]
    fn rejects_modified_registry_payload() {
        let payload = b"{\"skills\":[]}";
        let (signed, anchor) = signed_registry(payload);

        assert!(matches!(
            verify_skill_registry(
                b"{\"skills\":[{\"name\":\"x\",\"skill_sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"status\":\"active\"}]}",
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_SKILL_REGISTRY_SUBJECT
            ),
            Err(SkillRegistryError::Trust(TrustError::PayloadDigestMismatch))
        ));
    }

    #[test]
    fn rejects_unknown_status_and_invalid_hash() {
        let bad_status =
            b"{\"skills\":[{\"name\":\"x\",\"skill_sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"status\":\"paused\"}]}";
        let (signed, anchor) = signed_registry(bad_status);
        assert!(matches!(
            verify_skill_registry(
                bad_status,
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_SKILL_REGISTRY_SUBJECT
            ),
            Err(SkillRegistryError::UnknownStatus(_))
        ));

        let bad_hash =
            b"{\"skills\":[{\"name\":\"x\",\"skill_sha256\":\"nope\",\"status\":\"active\"}]}";
        let (signed, anchor) = signed_registry(bad_hash);
        assert!(matches!(
            verify_skill_registry(
                bad_hash,
                &signed,
                &anchor,
                1_750_000_000,
                DEFAULT_SKILL_REGISTRY_SUBJECT
            ),
            Err(SkillRegistryError::InvalidSkillHash { .. })
        ));
    }
}
