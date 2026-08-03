//! Fold a coarse outcome into a primary HitReport.

use bevy::prelude::Entity;

use super::{
    super::query::{BattleGrids, PieceQuery, TargetQuery, WearsQuery},
    snapshot::ShooterSnapshot,
};
use crate::{
    armor::BodyPart,
    injuries::{InjuryRegistry, InjuryTables},
    resolve_and_apply::{HitReport, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply},
    resolve_coarse::ShotKind,
    rng::{InjuryRng, SeverityRng},
    tuning::CombatTuning,
};

fn struck_piece_entity(
    ganger: Entity,
    part: BodyPart,
    wears: &WearsQuery,
    pieces: &PieceQuery,
) -> Option<Entity> {
    let worn = wears.get(ganger).ok()?;
    worn.pieces()
        .find(|&piece| pieces.get(piece).is_ok_and(|p| *p.part == part))
}

/// Resolve the primary impact of a round into a HitReport.
#[expect(
    clippy::too_many_arguments,
    reason = "this is the exact irreducible fold set resolve_round passed inline before \
              GTW-541 (outcome / snapshot / grids + the disjoint wears/pieces queries + \
              tuning + the severity/injury RNG streams + the injury tables/registry); \
              bundling the queries would obscure the GTW-323 disjointness the ParamSet-free \
              coexistence relies on — the same reason fold_ganger_round documents"
)]
pub(super) fn resolve_primary_report(
    outcome: &crate::resolve_coarse::ShotOutcome,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    if let ShotKind::Ganger(struck) = outcome.kind {
        fold_ganger_round(
            outcome,
            struck,
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
        )
    } else {
        resolve_and_apply(
            outcome,
            snapshot.weapon_stats(),
            snapshot.luck,
            None,
            Entity::PLACEHOLDER,
            StruckSurfaces {
                cover: grids.cover,
                slab: grids.slab,
            },
            tuning,
            severity_rng,
            tables,
            registry,
            injury_rng,
        )
    }
}

/// Apply damage to a living combatant hit by this round.
#[expect(
    clippy::too_many_arguments,
    reason = "the ganger fold needs the outcome / struck entity / snapshot / grids plus \
              the disjoint wears+pieces queries + tuning + severity-rng, and GTW-438 adds \
              the injury-roll inputs (InjuryTables + InjuryRegistry + the &mut InjuryRng \
              draw stream); bundling the queries would obscure the GTW-323 disjointness \
              the ParamSet-free coexistence relies on"
)]
pub(super) fn fold_ganger_round(
    outcome: &crate::resolve_coarse::ShotOutcome,
    struck: Entity,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    targets: &mut TargetQuery,
    wears: &WearsQuery,
    pieces: &mut PieceQuery,
    tuning: &CombatTuning,
    severity_rng: &mut SeverityRng,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    injury_rng: &mut InjuryRng,
) -> HitReport {
    let struck_piece_view = outcome
        .body_part
        .and_then(|part| struck_piece_entity(struck, part, wears, pieces))
        .and_then(|piece_entity| {
            pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                floor: *piece.floor,
                protection: *piece.protection,
                hardness: *piece.hardness,
                armor_type: *piece.armor_type,
                integrity: piece.integrity.into_inner(),
            })
        });

    match targets.get_mut(struck) {
        Ok((mut hp, mut wounds, mut life, mut inflicted, toughness, target_luck, injuries)) => {
            let (effective_toughness, effective_luck) = match injuries {
                Some(ledger) => (
                    crate::ganger::effective_toughness(*toughness, ledger),
                    crate::ganger::effective_luck(*target_luck, ledger),
                ),
                None => (*toughness, *target_luck),
            };
            resolve_and_apply(
                outcome,
                snapshot.weapon_stats(),
                snapshot.luck,
                Some(TargetGanger {
                    hp: &mut hp,
                    wounds: &mut wounds,
                    life: &mut life,
                    piece: struck_piece_view,
                    inflicted: &mut inflicted,
                    toughness: effective_toughness,
                    luck: effective_luck,
                }),
                struck,
                StruckSurfaces {
                    cover: grids.cover,
                    slab: grids.slab,
                },
                tuning,
                severity_rng,
                tables,
                registry,
                injury_rng,
            )
        }
        Err(_) => HitReport::no_effect(outcome.kind),
    }
}
