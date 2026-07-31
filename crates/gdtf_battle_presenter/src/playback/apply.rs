//! SHOWING one act: writing its drawn state and emitting its `Played<M>` (GTW-727 C16 /
//! C17 / C20).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry},
    acts::{
        FireDeclaration, MeleeResolved, MeleeStruck, MoveRejected, MovementOccurred, ReloadResult,
        ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

use super::{
    cursor::ActHold,
    drawn::{DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition, DrawnVitals},
    dwell::PlaybackTuning,
    emit::{Played, PlayedSignals},
};

/// The `Drawn*` mirrors the cursor writes, bundled into ONE [`SystemParam`].
///
/// Each query writes a DISTINCT component type, so they are mutually disjoint and need no
/// [`ParamSet`](bevy::prelude::ParamSet). Every write goes through `set_if_neq`, so
/// re-applying an unchanged value does not trip change detection and therefore does not
/// make a mirror repaint for nothing.
#[derive(SystemParam)]
pub struct DrawnWriters<'w, 's> {
    /// The drawn `(cell, level)` of each ganger.
    pub(super) positions: Query<'w, 's, &'static mut DrawnPosition>,
    /// The drawn posture of each ganger.
    pub(super) poses:     Query<'w, 's, &'static mut DrawnPose>,
    /// The drawn life state of each ganger.
    pub(super) lives:     Query<'w, 's, &'static mut DrawnLife>,
    /// The drawn vitals of each ganger.
    pub(super) vitals:    Query<'w, 's, &'static mut DrawnVitals>,
    /// The drawn magazine of each wielded weapon.
    pub(super) magazines: Query<'w, 's, &'static mut DrawnMagazine>,
}

/// SHOW one act-log entry: write whatever drawn state it settled, emit its `Played<M>`, and
/// return the hold to serve before the next entry.
///
/// The dwell is read HERE, from the live [`PlaybackTuning`], at the moment the hold begins —
/// never captured when the entry was recorded. That is what keeps a later skip /
/// fast-forward affordance a multiplier applied at this one read rather than a restructure.
///
/// A reaction-fire declaration takes its own longer beat: an interrupt the player did not
/// order needs a moment to register as its own event rather than as part of the act it
/// interrupted — which is the whole complaint this ticket exists to fix.
#[expect(
    clippy::too_many_lines,
    reason = "this is ONE exhaustive match over the act vocabulary — a flat dispatch table \
              with a two-to-four line arm each. Splitting it would scatter the deed→beat \
              mapping across files, which is the one thing a reader most needs to see \
              whole; the compiler's exhaustiveness check over it is the completeness \
              guarantee"
)]
pub(super) fn show_entry(
    entry: &ActEntry,
    tuning: &PlaybackTuning,
    drawn: &mut DrawnWriters,
    played: &mut PlayedSignals,
) -> ActHold {
    let actor = entry.actor();
    match entry.deed() {
        ActDeed::TurnBegan { now_active } => {
            played
                .turn
                .write(Played::new(TurnStarted::new(*now_active)));
            ActHold::timed(*tuning.turn_beat_seconds)
        }
        ActDeed::PostureChanged { pose } => {
            if let Ok(mut drawn_pose) = drawn.poses.get_mut(actor) {
                drawn_pose.set_if_neq(DrawnPose::new(*pose));
            }
            ActHold::timed(*tuning.posture_seconds)
        }
        ActDeed::Stepped { from, to, position } => {
            if let Ok(mut drawn_position) = drawn.positions.get_mut(actor) {
                drawn_position.set_if_neq(DrawnPosition::new(*position));
            }
            played
                .step
                .write(Played::new(MovementOccurred::new(actor, *from, *to)));
            ActHold::timed(*tuning.step_seconds)
        }
        ActDeed::MovedTo { position } => {
            if let Ok(mut drawn_position) = drawn.positions.get_mut(actor) {
                drawn_position.set_if_neq(DrawnPosition::new(*position));
            }
            ActHold::timed(*tuning.step_seconds)
        }
        ActDeed::MoveRefused { reason } => {
            played
                .refusal
                .write(Played::new(MoveRejected::new(actor, *reason)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::Fired {
            target,
            mode,
            rounds,
        } => {
            played.declaration.write(Played::new(FireDeclaration::new(
                actor, *target, *mode, *rounds,
            )));
            if entry.provenance().is_reaction() {
                ActHold::timed(*tuning.reaction_beat_seconds)
            } else {
                ActHold::timed(*tuning.fire_beat_seconds)
            }
        }
        ActDeed::RoundResolved { shot } => {
            // Emitting this is what SPAWNS the bolt. Everything the round did — the damage
            // numbers, the injury, the death — sits behind it in the log, so the impact
            // hold below is what stops those being drawn while the bolt is still flying.
            played.round.write(Played::new((**shot).clone()));
            ActHold::awaiting_impact(*tuning.impact_cap_seconds, *tuning.round_seconds)
        }
        ActDeed::Reloaded { outcome } => {
            played
                .reload
                .write(Played::new(ReloadResult::new(actor, *outcome)));
            ActHold::timed(*tuning.reload_seconds)
        }
        ActDeed::MagazineChanged { magazine } => {
            if let Ok(mut drawn_magazine) = drawn.magazines.get_mut(actor) {
                drawn_magazine.set_if_neq(DrawnMagazine::new(*magazine));
            }
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::Injured { injury } => {
            played.injury.write(Played::new((**injury).clone()));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::VitalsChanged { vitals } => {
            if let Ok(mut drawn_vitals) = drawn.vitals.get_mut(actor) {
                drawn_vitals.set_if_neq(DrawnVitals::new(vitals.clone()));
            }
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::Fell {
            from_level,
            to_level,
            storeys,
        } => {
            played.fall.write(Played::new(FallOccurred::new(
                actor,
                *from_level,
                *to_level,
                *storeys,
            )));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::Struck { target, hp_damage } => {
            played
                .strike
                .write(Played::new(MeleeStruck::new(actor, *target, *hp_damage)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::DiedAt { at } => {
            played
                .death
                .write(Played::new(OnDeathOccurred::new(actor, *at)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::Suppressed { at } => {
            played
                .suppression
                .write(Played::new(SuppressionApplied::new(actor, *at)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::ArmorBroke { part } => {
            played
                .armor_broken
                .write(Played::new(ArmorBroken::new(actor, *part)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::DotStarted { per_turn } => {
            played
                .dot
                .write(Played::new(DotAfflicted::new(actor, *per_turn)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::FieldStarted { at } => {
            played
                .field
                .write(Played::new(FieldAfflicted::new(actor, *at)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::DotTicked { at, amount } => {
            played
                .dot_tick
                .write(Played::new(DotTicked::new(actor, *at, *amount)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::FieldTicked { at, amount } => {
            played
                .field_tick
                .write(Played::new(FieldTicked::new(actor, *at, *amount)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::BleedStarted => {
            played
                .bleed_started
                .write(Played::new(BleedStarted::new(actor)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::Bled => {
            played.bleeding.write(Played::new(Bleeding::new(actor)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::CoverSmashed { at } => {
            played.cover.write(Played::new(CoverDestroyed::new(*at)));
            ActHold::timed(*tuning.minor_seconds)
        }
        ActDeed::MeleeLanded { at, damage } => {
            played
                .melee_landed
                .write(Played::new(MeleeResolved::new(*at, *damage)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::ThrowLanded { at, damage } => {
            played
                .throw_landed
                .write(Played::new(ThrowResolved::new(*at, *damage)));
            ActHold::timed(*tuning.consequence_seconds)
        }
        ActDeed::LifeChanged { to, .. } => {
            if let Ok(mut drawn_life) = drawn.lives.get_mut(actor) {
                drawn_life.set_if_neq(DrawnLife::new(*to));
            }
            ActHold::timed(*tuning.life_change_seconds)
        }
    }
}
