//! Provider capability map + fallback 정책 (설계 §31.9, M4/W15).
//!
//! provider 차이를 숨기지 않고 capability로 관리한다. 미지원 기능은 명시적 fallback:
//! token counting 미지원 → estimated, usage reporting 미지원 → pricing table,
//! streaming 미지원 → non-streaming. tool use는 MVP 제외.

use crate::usage::{CostSource, TokenSource};

/// 모델 capability(§31.9).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelCapability {
    pub name: String,
    pub max_context_tokens: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub supports_json_mode: bool,
    pub supports_tool_use: bool,
    pub supports_token_counting: bool,
    pub supports_usage_reporting: bool,
    pub supports_context_caching: bool,
}

/// provider 등록 정보.
#[derive(Debug, Clone, PartialEq)]
pub struct Provider {
    pub name: String,
    pub display_name: String,
    pub models: Vec<ModelCapability>,
}

impl Provider {
    /// CI/테스트용 mock provider(외부 호출 없음, 보수적 capability).
    pub fn mock() -> Provider {
        Provider {
            name: "mock".into(),
            display_name: "Mock Provider".into(),
            models: vec![ModelCapability {
                name: "mock-model".into(),
                max_context_tokens: 8192,
                max_output_tokens: 2048,
                supports_streaming: false,
                supports_json_mode: true,
                supports_tool_use: false,
                supports_token_counting: false,
                supports_usage_reporting: false,
                supports_context_caching: false,
            }],
        }
    }
}

/// token counting 미지원 시 estimated로 fallback(§31.9).
pub fn token_source(cap: &ModelCapability) -> TokenSource {
    if cap.supports_token_counting {
        TokenSource::ProviderReported
    } else {
        TokenSource::Estimated
    }
}

/// usage reporting 미지원 시 pricing table로 fallback(§31.9).
pub fn cost_source(cap: &ModelCapability) -> CostSource {
    if cap.supports_usage_reporting {
        CostSource::ProviderReported
    } else {
        CostSource::PricingTable
    }
}

/// streaming 사용 여부(미지원 시 non-streaming).
pub fn use_streaming(cap: &ModelCapability) -> bool {
    cap.supports_streaming
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_has_a_model() {
        let p = Provider::mock();
        assert!(!p.models.is_empty());
        assert_eq!(p.name, "mock");
    }

    #[test]
    fn falls_back_to_estimated_when_unsupported() {
        let cap = &Provider::mock().models[0];
        // mock은 token counting/usage reporting 미지원 → estimated/pricing table
        assert_eq!(token_source(cap), TokenSource::Estimated);
        assert_eq!(cost_source(cap), CostSource::PricingTable);
    }

    #[test]
    fn reports_provider_data_when_supported() {
        let cap = ModelCapability {
            name: "x".into(),
            max_context_tokens: 128000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_json_mode: true,
            supports_tool_use: false,
            supports_token_counting: true,
            supports_usage_reporting: true,
            supports_context_caching: false,
        };
        assert_eq!(token_source(&cap), TokenSource::ProviderReported);
        assert_eq!(cost_source(&cap), CostSource::ProviderReported);
        assert!(use_streaming(&cap));
    }
}
