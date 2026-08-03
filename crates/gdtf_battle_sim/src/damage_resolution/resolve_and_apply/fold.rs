//! Dispatch a coarse shot outcome to the correct apply path.

use bevy::prelude::Entity;

use crate::{
    ganger::Luck,
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    resolve_and_apply::{
        kinds,
        report::{HitReport, HitVerdict, StruckSurfaces, TargetGanger},
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::{InjuryRng, SeverityRng},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// Resolve a shot outcome and apply the results.
///
/// Routes to the ganger, cover, slab, or ground path based on the shot kind.
/// Misses produce a no-effect report.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "StruckSurfaces already groups cover/slab; TargetGanger groups the target; rest are distinct inputs"
)]
pub fn resolve_and_apply(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: Option<TargetGanger<'_>>,
    target_entity: Entity,
    surfaces: StruckSurfaces<'_>,
    tuning: &CombatTuning,
    rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    let verdict = match outcome.kind {
        ShotKind::Ganger(_) => kinds::ganger::fold(
            outcome,
            weapon,
            shooter_luck,
            target,
            target_entity,
            tuning,
            rng,
            tables,
            registry,
            injury_rng,
        ),
        ShotKind::Cover(entry) => kinds::cover::fold(
            &entry,
            CellLevel::new(outcome.cell, outcome.level),
            weapon,
            surfaces.cover,
            tuning,
        ),
        ShotKind::Slab(at) => kinds::slab::fold(at, weapon, surfaces.slab, tuning),
        ShotKind::Ground(at) => kinds::ground::fold(at, weapon),
        ShotKind::Miss => HitVerdict::NoEffect,
    };
    HitReport {
        kind: outcome.kind,
        verdict,
    }
}
