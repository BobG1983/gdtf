//! Deliberate name collision with [`FakePhase`] for uniqueness tests.

use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::{
    facts::FakeFacts,
    phase::{FakePhase, FakePhaseArgs, FakePhaseReply},
};
use crate::command::QaCommand;

/// Same name as `FakePhase`; used only in collision tests.
pub struct FakePhaseTwin;

impl QaCommand for FakePhaseTwin {
    type Args = FakePhaseArgs;
    type Facts = FakeFacts;
    type Reply = FakePhaseReply;

    const NAME: CommandName = FakePhase::NAME;
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("A deliberate name collision. Never wired into a real set.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(_app: &mut App) {}
}
