//! Re-emit an act-log entry's facts as `Played` messages for the view.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry},
    acts::{
        FireDeclaration, MeleeResolved, MeleeStruck, MoveCompleted, MoveRejected, MovementOccurred,
        ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::TerrainPieceDestroyed,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

use super::super::emit::{Played, PlayedSignals};

pub(super) fn emit_played(entry: &ActEntry, played: &mut PlayedSignals) {
    let actor = entry.actor();
    match entry.deed() {
        ActDeed::TurnBegan { now_active } => play(&mut played.turn, TurnStarted::new(*now_active)),
        ActDeed::Stepped { from, to, .. } => {
            play(&mut played.step, MovementOccurred::new(actor, *from, *to));
        }
        ActDeed::MovedTo { position } => {
            play(
                &mut played.move_completed,
                MoveCompleted::new(actor, *position.inner()),
            );
        }
        ActDeed::MoveRefused { reason } => {
            play(&mut played.refusal, MoveRejected::new(actor, *reason));
        }
        ActDeed::Fired {
            target,
            mode,
            rounds,
        } => play(
            &mut played.declaration,
            FireDeclaration::new(actor, *target, *mode, *rounds),
        ),
        ActDeed::RoundResolved { shot } => play(&mut played.round, (**shot).clone()),
        ActDeed::Reloaded { outcome } => {
            play(&mut played.reload, ReloadResult::new(actor, *outcome));
        }
        ActDeed::Injured { injury } => play(&mut played.injury, (**injury).clone()),
        ActDeed::Fell {
            from_level,
            to_level,
            storeys,
        } => play(
            &mut played.fall,
            FallOccurred::new(actor, *from_level, *to_level, *storeys),
        ),
        ActDeed::Struck { target, hp_damage } => {
            play(
                &mut played.strike,
                MeleeStruck::new(actor, *target, *hp_damage),
            );
        }
        ActDeed::DiedAt { at } => play(&mut played.death, OnDeathOccurred::new(actor, *at)),
        ActDeed::Suppressed { at } => {
            play(&mut played.suppression, SuppressionApplied::new(actor, *at));
        }
        ActDeed::ArmorBroke { part } => {
            play(&mut played.armor_broken, ArmorBroken::new(actor, *part));
        }
        ActDeed::DotStarted { per_turn } => {
            play(&mut played.dot, DotAfflicted::new(actor, *per_turn));
        }
        ActDeed::FieldStarted { at } => play(&mut played.field, FieldAfflicted::new(actor, *at)),
        ActDeed::DotTicked { at, amount } => {
            play(&mut played.dot_tick, DotTicked::new(actor, *at, *amount));
        }
        ActDeed::FieldTicked { at, amount } => {
            play(
                &mut played.field_tick,
                FieldTicked::new(actor, *at, *amount),
            );
        }
        ActDeed::BleedStarted => play(&mut played.bleed_started, BleedStarted::new(actor)),
        ActDeed::Bled => play(&mut played.bleeding, Bleeding::new(actor)),
        ActDeed::TerrainPieceSmashed { at, kind } => {
            play(&mut played.cover, TerrainPieceDestroyed::new(*at, *kind));
        }
        ActDeed::MeleeLanded { at, damage } => {
            play(&mut played.melee_landed, MeleeResolved::new(*at, *damage));
        }
        ActDeed::ThrowLanded { at, damage } => {
            play(&mut played.throw_landed, ThrowResolved::new(*at, *damage));
        }
        ActDeed::PostureChanged { .. }
        | ActDeed::EnteredView { .. }
        | ActDeed::MagazineChanged { .. }
        | ActDeed::VitalsChanged { .. }
        | ActDeed::LifeChanged { .. } => {}
    }
}

// Wrap a sim fact as played and hand it to its writer.
fn play<M: Message + Clone>(writer: &mut MessageWriter<Played<M>>, fact: M) {
    writer.write(Played::new(fact));
}
