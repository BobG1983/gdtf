use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{cover::CoverLedger, prelude::OccupancyGrid};

use super::resources::{ShownCoverLedger, ShownOccupancyGrid};

pub(crate) fn promote_shown_occupancy(
    live: Option<Res<OccupancyGrid>>,
    mut shadow: ResMut<ShownOccupancyGrid>,
    playback: PlaybackGate,
    mut was_caught_up: Local<bool>,
) {
    let caught_up = playback.is_open();
    let just_caught_up = caught_up && !*was_caught_up;
    *was_caught_up = caught_up;
    let Some(live) = live else {
        return;
    };
    if caught_up && (live.is_changed() || just_caught_up) {
        shadow.promote(&live);
    }
}

pub(crate) fn promote_shown_cover(
    live: Option<Res<CoverLedger>>,
    mut shadow: ResMut<ShownCoverLedger>,
    playback: PlaybackGate,
) {
    if playback.is_open()
        && let Some(live) = live
    {
        shadow.promote(&live);
    }
}
