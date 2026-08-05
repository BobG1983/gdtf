//! Advance the playback cursor through the act log.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{ActSeq, PositionFacts, VitalsFacts},
    ganger::{Hp, LifeState, Position, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
};

use super::{
    apply::{DrawnWriters, show_entry},
    cursor::LogPlayhead,
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

/// Tick holds and show the next act-log entry when free.
pub fn advance_playback(
    time: Res<Time>,
    mut playhead: LogPlayhead,
    tuning: Res<PlaybackTuning>,
    fx: FxPipelineProbe,
    resync: Query<ResyncData>,
    mut drawn: DrawnWriters,
    mut played: PlayedSignals,
) {
    let Some((oldest, head)) = playhead.span() else {
        if playhead.cursor.shown() != ActSeq::START || playhead.cursor.is_holding() {
            playhead.cursor.reset();
        }
        return;
    };

    if playhead.cursor.shown() > head {
        playhead.cursor.reset();
    }

    if !playhead.cursor.tick_hold(time.delta(), fx.is_busy()) {
        return;
    }

    if playhead.cursor.shown() < oldest {
        resync_drawn_world(&resync, &mut drawn);
        playhead.cursor.jump_to(head);
        return;
    }

    let shown = playhead.cursor.shown();
    let Some(entry) = playhead.log.as_ref().and_then(|log| log.at(shown)) else {
        return;
    };
    let hold = show_entry(entry, &tuning, &mut drawn, &mut played);
    playhead.cursor.advance_past_shown();
    playhead.cursor.hold_for(hold);
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
