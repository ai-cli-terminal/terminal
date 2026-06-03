//! AI Model Gateway — AI 요청 파이프라인 (설계 §5 AI Service Layer, Phase 2).
//!
//! 경로: prompt+context → **마스킹**(secret 차단/치환) → 토큰 윈도 → 백엔드 생성 → usage.
//! 마스킹 실패(예: private key) 시 원격 전송을 차단한다(fail-closed, `docs/RULES.md` §2).
//! 실제 provider(HTTP/Ollama)는 [`LlmBackend`] 구현으로 주입한다. MVP는 mock으로 검증.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Notify;

use crate::aitask::{run_cancellable, RequestError};
use crate::cache::ResponseCache;
use crate::mask::Masker;
use crate::provider::ModelCapability;
use crate::tokenwin;

/// LLM 백엔드 인터페이스. 실제 provider/로컬 LLM이 구현한다.
///
/// `Send + Sync`를 요구해 게이트웨이를 워커 스레드(`spawn_blocking`)로 옮겨
/// 타임아웃/취소와 함께 실행할 수 있게 한다(§16.2).
pub trait LlmBackend: Send + Sync {
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
    cache: Mutex<ResponseCache>,
}

impl Gateway {
    /// 주어진 백엔드·capability로 게이트웨이를 만든다(캐시 TTL 24h).
    pub fn new(backend: Box<dyn LlmBackend>, cap: ModelCapability) -> Gateway {
        Gateway {
            backend,
            cap,
            masker: Masker::baseline(),
            cache: Mutex::new(ResponseCache::new(86_400)),
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

        // 3) 캐시 조회(§29.6) — 히트 시 백엔드 호출 생략.
        //    값을 복제해 락을 즉시 해제한다(백엔드 호출 중 락 보유 금지).
        let key = ResponseCache::key(&sent);
        let now = now_ms();
        let cached = self
            .cache
            .lock()
            .expect("cache mutex poisoned")
            .get(&key, now)
            .map(|s| s.to_string());
        if let Some(text) = cached {
            let output_tokens = tokenwin::estimate_tokens(&text);
            return Ok(GatewayOutcome::Answered {
                text,
                input_tokens,
                output_tokens,
            });
        }

        // 4) 백엔드 생성 + 캐시 저장
        let text = self.backend.generate(&sent)?;
        self.cache
            .lock()
            .expect("cache mutex poisoned")
            .put(key, text.clone(), now);
        let output_tokens = tokenwin::estimate_tokens(&text);

        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
        })
    }

    /// [`ask`](Self::ask)를 타임아웃·취소와 함께 워커 스레드에서 실행한다(§16.2).
    ///
    /// 동기 백엔드 호출을 `spawn_blocking`으로 옮겨, 느린 응답/Ctrl+C에도 호출자가
    /// 즉시 제어를 되찾는다. 실패·타임아웃·취소는 모두 [`RequestError`]로 돌아가
    /// 일반 셸 사용을 막지 않는다. (타임아웃된 동기 호출 자체는 백그라운드에서 종료된다.)
    pub async fn ask_cancellable(
        self: Arc<Self>,
        prompt: String,
        context: String,
        timeout: Duration,
        cancel: Arc<Notify>,
    ) -> Result<GatewayOutcome, RequestError> {
        let gw = self;
        run_cancellable(
            async move {
                tokio::task::spawn_blocking(move || gw.ask(&prompt, &context))
                    .await
                    .map_err(|e| anyhow::anyhow!("백엔드 워커 조인 실패: {e}"))?
            },
            timeout,
            cancel,
        )
        .await
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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
    fn repeated_prompt_hits_cache() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counting(Arc<AtomicUsize>);
        impl LlmBackend for Counting {
            fn generate(&self, prompt: &str) -> Result<String> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(format!("r:{prompt}"))
            }
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Gateway::new(Box::new(Counting(calls.clone())), cap);
        let _ = gw.ask("same prompt", "").unwrap();
        let _ = gw.ask("same prompt", "").unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "backend should be called once (cached)"
        );
    }

    /// 느린 동기 백엔드는 `ask_cancellable`의 타임아웃에 걸려 호출자가 제어를 되찾는다.
    #[tokio::test]
    async fn slow_backend_times_out() {
        struct Slow;
        impl LlmBackend for Slow {
            fn generate(&self, _prompt: &str) -> Result<String> {
                std::thread::sleep(Duration::from_millis(300));
                Ok("late".into())
            }
        }
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Arc::new(Gateway::new(Box::new(Slow), cap));
        let cancel = Arc::new(Notify::new());
        let r = gw
            .ask_cancellable("q".into(), String::new(), Duration::from_millis(20), cancel)
            .await;
        assert!(matches!(r, Err(RequestError::TimedOut(_))), "{r:?}");
    }

    /// 정상 응답은 `ask_cancellable`로도 그대로 통과한다.
    #[tokio::test]
    async fn fast_backend_answers_through_cancellable() {
        let gw = Arc::new(Gateway::mock());
        let cancel = Arc::new(Notify::new());
        let r = gw
            .ask_cancellable(
                "hello".into(),
                String::new(),
                Duration::from_secs(5),
                cancel,
            )
            .await
            .expect("정상 응답");
        match r {
            GatewayOutcome::Answered { text, .. } => assert!(text.contains("hello"), "{text}"),
            GatewayOutcome::Blocked(reason) => panic!("unexpected block: {reason}"),
        }
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
