//! The shared **shove-application** helper (GTW-525 C1) — [`apply_shove`], which folds one
//! resolved [`ShoveOutcome`] onto the target: rewrites its [`Position`] and, on a fall, routes
//! the drop through the SHARED GTW-523 fall-damage fork ([`resolve_fall_hit`]) and emits a
//! [`FallOccurred`] (+ any rolled [`InjuryInflicted`]).
//!
//! This is the ONE place a shove's effect is applied, shared by all three shove trigger sites
//! (the deliberate act + the two weapon-tag auto-shove hooks): the shove itself is pure
//! displacement (no wound), so a [`ShoveOutcome::Moved`] only moves the target, and a
//! [`ShoveOutcome::Fell`] moves it AND runs the exact same weight-free fall-damage synthesis
//! `apply_falls` runs (REUSING `resolve_fall_hit` — NO reimplemented drop / fall-damage path).
//! The shove draws NO RNG; only the fall does, via the two GTW-523 streams threaded here.

use bevy::prelude::{Entity, MessageWriter};

use super::verb::ShoveOutcome;
use crate::{
    acts::InjuryInflicted,
    armor::{BodyPart, PieceArmorMut, Wears},
    falls::{FallImpact, FallOccurred, FallWoundEnv, resolve_fall_hit},
    ganger::{Hp, LifeState, Luck, Position, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{InjuryRng, SeverityRng},
    tuning::CombatTuning,
};

/// The target's mutable battle surfaces + read attribute stats a fall fold writes/reads — the
/// exact set [`apply_falls`](crate::falls::apply_falls) folds a fall onto, borrowed by `&mut`
/// so [`apply_shove`] can rewrite the [`Position`] and (on a fall) route the drop through the
/// shared fall-damage fork.
///
/// A transparent borrow bundle of the named ganger components (no bare types), grouped so
/// [`apply_shove`] stays under clippy's argument-count gate (the [`FallImpact`] /
/// `TargetGanger` grouping precedent). Mutable on `Position` (the shove/fall write) +
/// `Hp` / `Wounds` / `LifeState` / `InflictedWounds` (the fall fold), read-only on
/// `Toughness` / `Luck` (the §6 severity inputs).
pub(crate) struct ShoveTargetSurfaces<'a> {
    /// The target's [`Position`] — rewritten to the shove destination (and, on a fall, to the
    /// landing storey) as ONE involuntary write; `Changed<Position>` drives the occupancy
    /// teardown automatically (the `apply_falls` precedent).
    pub position:  &'a mut Position,
    /// The target's `&mut Hp` — the fall fold's HP loss.
    pub hp:        &'a mut Hp,
    /// The target's `&mut Wounds` — the fall fold's §6 wound spend.
    pub wounds:    &'a mut Wounds,
    /// The target's `&mut LifeState` — the fall fold's down/kill transition.
    pub life:      &'a mut LifeState,
    /// The target's `&mut InflictedWounds` — the fall fold's wound ledger.
    pub inflicted: &'a mut InflictedWounds,
    /// The target's [`Toughness`] — the §6 severity mitigation.
    pub toughness: Toughness,
    /// The target's [`Luck`] — the §6 defender floor-extend.
    pub luck:      Luck,
}

/// The world content + streams a shove's FALL reads — the [`CombatTuning`] (the per-storey
/// magnitude + §6 scaling), the injury [`InjuryTables`] / [`InjuryRegistry`] pool, and the two
/// seeded draw streams the §6 severity / §8 injury roll advance.
///
/// Mirrors [`FallWoundEnv`] (the [`apply_falls`](crate::falls::apply_falls) fall environment)
/// so the shove-triggered fall runs the IDENTICAL synthesis — no drift. A named bundle so
/// [`apply_shove`] stays under the argument-count gate; the two streams are `&mut` (advanced by
/// the fold), the rest read-only. The shove itself draws NO RNG — only the fall does, through
/// these two streams.
pub(crate) struct ShoveFallEnv<'a> {
    /// The shared combat tuning — the §Falls per-storey magnitude + §6 scaling + wound costs.
    pub tuning:       &'a CombatTuning,
    /// The shared weighted `(part, severity)` injury tables — the §8 roll's pool (AS-IS).
    pub tables:       &'a InjuryTables,
    /// The injury registry — resolves the §8 roll's picked key to its authored def.
    pub registry:     &'a InjuryRegistry,
    /// The §6 severity-roll stream — one draw per shoved-off-a-ledge faller.
    pub severity_rng: &'a mut SeverityRng,
    /// The §8 injury-roll stream — one draw per such faller with a non-graze / non-fatal wound.
    pub injury_rng:   &'a mut InjuryRng,
}

/// Apply one resolved [`ShoveOutcome`] onto the target (GTW-525 C1) — the SHARED shove effect
/// all three trigger sites route through.
///
/// - [`ShoveOutcome::Blocked`] — nothing (a no-op shove into a solid); returns immediately.
/// - [`ShoveOutcome::Moved`] — rewrite the target's [`Position`] to `dest` (a lateral push, no
///   fall, no wound). `Changed<Position>` drives the occupancy re-sync next frame.
/// - [`ShoveOutcome::Fell`] — rewrite the [`Position`] to `(dest.cell, landing.landing)` (the
///   landing storey) as ONE involuntary write, then run the weight-free fall-damage synthesis
///   through the SHARED [`resolve_fall_hit`] fork (the EXACT `apply_falls` fold: the struck part
///   is [`BodyPart::Torso`], armor honored via the target's first worn piece, one
///   [`SeverityRng`] draw + one [`InjuryRng`] draw on a non-graze / non-fatal wound), bridge any
///   rolled injury into the EXISTING [`InjuryInflicted`] message, and emit a [`FallOccurred`].
///
/// `wears` / `pieces` resolve the faller's struck worn piece (`target → Wears → the piece`, the
/// `apply_falls` precedent); a target wearing nothing folds to bare flesh. Fail-closed: a
/// missing component simply skips (no panic). Param-only (`bevy-traps.md` #7 — no `&mut World`).
#[expect(
    clippy::too_many_arguments,
    reason = "the shared shove-apply helper threads the resolved outcome, the target's mutable \
              surfaces + read stats (ShoveTargetSurfaces), the target entity, the two armor \
              relationship queries (the struck-piece resolution), the fall environment + \
              streams (ShoveFallEnv), and the two output writers — the irreducible fall-fold \
              access set (the resolve_fall_hit / apply_falls precedent); bundling further would \
              only hide the access set"
)]
pub(crate) fn apply_shove(
    outcome: ShoveOutcome,
    surfaces: ShoveTargetSurfaces<'_>,
    target_entity: Entity,
    wears: &bevy::prelude::Query<&Wears>,
    pieces: &mut bevy::prelude::Query<PieceArmorMut, bevy::prelude::With<crate::armor::WornBy>>,
    env: ShoveFallEnv<'_>,
    fell: &mut MessageWriter<FallOccurred>,
    injuries: &mut MessageWriter<InjuryInflicted>,
) {
    let ShoveTargetSurfaces {
        position,
        hp,
        wounds,
        life,
        inflicted,
        toughness,
        luck,
    } = surfaces;

    match outcome {
        // No legal cell to push into — never shove into a solid.
        ShoveOutcome::Blocked => {}

        // A lateral push onto a supported cell — move only, no fall, no wound.
        ShoveOutcome::Moved { dest } => {
            *position = Position::new(dest);
        }

        // A push off a ledge — move to the destination, then fall via the shared GTW-523 fork.
        ShoveOutcome::Fell { dest, landing } => {
            // The canonical CellLevel accessors (GTW-565): the storey the target fell
            // FROM and the ground cell it was pushed onto.
            let start = dest.level();
            let dest_cell = dest.cell();

            // ONE involuntary write to the LANDING storey — Changed<Position> drives the
            // occupancy re-sync (the apply_falls precedent — the grid is never hand-edited).
            *position = Position::new(CellLevel::new(dest_cell, landing.landing));

            // Resolve the struck worn piece — `target → Wears → the first worn piece` (the
            // apply_falls precedent). A faller wearing nothing folds to bare flesh. The fall
            // lands on the whole body -> Torso (deterministic, no body-part draw).
            let piece_view = wears
                .get(target_entity)
                .ok()
                .and_then(|worn| worn.pieces().next())
                .and_then(|piece_entity| {
                    pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                        floor:      *piece.floor,
                        protection: *piece.protection,
                        hardness:   *piece.hardness,
                        armor_type: *piece.armor_type,
                        integrity:  piece.integrity.into_inner(),
                    })
                });

            // The SHARED weight-free fall-damage synthesis (REUSES resolve_fall_hit verbatim —
            // the SAME fork apply_falls calls; NO reimplemented drop / fall damage). One
            // SeverityRng draw + one InjuryRng draw on a non-graze / non-fatal wound.
            let rolled = resolve_fall_hit(
                FallImpact {
                    per_storey: env.tuning.per_storey_damage,
                    storeys: landing.storeys,
                    part: BodyPart::Torso,
                    target: TargetGanger {
                        hp,
                        wounds,
                        life,
                        piece: piece_view,
                        inflicted,
                        toughness,
                        luck,
                    },
                    target_entity,
                },
                FallWoundEnv {
                    tuning:       env.tuning,
                    tables:       env.tables,
                    registry:     env.registry,
                    severity_rng: env.severity_rng,
                    injury_rng:   env.injury_rng,
                },
            );

            // Bridge a rolled injury into the EXISTING InjuryInflicted message (the apply_falls
            // bridge precedent), addressed to the shoved faller.
            if let Some(rolled) = rolled {
                injuries.write(InjuryInflicted::from_rolled(target_entity, rolled));
            }
            // Emit the FallOccurred signal — the presenter's fall FX / log reads it exactly as
            // it reads a slab-destroy fall (the shove-off-a-ledge fall is a real fall).
            fell.write(FallOccurred::new(
                target_entity,
                start,
                landing.landing,
                landing.storeys,
            ));
        }
    }
}
