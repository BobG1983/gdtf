use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaJson,
};

use super::{QaCommand, schema::schema_text};

/// #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
/// #[derive(serde::Serialize, schemars::JsonSchema)]
pub trait ErasedCommand<F>: Send + Sync {
        fn name(&self) -> CommandName;
        fn summary(&self) -> CommandSummary;
        fn timing(&self) -> CommandTiming;
        fn arg_schema(&self) -> ArgSchemaJson;
        fn reply_schema(&self) -> ReplySchemaJson;
        fn availability(&self, facts: &F) -> CommandAvailability;
                            fn register(&self, app: &mut App);
}

impl<C: QaCommand> ErasedCommand<C::Facts> for C {
    fn name(&self) -> CommandName {
        C::NAME
    }

    fn summary(&self) -> CommandSummary {
        C::SUMMARY
    }

    fn timing(&self) -> CommandTiming {
        C::TIMING
    }

    fn arg_schema(&self) -> ArgSchemaJson {
        ArgSchemaJson::new(schema_text::<C::Args>())
    }

    fn reply_schema(&self) -> ReplySchemaJson {
        ReplySchemaJson::new(schema_text::<C::Reply>())
    }

    fn availability(&self, facts: &C::Facts) -> CommandAvailability {
        C::availability(facts)
    }

    fn register(&self, app: &mut App) {
        crate::dispatch::register_command::<C>(app);
    }
}
