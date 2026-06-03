//! 실제 AI 응답기(설계 §5). `dispatch::AiResponder`를 게이트웨이+런타임으로 구현한다.
//!
//! 동기 `block_on`으로 async 게이트웨이를 감싸 디스패처를 sync로 유지한다.
//! 실패·타임아웃·취소는 비치명적([`AiOutcome::Unavailable`])으로 흡수해 셸을 막지 않는다.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::aitask::{RequestError, Timeouts};
use crate::context;
use crate::dispatch::{AiOutcome, AiResponder};
use crate::gateway::{Gateway, GatewayOutcome};
use crate::pipeline::OutputSink;

/// 게이트웨이 기반 AI 응답기.
pub struct GatewayResponder {
    gateway: Gateway,
    runtime: tokio::runtime::Runtime,
    timeout: Duration,
}

impl GatewayResponder {
    /// 주어진 게이트웨이·타임아웃으로 만든다(current-thread 런타임 1개 보유).
    pub fn new(gateway: Gateway, timeout: Duration) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            gateway,
            runtime,
            timeout,
        })
    }

    /// mock(echo) 게이트웨이 + 기본 요청 타임아웃.
    pub fn mock() -> anyhow::Result<Self> {
        Self::new(Gateway::mock(), Timeouts::defaults().request)
    }
}

/// 게이트웨이 결과를 [`AiOutcome`]으로 매핑한다(성공 시 text를 sink에 기록).
fn finish(result: Result<GatewayOutcome, RequestError>, sink: &mut dyn OutputSink) -> AiOutcome {
    match result {
        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
        }) => {
            sink.write(&text);
            AiOutcome::Answered {
                text,
                input_tokens,
                output_tokens,
            }
        }
        Ok(GatewayOutcome::Blocked(reason)) => AiOutcome::Blocked(reason),
        Err(e) => AiOutcome::Unavailable(e.to_string()),
    }
}

impl AiResponder for GatewayResponder {
    fn respond(&mut self, prompt: &str, sink: &mut dyn OutputSink) -> anyhow::Result<AiOutcome> {
        let ctx = context::gather();
        let ctx_str = format!("cwd={}", ctx.cwd);
        let timeout = self.timeout;
        let gw = &self.gateway;
        // Ctrl+C는 select 분기로 처리한다(백그라운드 spawn 누수 방지). raw-mode(TUI)에선
        // SIGINT가 KeyEvent로 잡혀 이 분기가 발동하지 않으므로 타임아웃이 상한 역할을 한다.
        let result = self.runtime.block_on(async {
            let cancel = Arc::new(Notify::new());
            tokio::select! {
                r = gw.ask_cancellable(prompt, &ctx_str, timeout, cancel) => r,
                _ = tokio::signal::ctrl_c() => Err(RequestError::Cancelled),
            }
        });
        Ok(finish(result, sink))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sink(String);
    impl OutputSink for Sink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    #[test]
    fn finish_answered_writes_to_sink() {
        let mut sink = Sink(String::new());
        let out = finish(
            Ok(GatewayOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4,
            }),
            &mut sink,
        );
        assert_eq!(
            out,
            AiOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4
            }
        );
        assert_eq!(sink.0, "hello");
    }

    #[test]
    fn finish_blocked_maps_and_no_write() {
        let mut sink = Sink(String::new());
        let out = finish(Ok(GatewayOutcome::Blocked("masking".into())), &mut sink);
        assert_eq!(out, AiOutcome::Blocked("masking".into()));
        assert_eq!(sink.0, "");
    }

    #[test]
    fn finish_error_maps_to_unavailable() {
        let mut sink = Sink(String::new());
        let out = finish(Err(RequestError::Cancelled), &mut sink);
        assert!(matches!(out, AiOutcome::Unavailable(_)), "{out:?}");
        assert_eq!(sink.0, "");
    }

    #[test]
    fn mock_responder_answers_via_echo() {
        let mut r = GatewayResponder::mock().unwrap();
        let mut sink = Sink(String::new());
        let out = r.respond("ping", &mut sink).unwrap();
        // EchoBackend는 입력을 그대로 돌려주므로 Answered + sink에 내용이 있어야 한다.
        assert!(matches!(out, AiOutcome::Answered { .. }), "{out:?}");
        assert!(!sink.0.is_empty(), "echo answer should be written to sink");
    }
}
