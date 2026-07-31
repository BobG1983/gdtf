//! [`FakeEcho`] — the command the growth test ADDS to the fake set.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, RefusalNote, UnavailableCode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

/// The text [`FakeEcho`] carries in and back out.
///
/// Private-inner newtype over `String` (no-bare-types).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FakeEchoText(String);

impl FakeEchoText {
    /// Build echo text.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }

    /// This text as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// [`FakeEcho`]'s arguments — the text to echo.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeEchoArgs {
    /// The text to send back.
    pub text: FakeEchoText,
}

/// [`FakeEcho`]'s reply — the same text.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeEchoReply {
    /// The text that came in.
    pub text: FakeEchoText,
}

/// The THIRD fake command — the one the growth test adds to prove that adding a command
/// moves no protocol version.
///
/// It keys its availability on a DIFFERENT fact from [`FakeCell`](super::FakeCell) (the
/// level rather than the model), so a grown set still has two independent predicates in it.
pub struct FakeEcho;

impl QaCommand for FakeEcho {
    type Args = FakeEchoArgs;
    type Facts = FakeFacts;
    type Reply = FakeEchoReply;

    const NAME: CommandName = CommandName::from_static("fake.echo");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Echo text back. Needs a level above zero.");

    fn availability(facts: &FakeFacts) -> CommandAvailability {
        if *facts.level() > 0 {
            return CommandAvailability::Available;
        }
        CommandAvailability::Unavailable {
            code: UnavailableCode::WrongState,
            note: RefusalNote::from_static("the fake host is still on level zero"),
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_fake_echo.after(QaCommandSystems::Claim));
    }
}

/// Answer every claimed [`FakeEcho`] call with the text it carried.
fn handle_fake_echo(mut queue: ResMut<PendingQueue<CommandCall<FakeEcho>>>) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<FakeEcho>(&mut queue) {
        responder.answer(&FakeEchoReply { text: args.text });
    }
}
