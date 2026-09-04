//! Command replies held until the capture rider's screenshot lands.

use std::sync::{
    Mutex,
    mpsc::{Receiver, TryRecvError},
};

use bevy::prelude::*;
use cobalt_mcp_protocol::{
    command::{ArtifactPath, AttachmentKind, CommandOutcome, CommandReplyRon, ReplyAttachment},
    ids::ShotName,
    message::{McpResponse, McpSessionError},
};

use crate::transport::Responder;

/// Names one held reply and the screenshot it waits for.
#[derive(Deref, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CaptureTicket(u64);

impl CaptureTicket {
    /// Wrap a ticket number.
    #[must_use]
    pub const fn new(ticket: u64) -> Self {
        Self(ticket)
    }

    const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

/// One screenshot a rider has asked the host to take.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShotRequest {
    ticket: CaptureTicket,
    name:   Option<ShotName>,
}

impl ShotRequest {
    /// Ticket this shot's result is reported under.
    #[must_use]
    pub const fn ticket(&self) -> CaptureTicket {
        self.ticket
    }

    /// File stem the caller asked for, if any.
    #[must_use]
    pub const fn name(&self) -> Option<&ShotName> {
        self.name.as_ref()
    }
}

/// How a rider's screenshot finished.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RiderShot {
    /// A PNG landed at this path.
    Landed(ArtifactPath),
    /// No PNG ever landed.
    Lost,
}

struct RanReply {
    reply:       CommandReplyRon,
    attachments: Vec<ReplyAttachment>,
}

enum HoldStage {
    Running,
    Shooting(RanReply),
}

enum HoldStep {
    Keep(HeldReply),
    Shoot(HeldReply, ShotRequest),
    Answered,
}

struct HeldReply {
    ticket: CaptureTicket,
    name:   Option<ShotName>,
    caller: Responder,
    answer: Mutex<Receiver<McpResponse>>,
    stage:  HoldStage,
}

impl HeldReply {
    const fn is_shooting(&self) -> bool {
        matches!(self.stage, HoldStage::Shooting(_))
    }

    fn take_reply(&mut self) -> Option<McpResponse> {
        let answer = match self.answer.get_mut() {
            Ok(answer) => answer,
            Err(poisoned) => poisoned.into_inner(),
        };
        match answer.try_recv() {
            Ok(response) => Some(response),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(McpResponse::Error(McpSessionError::Timeout)),
        }
    }

    fn step(mut self) -> HoldStep {
        if self.is_shooting() {
            return HoldStep::Keep(self);
        }
        match self.take_reply() {
            None => HoldStep::Keep(self),
            Some(McpResponse::Outcome(CommandOutcome::Ran { reply, attachments })) => {
                let request = ShotRequest {
                    ticket: self.ticket,
                    name:   self.name.clone(),
                };
                self.stage = HoldStage::Shooting(RanReply { reply, attachments });
                HoldStep::Shoot(self, request)
            }
            Some(other) => {
                self.caller.reply(other);
                HoldStep::Answered
            }
        }
    }

    fn answer(self, shot: RiderShot) {
        let Self { caller, stage, .. } = self;
        let HoldStage::Shooting(ran) = stage else {
            return;
        };
        match shot {
            RiderShot::Landed(path) => {
                let mut attachments = ran.attachments;
                attachments.push(ReplyAttachment::new(AttachmentKind::Png, path));
                caller.reply(McpResponse::Outcome(CommandOutcome::Ran {
                    reply: ran.reply,
                    attachments,
                }));
            }
            RiderShot::Lost => caller.reply(McpResponse::Error(McpSessionError::Timeout)),
        }
    }
}

/// Bevy resource holding each capture rider's reply until its screenshot lands.
#[derive(Resource, Default)]
pub struct CaptureHolds {
    held:     Vec<HeldReply>,
    requests: Vec<ShotRequest>,
    next:     CaptureTicket,
}

impl CaptureHolds {
    /// Hold `caller`'s reply behind a shot named `name`; the command answers the responder returned.
    #[must_use]
    pub fn hold(&mut self, caller: Responder, name: Option<ShotName>) -> Responder {
        let (inner, answer) = Responder::channel();
        self.held.push(HeldReply {
            ticket: self.next.advance(),
            name,
            caller,
            answer: Mutex::new(answer),
            stage: HoldStage::Running,
        });
        inner
    }

    /// Take the shots the host has not queued yet.
    #[must_use]
    pub fn drain_requests(&mut self) -> Vec<ShotRequest> {
        core::mem::take(&mut self.requests)
    }

    /// Report how a requested shot finished and answer the call it belongs to.
    pub fn complete(&mut self, ticket: CaptureTicket, shot: RiderShot) {
        let found = self
            .held
            .iter()
            .position(|entry| entry.ticket == ticket && entry.is_shooting());
        let Some(index) = found else {
            return;
        };
        self.held.remove(index).answer(shot);
    }

    /// Whether nothing is held.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Number of replies held.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.held.len()
    }

    fn poll(&mut self) {
        let mut kept = Vec::with_capacity(self.held.len());
        for entry in core::mem::take(&mut self.held) {
            match entry.step() {
                HoldStep::Keep(entry) => kept.push(entry),
                HoldStep::Shoot(entry, request) => {
                    self.requests.push(request);
                    kept.push(entry);
                }
                HoldStep::Answered => {}
            }
        }
        self.held = kept;
    }
}

/// Move a finished reply on to its screenshot, and pass any other outcome straight back.
pub fn poll_capture_holds(mut holds: ResMut<CaptureHolds>) {
    if holds.is_empty() {
        return;
    }
    holds.poll();
}
