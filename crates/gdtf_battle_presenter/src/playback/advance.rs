//! Advance the playback cursor through the act log.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{ActLog, PositionFacts, VitalsFacts},
    ganger::{Hp, LifeState, Position, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
};

use super::{
    apply::{DrawnWriters, show_entry},
    cursor::PlaybackCursor,
    drawn::{DrawnLife, DrawnPosition, DrawnVitals},
    dwell::PlaybackTuning,
    emit::PlayedSignals,
};
use crate::{PendingImpact, ShotProjectile};

type ResyncData = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Tu,
    &'static Hp,
    &'static Wounds,
    Option<&'static InflictedWounds>,
    Option<&'static InflictedInjuries>,
);

/// Probe for in-flight projectiles and pending impacts.
#[derive(bevy::ecs::system::SystemParam)]
pub struct FxPipelineProbe<'w, 's> {
    projectiles: Query<'w, 's, (), With<ShotProjectile>>,
    pending:     Query<'w, 's, (), With<PendingImpact>>,
}

impl FxPipelineProbe<'_, '_> {
    fn is_busy(&self) -> bool {
        !self.projectiles.is_empty() || !self.pending.is_empty()
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "each param is a distinct, independently-borrowed read the cursor genuinely \
              needs — the clock, the log, the cursor, the dwell table, the FX-busy probe, \
              the gap-resync read, the drawn writers, and the played-signal writers. The \
              two multi-query groups are ALREADY bundled (`FxPipelineProbe` / \
              `DrawnWriters`); bundling the rest would only hide the access set"
)]
/// Tick holds and show the next act-log entry when free.
pub fn advance_playback(
    time: Res<Time>,
    log: Option<Res<ActLog>>,
    mut cursor: ResMut<PlaybackCursor>,
    tuning: Res<PlaybackTuning>,
    fx: FxPipelineProbe,
    resync: Query<ResyncData>,
    mut drawn: DrawnWriters,
    mut played: PlayedSignals,
) {
    let Some(log) = log else {
        if cursor.shown() != gdtf_battle_sim::act_log::ActSeq::START || cursor.is_holding() {
            cursor.reset();
        }
        return;
    };

    if cursor.shown() > log.head() {
        cursor.reset();
    }

    if !cursor.tick_hold(time.delta(), fx.is_busy()) {
        return;
    }

    if cursor.shown() < log.oldest_seq() {
        resync_drawn_world(&resync, &mut drawn);
        cursor.jump_to(log.head());
        return;
    }

    let Some(entry) = log.at(cursor.shown()) else {
        return;
    };
    let hold = show_entry(entry, &tuning, &mut drawn, &mut played);
    cursor.advance_past_shown();
    cursor.hold_for(hold);
}

fn resync_drawn_world(resync: &Query<ResyncData>, drawn: &mut DrawnWriters) {
    for (entity, position, life, tu, hp, wounds, inflicted, injuries) in resync {
        if let Ok(mut drawn_position) = drawn.positions.get_mut(entity) {
            drawn_position.set_if_neq(DrawnPosition::new(PositionFacts::new(*position)));
        }
        if let Ok(mut drawn_life) = drawn.lives.get_mut(entity) {
            drawn_life.set_if_neq(DrawnLife::new(*life));
        }
        if let Ok(mut drawn_vitals) = drawn.vitals.get_mut(entity) {
            drawn_vitals.set_if_neq(DrawnVitals::new(VitalsFacts::new(
                *tu,
                *hp,
                *wounds,
                inflicted.cloned().unwrap_or_default(),
                injuries.cloned().unwrap_or_default(),
            )));
        }
    }
}
