//! Ollama 로컬 LLM 백엔드 (설계 §24 로컬 LLM, Phase 2).
//!
//! `/api/generate`(stream=false)로 로컬 모델을 호출한다. 요청 빌드/응답 파싱은 순수 함수로
//! 분리해 오프라인 테스트하고, 실제 호출은 주입된 [`HttpTransport`]가 담당한다.

use anyhow::{anyhow, Result};

use crate::gateway::LlmBackend;
use crate::http::HttpTransport;

/// Ollama 백엔드(전송 주입).
pub struct OllamaBackend<T: HttpTransport> {
    transport: T,
    base_url: String,
    model: String,
}

impl<T: HttpTransport> OllamaBackend<T> {
    pub fn new(transport: T, base_url: &str, model: &str) -> OllamaBackend<T> {
        OllamaBackend {
            transport,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
        }
    }
}

impl<T: HttpTransport> LlmBackend for OllamaBackend<T> {
    fn generate(&self, prompt: &str) -> Result<String> {
        let body = build_request(&self.model, prompt);
        let url = format!("{}/api/generate", self.base_url);
        let resp = self.transport.post_json(&url, &body, None)?;
        parse_response(&resp)
    }
}

/// `/api/generate` 요청 본문(stream=false).
pub fn build_request(model: &str, prompt: &str) -> String {
    serde_json::json!({ "model": model, "prompt": prompt, "stream": false }).to_string()
}

/// 응답 본문에서 `response` 필드를 추출한다.
pub fn parse_response(body: &str) -> Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)?;
    v.get("response")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("ollama response missing 'response' field"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockTransport {
        reply: String,
        last_url: Mutex<String>,
        last_body: Mutex<String>,
    }
    impl HttpTransport for MockTransport {
        fn post_json(&self, url: &str, body: &str, _bearer: Option<&str>) -> Result<String> {
            *self.last_url.lock().unwrap() = url.to_string();
            *self.last_body.lock().unwrap() = body.to_string();
            Ok(self.reply.clone())
        }
    }

    #[test]
    fn request_has_model_prompt_and_no_stream() {
        let b = build_request("qwen2.5-coder", "hello");
        assert!(b.contains("\"model\":\"qwen2.5-coder\""));
        assert!(b.contains("\"prompt\":\"hello\""));
        assert!(b.contains("\"stream\":false"));
    }

    #[test]
    fn parses_response_field() {
        let out = parse_response(r#"{"response":"hi there","done":true}"#).unwrap();
        assert_eq!(out, "hi there");
        assert!(parse_response(r#"{"done":true}"#).is_err());
    }

    #[test]
    fn generate_uses_transport_and_parses() {
        let mock = MockTransport {
            reply: r#"{"response":"42","done":true}"#.to_string(),
            last_url: Mutex::new(String::new()),
            last_body: Mutex::new(String::new()),
        };
        let backend = OllamaBackend::new(mock, "http://localhost:11434/", "m");
        let out = backend.generate("q").unwrap();
        assert_eq!(out, "42");
        assert_eq!(
            backend.transport.last_url.lock().unwrap().as_str(),
            "http://localhost:11434/api/generate"
        );
        assert!(backend
            .transport
            .last_body
            .lock()
            .unwrap()
            .contains("\"prompt\":\"q\""));
    }
}
