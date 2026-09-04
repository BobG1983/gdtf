//! Fake command that queues calls and never drains them.

use bevy::prelude::*;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::command::McpCommand;

/// Arguments for `fake.stall`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FakeStallArgs {
    /// Label carried with the stalled call.
    pub label: FakeStallLabel,
}

/// Label string for a stalled call.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FakeStallLabel(String);

impl FakeStallLabel {
    /// Wrap a label.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// Reply type for `fake.stall` (never produced in normal use).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FakeStallReply {
    /// Echoed label.
    pub label: FakeStallLabel,
}

/// Command whose queue is intentionally never drained.
pub struct FakeStall;

impl McpCommand for FakeStall {
    type Args = FakeStallArgs;
    type Facts = FakeFacts;
    type Parked = ();
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
