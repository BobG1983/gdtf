//! [`FakeBrokenSchema`] — the one thing that can publish a schema document which is not
//! JSON, so `assert_schemas_parse` has something real to fail on.

use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaJson,
};

use super::facts::FakeFacts;
use crate::command::ErasedCommand;

/// The argument document this fixture publishes: deliberately not JSON.
const BROKEN_ARG_SCHEMA: &str = "{ this was written by hand and is not json";

/// An entry in a host's slice that publishes a schema document `schemars` could never
/// produce.
///
/// [`ErasedCommand`] is implemented BY HAND here — the one place in the repo that does it,
/// and the crate docs' warning against it is the reason this type exists. A hand-written
/// impl is precisely how a set could come to advertise a shape no Rust type backs, so the
/// assertion that catches it has to be watched catching it, and nothing derived from
/// `schemars` can be made to fail. It implements [`QaCommand`](crate::command::QaCommand)
/// NOT AT ALL, so it does not collide with the blanket impl, and it appears in exactly one
/// slice ([`FAKE_COMMANDS_BROKEN_SCHEMA`](super::FAKE_COMMANDS_BROKEN_SCHEMA)) which is
/// never registered and never run.
pub struct FakeBrokenSchema;

impl ErasedCommand<FakeFacts> for FakeBrokenSchema {
    fn name(&self) -> CommandName {
        CommandName::from_static("fake.broken_schema")
    }

    fn summary(&self) -> CommandSummary {
        CommandSummary::from_static("Publishes an argument document that is not JSON.")
    }

    fn timing(&self) -> CommandTiming {
        CommandTiming::Immediate
    }

    fn arg_schema(&self) -> ArgSchemaJson {
        ArgSchemaJson::new(BROKEN_ARG_SCHEMA.to_owned())
    }

    fn reply_schema(&self) -> ReplySchemaJson {
        ReplySchemaJson::new("{}".to_owned())
    }

    fn availability(&self, _facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    /// Registers nothing: this entry must never be wired into a running `App`.
    fn register(&self, _app: &mut App) {}
}
