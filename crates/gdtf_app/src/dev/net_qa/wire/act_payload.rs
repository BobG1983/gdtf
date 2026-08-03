use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{cell::CellLevelNet, token::GangerToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum StanceNet {
        Standing,
        Crouching,
        Prone,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AimNet(bool);

impl AimNet {
        #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum FacingNet {
        North,
        NorthEast,
        East,
        SouthEast,
        South,
        SouthWest,
        West,
        NorthWest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MeleeTargetNet {
        Ganger(GangerToken),
        Structure(CellLevelNet),
}
