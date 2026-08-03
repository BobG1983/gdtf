//! Typed QA command trait that hosts implement once per command.

use core::fmt::Debug;

use bevy::prelude::App;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};

/// A host command with fixed name, schemas, and a Bevy handler.
pub trait QaCommand: Sized + Send + Sync + 'static {
    /// Host facts type used for availability checks.
    type Facts: Send + Sync + 'static;

    /// Deserialized argument payload.
    type Args: DeserializeOwned + JsonSchema + Debug + Send + Sync + 'static;

    /// Serialized reply payload.
    type Reply: Serialize + JsonSchema + Send + Sync + 'static;

    /// Stable command name.
    const NAME: CommandName;

    /// Short human summary.
    const SUMMARY: CommandSummary;

    /// When the command may run relative to the frame.
    const TIMING: CommandTiming;

    /// Whether the command is available given host facts.
    fn availability(facts: &Self::Facts) -> CommandAvailability;

    /// Register the Bevy system that handles this command.
    fn register_handler(app: &mut App);
}
