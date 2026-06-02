//! AI 응답 캐시 (설계 §29.6, §13 `[ai.cache]`, Phase 2).
//!
//! 정확(exact) 캐시: 키 = **마스킹된** 프롬프트(컨텍스트 포함) 해시 → 응답 + 삽입 시각.
//! TTL 경과 시 무효. 캐시에는 마스킹된 컨텍스트만 저장한다(`docs/RULES.md` §2).
//! 시맨틱 캐시는 후속(임베딩 기반).

use std::collections::HashMap;

/// TTL 기반 응답 캐시(in-memory).
pub struct ResponseCache {
    entries: HashMap<String, (String, u64)>, // key -> (value, inserted_at_ms)
    ttl_ms: u64,
}

impl ResponseCache {
    /// `ttl_secs` TTL의 빈 캐시.
    pub fn new(ttl_secs: u64) -> ResponseCache {
        ResponseCache {
            entries: HashMap::new(),
            ttl_ms: ttl_secs.saturating_mul(1000),
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

    /// 값을 저장한다.
    pub fn put(&mut self, key: String, value: String, now_ms: u64) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
