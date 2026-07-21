//! The typed [`PackingError`] — the no-panic, fail-closed generation-abort contract
//! (GTW-424 / GTW-744), mirroring [`BattleSetupError`](crate::situation::BattleSetupError).
//!
//! Every way generation can fail to produce a valid placement is a named variant here —
//! the space-packing assembler's footprint failures (GTW-424) AND the GTW-744 roster
//! DEPLOYMENT failure (a zone too small to stand its whole roster). The assembler / deploy
//! step return these in the `Err` arm of a [`Result`] — they NEVER
//! `unwrap`/`expect`/`panic`. OQ-5 prefers REJECTING an over-large player footprint at
//! GTW-418 load time so runtime is always valid; these variants are the runtime
//! fail-closed backstop for the cases load-time rejection cannot cover (e.g. an empty
//! prefab registry, or a chosen footprint that does not fit). GTW-497 removed the OQ-4
//! disconnection variant: connectivity is by-construction via the 1-cell `default_floor`
//! seam, so there is no disconnection to report.

use bevy::prelude::Deref;

use super::{
    anchor::Anchor,
    geometry::{Footprint, MinPlayerSide, RegionRect},
};
use crate::level::{SpawnRole, ThemeUuid};

/// A **count of roster members** a deployment zone must stand (GTW-744) — the demand side
/// of the [`PackingError::DeploymentZoneTooSmall`] capacity check.
///
/// A named newtype over `usize` (no-bare-types: a member count is a domain quantity, not a
/// bare integer, and distinct from the capacity it is compared against). Private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterDemand(usize);

impl RosterDemand {
    /// Build a roster demand from its member count.
    #[must_use]
    pub const fn new(members: usize) -> Self {
        Self(members)
    }
}

/// A **count of standable cells** a deployment zone offers (GTW-744) — the capacity side of
/// the [`PackingError::DeploymentZoneTooSmall`] check.
///
/// A named newtype over `usize` (no-bare-types rule 3: distinct from [`RosterDemand`] even
/// over the same inner — a capacity is never a demand). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneCapacity(usize);

impl ZoneCapacity {
    /// Build a zone capacity from its standable-cell count.
    #[must_use]
    pub const fn new(cells: usize) -> Self {
        Self(cells)
    }
}

/// The typed ways generation can fail — the no-panic, fail-closed generation-abort
/// contract (GTW-424 packing + GTW-744 deployment).
///
/// A named domain enum (no-bare-types: a generation failure is a domain value, not a bare
/// string/`()`), the [`BattleSetupError`](crate::situation::BattleSetupError) precedent.
/// Each variant names the offending input so a caller can log exactly why generation
/// could not produce a valid level. Validated BEFORE any [`Situation`](crate::situation::Situation)
/// is emitted (the abort-first invariant the later GTW-431 emit step inherits), so a
/// failure never leaves a half-built level behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackingError {
    /// No prefab is registered for a `(theme, role)` the packer needs — the registry has
    /// no candidate fragment for this deployment role at this theme (across ANY footprint
    /// size). Fail-closed: generation cannot proceed without a player-spawn and an
    /// enemy-spawn prefab.
    NoPrefabForRole {
        /// The stable [`ThemeUuid`] that had no candidate for `role` (GTW-492).
        theme: ThemeUuid,
        /// The deployment role that had no candidate.
        role:  SpawnRole,
    },
    /// A chosen prefab's footprint does not fit the region it must occupy — the
    /// player-spawn prefab does not fit its anchor region, or the enemy-spawn prefab does
    /// not fit the strict-opposite region (C2). OQ-5 prefers rejecting an over-large
    /// player prefab at GTW-418 LOAD time, so this is the runtime backstop.
    FootprintDoesNotFit {
        /// The anchor the offending prefab was being placed at.
        anchor:    Anchor,
        /// The footprint that was too large.
        footprint: Footprint,
        /// The region it had to fit inside.
        region:    RegionRect,
    },
    /// A PLAYER-spawn prefab's footprint is below the OQ-5 minimum side (`~10x10`) — the
    /// deployment zone would be a cramped strip. OQ-5 prefers rejecting this at GTW-418
    /// load time; this is the runtime backstop.
    PlayerFootprintTooSmall {
        /// The undersized player footprint.
        footprint: Footprint,
        /// The minimum side it failed to clear (the OQ-5 floor, in cells).
        min_side:  MinPlayerSide,
    },
    /// A deployment zone cannot stand its whole roster (GTW-744) — the zone offers fewer
    /// STANDABLE cells (in-bounds, unblocked, unoccupied) than the roster has members to
    /// place. Fail-closed: [`deploy_rosters`](crate::procgen::deploy_rosters) returns this
    /// rather than dropping members or stacking them, so the app can abort setup (staying in
    /// Generation) rather than start an under-populated battle.
    DeploymentZoneTooSmall {
        /// The anchor of the zone that could not fit its roster.
        anchor:   Anchor,
        /// How many roster members needed placing in the zone.
        demand:   RosterDemand,
        /// How many standable cells the zone actually offered.
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
