//! Host command catalogue returned to clients.

use serde::{Deserialize, Serialize};

use crate::{
    command::{
        ArgSchemaJson, CommandAvailability, CommandName, CommandSummary, CommandTiming,
        ReplySchemaJson,
    },
    message::ServerNameNet,
};

/// Full catalogue for one host.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandCatalogue {
    /// Host server name.
    pub host:    ServerNameNet,
    /// Registered commands.
    pub entries: Vec<CommandEntry>,
}

impl CommandCatalogue {
    /// Build a catalogue.
    #[must_use]
    pub const fn new(host: ServerNameNet, entries: Vec<CommandEntry>) -> Self {
        Self { host, entries }
    }
}

/// One command entry in the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandEntry {
    /// Command id.
    pub command:      CommandName,
    /// Short description.
    pub summary:      CommandSummary,
    /// Immediate vs deferred (`#[serde(default)]` for older frames).
    #[serde(default)]
    pub timing:       CommandTiming,
    /// JSON schema for arguments.
    pub arguments:    ArgSchemaJson,
    /// JSON schema for the reply body.
    pub reply:        ReplySchemaJson,
    /// Whether the command can run right now.
    pub availability: CommandAvailability,
}

impl CommandEntry {
    /// Build a catalogue entry.
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
