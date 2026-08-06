//! Stance, aim, facing, and melee target wire payloads.

use bevy::prelude::Deref;
use gdtf_battle_sim::{
    ganger::Direction,
    prelude::{Stance, StanceKind},
};
use serde::{Deserialize, Serialize};

use super::{cell::CellLevelNet, token::GangerToken};

/// Stance on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceNet {
    /// Standing.
    Standing,
    /// Crouching.
    Crouching,
    /// Prone.
    Prone,
}

impl StanceNet {
    /// Mirror the sim's posture.
    #[must_use]
    pub fn from_sim(stance: Stance) -> Self {
        match *stance {
            StanceKind::Standing => Self::Standing,
            StanceKind::Crouching => Self::Crouching,
            StanceKind::Prone => Self::Prone,
        }
    }

    /// The posture the sim knows this one as.
    #[must_use]
    pub const fn to_sim(self) -> StanceKind {
        match self {
            Self::Standing => StanceKind::Standing,
            Self::Crouching => StanceKind::Crouching,
            Self::Prone => StanceKind::Prone,
        }
    }
}

/// Aiming flag on the wire.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl FacingNet {
    /// Mirror the sim's direction.
    #[must_use]
    pub const fn from_sim(facing: Direction) -> Self {
        match facing {
            Direction::North => Self::North,
            Direction::NorthEast => Self::NorthEast,
            Direction::East => Self::East,
            Direction::SouthEast => Self::SouthEast,
            Direction::South => Self::South,
            Direction::SouthWest => Self::SouthWest,
            Direction::West => Self::West,
            Direction::NorthWest => Self::NorthWest,
        }
    }

    /// The direction the sim knows this one as.
    #[must_use]
    pub const fn to_sim(self) -> Direction {
        match self {
            Self::North => Direction::North,
            Self::NorthEast => Direction::NorthEast,
            Self::East => Direction::East,
            Self::SouthEast => Direction::SouthEast,
            Self::South => Direction::South,
            Self::SouthWest => Direction::SouthWest,
            Self::West => Direction::West,
            Self::NorthWest => Direction::NorthWest,
        }
    }
}

/// Melee target on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeleeTargetNet {
    /// Attack a ganger.
    Ganger(GangerToken),
    /// Attack a structure at a cell.
    Structure(CellLevelNet),
}
