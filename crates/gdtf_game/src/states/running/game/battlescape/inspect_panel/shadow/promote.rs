use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{
    cover::CoverLedger,
    emplacement::{EmplacementState, MountedWeaponKey},
    entity::TerrainCell,
    prelude::OccupancyGrid,
};

use super::resources::{ShownCoverLedger, ShownEmplacement, ShownEmplacements, ShownOccupancyGrid};

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

type EmplacementRow = (
    &'static TerrainCell,
    &'static EmplacementState,
    &'static MountedWeaponKey,
);

pub(crate) fn promote_shown_emplacements(
    live: Query<EmplacementRow>,
    mut shadow: ResMut<ShownEmplacements>,
    playback: PlaybackGate,
) {
    if playback.is_open() {
        shadow.promote(
            live.iter().map(|(cell, state, weapon)| {
                (**cell, ShownEmplacement::new(*state, weapon.clone()))
            }),
        );
    }
}
