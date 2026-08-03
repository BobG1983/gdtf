use serde::{Deserialize, Serialize};

use super::hello::ProtocolVersion;
use crate::command::{CommandArgsJson, CommandName, RunOptions};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaRequest {
                                Hello(ProtocolVersion),
                                Catalogue,
                            Run(RunCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunCommand {
        pub command:   CommandName,
        pub arguments: CommandArgsJson,
            /// `#[serde(default)]` so a frame encoded before this field existed still decodes, as
        #[serde(default)]
    pub options:   RunOptions,
}

impl RunCommand {
        #[must_use]
    pub fn new(command: CommandName, arguments: CommandArgsJson) -> Self {
        Self::with_options(command, arguments, RunOptions::default())
    }

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
