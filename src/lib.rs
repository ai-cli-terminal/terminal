//! AI Terminal 라이브러리 크레이트.
//!
//! 5계층 아키텍처(설계 §5) / 7개 도메인(계획서 §1.4)을 모듈로 구성한다.
//! MVP는 보안 핵심부터 채운다: 위험도 엔진(§31.4) → 정책(§31.3) → 마스킹(§31.8).
//!
//! 현재 구현된 모듈:
//! - [`risk`] — rule-based 위험도 스코어링 (0~100, deterministic).
//! - [`policy`] — 정책 프로파일(balanced/paranoid) + 위험 등급별 결정.
//! - [`pty`] — PTY 기반 셸 실행(일반 셸 경로 토대, M1/W2).

#[cfg(not(target_os = "android"))]
pub mod aitask;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod approval;
#[cfg(not(target_os = "android"))]
pub mod cache;
#[cfg(not(target_os = "android"))]
pub mod cmdparse;
#[cfg(not(target_os = "android"))]
pub mod config;
#[cfg(not(target_os = "android"))]
pub mod context;
#[cfg(all(unix, not(target_os = "android")))]
pub mod daemon;
#[cfg(not(target_os = "android"))]
pub mod diff;
#[cfg(not(target_os = "android"))]
pub mod dispatch;
#[cfg(not(target_os = "android"))]
pub mod explain;
#[cfg(not(target_os = "android"))]
pub mod gate;
#[cfg(not(target_os = "android"))]
pub mod gated_runner;
#[cfg(not(target_os = "android"))]
pub mod gateway;
#[cfg(not(target_os = "android"))]
pub mod guardrails;
#[cfg(not(target_os = "android"))]
pub mod http;
#[cfg(not(target_os = "android"))]
pub mod index;
#[cfg(not(target_os = "android"))]
pub mod intent;
#[cfg(not(target_os = "android"))]
pub mod line_editor;
#[cfg(not(target_os = "android"))]
pub mod lock;
#[cfg(not(target_os = "android"))]
pub mod mask;
#[cfg(not(target_os = "android"))]
pub mod mcp;
pub mod mobile;
pub mod mobile_jni;
#[cfg(not(target_os = "android"))]
pub mod ollama;
#[cfg(not(target_os = "android"))]
pub mod openai;
#[cfg(not(target_os = "android"))]
pub mod pipeline;
#[cfg(not(target_os = "android"))]
pub mod planner;
#[cfg(not(target_os = "android"))]
pub mod policy;
#[cfg(not(target_os = "android"))]
pub mod preview;
#[cfg(not(target_os = "android"))]
pub mod provider;
#[cfg(not(target_os = "android"))]
pub mod pty;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod remote;
#[cfg(not(target_os = "android"))]
pub mod responder;
#[cfg(not(target_os = "android"))]
pub mod risk;
#[cfg(not(target_os = "android"))]
pub mod sandbox;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod session;
#[cfg(not(target_os = "android"))]
pub mod shell;
pub mod shellcore;
#[cfg(not(target_os = "android"))]
pub mod skill;
#[cfg(feature = "storage")]
#[cfg(not(target_os = "android"))]
pub mod store;
#[cfg(not(target_os = "android"))]
pub mod tokenwin;
#[cfg(not(target_os = "android"))]
pub mod ui;
#[cfg(not(target_os = "android"))]
pub mod undo;
#[cfg(not(target_os = "android"))]
pub mod usage;
#[cfg(not(target_os = "android"))]
pub mod verify;
#[cfg(not(target_os = "android"))]
pub mod verify_agent;
#[cfg(not(target_os = "android"))]
pub mod wrapper;
