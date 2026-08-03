use bevy::prelude::ResMut;

use super::{
    consequence::{record_consequence_messages, record_magazines, record_vitals},
    fire::record_fire,
    life::record_life,
    movement::record_movement,
    posture::record_posture,
    sources::{
        ConsequenceMessages, ConsequenceState, FireSources, LifeSources, MovementSources,
        PostureSources, ProvenanceSources, TurnSources,
    },
    turn::record_turn,
};
use crate::act_log::ActLog;

#[expect(
    clippy::too_many_arguments,
    reason = "the parameter list IS the recorder's source inventory: one SystemParam bundle \
              per act family, plus the log. Bundling them further would hide which family \
              reads what — the exact thing this system's fixed family order is documented \
              on — and Bevy's own arity ceiling is nowhere near being pressed (the sources \
              are already grouped into six bundles from ~20 underlying reads)"
)]
pub fn record_acts(
    mut log: ResMut<ActLog>,
    provenance: ProvenanceSources,
    mut turn: TurnSources,
    posture: PostureSources,
    mut movement: MovementSources,
    mut fire: FireSources,
    mut consequence_messages: ConsequenceMessages,
    consequence_state: ConsequenceState,
    life: LifeSources,
) {
    let log = &mut *log;
    record_turn(log, &mut turn, &provenance);
    record_posture(log, &posture, &provenance);
    record_movement(log, &mut movement, &provenance);
    record_fire(log, &mut fire, &provenance);
    record_consequence_messages(log, &mut consequence_messages, &provenance);
    record_vitals(log, &consequence_state, &provenance);
    record_magazines(log, &consequence_state, &provenance);
    record_life(log, &life, &provenance);
}
