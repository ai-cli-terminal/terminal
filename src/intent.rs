//! Intent Classifier (설계 §5 Input Handler, Phase 2).
//!
//! 입력을 일반 셸 경로 / AI 경로로 분기하기 위한 의도 분류(결정성 규칙 기반).
//! Hybrid Mode의 토대. AI 분류는 보조 신호이며 로컬 규칙이 우선한다(`docs/RULES.md`).

/// 입력 의도.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// 빈 입력.
    Empty,
    /// 명시적 AI 호출(`ai ...`).
    AiInline,
    /// 자연어 질의(AI 경로).
    AiQuery,
    /// 일반 셸 명령.
    Shell,
}

const QUESTION_WORDS: &[&str] = &[
    "how",
    "what",
    "why",
    "where",
    "when",
    "who",
    "which",
    "please",
    "explain",
    "summarize",
    "describe",
    "help",
];

const KR_REQUEST_MARKERS: &[&str] = &[
    "어떻게",
    "방법",
    "알려",
    "해줘",
    "만들어",
    "보여줘",
    "찾아줘",
    "설명",
    "분석",
    "요약",
];

/// 입력의 의도를 분류한다. 로컬 규칙 기반(deterministic).
pub fn classify(input: &str) -> Intent {
    let t = input.trim();
    if t.is_empty() {
        return Intent::Empty;
    }
    let first = t.split_whitespace().next().unwrap_or("");
    if first == "ai" {
        return Intent::AiInline;
    }
    if t.ends_with('?') {
        return Intent::AiQuery;
    }
    if QUESTION_WORDS.contains(&first.to_lowercase().as_str()) {
        return Intent::AiQuery;
    }
    if KR_REQUEST_MARKERS.iter().any(|m| t.contains(m)) {
        return Intent::AiQuery;
    }
    Intent::Shell
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(classify(""), Intent::Empty);
        assert_eq!(classify("   "), Intent::Empty);
    }

    #[test]
    fn explicit_ai_invocation() {
        assert_eq!(classify("ai explain last-error"), Intent::AiInline);
        assert_eq!(classify("ai"), Intent::AiInline);
    }

    #[test]
    fn shell_commands() {
        assert_eq!(classify("ls -al"), Intent::Shell);
        assert_eq!(classify("git status"), Intent::Shell);
        assert_eq!(classify("find . -name '*.rs'"), Intent::Shell);
        assert_eq!(classify("rm -rf /tmp/x"), Intent::Shell);
    }

    #[test]
    fn natural_language_queries() {
        assert_eq!(classify("how do I undo a commit?"), Intent::AiQuery);
        assert_eq!(classify("what does this error mean"), Intent::AiQuery);
        assert_eq!(classify("please summarize the log"), Intent::AiQuery);
    }

    #[test]
    fn korean_requests() {
        assert_eq!(classify("큰 파일 찾아줘"), Intent::AiQuery);
        assert_eq!(classify("이 로그 분석해줘"), Intent::AiQuery);
        assert_eq!(classify("커밋 되돌리는 방법"), Intent::AiQuery);
    }
}
