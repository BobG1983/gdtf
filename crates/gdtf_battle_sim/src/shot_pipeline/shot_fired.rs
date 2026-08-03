//! Message announcing a single resolved shot for FX and listeners.

use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level, SimPos},
    resolve_and_apply::HitReport,
    resolve_coarse::{ShotKind, ShotOutcome},
    sample_cone::ShotDir,
    weapon::DamageType,
};

/// One shot that left the muzzle and reached an impact.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotFired {
    /// Shooter entity.
    pub shooter: Entity,
    /// Muzzle position.
    pub muzzle: SimPos,
    /// Trajectory direction.
    pub trajectory: ShotDir,
    /// Impact cell.
    pub impact_cell: Cell,
    /// Impact level.
    pub impact_level: Level,
    /// What was hit (or miss).
    pub kind: ShotKind,
    /// Damage type of the weapon.
    pub damage: DamageType,
    /// Optional full hit report after damage fold.
    pub report: Option<HitReport>,
}

impl ShotFired {
    /// From a coarse outcome without a hit report.
    #[must_use]
    pub const fn from_outcome(shooter: Entity, damage: DamageType, outcome: &ShotOutcome) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: None,
        }
    }

    /// From a coarse outcome plus resolved hit report.
    #[must_use]
    pub const fn from_round(
        shooter: Entity,
        damage: DamageType,
        outcome: &ShotOutcome,
        report: HitReport,
    ) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: Some(report),
        }
    }
}
