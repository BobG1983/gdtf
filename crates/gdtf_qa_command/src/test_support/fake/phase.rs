use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::{FakeFacts, FakeLevel, FakeReady};
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakePhaseArgs {}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakePhaseReply {
        pub ready: FakeReady,
        pub level: FakeLevel,
}

pub struct FakePhase;

impl QaCommand for FakePhase {
    type Args = FakePhaseArgs;
    type Facts = FakeFacts;
    type Reply = FakePhaseReply;

    const NAME: CommandName = CommandName::from_static("fake.phase");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Read the fake host's facts. Always available.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_fake_phase.after(QaCommandSystems::Claim));
    }
}

fn handle_fake_phase(
    facts: Res<FakeFacts>,
    mut queue: ResMut<PendingQueue<CommandCall<FakePhase>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (_args, responder) in take_calls::<FakePhase>(&mut queue) {
        responder.answer(&FakePhaseReply {
            ready: facts.ready(),
            level: facts.level(),
        });
    }
}
