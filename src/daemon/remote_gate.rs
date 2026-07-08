#[cfg(feature = "remote")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "remote")]
use anyhow::{Context, Result};

use crate::gate;

use super::decide_with;
#[cfg(feature = "remote")]
use super::device_listener::{DeviceListenerHandle, DeviceListenerRequest};
#[cfg(feature = "remote")]
use super::relay_client::{CompanionRelayDaemonRuntime, RemoteDaemonBridge};
use super::{GateReply, GateRequest};

#[derive(Clone)]
pub(super) struct DaemonRuntime {
    #[cfg(feature = "remote")]
    remote: Option<RemoteDaemonState>,
}

#[cfg(feature = "remote")]
#[derive(Clone)]
pub(super) struct RemoteDaemonState {
    pub(super) registry: crate::device_registry::DeviceRegistry,
    pub(super) device_id: Option<String>,
    pub(super) bridge: RemoteDaemonBridge,
    pub(super) approval_ttl: u64,
    pub(super) response_timeout: std::time::Duration,
}

impl DaemonRuntime {
    pub(super) fn local() -> Self {
        Self {
            #[cfg(feature = "remote")]
            remote: None,
        }
    }

    #[cfg(feature = "remote")]
    pub(super) fn remote(
        registry: crate::device_registry::DeviceRegistry,
        listener: DeviceListenerHandle,
        device_id: Option<String>,
    ) -> Self {
        Self {
            remote: Some(RemoteDaemonState {
                registry,
                device_id,
                bridge: RemoteDaemonBridge::LiveListener(Arc::new(Mutex::new(listener))),
                approval_ttl: 60,
                response_timeout: std::time::Duration::from_secs(30),
            }),
        }
    }

    #[cfg(feature = "remote")]
    pub(super) fn remote_relay(
        registry: crate::device_registry::DeviceRegistry,
        relay: CompanionRelayDaemonRuntime,
        device_id: Option<String>,
    ) -> Self {
        Self {
            remote: Some(RemoteDaemonState {
                registry,
                device_id,
                bridge: RemoteDaemonBridge::Relay(Arc::new(Mutex::new(Box::new(relay)))),
                approval_ttl: 60,
                response_timeout: std::time::Duration::from_secs(30),
            }),
        }
    }

    pub(super) fn decide(&self, req: &GateRequest) -> GateReply {
        let (armed, allow_high) = gate::armed_path()
            .ok()
            .and_then(|p| gate::load_arm_state(&p))
            .map(|s| (true, s.allow_high))
            .unwrap_or((false, false));

        #[cfg(feature = "remote")]
        if let Some(remote) = &self.remote {
            return remote.decide_with_arm(req, armed, allow_high);
        }

        decide_with(&req.command, armed, allow_high)
    }
}

#[cfg(feature = "remote")]
impl RemoteDaemonState {
    pub(super) fn decide_with_arm(
        &self,
        req: &GateRequest,
        armed: bool,
        allow_high: bool,
    ) -> GateReply {
        let origin = req
            .context_origin
            .clone()
            .unwrap_or_else(crate::context::RemoteContextOrigin::gather);
        let issued_context_hash =
            crate::context::remote_context_hash_for_origin(&req.command, &origin);
        let run = RemoteGateRun {
            command: &req.command,
            armed,
            allow_high,
            now: now_secs(),
            ttl: self.approval_ttl,
            device_id: self.device_id.as_deref(),
            issued_context_hash: &issued_context_hash,
            context_origin: Some(&origin),
            current_context_hash: None,
            response_timeout: self.response_timeout,
        };

        match &self.bridge {
            RemoteDaemonBridge::LiveListener(listener) => {
                let Ok(listener) = listener.lock() else {
                    return GateReply::block("원격 디바이스 리스너 lock 실패".into());
                };
                decide_with_remote_listener(&self.registry, &listener, run)
            }
            RemoteDaemonBridge::Relay(relay) => {
                let Ok(mut relay) = relay.lock() else {
                    return GateReply::block("원격 relay runtime lock 실패".into());
                };
                decide_with_remote_relay_bridge(&self.registry, run, |plan, request| {
                    relay.roundtrip(plan, request, self.response_timeout)
                })
            }
        }
    }
}

#[cfg(feature = "remote")]
pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// RA-3 gate-flow가 로컬 결정으로 끝나는지, 등록 디바이스 승인 왕복이 필요한지.
#[cfg(feature = "remote")]
#[derive(Debug)]
pub enum RemoteGateStep {
    Local(GateReply),
    NeedsRemote(RemoteApprovalPlan),
}

/// 등록 디바이스로 보낼 pending approval 요청.
#[cfg(feature = "remote")]
#[derive(Debug)]
pub struct RemoteApprovalPlan {
    pub device_id: String,
    pub pending: crate::approval::PendingApproval,
    pub request: crate::session::ApprovalRequestMsg,
}

#[cfg(feature = "remote")]
pub struct RemoteGatePlanInput<'a> {
    pub command: &'a str,
    pub armed: bool,
    pub allow_high: bool,
    pub device_id: Option<&'a str>,
    pub now: u64,
    pub ttl: u64,
    pub context_hash: &'a str,
}

/// RA-3 첫 결선 경계: armed + High opt-in 명령은 즉시 allow하지 않고 registered
/// device 승인 요청으로 승격한다. Low/Medium은 로컬 allow, Critical은 로컬 block,
/// High without opt-in은 기존 opt-in 안내 block을 유지한다.
#[cfg(feature = "remote")]
pub fn plan_remote_gate(
    registry: &crate::device_registry::DeviceRegistry,
    input: RemoteGatePlanInput<'_>,
) -> Result<RemoteGateStep> {
    use crate::risk::RiskLevel;

    if !input.armed {
        return Ok(RemoteGateStep::Local(GateReply::allow()));
    }

    let assessment = crate::risk::assess(input.command);
    match assessment.level {
        RiskLevel::Low | RiskLevel::Medium => Ok(RemoteGateStep::Local(GateReply::allow())),
        RiskLevel::Critical => Ok(RemoteGateStep::Local(decide_with(
            input.command,
            true,
            input.allow_high,
        ))),
        RiskLevel::High if !input.allow_high => Ok(RemoteGateStep::Local(decide_with(
            input.command,
            true,
            false,
        ))),
        RiskLevel::High => {
            let device = registry
                .select_device(input.device_id)
                .context("원격 승인 디바이스 선택 실패")?;
            let nonce = crate::approval::gen_nonce();
            let approval_id = crate::approval::gen_nonce().to_vec();
            let pending = crate::approval::PendingApproval {
                approval_id,
                nonce,
                expires_at: input.now.saturating_add(input.ttl),
                context_hash: input.context_hash.to_string(),
                device_epoch: device.epoch,
            };
            let command_masked = crate::mask::Masker::baseline().mask(input.command).text;
            let request =
                crate::session::ApprovalRequestMsg::from_pending(&pending, &command_masked);
            Ok(RemoteGateStep::NeedsRemote(RemoteApprovalPlan {
                device_id: device.id.clone(),
                pending,
                request,
            }))
        }
    }
}

/// RA-3 응답 접기: listener에서 받은 서명 응답을 등록 디바이스/nonce/context에 대해
/// 검증하고 최종 gate reply로 변환한다. nonce 미소비, 미등록 디바이스, 서명/TOCTOU
/// 실패는 모두 fail-closed block이다.
#[cfg(feature = "remote")]
pub fn finish_remote_gate_response(
    registry: &crate::device_registry::DeviceRegistry,
    plan: &RemoteApprovalPlan,
    nonces: &mut crate::approval::NonceStore,
    now: u64,
    current_context_hash: &str,
    response: &crate::session::ApprovalResponseMsg,
) -> GateReply {
    if !nonces.consume(&plan.pending.nonce, now) {
        return GateReply::block("원격 승인 nonce가 없거나 만료됨(fail-closed)".into());
    }

    match crate::device_registry::validate_registered_response(
        registry,
        &plan.device_id,
        &plan.pending,
        now,
        current_context_hash,
        response,
    ) {
        Ok(crate::approval::ApprovalOutcome::Approved) => GateReply::allow(),
        Ok(crate::approval::ApprovalOutcome::Rejected) => {
            GateReply::block("원격 디바이스가 실행을 거부함".into())
        }
        Ok(crate::approval::ApprovalOutcome::Invalid(reason)) => {
            GateReply::block(format!("원격 승인 검증 실패: {reason:?}"))
        }
        Err(err) => GateReply::block(format!("원격 승인 처리 실패: {err}")),
    }
}

#[cfg(feature = "remote")]
pub fn remote_timeout_reply() -> GateReply {
    GateReply::block("원격 승인 시간 초과(fail-closed)".into())
}

#[cfg(feature = "remote")]
pub struct RemoteGateRun<'a> {
    pub command: &'a str,
    pub armed: bool,
    pub allow_high: bool,
    pub now: u64,
    pub ttl: u64,
    pub device_id: Option<&'a str>,
    pub issued_context_hash: &'a str,
    pub context_origin: Option<&'a crate::context::RemoteContextOrigin>,
    pub current_context_hash: Option<&'a str>,
    pub response_timeout: std::time::Duration,
}

#[cfg(feature = "remote")]
fn remote_gate_current_context_hash(run: &RemoteGateRun<'_>) -> String {
    run.current_context_hash
        .map(str::to_owned)
        .unwrap_or_else(|| {
            run.context_origin.map_or_else(
                || crate::context::remote_context_hash(run.command),
                |origin| crate::context::remote_context_hash_for_origin(run.command, origin),
            )
        })
}

/// RA-3 queue-backed listener 결선: High opt-in 명령이면 device listener에 승인 요청을
/// 보내고 응답을 기다려 최종 GateReply로 접는다. `current_context_hash`가 None이면
/// 응답 검증 직전에 현재 컨텍스트를 재계산해 TOCTOU drift를 fail-closed 처리한다.
#[cfg(feature = "remote")]
pub fn decide_with_remote_listener(
    registry: &crate::device_registry::DeviceRegistry,
    listener: &DeviceListenerHandle,
    run: RemoteGateRun<'_>,
) -> GateReply {
    let step = match plan_remote_gate(
        registry,
        RemoteGatePlanInput {
            command: run.command,
            armed: run.armed,
            allow_high: run.allow_high,
            device_id: run.device_id,
            now: run.now,
            ttl: run.ttl,
            context_hash: run.issued_context_hash,
        },
    ) {
        Ok(step) => step,
        Err(err) => return GateReply::block(format!("원격 승인 계획 실패: {err}")),
    };
    let plan = match step {
        RemoteGateStep::Local(reply) => return reply,
        RemoteGateStep::NeedsRemote(plan) => plan,
    };

    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(plan.pending.nonce, plan.pending.expires_at);
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    let request = DeviceListenerRequest {
        request: plan.request.clone(),
        response_tx,
        accept_timeout: run.response_timeout,
    };
    if listener.request_tx.send(request).is_err() {
        return GateReply::block("원격 디바이스 리스너에 승인 요청 전송 실패".into());
    }

    match response_rx.recv_timeout(run.response_timeout) {
        Ok(Ok(response)) => {
            let recomputed_context_hash = remote_gate_current_context_hash(&run);
            finish_remote_gate_response(
                registry,
                &plan,
                &mut nonces,
                run.now,
                &recomputed_context_hash,
                &response,
            )
        }
        Ok(Err(err)) => GateReply::block(format!("원격 디바이스 승인 왕복 실패: {err}")),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => remote_timeout_reply(),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            GateReply::block("원격 디바이스 리스너 응답 채널 종료".into())
        }
    }
}

/// Relay-backed gate bridge boundary: plan the same remote approval request as
/// the live listener path, send it through a relay roundtrip implementation,
/// and fold the returned response through the existing fail-closed validator.
#[cfg(feature = "remote")]
pub fn decide_with_remote_relay_bridge<F>(
    registry: &crate::device_registry::DeviceRegistry,
    run: RemoteGateRun<'_>,
    mut relay_roundtrip: F,
) -> GateReply
where
    F: FnMut(
        &RemoteApprovalPlan,
        crate::session::CompanionTransportMsg,
    ) -> Result<crate::session::CompanionTransportMsg>,
{
    let step = match plan_remote_gate(
        registry,
        RemoteGatePlanInput {
            command: run.command,
            armed: run.armed,
            allow_high: run.allow_high,
            device_id: run.device_id,
            now: run.now,
            ttl: run.ttl,
            context_hash: run.issued_context_hash,
        },
    ) {
        Ok(step) => step,
        Err(err) => return GateReply::block(format!("원격 승인 계획 실패: {err}")),
    };
    let plan = match step {
        RemoteGateStep::Local(reply) => return reply,
        RemoteGateStep::NeedsRemote(plan) => plan,
    };

    let mut nonces = crate::approval::NonceStore::new();
    nonces.register(plan.pending.nonce, plan.pending.expires_at);
    let request = crate::session::CompanionTransportMsg::ApprovalRequest {
        request: plan.request.clone(),
    };
    let response = match relay_roundtrip(&plan, request) {
        Ok(response) => response,
        Err(err) => return GateReply::block(format!("원격 relay 승인 왕복 실패: {err}")),
    };

    match response {
        crate::session::CompanionTransportMsg::ApprovalResponse { response } => {
            let recomputed_context_hash = remote_gate_current_context_hash(&run);
            finish_remote_gate_response(
                registry,
                &plan,
                &mut nonces,
                run.now,
                &recomputed_context_hash,
                &response,
            )
        }
        crate::session::CompanionTransportMsg::Error { message } => {
            GateReply::block(format!("원격 relay companion 오류: {message}"))
        }
        other => GateReply::block(format!("원격 relay 응답 타입 오류: {other:?}")),
    }
}
