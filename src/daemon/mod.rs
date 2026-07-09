//! 원격 승인 로컬 게이트 데몬 (M1 slice 1, unix 전용).
//!
//! 셸 hook(`ai __gate`)이 Unix 소켓으로 **블로킹 질의** → 데몬이 게이트 결정
//! (`gate::decide_gate` shared-core, §30-13) 회신. hook↔데몬은 **같은 머신의 신뢰된
//! 로컬 IPC**다(phone↔데몬의 Noise E2E[M0.5]와 다른 채널). 컨텍스트 스냅샷·폰 왕복·
//! nonce 소비·페어링은 M1 후속 슬라이스에서 데몬에 결합한다.
//!
//! 프레이밍: 개행 구분 JSON(연결당 1요청/1회신).

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::gate::{self, GateDecision};

mod companion_live;
mod device_listener;
mod relay_client;
mod remote_gate;
mod serve;
#[cfg(test)]
mod tests;

#[cfg(feature = "remote")]
pub use companion_live::{spawn_companion_live_endpoint, CompanionLiveEndpointHandle};
#[cfg(feature = "remote")]
pub use device_listener::{
    serve_device_loop, serve_device_once, spawn_device_listener, DeviceListenerHandle,
    DeviceListenerRequest,
};
#[cfg(feature = "remote")]
pub use relay_client::CompanionRelayDaemonRuntime;
#[cfg(feature = "remote")]
pub use remote_gate::{
    decide_with_remote_listener, decide_with_remote_relay_bridge, finish_remote_gate_response,
    plan_remote_gate, remote_timeout_reply, RemoteApprovalPlan, RemoteGatePlanInput, RemoteGateRun,
    RemoteGateStep,
};
pub use serve::{query, query_with_context, serve};
#[cfg(feature = "remote")]
pub use serve::{serve_with_remote, serve_with_remote_relay};

/// 게이트 질의 요청.
#[derive(Serialize, Deserialize, Debug)]
pub struct GateRequest {
    pub command: String,
    #[serde(default)]
    pub context_origin: Option<crate::context::RemoteContextOrigin>,
}

/// 게이트 결정 회신.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GateReply {
    pub decision: String, // "allow" | "block"
    pub reason: String,
}

impl GateReply {
    pub fn allow() -> Self {
        Self {
            decision: "allow".into(),
            reason: String::new(),
        }
    }
    pub fn block(reason: String) -> Self {
        Self {
            decision: "block".into(),
            reason,
        }
    }
    pub fn is_allow(&self) -> bool {
        self.decision == "allow"
    }
}

/// 게이트 소켓 경로: `<config_dir>/gate.sock`.
pub fn socket_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("gate.sock"))
}

/// 디바이스 연결 소켓 경로: `<config_dir>/device.sock`.
/// RA-1 device listener가 사용하는 daemon-owned endpoint다.
#[cfg(feature = "remote")]
pub fn device_socket_path() -> Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("device.sock"))
}

/// armed/allow_high가 주어졌을 때의 게이트 회신(순수, shared-core).
pub fn decide_with(command: &str, armed: bool, allow_high: bool) -> GateReply {
    match gate::decide_gate(command, armed, allow_high) {
        GateDecision::Allow => GateReply::allow(),
        GateDecision::Block { reason } => GateReply::block(reason),
    }
}

/// 현재 armed 상태를 읽어 게이트 회신을 만든다(데몬 핸들러용).
pub fn decide_request(command: &str) -> GateReply {
    let (armed, allow_high) = gate::armed_path()
        .ok()
        .and_then(|p| gate::load_arm_state(&p))
        .map(|s| (true, s.allow_high))
        .unwrap_or((false, false));
    decide_with(command, armed, allow_high)
}
