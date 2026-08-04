//! Apply AOE splash damage around a primary impact.

use super::{
    super::query::{BattleGrids, StruckBodies},
    fold::fold_ganger_round,
    snapshot::ShooterSnapshot,
};
use crate::{
    metric::CellLevel,
    resolve_and_apply::{HitReport, WoundRoll},
    resolve_coarse::ShotKind,
    rng::ShotRng,
};

/// The direct hit an area blast radiates from.
#[derive(Debug, Clone, Copy)]
pub(in crate::shot_pipeline::fire) struct PrimaryImpact<'a> {
    /// Coarse outcome of the direct hit.
    pub(in crate::shot_pipeline::fire) outcome:  &'a crate::resolve_coarse::ShotOutcome,
    /// Blast pattern the weapon throws.
    pub(in crate::shot_pipeline::fire) hit_type: crate::weapon::HitType,
    /// Report already folded for the direct target.
    pub(in crate::shot_pipeline::fire) report:   &'a HitReport,
}

/// Damage every living occupant in the AOE footprint except the primary target.
pub(super) fn apply_aoe_splash(
    impact: PrimaryImpact<'_>,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    bodies: &mut StruckBodies,
    shot_rng: &mut ShotRng,
    roll: &mut WoundRoll<'_>,
) -> Vec<HitReport> {
    let PrimaryImpact {
        outcome,
        hit_type,
        report: primary,
    } = impact;
    if matches!(hit_type, crate::weapon::HitType::Single) {
        return Vec::new();
    }

    let primary_struck = match primary.kind {
        ShotKind::Ganger(e) => Some(e),
        _ => None,
    };

    let at = CellLevel::new(outcome.cell, outcome.level);
    let shooter_cell: CellLevel = *snapshot.position;

    let affected = crate::aoe::aoe_affected(at, hit_type, shooter_cell);
    let mut reports = Vec::new();
    for cell in affected {
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue;
        };
        if Some(occupant) == primary_struck {
            continue;
        }
        let part =
            crate::hit_location::roll_body_part(&roll.tuning.body_part_weights, shot_rng.rng());
        let (splash_cell, splash_level) = (cell.cell(), outcome.level);
        let splash_outcome = crate::resolve_coarse::ShotOutcome {
            kind:       ShotKind::Ganger(occupant),
            cell:       splash_cell,
            level:      splash_level,
            body_part:  Some(part),
            band:       outcome.band,
            muzzle:     outcome.muzzle,
            trajectory: outcome.trajectory,
        };
        let report = fold_ganger_round(&splash_outcome, occupant, snapshot, grids, bodies, roll);
        reports.push(report);
    }
    reports
}
