//! Playback-gated copy of live squad visibility for fog draw.

use bevy::prelude::*;
use gdtf_battle_sim::visibility::SquadVisibility;

use crate::playback::PlaybackGate;

/// Fog uses this shadow so visibility only updates when playback is caught up.
#[derive(Resource, Debug, Clone, Default, Deref)]
pub struct ShownSquadVisibility(SquadVisibility);

impl ShownSquadVisibility {
    /// Replace the shadow with the live squad map.
    pub fn promote(&mut self, live: &SquadVisibility) {
        self.0 = live.clone();
    }

    /// Borrow the shadowed visibility.
    #[must_use]
    pub const fn visibility(&self) -> &SquadVisibility {
        &self.0
    }
}

/// Copy live fog into the shadow when the input gate is open.
pub fn promote_shown_fog(
    live: Res<SquadVisibility>,
    mut shadow: ResMut<ShownSquadVisibility>,
    playback: PlaybackGate,
) {
    if playback.is_open() {
        shadow.promote(&live);
    }
}
