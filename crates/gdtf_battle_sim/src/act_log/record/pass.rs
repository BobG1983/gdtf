//! Main act-log recording pass over all act families.

use bevy::prelude::ResMut;

use super::{
    consequence::{record_consequence_messages, record_magazines, record_vitals},
    fire::record_fire,
    life::record_life,
    movement::record_movement,
    posture::record_posture,
    sources::{ActMessages, ActState, ProvenanceSources},
    turn::record_turn,
};
use crate::act_log::ActLog;

/// Append new act-log entries from this frame's messages and state changes.
pub fn record_acts(
    mut log: ResMut<ActLog>,
    provenance: ProvenanceSources,
    mut messages: ActMessages,
    state: ActState,
) {
    let log = &mut *log;
    record_turn(log, &mut messages.turn, &provenance);
    record_posture(log, &state.posture, &provenance);
    record_movement(log, &mut messages.movement, &provenance);
    record_fire(log, &mut messages.fire, &provenance);
    record_consequence_messages(log, &mut messages.consequences, &provenance);
    record_vitals(log, &state.vitals, &provenance);
    record_magazines(log, &state.vitals, &provenance);
    record_life(log, &state.life, &provenance);
}
