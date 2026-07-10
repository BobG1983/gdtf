//! The typed [`PackingError`] — the no-panic, fail-closed procgen-abort contract
//! (GTW-424), mirroring [`BattleSetupError`](crate::situation::BattleSetupError).
//!
//! Every way the space-packing assembler can fail to produce a valid placement is a named
//! variant here. The assembler returns these in the `Err` arm of a [`Result`] — it NEVER
//! `unwrap`/`expect`/`panic`s. OQ-5 prefers REJECTING an over-large player footprint at
//! GTW-418 load time so runtime is always valid; these variants are the runtime
//! fail-closed backstop for the cases load-time rejection cannot cover (e.g. an empty
//! prefab registry, or a chosen footprint that does not fit). GTW-497 removed the OQ-4
//! disconnection variant: connectivity is by-construction via the 1-cell `default_floor`
//! seam, so there is no disconnection to report.

use super::{
    anchor::Anchor,
    geometry::{Footprint, MinPlayerSide, RegionRect},
};
use crate::level::{SpawnRole, ThemeUuid};

/// The typed ways the GTW-424 packer can fail — the no-panic, fail-closed procgen-abort
/// contract.
///
/// A named domain enum (no-bare-types: a packing failure is a domain value, not a bare
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
        }
    }
}

impl std::error::Error for PackingError {}
