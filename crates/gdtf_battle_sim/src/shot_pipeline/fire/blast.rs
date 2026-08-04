//! Resolve an area blast (grenade / splash) against all affected cells.

use bevy::prelude::Entity;

use super::query::{BattleGrids, PieceQuery, TargetQuery, WearsQuery};
use crate::{
    aoe::aoe_affected,
    ganger::{LifeState, Luck},
    injuries::{InjuryRegistry, InjuryTables},
    metric::{Cell, CellLevel, Level},
    resolve_and_apply::{HitReport, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::{InjuryRng, SeverityRng, ShotRng},
    sample_cone::ShotDir,
    tuning::CombatTuning,
    weapon::{HitType, WeaponStats},
};

fn struck_piece_entity(
    ganger: Entity,
    part: crate::armor::BodyPart,
    wears: &WearsQuery,
    pieces: &PieceQuery,
) -> Option<Entity> {
    let worn = wears.get(ganger).ok()?;
    worn.pieces()
        .find(|&piece| pieces.get(piece).is_ok_and(|p| *p.part == part))
}

/// Apply blast damage to every living occupant in the AOE footprint.
#[expect(
    clippy::too_many_arguments,
    reason = "the blast fan needs the landing / thrower cells + the weapon stats + hit type + \
              shooter luck + the disjoint target/wears/pieces queries + grids + tuning + the \
              three distinct RNG streams (shot / severity / injury) + the injury tables & \
              registry — the same irreducible set the fire path's apply_aoe_splash documents, \
              minus the primary report (a throw has no direct-impact target to skip)"
)]
pub fn resolve_blast(
    landing: CellLevel,
    thrower_cell: CellLevel,
    weapon: WeaponStats<'_>,
    hit: HitType,
    shooter_luck: Luck,
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
    let level = landing.level();
    let affected = aoe_affected(landing, hit, thrower_cell);
    let mut reports = Vec::new();
    for cell in affected {
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue;
        };
        if targets
            .get(occupant)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
        {
            continue;
        }
        let part = crate::hit_location::roll_body_part(&tuning.body_part_weights, shot_rng.rng());
        let struck_cell = cell.cell();
        let outcome = ShotOutcome {
            kind: ShotKind::Ganger(occupant),
            cell: struck_cell,
            level,
            body_part: Some(part),
            band: weapon_band(),
            muzzle: landing_point(struck_cell, level),
            trajectory: ShotDir::from_direction(bevy::math::Vec3::ZERO),
        };
        let report = fold_blast_ganger(
            &outcome,
            occupant,
            weapon,
            shooter_luck,
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

const fn weapon_band() -> crate::cover::HeightBand {
    crate::cover::HeightBand::Low
}

fn landing_point(cell: Cell, level: Level) -> crate::metric::SimPos {
    crate::metric::cell_center(cell, level)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the ganger fold needs the outcome / struck entity / weapon / shooter luck + the \
              disjoint wears/pieces queries + grids + tuning + severity-rng + the injury tables \
              & registry & injury-rng — the exact irreducible set the fire path's \
              fold_ganger_round documents"
)]
fn fold_blast_ganger(
    outcome: &ShotOutcome,
    struck: Entity,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
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
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
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
                weapon,
                shooter_luck,
                Some(TargetGanger {
                    hp:        &mut hp,
                    wounds:    &mut wounds,
                    life:      &mut life,
                    piece:     struck_piece_view,
                    inflicted: &mut inflicted,
                    toughness: effective_toughness,
                    luck:      effective_luck,
                }),
                struck,
                StruckSurfaces {
                    cover: grids.cover,
                    slab:  grids.slab,
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
