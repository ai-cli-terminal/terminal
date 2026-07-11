//! 모바일 AI 보조 구동기 (spec `2026-07-10-android-ai-assist-design.md`).
//!
//! AI 스택(dispatch 분류 → gateway → openai backend)을 모바일 경계에서 동기
//! 구동한다. async는 데스크톱 `responder.rs`와 같은 tokio current-thread
//! 런타임을 재사용한다(신규 의존 없음). 실 HTTP transport는 후속 슬라이스
//! (OkHttp JNI)이며, 이번 슬라이스의 openai provider는 [`UnavailableTransport`]
//! 로 결선되어 정직하게 Unavailable을 돌려준다(가짜 성공 금지). mock provider
//! 는 echo 게이트웨이로 동작해 라우팅·마스킹 경로를 실기기 없이 검증한다.
//!
//! **§3-11**: 이 모듈은 제안 텍스트만 만든다 — 어떤 경로도 명령을 실행하지 않는다.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::aitask::Timeouts;
use crate::gateway::{Gateway, GatewayOutcome};
use crate::http::HttpTransport;
use crate::openai::OpenAiBackend;
use crate::provider::Provider;

/// 모바일 AI 설정. JNI/C ABI 경계로 JSON 전달된다(필드 누락은 기본값).
#[derive(Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct MobileAiConfig {
    /// "mock"(echo) 또는 "openai". 그 외 값은 mock으로 취급.
    pub provider: String,
    pub model: String,
    pub openai_url: String,
    /// 비밀은 호스트 앱이 보관하고 호출 시에만 전달한다(저장하지 않음).
    pub api_key: Option<String>,
}

impl std::fmt::Debug for MobileAiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MobileAiConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("openai_url", &self.openai_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Default for MobileAiConfig {
    fn default() -> Self {
        Self {
            provider: "mock".to_string(),
            model: "default".to_string(),
            openai_url: "https://api.openai.com".to_string(),
            api_key: None,
        }
    }
}

/// AI 제안 결과(JSON 직렬화 계약). kind: "answered" | "blocked" | "unavailable".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobileAiOutcome {
    pub kind: String,
    pub text: String,
}

/// 실 transport 미결선 자리(후속 슬라이스: OkHttp JNI). openai 선택 시
/// 조용한 가짜 성공 대신 명확한 실패를 돌려준다(fail-soft, §3-3).
pub struct UnavailableTransport;

impl HttpTransport for UnavailableTransport {
    async fn post_json(
        &self,
        _url: &str,
        _body: &str,
        _bearer: Option<&str>,
    ) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "android HTTP transport 미결선 (후속 슬라이스: OkHttp JNI)"
        ))
    }
}

/// config와 주입 transport로 게이트웨이를 구성한다.
/// 후속 슬라이스는 실 transport만 바꿔 꽂으면 된다(교체 지점).
pub fn build_gateway<T: HttpTransport + 'static>(cfg: &MobileAiConfig, transport: T) -> Gateway {
    let cap = Provider::mock().models[0].clone();
    match cfg.provider.as_str() {
        "openai" => Gateway::new(
            Box::new(OpenAiBackend::new(
                transport,
                &cfg.openai_url,
                &cfg.model,
                cfg.api_key.clone(),
            )),
            cap,
        ),
        _ => Gateway::mock(),
    }
}

/// 모바일 AI 구동기 — gateway + current-thread 런타임 보유.
pub struct MobileAi {
    gateway: Gateway,
    runtime: tokio::runtime::Runtime,
    timeout: Duration,
}

impl MobileAi {
    /// config로 구성한다(이번 슬라이스의 openai는 [`UnavailableTransport`] 결선).
    pub fn from_config(cfg: &MobileAiConfig) -> anyhow::Result<MobileAi> {
        Self::with_gateway(build_gateway(cfg, UnavailableTransport))
    }

    /// 주어진 게이트웨이로 구성한다(테스트·후속 transport 주입 지점).
    pub fn with_gateway(gateway: Gateway) -> anyhow::Result<MobileAi> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?;
        Ok(MobileAi {
            gateway,
            runtime,
            timeout: Timeouts::defaults().request,
        })
    }

    /// 자연어 prompt에 대한 제안을 만든다(§3-11: 실행하지 않는다).
    /// 실패·타임아웃은 "unavailable"로 흡수해 셸 사용을 막지 않는다(§3-3).
    pub fn suggest(&self, prompt: &str, cwd: &str) -> MobileAiOutcome {
        let ctx = format!("cwd={cwd}");
        let timeout = self.timeout;
        let gw = &self.gateway;
        let result = self.runtime.block_on(async {
            let cancel = Arc::new(Notify::new());
            gw.ask_cancellable(prompt, &ctx, timeout, cancel).await
        });
        match result {
            Ok(GatewayOutcome::Answered { text, .. }) => MobileAiOutcome {
                kind: "answered".to_string(),
                text,
            },
            Ok(GatewayOutcome::Blocked(reason)) => MobileAiOutcome {
                kind: "blocked".to_string(),
                text: reason,
            },
            Err(err) => MobileAiOutcome {
                kind: "unavailable".to_string(),
                text: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_is_redacted_in_debug() {
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-super-secret-value".to_string()),
            ..Default::default()
        };
        let rendered = format!("{cfg:?}");
        assert!(
            !rendered.contains("sk-super-secret-value"),
            "api_key leaked: {rendered}"
        );
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    #[test]
    fn debug_shows_none_api_key_as_none() {
        let cfg = MobileAiConfig::default();
        assert!(format!("{cfg:?}").contains("api_key: None"), "{cfg:?}");
    }

    #[test]
    fn config_json_parses_with_defaults() {
        let cfg: MobileAiConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.provider, "mock");
        assert_eq!(cfg.model, "default");
        assert_eq!(cfg.openai_url, "https://api.openai.com");
        assert_eq!(cfg.api_key, None);

        let cfg: MobileAiConfig =
            serde_json::from_str(r#"{"provider":"openai","api_key":"sk-1"}"#).unwrap();
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-1"));
    }

    #[test]
    fn mock_provider_suggests_via_echo() {
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();
        let out = ai.suggest("큰 파일 찾아줘", "/app");
        assert_eq!(out.kind, "answered");
        assert!(out.text.contains("큰 파일 찾아줘"), "{out:?}");
        assert!(out.text.contains("cwd=/app"), "{out:?}");
    }

    #[test]
    fn openai_provider_with_mock_transport_answers() {
        struct CannedTransport;
        impl HttpTransport for CannedTransport {
            async fn post_json(
                &self,
                url: &str,
                _body: &str,
                bearer: Option<&str>,
            ) -> anyhow::Result<String> {
                assert!(url.ends_with("/v1/chat/completions"), "{url}");
                assert_eq!(bearer, Some("sk-test"));
                Ok(r#"{"choices":[{"message":{"content":"ls -al"}}]}"#.to_string())
            }
        }
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let ai = MobileAi::with_gateway(build_gateway(&cfg, CannedTransport)).unwrap();
        let out = ai.suggest("list files", "/app");
        assert_eq!(out.kind, "answered");
        assert_eq!(out.text, "ls -al");
    }

    #[test]
    fn openai_without_real_transport_is_unavailable() {
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            ..Default::default()
        };
        let ai = MobileAi::from_config(&cfg).unwrap();
        let out = ai.suggest("list files", "/app");
        assert_eq!(out.kind, "unavailable");
        assert!(out.text.contains("transport"), "{out:?}");
    }

    #[test]
    fn private_key_prompt_is_blocked_by_masking() {
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();
        let out = ai.suggest("-----BEGIN OPENSSH PRIVATE KEY-----", "/app");
        assert_eq!(out.kind, "blocked");
    }

    #[test]
    fn unknown_provider_falls_back_to_mock() {
        let cfg = MobileAiConfig {
            provider: "ollama".to_string(),
            ..Default::default()
        };
        let ai = MobileAi::from_config(&cfg).unwrap();
        let out = ai.suggest("what is this?", "/app");
        assert_eq!(out.kind, "answered", "{out:?}");
    }
}
