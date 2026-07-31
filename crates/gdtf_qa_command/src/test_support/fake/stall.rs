//! [`FakeStall`] — the fake command whose handler NEVER drains its queue, so the pending
//! queue's frame deadline has something to reap.

use bevy::prelude::*;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::command::QaCommand;

/// [`FakeStall`]'s arguments — one marker field, so the queue entry the sweep logs carries
/// something a reader can recognise.
///
/// This is where [`QaCommand::Args`]' `Debug` bound earns its place: `sweep_pending` prints
/// the timed-out payload, and a `CommandCall<C>` can only print itself because its `Args`
/// are printable.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeStallArgs {
    /// A label the timed-out payload carries into the sweep's log line.
    pub label: FakeStallLabel,
}

/// The label a stalled call carries.
///
/// Private-inner newtype over `String` (no-bare-types).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FakeStallLabel(String);

impl FakeStallLabel {
    /// Build a stall label.
    #[must_use]
    pub const fn new(label: String) -> Self {
        Self(label)
    }
}

/// [`FakeStall`]'s reply — declared, and never produced.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeStallReply {
    /// The label that came in.
    pub label: FakeStallLabel,
}

/// A fake command that is admitted, decoded, queued — and then abandoned.
///
/// It registers NO handler at all, deliberately. A real host's equivalent is a handler that
/// early-outs on a condition it turns out never to meet: the call sits in
/// `PendingQueue<CommandCall<FakeStall>>` with nobody to drain it. Without the
/// `sweep_pending::<CommandCall<C>>` registration in
/// [`register_command`](crate::dispatch::register_command) that call would hang the client's
/// socket instead of being answered [`Timeout`](gdtf_qa_protocol::envelope::QaError::Timeout),
/// which is exactly what the stalled-call test observes.
pub struct FakeStall;

impl QaCommand for FakeStall {
    type Args = FakeStallArgs;
    type Facts = FakeFacts;
    type Reply = FakeStallReply;

    const NAME: CommandName = CommandName::from_static("fake.stall");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Queue a call and never drain it. Test fixture only.");

    fn availability(_facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    /// Registers nothing — that is the fixture.
    fn register_handler(_app: &mut App) {}
}
