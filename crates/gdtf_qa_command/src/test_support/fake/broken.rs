use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaJson,
};

use super::facts::FakeFacts;
use crate::command::ErasedCommand;

const BROKEN_ARG_SCHEMA: &str = "{ this was written by hand and is not json";

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

        fn register(&self, _app: &mut App) {}
}
