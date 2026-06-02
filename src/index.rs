//! Semantic File Index (설계 §25.2 Semantic File Index, Phase 2).
//!
//! 프로젝트 파일을 키워드 인덱싱해 컨텍스트 검색에 쓴다(MVP: ripgrep/FTS 대용 키워드 매칭).
//! 임베딩 기반 시맨틱 검색은 후속. 무시 디렉터리·대용량/바이너리 파일은 건너뛴다.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

const IGNORE_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".ai-terminal",
    "dist",
    "build",
];
const MAX_FILE_BYTES: u64 = 1_000_000;

/// 파일 → 단어 집합 인덱스.
pub struct FileIndex {
    docs: Vec<(PathBuf, HashSet<String>)>,
}

impl FileIndex {
    /// `root` 아래 텍스트 파일을 인덱싱한다.
    pub fn build(root: &Path) -> FileIndex {
        let mut docs = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skip = path
                        .file_name()
                        .map(|n| IGNORE_DIRS.contains(&n.to_string_lossy().as_ref()))
                        .unwrap_or(false);
                    if !skip {
                        stack.push(path);
                    }
                } else if let Ok(meta) = entry.metadata() {
                    if meta.len() <= MAX_FILE_BYTES {
                        if let Ok(text) = std::fs::read_to_string(&path) {
                            docs.push((path, tokenize(&text)));
                        }
                    }
                }
            }
        }
        FileIndex { docs }
    }

    /// 쿼리 단어가 가장 많이 매칭되는 파일을 점수순 최대 `max`개 반환한다.
    pub fn search(&self, query: &str, max: usize) -> Vec<(PathBuf, usize)> {
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let mut scored: Vec<(PathBuf, usize)> = self
            .docs
            .iter()
            .filter_map(|(path, doc_words)| {
                let score = words
                    .iter()
                    .filter(|w| doc_words.contains(w.as_str()))
                    .count();
                (score > 0).then(|| (path.clone(), score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.truncate(max);
        scored
    }

    /// 인덱싱된 문서 수.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// 비었는지.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

/// 텍스트를 소문자 단어 집합으로 토큰화한다(2자 이상).
fn tokenize(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(|w| w.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn uniq() -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_index_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn builds_and_searches() {
        let root = uniq();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.rs"), "fn main risk score engine").unwrap();
        std::fs::write(root.join("b.md"), "docker compose deployment guide").unwrap();

        let idx = FileIndex::build(&root);
        assert_eq!(idx.len(), 2);

        let r = idx.search("risk", 5);
        assert_eq!(r.len(), 1);
        assert!(r[0].0.ends_with("a.rs"));

        assert!(idx.search("docker", 5)[0].0.ends_with("b.md"));
        assert!(idx.search("nonexistentword", 5).is_empty());
    }

    #[test]
    fn skips_ignored_dirs() {
        let root = uniq();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("target").join("junk.rs"), "risk risk risk").unwrap();
        std::fs::write(root.join("real.rs"), "risk here").unwrap();

        let idx = FileIndex::build(&root);
        let r = idx.search("risk", 5);
        assert_eq!(r.len(), 1, "target/ must be skipped");
        assert!(r[0].0.ends_with("real.rs"));
    }

    #[test]
    fn ranks_by_match_count() {
        let root = uniq();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("more.txt"), "git commit branch merge").unwrap();
        std::fs::write(root.join("less.txt"), "git only").unwrap();
        let idx = FileIndex::build(&root);
        let r = idx.search("git commit branch", 5);
        assert!(
            r[0].0.ends_with("more.txt"),
            "more matches should rank first: {r:?}"
        );
    }
}
