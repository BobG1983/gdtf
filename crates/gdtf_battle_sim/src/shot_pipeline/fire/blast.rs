//! Resolve an area blast (grenade / splash) against all affected cells.

use bevy::prelude::Entity;

use super::query::{BattleGrids, PieceQuery, StruckBodies, WearsQuery};
use crate::{
    aoe::aoe_affected,
    ganger::LifeState,
    metric::{Cell, CellLevel, Level},
    resolve_and_apply::{
        HitReport, ShotSource, StruckPiece, StruckSurfaces, TargetGanger, WoundRoll,
        resolve_and_apply,
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::ShotRng,
    sample_cone::ShotDir,
    weapon::HitType,
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

/// Where a blast lands, who threw it, and the pattern it throws.
#[derive(Debug, Clone, Copy)]
pub struct BlastFootprint {
    /// Cell and level the blast lands on.
    pub landing: CellLevel,
    /// Cell and level the throw came from.
    pub thrower: CellLevel,
    /// Blast pattern the weapon throws.
    pub hit:     HitType,
}

/// Apply blast damage to every living occupant in the AOE footprint.
pub fn resolve_blast(
    footprint: BlastFootprint,
    source: ShotSource<'_>,
    grids: &mut BattleGrids,
    bodies: &mut StruckBodies,
    shot_rng: &mut ShotRng,
    roll: &mut WoundRoll<'_>,
) -> Vec<HitReport> {
    let level = footprint.landing.level();
    let affected = aoe_affected(footprint.landing, footprint.hit, footprint.thrower);
    let mut reports = Vec::new();
    for cell in affected {
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue;
        };
        if bodies
            .targets
            .get(occupant)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
        {
            continue;
        }
        let part =
            crate::hit_location::roll_body_part(&roll.tuning.body_part_weights, shot_rng.rng());
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
        let report = fold_blast_ganger(&outcome, occupant, source, grids, bodies, roll);
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

fn fold_blast_ganger(
    outcome: &ShotOutcome,
    struck: Entity,
    source: ShotSource<'_>,
    grids: &mut BattleGrids,
    bodies: &mut StruckBodies,
    roll: &mut WoundRoll<'_>,
) -> HitReport {
    let struck_part = outcome
        .body_part
        .and_then(|part| struck_piece_entity(struck, part, &bodies.wears, &bodies.pieces));
    let struck_piece_view = struck_part.and_then(|piece_entity| {
        bodies
            .pieces
            .get_mut(piece_entity)
            .ok()
            .map(|piece| StruckPiece {
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
    });

    match bodies.targets.get_mut(struck) {
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
                source,
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
                roll,
            )
        }
        Err(_) => HitReport::no_effect(outcome.kind),
    }
}
