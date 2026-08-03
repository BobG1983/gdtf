//! Fake command that echoes text when the host level is above zero.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::facts::FakeFacts;
use crate::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};

/// Text payload for the echo command.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct FakeEchoText(String);

impl FakeEchoText {
    /// Wrap a text string.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }

    /// Borrow the text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Arguments for `fake.echo`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeEchoArgs {
    /// Text to echo.
    pub text: FakeEchoText,
}

/// Reply for `fake.echo`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FakeEchoReply {
    /// Echoed text.
    pub text: FakeEchoText,
}

/// Echo text; requires level above zero.
pub struct FakeEcho;

impl QaCommand for FakeEcho {
    type Args = FakeEchoArgs;
    type Facts = FakeFacts;
    type Reply = FakeEchoReply;

    const NAME: CommandName = CommandName::from_static("fake.echo");
    const SUMMARY: CommandSummary =
        CommandSummary::from_static("Echo text back. Needs a level above zero.");

    const TIMING: CommandTiming = CommandTiming::Immediate;

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

fn handle_fake_echo(mut queue: ResMut<PendingQueue<CommandCall<FakeEcho>>>) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<FakeEcho>(&mut queue) {
        responder.answer(&FakeEchoReply { text: args.text });
    }
}
