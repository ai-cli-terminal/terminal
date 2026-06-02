//! AI 요청 타임아웃 + 취소 + Graceful Recovery (설계 §13 `[ai.timeout]`, §16.2, W8).
//!
//! 임의의 async AI 작업을 타임아웃/취소와 함께 실행한다. 실패·타임아웃·취소는 모두
//! `Err`로 돌려주어 **AI 장애가 일반 셸 사용을 막지 않는다**(`docs/RULES.md` §1-3).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

/// AI 요청 단계별 타임아웃(§13 `[ai.timeout]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub first_token: Duration,
    pub request: Duration,
    pub long_task: Duration,
}

impl Timeouts {
    /// 기본값: 연결 5s / 첫 토큰 15s / 요청 60s / 장기 작업 180s.
    pub fn defaults() -> Timeouts {
        Timeouts {
            connect: Duration::from_secs(5),
            first_token: Duration::from_secs(15),
            request: Duration::from_secs(60),
            long_task: Duration::from_secs(180),
        }
    }
}

/// AI 요청 실패 사유. 모두 비치명적(셸은 계속 동작).
#[derive(Debug)]
pub enum RequestError {
    TimedOut(Duration),
    Cancelled,
    Failed(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::TimedOut(d) => write!(f, "AI 요청 타임아웃 ({d:?})"),
            RequestError::Cancelled => write!(f, "AI 요청 취소됨"),
            RequestError::Failed(m) => write!(f, "AI 요청 실패: {m}"),
        }
    }
}

impl std::error::Error for RequestError {}

/// 작업을 타임아웃·취소와 함께 실행한다.
///
/// - 작업 완료 → `Ok`/`Err(Failed)`
/// - `timeout` 경과 → `Err(TimedOut)`
/// - `cancel` 알림(예: Ctrl+C) → `Err(Cancelled)`
pub async fn run_cancellable<F, T>(
    fut: F,
    timeout: Duration,
    cancel: Arc<Notify>,
) -> Result<T, RequestError>
where
    F: Future<Output = anyhow::Result<T>>,
{
    tokio::select! {
        res = fut => res.map_err(|e| RequestError::Failed(e.to_string())),
        _ = tokio::time::sleep(timeout) => Err(RequestError::TimedOut(timeout)),
        _ = cancel.notified() => Err(RequestError::Cancelled),
    }
}

/// Ctrl+C(SIGINT) 수신 시 취소 알림을 보내는 백그라운드 태스크를 띄운다.
///
/// AI 요청이 [`run_cancellable`]로 감싸여 있으면 Ctrl+C 시 `Cancelled`로 끝나고
/// 일반 셸은 계속 동작한다(Graceful Recovery, §16.2). tokio 런타임 안에서 호출한다.
pub fn cancel_on_ctrl_c(cancel: Arc<Notify>) {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel.notify_one();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_defaults_match_spec() {
        let t = Timeouts::defaults();
        assert_eq!(t.connect, Duration::from_secs(5));
        assert_eq!(t.first_token, Duration::from_secs(15));
        assert_eq!(t.request, Duration::from_secs(60));
        assert_eq!(t.long_task, Duration::from_secs(180));
    }

    #[tokio::test]
    async fn completes_before_timeout() {
        let cancel = Arc::new(Notify::new());
        let r = run_cancellable(
            async { Ok::<_, anyhow::Error>(42) },
            Duration::from_secs(1),
            cancel,
        )
        .await;
        assert_eq!(r.unwrap(), 42);
    }

    #[tokio::test]
    async fn times_out_when_slow() {
        let cancel = Arc::new(Notify::new());
        let fut = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Ok::<(), anyhow::Error>(())
        };
        let r = run_cancellable(fut, Duration::from_millis(20), cancel).await;
        assert!(matches!(r, Err(RequestError::TimedOut(_))), "{r:?}");
    }

    #[tokio::test]
    async fn cancellation_interrupts() {
        let cancel = Arc::new(Notify::new());
        let c2 = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            c2.notify_one();
        });
        let fut = async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok::<(), anyhow::Error>(())
        };
        let r = run_cancellable(fut, Duration::from_secs(5), cancel).await;
        assert!(matches!(r, Err(RequestError::Cancelled)), "{r:?}");
    }

    #[tokio::test]
    async fn failure_is_reported_not_panicked() {
        let cancel = Arc::new(Notify::new());
        let fut = async { Err::<(), _>(anyhow::anyhow!("boom")) };
        let r = run_cancellable(fut, Duration::from_secs(1), cancel).await;
        assert!(matches!(r, Err(RequestError::Failed(_))), "{r:?}");
    }
}
