//! OpenAI-호환 HTTP provider 어댑터 (설계 §21 원격 AI, Phase 2).
//!
//! `/v1/chat/completions`(stream=false)로 OpenAI 호환 엔드포인트를 호출한다
//! (로컬 llama.cpp/LM Studio/vLLM 등은 평문 HTTP로 [`TcpTransport`] 사용 가능).
//! 클라우드 HTTPS 엔드포인트는 TLS 지원 transport가 필요하다(후속).

use anyhow::{anyhow, Result};

use crate::gateway::LlmBackend;
use crate::http::HttpTransport;

/// OpenAI 호환 chat 백엔드(전송 주입).
pub struct OpenAiBackend<T: HttpTransport> {
    transport: T,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl<T: HttpTransport> OpenAiBackend<T> {
    pub fn new(
        transport: T,
        base_url: &str,
        model: &str,
        api_key: Option<String>,
    ) -> OpenAiBackend<T> {
        OpenAiBackend {
            transport,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
        }
    }
}

impl<T: HttpTransport> LlmBackend for OpenAiBackend<T> {
    fn generate(&self, prompt: &str) -> Result<String> {
        let body = build_request(&self.model, prompt);
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .transport
            .post_json(&url, &body, self.api_key.as_deref())?;
        parse_response(&resp)
    }
}

/// chat completions 요청 본문(단일 user 메시지, stream=false).
pub fn build_request(model: &str, prompt: &str) -> String {
    serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    })
    .to_string()
}

/// 응답에서 `choices[0].message.content`를 추출한다.
pub fn parse_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)?;
    v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("openai response missing choices[0].message.content"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockTransport {
        reply: String,
        last_bearer: Mutex<Option<String>>,
    }
    impl HttpTransport for MockTransport {
        fn post_json(&self, _url: &str, _body: &str, bearer: Option<&str>) -> Result<String> {
            *self.last_bearer.lock().unwrap() = bearer.map(String::from);
            Ok(self.reply.clone())
        }
    }

    #[test]
    fn request_is_chat_completions_shape() {
        let b = build_request("gpt-x", "hello");
        assert!(b.contains("\"model\":\"gpt-x\""));
        assert!(b.contains("\"role\":\"user\""));
        assert!(b.contains("\"content\":\"hello\""));
        assert!(b.contains("\"stream\":false"));
    }

    #[test]
    fn parses_choice_content() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"hi!"}}]}"#;
        assert_eq!(parse_response(body).unwrap(), "hi!");
        assert!(parse_response(r#"{"choices":[]}"#).is_err());
    }

    #[test]
    fn generate_passes_api_key_as_bearer() {
        let mock = MockTransport {
            reply: r#"{"choices":[{"message":{"content":"ok"}}]}"#.to_string(),
            last_bearer: Mutex::new(None),
        };
        let backend =
            OpenAiBackend::new(mock, "http://localhost:8080", "m", Some("sk-test".into()));
        assert_eq!(backend.generate("q").unwrap(), "ok");
        assert_eq!(
            backend.transport.last_bearer.lock().unwrap().as_deref(),
            Some("sk-test")
        );
    }
}
