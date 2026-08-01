//! [`FakePhaseTwin`] — a deliberate name collision, so the uniqueness assertion has
//! something real to fail on.

use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::{
    facts::FakeFacts,
    phase::{FakePhase, FakePhaseArgs, FakePhaseReply},
};
use crate::command::QaCommand;

/// A second command claiming [`FakePhase`]'s name.
///
/// It exists ONLY to be put in a deliberately broken slice
/// ([`FAKE_COMMANDS_DUPLICATED`](super::FAKE_COMMANDS_DUPLICATED)) so
/// `assert_unique_names` can be shown to fail on one. It registers no handler at all,
/// because nothing should ever run it.
pub struct FakePhaseTwin;

impl QaCommand for FakePhaseTwin {
    type Args = FakePhaseArgs;
    type Facts = FakeFacts;
    type Reply = FakePhaseReply;

    /// The same name [`FakePhase`] declares — that is the whole point of this type.
    const NAME: CommandName = FakePhase::NAME;
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("A deliberate name collision. Never wired into a real set.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(_app: &mut App) {}
}
