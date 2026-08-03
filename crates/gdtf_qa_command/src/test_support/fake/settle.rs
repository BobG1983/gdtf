use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, QaCommandSystems, take_calls},
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeSettleArgs {}

#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema,
)]
pub struct FakeSettleCount(u32);

impl FakeSettleCount {
        #[must_use]
    pub const fn new(frames: u32) -> Self {
        Self(frames)
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeSettleReply {
        pub waited: FakeSettleCount,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FakeSettleRaised(bool);

impl FakeSettleRaised {
        #[must_use]
    pub const fn new(raised: bool) -> Self {
        Self(raised)
    }
}

#[derive(Resource, Debug, Default)]
pub struct FakeSettleSignal {
        raised: FakeSettleRaised,
        frames: FakeSettleCount,
}

impl FakeSettleSignal {
        #[must_use]
    pub const fn is_raised(&self) -> FakeSettleRaised {
        self.raised
    }

        pub const fn raise(&mut self) {
        self.raised = FakeSettleRaised::new(true);
    }

        fn count_frame(&mut self) -> FakeSettleCount {
        self.frames = FakeSettleCount::new(self.frames.saturating_add(1));
        self.frames
    }
}

pub struct FakeSettle;

impl QaCommand for FakeSettle {
    type Args = FakeSettleArgs;
    type Facts = FakeFacts;
    type Reply = FakeSettleReply;

    const NAME: CommandName = CommandName::from_static("fake.settle");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Answer on a later frame, once the fake signal is raised.");

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
                .after(QaCommandSystems::Claim),
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
        deferred.park(responder);
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
