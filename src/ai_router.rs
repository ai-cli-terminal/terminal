//! ash 입력 AI 라우팅(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

use std::io::Write;

use crate::aitask::Timeouts;
use crate::config;
use crate::dispatch::{self, AiOutcome, AiResponder, Route};
use crate::gateway::Gateway;
use crate::http::TcpTransport;
use crate::ollama::OllamaBackend;
use crate::openai::OpenAiBackend;
use crate::pipeline::OutputSink;
use crate::policy::PolicyProfile;
use crate::provider::Provider;
use crate::responder::GatewayResponder;
use crate::shellcore::repl::AiRouter;

/// AI 응답을 stdout으로 흘려보내는 sink.
struct StdoutSink;
impl OutputSink for StdoutSink {
    fn write(&mut self, c: &str) {
        print!("{c}");
        let _ = std::io::stdout().flush();
    }
}

/// 자연어 입력을 게이트웨이로 라우팅하는 AiRouter.
pub struct GatewayAiRouter {
    responder: GatewayResponder,
    profile: PolicyProfile,
}

impl GatewayAiRouter {
    /// config의 [ai]에 따라 실 gateway(또는 mock)로 구성한다.
    pub fn from_ai_config(ai: &crate::config::Ai) -> anyhow::Result<Self> {
        let cap = Provider::mock().models[0].clone();
        let gw = match ai.provider.as_str() {
            "ollama" => {
                let b = OllamaBackend::new(TcpTransport, &ai.ollama_url, &ai.model);
                Gateway::new(Box::new(b), cap)
            }
            "openai" => {
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                let b = OpenAiBackend::new(TcpTransport, &ai.openai_url, &ai.model, api_key);
                Gateway::new(Box::new(b), cap)
            }
            _ => Gateway::mock(),
        };
        let responder = GatewayResponder::new(gw, Timeouts::defaults().request)?;
        let profile = PolicyProfile::by_name(&config::get_active_profile())
            .unwrap_or_else(PolicyProfile::balanced);
        Ok(Self { responder, profile })
    }

    /// config 전체를 로드해 [ai]로 구성한다.
    pub fn from_environment() -> anyhow::Result<Self> {
        Self::from_ai_config(&config::load().config.ai)
    }
}

impl AiRouter for GatewayAiRouter {
    fn try_handle(&mut self, input: &str) -> bool {
        let prompt = match dispatch::dispatch(input, &self.profile) {
            Route::Ai { prompt } => prompt,
            _ => return false,
        };
        let mut sink = StdoutSink;
        match self.responder.respond(&prompt, &mut sink) {
            Ok(AiOutcome::Answered { .. }) => println!(),
            Ok(AiOutcome::Blocked(r)) => eprintln!("ash: AI 차단됨: {r}"),
            Ok(AiOutcome::Unavailable(r)) => eprintln!("ash: AI 사용 불가: {r}"),
            Err(e) => eprintln!("ash: AI 오류: {e}"),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::repl::AiRouter;

    fn mock_router() -> GatewayAiRouter {
        GatewayAiRouter::from_ai_config(&crate::config::Ai {
            provider: "mock".to_string(),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn routes_ai_queries_and_leaves_shell() {
        let mut router = mock_router();
        assert!(router.try_handle("how do I undo a commit?")); // AiQuery → handled
        assert!(router.try_handle("ai explain last-error")); // AiInline → handled
        assert!(!router.try_handle("ls -al")); // Shell → not handled
    }

    #[test]
    fn ollama_config_constructs() {
        assert!(GatewayAiRouter::from_ai_config(&crate::config::Ai::default()).is_ok());
    }
}
