//! The GTW-541 (`AoE` CORE of GTW-41) splash pass — fan a non-`Single` round's
//! template onto the OTHER occupants it covers, through the SAME per-round ganger
//! fold as the direct target.

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

/// Splash a non-[`Single`](crate::weapon::HitType::Single) round's `AoE` template onto the
/// OTHER occupants the shape covers — the GTW-541 (`AoE` CORE of GTW-41) resolver-to-damage
/// integration.
///
/// Returns an EMPTY `Vec` for a [`HitType::Single`](crate::weapon::HitType::Single) round
/// WITHOUT calling the resolver or taking any RNG draw — so the single-target path is
/// identical (the GTW-541 identity property). Otherwise it enumerates the affected
/// `(cell, level)` set ([`aoe_affected`](crate::aoe::aoe_affected), from the primary
/// impact cell, the shooter origin, and the mode's [`HitType`](crate::weapon::HitType))
/// and, in that CANONICAL sorted order (so the seeded RNG stream is deterministic), routes
/// each cell's occupant through the EXISTING [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) damage path EXACTLY
/// ONCE — reusing [`fold_ganger_round`] verbatim (no duplicated damage math).
///
/// Faction-blind: the splash strikes EVERY occupant it finds, including the shooter's own
/// gang if the geometry covers them (`docs/combat/resolution.md` §2 — "any other actor in
/// the path — including your own gang — true friendly fire"; grenades do not discriminate).
/// The DIRECT-impact target already folded above (`primary`) is skipped so it is never
/// double-hit. A non-ganger occupancy slot (empty / cover only) contributes nothing.
///
/// Each splashed ganger takes one [`ShotRng`](crate::rng::ShotRng) draw (the §4 body-part
/// roll — the splash has no march-computed part) plus the fold's one
/// [`SeverityRng`](crate::rng::SeverityRng) + one [`InjuryRng`](crate::rng::InjuryRng) draw,
/// EXACTLY the direct-target cost — so the streams stay content-independent and stable.
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
    // The identity short-circuit: a Single round splashes nothing and takes NO draw.
    if matches!(hit_type, crate::weapon::HitType::Single) {
        return Vec::new();
    }

    // The DIRECT-impact target already folded (skip it so it is never double-hit).
    let primary_struck = match primary.kind {
        ShotKind::Ganger(e) => Some(e),
        _ => None,
    };

    // The impact + shooter cells the resolver keys the template off (the storey is the
    // impact's own — the 2D-on-level ruling). `Position` derefs to `CellLevel`.
    let impact = CellLevel::new(outcome.cell, outcome.level);
    let shooter_cell: CellLevel = *shooter_position;

    let affected = crate::aoe::aoe_affected(impact, hit_type, shooter_cell);
    let mut reports = Vec::new();
    for cell in affected {
        // Read the occupant — a faction-blind cell peek (friendly fire hits all).
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue; // empty / cover-only cell — nothing to strike
        };
        if Some(occupant) == primary_struck {
            continue; // the direct target already took its hit
        }
        // Roll the §4 body part for the splashed ganger (its ONE ShotRng draw — the
        // splash has no march-computed part), then synthesize a Ganger outcome AT the
        // splashed cell and fold it through the SAME per-round ganger path.
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
