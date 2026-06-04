//! Token / Cost 사용량 + 예산 (설계 §31.7, M3/W11).
//!
//! 모든 AI 요청은 usage event로 기록하고, 실제 사용량이 없으면 estimated로 표시한다.
//! 예산(세션 $2 / 월 $30)의 80% 경고, 100% 도달 시 원격 AI 차단.

/// 토큰 수 출처(§31.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    ProviderReported,
    LocalTokenizer,
    Estimated,
    Unknown,
}

/// 비용 출처(§31.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostSource {
    ProviderReported,
    PricingTable,
    Estimated,
    Unknown,
}

/// 예산 설정(§13 `[ai.usage]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetConfig {
    pub session_usd: f64,
    pub monthly_usd: f64,
    pub warn_pct: u8,
    pub block_pct: u8,
}

impl BudgetConfig {
    /// 기본값: 세션 $2 / 월 $30 / 경고 80% / 차단 100%.
    pub fn defaults() -> BudgetConfig {
        BudgetConfig {
            session_usd: 2.0,
            monthly_usd: 30.0,
            warn_pct: 80,
            block_pct: 100,
        }
    }
}

/// 예산 평가 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetAction {
    Ok,
    Warn,
    Block,
}

/// MVP 추정 단가(provider 미보고 시, USD per token). 가정값 — provider 어댑터가
/// 실비용을 보고하면 `CostSource::ProviderReported`로 대체된다(후속).
const EST_INPUT_USD_PER_TOKEN: f64 = 0.000_003;
const EST_OUTPUT_USD_PER_TOKEN: f64 = 0.000_015;

/// 토큰 수로 비용을 추정한다(§31.7). provider가 실비용을 보고하지 않는 MVP 경로에서
/// 쓰며, 항상 [`CostSource::Estimated`]로 표시해 부정확성을 드러낸다.
pub fn estimate_cost(input_tokens: u64, output_tokens: u64) -> (f64, CostSource) {
    let cost = input_tokens as f64 * EST_INPUT_USD_PER_TOKEN
        + output_tokens as f64 * EST_OUTPUT_USD_PER_TOKEN;
    (cost, CostSource::Estimated)
}

/// 지출/한도로 예산 동작을 평가한다(원격 AI 차단 판단).
pub fn evaluate(spent: f64, limit: f64, warn_pct: u8, block_pct: u8) -> BudgetAction {
    if limit <= 0.0 {
        return BudgetAction::Ok;
    }
    let pct = spent / limit * 100.0;
    if pct >= block_pct as f64 {
        BudgetAction::Block
    } else if pct >= warn_pct as f64 {
        BudgetAction::Warn
    } else {
        BudgetAction::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_defaults_match_spec() {
        let b = BudgetConfig::defaults();
        assert_eq!(b.session_usd, 2.0);
        assert_eq!(b.monthly_usd, 30.0);
        assert_eq!(b.warn_pct, 80);
        assert_eq!(b.block_pct, 100);
    }

    #[test]
    fn under_warn_is_ok() {
        assert_eq!(evaluate(1.0, 2.0, 80, 100), BudgetAction::Ok);
    }

    #[test]
    fn at_warn_threshold_warns() {
        assert_eq!(evaluate(1.7, 2.0, 80, 100), BudgetAction::Warn);
    }

    #[test]
    fn at_block_threshold_blocks() {
        assert_eq!(evaluate(2.0, 2.0, 80, 100), BudgetAction::Block);
        assert_eq!(evaluate(2.5, 2.0, 80, 100), BudgetAction::Block);
    }

    #[test]
    fn zero_limit_never_blocks() {
        assert_eq!(evaluate(5.0, 0.0, 80, 100), BudgetAction::Ok);
    }

    #[test]
    fn estimate_cost_is_positive_and_estimated() {
        let (cost, src) = estimate_cost(1000, 500);
        assert!(cost > 0.0, "cost should be positive: {cost}");
        assert_eq!(src, CostSource::Estimated);
    }

    #[test]
    fn estimate_cost_zero_tokens_is_zero() {
        let (cost, src) = estimate_cost(0, 0);
        assert_eq!(cost, 0.0);
        assert_eq!(src, CostSource::Estimated);
    }

    #[test]
    fn estimate_cost_scales_with_tokens() {
        let (small, _) = estimate_cost(100, 100);
        let (big, _) = estimate_cost(1000, 1000);
        assert!(big > small, "more tokens should cost more: {big} > {small}");
    }
}
