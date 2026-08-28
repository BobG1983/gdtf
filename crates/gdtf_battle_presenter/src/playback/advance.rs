//! Advance the playback cursor through the act log.

use bevy::prelude::*;
use gdtf_battle_sim::act_log::ActSeq;

use super::{
    apply::{DrawnWriters, show_entry},
    cursor::LogPlayhead,
    dwell::PlaybackTuning,
    emit::PlayedSignals,
};
use crate::{PendingImpact, ShotProjectile};

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
    mut drawn: DrawnWriters,
    mut played: PlayedSignals,
) {
    let Some(head) = playhead.head() else {
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

    let shown = playhead.cursor.shown();
    let Some(entry) = playhead.log.as_ref().and_then(|log| log.at(shown)) else {
        return;
    };
    let hold = show_entry(entry, &tuning, &mut drawn, &mut played);
    playhead.cursor.advance_past_shown();
    playhead.cursor.hold_for(hold);
}
