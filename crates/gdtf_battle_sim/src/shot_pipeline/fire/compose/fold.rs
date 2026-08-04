//! Fold a coarse outcome into a primary `HitReport`.

use bevy::prelude::Entity;

use super::{
    super::query::{BattleGrids, PieceQuery, StruckBodies, WearsQuery},
    snapshot::ShooterSnapshot,
};
use crate::{
    armor::BodyPart,
    resolve_and_apply::{
        HitReport, ShotSource, StruckPiece, StruckSurfaces, TargetGanger, WoundRoll,
        resolve_and_apply,
    },
    resolve_coarse::ShotKind,
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

/// Resolve the primary impact of a round into a `HitReport`.
pub(super) fn resolve_primary_report(
    outcome: &crate::resolve_coarse::ShotOutcome,
    snapshot: &ShooterSnapshot,
    grids: &mut BattleGrids,
    bodies: &mut StruckBodies,
    roll: &mut WoundRoll<'_>,
) -> HitReport {
    if let ShotKind::Ganger(struck) = outcome.kind {
        fold_ganger_round(outcome, struck, snapshot, grids, bodies, roll)
    } else {
        resolve_and_apply(
            outcome,
            ShotSource {
                weapon: snapshot.weapon_stats(),
                luck:   snapshot.luck,
            },
            None,
            Entity::PLACEHOLDER,
            StruckSurfaces {
                cover: grids.cover,
                slab:  grids.slab,
            },
            roll,
        )
    }
}

/// Apply damage to a living combatant hit by this round.
pub(super) fn fold_ganger_round(
    outcome: &crate::resolve_coarse::ShotOutcome,
    struck: Entity,
    snapshot: &ShooterSnapshot,
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
                ShotSource {
                    weapon: snapshot.weapon_stats(),
                    luck:   snapshot.luck,
                },
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
