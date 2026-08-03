//! Type-erased command trait used by catalogues and registration.

use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaJson,
};

use super::{QaCommand, schema::schema_text};

/// Object-safe view of a command for catalogues and host registration.
///
/// Concrete types implement this automatically via [`QaCommand`].
/// Args should derive `Debug`, `serde::Deserialize`, and `schemars::JsonSchema`.
/// Replies should derive `serde::Serialize` and `schemars::JsonSchema`.
pub trait ErasedCommand<F>: Send + Sync {
    /// Stable command name.
    fn name(&self) -> CommandName;
    /// Short human summary.
    fn summary(&self) -> CommandSummary;
    /// When the command may run relative to the frame.
    fn timing(&self) -> CommandTiming;
    /// JSON Schema for the argument payload.
    fn arg_schema(&self) -> ArgSchemaJson;
    /// JSON Schema for the reply payload.
    fn reply_schema(&self) -> ReplySchemaJson;
    /// Whether the command is available given host facts.
    fn availability(&self, facts: &F) -> CommandAvailability;
    /// Register Bevy systems for this command on `app`.
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
