//! The impact fold onto struck targets — target resolution at the query boundary,
//! handing every outcome to the ONE [`resolve_and_apply`] delegation dispatch.

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

/// Resolve a struck ganger's worn-armor piece **entity** for a struck [`BodyPart`] —
/// the `ganger → Wears → the BodyPart-tagged piece` keyed lookup (GTW-323 / ADR-0004).
///
/// Reads the ganger's [`Wears`](crate::armor::Wears) collection (read-only,
/// `wears.get(ganger)`), iterates its related piece entities, and returns the one
/// tagged with `part` — keyed access (NOT order-dependent), so the looked-up piece is
/// identical regardless of entity storage / spawn order (the determinism property of
/// ADR-0004). Returns `None` when the ganger has no `Wears` collection or no piece
/// tags `part` (folds to bare flesh upstream). The `pieces` query is borrowed
/// immutably here only to read each candidate's [`BodyPart`] tag; the caller re-borrows
/// it mutably to wear the resolved piece.
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

/// Fold the round's PRIMARY (direct-impact) outcome into its [`HitReport`] — the
/// CALL-BOUNDARY half of the fold (GTW-573 C3): the per-kind dispatch itself lives in
/// exactly ONE place, [`resolve_and_apply`]'s delegation match — this verb no longer
/// stacks a second copy of it. All that remains here is TARGET RESOLUTION, which is
/// query work and therefore stays at the boundary (the GTW-323 disjointness rationale
/// — queries are never bundled into the fold):
///
/// - a [`ShotKind::Ganger`] outcome needs the struck ganger's borrowed views resolved
///   off the disjoint queries first → [`fold_ganger_round`] (which assembles the
///   [`TargetGanger`] and calls [`resolve_and_apply`]);
/// - every other outcome (cover / slab / ground / miss) has no ganger to resolve →
///   [`resolve_and_apply`] directly with a `None` target ([`Entity::PLACEHOLDER`]).
///   The structural arms spend their [`StruckSurfaces`] ledger HP (RNG-free); a miss
///   folds to no effect inside the ONE dispatch (no draw, no mutation).
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
    // Target resolution ONLY (not a kind dispatch — resolve_and_apply owns that): a
    // struck ganger's mutable views must be borrowed off the disjoint queries before
    // the fold runs; nothing else needs a query.
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
        // No struck ganger to resolve (cover / slab / ground / miss): hand straight to
        // the ONE delegation dispatch. The cover ledger is reborrowed `&mut` here (its
        // earlier `&` reborrow by resolve_coarse / cone_for / stability_for has ended);
        // the structural arms are RNG-free, so the streams' cursors never advance.
        resolve_and_apply(
            outcome,
            snapshot.weapon_stats(),
            snapshot.luck,
            None,
            Entity::PLACEHOLDER,
            StruckSurfaces {
                cover: grids.cover,
                slab:  grids.slab,
            },
            tuning,
            severity_rng,
            // GTW-438: a structural / miss round rolls NO injury — the tables/registry
            // are unread and the InjuryRng cursor never advances on these arms.
            tables,
            registry,
            injury_rng,
        )
    }
}

/// Fold a [`ShotKind::Ganger`] round onto the struck target — the wound arm of
/// [`resolve_round`](super::round::resolve_round), split out (GTW-365) so the per-round verb stays under clippy's
/// line cap once the slab arm joined the cover arm.
///
/// GTW-323 / ADR-0004: resolves the struck location's worn piece ENTITY via
/// `ganger → Wears → the BodyPart-tagged piece`, reads its stats + wears its
/// `&mut ArmorIntegrity` through the fold. The lookup keys on the §4 struck part (carried
/// on `outcome`); a missing part / piece folds to bare flesh (`StruckPiece == None`). The
/// `wears` / `pieces` queries are disjoint from `targets`, so they coexist with the
/// `targets.get_mut(struck)`. A struck entity that is not a queryable target folds to
/// [`HitReport::no_effect`] — never a panic. The [`StruckSurfaces`] bundle is threaded so
/// the SAME [`resolve_and_apply`] signature serves both arms (a ganger hit touches
/// neither ledger).
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
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
        });

    match targets.get_mut(struck) {
        Ok((mut hp, mut wounds, mut life, mut inflicted, toughness, target_luck, injuries)) => {
            // GTW-436: route the defender's Toughness + Luck through the gate-enforced
            // effective accessors over its injury ledger (an absent ledger = the
            // zero-delta identity), so a `Modify(Toughness)` / `Modify(Luck)` injury
            // shifts the §6 severity roll in step with the derived stats. This is the
            // SINGLE direct-read path for the defender's Toughness / Luck.
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
