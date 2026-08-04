//! Dispatch a coarse shot outcome to the correct apply path.

use bevy::prelude::Entity;

use crate::{
    metric::CellLevel,
    resolve_and_apply::{
        kinds,
        report::{HitReport, HitVerdict, ShotSource, StruckSurfaces, TargetGanger, WoundRoll},
    },
    resolve_coarse::{ShotKind, ShotOutcome},
};

/// Resolve a shot outcome and apply the results. Routes to the ganger, cover, slab, or ground path based on the shot kind.
/// Misses produce a no-effect report.
#[must_use]
pub fn resolve_and_apply(
    outcome: &ShotOutcome,
    source: ShotSource<'_>,
    target: Option<TargetGanger<'_>>,
    target_entity: Entity,
    surfaces: StruckSurfaces<'_>,
    roll: &mut WoundRoll<'_>,
) -> HitReport {
    let verdict = match outcome.kind {
        ShotKind::Ganger(_) => kinds::ganger::fold(outcome, source, target, target_entity, roll),
        ShotKind::Cover(entry) => kinds::cover::fold(
            &entry,
            CellLevel::new(outcome.cell, outcome.level),
            source.weapon,
            surfaces.cover,
            roll.tuning,
        ),
        ShotKind::Slab(at) => kinds::slab::fold(at, source.weapon, surfaces.slab, roll.tuning),
        ShotKind::Ground(at) => kinds::ground::fold(at, source.weapon),
        ShotKind::Miss => HitVerdict::NoEffect,
    };
    HitReport {
        kind: outcome.kind,
        verdict,
    }
}
