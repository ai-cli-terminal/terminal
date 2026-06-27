//! AI usage accounting shared by CLI and ash.
//!
//! This module keeps token/cost/cache semantics in one place. Storage writes are
//! optional and fail-soft so accounting can never break an AI response path.

use crate::cache::CacheSource;
use crate::config;
use crate::usage;

/// Normalized usage event payload for `usage_events`.
#[derive(Debug, Clone, PartialEq)]
pub struct AiUsageSummary {
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub cost_usd: f64,
    pub estimated: bool,
}

/// Summarize one successful AI answer for storage and display.
pub fn summarize(
    ai: &config::Ai,
    source: CacheSource,
    input_tokens: usize,
    output_tokens: usize,
) -> AiUsageSummary {
    let provider = normalize(&ai.provider, "mock");
    let model = normalize(&ai.model, "default");
    let input_tokens = input_tokens as i64;
    let output_tokens = output_tokens as i64;
    let provider_key = provider.to_ascii_lowercase();

    let cached_tokens = if matches!(source, CacheSource::Backend) {
        0
    } else {
        input_tokens
    };

    let (cost_usd, estimated) = if !matches!(source, CacheSource::Backend) {
        (0.0, false)
    } else if provider_key == "openai" {
        let (cost, _src) = usage::estimate_cost(input_tokens as u64, output_tokens as u64);
        (cost, true)
    } else {
        (0.0, false)
    };

    AiUsageSummary {
        provider,
        model,
        input_tokens,
        output_tokens,
        cached_tokens,
        cost_usd,
        estimated,
    }
}

fn normalize(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Record usage to the default store. Callers intentionally ignore errors.
#[cfg(feature = "storage")]
pub fn record(summary: &AiUsageSummary, session_id: Option<&str>) -> anyhow::Result<()> {
    let store = crate::store::Store::open_default()?;
    record_to_store(&store, summary, session_id)
}

/// Testable storage writer.
#[cfg(feature = "storage")]
pub fn record_to_store(
    store: &crate::store::Store,
    summary: &AiUsageSummary,
    session_id: Option<&str>,
) -> anyhow::Result<()> {
    store.record_usage(
        &summary.provider,
        &summary.model,
        summary.input_tokens,
        summary.output_tokens,
        summary.cached_tokens,
        summary.cost_usd,
        session_id,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ai(provider: &str, model: &str) -> config::Ai {
        config::Ai {
            provider: provider.to_string(),
            model: model.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn summarize_openai_backend_estimates_cost() {
        let out = summarize(&ai("openai", "gpt-test"), CacheSource::Backend, 100, 50);
        assert_eq!(out.provider, "openai");
        assert_eq!(out.model, "gpt-test");
        assert_eq!(out.input_tokens, 100);
        assert_eq!(out.output_tokens, 50);
        assert_eq!(out.cached_tokens, 0);
        assert!(out.cost_usd > 0.0, "{out:?}");
        assert!(out.estimated);
    }

    #[test]
    fn summarize_ollama_backend_costs_zero() {
        let out = summarize(&ai("ollama", "llama3"), CacheSource::Backend, 100, 50);
        assert_eq!(out.provider, "ollama");
        assert_eq!(out.model, "llama3");
        assert_eq!(out.cost_usd, 0.0);
        assert!(!out.estimated);
    }

    #[test]
    fn summarize_cache_hit_costs_zero_and_counts_cached_tokens() {
        let out = summarize(&ai("openai", "gpt-test"), CacheSource::Exact, 100, 50);
        assert_eq!(out.cached_tokens, 100);
        assert_eq!(out.cost_usd, 0.0);
        assert!(!out.estimated);
    }

    #[test]
    fn summarize_empty_model_uses_default() {
        let out = summarize(&ai("", "   "), CacheSource::Backend, 1, 2);
        assert_eq!(out.provider, "mock");
        assert_eq!(out.model, "default");
    }

    #[cfg(feature = "storage")]
    #[test]
    fn record_writes_provider_model_tokens_and_cost() {
        let store = crate::store::Store::open_in_memory().unwrap();
        let summary = AiUsageSummary {
            provider: "openai".into(),
            model: "gpt-test".into(),
            input_tokens: 10,
            output_tokens: 20,
            cached_tokens: 0,
            cost_usd: 0.001,
            estimated: true,
        };
        record_to_store(&store, &summary, None).unwrap();
        let total = store.total_cost(None).unwrap();
        assert!((total - 0.001).abs() < f64::EPSILON, "{total}");
    }
}
