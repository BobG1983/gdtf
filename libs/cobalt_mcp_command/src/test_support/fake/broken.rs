//! Command that publishes an unparseable argument shape.

use bevy::prelude::App;
use cobalt_mcp_protocol::command::{
    ArgSchemaRon, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaRon,
};

use super::facts::FakeFacts;
use crate::{
    command::ErasedCommand,
    dispatch::{DEFERRED_BUDGET, DeferredBudget},
};

const BROKEN_ARG_SCHEMA: &str = "( this was written by hand and is not a shape";

const EMPTY_SHAPE: &str = "(root:Unit,defs:[])";

/// Test command whose argument document is not a RON shape.
pub struct FakeBrokenSchema;

impl ErasedCommand<FakeFacts> for FakeBrokenSchema {
    fn name(&self) -> CommandName {
        CommandName::from_static("fake.broken_schema")
    }

    fn summary(&self) -> CommandSummary {
        CommandSummary::from_static("Publishes an argument document that is not a RON shape.")
    }

    fn timing(&self) -> CommandTiming {
        CommandTiming::Immediate
    }

    fn deferred_budget(&self) -> DeferredBudget {
        DEFERRED_BUDGET
    }

    fn arg_schema(&self) -> ArgSchemaRon {
        ArgSchemaRon::new(BROKEN_ARG_SCHEMA.to_owned())
    }

    fn reply_schema(&self) -> ReplySchemaRon {
        ReplySchemaRon::new(EMPTY_SHAPE.to_owned())
    }

    fn availability(&self, _facts: &FakeFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register(&self, _app: &mut App) {}
}
