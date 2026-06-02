//! Token Window Management (설계 §13 `[ai.context]`, §31.9, M4/W15).
//!
//! 컨텍스트를 토큰 예산에 맞춰 추정·청킹한다. MVP는 char/4 휴리스틱(결정성)이며,
//! provider가 token counting을 지원하면 그 값을 우선한다(§31.9 fallback).

/// 대략적 토큰 수(char/4 ceil). 빈 문자열은 0.
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    chars.div_ceil(4)
}

/// 텍스트를 토큰 윈도(겹침 포함)로 분할한다.
pub fn chunk(text: &str, chunk_tokens: usize, overlap_tokens: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || chunk_tokens == 0 {
        return vec![];
    }
    let size = chunk_tokens * 4;
    let overlap = (overlap_tokens * 4).min(size.saturating_sub(1));
    let step = (size - overlap).max(1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        out.push(chars[start..end].iter().collect());
        if end == chars.len() {
            break;
        }
        start += step;
    }
    out
}

/// 토큰 예산 내에 들어오는지.
pub fn fits(text: &str, max_tokens: usize) -> bool {
    estimate_tokens(text) <= max_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_is_char_quarter() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(estimate_tokens("abcde"), 2); // ceil(5/4)
    }

    #[test]
    fn chunk_splits_into_windows() {
        let text: String = "x".repeat(100);
        let chunks = chunk(&text, 5, 0); // 5 tokens = 20 chars, no overlap
        assert_eq!(chunks.len(), 5);
        assert_eq!(chunks[0].chars().count(), 20);
    }

    #[test]
    fn chunk_overlap_shares_chars() {
        let text: String = ('a'..='z').collect(); // 26 chars
        let chunks = chunk(&text, 5, 1); // size 20, overlap 4, step 16
        assert!(chunks.len() >= 2);
        // 두 번째 청크는 첫 청크 끝 4자와 겹친다.
        let first_tail: String = chunks[0]
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert!(
            chunks[1].starts_with(&first_tail),
            "overlap mismatch: {chunks:?}"
        );
    }

    #[test]
    fn fits_within_budget() {
        assert!(fits("abcd", 1));
        assert!(!fits("abcdefgh", 1));
    }
}
