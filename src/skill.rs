//! 통합 스킬 관리 (설계 §26, Phase 2).
//!
//! SKILL.md(프론트매터: name/description) 기반으로 스킬을 발견·매칭·로딩한다.
//! 스킬 콘텐츠는 **Zero-Trust 데이터**로 취급한다 — 여기서는 발견/매칭/로딩만 하고
//! 스크립트 실행은 일반 명령과 동일한 정책·샌드박스 경계를 거쳐야 한다(§26, RULES §2).

use std::path::{Path, PathBuf};

/// 발견된 스킬.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub body: String,
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
    let mut out = Vec::new();
    for base in paths {
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
                    out.push(parse_skill(&content, &skill_md));
                }
            }
        }
    }
    out
}

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
    fn matches_by_keyword_ranked() {
        let skills = vec![
            Skill {
                name: "git-helper".into(),
                description: "helps with git commit and branch".into(),
                path: PathBuf::new(),
                body: String::new(),
            },
            Skill {
                name: "docker".into(),
                description: "container stuff".into(),
                path: PathBuf::new(),
                body: String::new(),
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
            },
            Skill {
                name: "b".into(),
                description: "alpha tool".into(),
                path: PathBuf::new(),
                body: String::new(),
            },
        ];
        assert_eq!(match_skills(&skills, "alpha", 1).len(), 1);
    }
}
