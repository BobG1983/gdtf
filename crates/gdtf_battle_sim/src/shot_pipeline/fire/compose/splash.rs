use super::{
    super::query::{BattleGrids, PieceQuery, TargetQuery, WearsQuery},
    fold::fold_ganger_round,
    snapshot::ShooterSnapshot,
};
use crate::{
    ganger::Position,
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    resolve_and_apply::HitReport,
    resolve_coarse::ShotKind,
    rng::{InjuryRng, SeverityRng, ShotRng},
    tuning::CombatTuning,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the splash pass needs the primary outcome / hit-type / shooter origin / the \
              already-folded primary report (to skip the direct target) / the shooter \
              snapshot / grids plus the disjoint wears+pieces queries + tuning + the three \
              distinct RNG streams (shot / severity / injury) + the injury tables/registry; \
              this is the same irreducible set fold_ganger_round documents, plus the \
              `AoE`-specific outcome/hit-type/origin/primary inputs"
)]
pub(super) fn apply_aoe_splash(
    outcome: &crate::resolve_coarse::ShotOutcome,
    hit_type: crate::weapon::HitType,
    shooter_position: Position,
    primary: &HitReport,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> Vec<HitReport> {
    if matches!(hit_type, crate::weapon::HitType::Single) {
        return Vec::new();
    }

    let primary_struck = match primary.kind {
        ShotKind::Ganger(e) => Some(e),
        _ => None,
    };

    let impact = CellLevel::new(outcome.cell, outcome.level);
    let shooter_cell: CellLevel = *shooter_position;

    let affected = crate::aoe::aoe_affected(impact, hit_type, shooter_cell);
    let mut reports = Vec::new();
    for cell in affected {
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue; 
        };
        if Some(occupant) == primary_struck {
            continue; 
        }
        let part = crate::hit_location::roll_body_part(&tuning.body_part_weights, shot_rng.rng());
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
        let report = fold_ganger_round(
            &splash_outcome,
            occupant,
            snapshot,
            grids,
            targets,
            wears,
            pieces,
            tuning,
            severity_rng,
            tables,
            registry,
            injury_rng,
        );
        reports.push(report);
    }
    reports
}
