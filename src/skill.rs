//! 통합 스킬 관리 (설계 §26, Phase 2).
//!
//! SKILL.md(프론트매터: name/description) 기반으로 스킬을 발견·매칭·로딩한다.
//! 스킬 콘텐츠는 **Zero-Trust 데이터**로 취급한다 — 여기서는 발견/매칭/로딩만 하고
//! 스크립트 실행은 일반 명령과 동일한 정책·샌드박스 경계를 거쳐야 한다(§26, RULES §2).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 발견된 스킬.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub body: String,
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSource {
    Workspace,
    External,
}

impl SkillSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillSource::Workspace => "workspace",
            SkillSource::External => "external",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSkill {
    pub skill: Skill,
    pub source: SkillSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct EnabledSkills {
    pub enabled: Vec<String>,
}

/// SKILL.md 내용을 파싱한다(프론트매터 없으면 파일명/첫 줄에서 추론).
pub fn parse_skill(content: &str, path: &Path) -> Skill {
    let (name, description, body) = parse_frontmatter(content);
    let name = name.unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string())
    });
    let description = description.unwrap_or_else(|| {
        body.lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim_start_matches('#')
            .trim()
            .to_string()
    });
    Skill {
        name,
        description,
        path: path.to_path_buf(),
        body,
        raw: content.to_string(),
    }
}

fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, String) {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let fm = &rest[..end];
            let body = rest[end + 4..].trim_start_matches(['\n', '\r']).to_string();
            let mut name = None;
            let mut description = None;
            for line in fm.lines() {
                if let Some(v) = line.strip_prefix("name:") {
                    name = Some(v.trim().to_string());
                } else if let Some(v) = line.strip_prefix("description:") {
                    description = Some(v.trim().to_string());
                }
            }
            return (name, description, body);
        }
    }
    (None, None, content.to_string())
}

/// 주어진 경로들에서 스킬을 발견한다(`<dir>/SKILL.md`, `<dir>/<skill>/SKILL.md`).
pub fn discover(paths: &[PathBuf]) -> Vec<Skill> {
    let source_paths = paths
        .iter()
        .cloned()
        .map(|path| (path, SkillSource::Workspace))
        .collect::<Vec<_>>();
    discover_with_source(&source_paths)
        .into_iter()
        .map(|entry| entry.skill)
        .collect()
}

pub fn discover_with_source(paths: &[(PathBuf, SkillSource)]) -> Vec<DiscoveredSkill> {
    let mut out = Vec::new();
    for (base, source) in paths {
        let Ok(entries) = std::fs::read_dir(base) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let skill_md = if p.is_dir() {
                p.join("SKILL.md")
            } else if p.file_name() == Some("SKILL.md".as_ref()) {
                p.clone()
            } else {
                continue;
            };
            if skill_md.is_file() {
                if let Ok(content) = std::fs::read_to_string(&skill_md) {
                    out.push(DiscoveredSkill {
                        skill: parse_skill(&content, &skill_md),
                        source: *source,
                    });
                }
            }
        }
    }
    out
}

pub fn enabled_skills_path() -> anyhow::Result<PathBuf> {
    Ok(crate::config::config_dir()?
        .join("skills")
        .join("enabled.json"))
}

pub fn read_enabled_skills_from(path: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<EnabledSkills>(&text).ok())
        .map(|state| {
            state
                .enabled
                .into_iter()
                .filter(|name| !name.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn write_enabled_skills_to(path: &Path, enabled: &BTreeSet<String>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let state = EnabledSkills {
        enabled: enabled.iter().cloned().collect(),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&state)?)?;
    Ok(())
}

pub fn get_enabled_skills() -> BTreeSet<String> {
    enabled_skills_path()
        .map(|path| read_enabled_skills_from(&path))
        .unwrap_or_default()
}

pub fn enable_skill_name(name: &str) -> anyhow::Result<bool> {
    let path = enabled_skills_path()?;
    let mut enabled = read_enabled_skills_from(&path);
    let inserted = enabled.insert(name.to_string());
    write_enabled_skills_to(&path, &enabled)?;
    Ok(inserted)
}

pub fn disable_skill_name(name: &str) -> anyhow::Result<bool> {
    let path = enabled_skills_path()?;
    let mut enabled = read_enabled_skills_from(&path);
    let removed = enabled.remove(name);
    write_enabled_skills_to(&path, &enabled)?;
    Ok(removed)
}

pub fn filter_explicitly_enabled_external(
    discovered: Vec<DiscoveredSkill>,
    enabled: &BTreeSet<String>,
) -> Vec<DiscoveredSkill> {
    discovered
        .into_iter()
        .filter(|entry| {
            entry.source == SkillSource::Workspace || enabled.contains(entry.skill.name.as_str())
        })
        .collect()
}

pub fn external_skills_named<'a>(
    discovered: &'a [DiscoveredSkill],
    name: &str,
) -> Vec<&'a DiscoveredSkill> {
    discovered
        .iter()
        .filter(|entry| entry.source == SkillSource::External && entry.skill.name == name)
        .collect()
}

#[cfg(feature = "storage")]
pub fn record_skill_enable_audit(event_type: &str, name: &str, source: SkillSource) {
    let Ok(store) = crate::store::Store::open_default() else {
        return;
    };
    let payload = serde_json::json!({
        "name": name,
        "source": source.as_str(),
    })
    .to_string();
    let _ = store.record_audit(event_type, None, None, &payload);
}

#[cfg(not(feature = "storage"))]
pub fn record_skill_enable_audit(_event_type: &str, _name: &str, _source: SkillSource) {}

/// 쿼리 키워드로 스킬을 매칭해 점수 높은 순 최대 `max`개 반환한다.
pub fn match_skills<'a>(skills: &'a [Skill], query: &str, max: usize) -> Vec<&'a Skill> {
    let words: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let mut scored: Vec<(usize, &Skill)> = skills
        .iter()
        .filter_map(|s| {
            let hay = format!("{} {}", s.name, s.description).to_lowercase();
            let score = words.iter().filter(|w| hay.contains(w.as_str())).count();
            (score > 0).then_some((score, s))
        })
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().take(max).map(|(_, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn uniq(tag: &str) -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_skill_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn parses_frontmatter() {
        let content =
            "---\nname: git-helper\ndescription: helps with git tasks\n---\n# Body\nuse git";
        let s = parse_skill(content, Path::new("/x/SKILL.md"));
        assert_eq!(s.name, "git-helper");
        assert_eq!(s.description, "helps with git tasks");
        assert!(s.body.contains("use git"));
    }

    #[test]
    fn discovers_skills_in_subdirs() {
        let base = uniq("base");
        let skdir = base.join("docker-skill");
        std::fs::create_dir_all(&skdir).unwrap();
        std::fs::write(
            skdir.join("SKILL.md"),
            "---\nname: docker\ndescription: docker helpers\n---\nbody",
        )
        .unwrap();
        let found = discover(std::slice::from_ref(&base));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "docker");
    }

    #[test]
    fn discovers_with_source_and_filters_external_by_enabled_name() {
        let workspace = uniq("workspace");
        let external = uniq("external");
        let local_dir = workspace.join("local");
        let remote_dir = external.join("remote");
        std::fs::create_dir_all(&local_dir).unwrap();
        std::fs::create_dir_all(&remote_dir).unwrap();
        std::fs::write(
            local_dir.join("SKILL.md"),
            "---\nname: local\ndescription: workspace skill\n---\nbody",
        )
        .unwrap();
        std::fs::write(
            remote_dir.join("SKILL.md"),
            "---\nname: remote\ndescription: external skill\n---\nbody",
        )
        .unwrap();

        let discovered = discover_with_source(&[
            (workspace.clone(), SkillSource::Workspace),
            (external.clone(), SkillSource::External),
        ]);
        assert_eq!(discovered.len(), 2);
        assert_eq!(external_skills_named(&discovered, "remote").len(), 1);

        let filtered = filter_explicitly_enabled_external(discovered.clone(), &BTreeSet::new());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].skill.name, "local");

        let mut enabled = BTreeSet::new();
        enabled.insert("remote".to_string());
        let filtered = filter_explicitly_enabled_external(discovered, &enabled);
        assert_eq!(filtered.len(), 2);

        let _ = std::fs::remove_dir_all(workspace);
        let _ = std::fs::remove_dir_all(external);
    }

    #[test]
    fn enabled_skills_state_roundtrips_sorted_unique_names() {
        let p = uniq("enabled").join("enabled.json");
        let mut enabled = BTreeSet::new();
        enabled.insert("beta".to_string());
        enabled.insert("alpha".to_string());
        write_enabled_skills_to(&p, &enabled).unwrap();

        let loaded = read_enabled_skills_from(&p);
        assert_eq!(
            loaded.into_iter().collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );
        let raw = std::fs::read_to_string(&p).unwrap();
        assert!(raw.contains("\"alpha\""), "{raw}");
        assert!(raw.contains("\"beta\""), "{raw}");

        let root = p.parent().unwrap().to_path_buf();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn matches_by_keyword_ranked() {
        let skills = vec![
            Skill {
                name: "git-helper".into(),
                description: "helps with git commit and branch".into(),
                path: PathBuf::new(),
                body: String::new(),
                raw: String::new(),
            },
            Skill {
                name: "docker".into(),
                description: "container stuff".into(),
                path: PathBuf::new(),
                body: String::new(),
                raw: String::new(),
            },
        ];
        let m = match_skills(&skills, "git commit", 5);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].name, "git-helper");
        // 무관한 쿼리는 매칭 없음
        assert!(match_skills(&skills, "kubernetes", 5).is_empty());
    }

    #[test]
    fn match_respects_max() {
        let skills = vec![
            Skill {
                name: "a".into(),
                description: "alpha tool".into(),
                path: PathBuf::new(),
                body: String::new(),
                raw: String::new(),
            },
            Skill {
                name: "b".into(),
                description: "alpha tool".into(),
                path: PathBuf::new(),
                body: String::new(),
                raw: String::new(),
            },
        ];
        assert_eq!(match_skills(&skills, "alpha", 1).len(), 1);
    }
}
