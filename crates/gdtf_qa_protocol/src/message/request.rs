//! Client → server requests.

use serde::{Deserialize, Serialize};

use super::hello::ProtocolVersion;
use crate::command::{CommandArgsJson, CommandName, RunOptions};

/// Top-level request from a QA client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaRequest {
    /// Negotiate protocol version.
    Hello(ProtocolVersion),
    /// Ask for the command catalogue.
    Catalogue,
    /// Run a named command.
    Run(RunCommand),
}

/// Payload for [`QaRequest::Run`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunCommand {
    /// Command to run.
    pub command: CommandName,
    /// JSON argument blob for the command.
    pub arguments: CommandArgsJson,
    /// Optional run flags (`#[serde(default)]` for older frames).
    #[serde(default)]
    pub options: RunOptions,
}

impl RunCommand {
    /// Build a run request with default options.
    #[must_use]
    pub fn new(command: CommandName, arguments: CommandArgsJson) -> Self {
        Self::with_options(command, arguments, RunOptions::default())
    }

    /// Build a run request with explicit options.
    #[must_use]
    pub const fn with_options(
        command: CommandName,
        arguments: CommandArgsJson,
        options: RunOptions,
    ) -> Self {
        Self {
            command,
            arguments,
            options,
        }
    }
}
