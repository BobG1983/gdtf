use bevy::prelude::*;
use gdtf_battle_sim::visibility::SquadVisibility;

use crate::playback::PlaybackGate;

#[derive(Resource, Debug, Clone, Default, Deref)]
pub struct ShownSquadVisibility(SquadVisibility);

impl ShownSquadVisibility {
            pub fn promote(&mut self, live: &SquadVisibility) {
        self.0 = live.clone();
    }

            #[must_use]
    pub const fn visibility(&self) -> &SquadVisibility {
        &self.0
    }
}

pub fn promote_shown_fog(
    live: Res<SquadVisibility>,
    mut shadow: ResMut<ShownSquadVisibility>,
    playback: PlaybackGate,
) {
    if playback.is_open() {
        shadow.promote(&live);
    }
}
