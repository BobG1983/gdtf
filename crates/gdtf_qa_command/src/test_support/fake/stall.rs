use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::command::QaCommand;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeStallArgs {
        pub label: FakeStallLabel,
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FakeStallLabel(String);

impl FakeStallLabel {
        #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeStallReply {
        pub label: FakeStallLabel,
}

pub struct FakeStall;

impl QaCommand for FakeStall {
    type Args = FakeStallArgs;
    type Facts = FakeFacts;
    type Reply = FakeStallReply;

    const NAME: CommandName = CommandName::from_static("fake.stall");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Queue a call and never drain it. Test fixture only.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

        fn register_handler(_app: &mut App) {}
}
