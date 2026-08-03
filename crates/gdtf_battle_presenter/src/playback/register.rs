use bevy::prelude::*;

use super::{
    advance::advance_playback,
    cursor::PlaybackCursor,
    dwell::{PlaybackTuning, register_playback_tuning_hot_ron},
    emit::register_played_messages,
    seed::seed_drawn_state,
};
use crate::PresenterSystems;

pub fn register_playback(app: &mut App) {
    app.init_resource::<PlaybackCursor>()
        .init_resource::<PlaybackTuning>();
    register_played_messages(app);
    register_playback_tuning_hot_ron(app);
    app.add_systems(
        Update,
        (seed_drawn_state, advance_playback)
            .chain()
            .in_set(PresenterSystems::Replay),
    );
}
