//! AI Terminal 라이브러리 크레이트.
//!
//! 5계층 아키텍처(설계 §5) / 7개 도메인(계획서 §1.4)을 모듈로 구성한다.
//! MVP는 보안 핵심부터 채운다: 위험도 엔진(§31.4) → 정책(§31.3) → 마스킹(§31.8).
//!
//! 현재 구현된 모듈:
//! - [`risk`] — rule-based 위험도 스코어링 (0~100, deterministic).
//! - [`policy`] — 정책 프로파일(balanced/paranoid) + 위험 등급별 결정.
//! - [`pty`] — PTY 기반 셸 실행(일반 셸 경로 토대, M1/W2).

pub mod aitask;
pub mod config;
pub mod lock;
pub mod mask;
pub mod policy;
pub mod pty;
pub mod risk;
pub mod shell;
#[cfg(feature = "storage")]
pub mod store;
pub mod ui;
pub mod verify;
