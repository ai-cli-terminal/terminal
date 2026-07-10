//! `ai` CLI의 P3 trust channel 핸들러 — skill enable/registry·policy org·release manifest.
//! `main.rs` monolith에서 `cli/`로 재배치(Wave2↔trust 재조정). 핸들러는 `dispatch.rs`가 호출한다.
//! trust 관련 심볼은 `feature = "trust"` 게이트를 원본(main.rs) 그대로 유지한다.

use std::path::PathBuf;

use ai_terminal::{config, skill};

#[cfg(feature = "trust")]
fn path_state(path: &std::path::Path, check_readonly: bool) -> String {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if check_readonly {
                format!("exists readonly={}", metadata.permissions().readonly())
            } else {
                "exists".to_string()
            }
        }
        Err(_) => "missing".to_string(),
    }
}

#[cfg(feature = "trust")]
pub(crate) fn run_policy_org_status() -> anyhow::Result<()> {
    let paths = ai_terminal::policy_d::default_policy_d_paths().map_err(anyhow::Error::from)?;
    println!("organization_policy :");
    println!(
        "  policy   : {} ({})",
        paths.policy_path.display(),
        path_state(&paths.policy_path, true)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::policy_d::load_default_organization_policy(now) {
        Ok(Some(loaded)) => {
            println!("status    : active");
            println!("runtime   : organization policy overrides user active profile");
            println!("profile   : {}", loaded.verified.profile.name);
            println!(
                "skill_external_sources: {}",
                loaded.verified.external_skill_sources.as_str()
            );
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!("runtime   : user active profile");
            println!("profile   : {}", config::get_active_profile());
            println!("skill_external_sources: user-enabled");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : fail-closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_policy_org_status() -> anyhow::Result<()> {
    println!("organization_policy :");
    println!("status    : unavailable");
    println!("runtime   : user active profile");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

pub(crate) fn run_skill_list(query: Option<String>) -> anyhow::Result<()> {
    let paths = default_skill_discovery_paths();
    let enabled = skill::get_enabled_skills();
    let discovered =
        skill::filter_explicitly_enabled_external(skill::discover_with_source(&paths), &enabled);
    let skills: Vec<skill::Skill> = discovered.into_iter().map(|entry| entry.skill).collect();
    #[cfg(feature = "trust")]
    let skills = {
        let mut skills = skills;
        let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
        if let Some(registry) =
            ai_terminal::skill_registry::load_default_organization_skill_registry(now)
                .map_err(anyhow::Error::from)?
        {
            let discovered_count = skills.len();
            skills.retain(|skill| registry.verified.allows_skill(skill));
            ai_terminal::skill_registry::record_skill_registry_audit(
                "skill_registry_enforced",
                &registry,
                Some(discovered_count),
                Some(skills.len()),
            );
        }
        skills
    };
    let shown: Vec<&skill::Skill> = match &query {
        Some(q) => skill::match_skills(&skills, q, 5),
        None => skills.iter().collect(),
    };
    if shown.is_empty() {
        let path_list: Vec<PathBuf> = paths.iter().map(|(path, _)| path.clone()).collect();
        println!("(스킬 없음 — {:?})", path_list);
    }
    for s in shown {
        println!("- {} — {}", s.name, s.description);
    }
    Ok(())
}

fn default_skill_discovery_paths() -> Vec<(PathBuf, skill::SkillSource)> {
    let mut paths = vec![(
        PathBuf::from("./.ai-terminal/skills"),
        skill::SkillSource::Workspace,
    )];
    if let Ok(cd) = config::config_dir() {
        paths.push((cd.join("skills"), skill::SkillSource::External));
    }
    paths
}

pub(crate) fn run_skill_enabled() {
    let enabled = skill::get_enabled_skills();
    println!("external_skill_enabled :");
    if enabled.is_empty() {
        println!("(none)");
    } else {
        for name in enabled {
            println!("- {name}");
        }
    }
}

#[cfg(feature = "trust")]
fn validate_external_skill_enable_policy(
    entry: &skill::DiscoveredSkill,
) -> anyhow::Result<Option<&'static str>> {
    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let external_source_policy = ai_terminal::policy_d::load_default_organization_policy(now)
        .map_err(anyhow::Error::from)?
        .map(|policy| policy.verified.external_skill_sources)
        .unwrap_or_default();
    let source_policy_label = external_source_policy.as_str();
    let registry = ai_terminal::skill_registry::load_default_organization_skill_registry(now)
        .map_err(anyhow::Error::from)?;
    match external_source_policy {
        ai_terminal::policy_d::ExternalSkillSourcePolicy::Disabled => {
            anyhow::bail!(
                "organization policy disables external skill sources: {}",
                entry.skill.name
            );
        }
        ai_terminal::policy_d::ExternalSkillSourcePolicy::RegistryOnly => {
            let Some(registry) = registry else {
                anyhow::bail!(
                    "organization policy requires a signed skill registry before enabling \
                     external skills: {}",
                    entry.skill.name
                );
            };
            if !registry.verified.allows_skill(&entry.skill) {
                anyhow::bail!(
                    "external skill is not active in the signed organization registry: {}",
                    entry.skill.name
                );
            }
        }
        ai_terminal::policy_d::ExternalSkillSourcePolicy::UserEnabled => {
            if let Some(registry) = registry {
                if !registry.verified.allows_skill(&entry.skill) {
                    anyhow::bail!(
                        "external skill is not active in the signed organization registry: {}",
                        entry.skill.name
                    );
                }
            }
        }
    }
    Ok(Some(source_policy_label))
}

#[cfg(not(feature = "trust"))]
fn validate_external_skill_enable_policy(
    _entry: &skill::DiscoveredSkill,
) -> anyhow::Result<Option<&'static str>> {
    Ok(None)
}

fn skill_enable_confirmation_matches(input: &str, name: &str) -> bool {
    input.trim() == name
}

fn confirm_external_skill_enable(
    entry: &skill::DiscoveredSkill,
    source_policy_label: Option<&str>,
    assume_yes: bool,
) -> anyhow::Result<bool> {
    if assume_yes {
        return Ok(true);
    }

    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "external skill enable confirmation requires a TTY; rerun with --yes for automation"
        );
    }

    println!("external_skill : {}", entry.skill.name);
    println!("description    : {}", entry.skill.description);
    println!("source         : {}", entry.source.as_str());
    if let Some(source_policy_label) = source_policy_label {
        println!("source_policy  : {source_policy_label}");
    }
    print!(
        "Type `{}` to enable this external skill: ",
        entry.skill.name
    );
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(skill_enable_confirmation_matches(&line, &entry.skill.name))
}

pub(crate) fn run_skill_enable(name: String, yes: bool) -> anyhow::Result<()> {
    let paths = default_skill_discovery_paths();
    let discovered = skill::discover_with_source(&paths);
    let matches = skill::external_skills_named(&discovered, &name);
    if matches.is_empty() {
        anyhow::bail!("external skill not found: {name}");
    }
    if matches.len() > 1 {
        anyhow::bail!("multiple external skills named {name}; remove duplicate SKILL.md names");
    }

    let entry = matches[0];
    let source_policy_label = validate_external_skill_enable_policy(entry)?;
    if !confirm_external_skill_enable(entry, source_policy_label, yes)? {
        println!("external_skill : {name}");
        if let Some(source_policy_label) = source_policy_label {
            println!("source_policy  : {source_policy_label}");
        }
        println!("status         : declined");
        println!("reason         : confirmation_required");
        return Ok(());
    }

    let inserted = skill::enable_skill_name(&name)?;
    skill::record_skill_enable_audit("skill_enabled", &name, skill::SkillSource::External);
    println!("external_skill : {name}");
    if let Some(source_policy_label) = source_policy_label {
        println!("source_policy  : {source_policy_label}");
    }
    println!(
        "status         : {}",
        if inserted {
            "enabled"
        } else {
            "already_enabled"
        }
    );
    Ok(())
}

pub(crate) fn run_skill_disable(name: String) -> anyhow::Result<()> {
    let removed = skill::disable_skill_name(&name)?;
    skill::record_skill_enable_audit("skill_disabled", &name, skill::SkillSource::External);
    println!("external_skill : {name}");
    println!(
        "status         : {}",
        if removed {
            "disabled"
        } else {
            "already_disabled"
        }
    );
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) fn run_skill_registry_status() -> anyhow::Result<()> {
    let paths =
        ai_terminal::skill_registry::default_skill_registry_paths().map_err(anyhow::Error::from)?;
    println!("organization_skill_registry :");
    println!(
        "  registry : {} ({})",
        paths.registry_path.display(),
        path_state(&paths.registry_path, false)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::skill_registry::load_default_organization_skill_registry(now) {
        Ok(Some(loaded)) => {
            let revoked_names = loaded.verified.revoked_skill_names();
            println!("status    : active");
            println!("runtime   : ai skill filters to active name+hash matches");
            println!("entries   : {}", loaded.verified.entries.len());
            println!("active    : {}", loaded.verified.active_count());
            println!("revoked   : {}", loaded.verified.revoked_count());
            if !revoked_names.is_empty() {
                println!("revoked_names: {}", revoked_names.join(", "));
            }
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!("runtime   : local skill discovery");
            println!("entries   : 0");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : fail-closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) fn run_skill_registry_update(
    registry: PathBuf,
    manifest: PathBuf,
    require_revoked: bool,
) -> anyhow::Result<()> {
    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let candidate = ai_terminal::skill_registry::load_default_skill_registry_update_candidate(
        &registry, &manifest, now,
    )
    .map_err(anyhow::Error::from)?;
    if require_revoked && candidate.verified.revoked_count() == 0 {
        anyhow::bail!(
            "signed registry contains no revoked entries; use `ai skill registry update` \
             for non-revocation updates"
        );
    }
    let loaded = ai_terminal::skill_registry::install_default_skill_registry_update(candidate)
        .map_err(anyhow::Error::from)?;
    let event_type = if require_revoked {
        "skill_registry_revoked"
    } else {
        "skill_registry_updated"
    };
    ai_terminal::skill_registry::record_skill_registry_audit(event_type, &loaded, None, None);
    println!("organization_skill_registry :");
    println!("status    : installed");
    println!("event     : {event_type}");
    println!("entries   : {}", loaded.verified.entries.len());
    println!("active    : {}", loaded.verified.active_count());
    println!("revoked   : {}", loaded.verified.revoked_count());
    println!("key_id    : {}", loaded.verified.manifest.key_id);
    println!(
        "manifest  : {}",
        loaded.verified.manifest.manifest.manifest_id
    );
    println!("version   : {}", loaded.verified.manifest.manifest.version);
    println!("registry  : {}", loaded.registry_path.display());
    println!("manifest_path: {}", loaded.manifest_path.display());
    Ok(())
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_skill_registry_update(
    _registry: PathBuf,
    _manifest: PathBuf,
    _require_revoked: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("skill registry update requires the `trust` feature")
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_skill_registry_status() -> anyhow::Result<()> {
    println!("organization_skill_registry :");
    println!("status    : unavailable");
    println!("runtime   : local skill discovery");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) fn run_release_manifest_create(
    output: PathBuf,
    artifact: Vec<PathBuf>,
    release_version: Option<String>,
) -> anyhow::Result<()> {
    let document = ai_terminal::binary_manifest::build_binary_manifest_document(
        &artifact,
        release_version.as_deref(),
    )
    .map_err(anyhow::Error::from)?;
    let payload = ai_terminal::binary_manifest::binary_manifest_payload(&document)
        .map_err(anyhow::Error::from)?;
    std::fs::write(&output, payload)?;
    println!("organization_binary_manifest :");
    println!("status    : created");
    println!("payload   : {}", output.display());
    println!("artifacts : {}", document.artifacts.len());
    if let Some(release_version) = release_version {
        println!("release_version: {release_version}");
    }
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) struct ReleaseManifestSignInput {
    pub(crate) payload: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) key_id: String,
    pub(crate) private_key_env: String,
    pub(crate) manifest_version: u64,
    pub(crate) manifest_id: Option<String>,
    pub(crate) subject: String,
    pub(crate) issued_at_unix: Option<i64>,
    pub(crate) valid_days: i64,
}

#[cfg(feature = "trust")]
pub(crate) fn run_release_manifest_sign(input: ReleaseManifestSignInput) -> anyhow::Result<()> {
    let ReleaseManifestSignInput {
        payload,
        output,
        key_id,
        private_key_env,
        manifest_version,
        manifest_id,
        subject,
        issued_at_unix,
        valid_days,
    } = input;
    if valid_days <= 0 {
        anyhow::bail!("--valid-days must be positive");
    }
    let private_key_hex = std::env::var(&private_key_env).map_err(|_| {
        anyhow::anyhow!("release signing key env var is not set: {private_key_env}")
    })?;
    let payload_bytes = std::fs::read(&payload)?;
    let issued_at_unix = match issued_at_unix {
        Some(value) => value,
        None => ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?,
    };
    let valid_seconds = valid_days
        .checked_mul(24 * 60 * 60)
        .ok_or_else(|| anyhow::anyhow!("--valid-days is too large"))?;
    let expires_at_unix = issued_at_unix
        .checked_add(valid_seconds)
        .ok_or_else(|| anyhow::anyhow!("release manifest expiration overflow"))?;
    let manifest = ai_terminal::trust::TrustManifest {
        manifest_id: manifest_id
            .unwrap_or_else(|| format!("release-binary-manifest-{manifest_version}")),
        subject,
        version: manifest_version,
        issued_at_unix,
        expires_at_unix,
        payload_sha256: ai_terminal::trust::sha256_hex(&payload_bytes),
    };
    let signed = ai_terminal::trust::sign_manifest(manifest, key_id, &private_key_hex)
        .map_err(anyhow::Error::from)?;
    let signed_json = serde_json::to_vec_pretty(&signed)?;
    std::fs::write(&output, signed_json)?;
    println!("organization_binary_manifest :");
    println!("status    : signed");
    println!("payload   : {}", payload.display());
    println!("manifest_path: {}", output.display());
    println!("key_id    : {}", signed.key_id);
    println!("manifest  : {}", signed.manifest.manifest_id);
    println!("version   : {}", signed.manifest.version);
    println!("issued_at : {}", signed.manifest.issued_at_unix);
    println!("expires_at: {}", signed.manifest.expires_at_unix);
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) fn run_release_manifest_status() -> anyhow::Result<()> {
    let paths = ai_terminal::binary_manifest::default_binary_manifest_paths()
        .map_err(anyhow::Error::from)?;
    println!("organization_binary_manifest :");
    println!(
        "  payload  : {} ({})",
        paths.payload_path.display(),
        path_state(&paths.payload_path, false)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::binary_manifest::load_default_organization_binary_manifest(now) {
        Ok(Some(loaded)) => {
            println!("status    : active");
            println!("runtime   : install/update callers can require signed artifact matches");
            println!("artifacts : {}", loaded.verified.artifact_count());
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!(
                "runtime   : release downloads rely on checksums unless caller supplies a manifest"
            );
            println!("artifacts : 0");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : install/update enforcement must fail closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(feature = "trust")]
pub(crate) fn run_release_manifest_verify(
    payload: PathBuf,
    manifest: PathBuf,
    name: Option<String>,
    artifact: Option<PathBuf>,
) -> anyhow::Result<()> {
    if name.is_some() != artifact.is_some() {
        anyhow::bail!("--name and --artifact must be provided together");
    }

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let verified = ai_terminal::binary_manifest::load_default_binary_manifest_from_files(
        &payload, &manifest, now,
    )
    .map_err(anyhow::Error::from)?;

    println!("organization_binary_manifest :");
    println!("status    : verified");
    println!("artifacts : {}", verified.artifact_count());
    println!("key_id    : {}", verified.manifest.key_id);
    println!("manifest  : {}", verified.manifest.manifest.manifest_id);
    println!("version   : {}", verified.manifest.manifest.version);
    println!("payload   : {}", payload.display());
    println!("manifest_path: {}", manifest.display());

    if let (Some(name), Some(artifact)) = (name, artifact) {
        let sha256 = ai_terminal::binary_manifest::verify_artifact_against_manifest(
            &verified, &name, &artifact,
        )
        .map_err(anyhow::Error::from)?;
        println!("artifact  : {name}");
        println!("artifact_path: {}", artifact.display());
        println!("sha256    : {sha256}");
        println!("artifact_status: matched");
    }
    Ok(())
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_release_manifest_create(
    _output: PathBuf,
    _artifact: Vec<PathBuf>,
    _release_version: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("binary release manifest creation requires the `trust` feature")
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_release_manifest_sign() -> anyhow::Result<()> {
    anyhow::bail!("binary release manifest signing requires the `trust` feature")
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_release_manifest_status() -> anyhow::Result<()> {
    println!("organization_binary_manifest :");
    println!("status    : unavailable");
    println!("runtime   : release downloads rely on checksums");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

#[cfg(not(feature = "trust"))]
pub(crate) fn run_release_manifest_verify(
    _payload: PathBuf,
    _manifest: PathBuf,
    _name: Option<String>,
    _artifact: Option<PathBuf>,
) -> anyhow::Result<()> {
    anyhow::bail!("binary release manifest verification requires the `trust` feature")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::command::{
        Cli, Command, PolicyAction, PolicyOrgAction, ReleaseAction, ReleaseManifestAction,
        SkillAction, SkillRegistryAction,
    };
    use clap::Parser;

    #[test]
    fn cli_parses_policy_org_status() {
        let cli = Cli::try_parse_from(["ai", "policy", "org", "status"]).unwrap();
        match cli.command {
            Some(Command::Policy {
                action:
                    PolicyAction::Org {
                        action: PolicyOrgAction::Status,
                    },
            }) => {}
            _ => panic!("expected policy org status"),
        }
    }

    #[test]
    fn cli_parses_skill_command() {
        let cli = Cli::try_parse_from(["ai", "skill", "--query", "deploy"]).unwrap();
        match cli.command {
            Some(Command::Skill { query, action }) => {
                assert_eq!(query.as_deref(), Some("deploy"));
                assert!(action.is_none());
            }
            _ => panic!("expected skill subcommand"),
        }
    }

    #[test]
    fn cli_parses_skill_registry_status() {
        let cli = Cli::try_parse_from(["ai", "skill", "registry", "status"]).unwrap();
        match cli.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Status,
                    }),
            }) => {}
            _ => panic!("expected skill registry status"),
        }
    }

    #[test]
    fn cli_parses_skill_enable_disable_and_enabled() {
        let guarded_enable = Cli::try_parse_from(["ai", "skill", "enable", "deploy"]).unwrap();
        match guarded_enable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enable { name, yes }),
            }) => {
                assert_eq!(name, "deploy");
                assert!(!yes);
            }
            _ => panic!("expected guarded skill enable"),
        }

        let enable = Cli::try_parse_from(["ai", "skill", "enable", "deploy", "--yes"]).unwrap();
        match enable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enable { name, yes }),
            }) => {
                assert_eq!(name, "deploy");
                assert!(yes);
            }
            _ => panic!("expected skill enable"),
        }

        let disable = Cli::try_parse_from(["ai", "skill", "disable", "deploy"]).unwrap();
        match disable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Disable { name }),
            }) => assert_eq!(name, "deploy"),
            _ => panic!("expected skill disable"),
        }

        let enabled = Cli::try_parse_from(["ai", "skill", "enabled"]).unwrap();
        match enabled.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enabled),
            }) => {}
            _ => panic!("expected skill enabled"),
        }
    }

    #[test]
    fn skill_enable_confirmation_requires_exact_skill_name() {
        assert!(skill_enable_confirmation_matches("deploy\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("yes\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("Deploy\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("deploy now\n", "deploy"));
    }

    #[test]
    fn cli_parses_skill_registry_update_and_revoke() {
        let update = Cli::try_parse_from([
            "ai",
            "skill",
            "registry",
            "update",
            "--registry",
            "org-registry.json",
            "--manifest",
            "org-registry.manifest.json",
        ])
        .unwrap();
        match update.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Update { registry, manifest },
                    }),
            }) => {
                assert_eq!(registry, PathBuf::from("org-registry.json"));
                assert_eq!(manifest, PathBuf::from("org-registry.manifest.json"));
            }
            _ => panic!("expected skill registry update"),
        }

        let revoke = Cli::try_parse_from([
            "ai",
            "skill",
            "registry",
            "revoke",
            "--registry",
            "revoked-registry.json",
            "--manifest",
            "revoked-registry.manifest.json",
        ])
        .unwrap();
        match revoke.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Revoke { registry, manifest },
                    }),
            }) => {
                assert_eq!(registry, PathBuf::from("revoked-registry.json"));
                assert_eq!(manifest, PathBuf::from("revoked-registry.manifest.json"));
            }
            _ => panic!("expected skill registry revoke"),
        }
    }

    #[test]
    fn cli_parses_release_manifest_status_and_verify() {
        let status = Cli::try_parse_from(["ai", "release", "manifest", "status"]).unwrap();
        match status.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action: ReleaseManifestAction::Status,
                    },
            }) => {}
            _ => panic!("expected release manifest status"),
        }

        let create = Cli::try_parse_from([
            "ai",
            "release",
            "manifest",
            "create",
            "--output",
            "binary-manifest.json",
            "--artifact",
            "ai-linux-x86_64",
            "--artifact",
            "ash-linux-x86_64",
            "--release-version",
            "0.3.4",
        ])
        .unwrap();
        match create.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action:
                            ReleaseManifestAction::Create {
                                output,
                                artifact,
                                release_version,
                            },
                    },
            }) => {
                assert_eq!(output, PathBuf::from("binary-manifest.json"));
                assert_eq!(
                    artifact,
                    vec![
                        PathBuf::from("ai-linux-x86_64"),
                        PathBuf::from("ash-linux-x86_64")
                    ]
                );
                assert_eq!(release_version.as_deref(), Some("0.3.4"));
            }
            _ => panic!("expected release manifest create"),
        }

        let sign = Cli::try_parse_from([
            "ai",
            "release",
            "manifest",
            "sign",
            "--payload",
            "binary-manifest.json",
            "--output",
            "binary-manifest.manifest.json",
            "--key-id",
            "release-root",
            "--manifest-version",
            "42",
            "--manifest-id",
            "release-v0.3.4",
            "--issued-at-unix",
            "1700000000",
            "--valid-days",
            "30",
        ])
        .unwrap();
        match sign.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action:
                            ReleaseManifestAction::Sign {
                                payload,
                                output,
                                key_id,
                                private_key_env,
                                manifest_version,
                                manifest_id,
                                subject,
                                issued_at_unix,
                                valid_days,
                            },
                    },
            }) => {
                assert_eq!(payload, PathBuf::from("binary-manifest.json"));
                assert_eq!(output, PathBuf::from("binary-manifest.manifest.json"));
                assert_eq!(key_id, "release-root");
                assert_eq!(private_key_env, "AI_TERMINAL_RELEASE_SIGNING_KEY_HEX");
                assert_eq!(manifest_version, 42);
                assert_eq!(manifest_id.as_deref(), Some("release-v0.3.4"));
                assert_eq!(subject, "release/binary-manifest.json");
                assert_eq!(issued_at_unix, Some(1_700_000_000));
                assert_eq!(valid_days, 30);
            }
            _ => panic!("expected release manifest sign"),
        }

        let verify = Cli::try_parse_from([
            "ai",
            "release",
            "manifest",
            "verify",
            "--payload",
            "binary-manifest.json",
            "--manifest",
            "binary-manifest.manifest.json",
            "--name",
            "ai-linux-x86_64",
            "--artifact",
            "ai-linux-x86_64",
        ])
        .unwrap();
        match verify.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action:
                            ReleaseManifestAction::Verify {
                                payload,
                                manifest,
                                name,
                                artifact,
                            },
                    },
            }) => {
                assert_eq!(payload, PathBuf::from("binary-manifest.json"));
                assert_eq!(manifest, PathBuf::from("binary-manifest.manifest.json"));
                assert_eq!(name.as_deref(), Some("ai-linux-x86_64"));
                assert_eq!(artifact, Some(PathBuf::from("ai-linux-x86_64")));
            }
            _ => panic!("expected release manifest verify"),
        }
    }
}
