//! Typed QA command trait that hosts implement once per command.

use core::fmt::Debug;

use bevy::prelude::App;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::dispatch::DeferredBudget;

/// A host command with fixed name, shapes, and a Bevy handler.
pub trait McpCommand: Sized + Send + Sync + 'static {
    /// Host facts type used for availability checks.
    type Facts: Send + Sync + 'static;

    /// Deserialized argument payload.
    type Args: DeserializeOwned + Debug + Send + Sync + 'static;

    /// Serialized reply payload.
    type Reply: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// What a deferred call carries while it waits. `()` when every waiter settles alike.
    type Parked: Send + Sync + 'static;

    /// Stable command name.
    const NAME: CommandName;

    /// Short human summary.
    const SUMMARY: CommandSummary;

    /// When the command may run relative to the frame.
    const TIMING: CommandTiming;

    /// How long this command's deferred replies may wait before timing out.
    const DEFERRED_BUDGET: DeferredBudget = crate::dispatch::DEFERRED_BUDGET;

    /// Whether the command is available given host facts.
    fn availability(facts: &Self::Facts) -> CommandAvailability;

    /// Register the Bevy system that handles this command.
    fn register_handler(app: &mut App);
}
