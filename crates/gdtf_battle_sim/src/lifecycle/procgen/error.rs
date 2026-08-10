//! Errors from packing and deployment placement.

use bevy::prelude::Deref;

use super::{
    anchor::Anchor,
    geometry::{Footprint, MinPlayerSide, RegionRect},
};
use crate::level::{SpawnRole, ThemeUuid};

/// How many roster members need cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterDemand(usize);

impl RosterDemand {
    /// Wrap a member count.
    #[must_use]
    pub const fn new(members: usize) -> Self {
        Self(members)
    }
}

/// How many standable cells a zone has.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneCapacity(usize);

impl ZoneCapacity {
    /// Wrap a cell count.
    #[must_use]
    pub const fn new(cells: usize) -> Self {
        Self(cells)
    }
}

/// Why packing or deployment failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackingError {
    /// No prefab registered for this theme and role.
    NoPrefabForRole {
        /// Theme key.
        theme: ThemeUuid,
        /// Deployment role.
        role:  SpawnRole,
    },
    /// Prefab does not fit the region at this anchor.
    FootprintDoesNotFit {
        /// Anchor tried.
        anchor:    Anchor,
        /// Prefab size.
        footprint: Footprint,
        /// Available region.
        region:    RegionRect,
    },
    /// Player zone is smaller than the minimum side.
    PlayerFootprintTooSmall {
        /// Actual size.
        footprint: Footprint,
        /// Required minimum side.
        min_side:  MinPlayerSide,
    },
    /// Zone cannot stand the roster size.
    DeploymentZoneTooSmall {
        /// Anchor of the zone.
        anchor:   Anchor,
        /// Members that need cells.
        demand:   RosterDemand,
        /// Standable cells available.
        capacity: ZoneCapacity,
    },
    /// A combat side has zero roster members — battle setup must refuse.
    EmptySide {
        /// Which side had no members (`Player` or `Enemy`).
        side: SpawnRole,
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
            Self::EmptySide { side } => write!(
                f,
                "the {side:?} side roster is empty; battle setup refuses a side with no combatants",
            ),
        }
    }
}

impl std::error::Error for PackingError {}
