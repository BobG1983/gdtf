//! Server → client responses.

use serde::{Deserialize, Serialize};

use super::{error::McpSessionError, hello::HelloFacts};
use crate::command::{CommandCatalogue, CommandOutcome};

/// Top-level response to a QA client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpResponse {
    /// Successful hello negotiation.
    HelloOk(HelloFacts),
    /// Full command catalogue.
    Catalogue(CommandCatalogue),
    /// Result of a run command.
    Outcome(CommandOutcome),
    /// Protocol or session error.
    Error(McpSessionError),
}
