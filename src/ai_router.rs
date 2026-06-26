//! ash 입력 AI 라우팅(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

use std::io::Write;

use crate::config;
use crate::dispatch::{self, AiOutcome, AiResponder, Route};
use crate::pipeline::OutputSink;
use crate::policy::PolicyProfile;
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

/// 자연어 입력을 게이트웨이로 라우팅하는 AiRouter. S5는 mock(echo) 게이트웨이.
pub struct GatewayAiRouter {
    responder: GatewayResponder,
    profile: PolicyProfile,
}

impl GatewayAiRouter {
    /// mock(echo) 게이트웨이 + config 활성 profile로 구성한다.
    pub fn from_environment() -> anyhow::Result<Self> {
        let responder = GatewayResponder::mock()?;
        let profile = PolicyProfile::by_name(&config::get_active_profile())
            .unwrap_or_else(PolicyProfile::balanced);
        Ok(Self { responder, profile })
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

    #[test]
    fn routes_ai_queries_and_leaves_shell() {
        let mut router = GatewayAiRouter::from_environment().unwrap();
        assert!(router.try_handle("how do I undo a commit?")); // AiQuery → handled
        assert!(router.try_handle("ai explain last-error")); // AiInline → handled
        assert!(!router.try_handle("ls -al")); // Shell → not handled
    }
}
