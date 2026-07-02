//! The **blast-at-a-landing** resolver — [`resolve_blast`] fans a
//! [`HitType::Blast`](crate::weapon::HitType) template at a LANDING cell onto every ganger
//! the disc covers, routing each through the EXISTING GTW-541 damage path (GTW-546, child
//! GTW-41d of GTW-41).
//!
//! This is the throw's blast integration. A lobbed grenade's landing cell (from
//! [`march_arc`](crate::march::march_arc)) is the impact origin; [`resolve_blast`] enumerates
//! the affected `(cell, level)` set with the SAME pure resolver the fire path uses
//! ([`aoe_affected`](crate::aoe::aoe_affected)) and applies the weapon's damage to each
//! struck ganger through the SAME [`resolve_and_apply`] fold the fire path's
//! `apply_aoe_splash` routes through — no parallel damage math (GTW-546 C: "`AoE` damage
//! applied via the 41a resolver").
//!
//! Faction-blind: the blast strikes EVERY occupant it covers, including the thrower's own
//! gang (`docs/combat/resolution.md` §2 — grenades do not discriminate). Unlike the fire
//! path's splash, there is NO "primary" direct target to skip — the landing cell's own
//! occupant is just another blast cell. Each struck ganger takes exactly the same RNG draws
//! the splash costs (one [`ShotRng`] body-part roll + the fold's one
//! [`SeverityRng`] + one [`InjuryRng`] draw), so the stream cost is content-independent and
//! the affected set is resolved in the resolver's canonical sorted order for determinism.
//!
//! Pure model logic behind a query-based caller — no `&mut World`, no pixel.

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

/// Resolve a ganger's worn-armor piece **entity** for a struck body part — the
/// `ganger → Wears → the BodyPart-tagged piece` keyed lookup (GTW-323 / ADR-0004), mirroring
/// the fire path's `struck_piece_entity`. Returns `None` when the ganger has no `Wears`
/// collection or no piece tags the part (folds to bare flesh upstream).
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

/// Fan a [`HitType::Blast`] template at the `landing` cell onto every ganger the disc covers,
/// applying the grenade's damage through the EXISTING GTW-541 fold — the throw's blast
/// integration (GTW-546).
///
/// Enumerates the affected `(cell, level)` set with [`aoe_affected`] (the pure, RNG-free,
/// canonically-sorted resolver the fire path uses), then for each cell whose occupant is a
/// LIVE ganger: rolls the §4 body part (one [`ShotRng`] draw), synthesizes a
/// [`ShotKind::Ganger`] [`ShotOutcome`] AT that cell, and folds it through
/// [`resolve_and_apply`] onto the struck target (the SAME wound path the direct fire hit
/// takes) — reusing the grenade's [`WeaponStats`] borrow-view for the damage numbers. Returns
/// one [`HitReport`] per struck ganger, in the resolver's canonical order.
///
/// Faction-blind (friendly fire hits all) and no "primary" to skip — the landing cell's
/// occupant is folded like any other. A dead occupant is skipped (a corpse is not re-wounded);
/// an empty / cover-only cell contributes nothing. Non-[`Blast`](HitType::Blast) hit types are
/// treated as a radius-0 blast (the landing cell only) via the resolver — a grenade always
/// authors `Blast`, so this is the total path. `shooter_luck` is the thrower's Luck (the §6
/// score's nasty-wound term); `weapon` is the grenade's stats.
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
    let level = Level::new(u8::try_from(landing.z).unwrap_or(0));
    let affected = aoe_affected(landing, hit, thrower_cell);
    let mut reports = Vec::new();
    for cell in affected {
        // A faction-blind cell peek — an empty / cover-only cell contributes nothing.
        let Some(occupant) = grids.occupancy.occupant(&cell) else {
            continue;
        };
        // A corpse is not re-wounded (the fire path's dead-skip parity — a Dead occupant is
        // transparent). A missing target row folds to no-effect below, so guard on Dead only.
        if targets
            .get(occupant)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
        {
            continue;
        }
        // Roll the §4 body part (the ONE ShotRng draw per struck ganger — a blast has no
        // march-computed part), then synthesize a Ganger outcome AT the covered cell.
        let part = crate::hit_location::roll_body_part(&tuning.body_part_weights, shot_rng.rng());
        let struck_cell = Cell::new(cell.x, cell.y);
        let outcome = ShotOutcome {
            kind: ShotKind::Ganger(occupant),
            cell: struck_cell,
            level,
            body_part: Some(part),
            band: weapon_band(),
            // A lobbed blast has no meaningful muzzle / trajectory geometry for the fold (the
            // fold reads neither for a ganger wound); a zero-direction ShotDir + the cell centre
            // are inert placeholders that keep the outcome well-formed.
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

/// The clearance band a blast wound is recorded at — [`HeightBand::Low`](crate::cover::HeightBand),
/// the same the fire path's ground / splash arm records. A blast wound's band is not read by
/// the fold (the disc is a horizontal footprint, 2D-on-level), so any band is inert; `Low` is
/// the neutral choice.
const fn weapon_band() -> crate::cover::HeightBand {
    crate::cover::HeightBand::Low
}

/// The sim-unit centre of a covered cell — a well-formed [`SimPos`](crate::metric::SimPos)
/// placeholder muzzle for the synthesized outcome (the fold reads it only for structural
/// hits, never a ganger wound).
fn landing_point(cell: Cell, level: Level) -> crate::metric::SimPos {
    crate::metric::cell_center(cell, level)
}

/// Fold ONE blast-struck ganger onto its wound path — mirrors the fire path's
/// `fold_ganger_round`: resolves the struck location's worn piece via
/// `ganger → Wears → the BodyPart-tagged piece`, reads the defender's effective
/// Toughness / Luck over its injury ledger, and routes the whole thing through the SHARED
/// [`resolve_and_apply`]. A struck entity that is not a queryable target folds to
/// [`HitReport::no_effect`] — never a panic.
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
