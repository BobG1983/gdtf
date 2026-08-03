//! Stance, aim, facing, and melee target wire payloads.

use bevy::prelude::Deref;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{cell::CellLevelNet, token::GangerToken};

/// Stance on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum StanceNet {
    /// Standing.
    Standing,
    /// Crouching.
    Crouching,
    /// Prone.
    Prone,
}

/// Aiming flag on the wire.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AimNet(bool);

impl AimNet {
    /// Build from a bool.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// Facing on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum FacingNet {
    /// North.
    North,
    /// North-east.
    NorthEast,
    /// East.
    East,
    /// South-east.
    SouthEast,
    /// South.
    South,
    /// South-west.
    SouthWest,
    /// West.
    West,
    /// North-west.
    NorthWest,
}

/// Melee target on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MeleeTargetNet {
    /// Attack a ganger.
    Ganger(GangerToken),
    /// Attack a structure at a cell.
    Structure(CellLevelNet),
}
