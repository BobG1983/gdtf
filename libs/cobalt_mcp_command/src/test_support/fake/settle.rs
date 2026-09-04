//! Deferred fake command that answers after a signal is raised.

use core::time::Duration;

use bevy::prelude::*;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, McpCommandSystems, take_calls},
};

/// Empty arguments for `fake.settle`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FakeSettleArgs {}

/// Frames waited before the reply was delivered.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FakeSettleCount(u32);

impl FakeSettleCount {
    /// Wrap a frame count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

/// Reply for `fake.settle`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FakeSettleReply {
    /// Frames waited.
    pub waited: FakeSettleCount,
}

/// Whether the settle signal has been raised.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FakeSettleRaised(bool);

impl FakeSettleRaised {
    /// Wrap a raised flag.
    #[must_use]
    pub const fn new(raised: bool) -> Self {
        Self(raised)
    }
}

/// Bevy resource that tests raise to unblock deferred settle replies.
#[derive(Resource, Debug, Default)]
pub struct FakeSettleSignal {
    raised: FakeSettleRaised,
    frames: FakeSettleCount,
}

impl FakeSettleSignal {
    /// Whether the signal is raised.
    #[must_use]
    pub const fn is_raised(&self) -> FakeSettleRaised {
        self.raised
    }

    /// Raise the signal so parked replies can settle.
    pub const fn raise(&mut self) {
        self.raised = FakeSettleRaised::new(true);
    }

    fn count_frame(&mut self) -> FakeSettleCount {
        self.frames = FakeSettleCount::new(self.frames.saturating_add(1));
        self.frames
    }
}

/// Deferred command that parks until [`FakeSettleSignal`] is raised.
pub struct FakeSettle;

impl McpCommand for FakeSettle {
    type Args = FakeSettleArgs;
    type Facts = FakeFacts;
    type Parked = ();
    type Reply = FakeSettleReply;

    const NAME: CommandName = CommandName::from_static("fake.settle");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Answer on a later frame, once the fake signal is raised.");

    // Far above any case's own frames yet inside the must-beat-the-socket invariant;
    // a case that wants expiry sets a zero budget explicitly.
    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(60));
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.init_resource::<FakeSettleSignal>();
        app.add_systems(
            Update,
            (park_fake_settle, settle_fake_settle)
                .chain()
                .after(McpCommandSystems::Claim),
        );
    }
}

fn park_fake_settle(
    mut queue: ResMut<PendingQueue<CommandCall<FakeSettle>>>,
    mut deferred: ResMut<DeferredReplies<FakeSettle>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<FakeSettle>(&mut queue) {
        deferred.park(responder, ());
    }
}

fn settle_fake_settle(
    mut signal: ResMut<FakeSettleSignal>,
    mut deferred: ResMut<DeferredReplies<FakeSettle>>,
) {
    if !*signal.is_raised() || deferred.is_empty() {
        return;
    }
    let waited = signal.count_frame();
    let delivered = deferred.answer_all(&FakeSettleReply { waited });
    debug!(
        delivered = *delivered,
        "fake.settle: delivered parked replies"
    );
}
