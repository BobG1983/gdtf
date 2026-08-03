use bevy::prelude::Deref;

use super::{
    anchor::Anchor,
    geometry::{Footprint, MinPlayerSide, RegionRect},
};
use crate::level::{SpawnRole, ThemeUuid};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterDemand(usize);

impl RosterDemand {
        #[must_use]
    pub const fn new(members: usize) -> Self {
        Self(members)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneCapacity(usize);

impl ZoneCapacity {
        #[must_use]
    pub const fn new(cells: usize) -> Self {
        Self(cells)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackingError {
                    NoPrefabForRole {
                theme: ThemeUuid,
                role:  SpawnRole,
    },
                    FootprintDoesNotFit {
                anchor:    Anchor,
                footprint: Footprint,
                region:    RegionRect,
    },
                PlayerFootprintTooSmall {
                footprint: Footprint,
                min_side:  MinPlayerSide,
    },
                        DeploymentZoneTooSmall {
                anchor:   Anchor,
                demand:   RosterDemand,
                capacity: ZoneCapacity,
    },
}

impl std::fmt::Display for PackingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPrefabForRole { theme, role } => write!(
                f,
                "no {role:?} prefab is registered for theme {theme:?}; cannot assemble a level \
                 without a candidate for this deployment role",
            ),
            Self::FootprintDoesNotFit {
                anchor,
                footprint,
                region,
            } => write!(
                f,
                "prefab footprint {}x{} does not fit the {anchor:?} region {region:?}",
                footprint.width(),
                footprint.height(),
            ),
            Self::PlayerFootprintTooSmall {
                footprint,
                min_side,
            } => write!(
                f,
                "player-spawn footprint {}x{} is below the minimum side {}",
                footprint.width(),
                footprint.height(),
                *min_side.cells(),
            ),
            Self::DeploymentZoneTooSmall {
                anchor,
                demand,
                capacity,
            } => write!(
                f,
                "the {anchor:?} deployment zone has {} standable cells but must stand {} roster \
                 members",
                **capacity, **demand,
            ),
        }
    }
}

impl std::error::Error for PackingError {}
