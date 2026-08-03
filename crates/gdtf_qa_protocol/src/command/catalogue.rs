use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming,
        ReplySchemaJson,
    },
    message::ServerNameNet,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandCatalogue {
        pub host:    ServerNameNet,
        pub entries: Vec<CommandEntry>,
}

impl CommandCatalogue {
        #[must_use]
    pub const fn new(host: ServerNameNet, entries: Vec<CommandEntry>) -> Self {
        Self { host, entries }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandEntry {
        pub command:      CommandName,
        pub summary:      CommandSummary,
            /// `#[serde(default)]` so a frame encoded before this field existed still decodes, as
            #[serde(default)]
    pub timing:       CommandTiming,
        pub arguments:    ArgSchemaJson,
        pub reply:        ReplySchemaJson,
        pub availability: CommandAvailability,
}

impl CommandEntry {
            #[must_use]
    pub const fn new(
        command: CommandName,
        summary: CommandSummary,
        timing: CommandTiming,
        arguments: ArgSchemaJson,
        reply: ReplySchemaJson,
        availability: CommandAvailability,
    ) -> Self {
        Self {
            command,
            summary,
            timing,
            arguments,
            reply,
            availability,
        }
    }
}
