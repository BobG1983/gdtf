use core::fmt::Debug;

use bevy::prelude::App;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};

pub trait QaCommand: Sized + Send + Sync + 'static {
                        type Facts: Send + Sync + 'static;

        type Args: DeserializeOwned + JsonSchema + Debug + Send + Sync + 'static;

                                type Reply: Serialize + JsonSchema + Send + Sync + 'static;

                        const NAME: CommandName;

        const SUMMARY: CommandSummary;

                                    const TIMING: CommandTiming;

                    fn availability(facts: &Self::Facts) -> CommandAvailability;

                                fn register_handler(app: &mut App);
}
