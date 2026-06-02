//! Secret / PII 마스킹 파이프라인 (설계 §31.8, M1/W7).
//!
//! 확정값(§31.8): `mask_secrets`/`mask_pii` 기본 ON, 마스킹 실패 시 원격 AI 차단(fail-closed).
//! 순서: Raw → Secret 탐지 → PII 탐지 → Masking → Validation Scan → Remote Eligibility.
//!
//! 불변식(`docs/RULES.md` §2): 원문 secret은 디스크/로그/컨텍스트에 남기지 않는다.
//! private key block 감지 시 원격 호출을 **차단**한다(고엔트로피 키는 안전 마스킹 불확실).

use regex::Regex;

/// 규칙 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskKind {
    Secret,
    Pii,
}

/// 단일 마스킹 규칙.
pub struct MaskRule {
    pub name: &'static str,
    pub kind: MaskKind,
    pub replacement: &'static str,
    /// true면 매치 시 원격 호출을 차단한다(fail-closed).
    pub hard_block: bool,
    pattern: Regex,
}

/// 마스킹 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskOutcome {
    /// 마스킹된 텍스트.
    pub text: String,
    /// 발동한 규칙 이름들.
    pub redactions: Vec<&'static str>,
    /// 원격 AI 전송 차단 여부(true면 보내면 안 됨).
    pub blocked: bool,
    /// 차단 사유.
    pub block_reason: Option<String>,
}

/// 마스킹 엔진.
pub struct Masker {
    rules: Vec<MaskRule>,
}

impl Masker {
    /// §31.8 baseline 규칙으로 구성한다(Secret 먼저, PII 나중).
    pub fn baseline() -> Masker {
        let r = |name, kind, hard_block, pat: &str, replacement| MaskRule {
            name,
            kind,
            replacement,
            hard_block,
            pattern: Regex::new(pat).expect("baseline regex must compile"),
        };
        use MaskKind::{Pii, Secret};
        let rules = vec![
            // --- Secrets (먼저 적용) ---
            r(
                "private_key_block",
                Secret,
                true,
                r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
                "[PRIVATE_KEY_REDACTED]",
            ),
            r(
                "aws_access_key",
                Secret,
                false,
                r"AKIA[0-9A-Z]{16}",
                "[AWS_ACCESS_KEY_REDACTED]",
            ),
            r(
                "github_token",
                Secret,
                false,
                r"gh[pousr]_[A-Za-z0-9]{20,}",
                "[GITHUB_TOKEN_REDACTED]",
            ),
            r(
                "slack_token",
                Secret,
                false,
                r"xox[baprs]-[A-Za-z0-9-]{10,}",
                "[SLACK_TOKEN_REDACTED]",
            ),
            r(
                "bearer_token",
                Secret,
                false,
                r"Bearer\s+[A-Za-z0-9._~+/=-]+",
                "Bearer [TOKEN_REDACTED]",
            ),
            r(
                "authorization_header",
                Secret,
                false,
                r"(?i)authorization:\s*\S+",
                "[AUTHORIZATION_REDACTED]",
            ),
            r(
                "password_assignment",
                Secret,
                false,
                r"(?i)(password|passwd|pwd)\s*[=:]\s*\S+",
                "[PASSWORD_REDACTED]",
            ),
            // --- PII (나중에 적용) ---
            r(
                "email",
                Pii,
                false,
                r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
                "[EMAIL_REDACTED]",
            ),
            r(
                "kr_rrn",
                Pii,
                false,
                r"\b\d{6}-?[1-4]\d{6}\b",
                "[KR_RRN_REDACTED]",
            ),
            r(
                "ipv4",
                Pii,
                false,
                r"\b(?:\d{1,3}\.){3}\d{1,3}\b",
                "[IP_REDACTED]",
            ),
        ];
        Masker { rules }
    }

    /// 입력을 마스킹하고 원격 전송 가능 여부를 판정한다.
    pub fn mask(&self, input: &str) -> MaskOutcome {
        let mut text = input.to_string();
        let mut redactions = Vec::new();
        let mut blocked = false;
        let mut block_reason = None;

        for rule in &self.rules {
            if rule.pattern.is_match(&text) {
                redactions.push(rule.name);
                if rule.hard_block && !blocked {
                    blocked = true;
                    block_reason = Some(format!(
                        "{} detected; remote AI blocked (fail-closed)",
                        rule.name
                    ));
                }
                text = rule
                    .pattern
                    .replace_all(&text, rule.replacement)
                    .into_owned();
            }
        }

        // Validation scan: 마스킹 후에도 secret 패턴이 남으면 차단(fail-closed).
        for rule in self.rules.iter().filter(|r| r.kind == MaskKind::Secret) {
            if rule.pattern.is_match(&text) {
                blocked = true;
                block_reason = Some(format!("masking validation failed for {}", rule.name));
            }
        }

        MaskOutcome {
            text,
            redactions,
            blocked,
            block_reason,
        }
    }
}

/// 원격 컨텍스트에서 기본 제외할 민감 파일인지(§31.8: `.env` 등).
pub fn is_sensitive_path(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name == ".env"
        || name.starts_with(".env.")
        || name == "id_rsa"
        || name == "credentials"
        || name.ends_with(".pem")
        || name.ends_with(".key")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_pii_email_and_ip() {
        let m = Masker::baseline();
        let out = m.mask("mail me at a.b@example.com from 10.0.0.1");
        assert!(out.text.contains("[EMAIL_REDACTED]"), "{}", out.text);
        assert!(out.text.contains("[IP_REDACTED]"), "{}", out.text);
        assert!(!out.text.contains("example.com"));
        assert!(!out.text.contains("10.0.0.1"));
    }

    #[test]
    fn masks_secrets_aws_and_bearer() {
        let m = Masker::baseline();
        let out = m.mask("key AKIAIOSFODNN7EXAMPLE and Bearer abc.def-123");
        assert!(
            out.text.contains("[AWS_ACCESS_KEY_REDACTED]"),
            "{}",
            out.text
        );
        assert!(out.text.contains("Bearer [TOKEN_REDACTED]"), "{}", out.text);
        assert!(!out.text.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn private_key_block_blocks_remote() {
        let m = Masker::baseline();
        let out = m.mask("-----BEGIN OPENSSH PRIVATE KEY-----\nxxxx");
        assert!(out.blocked, "private key must block remote");
        assert!(out.block_reason.is_some());
        assert!(out.text.contains("[PRIVATE_KEY_REDACTED]"));
    }

    #[test]
    fn kr_rrn_is_masked() {
        let m = Masker::baseline();
        let out = m.mask("주민번호 900101-1234567 입니다");
        assert!(out.text.contains("[KR_RRN_REDACTED]"), "{}", out.text);
        assert!(!out.text.contains("900101-1234567"));
    }

    #[test]
    fn clean_text_is_not_blocked_and_unchanged() {
        let m = Masker::baseline();
        let out = m.mask("just run ls -al in the project");
        assert!(out.redactions.is_empty());
        assert!(!out.blocked);
        assert_eq!(out.text, "just run ls -al in the project");
    }

    #[test]
    fn no_raw_secret_survives_masking() {
        let m = Masker::baseline();
        let out = m.mask("token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
        assert!(!out
            .text
            .contains("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"));
        assert!(out.redactions.contains(&"github_token"));
    }

    #[test]
    fn dotenv_is_sensitive() {
        assert!(is_sensitive_path("/home/u/project/.env"));
        assert!(is_sensitive_path(".env.local"));
        assert!(!is_sensitive_path("/home/u/project/main.rs"));
    }
}
