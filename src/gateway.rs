//! AI Model Gateway — AI 요청 파이프라인 (설계 §5 AI Service Layer, Phase 2).
//!
//! 경로: prompt+context → **마스킹**(secret 차단/치환) → 토큰 윈도 → 백엔드 생성 → usage.
//! 마스킹 실패(예: private key) 시 원격 전송을 차단한다(fail-closed, `docs/RULES.md` §2).
//! 실제 provider(HTTP/Ollama)는 [`LlmBackend`] 구현으로 주입한다. MVP는 mock으로 검증.

use anyhow::Result;

use crate::mask::Masker;
use crate::provider::ModelCapability;
use crate::tokenwin;

/// LLM 백엔드 인터페이스. 실제 provider/로컬 LLM이 구현한다.
pub trait LlmBackend {
    /// (이미 마스킹된) 프롬프트로 응답을 생성한다.
    fn generate(&self, prompt: &str) -> Result<String>;
}

/// 테스트/CI용 echo 백엔드 — 입력을 그대로 되돌려 마스킹 적용을 검증 가능하게 한다.
pub struct EchoBackend;

impl LlmBackend for EchoBackend {
    fn generate(&self, prompt: &str) -> Result<String> {
        Ok(format!("echo: {prompt}"))
    }
}

/// 게이트웨이 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayOutcome {
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
    },
    /// 마스킹 실패 등으로 원격 전송 차단.
    Blocked(String),
}

/// AI 요청 게이트웨이.
pub struct Gateway {
    backend: Box<dyn LlmBackend>,
    cap: ModelCapability,
    masker: Masker,
}

impl Gateway {
    /// 주어진 백엔드·capability로 게이트웨이를 만든다.
    pub fn new(backend: Box<dyn LlmBackend>, cap: ModelCapability) -> Gateway {
        Gateway {
            backend,
            cap,
            masker: Masker::baseline(),
        }
    }

    /// mock(echo) 백엔드 게이트웨이.
    pub fn mock() -> Gateway {
        let cap = crate::provider::Provider::mock().models[0].clone();
        Gateway::new(Box::new(EchoBackend), cap)
    }

    /// prompt+context를 마스킹·토큰 점검 후 백엔드로 보내 응답을 만든다.
    pub fn ask(&self, prompt: &str, context: &str) -> Result<GatewayOutcome> {
        let combined = if context.is_empty() {
            prompt.to_string()
        } else {
            format!("{context}\n{prompt}")
        };

        // 1) 마스킹 (실패 시 fail-closed 차단)
        let masked = self.masker.mask(&combined);
        if masked.blocked {
            return Ok(GatewayOutcome::Blocked(
                masked
                    .block_reason
                    .unwrap_or_else(|| "masking failed".into()),
            ));
        }

        // 2) 토큰 윈도 — 컨텍스트 한도 초과 시 앞부분으로 자른다(MVP: 단순 truncate).
        let mut sent = masked.text;
        let max = self.cap.max_context_tokens as usize;
        if tokenwin::estimate_tokens(&sent) > max {
            let chunks = tokenwin::chunk(&sent, max, 0);
            sent = chunks.into_iter().next().unwrap_or_default();
        }
        let input_tokens = tokenwin::estimate_tokens(&sent);

        // 3) 백엔드 생성
        let text = self.backend.generate(&sent)?;
        let output_tokens = tokenwin::estimate_tokens(&text);

        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_with_backend() {
        let gw = Gateway::mock();
        let out = gw.ask("hello", "").unwrap();
        match out {
            GatewayOutcome::Answered {
                text, input_tokens, ..
            } => {
                assert!(text.contains("echo: "), "{text}");
                assert!(text.contains("hello"));
                assert!(input_tokens > 0);
            }
            GatewayOutcome::Blocked(r) => panic!("unexpected block: {r}"),
        }
    }

    #[test]
    fn masks_secret_before_backend() {
        let gw = Gateway::mock();
        let out = gw.ask("my key AKIAIOSFODNN7EXAMPLE", "").unwrap();
        match out {
            GatewayOutcome::Answered { text, .. } => {
                // echo 백엔드는 받은 입력을 그대로 반환하므로, 원문이 없으면 마스킹된 것.
                assert!(
                    !text.contains("AKIAIOSFODNN7EXAMPLE"),
                    "secret leaked: {text}"
                );
                assert!(text.contains("[AWS_ACCESS_KEY_REDACTED]"), "{text}");
            }
            GatewayOutcome::Blocked(r) => panic!("unexpected block: {r}"),
        }
    }

    #[test]
    fn blocks_on_private_key() {
        let gw = Gateway::mock();
        let out = gw.ask("-----BEGIN OPENSSH PRIVATE KEY-----", "").unwrap();
        assert!(matches!(out, GatewayOutcome::Blocked(_)), "{out:?}");
    }

    #[test]
    fn context_is_included_in_prompt() {
        let gw = Gateway::mock();
        let out = gw.ask("question", "cwd=/srv/app").unwrap();
        match out {
            GatewayOutcome::Answered { text, .. } => assert!(text.contains("cwd=/srv/app")),
            GatewayOutcome::Blocked(r) => panic!("unexpected block: {r}"),
        }
    }
}
