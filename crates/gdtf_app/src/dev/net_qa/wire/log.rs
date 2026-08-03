//! Combat log provenance and read limits on the wire.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::token::GangerToken;

/// Why an act was logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ActProvenanceNet {
    /// Player or QA commanded.
    Commanded,
    /// AI turn.
    AiTurn,
    /// Reaction fire interrupting another actor.
    Reaction {
        /// Actor that was interrupted.
        interrupted: GangerToken,
    },
    /// Clock / timer driven.
    Clock,
}

/// Max number of log lines to return in one read.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogReadCap(u32);

impl LogReadCap {
    /// Build from a max line count.
    #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }
}

/// How many log lines were dropped before the read window.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogDroppedCount(u32);

impl LogDroppedCount {
    /// Build from a dropped count.
    #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }
}
