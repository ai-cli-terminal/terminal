//! AI 응답 캐시 (설계 §29.6, §13 `[ai.cache]`, Phase 2).
//!
//! 정확(exact) 캐시: 키 = **마스킹된** 프롬프트(컨텍스트 포함) 해시 → 응답 + 삽입 시각.
//! TTL 경과 시 무효. 캐시에는 마스킹된 컨텍스트만 저장한다(`docs/RULES.md` §2).
//! 시맨틱 캐시는 후속(임베딩 기반).

use std::collections::HashMap;

/// 응답 출처(캐시 계층 식별 — telemetry/표시용).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// 백엔드가 실제 생성.
    Backend,
    /// exact 캐시 히트.
    Exact,
    /// 시맨틱 캐시 히트(근사).
    Semantic,
}

/// 캐시 기본 용량 상한(항목 수). 장기 세션 메모리 무한 증가 방지.
pub const DEFAULT_CACHE_CAPACITY: usize = 1024;

/// TTL 기반 응답 캐시(in-memory). 용량 상한 초과 시 가장 오래된 항목을 축출한다.
pub struct ResponseCache {
    entries: HashMap<String, (String, u64)>, // key -> (value, inserted_at_ms)
    ttl_ms: u64,
    capacity: usize,
}

impl ResponseCache {
    /// `ttl_secs` TTL의 빈 캐시(기본 용량 상한).
    pub fn new(ttl_secs: u64) -> ResponseCache {
        Self::with_capacity(ttl_secs, DEFAULT_CACHE_CAPACITY)
    }

    /// `ttl_secs` TTL + 항목 수 상한(`max_entries`, 최소 1)으로 만든다.
    pub fn with_capacity(ttl_secs: u64, max_entries: usize) -> ResponseCache {
        ResponseCache {
            entries: HashMap::new(),
            ttl_ms: ttl_secs.saturating_mul(1000),
            capacity: max_entries.max(1),
        }
    }

    /// 프롬프트로부터 결정적 캐시 키를 만든다.
    pub fn key(prompt: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        prompt.hash(&mut h);
        format!("{:016x}", h.finish())
    }

    /// 유효한(만료 전) 값을 반환한다.
    pub fn get(&self, key: &str, now_ms: u64) -> Option<&str> {
        let (value, inserted) = self.entries.get(key)?;
        if now_ms.saturating_sub(*inserted) > self.ttl_ms {
            return None;
        }
        Some(value.as_str())
    }

    /// 값을 저장한다. 새 키로 용량을 초과하면 가장 오래된(삽입 시각 최소) 항목을 축출한다.
    pub fn put(&mut self, key: String, value: String, now_ms: u64) {
        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, t))| *t)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, (value, now_ms));
    }

    /// 저장된 항목 수.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 비었는지.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 두 텍스트의 단어 집합 Jaccard 유사도(0.0~1.0).
pub fn similarity(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let wa: HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let wb: HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    if wa.is_empty() && wb.is_empty() {
        return 1.0;
    }
    let inter = wa.intersection(&wb).count() as f64;
    let union = wa.union(&wb).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// 유사도 기반 시맨틱 캐시(임베딩 없이 단어 집합 유사도 휴리스틱).
/// 용량 상한 초과 시 가장 오래된 항목을 축출한다(선형 탐색 비용도 함께 제어).
pub struct SemanticCache {
    entries: Vec<(String, String, u64)>, // (prompt, value, inserted_at_ms) — 삽입 순서 유지
    ttl_ms: u64,
    threshold: f64,
    capacity: usize,
}

impl SemanticCache {
    /// `ttl_secs` TTL, `threshold`(0~1) 이상 유사 시 히트(기본 용량 상한).
    pub fn new(ttl_secs: u64, threshold: f64) -> SemanticCache {
        Self::with_capacity(ttl_secs, threshold, DEFAULT_CACHE_CAPACITY)
    }

    /// TTL·threshold·항목 수 상한(`max_entries`, 최소 1)으로 만든다.
    pub fn with_capacity(ttl_secs: u64, threshold: f64, max_entries: usize) -> SemanticCache {
        SemanticCache {
            entries: Vec::new(),
            ttl_ms: ttl_secs.saturating_mul(1000),
            threshold,
            capacity: max_entries.max(1),
        }
    }

    /// 프롬프트-응답을 저장한다. 만료 항목을 먼저 정리하고, 용량 초과 시 가장 오래된
    /// (앞쪽) 항목을 축출한다(삽입 순서 = 시간 순서).
    pub fn put(&mut self, prompt: String, value: String, now_ms: u64) {
        self.entries
            .retain(|(_, _, t)| now_ms.saturating_sub(*t) <= self.ttl_ms);
        while self.entries.len() >= self.capacity && !self.entries.is_empty() {
            self.entries.remove(0);
        }
        self.entries.push((prompt, value, now_ms));
    }

    /// 저장된 항목 수.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 비었는지.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 임계값 이상으로 가장 유사한(만료 전) 응답을 반환한다.
    pub fn get_similar(&self, prompt: &str, now_ms: u64) -> Option<&str> {
        self.entries
            .iter()
            .filter(|(_, _, t)| now_ms.saturating_sub(*t) <= self.ttl_ms)
            .map(|(p, v, _)| (similarity(prompt, p), v))
            .filter(|(s, _)| *s >= self.threshold)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_similarity() {
        assert_eq!(similarity("a b c", "a b c"), 1.0);
        assert_eq!(similarity("a b", "c d"), 0.0);
        assert!(similarity("find big files", "find large files") > 0.0);
    }

    #[test]
    fn semantic_hit_on_similar_prompt() {
        let mut c = SemanticCache::new(3600, 0.5);
        c.put("how to list files".into(), "use ls".into(), 1000);
        assert_eq!(c.get_similar("how to list all files", 1000), Some("use ls"));
    }

    #[test]
    fn semantic_miss_below_threshold() {
        let mut c = SemanticCache::new(3600, 0.5);
        c.put("how to list files".into(), "use ls".into(), 1000);
        assert_eq!(c.get_similar("docker compose up now", 1000), None);
    }

    #[test]
    fn semantic_respects_ttl() {
        let mut c = SemanticCache::new(10, 0.3);
        c.put("alpha beta".into(), "v".into(), 1000);
        assert_eq!(c.get_similar("alpha beta", 1000 + 11_000), None);
    }

    #[test]
    fn put_then_get_is_hit() {
        let mut c = ResponseCache::new(3600);
        c.put("k".into(), "v".into(), 1000);
        assert_eq!(c.get("k", 1000), Some("v"));
        assert_eq!(c.get("k", 2000), Some("v"));
    }

    #[test]
    fn missing_key_is_none() {
        let c = ResponseCache::new(3600);
        assert_eq!(c.get("nope", 0), None);
    }

    #[test]
    fn expired_entry_is_none() {
        let mut c = ResponseCache::new(10); // 10s = 10_000ms
        c.put("k".into(), "v".into(), 1_000);
        assert_eq!(c.get("k", 1_000 + 9_000), Some("v"));
        assert_eq!(c.get("k", 1_000 + 11_000), None);
    }

    #[test]
    fn key_is_deterministic_and_distinct() {
        assert_eq!(ResponseCache::key("hello"), ResponseCache::key("hello"));
        assert_ne!(ResponseCache::key("hello"), ResponseCache::key("world"));
    }

    #[test]
    fn response_cache_evicts_oldest_over_capacity() {
        let mut c = ResponseCache::with_capacity(3600, 2);
        c.put("a".into(), "1".into(), 1000);
        c.put("b".into(), "2".into(), 1001);
        c.put("c".into(), "3".into(), 1002); // 용량 초과 → 가장 오래된 "a" 축출
        assert_eq!(c.len(), 2, "capacity must be bounded");
        assert_eq!(c.get("a", 1002), None, "oldest evicted");
        assert_eq!(c.get("b", 1002), Some("2"));
        assert_eq!(c.get("c", 1002), Some("3"));
    }

    #[test]
    fn response_cache_update_existing_does_not_evict() {
        let mut c = ResponseCache::with_capacity(3600, 2);
        c.put("a".into(), "1".into(), 1000);
        c.put("b".into(), "2".into(), 1001);
        c.put("a".into(), "1b".into(), 1002); // 기존 키 갱신 → 축출 없음
        assert_eq!(c.len(), 2);
        assert_eq!(c.get("a", 1002), Some("1b"));
        assert_eq!(c.get("b", 1002), Some("2"));
    }

    #[test]
    fn semantic_cache_bounded_by_capacity() {
        let mut c = SemanticCache::with_capacity(3600, 0.1, 2);
        c.put("alpha one".into(), "1".into(), 1000);
        c.put("beta two".into(), "2".into(), 1001);
        c.put("gamma three".into(), "3".into(), 1002); // 초과 → 가장 오래된 축출
        assert!(c.len() <= 2, "semantic cache must be bounded: {}", c.len());
        assert_eq!(c.get_similar("alpha one", 1002), None, "oldest evicted");
        assert_eq!(c.get_similar("gamma three", 1002), Some("3"));
    }
}
