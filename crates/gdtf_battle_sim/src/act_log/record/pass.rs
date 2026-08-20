//! Main act-log recording pass over all act families.

use bevy::prelude::ResMut;

use super::{
    consequence::{record_consequence_messages, record_magazines, record_vitals},
    fire::record_fire,
    life::record_life,
    movement::record_movement,
    posture::record_posture,
    sources::{ActMessages, ActObservation, ActState, ProvenanceSources},
    turn::record_turn,
    view::record_entered_view,
};
use crate::act_log::ActLog;

/// Append new act-log entries from this frame's messages and state changes.
pub fn record_acts(
    mut log: ResMut<ActLog>,
    provenance: ProvenanceSources,
    seen: ActObservation,
    mut messages: ActMessages,
    state: ActState,
) {
    let log = &mut *log;
    record_turn(log, &mut messages.turn, &provenance, &seen);
    record_posture(log, &state.posture, &provenance, &seen);
    record_movement(log, &mut messages.movement, &provenance, &seen);
    record_fire(log, &mut messages.fire, &provenance, &seen);
    record_consequence_messages(log, &mut messages.consequences, &provenance, &seen);
    record_vitals(log, &state.vitals, &provenance, &seen);
    record_magazines(log, &state.vitals, &provenance, &seen);
    record_life(log, &state.life, &provenance, &seen);
    record_entered_view(log, &state.view, &provenance, &seen);
}
