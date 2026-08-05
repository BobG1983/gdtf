//! Type-erased command trait used by catalogues and registration.

use bevy::prelude::App;
use gdtf_qa_protocol::command::{
    ArgSchemaRon, CommandAvailability, CommandName, CommandSummary, CommandTiming, ReplySchemaRon,
    shape_text,
};

use super::QaCommand;
use crate::dispatch::DeferredBudget;

/// Object-safe view of a command for catalogues and host registration.
/// Concrete types implement this automatically via [`QaCommand`]. Both the argument and the
/// reply type derive `serde::Deserialize`, which is what the published shape is read out of.
pub trait ErasedCommand<F>: Send + Sync {
    /// Stable command name.
    fn name(&self) -> CommandName;
    /// Short human summary.
    fn summary(&self) -> CommandSummary;
    /// When the command may run relative to the frame.
    fn timing(&self) -> CommandTiming;
    /// How long this command's deferred replies may wait.
    fn deferred_budget(&self) -> DeferredBudget;
    /// RON shape for the argument payload.
    fn arg_schema(&self) -> ArgSchemaRon;
    /// RON shape for the reply payload.
    fn reply_schema(&self) -> ReplySchemaRon;
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

    fn deferred_budget(&self) -> DeferredBudget {
        C::DEFERRED_BUDGET
    }

    fn arg_schema(&self) -> ArgSchemaRon {
        ArgSchemaRon::new(shape_text::<C::Args>())
    }

    fn reply_schema(&self) -> ReplySchemaRon {
        ReplySchemaRon::new(shape_text::<C::Reply>())
    }

    fn availability(&self, facts: &C::Facts) -> CommandAvailability {
        C::availability(facts)
    }

    fn register(&self, app: &mut App) {
        crate::dispatch::register_command::<C>(app);
    }
}
