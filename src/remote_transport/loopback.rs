use std::collections::{HashMap, VecDeque};

use anyhow::{bail, Result};

use super::frame::{CompanionRelayFrame, CompanionRelayPeer};
use super::validate::valid_relay_session_id;
use super::DEFAULT_COMPANION_RELAY_FRAME_TTL_MS;

#[derive(Debug, Default)]
pub struct CompanionRelayLoopback {
    sessions: HashMap<String, CompanionRelaySessionQueue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionRelayLoopbackStats {
    pub session_count: usize,
    pub queued_frames: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionRelayEndpoint {
    session_id: String,
    peer: CompanionRelayPeer,
    next_sequence: u64,
    frame_ttl_ms: u64,
}

#[derive(Debug, Default)]
struct CompanionRelaySessionQueue {
    daemon_to_companion: VecDeque<CompanionRelayFrame>,
    companion_to_daemon: VecDeque<CompanionRelayFrame>,
    last_daemon_sequence: u64,
    last_companion_sequence: u64,
}

impl CompanionRelayLoopback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, frame: CompanionRelayFrame) -> Result<()> {
        frame.validate_metadata()?;
        let session = self.sessions.entry(frame.session_id.clone()).or_default();
        session.enqueue(frame)
    }

    pub fn dequeue(
        &mut self,
        session_id: &str,
        recipient: CompanionRelayPeer,
        now_ms: u64,
    ) -> Result<Option<CompanionRelayFrame>> {
        if !valid_relay_session_id(session_id) {
            bail!("relay session_id format error");
        }

        let (frame, remove_session) = match self.sessions.get_mut(session_id) {
            Some(session) => {
                let frame = session.dequeue_for_recipient(recipient, now_ms);
                (frame, session.is_empty())
            }
            None => return Ok(None),
        };
        if remove_session {
            self.sessions.remove(session_id);
        }
        Ok(frame)
    }

    pub fn queued_for(&self, session_id: &str, recipient: CompanionRelayPeer) -> Result<usize> {
        if !valid_relay_session_id(session_id) {
            bail!("relay session_id format error");
        }
        Ok(self
            .sessions
            .get(session_id)
            .map(|session| session.queue_for_recipient(recipient).len())
            .unwrap_or(0))
    }

    pub fn stats(&self) -> CompanionRelayLoopbackStats {
        CompanionRelayLoopbackStats {
            session_count: self.sessions.len(),
            queued_frames: self
                .sessions
                .values()
                .map(CompanionRelaySessionQueue::queued_frames)
                .sum(),
        }
    }
}

impl CompanionRelayEndpoint {
    pub fn new(session_id: impl Into<String>, peer: CompanionRelayPeer) -> Result<Self> {
        Self::with_frame_ttl(session_id, peer, DEFAULT_COMPANION_RELAY_FRAME_TTL_MS)
    }

    pub fn with_frame_ttl(
        session_id: impl Into<String>,
        peer: CompanionRelayPeer,
        frame_ttl_ms: u64,
    ) -> Result<Self> {
        let session_id = session_id.into();
        if !valid_relay_session_id(&session_id) {
            bail!("relay session_id format error");
        }
        if frame_ttl_ms == 0 {
            bail!("relay frame_ttl_ms must be positive");
        }
        Ok(Self {
            session_id,
            peer,
            next_sequence: 1,
            frame_ttl_ms,
        })
    }

    pub fn daemon(session_id: impl Into<String>) -> Result<Self> {
        Self::new(session_id, CompanionRelayPeer::Daemon)
    }

    pub fn companion(session_id: impl Into<String>) -> Result<Self> {
        Self::new(session_id, CompanionRelayPeer::Companion)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn peer(&self) -> CompanionRelayPeer {
        self.peer
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn send_message(
        &mut self,
        relay: &mut CompanionRelayLoopback,
        now_ms: u64,
        message: &crate::session::CompanionTransportMsg,
    ) -> Result<u64> {
        let frame = self.next_frame(now_ms, message)?;
        let sequence = frame.sequence;
        relay.enqueue(frame)?;
        Ok(sequence)
    }

    pub fn next_frame(
        &mut self,
        now_ms: u64,
        message: &crate::session::CompanionTransportMsg,
    ) -> Result<CompanionRelayFrame> {
        let sequence = self.next_sequence;
        let expires_at_ms = match now_ms.checked_add(self.frame_ttl_ms) {
            Some(value) => value,
            None => bail!("relay frame expiry overflow"),
        };
        let frame = CompanionRelayFrame::from_message(
            self.session_id.clone(),
            self.peer,
            sequence,
            now_ms,
            expires_at_ms,
            message,
        )?;
        self.next_sequence = match self.next_sequence.checked_add(1) {
            Some(value) => value,
            None => bail!("relay sequence overflow"),
        };
        Ok(frame)
    }

    pub fn accept_frame(
        &self,
        frame: CompanionRelayFrame,
        now_ms: u64,
    ) -> Result<Option<crate::session::CompanionTransportMsg>> {
        frame.validate_metadata()?;
        if frame.session_id != self.session_id {
            bail!("relay session_id mismatch");
        }
        if frame.sender == self.peer {
            bail!("relay sender matches endpoint");
        }
        if now_ms >= frame.expires_at_ms {
            return Ok(None);
        }
        Ok(Some(frame.payload_message()?))
    }

    pub fn recv_message(
        &self,
        relay: &mut CompanionRelayLoopback,
        now_ms: u64,
    ) -> Result<Option<crate::session::CompanionTransportMsg>> {
        match relay.dequeue(&self.session_id, self.peer, now_ms)? {
            Some(frame) => Ok(Some(frame.payload_message()?)),
            None => Ok(None),
        }
    }
}

impl CompanionRelaySessionQueue {
    fn enqueue(&mut self, frame: CompanionRelayFrame) -> Result<()> {
        let last_sequence = match frame.sender {
            CompanionRelayPeer::Daemon => &mut self.last_daemon_sequence,
            CompanionRelayPeer::Companion => &mut self.last_companion_sequence,
        };
        if frame.sequence <= *last_sequence {
            bail!("relay sequence must increase for sender");
        }
        *last_sequence = frame.sequence;
        self.queue_for_sender_mut(frame.sender).push_back(frame);
        Ok(())
    }

    fn dequeue_for_recipient(
        &mut self,
        recipient: CompanionRelayPeer,
        now_ms: u64,
    ) -> Option<CompanionRelayFrame> {
        let queue = self.queue_for_recipient_mut(recipient);
        while let Some(frame) = queue.pop_front() {
            if now_ms < frame.expires_at_ms {
                return Some(frame);
            }
        }
        None
    }

    fn queued_frames(&self) -> usize {
        self.daemon_to_companion.len() + self.companion_to_daemon.len()
    }

    fn is_empty(&self) -> bool {
        self.queued_frames() == 0
    }

    fn queue_for_sender_mut(
        &mut self,
        sender: CompanionRelayPeer,
    ) -> &mut VecDeque<CompanionRelayFrame> {
        match sender {
            CompanionRelayPeer::Daemon => &mut self.daemon_to_companion,
            CompanionRelayPeer::Companion => &mut self.companion_to_daemon,
        }
    }

    fn queue_for_recipient_mut(
        &mut self,
        recipient: CompanionRelayPeer,
    ) -> &mut VecDeque<CompanionRelayFrame> {
        match recipient {
            CompanionRelayPeer::Daemon => &mut self.companion_to_daemon,
            CompanionRelayPeer::Companion => &mut self.daemon_to_companion,
        }
    }

    fn queue_for_recipient(&self, recipient: CompanionRelayPeer) -> &VecDeque<CompanionRelayFrame> {
        match recipient {
            CompanionRelayPeer::Daemon => &self.companion_to_daemon,
            CompanionRelayPeer::Companion => &self.daemon_to_companion,
        }
    }
}
