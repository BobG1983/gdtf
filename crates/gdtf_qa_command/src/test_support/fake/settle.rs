//! [`FakeSettle`] — the fake command that DEFERS its answer to a later frame.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, QaCommandSystems, take_calls},
};

/// [`FakeSettle`]'s arguments: none.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeSettleArgs {}

/// How many frames the fake host says it took to settle.
///
/// Private-inner newtype over `u32` (no-bare-types).
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct FakeSettleCount(u32);

impl FakeSettleCount {
    /// Build a settle count.
    #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

/// [`FakeSettle`]'s reply — how long it waited.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeSettleReply {
    /// Frames the call waited before settling.
    pub waited: FakeSettleCount,
}

/// Whether the world condition [`FakeSettle`] waits on has arrived.
///
/// Private-inner newtype over `bool` (no-bare-types).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FakeSettleRaised(bool);

impl FakeSettleRaised {
    /// Build a raised-or-not fact.
    #[must_use]
    pub const fn new(raised: bool) -> Self {
        Self(raised)
    }
}

/// The world condition [`FakeSettle`] is waiting on.
///
/// A stand-in for whatever a real deferred command waits for — a turn ending, a capture
/// landing. A test raises it on the frame it wants the parked reply delivered, or never
/// raises it at all to watch the deferral deadline do its job.
#[derive(Resource, Debug, Default)]
pub struct FakeSettleSignal {
    /// Whether the condition has been met.
    raised: FakeSettleRaised,
    /// Frames counted since the plugin started, so the reply carries something real.
    frames: FakeSettleCount,
}

impl FakeSettleSignal {
    /// Whether the condition has been met.
    #[must_use]
    pub const fn is_raised(&self) -> FakeSettleRaised {
        self.raised
    }

    /// Mark the condition met, so the next frame delivers every parked reply.
    pub const fn raise(&mut self) {
        self.raised = FakeSettleRaised::new(true);
    }

    /// Count this frame and report the running total the reply carries.
    fn count_frame(&mut self) -> FakeSettleCount {
        self.frames = FakeSettleCount::new(self.frames.saturating_add(1));
        self.frames
    }
}

/// A fake command that parks its answer and settles it on a later frame.
///
/// Always admissible — the interesting part is not admission but that the ANSWER arrives
/// later, and that a parked answer which never settles still produces a reply.
pub struct FakeSettle;

impl QaCommand for FakeSettle {
    type Args = FakeSettleArgs;
    type Facts = FakeFacts;
    type Reply = FakeSettleReply;

    const NAME: CommandName = CommandName::from_static("fake.settle");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Answer on a later frame, once the fake signal is raised.");

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.init_resource::<FakeSettleSignal>();
        app.add_systems(
            Update,
            (park_fake_settle, settle_fake_settle)
                .chain()
                .after(QaCommandSystems::Claim),
        );
    }
}

/// Park every claimed [`FakeSettle`] call instead of answering it.
fn park_fake_settle(
    mut queue: ResMut<PendingQueue<CommandCall<FakeSettle>>>,
    mut deferred: ResMut<DeferredReplies<FakeSettle>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<FakeSettle>(&mut queue) {
        deferred.park(responder);
    }
}

/// Deliver every parked [`FakeSettle`] reply once the signal is raised.
fn settle_fake_settle(
    mut signal: ResMut<FakeSettleSignal>,
    mut deferred: ResMut<DeferredReplies<FakeSettle>>,
) {
    // Both resources are read through their immutable accessors first, so an idle frame
    // never dirties either one's change-detection flag.
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
