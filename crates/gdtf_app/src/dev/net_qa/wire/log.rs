use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::token::GangerToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ActProvenanceNet {
        Commanded,
        AiTurn,
            Reaction {
                        interrupted: GangerToken,
    },
            Clock,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogReadCap(u32);

impl LogReadCap {
        #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LogDroppedCount(u32);

impl LogDroppedCount {
        #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }
}
