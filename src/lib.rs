//! AI Terminal 라이브러리 크레이트.
//!
//! 5계층 아키텍처(설계 §5) / 7개 도메인(계획서 §1.4)을 모듈로 구성한다.
//! MVP는 보안 핵심부터 채운다: 위험도 엔진(§31.4) → 정책(§31.3) → 마스킹(§31.8).
//!
//! 아래 `pub mod` 선언은 **도메인 클러스터**로 묶여 있다(Wave 2 정리, move-only —
//! 모듈 이름·경로·`#[cfg]`는 불변, 재배열 + 섹션 주석만). 대부분 모듈은 데스크톱 전용
//! (`cfg(not(target_os = "android"))`)이며 android는 `shellcore`/`mobile*`과 AI 보조
//! 스택(intent·dispatch 분류·gateway·openai + 순수 의존: risk·policy·cache·mask·
//! provider·tokenwin·usage·aitask, 실 I/O transport 제외)을 컴파일한다.

// === 보안 코어 (위험도·정책·마스킹·프리뷰·undo·가드레일) ===
#[cfg(not(target_os = "android"))]
pub mod diff;
#[cfg(not(target_os = "android"))]
pub mod guardrails;
pub mod mask;
pub mod policy;
#[cfg(not(target_os = "android"))]
pub mod preview;
pub mod risk;
#[cfg(not(target_os = "android"))]
pub mod sandbox;
#[cfg(not(target_os = "android"))]
pub mod undo;

// === AI / 게이트웨이 (의도 분류·라우팅·실행 파이프라인·provider·검증) ===
#[cfg(not(target_os = "android"))]
pub mod ai_router;
pub mod aitask;
pub mod cache;
pub mod dispatch;
pub mod gateway;
pub mod http;
pub mod intent;
#[cfg(not(target_os = "android"))]
pub mod ollama;
pub mod openai;
#[cfg(not(target_os = "android"))]
pub mod pipeline;
#[cfg(not(target_os = "android"))]
pub mod planner;
pub mod provider;
#[cfg(not(target_os = "android"))]
pub mod responder;
pub mod tokenwin;
#[cfg(not(target_os = "android"))]
pub mod verify;
#[cfg(not(target_os = "android"))]
pub mod verify_agent;

// === 컨텍스트 / 인덱스 / 설명 ===
#[cfg(not(target_os = "android"))]
pub mod cmdparse;
#[cfg(not(target_os = "android"))]
pub mod context;
#[cfg(not(target_os = "android"))]
pub mod explain;
#[cfg(not(target_os = "android"))]
pub mod index;

// === 셸 (독립 구조화 셸 shellcore + 호스트 어댑터·wrapper·라인에디터·게이트 러너) ===
#[cfg(not(target_os = "android"))]
pub mod gated_runner;
#[cfg(not(target_os = "android"))]
pub mod line_editor;
#[cfg(not(target_os = "android"))]
pub mod pty;
#[cfg(not(target_os = "android"))]
pub mod shell;
#[cfg(not(target_os = "android"))]
pub mod shell_audit;
pub mod shellcore;
#[cfg(not(target_os = "android"))]
pub mod wrapper;

// === 원격 승인(RA) — 게이트·데몬·Noise 전송·페어링·디바이스 레지스트리 (대부분 `remote` feature) ===
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod approval;
#[cfg(all(unix, not(target_os = "android")))]
pub mod daemon;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod device_registry;
#[cfg(not(target_os = "android"))]
pub mod gate;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod pairing;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod qr;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod remote;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod remote_transport;
#[cfg(feature = "remote")]
#[cfg(not(target_os = "android"))]
pub mod session;

// === 저장 / 사용량 (store는 `storage` feature) ===
#[cfg(not(target_os = "android"))]
pub mod ai_usage;
#[cfg(not(target_os = "android"))]
pub mod lock;
#[cfg(feature = "storage")]
#[cfg(not(target_os = "android"))]
pub mod store;
pub mod usage;

// === skill / MCP ===
#[cfg(not(target_os = "android"))]
pub mod mcp;
#[cfg(not(target_os = "android"))]
pub mod skill;

// === 신뢰 채널 (P3 trust — signed manifest·policy.d·skill registry·binary manifest, `trust` feature) ===
#[cfg(feature = "trust")]
#[cfg(not(target_os = "android"))]
pub mod binary_manifest;
#[cfg(feature = "trust")]
#[cfg(not(target_os = "android"))]
pub mod policy_d;
#[cfg(feature = "trust")]
#[cfg(not(target_os = "android"))]
pub mod skill_registry;
#[cfg(feature = "trust")]
#[cfg(not(target_os = "android"))]
pub mod trust;

// === 모바일 (Android/iOS 공통 bridge + AI 보조 + Android JNI) ===
pub mod mobile;
pub mod mobile_ai;
pub mod mobile_ffi;
#[cfg(target_os = "android")]
pub mod mobile_jni;

// === config / UI ===
#[cfg(not(target_os = "android"))]
pub mod config;
#[cfg(not(target_os = "android"))]
pub mod ui;
