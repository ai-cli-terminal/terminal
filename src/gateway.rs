//! AI Model Gateway — AI 요청 파이프라인 (설계 §5 AI Service Layer, Phase 2).
//!
//! 경로: prompt+context → **마스킹**(secret 차단/치환) → 토큰 윈도 → 백엔드 생성 → usage.
//! 마스킹 실패(예: private key) 시 원격 전송을 차단한다(fail-closed, `docs/RULES.md` §2).
//! 실제 provider(HTTP/Ollama)는 [`LlmBackend`] 구현으로 주입한다. MVP는 mock으로 검증.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Notify;

use crate::aitask::{run_cancellable, RequestError};
use crate::cache::{CacheSource, ResponseCache, SemanticCache};
use crate::mask::Masker;
use crate::provider::ModelCapability;
use crate::tokenwin;

/// 백엔드가 돌려주는 비동기 응답(dyn 호환을 위해 박싱한다).
pub type GenerateFuture<'a> = Pin<Box<dyn Future<Output = Result<String>> + 'a>>;

/// LLM 백엔드 인터페이스. 실제 provider/로컬 LLM이 구현한다.
///
/// 비동기 트레이트(dyn 디스패치를 위해 박싱된 future 반환). future는 current-thread
/// 런타임에서 `block_on`으로 구동되므로 `Send`를 요구하지 않는다.
pub trait LlmBackend {
    /// (이미 마스킹된) 프롬프트로 응답을 생성한다.
    fn generate<'a>(&'a self, prompt: &'a str) -> GenerateFuture<'a>;
}

/// 테스트/CI용 echo 백엔드 — 입력을 그대로 되돌려 마스킹 적용을 검증 가능하게 한다.
pub struct EchoBackend;

impl LlmBackend for EchoBackend {
    fn generate<'a>(&'a self, prompt: &'a str) -> GenerateFuture<'a> {
        Box::pin(async move { Ok(format!("echo: {prompt}")) })
    }
}

/// 게이트웨이 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayOutcome {
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
        source: CacheSource,
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
    semantic: Mutex<SemanticCache>,
}

impl Gateway {
    /// 주어진 백엔드·capability로 게이트웨이를 만든다(캐시 TTL 24h).
    pub fn new(backend: Box<dyn LlmBackend>, cap: ModelCapability) -> Gateway {
        Gateway {
            backend,
            cap,
            masker: Masker::baseline(),
            cache: Mutex::new(ResponseCache::new(86_400)),
            semantic: Mutex::new(SemanticCache::new(86_400, 0.85)),
        }
    }

    /// mock(echo) 백엔드 게이트웨이.
    pub fn mock() -> Gateway {
        let cap = crate::provider::Provider::mock().models[0].clone();
        Gateway::new(Box::new(EchoBackend), cap)
    }

    /// prompt+context를 마스킹·토큰 점검 후 백엔드로 보내 응답을 만든다.
    pub async fn ask(&self, prompt: &str, context: &str) -> Result<GatewayOutcome> {
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

        // 3) exact 캐시 조회(§29.6) — 히트 시 백엔드 생략. 값 복제로 락 즉시 해제.
        let key = ResponseCache::key(&sent);
        let now = now_ms();
        let cached = self
            .cache
            .lock()
            .expect("cache mutex poisoned")
            .get(&key, now)
            .map(|s| s.to_string());
        if let Some(text) = cached {
            return Ok(Self::answered(text, input_tokens, CacheSource::Exact));
        }

        // 3b) 시맨틱 2차 조회 — 임계값 이상 유사 시 히트, 그 답을 exact 캐시에 승격 저장.
        let similar = self
            .semantic
            .lock()
            .expect("semantic cache mutex poisoned")
            .get_similar(&sent, now)
            .map(|s| s.to_string());
        if let Some(text) = similar {
            self.cache
                .lock()
                .expect("cache mutex poisoned")
                .put(key, text.clone(), now);
            return Ok(Self::answered(text, input_tokens, CacheSource::Semantic));
        }

        // 4) 백엔드 생성 + exact·semantic 양쪽 저장. await 후 시각으로 TTL 기록(지연 보정).
        let text = self.backend.generate(&sent).await?;
        let stored_at = now_ms();
        self.cache
            .lock()
            .expect("cache mutex poisoned")
            .put(key, text.clone(), stored_at);
        self.semantic
            .lock()
            .expect("semantic cache mutex poisoned")
            .put(sent, text.clone(), stored_at);
        Ok(Self::answered(text, input_tokens, CacheSource::Backend))
    }

    /// 응답 텍스트로 `Answered`를 구성한다(출력 토큰 추정 공통화).
    fn answered(text: String, input_tokens: usize, source: CacheSource) -> GatewayOutcome {
        let output_tokens = tokenwin::estimate_tokens(&text);
        GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
            source,
        }
    }

    /// [`ask`](Self::ask)를 타임아웃·취소와 함께 실행한다(§16.2, Graceful Recovery).
    ///
    /// 진짜 async I/O이므로 타임아웃/취소 시 `ask` future가 drop되며 진행 중인 연결도
    /// 함께 취소된다(고아 호출 없음). 실패·타임아웃·취소는 모두 [`RequestError`]로
    /// 돌아가 일반 셸 사용을 막지 않는다.
    pub async fn ask_cancellable(
        &self,
        prompt: &str,
        context: &str,
        timeout: Duration,
        cancel: Arc<Notify>,
    ) -> Result<GatewayOutcome, RequestError> {
        run_cancellable(self.ask(prompt, context), timeout, cancel).await
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
    use crate::cache::CacheSource;

    #[tokio::test]
    async fn answers_with_backend() {
        let gw = Gateway::mock();
        let out = gw.ask("hello", "").await.unwrap();
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

    #[tokio::test]
    async fn masks_secret_before_backend() {
        let gw = Gateway::mock();
        let out = gw.ask("my key AKIAIOSFODNN7EXAMPLE", "").await.unwrap();
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

    #[tokio::test]
    async fn blocks_on_private_key() {
        let gw = Gateway::mock();
        let out = gw
            .ask("-----BEGIN OPENSSH PRIVATE KEY-----", "")
            .await
            .unwrap();
        assert!(matches!(out, GatewayOutcome::Blocked(_)), "{out:?}");
    }

    #[tokio::test]
    async fn repeated_prompt_hits_cache() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counting(Arc<AtomicUsize>);
        impl LlmBackend for Counting {
            fn generate<'a>(&'a self, prompt: &'a str) -> GenerateFuture<'a> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { Ok(format!("r:{prompt}")) })
            }
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Gateway::new(Box::new(Counting(calls.clone())), cap);
        let _ = gw.ask("same prompt", "").await.unwrap();
        let _ = gw.ask("same prompt", "").await.unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "backend should be called once (cached)"
        );
    }

    /// 느린 비동기 백엔드는 `ask_cancellable`의 타임아웃에 걸려 호출자가 제어를 되찾는다.
    #[tokio::test]
    async fn slow_backend_times_out() {
        struct Slow;
        impl LlmBackend for Slow {
            fn generate<'a>(&'a self, _prompt: &'a str) -> GenerateFuture<'a> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    Ok("late".into())
                })
            }
        }
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Gateway::new(Box::new(Slow), cap);
        let cancel = Arc::new(Notify::new());
        let r = gw
            .ask_cancellable("q", "", Duration::from_millis(20), cancel)
            .await;
        assert!(matches!(r, Err(RequestError::TimedOut(_))), "{r:?}");
    }

    /// 정상 응답은 `ask_cancellable`로도 그대로 통과한다.
    #[tokio::test]
    async fn fast_backend_answers_through_cancellable() {
        let gw = Gateway::mock();
        let cancel = Arc::new(Notify::new());
        let r = gw
            .ask_cancellable("hello", "", Duration::from_secs(5), cancel)
            .await
            .expect("정상 응답");
        match r {
            GatewayOutcome::Answered { text, .. } => assert!(text.contains("hello"), "{text}"),
            GatewayOutcome::Blocked(reason) => panic!("unexpected block: {reason}"),
        }
    }

    #[tokio::test]
    async fn context_is_included_in_prompt() {
        let gw = Gateway::mock();
        let out = gw.ask("question", "cwd=/srv/app").await.unwrap();
        match out {
            GatewayOutcome::Answered { text, .. } => assert!(text.contains("cwd=/srv/app")),
            GatewayOutcome::Blocked(r) => panic!("unexpected block: {r}"),
        }
    }

    #[tokio::test]
    async fn semantic_hit_then_promotes_to_exact() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Counting(Arc<AtomicUsize>);
        impl LlmBackend for Counting {
            fn generate<'a>(&'a self, prompt: &'a str) -> GenerateFuture<'a> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { Ok(format!("r:{prompt}")) })
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let cap = crate::provider::Provider::mock().models[0].clone();
        let gw = Gateway::new(Box::new(Counting(calls.clone())), cap);

        let o1 = gw.ask("alpha beta gamma", "").await.unwrap();
        assert!(
            matches!(
                o1,
                GatewayOutcome::Answered {
                    source: CacheSource::Backend,
                    ..
                }
            ),
            "{o1:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let o2 = gw.ask("gamma alpha beta", "").await.unwrap();
        assert!(
            matches!(
                o2,
                GatewayOutcome::Answered {
                    source: CacheSource::Semantic,
                    ..
                }
            ),
            "{o2:?}"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "semantic hit must not call backend"
        );

        let o3 = gw.ask("gamma alpha beta", "").await.unwrap();
        assert!(
            matches!(
                o3,
                GatewayOutcome::Answered {
                    source: CacheSource::Exact,
                    ..
                }
            ),
            "{o3:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn source_is_backend_then_exact_on_repeat() {
        let gw = Gateway::mock();
        let a = gw.ask("repeat me", "").await.unwrap();
        assert!(
            matches!(
                a,
                GatewayOutcome::Answered {
                    source: CacheSource::Backend,
                    ..
                }
            ),
            "{a:?}"
        );
        let b = gw.ask("repeat me", "").await.unwrap();
        assert!(
            matches!(
                b,
                GatewayOutcome::Answered {
                    source: CacheSource::Exact,
                    ..
                }
            ),
            "{b:?}"
        );
    }
}
