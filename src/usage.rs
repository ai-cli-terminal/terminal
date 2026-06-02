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
}
