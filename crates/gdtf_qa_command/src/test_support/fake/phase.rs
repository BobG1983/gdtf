//! [`FakePhase`] — a fake command that is always available and takes no arguments.

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

/// [`FakePhase`]'s arguments: none at all.
///
/// `deny_unknown_fields` is what turns "you sent a field I do not have" into a
/// `BadArguments` answer carrying this type's own schema, instead of a silently ignored
/// key — and it is what puts `"additionalProperties": false` in the published schema.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakePhaseArgs {}

/// [`FakePhase`]'s reply: the two facts, echoed back.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakePhaseReply {
    /// Whether the fake model is loaded.
    pub ready: FakeReady,
    /// The current level index.
    pub level: FakeLevel,
}

/// A fake command that reads the host's facts and is always admissible.
///
/// The always-available half of the availability test: whatever the facts say, this one is
/// advertised `Available` and admitted.
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

/// Answer every claimed [`FakePhase`] call from the frame's facts.
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
